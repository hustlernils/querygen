# querygen

A small DSL for building search queries for scientific databases.
Used as a spike to evaluate chumsky, ariadne and clap.

Currently, search queries for ACM DL, IEEE Xplore, ScienceDirect and SpringerLink can be generated.
For ScienceDirect, paste the output into the "Title, abstract or author-specified
keywords" field of the Advanced Search form (the generator emits no field names).

A **facet** is one concept of the research question (e.g. population or
intervention, following Kitchenham & Charters' SLR guidelines), written as a
list of synonyms joined with OR. The query combines facets with AND/OR.

## Usage

Generate query for a single database:

```bash
querygen jsonb.query --target <database-id>
```

Example:

```bash
querygen jsonb.query --target acm
```

Generate query for multiple databases:

```bash
querygen jsonb.query --target <database-id>,<database-id>
```

Example:

```bash
querygen jsonb.query --target acm,ieee
```

Output is written to stdout by default. Use `-o, --output <file>` to write it
to a file instead.

### Example

```
fields title, abstract, keywords

facet data_model =
    json OR jsonb OR nosql OR "document store" OR "document database"
  OR "document-oriented" OR "semi-structured" OR schemaless OR "schema-less" OR mongodb

facet migration =
    "schema evolution" OR "schema migration" OR "schema change"
  OR "schema versioning" OR "data migration" OR "lazy migration" OR "database migration"

query data_model AND migration
```

## ScienceDirect

ScienceDirect's Advanced Search allows at most **8 Boolean connectors per
field** (error: "Use fewer boolean connectors (max 8 per field)"). Longer
queries are therefore split into sub-queries, written one per line. Paste each
one into the "Title, abstract or author-specified keywords" field, run it, and
export the results. Records found by several sub-queries have to be
deduplicated afterwards.

How the split works:

1. The query has to be an AND of groups, where each group is a facet or an OR
   of facets, e.g. `a AND (b OR c)`. An OR of facets
   is merged into one group of terms (`b ∪ c`).
2. Each group is split into chunks of nearly equal size. A sub-query takes one
   chunk per group and ANDs them; the sub-queries are the cross product of all
   chunks.
3. A sub-query with chunk sizes k₁…k_g has (k₁−1) + … + (k_g−1) ORs plus g−1
   ANDs, so k₁ + … + k_g ≤ 9 must hold. The chunk sizes are chosen to
   minimise the number of sub-queries.

Because AND distributes over OR, each combination of one term per group is in
exactly one sub-query, so **the union of the sub-queries matches exactly the
documents the full query matches**. Only the way the search is executed
differs from a single search string.

Example: for a query with 13 × (10 + 10) terms the chunk sizes are
5 and 4, giving 3 × 5 = 15 sub-queries with 8 connectors each.

## SpringerLink

SpringerLink's Advanced Search has no abstract or author-keyword field. It
offers a "Title" box and a "Keywords" box; despite its name, the latter searches
the whole record including full text. Like for ScienceDirect, the generator
emits only the Boolean expression (`AND`, `OR`, `()`, `""`), which can be pasted
into either box. If the query's `fields` include `abstract` or `keywords`, a
warning is printed to stderr, because neither box matches them exactly: this has
to be documented as a deviation.

No connector limit is known, so the query is not split.

## EBNF

```ebnf
Program    = Fields, { Facet }, Query ;
Fields     = "fields", Field, { ",", Field } ;
Field     = "title" | "abstract" | "keywords" ;
Facet      = "facet", Identifier, "=", Term, { "OR", Term } ;
Term       = Word | String ;
Query      = "query", OrExpr ;
OrExpr     = AndExpr, { "OR", AndExpr } ;
AndExpr    = Atom, { "AND", Atom } ;
Atom       = Identifier | "(", OrExpr, ")" ;
Identifier = Letter, { Letter | Digit | "_" } ;
Word       = Letter, { Letter | Digit | "_" } ;
String     = '"', { Character - '"' }, '"' ;
```