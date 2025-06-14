#!/usr/bin/env bash

set -euo pipefail

function main() {
    rm -f wrapper.log
    nvim -u radebug.lua crates/cfn-lsp/src/main.rs $@
}

main "$@"

