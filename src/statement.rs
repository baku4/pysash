use super::Range;
use super::canonical_statement::CanonicalStatement;
use super::statement_facts::StatementFacts;

/// A parsed top-level statement with its source range, identity, and conservative facts.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Statement {
    pub range: Range,
    pub canonical: CanonicalStatement,
    pub facts: StatementFacts,
}
