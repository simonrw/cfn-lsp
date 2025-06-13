#!/usr/bin/env bash

set -euo pipefail

function main() {
    cargo build -p cfn-lsp --release
    nvim --cmd 'source nvim.lua' template.json
}

main "$@"

