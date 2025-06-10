package handlers

import (
	"github.com/tliron/commonlog"
	"github.com/tliron/glsp"
	protocol "github.com/tliron/glsp/protocol_3_16"
)

func TextDocumentDidSave(ctx *glsp.Context, params *protocol.DidSaveTextDocumentParams)  error {
	commonlog.NewInfoMessage(0, "document saved")
	return nil
}
