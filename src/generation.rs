use crate::ast::{Expr, Facet, Field, Program, Term};

mod acm_dl;
mod ieee;
mod science_direct;
mod springer;

pub use acm_dl::{ACM_DL, AcmDl};
pub use ieee::{IEEE_XPLORE, IeeeXplore};
pub use science_direct::{SCIENCEDIRECT_MAX_CONNECTORS, ScienceDirect, SubQuery, split_query};
pub use springer::SpringerLink;

pub trait QueryGenerator {
    fn generate_facet(&self, out: &mut String, facet: &Facet, fields: &[Field]);

    fn generate(&self, program: &Program) -> String {
        let mut query = String::new();

        self.generate_expr(&mut query, &program.query, program);

        query
    }

    /// Generates one or more queries whose union is equivalent to the
    /// program's query. Most databases accept the query as a single string.
    fn generate_all(&self, program: &Program) -> Result<Vec<String>, String> {
        Ok(vec![self.generate(program)])
    }

    /// Notes on how the generated query deviates from the program, e.g.
    /// fields the database cannot search.
    fn warnings(&self, _program: &Program) -> Vec<String> {
        Vec::new()
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

/// Emits a facet as a bare OR of its terms, without field names. Used by
/// databases where the field is chosen in the search form instead.
fn generate_bare_facet(out: &mut String, facet: &Facet) {
    enclose(out, |out| {
        separated(out, &facet.terms, " OR ", generate_term);
    });
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

/// Parses `input` and generates a single query with `generator`.
#[cfg(test)]
fn generate_with(generator: &impl QueryGenerator, input: &str) -> String {
    use chumsky::Parser;

    let ast = crate::parser::program().parse(input).into_result().unwrap();
    generator.generate(&ast)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate(input: &str) -> String {
        generate_with(&AcmDl, input)
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
    #[should_panic(expected = "facet `missing` is not defined")]
    fn undefined_facet_panics() {
        generate("fields title facet a = x query missing");
    }
}
