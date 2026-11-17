use crate::ast::{Expr, Facet, Field, Program, Term};

pub trait QueryGenerator {
    fn generate(&self, program: &Program) -> String;
}

pub struct AcmDl;

impl QueryGenerator for AcmDl {
    fn generate(&self, program: &Program) -> String {
        let mut query = String::new();

        self.generate_expr(&mut query, &program.query, program);

        query
    }
}

impl AcmDl {
    const TITLE_FIELD: &str = "Title";
    const ABSTRACT_FIELD: &str = "Abstract";
    const KEYWORDS_FIELD: &str = "Keyword";

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

    fn generate_facet(&self, out: &mut String, facet: &Facet, fields: &[Field]) {
        enclose(out, |out| {
            separated(out, fields, " OR ", |out, field| {
                out.push_str(self.field_name(field));
                out.push(':');
                enclose(out, |out| {
                    separated(out, &facet.terms, " OR ", |out, term| {
                        self.generate_term(out, term)
                    });
                });
            });
        });
    }

    fn field_name(&self, field: &Field) -> &'static str {
        match field {
            Field::Title => Self::TITLE_FIELD,
            Field::Abstract => Self::ABSTRACT_FIELD,
            Field::Keywords => Self::KEYWORDS_FIELD,
        }
    }

    fn generate_term(&self, out: &mut String, term: &Term) {
        match term {
            Term::Word(s) => out.push_str(s),
            Term::Phrase(s) => out.push_str(&format!("\"{}\"", s)),
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
        let ast = program().parse(input).into_result().unwrap();
        AcmDl.generate(&ast)
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

        assert_eq!(generate(input), "((Title:(x)) OR (Title:(y))) AND (Title:(z))");
    }

    #[test]
    fn and_inside_or_is_enclosed() {
        let input = "fields title facet a = x facet b = y facet c = z query a OR b AND c";

        assert_eq!(generate(input), "(Title:(x)) OR ((Title:(y)) AND (Title:(z)))");
    }

    #[test]
    #[should_panic(expected = "facet `missing` is not defined")]
    fn undefined_facet_panics() {
        generate("fields title facet a = x query missing");
    }
}
