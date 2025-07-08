#!/usr/bin/env bash

set -euo pipefail

function main() {
    cargo build -p cfn-lsp
    nvim --cmd 'source nvim.lua' crates/cfn-lsp/testdata/template.yml
}

main "$@"

