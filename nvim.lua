-- vim.lsp.start({
-- 	name = "cfn-lsp",
-- 	cmd = { "./cfn-lsp" },
-- 	root_dir = vim.fn.getcwd(),
-- })
local target_dir = os.getenv("CARGO_TARGET_DIR") or "."
vim.lsp.config["cfn-lsp"] = {
    cmd = { target_dir .. "/target/debug/cfn-lsp" },
    filetypes = { "yaml", "json" },
    -- cmd_env = { RUST_LOG = "debug" },
}

vim.lsp.enable("cfn-lsp")
