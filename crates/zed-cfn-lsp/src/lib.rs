use zed_extension_api as zed;

struct CloudFormationLanguageServer {}

impl zed::Extension for CloudFormationLanguageServer {
    fn new() -> Self
    where
        Self: Sized,
    {
        todo!()
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        Ok(zed::Command {
            command: get_path_to_language_server_executable()?,
            args: get_args_for_language_server()?,
            env: get_env_for_language_server()?,
        })
    }
}

fn get_env_for_language_server() -> zed::Result<Vec<(String, String)>> {
    todo!()
}

fn get_args_for_language_server() -> zed::Result<Vec<String>> {
    todo!()
}

fn get_path_to_language_server_executable() -> zed::Result<String> {
    todo!()
}

zed::register_extension!(CloudFormationLanguageServer);
