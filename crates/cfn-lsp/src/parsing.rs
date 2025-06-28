use std::{collections::HashMap, fmt::Debug, hash::Hash, io::Read, path::Path};

use anyhow::Context;
use yaml_rust::{Event, parser::MarkedEventReceiver, scanner::Marker};

macro_rules! span {
    ($start_line:expr, $start_col:expr, $end_line:expr, $end_col:expr) => {
        Range {
            start: Position {
                line: $start_line,
                column: $start_col,
            },
            end: Position {
                line: $end_line,
                column: $end_col,
            },
        }
    };
    () => {
        span!(0, 0, 0, 0)
    };
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct Position {
    line: usize,
    column: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct Range {
    start: Position,
    end: Position,
}

#[derive(Debug)]
struct Located<T>
where
    T: Debug,
{
    value: T,
    range: Range,
}

impl<T> Located<T>
where
    T: Debug,
{
    fn new_at(value: T, range: Range) -> Self {
        Located { value, range }
    }
}

impl<T> PartialEq for Located<T>
where
    T: PartialEq + Debug,
{
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.range == other.range
    }
}

impl<T> Eq for Located<T> where T: Eq + Debug {}

impl<T> Clone for Located<T>
where
    T: Debug + Clone,
{
    fn clone(&self) -> Self {
        Located {
            value: self.value.clone(),
            range: self.range,
        }
    }
}

impl<T> Hash for Located<T>
where
    T: Hash + Debug,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
        self.range.hash(state);
    }
}

type LocatedString = Located<String>;

// parsing
#[derive(Debug, Clone, Default, Eq, PartialEq)]
struct Template {
    version: Option<LocatedString>,
    description: Option<LocatedString>,
    resources: HashMap<LocatedString, Located<Resource>>,
    outputs: HashMap<LocatedString, Located<Output>>,
    parameters: HashMap<LocatedString, Located<Parameter>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResourceProperties {
    Map(HashMap<LocatedString, Located<ResourceProperties>>),
    String(LocatedString),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Resource {
    r#type: LocatedString,
    properties: Option<Located<ResourceProperties>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Output {
    value: LocatedString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Parameter {
    r#type: LocatedString,
    default: Option<LocatedString>,
}

#[derive(Debug, Clone, Default)]
enum State {
    #[default]
    Start,
    Doc,
    Root,
    InResources,
    InResource,
    InParameters,
    InParameter,
    InOutputs,
    InOutput,
    InProperties(Vec<LocatedString>),
}

struct Node {
    r#type: NodeType,
    range: Range,
}

enum NodeType {
    Map(HashMap<String, Node>),
    Array(Vec<Node>),
    String(String),
}

#[derive(Default)]
struct CloudformationParser {
    // template: Template,
    // state_stack: Vec<State>,
    // key_stack: Vec<LocatedString>,
    // map_start_stack: Vec<Marker>,
    // is_key: bool,
    // temp_resource: Option<Resource>,
    // temp_parameter: Option<Parameter>,
    // temp_output: Option<Output>,
    // temp_properties_stack: Vec<HashMap<LocatedString, Located<ResourceProperties>>>,
    state: State,
    node_stack: Vec<Node>,
    template: Template,
}

impl CloudformationParser {
    fn get_template(self) -> Template {
        self.template
    }
}
//     fn located_scalar(&self, s: String, mark: &Marker) -> LocatedString {
//         let line = mark.line();
//         let col = mark.col();
//         Located::new_at(s.clone(), span!(line, col, line, col + s.len()))
//     }
// }

/*
StreamStart | Marker { index: 0, line: 1, col: 0 }
DocumentStart | Marker { index: 9, line: 1, col: 9 }
MappingStart(0) | Marker { index: 9, line: 1, col: 9 }
Scalar("Resources", Plain, 0, None) | Marker { index: 0, line: 1, col: 0 }
MappingStart(0) | Marker { index: 20, line: 2, col: 9 }
Scalar("MyTopic", Plain, 0, None) | Marker { index: 13, line: 2, col: 2 }
MappingStart(0) | Marker { index: 30, line: 3, col: 8 }
Scalar("Type", Plain, 0, None) | Marker { index: 26, line: 3, col: 4 }
Scalar("AWS::SNS::Topic", Plain, 0, None) | Marker { index: 32, line: 3, col: 10 }
MappingEnd | Marker { index: 47, line: 4, col: 0 }
MappingEnd | Marker { index: 47, line: 4, col: 0 }
MappingEnd | Marker { index: 47, line: 4, col: 0 }
DocumentEnd | Marker { index: 47, line: 4, col: 0 }
StreamEnd | Marker { index: 47, line: 4, col: 0 }
*/

impl MarkedEventReceiver for CloudformationParser {
    fn on_event(&mut self, ev: Event, mark: Marker) {
        let line = mark.line();
        let column = mark.line();
        match ev {
            Event::StreamStart => {},
            Event::StreamEnd => {},
            Event::DocumentStart => self.state = State::Doc,
            Event::DocumentEnd => todo!(),
            Event::MappingStart(_) => {
                self.node_stack.push(Node {
                    r#type: NodeType::Map(HashMap::new()),
                    range: Range {
                        start: Position { line, column },
                        end: Position { line: 0, column: 0 },
                    },
                });
            }
            Event::MappingEnd => {
                todo!()
                // loop {
                //     let last_node = self.node_stack.pop();

                // }
            }
            Event::Scalar(value, ..) => todo!(),
            Event::SequenceStart(..) | Event::SequenceEnd => {
                todo!()
            }
            _ => {}
        }
    }
}

fn parse_cfn_yaml_from_file(p: impl AsRef<Path>) -> anyhow::Result<Template> {
    let f = std::fs::File::open(p).context("opening specified file")?;
    parse_cfn_yaml_from_reader(f)
}

fn parse_cfn_yaml_from_reader(mut r: impl Read) -> anyhow::Result<Template> {
    let mut contents = String::new();
    let _ = r
        .read_to_string(&mut contents)
        .context("reading file contents")?;
    let mut loader = CloudformationParser::default();
    let mut parser = yaml_rust::parser::Parser::new(contents.chars());
    parser.load(&mut loader, true).context("parsing template")?;
    Ok(loader.get_template())
}

// fn parse_json_main() {
//     let contents = std::fs::read_to_string("template.json").expect("reading template json");
//     let parsed = spanned_json_parser::parse(&contents).unwrap();
//
//     // extract reference targets
//     let mut targets = Vec::<JumpTarget>::new();
//     let template = parsed.value.unwrap_object();
//
//     let resources = template["Resources"].value.unwrap_object();
//     targets.extend(extract_jump_targets(resources, TargetType::Resource));
//     if let Some(parameters) = template.get("Parameters") {
//         targets.extend(extract_jump_targets(
//             parameters.value.unwrap_object(),
//             TargetType::Parameter,
//         ));
//     }
//     if let Some(mappings) = template.get("Mappings") {
//         targets.extend(extract_jump_targets(
//             mappings.value.unwrap_object(),
//             TargetType::Mapping,
//         ));
//     }
//
//     // TODO extract jump sources
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_template() {
        let example_contents = include_str!("../testdata/basic-template.yml");
        let template =
            parse_cfn_yaml_from_reader(std::io::Cursor::new(example_contents.trim())).unwrap();

        // TODO: support serialize for yaml snapshots
        insta::assert_debug_snapshot!(template);
    }
}
