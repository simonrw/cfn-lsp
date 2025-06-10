package handlers

import (
	"github.com/tliron/commonlog"
	"github.com/tliron/glsp"
	protocol "github.com/tliron/glsp/protocol_3_16"
)

func TextDocumentDefinition(ctx *glsp.Context, params *protocol.DefinitionParams)  (any, error) {
	commonlog.NewInfoMessage(0, "Definition requested")
	var location protocol.Location
	location.URI = params.TextDocument.URI
	location.Range = protocol.Range{
		Start: protocol.Position{
			Line: 2,
			Character: 0,
		},
		End: protocol.Position{
			Line: 2,
			Character: 3,
		},
	}

	return location, nil
}
