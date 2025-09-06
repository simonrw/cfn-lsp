use std::{path::Path, sync::Arc};

use anyhow::Context;
use tokio::sync::Mutex;
use tower_lsp::{
    Client, LanguageServer, LspService, Server,
    lsp_types::{
        CompletionOptions, CompletionParams, CompletionResponse, DidChangeTextDocumentParams,
        DidOpenTextDocumentParams, DidSaveTextDocumentParams, Documentation, GotoDefinitionParams,
        GotoDefinitionResponse, Hover, HoverContents, HoverParams, HoverProviderCapability,
        InitializeParams, InitializeResult, Location, MarkupContent, MarkupKind, OneOf, Position,
        ServerCapabilities, ServerInfo, TextDocumentItem, TextDocumentSyncCapability,
        TextDocumentSyncKind, Url,
    },
};
use tracing::Level;

use crate::destinations::{Destinations, JumpDestination};

mod destinations;

// lsp

struct ServerState {
    _client: Client,
    inner: Arc<Mutex<ServerStateInner>>,
}

impl ServerState {
    async fn set_current_document_from_url(&self, url: Url) {
        let Ok(path) = url.to_file_path() else {
            tracing::warn!(?url, "cannot be converted to path");
            return;
        };
        tracing::debug!(uri = %path.display(), "opened file");
        let Ok(contents) = std::fs::read_to_string(&path) else {
            tracing::warn!(path = %path.display(), "could not read file");
            return;
        };
        let mut inner = self.inner.lock().await;
        inner.current_document = Some(TextDocumentItem {
            uri: url,
            language_id: "".to_string(),
            version: 0,
            text: contents.clone(),
        });

        // also recompute the jump destinations
        let mut destinations = Destinations::new(&contents);
        match destinations.definitions() {
            Ok(destinations) => {
                tracing::debug!(?destinations, "extracted goto definition targets");
                inner.jump_destinations = destinations;
            }
            Err(e) => {
                tracing::warn!(error = %e, "error computing jump destinations");
            }
        }
    }
}

// free floating function to make testing easier
fn word_under_cursor(content: &str, cursor: Position) -> anyhow::Result<Option<String>> {
    let line_number = usize::try_from(cursor.line).unwrap();
    let character_number = usize::try_from(cursor.character).unwrap();

    let mut lines = content.split('\n');
    let current_line = lines
        .nth(line_number)
        .ok_or(anyhow::anyhow!("Line out of bounds"))?;

    // check if the character at the current position is a space or not. If so return None
    let current_char = current_line
        .chars()
        .nth(character_number)
        .ok_or(anyhow::anyhow!("character out of range"))?;
    if current_char.is_whitespace() {
        return Ok(None);
    }

    // find index of end of word
    let chars: Vec<_> = current_line.chars().collect();

    let mut start_index = character_number;
    while start_index > 0 {
        if chars[start_index].is_whitespace() {
            start_index += 1;
            break;
        }
        start_index -= 1;
    }

    let mut end_index = character_number;
    while end_index < current_line.len() {
        if chars[end_index].is_whitespace() {
            break;
        }
        end_index += 1;
    }

    Ok(Some(current_line[start_index..end_index].to_string()))
}

struct ServerStateInner {
    current_document: Option<TextDocumentItem>,
    jump_destinations: Vec<JumpDestination>,
}

