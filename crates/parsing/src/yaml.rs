use std::collections::HashSet;

use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator};

use crate::{Location, Range, Targets};

macro_rules! build_queries {
    ($($name:expr),+) => {{
        let mut query = String::new();
        $(query.push_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/queries/", $name)));)+
        query
    }}
}

pub(crate) fn parse(content: &str) -> std::result::Result<Targets, crate::ParsingError> {
    let tree = {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_yaml::LANGUAGE.into());
        parser.parse(content.as_bytes(), None)
    }
    .unwrap();
    let root_node = tree.root_node();

    let query_content = build_queries!("resources.scm", "outputs.scm", "parameters.scm");
    let query = Query::new(&tree_sitter_yaml::LANGUAGE.into(), &query_content).unwrap();
    let mut query_cursor = QueryCursor::new();
    let mut captures = query_cursor.captures(&query, root_node, content.as_bytes());

    // TODO: work out why we get duplicates
    let mut destinations = HashSet::new();

    while let Some((mat, _)) = captures.next() {
        for capture in mat.captures.iter().filter(|c| c.index == 1) {
            let node = capture.node;
            let node_text = node.utf8_text(content.as_bytes()).unwrap();

            let location = Location {
                name: node_text.to_string(),
                range: Range::from_node(&node),
            };
            destinations.insert(location);
        }
    }

    Ok(Targets {
        destinations: destinations.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use crate::{Location, Position, Range};

    use super::*;

    #[test]
    fn parse_simple() {
        // let contents = include_str!("../../cfn-lsp/testdata/template.yml");
        let contents = include_str!("../../cfn-lsp/testdata/simple.yml");
        let targets = parse(contents).expect("parsing file for targets");
        assert_eq!(
            targets,
            Targets {
                destinations: vec![Location {
                    name: "MyTopic".to_string(),
                    range: Range {
                        start: Position { line: 1, column: 2 },
                        end: Position { line: 1, column: 9 },
                    },
                },]
            }
        )
    }
}
