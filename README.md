# querygen

A small DSL for building search queries for scientific databases.
Used as a spike to test chumsky, ariadne and clap.

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

With more than one target, each search string is preceded by a `# <database-id>`
header line, and a blank line separates them. Duplicate targets are ignored.

Exit codes: `0` on success, `1` if the query file can't be read or parsed (or
the output can't be written), `2` on invalid command-line usage.

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