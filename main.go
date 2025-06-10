package main

import (
	// "golsp/handlers"
	"log"

	"github.com/tliron/commonlog"
	"github.com/tliron/glsp"
	protocol "github.com/tliron/glsp/protocol_3_16"
	"github.com/tliron/glsp/server"

	_ "github.com/tliron/commonlog/simple"

	"github.com/simonrw/cfn-lsp/handlers"
)

const lsName = "CloudFormatin LSP"

var version = "0.0.1"
var handler protocol.Handler

func main() {
	commonlog.Configure(2, nil)

	handler = protocol.Handler{
		Initialize: initialize,
		Shutdown:   shutdown,
		// TextDocumentCompletion: handlers.TextDocumentCompletion,
		TextDocumentDefinition: handlers.TextDocumentDefinition,
		TextDocumentDidSave: handlers.TextDocumentDidSave,
	}

	server := server.NewServer(&handler, lsName, true)
	log.Fatal(server.RunStdio())
}

func initialize(ctx *glsp.Context, params *protocol.InitializeParams) (any, error) {
	commonlog.NewInfoMessage(0, "Initializing server...")

	capabilities := handler.CreateServerCapabilities()
	capabilities.DefinitionProvider = &protocol.DefinitionOptions{}

	return protocol.InitializeResult{
		Capabilities: capabilities,
		ServerInfo: &protocol.InitializeResultServerInfo{
			Name:    lsName,
			Version: &version,
		},
	}, nil
}

func shutdown(ctx *glsp.Context) error {
	return nil
}
