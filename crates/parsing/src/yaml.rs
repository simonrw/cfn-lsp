use std::collections::HashSet;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::{Location, ParsingError, Range, Targets};

macro_rules! build_queries {
    ($($name:expr),+) => {{
        let mut query = String::new();
        $(query.push_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/queries/", $name)));)+
        query
    }}
}

pub(crate) fn parse(content: &str) -> std::result::Result<Targets, crate::ParsingError> {
    eprintln!("{content}");
    let tree = {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_yaml::LANGUAGE.into())
            .map_err(ParsingError::SettingLanguage)?;
        parser.parse(content.as_bytes(), None)
    }
    .unwrap();
    let root_node = tree.root_node();

    let destinations = {
        let query_content = build_queries!(
            // "targets/resources.scm",
            // "targets/outputs.scm",
            // "targets/parameters.scm"
            "targets/query.scm"
        );
        eprintln!("{query_content}");
        let query = Query::new(&tree_sitter_yaml::LANGUAGE.into(), &query_content).unwrap();
        let mut query_cursor = QueryCursor::new();
        let mut captures = query_cursor.captures(&query, root_node, content.as_bytes());

        // TODO: work out why we get duplicates
        let mut destinations = HashSet::new();
        while let Some((mat, idx)) = captures.next() {
            for capture in mat.captures {
                if capture.index == 0 {
                    continue;
                }
                let node = capture.node;
                let node_text = node.utf8_text(content.as_bytes()).unwrap();
                dbg!(node_text, idx, capture.index);

                let location = Location {
                    name: node_text.to_string(),
                    range: Range::from_node(&node),
                };
                destinations.insert(location);
            }
        }
        let mut destinations: Vec<_> = destinations.into_iter().collect();
        destinations.sort();
        destinations
    };

    let sources = {
        let query_content = build_queries!("sources/fn_ref.scm");
        let query = Query::new(&tree_sitter_yaml::LANGUAGE.into(), &query_content).unwrap();
        let mut query_cursor = QueryCursor::new();
        let mut captures = query_cursor.captures(&query, root_node, content.as_bytes());

        // TODO: work out why we get duplicates
        let mut sources = HashSet::new();
        while let Some((mat, _)) = captures.next() {
            for capture in mat.captures.iter().filter(|c| c.index == 1) {
                let node = capture.node;
                let node_text = node.utf8_text(content.as_bytes()).unwrap();

                let location = Location {
                    name: node_text.to_string(),
                    range: Range::from_node(&node),
                };
                sources.insert(location);
            }
        }
        let mut sources: Vec<_> = sources.into_iter().collect();
        sources.sort();
        sources
    };

    Ok(Targets {
        destinations,
        sources,
    })
}

#[cfg(test)]
mod tests {
    use crate::{Location, Position, Range};

    use super::*;

    macro_rules! gen_test_for_template {
        ($name:ident, $template_name:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let contents = std::fs::read_to_string($template_name).unwrap();
                let targets = parse(&contents).expect("parsing file for targets");
                assert_eq!(targets, $expected)
            }
        };
    }

    gen_test_for_template!(
        parse_simple,
        "../cfn-lsp/testdata/simple.yml",
        Targets {
            destinations: vec![Location {
                name: "MyTopic".to_string(),
                range: Range {
                    start: Position { line: 1, column: 2 },
                    end: Position { line: 1, column: 9 },
                },
            },],
            sources: Vec::new(),
        }
    );

    gen_test_for_template!(
        parse_two_resources,
        "../cfn-lsp/testdata/two_resources.yml",
        Targets {
            destinations: vec![
                Location {
                    name: "Parameter".to_string(),
                    range: Range {
                        start: Position { line: 4, column: 2 },
                        end: Position {
                            line: 4,
                            column: 11
                        },
                    },
                },
                Location {
                    name: "Topic".to_string(),
                    range: Range {
                        start: Position { line: 1, column: 2 },
                        end: Position { line: 1, column: 7 },
                    },
                },
            ],
            sources: vec![Location {
                name: "Topic".to_string(),
                range: Range {
                    start: Position {
                        line: 8,
                        column: 18,
                    },
                    end: Position {
                        line: 8,
                        column: 23,
                    }
                },
            }],
        }
    );

    #[test]
    fn parse_with_references() {
        let contents = std::fs::read_to_string("../cfn-lsp/testdata/parameters.yml").unwrap();
        let targets = parse(&contents).expect("parsing file for targets");
        assert_eq!(
            targets,
            Targets {
                destinations: vec![
                    Location {
                        name: "MyParameter".to_string(),
                        range: Range {
                            start: Position { line: 1, column: 2 },
                            end: Position {
                                line: 1,
                                column: 13
                            },
                        },
                    },
                    Location {
                        name: "Topic".to_string(),
                        range: Range {
                            start: Position { line: 4, column: 2 },
                            end: Position { line: 4, column: 7 },
                        },
                    },
                ],
                sources: vec![Location {
                    name: "MyParameter".to_string(),
                    range: Range {
                        start: Position {
                            line: 7,
                            column: 24
                        },
                        end: Position {
                            line: 7,
                            column: 35
                        },
                    },
                }]
            }
        )
    }
}
