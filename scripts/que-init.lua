-- Que workstation configuration installed by scripts/install-all-linux.sh.
-- Self-contained: no files from the Kickstart repository are required.
vim.loader.enable()
vim.g.mapleader = " "
vim.g.maplocalleader = " "
vim.g.have_nerd_font = false

vim.o.number = true
vim.o.mouse = "a"
vim.o.showmode = false
vim.o.breakindent = true
vim.o.undofile = true
vim.o.ignorecase = true
vim.o.smartcase = true
vim.o.signcolumn = "yes"
vim.o.updatetime = 250
vim.o.timeoutlen = 300
vim.o.splitright = true
vim.o.splitbelow = true
vim.o.list = true
vim.opt.listchars = { tab = "· ", trail = "·", nbsp = "␣" }
vim.o.inccommand = "split"
vim.o.cursorline = true
vim.o.scrolloff = 10
vim.o.confirm = true
vim.opt.whichwrap:append("<,>,[,]")
vim.opt.tabstop = 2
vim.opt.shiftwidth = 2
vim.opt.softtabstop = 2
vim.opt.expandtab = true
vim.opt.smartindent = false
vim.opt.autoindent = false
vim.opt.fillchars:append({ eob = " " })
vim.schedule(function() vim.o.clipboard = "unnamedplus" end)

vim.keymap.set("n", "<Esc>", "<cmd>nohlsearch<CR>")
vim.keymap.set({ "n", "i", "v" }, "<C-s>", "<Esc>:w<CR>", { desc = "Save file" })
vim.keymap.set("t", "<Esc><Esc>", "<C-\\><C-n>", { desc = "Exit terminal mode" })
for _, key in ipairs({ "h", "j", "k", "l" }) do
  vim.keymap.set("n", "<C-" .. key .. ">", "<C-w><C-" .. key .. ">")
end

local selection_opts = { noremap = true, silent = true }
for _, key in ipairs({ "<C-a>", "<D-a>" }) do
  vim.keymap.set({ "n", "x" }, key, "ggVG", selection_opts)
  vim.keymap.set("i", key, "<Esc>ggVG", selection_opts)
end
vim.keymap.set("n", "<S-Up>", "v0", selection_opts)
vim.keymap.set("x", "<S-Up>", "0", selection_opts)
vim.keymap.set("i", "<S-Up>", "<Esc>v0", selection_opts)
vim.keymap.set("n", "<S-Down>", "v$", selection_opts)
vim.keymap.set("x", "<S-Down>", "$", selection_opts)
vim.keymap.set("i", "<S-Down>", "<Esc>v$", selection_opts)

vim.api.nvim_create_autocmd("TextYankPost", {
  group = vim.api.nvim_create_augroup("que-highlight-yank", { clear = true }),
  callback = function() vim.highlight.on_yank() end,
})

local lazy_path = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
if not vim.uv.fs_stat(lazy_path) then
  local result = vim.fn.system({
    "git", "clone", "--filter=blob:none", "--branch=stable",
    "https://github.com/folke/lazy.nvim.git", lazy_path,
  })
  if vim.v.shell_error ~= 0 then error("failed to install lazy.nvim: " .. result) end
end
vim.opt.rtp:prepend(lazy_path)

