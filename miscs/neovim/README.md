# Neovim Support

This folder contains a minimal Neovim integration for Que / Eclisp.

It provides:

- `.que` filetype detection
- a basic syntax file
- comment settings for `;`
- `quelsp` setup through `nvim-lspconfig`
- completion kind icons through `blink.cmp` or `nvim-cmp`, with Neovim's built-in LSP completion as a fallback
- live function signatures and active-argument tracking while typing applications
- clean Neovim hover text without Markdown language-fence markers

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

Default root markers:

- `que.toml`
- `.git`

## Notes

- This is intentionally thin. Hover, completions, and diagnostics come from `quelsp`.
- If `nvim-lspconfig` is not installed, `require("que").setup()` will fail with a clear error.
- This does not include Tree-sitter. The syntax file is simple on purpose.
