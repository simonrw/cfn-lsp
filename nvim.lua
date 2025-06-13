-- vim.lsp.start({
-- 	name = "cfn-lsp",
-- 	cmd = { "./cfn-lsp" },
-- 	root_dir = vim.fn.getcwd(),
-- })
vim.lsp.config["cfn-lsp"] = {
    cmd = { "/Users/simon/.cargo-target/release/cfn-lsp" },
    filetypes = { "yaml", "json" },
    -- cmd_env = { RUST_LOG = "debug" },
}

vim.lsp.enable("cfn-lsp")
