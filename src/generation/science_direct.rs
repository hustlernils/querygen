use super::{QueryGenerator, enclose, generate_bare_facet, generate_term, separated};
use crate::ast::{Expr, Facet, Field, Program, Term};

/// ScienceDirect has no field syntax: the field is chosen in the
/// Advanced Search form ("Title, abstract or author-specified keywords"),
/// so only the Boolean expression of terms is emitted.
///
/// The form allows at most [`SCIENCEDIRECT_MAX_CONNECTORS`] Boolean
/// connectors per field, so the query is split into sub-queries (see
/// [`split_query`]).
pub struct ScienceDirect;

pub const SCIENCEDIRECT_MAX_CONNECTORS: usize = 8;

impl QueryGenerator for ScienceDirect {
    fn generate_facet(&self, out: &mut String, facet: &Facet, _fields: &[Field]) {
        generate_bare_facet(out, facet);
    }

    fn generate_all(&self, program: &Program) -> Result<Vec<String>, String> {
        let sub_queries = split_query(program, SCIENCEDIRECT_MAX_CONNECTORS)?;

        Ok(sub_queries
            .iter()
            .map(|groups| {
                let mut out = String::new();
                separated(&mut out, groups, " AND ", |out, terms| {
                    enclose(out, |out| {
                        separated(out, terms, " OR ", |out, term| generate_term(out, term))
                    });
                });
                out
            })
            .collect())
    }
}

/// A sub-query: an AND of groups, each group an OR of terms.
pub type SubQuery<'p, 'a> = Vec<Vec<&'p Term<'a>>>;

/// Splits the query into sub-queries with at most `max_connectors` Boolean
/// connectors each, such that their union is equivalent to the query.
pub fn split_query<'p, 'a>(
    program: &'p Program<'a>,
    max_connectors: usize,
) -> Result<Vec<SubQuery<'p, 'a>>, String> {
    let groups = query_groups(program)?;

    let max_terms = max_connectors + 1;
    if groups.len() > max_terms {
        return Err(format!(
            "the query ANDs {} groups, but at most {max_terms} fit into {max_connectors} connectors",
            groups.len()
        ));
    }

    let sizes: Vec<usize> = groups.iter().map(Vec::len).collect();
    let chunk_sizes = best_chunk_sizes(&sizes, max_terms);

    let chunked: Vec<Vec<&[&Term]>> = groups
        .iter()
        .zip(&chunk_sizes)
        .map(|(terms, &k)| balanced_chunks(terms, k))
        .collect();

    let mut sub_queries: Vec<SubQuery> = vec![Vec::new()];
    for chunks in &chunked {
        sub_queries = sub_queries
            .iter()
            .flat_map(|prefix| {
                chunks.iter().map(move |chunk| {
                    let mut next = prefix.clone();
                    next.push(chunk.to_vec());
                    next
                })
            })
            .collect();
    }

    Ok(sub_queries)
}

/// Turns the query into a list of ANDed groups of ORed terms.
fn query_groups<'p, 'a>(program: &'p Program<'a>) -> Result<Vec<Vec<&'p Term<'a>>>, String> {
    let top = match &program.query {
        Expr::And(children) => children.iter().collect(),
        other => vec![other],
    };

    top.into_iter()
        .map(|expr| {
            let mut terms = Vec::new();
            collect_or_terms(expr, program, &mut terms)?;
            Ok(terms)
        })
        .collect()
}

fn collect_or_terms<'p, 'a>(
    expr: &'p Expr<'a>,
    program: &'p Program<'a>,
    terms: &mut Vec<&'p Term<'a>>,
) -> Result<(), String> {
    match expr {
        Expr::Ref(name) => {
            let facet = program
                .facet(name)
                .ok_or_else(|| format!("facet `{name}` is not defined"))?;
            for term in &facet.terms {
                if !terms.contains(&term) {
                    terms.push(term);
                }
            }
            Ok(())
        }
        Expr::Or(children) => children
            .iter()
            .try_for_each(|child| collect_or_terms(child, program, terms)),
        Expr::And(_) => Err(
            "the query can only be split if it is an AND of facets or ORs of facets, \
             e.g. `a AND (b OR c)`"
                .to_string(),
        ),
    }
}

