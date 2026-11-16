use crate::ast::Program;

pub trait QueryGenerator {
    fn generate(&self, program: &Program) -> String;
}

pub struct AcmDl;

impl QueryGenerator for AcmDl {
    fn generate(&self, program: &Program) -> String {
        todo!();
    }
}