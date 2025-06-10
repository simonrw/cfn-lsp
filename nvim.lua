-- vim.lsp.start({
-- 	name = "cfn-lsp",
-- 	cmd = { "./cfn-lsp" },
-- 	root_dir = vim.fn.getcwd(),
-- })
vim.lsp.config["cfn-lsp"] = {
    cmd = { "./cfn-lsp" },
    filetypes = { "yaml" },
}

vim.lsp.enable("cfn-lsp")
