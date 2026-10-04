# Neovim Support

This folder contains a minimal Neovim integration for Que / Eclisp.

It provides:

- `.que` filetype detection
- a basic syntax file
- comment settings for `;`
- `quelsp` setup through `nvim-lspconfig`
- completion kind icons through `blink.cmp` or `nvim-cmp`, with Neovim's built-in LSP completion as a fallback
- live function signatures and active-argument tracking while typing applications
- theme-native plain-text hover windows with borders and no Markdown colouring or markers
- automatic signature help, without duplicating Blink's signature window
- `:QueFormat` and `<Space>f` formatting through `que fmt --stdin`
- `:QueLib` and `<Space>g` fuzzy browsing of library names and inferred types

The formatter preserves comments, keeps short forms compact, and leaves runs
of closing delimiters grouped on the final line. Set `format_key` in
`require("que").setup()` to change the mapping, or set it to `false` to disable
the shortcut.

## Requirements

- Neovim 0.9+
- `quelsp` on your `PATH`
- `nvim-lspconfig`

## Layout

- `ftdetect/que.lua`
- `ftplugin/que.lua`
- `syntax/que.vim`
- `lua/que/init.lua`

## Install

Copy `miscs/neovim` into your Neovim runtime path, or use it as a local plugin.

Quick install script:

```bash
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/main/scripts/install-nvim.sh | bash
```

For a fresh Linux/SSH machine, the all-in-one bootstrap also installs Que,
`quelsp`, Wasmtime, Neovim, Telescope, and a portable starter config:

```bash
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/main/scripts/install-all-linux.sh | bash
```

It preserves an existing `~/.config/nvim/init.lua`; set
`QUE_OVERWRITE_NVIM=1` to replace it. To provide your own init file, set
`QUE_NVIM_INIT_URL` to a URL before running the installer.

Example with `lazy.nvim`:

```lua
{
  dir = "/Users/anthony/Desktop/projects/que-script/miscs/neovim",
  name = "que-nvim",
  config = function()
    require("que").setup()
  end,
}
```

Example with `packer.nvim`:

```lua
use {
  "/Users/anthony/Desktop/projects/que-script/miscs/neovim",
  config = function()
    require("que").setup()
  end,
}
```

## LSP

Minimal manual setup:

```lua
require("que").setup({
  cmd = { "quelsp" },
  filetypes = { "que", "eclisp" },
})
```

Completion icons are enabled by default. They can be changed or disabled:

```lua
require("que").setup({
  completion_icons = {
    Function = "ƒ",
    Variable = "v",
    Constant = "c",
    Keyword = "k",
  },
})

-- Or keep completion kinds as plain text:
require("que").setup({ completion_icons = false })
```

The default glyphs use ordinary Unicode symbols and do not require a Nerd Font.
Existing `on_attach` callbacks and `nvim-cmp` formatting are preserved and then decorated for Que buffers.
Inferred completion types are displayed alongside candidates by default; use
`completion_type_hints = false` to hide that column.

## Scratch runner

Inside `que nvim`, these buffer-local commands and shortcuts save the scratch file and show their output in a syntax-highlighted result split:

- `:QueRun` / `<leader>r`: optimized run
- `:QueDebug` / `<leader>d`: debug run
- `:QueWat` / `<leader>w`: optimized WAT output
- `:QueTypes` / `<leader>a`: optimized inferred types
- `:QueExplain` / `<leader>e`: optimized explanation
- `:QueSource` / `<leader>z`: optimized expanded source
- `:QueLib` / `<leader>g`: enter a library glob (for example `map*` or `push!`), fuzzy-filter the matching type signatures, and press Enter to open the selected source

The shortcuts use Space as the leader in the preconfigured Que scratch editor.
Normal and debug runs retain their original interactive terminal splits. Source and inferred
types use Que highlighting, WAT uses WAT highlighting, and explanations use Markdown.

Default root markers:

- `que.toml`
- `.git`

## Notes

- This is intentionally thin. Hover, completions, and diagnostics come from `quelsp`.
- If `nvim-lspconfig` is not installed, `require("que").setup()` will fail with a clear error.
- This does not include Tree-sitter. The syntax file is simple on purpose.
