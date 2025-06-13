use std::collections::HashMap;

use yaml_rust::Event;

// parsing
#[derive(Debug)]
struct Template {
    version: Option<String>,
    description: Option<String>,
    resources: HashMap<String, Resource>,
}

#[derive(Debug)]
struct Resource {}

#[derive(Debug)]
struct Parameter {}

#[derive(Default)]
struct CloudformationParser {
    indent: usize,

    resources: HashMap<String, Resource>,
    parameters: HashMap<String, Parameter>,
}

impl yaml_rust::parser::MarkedEventReceiver for CloudformationParser {
    fn on_event(&mut self, ev: Event, mark: yaml_rust::scanner::Marker) {
        let prefix: String = (0..self.indent * 2).map(|_| ' ').collect();
        println!("{prefix}Got event {ev:?}, mark: {mark:?}");
        match ev {
            Event::MappingStart(_) => self.indent += 1,
            Event::MappingEnd => self.indent -= 1,
            Event::SequenceStart(_) => self.indent += 1,
            Event::SequenceEnd => self.indent -= 1,
            _ => {}
        }
    }
}

fn parse_cfn_yaml() {
    let contents = std::fs::read_to_string("./template.yml").unwrap();
    let mut loader = CloudformationParser::default();
    let mut parser = yaml_rust::parser::Parser::new(contents.chars());
    parser.load(&mut loader, true).unwrap();
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
