local M = {}

M.completion_icons = {
  Text = "≡",
  Method = "ƒ",
  Function = "λ",
  Constructor = "+",
  Field = "·",
  Variable = "α",
  Class = "C",
  Interface = "I",
  Module = "m",
  Property = "·",
  Unit = "()",
  Value = "◇",
  Enum = "E",
  Keyword = "β",
  Snippet = "⋯",
  Color = "■",
  File = "#",
  Reference = "↗",
  Folder = "/*",
  EnumMember = "◇",
  Constant = "π",
  Struct = "{}",
  Event = "!",
  Operator = "●",
  TypeParameter = "T",
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
  return icon
end

local function setup_blink_completion(icons)
  local ok, config = pcall(require, "blink.cmp.config")
  if not ok or not config.appearance or not config.appearance.kind_icons then
    return false
  end
  for kind, icon in pairs(icons) do
    config.appearance.kind_icons[kind] = icon
  end
  return true
end

local function setup_cmp_completion(bufnr, icons, show_types)
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
        local detail = entry.completion_item and entry.completion_item.detail
        if show_types and detail and detail ~= "" then
          item.menu = detail
        end
        return item
      end,
    },
  })
  vim.b[bufnr].que_cmp_icons_configured = true
  return true
end

local function setup_builtin_completion(client, bufnr, icons, show_types)
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
        menu = show_types and item.detail or nil,
      }
    end,
  })
end

local function setup_completion(client, bufnr, opts)
  if opts.completion_icons == false then
    return
  end
  local icons = vim.tbl_extend("force", M.completion_icons, opts.completion_icons or {})
  local show_types = opts.completion_type_hints ~= false
  if setup_blink_completion(icons) then
    return
  end
  if not setup_cmp_completion(bufnr, icons, show_types) then
    setup_builtin_completion(client, bufnr, icons, show_types)
  end
end

local function blink_handles_signature_help()
  local ok, config = pcall(require, "blink.cmp.config")
  return ok and config.signature and config.signature.enabled == true
end

local function setup_signature_help(client, bufnr, opts)
  if not client.server_capabilities.signatureHelpProvider then
    return
  end
  if opts.signature_help == false or (opts.signature_help == nil and blink_handles_signature_help()) then
    return
  end
  local group = vim.api.nvim_create_augroup("QueSignatureHelp" .. bufnr, { clear = true })
  vim.api.nvim_create_autocmd("InsertCharPre", {
    group = group,
    buffer = bufnr,
    callback = function()
      if vim.v.char ~= " " and vim.v.char ~= "(" then
        return
      end
      vim.schedule(function()
        if vim.api.nvim_get_current_buf() == bufnr and vim.fn.mode():sub(1, 1) == "i" then
          vim.lsp.buf.signature_help()
        end
      end)
    end,
  })
end

local function clean_que_hover(result)
  if not (result and type(result.contents) == "table") then
    return result
  end
  if result.contents.kind ~= "markdown" or type(result.contents.value) ~= "string" then
    return result
  end
  local value = result.contents.value
  value = value:gsub("```que\r?\n(.-)\r?\n```", "%1")
  value = value:gsub("`([^`\r\n]+)`", "%1")
  value = value:gsub("\r?\n%s*\r?\n", "\n")
  value = value:gsub("^%s+", ""):gsub("%s+$", "")
  value = value:gsub("([^\r\n]+)", " %1 ")
  result.contents.value = value
  result.contents.kind = "plaintext"
  return result
end

local function setup_hover(client, bufnr, hover_handler, opts)
  vim.keymap.set("n", "K", function()
    local params = vim.lsp.util.make_position_params(0, client.offset_encoding)
    client:request("textDocument/hover", params, function(err, result, ctx, config)
      config = vim.tbl_extend("force", config or {}, {
        border = opts.hover_border or "single",
      })
      local hover_buf, hover_win = hover_handler(err, clean_que_hover(result), ctx, config)
      if hover_buf and vim.api.nvim_buf_is_valid(hover_buf) then
        vim.bo[hover_buf].syntax = ""
      end
      if hover_win and vim.api.nvim_win_is_valid(hover_win) then
        vim.wo[hover_win].winhighlight = "Normal:NormalFloat,FloatBorder:FloatBorder"
      end
    end, bufnr)
  end, { buffer = bufnr, desc = "Que hover" })
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
  local user_handlers = opts.handlers or {}
  local hover_handler = user_handlers["textDocument/hover"] or vim.lsp.handlers.hover
  local lsp_opts = vim.deepcopy(opts)
  lsp_opts.completion_icons = nil
  lsp_opts.completion_type_hints = nil
  lsp_opts.signature_help = nil
  lsp_opts.hover_border = nil
  lsp_opts.handlers = vim.tbl_extend("force", user_handlers, {
    ["textDocument/hover"] = function(err, result, ctx, config)
      return hover_handler(err, clean_que_hover(result), ctx, config)
    end,
  })
  lsp_opts.on_attach = function(client, bufnr)
    setup_completion(client, bufnr, opts)
    setup_signature_help(client, bufnr, opts)
    setup_hover(client, bufnr, hover_handler, opts)
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
