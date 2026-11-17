use crate::ast::*;
use chumsky::prelude::*;

const FIELDS_KEYWORD: &str = "fields";
const TITLE_KEYWORD: &str = "title";
const ABSTRACT_KEYWORD: &str = "abstract";
const KEYWORDS_KEYWORD: &str = "keywords";
const FACET_KEYWORD: &str = "facet";
const OR_KEYWORD: &str = "OR";
const AND_KEYWORD: &str = "AND";
const QUERY_KEYWORD: &str = "query";

pub fn field<'a>() -> impl Parser<'a, &'a str, Field, extra::Err<Rich<'a, char>>> {
    let title = text::ascii::keyword(TITLE_KEYWORD).to(Field::Title);
    let abs = text::ascii::keyword(ABSTRACT_KEYWORD).to(Field::Abstract);
    let keywords = text::ascii::keyword(KEYWORDS_KEYWORD).to(Field::Keywords);

    choice((title, abs, keywords))
}

pub fn fields<'a>() -> impl Parser<'a, &'a str, Vec<Field>, extra::Err<Rich<'a, char>>> {
    let keyword = text::ascii::keyword(FIELDS_KEYWORD).padded();

    let fields = field()
        .separated_by(just(',').padded())
        .at_least(1)
        .collect::<Vec<_>>();

    keyword.ignore_then(fields)
}

pub fn term<'a>() -> impl Parser<'a, &'a str, Term<'a>, extra::Err<Rich<'a, char>>> {
    let word = text::ascii::ident().map(Term::Word);

    let phrase = none_of('"')
        .repeated()
        .to_slice()
        .delimited_by(just('"'), just('"'))
        .map(Term::Phrase);

    choice((word, phrase))
}

pub fn facet<'a>() -> impl Parser<'a, &'a str, Facet<'a>, extra::Err<Rich<'a, char>>> {
    let keyword = text::ascii::keyword(FACET_KEYWORD).padded();

    let facet_name = text::ascii::ident().padded();
    let equals = just('=').padded();

    let or_keyword = text::ascii::keyword(OR_KEYWORD).padded();
    let terms = term()
        .padded()
        .separated_by(or_keyword)
        .at_least(1)
        .collect::<Vec<_>>();

    keyword
        .ignore_then(facet_name)
        .then_ignore(equals)
        .then(terms)
        .map(|(name, terms)| Facet { name, terms })
}

/// Wraps `items` with `wrap`, unless there is just one item: then that item is
/// returned itself, so `a` becomes `Ref("a")` and not `Or([And([Ref("a")])])`.
fn flatten<'a>(mut items: Vec<Expr<'a>>, wrap: fn(Vec<Expr<'a>>) -> Expr<'a>) -> Expr<'a> {
    if items.len() == 1 {
        items.pop().unwrap()
    } else {
        wrap(items)
    }
}

/// OrExpr = AndExpr { "OR" AndExpr } ;
/// AndExpr = Atom { "AND" Atom } ;
/// Atom = Identifier | "(" OrExpr ")" ;
pub fn expr<'a>() -> impl Parser<'a, &'a str, Expr<'a>, extra::Err<Rich<'a, char>>> {
    // `expr` inside the closure is a handle to the parser being defined,
    // so `Atom` can refer back to `OrExpr` for parenthesised sub-expressions.
    recursive(|expr| {
        let atom = text::ascii::ident()
            .padded()
            .map(Expr::Ref)
            .or(expr.delimited_by(just('(').padded(), just(')').padded()));

        // AND is parsed one level deeper than OR, so it binds tighter.
        let and_expr = atom
            .separated_by(text::ascii::keyword(AND_KEYWORD).padded())
            .at_least(1)
            .collect::<Vec<_>>()
            .map(|items| flatten(items, Expr::And));

        and_expr
            .separated_by(text::ascii::keyword(OR_KEYWORD).padded())
            .at_least(1)
            .collect::<Vec<_>>()
            .map(|items| flatten(items, Expr::Or))
    })
}

pub fn query<'a>() -> impl Parser<'a, &'a str, Expr<'a>, extra::Err<Rich<'a, char>>> {
    text::ascii::keyword(QUERY_KEYWORD)
        .padded()
        .ignore_then(expr())
}

