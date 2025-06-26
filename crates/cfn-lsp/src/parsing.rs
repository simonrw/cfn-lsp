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

#[derive(Default)]
struct CloudformationParser {
    template: Template,
}

impl CloudformationParser {
    fn get_template(self) -> Template {
        self.template
    }
}

impl yaml_rust::parser::MarkedEventReceiver for CloudformationParser {
    fn on_event(&mut self, ev: Event, mark: yaml_rust::scanner::Marker) {
        todo!()
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
        let example_contents = r#"\
Resources:
    MyTopic:
        Type: AWS::SNS::Topic
"#;
        let template = parse_cfn_yaml_from_reader(std::io::Cursor::new(example_contents)).unwrap();

        let my_topic = {
            let topic_type: LocatedString =
                Located::new_at("AWS::SNS::Topic".to_string(), span!(2, 14, 2, 28));

            let properties = None;
            let resource = Resource {
                r#type: topic_type,
                properties: properties,
            };
            Located::new_at(resource, span!(1, 4, 2, 28))
        };

        let resources = {
            let mut resources = HashMap::new();
            resources.insert(
                Located::new_at("MyTopic".to_string(), span!(1, 4, 2, 28)),
                my_topic,
            );
            resources
        };

        let expected = Template {
            resources,
            ..Default::default()
        };
        assert_eq!(template, expected);
    }
}