impl ServerStateInner {
    async fn word_under_cursor(&self, cursor: Position) -> anyhow::Result<Option<String>> {
        let Some(doc) = &self.current_document else {
            return Ok(None);
        };

        word_under_cursor(&doc.text, cursor)
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for ServerState {
    async fn initialize(
        &self,
        params: InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        tracing::debug!(?params, "initializing server");
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        tracing::trace!(?params, "document opened");
        let uri = params.text_document.uri.clone();
        self.set_current_document_from_url(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        tracing::trace!(?params, "document changed");
        let mut inner = self.inner.lock().await;
        let Some(current_document) = inner.current_document.as_mut() else {
            tracing::warn!("no current document");
            return;
        };
        current_document.text = params
            .content_changes
            .first()
            .map_or(current_document.text.clone(), |change| change.text.clone());
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        tracing::trace!(?params, "document saved");
        let url = params.text_document.uri.clone();
        self.set_current_document_from_url(url).await;
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<CompletionResponse>> {
        tracing::debug!(?params, "got completion request");

        let uri = params.text_document_position.text_document.uri.clone();
        let file_path = uri.to_file_path().map_err(|_| {
            tracing::warn!(?uri, "cannot convert URI to file path");
            tower_lsp::jsonrpc::Error::invalid_params("Invalid URI")
        })?;
        // TODO: don't hold the mutex for the entire operation
        let inner = self.inner.lock().await;
        let Some(current_document) = inner.current_document.as_ref() else {
            tracing::warn!("no current document");
            return Ok(None);
        };
        let text = current_document.text.as_str();
        let template_language = detect_template_language(&file_path, text);
        tracing::debug!(?template_language, "detected template language");
        let pos = params.text_document_position.position;
        let line = current_document
            .text
            .lines()
            .nth(pos.line.try_into().unwrap())
            .unwrap_or_default();

        let should_complete = match template_language {
            TemplateLanguage::Yaml => {
                let prefix = "Type:";
                line.trim_start().starts_with(prefix)
            }
            TemplateLanguage::Json => {
                let prefix = "\"Type\":";
                line.trim_start().starts_with(prefix)
            }
        };

        if !should_complete {
            tracing::debug!(?line, ?pos, "not completing");
            return Ok(None);
        }

        let completion_items: Vec<_> = cfn_lsp_schema::get_resource_types()
            .iter()
            .map(|resource| tower_lsp::lsp_types::CompletionItem {
                label: resource.type_name.clone(),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::CLASS),
                // detail: Some(resource.type_name),
                documentation: resource.description.clone().map(Documentation::String),
                ..Default::default()
            })
            .collect();

        Ok(Some(CompletionResponse::Array(completion_items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        tracing::debug!(?params, "got goto definition request");
        let position = params.text_document_position_params.position;
        let inner = self.inner.lock().await;
        match inner.word_under_cursor(position).await {
            Ok(Some(word)) => {
                let mut candidates = Vec::new();
                for destination in &inner.jump_destinations {
                    if word == destination.name {
                        candidates.push(destination);
                    }
                }

                tracing::debug!(?candidates, "found match candidates");
                if candidates.len() == 0 {
                    return Ok(None);
                } else if candidates.len() > 1 {
                    todo!("Unhandled case with more than one target: {:?}", candidates);
                } else {
                    let location = Location {
                        uri: params.text_document_position_params.text_document.uri,
                        range: candidates[0].span.to_range(),
                    };
                    let result = GotoDefinitionResponse::Scalar(location);
                    tracing::debug!(?result, "returning jump response");
                    return Ok(Some(result));
                }
            }
            Ok(None) => todo!(),
            Err(e) => tracing::warn!(error = %e, "No completions found"),
        }
        tracing::debug!("no destinations found");
        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> tower_lsp::jsonrpc::Result<Option<Hover>> {
        tracing::debug!(?params, "got hover request");
        let pos = params.text_document_position_params.position;
        let inner = self.inner.lock().await;
        let Some(current_document) = inner.current_document.as_ref() else {
            tracing::warn!("no current document");
            return Ok(None);
        };
        let text = current_document.text.as_str();
        let mut lines = text.split('\n');
        let Some(line) = lines
            .nth(pos.line.try_into().unwrap())
            .map(ToString::to_string)
        else {
            return Ok(None);
        };

        let Some(resource_type) = extract_resource_type(&line, pos) else {
            tracing::warn!(%line, ?pos, "no resource name found");
            return Ok(None);
        };
        let resource_info =
            cfn_lsp_schema::extract_resource_from_bundle(&resource_type).map_err(|e| {
                tracing::warn!(%e, "error extracting resource info");
                tower_lsp::jsonrpc::Error::internal_error()
            })?;

        // tracing::info!(?pos, ?line, ?resource_type,

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: resource_info.description.unwrap_or_default(),
            }),
            range: None,
        }))
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TemplateLanguage {
    Yaml,
    Json,
}

fn detect_template_language(filename: impl AsRef<Path>, text: &str) -> TemplateLanguage {
    // heuristics based on file extension
    if filename.as_ref().extension().and_then(|s| s.to_str()) == Some("json") {
        return TemplateLanguage::Json;
    }
    if filename.as_ref().extension().and_then(|s| s.to_str()) == Some("yaml")
        || filename.as_ref().extension().and_then(|s| s.to_str()) == Some("yml")
    {
        return TemplateLanguage::Yaml;
    }

    // heuristics based on content
    if text.trim().starts_with('{') {
        TemplateLanguage::Json
    } else {
        TemplateLanguage::Yaml
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_file = std::fs::File::create("/tmp/server.log").context("creating log file")?;
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_ansi(false)
        .with_writer(log_file)
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| ServerState {
        _client: client,
        inner: Arc::new(Mutex::new(ServerStateInner {
            current_document: None,
            jump_destinations: Vec::new(),
        })),
    });
    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}

fn extract_resource_type(line: &str, position: Position) -> Option<String> {
    // extract position of resource reference in line
    for c in 0..line.len() - 1 {
        let trailing = line[c..]
            .chars()
            .take_while(|&c| !c.is_whitespace() && c != '"')
            .collect::<String>();
        if trailing.starts_with("AWS::") {
            let words: Vec<_> = trailing.split_whitespace().collect();
            let name = words[0];
            let range = (c, c + name.len());
            let character = position.character as usize;
            if character >= range.0 && character < range.1 {
                return Some(trailing.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_simple() {
        let line = "Type: AWS::SNS::Topic";
        let pos = Position::new(0, 8);

        let resource_type = extract_resource_type(line, pos).expect("extracting resource name");

        assert_eq!(resource_type, "AWS::SNS::Topic");
    }

    #[test]
    fn with_indent() {
        let line = "        Type: AWS::SNS::Topic";
        let pos = Position::new(0, 16);

        let resource_type = extract_resource_type(line, pos).expect("extracting resource name");

        assert_eq!(resource_type, "AWS::SNS::Topic");
    }

    #[test]
    fn extract_not_over() {
        let line = "Type: AWS::SNS::Topic";
        let pos = Position::new(0, 0);

        assert!(extract_resource_type(line, pos).is_none());
    }

    #[test]
    fn extract_past_end() {
        let line = "Type: AWS::SNS::Topic     ";
        let pos = Position::new(0, u32::try_from(line.len() - 1).unwrap());

        assert!(extract_resource_type(line, pos).is_none());
    }

    #[test]
    fn extract_from_json() {
        let line = r#""Type": "AWS::SNS::Topic",""#;
        let pos = Position::new(0, 10);

        assert_eq!(
            extract_resource_type(line, pos),
            Some("AWS::SNS::Topic".to_string())
        );
    }

    // tests for word under cursor
    #[test]
    fn single_word_under_cursor() {
        let content = "This is a test";
        //             0123456789

        for (c, expected) in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].into_iter().zip([
            Some("This".to_string()),
            Some("This".to_string()),
            Some("This".to_string()),
            Some("This".to_string()),
            None,
            Some("is".to_string()),
            Some("is".to_string()),
            None,
            Some("a".to_string()),
            None,
        ]) {
            let position = Position {
                line: 0,
                character: c,
            };

            assert_eq!(word_under_cursor(content, position).unwrap(), expected);
        }
    }

    #[test]
    fn larger_template() {
        let content = include_str!("../testdata/template.yml");
        let position = Position {
            line: 46,
            character: 24,
        };
        assert_eq!(
            word_under_cursor(content, position).unwrap(),
            Some("TrustedAccounts".to_string())
        );
    }
}
