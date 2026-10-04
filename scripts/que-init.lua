-- Small, portable Que Neovim setup for a fresh Linux machine.
-- It intentionally avoids Nerd Font-only icons and works with Neovim 0.9+.
vim.g.mapleader = " "
vim.g.maplocalleader = " "
vim.g.have_nerd_font = false
vim.opt.number = true
vim.opt.signcolumn = "yes"
vim.opt.termguicolors = true
vim.opt.updatetime = 250
vim.opt.splitright = true
vim.opt.splitbelow = true
vim.opt.ignorecase = true
vim.opt.smartcase = true
vim.opt.whichwrap:append("<,>,[,]")
vim.cmd("filetype plugin indent on")
vim.cmd("syntax enable")

local lazypath = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
if not vim.loop.fs_stat(lazypath) then
  vim.fn.system({ "git", "clone", "--filter=blob:none", "https://github.com/folke/lazy.nvim.git", lazypath })
end
vim.opt.rtp:prepend(lazypath)

require("lazy").setup({
  { "nvim-lua/plenary.nvim" },
  {
    "nvim-telescope/telescope.nvim",
    dependencies = { "nvim-lua/plenary.nvim" },
    config = function()
      local telescope = require("telescope")
      telescope.setup({})
      local builtin = require("telescope.builtin")
      vim.keymap.set("n", "<leader>sf", builtin.find_files, { desc = "Find files" })
      vim.keymap.set("n", "<leader>sg", builtin.live_grep, { desc = "Live grep" })
      vim.keymap.set("n", "<leader>sb", builtin.buffers, { desc = "Find buffers" })
      vim.keymap.set("n", "<leader>/", builtin.current_buffer_fuzzy_find, { desc = "Search buffer" })
    end,
  },
  { "neovim/nvim-lspconfig" },
  { "folke/tokyonight.nvim", priority = 1000, config = function() vim.cmd.colorscheme("tokyonight-night") end },
})

-- Lazy.nvim rebuilds Lua's search paths. Add Que after Lazy has initialized.
vim.opt.rtp:prepend(vim.fn.stdpath("data") .. "/site/pack/que/start/que-nvim")
local que_plugin_lua = vim.fn.stdpath("data") .. "/site/pack/que/start/que-nvim/lua"
package.path = que_plugin_lua .. "/?.lua;" .. que_plugin_lua .. "/?/init.lua;" .. package.path

local que_plugin = dofile(que_plugin_lua .. "/que/init.lua")
que_plugin.setup({})

-- Basic LSP navigation and diagnostics.
vim.diagnostic.config({
  severity_sort = true,
  virtual_text = true,
  float = { border = "single" },
})
vim.keymap.set("n", "K", vim.lsp.buf.hover, { desc = "LSP hover" })
vim.keymap.set("n", "gd", vim.lsp.buf.definition, { desc = "LSP definition" })
vim.keymap.set("n", "<leader>q", vim.diagnostic.setloclist, { desc = "Diagnostics" })

-- The Que plugin provides <leader>r/d/e/w/a/z/g in Que buffers.
