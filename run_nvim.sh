#!/usr/bin/env bash

set -euo pipefail

function main() {
    cargo build
    nvim --cmd 'source nvim.lua' template.json
}

main "$@"

