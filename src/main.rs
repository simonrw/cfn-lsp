use std::{collections::HashMap, io::Write, ops::ControlFlow, time::Duration};

use anyhow::{Context, Result};
use async_lsp::{
    ClientSocket, LanguageServer, ResponseError,
    client_monitor::ClientProcessMonitorLayer,
    concurrency::ConcurrencyLayer,
    lsp_types::{
        DidChangeTextDocumentParams, DidOpenTextDocumentParams, GotoDefinitionParams,
        GotoDefinitionResponse, Hover, HoverContents, HoverParams, HoverProviderCapability,
        InitializeParams, InitializeResult, MarkupContent, MarkupKind, OneOf, Position, Range,
        ServerCapabilities, ServerInfo, TextDocumentItem, notification, request,
    },
    panic::CatchUnwindLayer,
    router::Router,
    server::LifecycleLayer,
    tracing::TracingLayer,
};
use futures::future::BoxFuture;
use spanned_json_parser::SpannedValue;
use tower::ServiceBuilder;
use tracing::Level;
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

// lsp

struct ServerState {
    client: ClientSocket,
    jump_targets: Vec<JumpTarget>,
    current_document: Option<TextDocumentItem>,
}

struct TickEvent;

#[derive(Debug, Clone)]
struct JumpTarget {
    name: String,
    location: Range,
    r#type: TargetType,
}

#[derive(Debug, Clone, Copy)]
enum TargetType {
    Resource,
    Parameter,
    Mapping,
}

fn extract_jump_targets(
    objects: &HashMap<String, SpannedValue>,
    target_type: TargetType,
) -> Vec<JumpTarget> {
    let mut targets = Vec::new();
    for (name, value) in objects {
        let range = Range {
            start: Position {
                line: value.start.line as u32,
                character: value.start.col as u32,
            },
            end: Position {
                line: value.end.line as u32,
                character: value.end.col as u32,
            },
        };
        targets.push(JumpTarget {
            name: name.to_string(),
            location: range,
            r#type: target_type,
        });
    }
    targets
}

// main
fn parse_json_main() {
    let contents = std::fs::read_to_string("template.json").expect("reading template json");
    let parsed = spanned_json_parser::parse(&contents).unwrap();

    // extract reference targets
    let mut targets = Vec::<JumpTarget>::new();
    let template = parsed.value.unwrap_object();

    let resources = template["Resources"].value.unwrap_object();
    targets.extend(extract_jump_targets(resources, TargetType::Resource));
    if let Some(parameters) = template.get("Parameters") {
        targets.extend(extract_jump_targets(
            parameters.value.unwrap_object(),
            TargetType::Parameter,
        ));
    }
    if let Some(mappings) = template.get("Mappings") {
        targets.extend(extract_jump_targets(
            mappings.value.unwrap_object(),
            TargetType::Mapping,
        ));
    }

    // TODO extract jump sources
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_file = std::fs::File::create("/tmp/server.log").context("creating log file")?;
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_ansi(false)
        .with_writer(log_file)
        .init();

    let (server, _) = async_lsp::MainLoop::new_server(|client| {
        // generate tick events
        tokio::spawn({
            let client = client.clone();
            async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    if client.emit(TickEvent).is_err() {
                        break;
                    }
                }
            }
        });

        let mut router = Router::new(ServerState {
            client: client.clone(),
            jump_targets: Vec::new(),
            current_document: None,
        });
        router
            .request::<request::Initialize, _>(|_, _params| async move {
                tracing::debug!("server initialized");
                Ok(InitializeResult {
                    capabilities: ServerCapabilities {
                        hover_provider: Some(HoverProviderCapability::Simple(true)),
                        definition_provider: Some(OneOf::Left(true)),
                        ..Default::default()
                    },
                    server_info: Some(ServerInfo {
                        name: "cfn-lsp".to_string(),
                        version: Some("0.0.1".to_string()),
                    }),
                })
            })
            .request::<request::HoverRequest, _>(|_, params| async move {
                tracing::debug!(?params, "got hover request");
                Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::PlainText,
                        value: "Hover information".to_string(),
                    }),
                    range: None,
                }))
            })
            .notification::<notification::Initialized>(|_, params| {
                tracing::info!(initialized_params = ?params, "server initialized");
                ControlFlow::Continue(())
            })
            .notification::<notification::DidSaveTextDocument>(|_, params| {
                tracing::debug!(save_params = ?params, "document saved");
                ControlFlow::Continue(())
            })
            .notification::<notification::DidChangeTextDocument>(|_, params| {
                tracing::debug!(change_params = ?params, "document changed");
                ControlFlow::Continue(())
            })
            .notification::<notification::DidOpenTextDocument>(|_, params| {
                tracing::debug!(open_params = ?params, "document did open");
                ControlFlow::Continue(())
            })
            .event::<TickEvent>(|_st, _| {
                tracing::debug!("tick");
                ControlFlow::Continue(())
            });
        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(LifecycleLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .layer(ClientProcessMonitorLayer::new(client))
            .service(router)
    });

    let (stdin, stdout) = (
        async_lsp::stdio::PipeStdin::lock_tokio().unwrap(),
        async_lsp::stdio::PipeStdout::lock_tokio().unwrap(),
    );
    tracing::info!("starting server");
    server
        .run_buffered(stdin, stdout)
        .await
        .context("running server")?;

    Ok(())
}
