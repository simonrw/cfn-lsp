use std::{collections::HashMap, ops::ControlFlow};

use anyhow::{Context, Result};
use async_lsp::{
    ClientSocket, LanguageServer, ResponseError,
    lsp_types::{
        GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
        HoverProviderCapability, InitializeParams, InitializeResult, OneOf, Position, Range,
        ServerCapabilities, request::GotoDefinition,
    },
    router::Router,
};
use futures::future::BoxFuture;
use tower::ServiceBuilder;
use tracing::Level;
use yaml_rust::Event;

// parsing

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

fn parse_cfn() {
    let contents = std::fs::read_to_string("./template.yml").unwrap();
    let mut loader = CloudformationParser::default();
    let mut parser = yaml_rust::parser::Parser::new(contents.chars());
    parser.load(&mut loader, true).unwrap();
}

// lsp

struct ServerState {
    client: ClientSocket,
    counter: i32,
}

impl LanguageServer for ServerState {
    type Error = ResponseError;

    #[doc = r" Should always be defined to `ControlFlow<Result<()>>` for user implementations."]
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn initialize(
        &mut self,
        _params: InitializeParams,
    ) -> BoxFuture<'static, Result<InitializeResult, Self::Error>> {
        Box::pin(async move {
            Ok(InitializeResult {
                capabilities: ServerCapabilities {
                    hover_provider: Some(HoverProviderCapability::Simple(true)),
                    definition_provider: Some(OneOf::Left(true)),
                    ..Default::default()
                },
                server_info: None,
            })
        })
    }

    fn hover(
        &mut self,
        _params: HoverParams,
    ) -> BoxFuture<'static, Result<Option<Hover>, Self::Error>> {
        let contents = HoverContents::Markup(async_lsp::lsp_types::MarkupContent {
            kind: async_lsp::lsp_types::MarkupKind::Markdown,
            value: "This is a hover response".to_string(),
        });

        let hover = async_lsp::lsp_types::Hover {
            contents,
            range: None,
        };

        Box::pin(async move { Ok(Some(hover)) })
    }

    fn definition(
        &mut self,
        params: GotoDefinitionParams,
    ) -> BoxFuture<'static, Result<Option<GotoDefinitionResponse>, Self::Error>> {
        let res = GotoDefinitionResponse::Scalar(async_lsp::lsp_types::Location {
            uri: params.text_document_position_params.text_document.uri,
            range: Range {
                start: Position {
                    line: 1,
                    character: 2,
                },
                end: Position {
                    line: 1,
                    character: 3,
                },
            },
        });
        Box::pin(async move { Ok(Some(res)) })
    }
}

impl ServerState {
    fn new_router(client: ClientSocket) -> Router<Self> {
        let mut router = Router::from_language_server(Self { client, counter: 0 });
        router.event(Self::on_tick);
        router
    }

    fn on_tick(&mut self, _: TickEvent) -> ControlFlow<async_lsp::Result<()>> {
        tracing::info!("tick");
        self.counter += 1;
        ControlFlow::Continue(())
    }
}

struct TickEvent;

// main

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut log_file = std::fs::File::create("/tmp/server.log").context("creating log file")?;
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

        ServiceBuilder::new().service(ServerState::new_router(client))
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
