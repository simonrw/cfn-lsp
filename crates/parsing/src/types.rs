use std::fmt::Debug;

use tree_sitter::Node;

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Debug for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Debug for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}-{:?}", self.start, self.end)
    }
}

impl Range {
    pub(crate) fn from_node(node: &Node<'_>) -> Range {
        let node_range = node.range();
        Range {
            start: Position {
                line: node_range.start_point.row,
                column: node_range.start_point.column,
            },
            end: Position {
                line: node_range.end_point.row,
                column: node_range.end_point.column,
            },
        }
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct Location {
    pub name: String,
    pub range: Range,
}

impl Debug for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Location {}@{:?}>", self.name, self.range)
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Targets {
    pub destinations: Vec<Location>,
    pub sources: Vec<Location>,
}