/// Picks a chunk size per group, with all sizes summing to at most
/// `max_terms`, that minimises the number of sub-queries. Ties go to the
/// first candidate found, i.e. larger chunks for earlier groups.
fn best_chunk_sizes(sizes: &[usize], max_terms: usize) -> Vec<usize> {
    fn search(
        sizes: &[usize],
        budget: usize,
        current: &mut Vec<usize>,
        count: usize,
        best: &mut Option<(usize, Vec<usize>)>,
    ) {
        let Some((&n, rest)) = sizes.split_first() else {
            if best
                .as_ref()
                .is_none_or(|(best_count, _)| count < *best_count)
            {
                *best = Some((count, current.clone()));
            }
            return;
        };

        // Every remaining group needs at least one term of the budget.
        let max_k = n.min(budget - rest.len());
        for k in (1..=max_k).rev() {
            current.push(k);
            search(rest, budget - k, current, count * n.div_ceil(k), best);
            current.pop();
        }
    }

    let mut best = None;
    search(sizes, max_terms, &mut Vec::new(), 1, &mut best);
    best.map(|(_, sizes)| sizes).unwrap_or_default()
}

/// Splits `items` into ceil(len / max) chunks of at most `max` items whose
/// sizes differ by at most one.
fn balanced_chunks<T>(items: &[T], max: usize) -> Vec<&[T]> {
    let count = items.len().div_ceil(max).max(1);
    let base = items.len() / count;
    let extra = items.len() % count;

    let mut chunks = Vec::with_capacity(count);
    let mut start = 0;
    for i in 0..count {
        let len = base + usize::from(i < extra);
        chunks.push(&items[start..start + len]);
        start += len;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::program;
    use chumsky::Parser;

    const SPLIT: &str = include_str!("../../test/split.query");

    fn sciencedirect(input: &str) -> Result<Vec<String>, String> {
        let ast = program().parse(input).into_result().unwrap();
        ScienceDirect.generate_all(&ast)
    }

    fn connectors(query: &str) -> usize {
        query.matches(" AND ").count() + query.matches(" OR ").count()
    }

    #[test]
    fn sciencedirect_emits_terms_without_fields() {
        let input = "fields title, abstract, keywords \
                        facet a = json OR \"document store\" facet b = x facet c = y \
                        query a AND (b OR c)";

        assert_eq!(
            sciencedirect(input).unwrap(),
            vec!["(json OR \"document store\") AND (x OR y)"]
        );
    }

    #[test]
    fn sciencedirect_splits_13_by_20_into_15_sub_queries() {
        let queries = sciencedirect(SPLIT).unwrap();

        assert_eq!(queries.len(), 15);
        for query in &queries {
            assert!(
                connectors(query) <= SCIENCEDIRECT_MAX_CONNECTORS,
                "too many connectors: {query}"
            );
        }
    }

    #[test]
    fn sciencedirect_sub_queries_cover_every_term_combination_exactly_once() {
        let ast = program().parse(SPLIT).into_result().unwrap();
        let sub_queries = split_query(&ast, SCIENCEDIRECT_MAX_CONNECTORS).unwrap();

        let first = &ast.facet("a").unwrap().terms;
        let other: Vec<&Term> = ["b", "c"]
            .iter()
            .flat_map(|name| &ast.facet(name).unwrap().terms)
            .collect();

        for a in first {
            for b in &other {
                let covering = sub_queries
                    .iter()
                    .filter(|groups| groups[0].contains(&a) && groups[1].contains(b))
                    .count();
                assert_eq!(covering, 1, "{a:?} AND {b:?} covered {covering} times");
            }
        }

        let pairs: usize = sub_queries.iter().map(|g| g[0].len() * g[1].len()).sum();
        assert_eq!(pairs, first.len() * other.len());
    }

    #[test]
    fn sciencedirect_keeps_small_query_in_one_piece() {
        let input = "fields title facet a = x OR y facet b = z query a AND b";

        assert_eq!(sciencedirect(input).unwrap(), vec!["(x OR y) AND (z)"]);
    }

    #[test]
    fn sciencedirect_rejects_and_nested_in_or() {
        let input = "fields title facet a = x facet b = y facet c = z query a OR b AND c";

        assert!(sciencedirect(input).is_err());
    }

    #[test]
    fn sciencedirect_rejects_undefined_facet() {
        let input = "fields title facet a = x query a AND missing";

        assert_eq!(
            sciencedirect(input).unwrap_err(),
            "facet `missing` is not defined"
        );
    }

    #[test]
    fn best_chunk_sizes_minimises_sub_queries() {
        assert_eq!(best_chunk_sizes(&[13, 20], 9), vec![5, 4]);
        assert_eq!(best_chunk_sizes(&[3, 2], 9), vec![3, 2]);
    }

    #[test]
    fn balanced_chunks_differ_by_at_most_one() {
        let items: Vec<u8> = (0..13).collect();
        let sizes: Vec<usize> = balanced_chunks(&items, 5).iter().map(|c| c.len()).collect();

        assert_eq!(sizes, vec![5, 4, 4]);
    }
}
