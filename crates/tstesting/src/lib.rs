use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Position {
    line: u32,
    column: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Span {
    start: Position,
    end: Position,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct SpannedNode {
    span: Span,
    node: Node,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum Node {
    Sub {
        template: String,
        arguments: Option<HashMap<String, Node>>,
    },
}

fn parse(input: &str) -> Result<Vec<SpannedNode>, Box<dyn std::error::Error>> {
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn template_string(stub: impl AsRef<Path>) -> Result<String, Box<dyn std::error::Error>> {
        let path = dbg!(std::env::current_dir()?)
            .join("..")
            .join("..")
            .join("crates")
            .join("cfn-lsp")
            .join("testdata")
            .join(stub.as_ref());
        let content = std::fs::read_to_string(path)?;
        Ok(content)
    }

    #[test]
    fn it_works() -> Result<(), Box<dyn std::error::Error>> {
        let sub_inline = template_string("sub_inline.json")?;
        let intrinsics = parse(&sub_inline)?;
        assert_eq!(
            intrinsics,
            [SpannedNode {
                span: Span {
                    start: Position { line: 0, column: 0 },
                    end: Position { line: 0, column: 0 },
                },
                node: Node::Sub {
                    template: "${MyParameter}".to_string(),
                    arguments: None,
                },
            }]
        );
        Ok(())
    }
}