require("lazy").setup({
  { "NMAC427/guess-indent.nvim", opts = {} },
  {
    "lewis6991/gitsigns.nvim",
    opts = { signs = {
      add = { text = "+" }, change = { text = "~" }, delete = { text = "_" },
      topdelete = { text = "‾" }, changedelete = { text = "~" },
    } },
  },
  {
    "folke/which-key.nvim",
    event = "VeryLazy",
    opts = {
      delay = 0,
      icons = { mappings = false },
      spec = {
        { "<leader>s", group = "[S]earch", mode = { "n", "v" } },
        { "<leader>t", group = "[T]oggle" },
        { "<leader>h", group = "Git [H]unk", mode = { "n", "v" } },
        { "gr", group = "LSP Actions" },
      },
    },
  },
  {
    "catppuccin/nvim",
    name = "catppuccin",
    priority = 1000,
    config = function()
      require("catppuccin").setup({ styles = { comments = {} } })
      vim.cmd.colorscheme("catppuccin")
      vim.api.nvim_set_hl(0, "NormalFloat", { link = "Pmenu" })
      local float = vim.api.nvim_get_hl(0, { name = "NormalFloat", link = false })
      local normal = vim.api.nvim_get_hl(0, { name = "Normal", link = false })
      vim.api.nvim_set_hl(0, "FloatBorder", { fg = normal.fg, bg = float.bg })
    end,
  },
  { "folke/todo-comments.nvim", dependencies = { "nvim-lua/plenary.nvim" }, opts = { signs = false } },
  { "kylechui/nvim-surround", version = "*", opts = {} },
  {
    "nvim-mini/mini.nvim",
    config = function()
      require("mini.ai").setup({ mappings = { around_next = "aa", inside_next = "ii" }, n_lines = 500 })
      require("mini.surround").setup()
      local statusline = require("mini.statusline")
      statusline.setup({ use_icons = false })
      statusline.section_location = function() return "%2l:%-2v" end
    end,
  },
  { "nvim-lua/plenary.nvim" },
  {
    "nvim-telescope/telescope.nvim",
    dependencies = {
      "nvim-lua/plenary.nvim",
      "nvim-telescope/telescope-ui-select.nvim",
      { "nvim-telescope/telescope-fzf-native.nvim", build = "make", cond = vim.fn.executable("make") == 1 },
    },
    config = function()
      local telescope = require("telescope")
      telescope.setup({ extensions = { ["ui-select"] = require("telescope.themes").get_dropdown() } })
      pcall(telescope.load_extension, "fzf")
      pcall(telescope.load_extension, "ui-select")
      local builtin = require("telescope.builtin")
      vim.keymap.set("n", "<leader>sh", builtin.help_tags, { desc = "[S]earch [H]elp" })
      vim.keymap.set("n", "<leader>sk", builtin.keymaps, { desc = "[S]earch [K]eymaps" })
      vim.keymap.set("n", "<leader>sf", builtin.find_files, { desc = "[S]earch [F]iles" })
      vim.keymap.set("n", "<leader>ss", builtin.builtin, { desc = "[S]earch [S]elect Telescope" })
      vim.keymap.set({ "n", "v" }, "<leader>sw", builtin.grep_string, { desc = "[S]earch current [W]ord" })
      vim.keymap.set("n", "<leader>sg", builtin.live_grep, { desc = "[S]earch by [G]rep" })
      vim.keymap.set("n", "<leader>sd", builtin.diagnostics, { desc = "[S]earch [D]iagnostics" })
      vim.keymap.set("n", "<leader>sr", builtin.resume, { desc = "[S]earch [R]esume" })
      vim.keymap.set("n", "<leader>s.", builtin.oldfiles, { desc = "[S]earch recent files" })
      vim.keymap.set("n", "<leader>sc", builtin.commands, { desc = "[S]earch [C]ommands" })
      vim.keymap.set("n", "<leader><leader>", builtin.buffers, { desc = "Find buffers" })
      vim.keymap.set("n", "<leader>/", function()
        builtin.current_buffer_fuzzy_find(require("telescope.themes").get_dropdown({ winblend = 10, previewer = false }))
      end, { desc = "Search current buffer" })
    end,
  },
  { "j-hui/fidget.nvim", opts = {} },
  { "neovim/nvim-lspconfig" },
  {
    "mason-org/mason.nvim",
    dependencies = {
      "mason-org/mason-lspconfig.nvim",
      "WhoIsSethDaniel/mason-tool-installer.nvim",
    },
    config = function()
      require("mason").setup({})
      require("mason-lspconfig").setup({ automatic_enable = false })
      require("mason-tool-installer").setup({ ensure_installed = { "rust-analyzer", "lua-language-server", "stylua" } })
    end,
  },
  {
    "stevearc/conform.nvim",
    opts = { notify_on_error = false, default_format_opts = { lsp_format = "fallback" } },
    config = function(_, opts)
      require("conform").setup(opts)
      vim.keymap.set({ "n", "v" }, "<leader>f", function()
        require("conform").format({ async = true })
      end, { desc = "Format buffer" })
    end,
  },
  { "L3MON4D3/LuaSnip", version = "v2.*", build = "make install_jsregexp", opts = {} },
  {
    "saghen/blink.cmp",
    version = "1.*",
    dependencies = { "L3MON4D3/LuaSnip" },
    opts = {
      keymap = { preset = "default" },
      appearance = { nerd_font_variant = "mono" },
      completion = {
        accept = { auto_brackets = { blocked_filetypes = { "que", "eclisp" } } },
        documentation = { auto_show = false, auto_show_delay_ms = 500 },
      },
      sources = { default = { "lsp", "path", "snippets" } },
      snippets = { preset = "luasnip" },
      fuzzy = { implementation = "lua" },
      signature = { enabled = true, window = {
        border = "single", winhighlight = "Normal:NormalFloat,FloatBorder:FloatBorder",
      } },
    },
  },
  { "windwp/nvim-autopairs", event = "InsertEnter", opts = {} },
  {
    "nvim-treesitter/nvim-treesitter",
    branch = "main",
    build = ":TSUpdate",
    config = function()
      local treesitter = require("nvim-treesitter")
      treesitter.install({ "bash", "c", "diff", "html", "lua", "luadoc", "markdown", "markdown_inline", "query", "vim", "vimdoc" })
      vim.api.nvim_create_autocmd("FileType", {
        callback = function(args)
          local language = vim.treesitter.language.get_lang(args.match)
          if not language then return end
          local installed = treesitter.get_installed("parsers")
          if vim.tbl_contains(installed, language) and vim.treesitter.language.add(language) then
            vim.treesitter.start(args.buf, language)
          end
        end,
      })
    end,
  },
}, { checker = { enabled = false }, change_detection = { notify = false } })

