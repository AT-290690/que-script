# Emacs Support

`que-mode` provides Calque editing support for Emacs 29.1 and newer.

It includes:

- `.que` and `.eclisp` file recognition
- syntax highlighting, comments, and Lisp-style indentation
- Eglot integration with `quelsp`: diagnostics, inferred-type hover,
  completion, signatures, and definitions
- `que fmt --stdin` formatting
- compilation buffers for optimized runs, debug runs, WAT, and `explain`

It deliberately uses built-in Emacs facilities only. Eglot is bundled with
Emacs 29+, so there is no MELPA dependency.

## Install

From this checkout, add the mode directory to your Emacs configuration:

```elisp
(add-to-list 'load-path "/path/to/que-script/miscs/emacs")
(require 'que-mode)
```

Or install the current release copy and add the same two forms:

```sh
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/main/scripts/install-emacs.sh | bash
```

The installer copies the mode to `~/.emacs.d/site-lisp/que` by default and
prints the exact configuration snippet. It intentionally does not rewrite an
existing Emacs init file. Set `QUE_EMACS_DIR` to use another Emacs directory.

`quelsp` must be on `PATH`; install it with the normal Calque LSP installer:

```sh
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/main/scripts/lsp.sh | bash
```

## Eglot

Opening a `.que` file starts Eglot automatically when `quelsp` is available.
It supplies Flymake diagnostics, completion through `completion-at-point`,
inferred-type documentation through `eldoc`, `M-x eglot-help-at-point`, and
project-aware navigation.

To manage the connection yourself, put this before `(require 'que-mode)`:

```elisp
(setq que-eglot-auto-start nil)
```

Then use `M-x eglot` in a Calque buffer. To choose a non-default server:

```elisp
(setq que-lsp-server-command '("/absolute/path/to/quelsp"))
```

## Commands

| Command | Key | Meaning |
| --- | --- | --- |
| `M-x que-format-buffer` | `C-c C-f` | Format the buffer with `que fmt --stdin` |
| `M-x que-run` | `C-c C-r` | Run the saved file with `--opt` |
| `M-x que-debug` | `C-c C-d` | Run with debug safety checks |
| `M-x que-wat` | `C-c C-w` | Show optimized WAT |
| `M-x que-explain` | `C-c C-e` | Show static-analysis explanations |

Run/debug commands require a saved file. Output appears in Emacs's ordinary
compilation buffer, so source locations remain clickable.
