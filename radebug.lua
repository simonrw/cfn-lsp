vim.lsp.config('rust_analyzer', {
    cmd = {"go", "run", "./proxy.go", "rust-analyzer", "--log-file", "/tmp/ra.log", "-v", "-v", "-v", "--no-log-buffering" },
    root_markers = {"Cargo.toml", ".git"},
    filetypes = {"rust"},
})

vim.lsp.enable("rust_analyzer")
