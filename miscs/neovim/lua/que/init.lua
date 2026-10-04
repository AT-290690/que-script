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

local function setup_format(bufnr, opts)
  local command = opts.format_cmd or { "que", "fmt", "--stdin" }
  local function format_buffer()
    local source = table.concat(vim.api.nvim_buf_get_lines(bufnr, 0, -1, false), "\n") .. "\n"
    local formatted = vim.fn.system(command, source)
    if vim.v.shell_error ~= 0 then
      vim.notify(formatted ~= "" and formatted or "Que formatter failed", vim.log.levels.ERROR)
      return
    end
    local cursor = vim.api.nvim_win_get_cursor(0)
    local lines = vim.split(formatted:gsub("\n$", ""), "\n", { plain = true })
    vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
    local last_line = math.max(1, #lines)
    cursor[1] = math.min(cursor[1], last_line)
    cursor[2] = math.min(cursor[2], #(lines[cursor[1]] or ""))
    vim.api.nvim_win_set_cursor(0, cursor)
  end
  pcall(vim.api.nvim_buf_del_user_command, bufnr, "QueFormat")
  vim.api.nvim_buf_create_user_command(bufnr, "QueFormat", format_buffer, {
    desc = "Format the current Que buffer",
  })
  if opts.format_key ~= false then
    vim.keymap.set("n", opts.format_key or "<leader>f", format_buffer, {
      buffer = bufnr,
      silent = true,
      desc = "Format Que buffer",
    })
  end
end

local function open_library_source(executable, name)
  local lines = vim.fn.systemlist({ executable, "--lib", "source", name })
  if vim.v.shell_error ~= 0 then
    vim.notify(table.concat(lines, "\n"), vim.log.levels.ERROR)
    return
  end
  vim.cmd("botright new")
  local bufnr = vim.api.nvim_get_current_buf()
  vim.bo[bufnr].buftype = "nofile"
  vim.bo[bufnr].bufhidden = "wipe"
  vim.bo[bufnr].swapfile = false
  vim.bo[bufnr].filetype = "que"
  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
  vim.api.nvim_buf_set_name(bufnr, "que-lib://" .. name)
end

local function library_picker(executable)
  vim.ui.input({ prompt = "Que library pattern: ", default = "*" }, function(pattern)
    if not pattern or pattern == "" then
      return
    end
    local lines = vim.fn.systemlist({ executable, "--lib", "types", pattern })
    if vim.v.shell_error ~= 0 then
      vim.notify(table.concat(lines, "\n"), vim.log.levels.ERROR)
      return
    end
    local entries = {}
    for _, line in ipairs(lines) do
      if line ~= "" then
        local name = line:match("^([^%s]+)%s+:") or line
        table.insert(entries, { name = name, text = line })
      end
    end
    if #entries == 0 then
      vim.notify("No Que library symbols match " .. pattern, vim.log.levels.INFO)
      return
    end

    local ok, pickers = pcall(require, "telescope.pickers")
    local finder_ok, finders = pcall(require, "telescope.finders")
    local sorter_ok, sorters = pcall(require, "telescope.sorters")
    if ok and finder_ok and sorter_ok then
      pickers.new({}, {
        prompt_title = "Que library types: " .. pattern,
        finder = finders.new_table({
          results = entries,
          entry_maker = function(entry)
            return { value = entry, display = entry.text, ordinal = entry.text }
          end,
        }),
        sorter = sorters.get_generic_fuzzy_sorter(),
        attach_mappings = function(_, map)
          local actions = require("telescope.actions")
          local state = require("telescope.actions.state")
          local function open_selected()
            local selection = state.get_selected_entry()
            actions.close(_)
            if selection and selection.value then
              open_library_source(executable, selection.value.name)
            end
          end
          map("i", "<CR>", open_selected)
          map("n", "<CR>", open_selected)
          return true
        end,
      }):find()
      return
    end

    vim.ui.select(entries, {
      prompt = "Que library types: " .. pattern,
      format_item = function(entry)
        return entry.text
      end,
    }, function(entry)
      if entry then
        open_library_source(executable, entry.name)
      end
    end)
  end)
end

function M.setup(opts)
  opts = opts or {}

  -- Completion presentation is editor configuration, not an LSP capability.
  -- Apply Blink's Que labels immediately so they do not depend on whether the
  -- language server has attached yet. on_attach repeats this harmlessly for
  -- configurations that load Blink after que.nvim.
  if opts.completion_icons ~= false then
    local icons = vim.tbl_extend("force", M.completion_icons, opts.completion_icons or {})
    setup_blink_completion(icons)
  end

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
  lsp_opts.format_cmd = nil
  lsp_opts.format_key = nil
  lsp_opts.handlers = vim.tbl_extend("force", user_handlers, {
    ["textDocument/hover"] = function(err, result, ctx, config)
      return hover_handler(err, clean_que_hover(result), ctx, config)
    end,
  })
  lsp_opts.on_attach = function(client, bufnr)
    setup_completion(client, bufnr, opts)
    setup_signature_help(client, bufnr, opts)
    setup_hover(client, bufnr, hover_handler, opts)
    vim.api.nvim_buf_create_user_command(bufnr, "QueLib", function()
      library_picker(opts.que_executable or "que")
    end, { desc = "Browse Que library types" })
    vim.keymap.set("n", opts.library_key or "<leader>g", function()
      library_picker(opts.que_executable or "que")
    end, { buffer = bufnr, silent = true, desc = "Browse Que library types" })
    if user_on_attach then
      user_on_attach(client, bufnr)
    end
  end

  local format_group = vim.api.nvim_create_augroup("QueFormat", { clear = true })
  vim.api.nvim_create_autocmd("FileType", {
    group = format_group,
    pattern = opts.filetypes or { "que", "eclisp" },
    callback = function(event)
      setup_format(event.buf, opts)
    end,
  })
  if vim.bo.filetype == "que" or vim.bo.filetype == "eclisp" then
    setup_format(vim.api.nvim_get_current_buf(), opts)
  end

  lspconfig.quelsp.setup(vim.tbl_deep_extend("force", {
    cmd = opts.cmd or { "quelsp" },
    filetypes = opts.filetypes or { "que", "eclisp" },
    root_dir = opts.root_dir or root_dir,
    single_file_support = true,
  }, lsp_opts))
end

return M
