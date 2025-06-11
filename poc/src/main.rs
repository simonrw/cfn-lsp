use std::collections::HashMap;

use yaml_rust::Event;

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

fn main() {
    let contents = std::fs::read_to_string("./template.yml").unwrap();
    let mut loader = CloudformationParser::default();
    let mut parser = yaml_rust::parser::Parser::new(contents.chars());
    parser.load(&mut loader, true).unwrap();
}
