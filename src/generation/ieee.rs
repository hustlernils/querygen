use super::{DatabaseConfig, QueryGenerator, enclose, generate_term, separated};
use crate::ast::{Facet, Field};

pub const IEEE_XPLORE: DatabaseConfig = DatabaseConfig {
    title_field: "\"Document Title\"",
    abstract_field: "\"Abstract\"",
    keywords_field: "\"Author Keywords\"",
};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::{ACM_DL, generate_with};

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
}
