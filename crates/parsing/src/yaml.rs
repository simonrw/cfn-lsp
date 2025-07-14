use crate::Targets;

pub(crate) fn parse(content: &str) -> std::result::Result<Targets, crate::ParsingError> {
    Ok(Targets {})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let contents = include_str!("../../cfn-lsp/testdata/simple.yml");
        let targets = parse(&contents).expect("parsing file for targets");
        assert_eq!(targets, Targets {})
    }
}
