use std::{collections::HashMap, ops::ControlFlow};

use anyhow::Context;
use async_lsp::{
    ClientSocket,
    lsp_types::{
        HoverProviderCapability, InitializeResult, OneOf, ServerCapabilities, notification, request,
    },
    router::Router,
};
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

struct TickEvent;

// main

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stderr)
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

        let mut router = Router::new(ServerState { client, counter: 0 });

        router
            .request::<request::Initialize, _>(|_, params| async move {
                tracing::info!(initialize = ?params, "Server initialized");
                Ok(InitializeResult {
                    capabilities: ServerCapabilities {
                        hover_provider: Some(HoverProviderCapability::Simple(true)),
                        definition_provider: Some(OneOf::Left(true)),
                        ..Default::default()
                    },
                    server_info: None,
                })
            })
            .notification::<notification::Initialized>(|_, _| ControlFlow::Continue(()))
            .event::<TickEvent>(|st, _| {
                st.counter += 1;
                ControlFlow::Continue(())
            });
        ServiceBuilder::new().service(router)
    });

    let (stdin, stdout) = (
        async_lsp::stdio::PipeStdin::lock_tokio().unwrap(),
        async_lsp::stdio::PipeStdout::lock_tokio().unwrap(),
    );
    server
        .run_buffered(stdin, stdout)
        .await
        .context("running server")?;

    Ok(())
}
