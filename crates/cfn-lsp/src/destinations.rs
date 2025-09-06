use std::collections::HashMap;

use anyhow::Context;
use serde::Deserialize;
use tower_lsp::lsp_types::Position as LspPosition;
use tower_lsp::lsp_types::Range;
use yaml_rust::YamlLoader;

#[derive(Default)]
enum State {
    #[default]
    Init,
    ParsingResources,
    ParsingOutputs,
    ParsingParameters,
    ParsingMappings,
    // TODO: other template top level fields
}
pub struct Destinations<'s> {
    content: &'s str,
    state: State,
}

macro_rules! parse_line {
    ($line:ident, $line_number:expr, $parsed_structure:ident, $jump_type:expr, $field:ident => $destinations:ident, false) => {{
        let sanitised_line = $line.trim().replace(":", "");
        let values = &$parsed_structure.$field;
        if values.contains_key(&sanitised_line) {
            let span = span_from_line($line_number, $line, &sanitised_line)
                .context("constructing span")?;
            let dest = JumpDestination {
                name: sanitised_line.to_string(),
                r#type: $jump_type,
                span,
            };
            $destinations.push(dest);
        }
    }};
    ($line:ident, $line_number:expr, $parsed_structure:ident, $jump_type:expr, $field:ident => $destinations:ident, true) => {{
        let sanitised_line = $line.trim().replace(":", "");
        if let Some(values) = &$parsed_structure.$field {
            if values.contains_key(&sanitised_line) {
                let span = span_from_line($line_number, $line, &sanitised_line)
                    .context("constructing span")?;
                let dest = JumpDestination {
                    name: sanitised_line.to_string(),
                    r#type: $jump_type,
                    span,
                };
                $destinations.push(dest);
            }
        }
    }};
}

impl<'s> Destinations<'s> {
    pub fn new(content: &'s str) -> Self {
        Self {
            content,
            state: State::default(),
        }
    }

    pub fn definitions(&mut self) -> anyhow::Result<Vec<JumpDestination>> {
        let parsed_structure = self.raw_parse().context("parsing template")?;

        let mut destinations = Vec::new();

        let _parsed_template = YamlLoader::load_from_str(self.content).expect("loading the yaml");

        for (line_number, line) in self.content.lines().enumerate() {
            let trimmed_line = line.trim();
            if trimmed_line == "Resources:" {
                self.state = State::ParsingResources;
                continue;
            } else if trimmed_line == "Outputs:" {
                self.state = State::ParsingOutputs;
                continue;
            } else if trimmed_line == "Mappings:" {
                self.state = State::ParsingMappings;
                continue;
            } else if trimmed_line == "Parameters:" {
                self.state = State::ParsingParameters;
                continue;
            }

            // we are not opening a new section
            match self.state {
                State::Init => {}
                State::ParsingResources => {
                    parse_line!(line, line_number, parsed_structure, JumpDestinationType::Resource, resources => destinations, false);
                }
                State::ParsingOutputs => {
                    parse_line!(line, line_number, parsed_structure, JumpDestinationType::Output, outputs => destinations, true)
                }
                State::ParsingParameters => {
                    parse_line!(line, line_number, parsed_structure, JumpDestinationType::Parameter, parameters => destinations, true);
                }
                State::ParsingMappings => {
                    parse_line!(line, line_number, parsed_structure, JumpDestinationType::Mapping, mappings => destinations, true);
                }
            }
        }
        Ok(destinations)
    }

    fn raw_parse(&self) -> anyhow::Result<Template> {
        serde_yaml::from_str(self.content).context("parsing template")
    }
}

fn span_from_line(line_number: usize, line: &str, target: &str) -> anyhow::Result<Span> {
    for i in 0..(line.len() - target.len()) {
        if &line[i..(i + target.len())] == target {
            return Ok(Span {
                start: Position {
                    line: line_number,
                    col: i,
                },
                end: Position {
                    line: line_number,
                    col: i + target.len() - 1,
                },
            });
        }
    }

    Err(anyhow::anyhow!("programming error"))
}

#[derive(Deserialize)]
struct Template {
    #[serde(rename = "Resources")]
    resources: HashMap<String, serde_yaml::Value>,
    #[serde(rename = "Outputs")]
    outputs: Option<HashMap<String, serde_yaml::Value>>,
    #[serde(rename = "Mappings")]
    mappings: Option<HashMap<String, serde_yaml::Value>>,
    #[serde(rename = "Parameters")]
    parameters: Option<HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug)]
pub struct Position {
    line: usize,
    col: usize,
}

impl Position {
    fn to_lsp_position(&self) -> LspPosition {
        LspPosition {
            line: u32::try_from(self.line).expect("line larger than 32 bit integer"),
            character: u32::try_from(self.col).expect("column larger than 32 bit integer"),
        }
    }
}

#[derive(Debug)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn to_range(&self) -> Range {
        Range {
            start: self.start.to_lsp_position(),
            end: self.end.to_lsp_position(),
        }
    }
}

#[derive(Debug)]
pub enum JumpDestinationType {
    Resource,
    Parameter,
    Mapping,
    Output,
}

#[derive(Debug)]
pub struct JumpDestination {
    pub name: String,
    pub r#type: JumpDestinationType,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let contents = include_str!("../testdata/simple.yml");
        let mut destinations = Destinations::new(contents);
        let targets = destinations.definitions();
        insta::assert_debug_snapshot!(targets);
    }

    #[test]
    fn parse_two_resources() {
        let contents = include_str!("../testdata/two_resources.yml");
        let mut destinations = Destinations::new(contents);
        let targets = destinations.definitions();
        insta::assert_debug_snapshot!(targets);
    }

    #[test]
    fn parse_with_outputs() {
        let contents = include_str!("../testdata/outputs.yml");
        let mut destinations = Destinations::new(contents);
        let targets = destinations.definitions();
        insta::assert_debug_snapshot!(targets);
    }

    #[test]
    fn parse_parameters() {
        let contents = include_str!("../testdata/parameters.yml");
        let mut destinations = Destinations::new(contents);
        let targets = destinations.definitions();
        insta::assert_debug_snapshot!(targets);
    }
}
