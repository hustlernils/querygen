#[derive(Debug, Clone, PartialEq)]
pub struct Program<'a> {
    pub fields: Vec<Field>,
    pub facets: Vec<Facet<'a>>,
    pub query: Expr<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Title,
    Abstract,
    Keywords,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Facet<'a> {
    pub name: &'a str,
    pub terms: Vec<Term<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'a> {
    Ref(&'a str),       // reference to a facet by name
    And(Vec<Expr<'a>>), // n-ary: a AND b AND c
    Or(Vec<Expr<'a>>),  // n-ary: a OR b OR c
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term<'a> {
    Word(&'a str),   // word without quotes
    Phrase(&'a str), // quoted string, without the quotes
}
