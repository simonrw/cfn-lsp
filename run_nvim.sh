#!/usr/bin/env bash

set -euo pipefail

function main() {
    nvim --cmd 'source nvim.lua' template.yml
}

main "$@"