vim.diagnostic.config({
  update_in_insert = false,
  severity_sort = true,
  float = { border = "rounded", source = "if_many" },
  underline = { severity = { min = vim.diagnostic.severity.WARN } },
  virtual_text = true,
  virtual_lines = false,
})
vim.keymap.set("n", "<leader>q", vim.diagnostic.setloclist, { desc = "Open diagnostic quickfix list" })

vim.api.nvim_create_autocmd("LspAttach", {
  group = vim.api.nvim_create_augroup("que-lsp-attach", { clear = true }),
  callback = function(event)
    local function map(keys, func, desc, mode)
      vim.keymap.set(mode or "n", keys, func, { buffer = event.buf, desc = "LSP: " .. desc })
    end
    map("grn", vim.lsp.buf.rename, "Rename")
    map("gra", vim.lsp.buf.code_action, "Code action", { "n", "x" })
    map("grD", vim.lsp.buf.declaration, "Declaration")
    local ok, builtin = pcall(require, "telescope.builtin")
    if ok then
      map("grr", builtin.lsp_references, "References")
      map("gri", builtin.lsp_implementations, "Implementation")
      map("grd", builtin.lsp_definitions, "Definition")
      map("gO", builtin.lsp_document_symbols, "Document symbols")
      map("gW", builtin.lsp_dynamic_workspace_symbols, "Workspace symbols")
      map("grt", builtin.lsp_type_definitions, "Type definition")
    end
  end,
})

-- Que is copied by install-all-linux.sh. Load it after Blink so Que can apply
-- the same completion labels used by the local workstation.
local que_root = vim.fn.stdpath("data") .. "/site/pack/que/start/que-nvim"
local que_lua = que_root .. "/lua"
vim.opt.rtp:prepend(que_root)
package.path = que_lua .. "/?.lua;" .. que_lua .. "/?/init.lua;" .. package.path
local que = dofile(que_lua .. "/que/init.lua")
que.setup({ completion_icons = {
  Text = "≡", Method = "ƒ", Function = "λ", Constructor = "+",
  Field = "·", Variable = "α", Class = "C", Interface = "I",
  Module = "m", Property = "·", Unit = "()", Value = "◇",
  Enum = "E", Keyword = "β", Snippet = "⋯", Color = "■",
  File = "#", Reference = "↗", Folder = "/*", EnumMember = "◇",
  Constant = "π", Struct = "{}", Event = "!", Operator = "●",
  TypeParameter = "T",
} })

vim.cmd("filetype plugin indent on")
vim.cmd("syntax enable")
