use tree_sitter::Node;

#[derive(Hash, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Hash, Debug, PartialEq, Eq, Clone, Copy)]
pub struct Range {
    pub start: Position,
    pub end: Position,
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

#[derive(Hash, Debug, PartialEq, Eq, Clone)]
pub struct Location {
    pub name: String,
    pub range: Range,
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Targets {
    pub destinations: Vec<Location>,
}
