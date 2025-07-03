use yaml_rust::YamlLoader;

#[derive(Default)]
enum State {
    #[default]
    Init,
    ParsingResources,
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

    pub fn definitions(&mut self) -> Vec<JumpDestination> {
        let mut destinations = Vec::new();

        let parsed_template = YamlLoader::load_from_str(self.content).expect("loading the yaml");

        for line in self.content.lines() {
            if line.trim_start() == "Resources:" {
                self.state = State::ParsingResources;
                continue;
            }
        }
        destinations
    }
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
}
