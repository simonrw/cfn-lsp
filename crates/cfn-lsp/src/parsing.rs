use std::{collections::HashMap, fmt::Debug, hash::Hash, io::Read, path::Path};

use anyhow::Context;
use yaml_rust::Event;

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

#[derive(Debug, Clone)]
enum State {
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

struct CloudformationParser {
    template: Template,
    state_stack: Vec<State>,
    key_stack: Vec<LocatedString>,
    map_start_stack: Vec<yaml_rust::scanner::Marker>,
    is_key: bool,
    temp_resource: Option<Resource>,
    temp_parameter: Option<Parameter>,
    temp_output: Option<Output>,
    temp_properties_stack: Vec<HashMap<LocatedString, Located<ResourceProperties>>>,
}

impl Default for CloudformationParser {
    fn default() -> Self {
        CloudformationParser {
            template: Template::default(),
            state_stack: vec![State::Start],
            key_stack: Vec::new(),
            map_start_stack: Vec::new(),
            is_key: true,
            temp_resource: None,
            temp_parameter: None,
            temp_output: None,
            temp_properties_stack: Vec::new(),
        }
    }
}

impl CloudformationParser {
    fn get_template(self) -> Template {
        self.template
    }

    fn located_scalar(&self, s: String, mark: &yaml_rust::scanner::Marker) -> LocatedString {
        let line = mark.line();
        let col = mark.col();
        Located::new_at(s.clone(), span!(line, col, line, col + s.len()))
    }
}

impl yaml_rust::parser::MarkedEventReceiver for CloudformationParser {
    fn on_event(&mut self, ev: Event, mark: yaml_rust::scanner::Marker) {
        let state = self.state_stack.last().unwrap().clone();

        match ev {
            Event::StreamStart | Event::StreamEnd => {}
            Event::DocumentStart => self.state_stack.push(State::Doc),
            Event::DocumentEnd => {
                self.state_stack.pop();
            }
            Event::MappingStart(_) => {
                self.map_start_stack.push(mark);
                self.is_key = true;

                let next_state = match state {
                    State::Doc => State::Root,
                    State::Root => match self.key_stack.last().unwrap().value.as_str() {
                        "Resources" => State::InResources,
                        "Parameters" => State::InParameters,
                        "Outputs" => State::InOutputs,
                        _ => state,
                    },
                    State::InResources => {
                        self.temp_resource = Some(Resource {
                            r#type: Located::new_at("".to_string(), span!()),
                            properties: None,
                        });
                        State::InResource
                    }
                    State::InParameters => {
                        self.temp_parameter = Some(Parameter {
                            r#type: Located::new_at("".to_string(), span!()),
                            default: None,
                        });
                        State::InParameter
                    }
                    State::InOutputs => {
                        self.temp_output = Some(Output {
                            value: Located::new_at("".to_string(), span!()),
                        });
                        State::InOutput
                    }
                    State::InResource | State::InParameter | State::InOutput => {
                        if self.key_stack.last().unwrap().value == "Properties" {
                            self.temp_properties_stack.push(HashMap::new());
                            State::InProperties(vec![])
                        } else {
                            state
                        }
                    }
                    State::InProperties(mut path) => {
                        path.push(self.key_stack.last().unwrap().clone());
                        self.temp_properties_stack.push(HashMap::new());
                        State::InProperties(path)
                    }
                    State::Start => todo!(),
                };
                self.state_stack.push(next_state);
            }
            Event::MappingEnd => {
                let start_mark = self.map_start_stack.pop().unwrap();
                let map_range = span!(start_mark.line(), start_mark.col(), mark.line(), mark.col());

                let old_state = self.state_stack.pop().unwrap();
                self.is_key = true;

                match old_state {
                    State::InResource => {
                        let resource = self.temp_resource.take().unwrap();
                        let mut resource_name = self.key_stack.pop().unwrap();
                        resource_name.range = map_range;
                        let located_resource = Located::new_at(resource, map_range);
                        self.template
                            .resources
                            .insert(resource_name, located_resource);
                    }
                    State::InParameter => {
                        let parameter = self.temp_parameter.take().unwrap();
                        let mut parameter_name = self.key_stack.pop().unwrap();
                        parameter_name.range = map_range;
                        let located_parameter = Located::new_at(parameter, map_range);
                        todo!()
                    }
                    State::InOutput => {
                        let output = self.temp_output.take().unwrap();
                        let mut output_name = self.key_stack.pop().unwrap();
                        output_name.range = map_range;
                        let located_output = Located::new_at(output, map_range);
                        self.template.outputs.insert(output_name, located_output);
                    }
                    State::InProperties(path) => {
                        let properties_map = self.temp_properties_stack.pop().unwrap();
                        let properties = ResourceProperties::Map(properties_map);
                        let located_properties = Located::new_at(properties, map_range);

                        if path.is_empty() {
                            if let Some(resource) = self.temp_resource.as_mut() {
                                resource.properties = Some(located_properties);
                            }
                        } else {
                            let parent_properties = self.temp_properties_stack.last_mut().unwrap();
                            let mut property_key = self.key_stack.pop().unwrap();
                            property_key.range = map_range;
                            parent_properties.insert(property_key, located_properties);
                        }
                    }
                    _ => {}
                }
            }
            Event::Scalar(value, ..) => {
                if self.is_key {
                    self.key_stack.push(self.located_scalar(value, &mark));
                } else {
                    let key = self.key_stack.pop().unwrap();
                    let located_value = self.located_scalar(value, &mark);

                    match state {
                        State::Root => {
                            if key.value == "AWSTemplateFormatVersion" {
                                self.template.version = Some(located_value);
                            } else if key.value == "Description" {
                                self.template.description = Some(located_value);
                            }
                        }
                        State::InResource => {
                            if key.value == "Type" {
                                self.temp_resource.as_mut().unwrap().r#type = located_value;
                            }
                        }
                        State::InParameter => {
                            let param = self.temp_parameter.as_mut().unwrap();
                            if key.value == "Type" {
                                param.r#type = located_value;
                            } else if key.value == "Default" {
                                param.default = Some(located_value);
                            }
                        }
                        State::InOutput => {
                            if key.value == "Value" {
                                self.temp_output.as_mut().unwrap().value = located_value;
                            }
                        }
                        State::InProperties(_) => {
                            let properties = ResourceProperties::String(located_value.clone());
                            let located_properties =
                                Located::new_at(properties, located_value.range);
                            self.temp_properties_stack
                                .last_mut()
                                .unwrap()
                                .insert(key, located_properties);
                        }
                        _ => {}
                    }
                }
                self.is_key = !self.is_key;
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
        let example_contents = "Resources:
    MyTopic:
        Type: AWS::SNS::Topic
";
        let template =
            parse_cfn_yaml_from_reader(std::io::Cursor::new(example_contents.trim())).unwrap();

        // TODO: support serialize for yaml snapshots
        insta::assert_debug_snapshot!(template);
    }
}
