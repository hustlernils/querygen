use super::{QueryGenerator, generate_bare_facet};
use crate::ast::{Facet, Field, Program};

/// SpringerLink has no field syntax and no abstract or author-keyword field:
/// the Advanced Search form only offers "Title" and "Keywords" (which searches
/// the whole record, including full text). Only the Boolean expression of
/// terms is emitted; it works in either box.
pub struct SpringerLink;

impl QueryGenerator for SpringerLink {
    fn generate_facet(&self, out: &mut String, facet: &Facet, _fields: &[Field]) {
        generate_bare_facet(out, facet);
    }

    fn warnings(&self, program: &Program) -> Vec<String> {
        let unsupported: Vec<&str> = program
            .fields
            .iter()
            .filter_map(|field| match field {
                Field::Title => None,
                Field::Abstract => Some("abstract"),
                Field::Keywords => Some("keywords"),
            })
            .collect();

        if unsupported.is_empty() {
            return Vec::new();
        }

        vec![format!(
            "SpringerLink cannot search {} separately; paste the query into the \
             \"Title\" box (title only) or the \"Keywords\" box (full text)",
            unsupported.join(", ")
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::generate_with;
    use crate::parser::program;
    use chumsky::Parser;

    fn warnings(input: &str) -> Vec<String> {
        SpringerLink.warnings(&program().parse(input).into_result().unwrap())
    }

    #[test]
    fn springer_emits_terms_without_fields() {
        let input = "fields title, abstract, keywords \
                     facet a = json OR \"document store\" facet b = x facet c = y \
                     query a AND (b OR c)";

        assert_eq!(
            generate_with(&SpringerLink, input),
            "(json OR \"document store\") AND ((x) OR (y))"
        );
    }

    #[test]
    fn springer_warns_about_abstract_and_keywords() {
        let warnings = warnings("fields title, abstract, keywords facet a = x query a");

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("abstract, keywords"), "{warnings:?}");
    }

    #[test]
    fn springer_title_only_has_no_warning() {
        assert!(warnings("fields title facet a = x query a").is_empty());
    }
}
