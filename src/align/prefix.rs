use crate::statement::Statement;

/// Returns the positional common canonical prefix of two linear statement sequences.
pub fn prefix_len(realized: &[&Statement], code: &[Statement]) -> usize {
    realized
        .iter()
        .zip(code)
        .take_while(|(executed, incoming)| executed.canonical == incoming.canonical)
        .count()
}
