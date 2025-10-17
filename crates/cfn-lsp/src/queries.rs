use anyhow::Context;
#[cfg(test)]
use serde::Serialize;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
pub(crate) struct Position {
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
pub(crate) struct Ref {
    pub(crate) target: String,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
pub(crate) struct Sub {
    pub(crate) target: String,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
pub(crate) struct GetAtt {
    pub(crate) target: String,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
pub(crate) struct FindInMap {
    pub(crate) target: String,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
pub(crate) struct If {
    pub(crate) target: String,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
pub(crate) struct DependsOn {
    pub(crate) target: String,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
pub(crate) struct Reference {
    pub(crate) typ: ReferenceType,
    pub(crate) start: Position,
    pub(crate) end: Position,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Serialize))]
pub(crate) enum ReferenceType {
    Ref(Ref),
    Sub(Sub),
    GetAtt(GetAtt),
    FindInMap(FindInMap),
    If(If),
    DependsOn(DependsOn),
}

pub(crate) struct Extractor {
    tree: Tree,
}

impl Extractor {
    pub(crate) fn new(content: &str) -> anyhow::Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_yaml::LANGUAGE.into())
            .context("Error loading Rust grammar")?;
        let tree = parser.parse(content, None).context("parsing text")?;
        Ok(Self { tree })
    }

    pub(crate) fn extract_refs(&self, content: &str) -> anyhow::Result<Vec<Reference>> {
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

    pub(crate) fn extract_subs(&self, content: &str) -> anyhow::Result<Vec<Reference>> {
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

    pub(crate) fn extract_getatts(&self, content: &str) -> anyhow::Result<Vec<Reference>> {
        let root_node = self.tree.root_node();

        let query_contents = include_str!("queries/getatt.scm");
        let query = Query::new(&tree_sitter_yaml::LANGUAGE.into(), query_contents)
            .context("parsing query")?;
        let capture_names = query.capture_names();

        let mut cursor = QueryCursor::new();

        let mut out = Vec::new();

        let mut matches = cursor.matches(&query, root_node, content.as_bytes());
        while let Some(m) = matches.next() {
            for capture in m.captures {
                let capture_name = capture_names[capture.index as usize];

                // Handle different capture patterns
                if capture_name.ends_with(".target") {
                    // For Fn::GetAtt: [Name, Property] or block sequence form
                    let node_text = capture.node.utf8_text(content.as_bytes())?;
                    let node = capture.node;

                    let target = node_text.to_string();

                    let reference = Reference {
                        typ: ReferenceType::GetAtt(GetAtt { target }),
                        start: node.start_position().into(),
                        end: node.end_position().into(),
                    };
                    out.push(reference);
                } else if capture_name.ends_with(".value") {
                    // For !GetAtt Name.Property form
                    let node_text = capture.node.utf8_text(content.as_bytes())?;
                    let node = capture.node;

                    // Extract just the Name part (before the dot)
                    let target = node_text.split('.').next().unwrap_or(node_text).to_string();

                    let mut end = node.start_position();
                    end.column += target.len();

                    let reference = Reference {
                        typ: ReferenceType::GetAtt(GetAtt { target }),
                        start: node.start_position().into(),
                        end: end.into(),
                    };
                    out.push(reference);
                }
            }
        }

        Ok(out)
    }

    pub(crate) fn extract_findinmaps(&self, content: &str) -> anyhow::Result<Vec<Reference>> {
        let root_node = self.tree.root_node();

        let query_contents = include_str!("queries/findinmap.scm");
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

                let target = node_text.to_string();

                let reference = Reference {
                    typ: ReferenceType::FindInMap(FindInMap { target }),
                    start: node.start_position().into(),
                    end: node.end_position().into(),
                };
                out.push(reference);
            }
        }

        Ok(out)
    }

    pub(crate) fn extract_ifs(&self, content: &str) -> anyhow::Result<Vec<Reference>> {
        let root_node = self.tree.root_node();

        let query_contents = include_str!("queries/if.scm");
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

                let target = node_text.to_string();

                let reference = Reference {
                    typ: ReferenceType::If(If { target }),
                    start: node.start_position().into(),
                    end: node.end_position().into(),
                };
                out.push(reference);
            }
        }

        Ok(out)
    }

    pub(crate) fn extract_dependsons(&self, content: &str) -> anyhow::Result<Vec<Reference>> {
        let root_node = self.tree.root_node();

        let query_contents = include_str!("queries/dependson.scm");
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

                let target = node_text.to_string();

                let reference = Reference {
                    typ: ReferenceType::DependsOn(DependsOn { target }),
                    start: node.start_position().into(),
                    end: node.end_position().into(),
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

    mod getatts {
        use super::*;

        #[test]
        fn extract_from_getatt() -> anyhow::Result<()> {
            let contents = std::fs::read_to_string("testdata/getatt.yml").unwrap();
            let extractor = Extractor::new(&contents)?;
            let refs = extractor
                .extract_getatts(&contents)
                .context("extracting getatts")?;
            insta::assert_yaml_snapshot!(refs);
            Ok(())
        }
    }

    mod findinmaps {
        use super::*;

        #[test]
        fn extract_from_findinmap() -> anyhow::Result<()> {
            let contents = std::fs::read_to_string("testdata/findinmap.yml").unwrap();
            let extractor = Extractor::new(&contents)?;
            let refs = extractor
                .extract_findinmaps(&contents)
                .context("extracting findinmaps")?;
            insta::assert_yaml_snapshot!(refs);
            Ok(())
        }
    }

    mod ifs {
        use super::*;

        #[test]
        fn extract_from_if() -> anyhow::Result<()> {
            let contents = std::fs::read_to_string("testdata/if.yml").unwrap();
            let extractor = Extractor::new(&contents)?;
            let refs = extractor.extract_ifs(&contents).context("extracting ifs")?;
            insta::assert_yaml_snapshot!(refs);
            Ok(())
        }
    }

    mod dependsons {
        use super::*;

        #[test]
        fn extract_from_dependson() -> anyhow::Result<()> {
            let contents = std::fs::read_to_string("testdata/dependson.yml").unwrap();
            let extractor = Extractor::new(&contents)?;
            let refs = extractor
                .extract_dependsons(&contents)
                .context("extracting dependsons")?;
            insta::assert_yaml_snapshot!(refs);
            Ok(())
        }
    }
}
