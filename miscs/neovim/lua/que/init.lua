local M = {}

M.completion_icons = {
  Text = "󰉿",
  Method = "󰆧",
  Function = "󰊕",
  Constructor = "",
  Field = "󰜢",
  Variable = "󰀫",
  Class = "󰠱",
  Interface = "",
  Module = "",
  Property = "󰜢",
  Unit = "󰑭",
  Value = "󰎠",
  Enum = "",
  Keyword = "󰌋",
  Snippet = "",
  Color = "󰏘",
  File = "󰈙",
  Reference = "󰈇",
  Folder = "󰉋",
  EnumMember = "",
  Constant = "󰏿",
  Struct = "󰙅",
  Event = "",
  Operator = "󰆕",
  TypeParameter = "󰊄",
}

local function root_dir(fname)
  local ok, util = pcall(require, "lspconfig.util")
  if not ok then
    return nil
  end
  return util.root_pattern("que.toml", ".git")(fname)
end

local function completion_kind(kind, icons)
  local icon = icons[kind]
  if not icon then
    return kind
  end
  return icon .. " " .. kind
end

local function setup_cmp_completion(bufnr, icons)
  local ok, cmp = pcall(require, "cmp")
  if not ok then
    return false
  end
  if vim.b[bufnr].que_cmp_icons_configured then
    return true
  end

  local config = cmp.get_config and cmp.get_config() or {}
  local previous = config.formatting and config.formatting.format
  cmp.setup.buffer({
    formatting = {
      format = function(entry, item)
        if previous then
          item = previous(entry, item) or item
        end
        item.kind = completion_kind(item.kind, icons)
        return item
      end,
    },
  })
  vim.b[bufnr].que_cmp_icons_configured = true
  return true
end

local function setup_builtin_completion(client, bufnr, icons)
  if not (vim.lsp.completion and vim.lsp.completion.enable) then
    return
  end
  vim.opt.completeopt:append({ "menuone", "noselect", "popup" })
  vim.lsp.completion.enable(true, client.id, bufnr, {
    autotrigger = true,
    convert = function(item)
      local kind = vim.lsp.protocol.CompletionItemKind[item.kind] or "Text"
      return {
        abbr = item.label,
        kind = completion_kind(kind, icons),
      }
    end,
  })
end

local function setup_completion(client, bufnr, opts)
  if opts.completion_icons == false then
    return
  end
  local icons = vim.tbl_extend("force", M.completion_icons, opts.completion_icons or {})
  if not setup_cmp_completion(bufnr, icons) then
    setup_builtin_completion(client, bufnr, icons)
  end
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

  local user_on_attach = opts.on_attach
  local lsp_opts = vim.deepcopy(opts)
  lsp_opts.completion_icons = nil
  lsp_opts.on_attach = function(client, bufnr)
    setup_completion(client, bufnr, opts)
    if user_on_attach then
      user_on_attach(client, bufnr)
    end
  end

  lspconfig.quelsp.setup(vim.tbl_deep_extend("force", {
    cmd = opts.cmd or { "quelsp" },
    filetypes = opts.filetypes or { "que", "eclisp" },
    root_dir = opts.root_dir or root_dir,
    single_file_support = true,
  }, lsp_opts))
end

return M
