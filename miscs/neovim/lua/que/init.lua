local M = {}

local function root_dir(fname)
  local ok, util = pcall(require, "lspconfig.util")
  if not ok then
    return nil
  end
  return util.root_pattern("que.toml", ".git")(fname)
end

function M.setup(opts)
  opts = opts or {}

  local ok_lspconfig, lspconfig = pcall(require, "lspconfig")
  if not ok_lspconfig then
    error("que.nvim requires nvim-lspconfig")
  end

  local ok_configs, configs = pcall(require, "lspconfig.configs")
  if not ok_configs then
    error("que.nvim requires lspconfig.configs")
  end

  if not configs.quelsp then
    configs.quelsp = {
      default_config = {
        cmd = opts.cmd or { "quelsp" },
        filetypes = opts.filetypes or { "que", "eclisp" },
        root_dir = opts.root_dir or root_dir,
        single_file_support = true,
      },
    }
  end

  lspconfig.quelsp.setup(vim.tbl_deep_extend("force", {
    cmd = opts.cmd or { "quelsp" },
    filetypes = opts.filetypes or { "que", "eclisp" },
    root_dir = opts.root_dir or root_dir,
    single_file_support = true,
  }, opts))
end

return M
