use std::collections::HashMap;

use anyhow::Context;
use serde::Deserialize;
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

impl<'s> Destinations<'s> {
    fn new(content: &'s str) -> Self {
        Self {
            content,
            state: State::default(),
        }
    }

    pub fn definitions(&mut self) -> anyhow::Result<Vec<JumpDestination>> {
        let parsed_structure = self.raw_parse().context("parsing template")?;

        let mut destinations = Vec::new();

        let parsed_template = YamlLoader::load_from_str(self.content).expect("loading the yaml");

        for (line_number, line) in self.content.lines().enumerate() {
            let trimmed_line = line.trim();
            if trimmed_line == "Resources:" {
                self.state = State::ParsingResources;
                continue;
            } else if trimmed_line == "Outputs:" {
                self.state = State::ParsingOutputs;
                continue;
            } else if trimmed_line == "Mappings:" {
            } else if trimmed_line == "Parameters:" {
            }

            // we are not opening a new section
            match self.state {
                State::Init => todo!(),
                State::ParsingResources => {
                    let sanitised_line = line.trim().replace(":", "");
                    if parsed_structure.resources.contains_key(&sanitised_line) {
                        let span = span_from_line(line_number, line, &sanitised_line)
                            .context("constructing span")?;
                        let dest = JumpDestination {
                            name: sanitised_line.to_string(),
                            span,
                        };
                        destinations.push(dest);
                    }
                }
                State::ParsingOutputs => {
                    let sanitised_line = line.trim().replace(":", "");
                    if let Some(outputs) = &parsed_structure.outputs {
                        if outputs.contains_key(&sanitised_line) {
                            let span = span_from_line(line_number, line, &sanitised_line)
                                .context("constructing span")?;
                            let dest = JumpDestination {
                                name: sanitised_line.to_string(),
                                span,
                            };
                            destinations.push(dest);
                        }
                    }
                }
                State::ParsingParameters => todo!(),
                State::ParsingMappings => todo!(),
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
}

#[derive(Debug)]
pub struct Position {
    line: usize,
    col: usize,
}

#[derive(Debug)]
pub struct Span {
    start: Position,
    end: Position,
}

#[derive(Debug)]
pub struct JumpDestination {
    name: String,
    span: Span,
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
}
