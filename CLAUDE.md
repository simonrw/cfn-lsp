# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Language Server Protocol (LSP) implementation for AWS CloudFormation templates, providing IDE features like autocomplete, go-to-definition, hover information, and validation for CloudFormation YAML/JSON files.

## Project Structure

This is a Rust workspace with three crates:

- **cfn-lsp**: Main LSP server implementation using tower-lsp
- **cfn-lsp-schema**: CloudFormation resource schema parsing and code generation from AWS schema bundles
- **cfn-docs**: CLI tool for viewing CloudFormation resource documentation in the terminal

## Common Commands

### Building and Testing

```bash
# Build all crates
cargo build

# Build with verbose output
cargo build --verbose

# Run all tests
cargo test

# Run tests with verbose output
cargo test --verbose

# Run a specific test
cargo test <test_name>

# Run tests in a specific crate
cargo test -p cfn-lsp

# Run tests for a specific module
cargo test queries::tests
```

### Linting

```bash
# Run clippy linter
cargo clippy

# Run clippy on all targets
cargo clippy --all-targets

# Apply clippy fixes automatically
cargo clippy --fix
```

### Test Snapshots

The project uses `insta` for snapshot testing. When tests fail due to snapshot mismatches:

```bash
# Review snapshot changes
cargo insta review

# Accept all snapshot changes
cargo insta accept
```

## Architecture

### LSP Server (cfn-lsp)

The LSP server provides:
- **Autocomplete**: Resource type completion when typing `Type:` in YAML or `"Type":` in JSON
- **Go-to-definition**: Jump to resource/parameter/output/mapping definitions via `Destinations`
- **Hover**: Show resource documentation from schema

Key modules:
- `main.rs`: LSP server setup with tower-lsp, implements `LanguageServer` trait
- `destinations.rs`: Extracts jump targets (Resources, Outputs, Parameters, Mappings) by parsing template structure
- `queries.rs`: Tree-sitter based query system for extracting CloudFormation intrinsic function references (Ref, Sub, etc.)
- `queries/*.scm`: Tree-sitter query files for pattern matching YAML/JSON structures

### Schema System (cfn-lsp-schema)

Processes AWS CloudFormation schema bundles (`CloudformationSchema.zip`) to generate Rust code containing resource type information, descriptions, and IAM permissions. The `render_to` function generates a large match expression mapping resource types to `ResourceInfo` structs.

### Tree-sitter Integration

Uses tree-sitter-yaml and tree-sitter-json parsers with custom query files (`.scm`) to extract CloudFormation intrinsic function references. Query captures are named like `@fn.target` and `@tag.target` to identify reference targets.

### Testing

Test data files are in `crates/cfn-lsp/testdata/`. Tests use snapshot testing with `insta` to verify parsing results remain consistent.

## Git Submodule

The repository includes `cfn-lint` as a git submodule (aws-cloudformation/cfn-lint), likely used as a reference for test cases.

## Development Notes

- Edition: Rust 2024
- Main LSP feature: On file open/save, the server parses the template to extract jump destinations for go-to-definition
- State management: ServerState uses Arc<Mutex<ServerStateInner>> to share document state and jump destinations across async handlers
- Text synchronization: Full document sync (TextDocumentSyncKind::FULL) - the entire document is sent on changes
