use crate::ast::*;
use chumsky::prelude::*;

pub fn field<'a>() -> impl Parser<'a, &'a str, Field, extra::Err<Rich<'a, char>>> {
    let title = just("title").to(Field::Title);
    let abs = just("abstract").to(Field::Abstract);
    let keywords = just("keywords").to(Field::Keywords);
    choice((title, abs, keywords))
}

#[test]
fn parse_field_returns_field() {
    assert_eq!(field().parse("title").into_result(), Ok(Field::Title));

    assert!(field().parse("123").has_errors());
}

#[test]
fn parse_field_returns_abstract() {
    assert_eq!(field().parse("abstract").into_result(), Ok(Field::Abstract));

    assert!(field().parse("123").has_errors());
}

#[test]
fn parse_field_returns_keywords() {
    assert_eq!(field().parse("keywords").into_result(), Ok(Field::Keywords));

    assert!(field().parse("123").has_errors());
}