pub fn program<'a>() -> impl Parser<'a, &'a str, Program<'a>, extra::Err<Rich<'a, char>>> {
    let fields = fields();

    let facets = facet().repeated().collect::<Vec<_>>();

    let query = query();

    fields
        .then(facets)
        .then(query)
        .then_ignore(end())
        .map(|((fields, facets), query)| Program {
            fields,
            facets,
            query,
        })
}

#[test]
fn parse_program_returns_program() {
    let query = include_str!("../test/jsonb.query");

    let expected = Program {
        fields: vec![Field::Title, Field::Abstract, Field::Keywords],
        facets: vec![
            Facet {
                name: "data_model",
                terms: vec![
                    Term::Word("json"),
                    Term::Word("jsonb"),
                    Term::Word("nosql"),
                    Term::Phrase("document store"),
                    Term::Phrase("document database"),
                    Term::Phrase("document-oriented"),
                    Term::Phrase("semi-structured"),
                    Term::Word("schemaless"),
                    Term::Phrase("schema-less"),
                    Term::Word("mongodb"),
                ],
            },
            Facet {
                name: "migration",
                terms: vec![
                    Term::Phrase("schema evolution"),
                    Term::Phrase("schema migration"),
                    Term::Phrase("schema change"),
                    Term::Phrase("schema versioning"),
                    Term::Phrase("data migration"),
                    Term::Phrase("lazy migration"),
                    Term::Phrase("database migration"),
                ],
            },
        ],
        query: Expr::And(vec![Expr::Ref("data_model"), Expr::Ref("migration")]),
    };

    assert_eq!(program().parse(query).into_result(), Ok(expected));
}

#[test]
fn parse_program_rejects_trailing_input() {
    let input = "fields title\nfacet a = json\nquery a garbage";

    assert!(program().parse(input).has_errors());
}

#[cfg(test)]
mod expr_tests {
    use super::*;

    fn r(name: &str) -> Expr<'_> {
        Expr::Ref(name)
    }

    #[test]
    fn single_ref_is_not_wrapped() {
        assert_eq!(expr().parse("a").into_result(), Ok(r("a")));
    }

    #[test]
    fn and_of_two() {
        assert_eq!(
            expr().parse("a AND b").into_result(),
            Ok(Expr::And(vec![r("a"), r("b")]))
        );
    }

    #[test]
    fn and_binds_tighter_than_or() {
        assert_eq!(
            expr().parse("a OR b AND c").into_result(),
            Ok(Expr::Or(vec![r("a"), Expr::And(vec![r("b"), r("c")])]))
        );
    }

    #[test]
    fn parentheses_override_precedence() {
        assert_eq!(
            expr().parse("(a OR b) AND c").into_result(),
            Ok(Expr::And(vec![Expr::Or(vec![r("a"), r("b")]), r("c")]))
        );
    }

    #[test]
    fn nested_parentheses() {
        assert_eq!(
            expr().parse("a AND (b OR (c AND d))").into_result(),
            Ok(Expr::And(vec![
                r("a"),
                Expr::Or(vec![r("b"), Expr::And(vec![r("c"), r("d")])]),
            ]))
        );
    }

    #[test]
    fn query_keyword() {
        assert_eq!(
            query()
                .parse("query data_model AND migration")
                .into_result(),
            Ok(Expr::And(vec![r("data_model"), r("migration")]))
        );
    }

    #[test]
    fn invalid_expressions_fail() {
        for input in ["a AND", "(a", "a OR b)", ""] {
            assert!(expr().parse(input).has_errors(), "should fail: {input:?}");
        }
    }
}

#[test]
fn parse_facet_returns_facet() {
    let expected = Facet {
        name: "data_model",
        terms: vec![Term::Word("json"), Term::Word("jsonb"), Term::Word("nosql")],
    };

    assert_eq!(
        facet()
            .parse("facet data_model = json OR jsonb OR nosql")
            .into_result(),
        Ok(expected)
    );
}

#[test]
fn parse_term_returns_term() {
    let expected = Term::Phrase("schema evolution");

    assert_eq!(
        term().parse("\"schema evolution\"").into_result(),
        Ok(expected)
    );
}

#[test]
fn parse_term_returns_word() {
    let expected = Term::Word("jsonb");

    assert_eq!(term().parse("jsonb").into_result(), Ok(expected));
}

#[test]
fn parse_fields_returns_fields() {
    let expected = vec![Field::Title, Field::Abstract, Field::Keywords];
    assert_eq!(
        fields()
            .parse("fields title, abstract, keywords")
            .into_result(),
        Ok(expected)
    );
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
