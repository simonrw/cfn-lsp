use anyhow::Context;
#[cfg(test)]
use serde::Serialize;
use tree_sitter::{Node, Parser, Query, QueryCursor, StreamingIterator, Tree};

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
struct Position {
    line: usize,
    col: usize,
}

impl Position {
    fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl From<tree_sitter::Point> for Position {
    fn from(value: tree_sitter::Point) -> Self {
        Self {
            line: value.row,
            col: value.column,
        }
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
struct Ref {
    target: String,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
struct Sub {
    target: String,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
struct Reference {
    typ: ReferenceType,
    start: Position,
    end: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
enum ReferenceType {
    Ref(Ref),
    Sub(Sub),
}

struct Extractor {
    tree: Tree,
}

impl Extractor {
    fn new(content: &str) -> anyhow::Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_yaml::LANGUAGE.into())
            .context("Error loading Rust grammar")?;
        let tree = parser.parse(content, None).context("parsing text")?;
        Ok(Self { tree })
    }

    fn extract_refs(&self, content: &str) -> anyhow::Result<Vec<Reference>> {
        let root_node = self.tree.root_node();

        let query_contents = include_str!("queries/ref.scm");
        let query = Query::new(&tree_sitter_yaml::LANGUAGE.into(), query_contents)
            .context("parsing query")?;
        let capture_names = query.capture_names();

        let mut cursor = QueryCursor::new();

        let mut out = Vec::new();

        let mut matches = cursor.matches(&query, root_node, content.as_bytes());
        while let Some(m) = matches.next() {
            for capture in m.captures {
                let capture_name = capture_names[capture.index as usize];
                if !capture_name.ends_with(".target") {
                    continue;
                }

                let node_text = capture.node.utf8_text(content.as_bytes())?;
                let node = capture.node;

                let r = Ref {
                    target: node_text.to_string(),
                };
                let reference = Reference {
                    typ: ReferenceType::Ref(r),
                    start: node.start_position().into(),
                    end: node.end_position().into(),
                };
                out.push(reference);
            }
        }

        Ok(out)
    }

    fn extract_subs(&self, content: &str) -> anyhow::Result<Vec<Reference>> {
        let root_node = self.tree.root_node();

        let query_contents = include_str!("queries/sub.scm");
        let query = Query::new(&tree_sitter_yaml::LANGUAGE.into(), query_contents)
            .context("parsing query")?;
        let capture_names = query.capture_names();

        let mut cursor = QueryCursor::new();

        let mut out = Vec::new();

        let mut matches = cursor.matches(&query, root_node, content.as_bytes());
        while let Some(m) = matches.next() {
            for capture in m.captures {
                let capture_name = capture_names[capture.index as usize];
                if !capture_name.ends_with(".target") {
                    continue;
                }

                let node_text = capture.node.utf8_text(content.as_bytes())?;
                let node = capture.node;

                // Strip surrounding quotes from the captured string
                let target = node_text.trim_matches('"').to_string();

                // Adjust positions to exclude the surrounding quotes
                let mut start = node.start_position();
                let mut end = node.end_position();
                start.column += 1; // Skip opening quote
                end.column -= 1; // Skip closing quote

                let r = Sub { target };
                let reference = Reference {
                    typ: ReferenceType::Sub(r),
                    start: start.into(),
                    end: end.into(),
                };
                out.push(reference);
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod refs {
        use super::*;

        #[test]
        fn extract_from_outputs() -> anyhow::Result<()> {
            let contents = std::fs::read_to_string("testdata/outputs.yml").unwrap();
            let extractor = Extractor::new(&contents)?;
            let refs = extractor
                .extract_refs(&contents)
                .context("extracting refs")?;
            insta::assert_yaml_snapshot!(refs);
            Ok(())
        }

        #[test]
        fn extract_from_parameters() -> anyhow::Result<()> {
            let contents = std::fs::read_to_string("testdata/parameters.yml").unwrap();
            let extractor = Extractor::new(&contents)?;
            let refs = extractor
                .extract_refs(&contents)
                .context("extracting refs")?;
            insta::assert_yaml_snapshot!(refs);
            Ok(())
        }

        #[test]
        fn extract_from_two_resources() -> anyhow::Result<()> {
            let contents = std::fs::read_to_string("testdata/two_resources.yml").unwrap();
            let extractor = Extractor::new(&contents)?;
            let refs = extractor
                .extract_refs(&contents)
                .context("extracting refs")?;
            insta::assert_yaml_snapshot!(refs);
            Ok(())
        }

        #[test]
        fn extract_from_template() -> anyhow::Result<()> {
            let contents = std::fs::read_to_string("testdata/template.yml").unwrap();
            let extractor = Extractor::new(&contents)?;
            let refs = extractor
                .extract_refs(&contents)
                .context("extracting refs")?;
            insta::assert_yaml_snapshot!(refs);
            Ok(())
        }
    }

    mod subs {
        use super::*;

        #[test]
        fn extract_from_subs() -> anyhow::Result<()> {
            let contents = std::fs::read_to_string("testdata/subs.yml").unwrap();
            let extractor = Extractor::new(&contents)?;
            let refs = extractor
                .extract_subs(&contents)
                .context("extracting refs")?;
            insta::assert_yaml_snapshot!(refs);
            Ok(())
        }
    }
}
