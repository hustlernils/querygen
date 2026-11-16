# querygen

A small DSL for building search queries for scientific databases.
Used as a spike to test chumsky, ariadne and clap.

A **facet** is one concept of the research question (e.g. population or
intervention, following Kitchenham & Charters' SLR guidelines), written as a
list of synonyms joined with OR. The query combines facets with AND/OR.

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