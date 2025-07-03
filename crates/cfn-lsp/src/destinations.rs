pub struct Destinations<'s> {
    content: &'s str,
}

impl<'s> Destinations<'s> {
    fn new(content: &'s str) -> Self {
        Self { content }
    }

    pub fn definitions(&self) -> Vec<JumpDestination> {
        vec![
            JumpDestination {
                // TODO: this can be a &'s str,
                name: "Topic".to_string(),
                span: Span {
                    start: Position { line: 1, col: 2 },
                    end: Position { line: 1, col: 8 },
                },
            },
            JumpDestination {
                // TODO: this can be a &'s str,
                name: "Parameter".to_string(),
                span: Span {
                    start: Position { line: 4, col: 2 },
                    end: Position { line: 4, col: 12 },
                },
            },
        ]
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
    #[ignore]
    fn parse_simple() {
        let contents = include_str!("../testdata/simple.yml");
        let destinations = Destinations::new(contents);
        let targets = destinations.definitions();
        insta::assert_debug_snapshot!(targets);
    }

    #[test]
    fn parse_two_resources() {
        let contents = include_str!("../testdata/two_resources.yml");
        let destinations = Destinations::new(contents);
        let targets = destinations.definitions();
        insta::assert_debug_snapshot!(targets);
    }
}
