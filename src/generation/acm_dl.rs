use super::{DatabaseConfig, QueryGenerator, enclose, generate_term, separated};
use crate::ast::{Facet, Field};

pub const ACM_DL: DatabaseConfig = DatabaseConfig {
    title_field: "Title",
    abstract_field: "Abstract",
    keywords_field: "Keyword",
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
