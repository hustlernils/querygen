use crate::ast::{Expr, Facet, Field, Program, Term};

pub trait QueryGenerator {
    fn generate_facet(&self, out: &mut String, facet: &Facet, fields: &[Field]);

    fn generate(&self, program: &Program) -> String {
        let mut query = String::new();

        self.generate_expr(&mut query, &program.query, program);

        query
    }

    fn generate_expr(&self, out: &mut String, expr: &Expr, program: &Program) {
        match expr {
            Expr::Ref(name) => self.generate_ref(out, name, program),
            Expr::And(children) => self.generate_children(out, children, " AND ", program),
            Expr::Or(children) => self.generate_children(out, children, " OR ", program),
        }
    }

    fn generate_ref(&self, out: &mut String, name: &str, program: &Program) {
        // TODO: replace the panic with a semantic check after parsing
        let facet = program
            .facet(name)
            .unwrap_or_else(|| panic!("facet `{name}` is not defined"));

        self.generate_facet(out, facet, &program.fields);
    }

    fn generate_children(
        &self,
        out: &mut String,
        children: &[Expr],
        separator: &str,
        program: &Program,
    ) {
        separated(out, children, separator, |out, child| match child {
            Expr::Ref(_) => self.generate_expr(out, child, program),
            Expr::And(_) | Expr::Or(_) => {
                enclose(out, |out| self.generate_expr(out, child, program))
            }
        });
    }
}

pub struct DatabaseConfig {
    pub title_field: &'static str,
    pub abstract_field: &'static str,
    pub keywords_field: &'static str,
}

impl DatabaseConfig {
    /// Maps our fields to the database's name
    pub fn field_name(&self, field: &Field) -> &'static str {
        match field {
            Field::Title => self.title_field,
            Field::Abstract => self.abstract_field,
            Field::Keywords => self.keywords_field,
        }
    }
}

pub const ACM_DL: DatabaseConfig = DatabaseConfig {
    title_field: "Title",
    abstract_field: "Abstract",
    keywords_field: "Keyword",
};

pub const IEEE_XPLORE: DatabaseConfig = DatabaseConfig {
    title_field: "\"Document Title\"",
    abstract_field: "\"Abstract\"",
    keywords_field: "\"Author Keywords\"",
};

pub struct AcmDl;

impl QueryGenerator for AcmDl {
    fn generate_facet(&self, out: &mut String, facet: &Facet, fields: &[Field]) {
        enclose(out, |out| {
            separated(out, fields, " OR ", |out, field| {
                out.push_str(ACM_DL.field_name(field));
                out.push(':');
                enclose(out, |out| {
                    separated(out, &facet.terms, " OR ", generate_term);
                });
            });
        });
    }
}

pub struct IeeeXplore;

impl QueryGenerator for IeeeXplore {
    fn generate_facet(&self, out: &mut String, facet: &Facet, fields: &[Field]) {
        enclose(out, |out| {
            separated(out, fields, " OR ", |out, field| {
                separated(out, &facet.terms, " OR ", |out, term| {
                    out.push_str(IEEE_XPLORE.field_name(field));
                    out.push(':');
                    generate_term(out, term);
                });
            });
        });
    }
}

fn generate_term(out: &mut String, term: &Term) {
    match term {
        Term::Word(s) => out.push_str(s),
        Term::Phrase(s) => {
            out.push('"');
            out.push_str(s);
            out.push('"');
        }
    }
}

fn enclose(out: &mut String, inner: impl FnOnce(&mut String)) {
    out.push('(');
    inner(out);
    out.push(')');
}

fn separated<T>(
    out: &mut String,
    items: &[T],
    separator: &str,
    mut render: impl FnMut(&mut String, &T),
) {
    for (i, item) in items.iter().enumerate() {
        if i != 0 {
            out.push_str(separator);
        }
        render(out, item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::program;
    use chumsky::Parser;

    fn generate(input: &str) -> String {
        generate_with(&AcmDl, input)
    }

    fn generate_with(generator: &impl QueryGenerator, input: &str) -> String {
        let ast = program().parse(input).into_result().unwrap();
        generator.generate(&ast)
    }

    #[test]
    fn generates_acm_dl_query() {
        let input = include_str!("../test/jsonb.query");
        let expected = include_str!("../test/acm_dl");

        assert_eq!(generate(input), expected);
    }

    #[test]
    fn nested_or_inside_and_keeps_parentheses() {
        let input = "fields title facet a = x facet b = y facet c = z query (a OR b) AND c";

        assert_eq!(
            generate(input),
            "((Title:(x)) OR (Title:(y))) AND (Title:(z))"
        );
    }

    #[test]
    fn and_inside_or_is_enclosed() {
        let input = "fields title facet a = x facet b = y facet c = z query a OR b AND c";

        assert_eq!(
            generate(input),
            "(Title:(x)) OR ((Title:(y)) AND (Title:(z)))"
        );
    }

    #[test]
    fn field_names_per_database() {
        assert_eq!(ACM_DL.field_name(&Field::Keywords), "Keyword");
        assert_eq!(IEEE_XPLORE.field_name(&Field::Title), "\"Document Title\"");
        assert_eq!(
            IEEE_XPLORE.field_name(&Field::Keywords),
            "\"Author Keywords\""
        );
    }

    #[test]
    fn ieee_prefixes_every_term_fields_first() {
        let input =
            "fields title, abstract, keywords facet a = json OR \"schema evolution\" query a";

        assert_eq!(
            generate_with(&IeeeXplore, input),
            "(\"Document Title\":json OR \"Document Title\":\"schema evolution\" \
             OR \"Abstract\":json OR \"Abstract\":\"schema evolution\" \
             OR \"Author Keywords\":json OR \"Author Keywords\":\"schema evolution\")"
        );
    }

    #[test]
    fn ieee_nested_or_inside_and_keeps_parentheses() {
        let input = "fields title facet a = x facet b = y facet c = z query (a OR b) AND c";

        assert_eq!(
            generate_with(&IeeeXplore, input),
            "((\"Document Title\":x) OR (\"Document Title\":y)) AND (\"Document Title\":z)"
        );
    }

    #[test]
    #[should_panic(expected = "facet `missing` is not defined")]
    fn undefined_facet_panics() {
        generate("fields title facet a = x query missing");
    }
}
