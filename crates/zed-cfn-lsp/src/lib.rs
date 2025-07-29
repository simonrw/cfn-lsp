use zed_extension_api as zed;

struct CloudFormationLanguageServer {}

impl zed::Extension for CloudFormationLanguageServer {
    fn new() -> Self
    where
        Self: Sized,
    {
        todo!()
    }
}

zed::register_extension!(CloudFormationLanguageServer);
