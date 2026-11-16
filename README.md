# querygen

A small DSL for building search queries for scientific databases.
Used as a spike to test chunksy, ariadne and clap.

## EBNF

```ebnf
Program    = Fields, { Block }, Query ;
Fields     = "fields", Field, { ",", Field } ;
Field     = "title" | "abstract" | "keywords" ;
Block      = "block", Identifier, "=", Term, { "OR", Term } ;
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

block data_model =
    json OR jsonb OR nosql OR "document store" OR "document database"
  OR "document-oriented" OR "semi-structured" OR schemaless OR "schema-less" OR mongodb

block migration =
    "schema evolution" OR "schema migration" OR "schema change"
  OR "schema versioning" OR "data migration" OR "lazy migration" OR "database migration"

query data_model AND migration
```