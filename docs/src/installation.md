# Installation

Calque uses the `que` command and `.que` source files. The compiler is
installed separately from the WASI runtime that executes compiled programs.
`wasmtime` is the default runtime; `wasmer` and `iwasm`/WAMR also work.

## Compiler and bundled library

### macOS and Linux

The installer downloads the latest release for the current architecture and
installs `que` in `/usr/local/bin` and the bundled library in
`/usr/local/share/que`. It may ask for your `sudo` password.

```sh
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/main/scripts/install.sh | bash
```

Install a compatible WASI runtime separately and ensure it is on `PATH`. Then
verify the installation:

```sh
que --help
wasmtime --version
```

### Windows

Run this in PowerShell. It installs the latest `que.exe` and `que-lib.lisp`
under `%LOCALAPPDATA%\\Programs\\Que` and adds its `bin` directory to your user
`PATH`; administrator rights are not needed.

```powershell
irm https://raw.githubusercontent.com/AT-290690/que-script/main/scripts/install.ps1 | iex
```

Restart PowerShell after the installer finishes, then install a compatible
WASI runtime separately (normally Wasmtime) and ensure it is on `PATH`:

```powershell
que --help
wasmtime --version
```

For one invocation, choose a runtime other than the default:

```sh
que hello.que --runtime wasmer
que hello.que --runtime /path/to/iwasm
```

## Language server and editors

`quelsp` is a separate executable. It provides diagnostics, type hovers,
completion, signatures, formatting integration, and project-aware navigation.
It does not need a WASI runtime merely to serve an editor.

### macOS and Linux

```sh
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/main/scripts/lsp.sh | bash
quelsp --help
```

### Windows

Run this in PowerShell after the compiler installation:

```powershell
irm https://raw.githubusercontent.com/AT-290690/que-script/main/scripts/lsp.ps1 | iex
```

Restart PowerShell if necessary, then check it:

```powershell
quelsp --help
```

The VS Code extension uses `quelsp` from `PATH` by default. To choose a
specific executable, set `que.languageServer.path` to its full path. The
included Neovim integration also looks for `quelsp` on `PATH`.

For the managed Neovim workstation on a fresh Linux machine—Calque, `quelsp`,
Wasmtime, Neovim, Telescope, Catppuccin, and the Calque setup—use:

```sh
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/main/scripts/install-all-linux.sh | bash
```

See [Projects, tests, and editors](compiler/projects-and-editors.md) for
Neovim commands, VS Code configuration, and project behavior.
