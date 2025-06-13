#!/usr/bin/env bash

set -euo pipefail

function main() {
    cargo build -p cfn-lsp
    nvim --cmd 'source nvim.lua' template.json
}

main "$@"

