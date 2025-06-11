#!/usr/bin/env bash

set -euo pipefail

function main() {
    cargo build --manifest-path ./poc/Cargo.toml
    nvim --cmd 'source nvim.lua' template.yml
}

main "$@"

