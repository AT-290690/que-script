const fs = require("fs");
const os = require("os");
const path = require("path");
const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

/** @type {LanguageClient | undefined} */
let client;
/** @type {vscode.DiagnosticCollection | undefined} */
let shellDiagnostics;

const QUE_HEREDOC_TAG = "QUE";
const SHELL_ANALYSIS_DEBOUNCE_MS = 250;
const SHELL_LANG_IDS = new Set(["shellscript", "shell", "bash", "zsh", "sh"]);
/** @type {Map<string, ReturnType<typeof setTimeout>>} */
const shellAnalysisTimers = new Map();

function splitSignatureTopLevel(signature) {
  const parts = [];
  let buf = "";
  let paren = 0;
  let bracket = 0;
  let inString = false;

  for (let i = 0; i < signature.length; i += 1) {
    const ch = signature[i];
    const prev = i > 0 ? signature[i - 1] : "";
    const next = i + 1 < signature.length ? signature[i + 1] : "";

    if (ch === '"' && prev !== "\\") {
      inString = !inString;
      buf += ch;
      continue;
    }

    if (!inString) {
      if (ch === "(") paren += 1;
      else if (ch === ")") paren = Math.max(0, paren - 1);
      else if (ch === "[") bracket += 1;
      else if (ch === "]") bracket = Math.max(0, bracket - 1);

      if (ch === "-" && next === ">" && paren === 0 && bracket === 0) {
        parts.push(buf.trim());
        buf = "";
        i += 1;
        continue;
      }
    }

    buf += ch;
  }

  if (buf.trim().length > 0) parts.push(buf.trim());
  return parts;
}

function signatureTypePart(signature) {
  const idx = signature.indexOf(":");
  if (idx === -1) return signature.trim();
  return signature.slice(idx + 1).trim();
}

function findCallStart(text) {
  let inString = false;
  let bracketDepth = 0;
  let parenDepth = 0;

  for (let i = text.length - 1; i >= 0; i -= 1) {
    const ch = text[i];
    const prev = i > 0 ? text[i - 1] : "";

    if (ch === '"' && prev !== "\\") {
      inString = !inString;
      continue;
    }
    if (inString) continue;

    if (ch === "]") {
      bracketDepth += 1;
      continue;
    }
    if (ch === "[") {
      bracketDepth = Math.max(0, bracketDepth - 1);
      continue;
    }
    if (bracketDepth > 0) continue;

    if (ch === ")") {
      parenDepth += 1;
      continue;
    }
    if (ch === "(") {
      if (parenDepth === 0) return i;
      parenDepth -= 1;
    }
  }

  return -1;
}

function tokenizeTopLevel(content) {
  const tokens = [];
  let cur = "";
  let inString = false;
  let bracketDepth = 0;
  let parenDepth = 0;
  let lastWasTopLevelSpace = false;

  for (let i = 0; i < content.length; i += 1) {
    const ch = content[i];
    const prev = i > 0 ? content[i - 1] : "";

    if (ch === '"' && prev !== "\\") {
      inString = !inString;
      cur += ch;
      lastWasTopLevelSpace = false;
      continue;
    }

    if (!inString) {
      if (ch === "[") bracketDepth += 1;
      else if (ch === "]") bracketDepth = Math.max(0, bracketDepth - 1);
      else if (ch === "(") parenDepth += 1;
      else if (ch === ")") parenDepth = Math.max(0, parenDepth - 1);
    }

    const atTopLevel = !inString && bracketDepth === 0 && parenDepth === 0;
    if (atTopLevel && /\s/.test(ch)) {
      if (cur.length > 0) {
        tokens.push(cur);
        cur = "";
      }
      lastWasTopLevelSpace = true;
      continue;
    }

    cur += ch;
    lastWasTopLevelSpace = false;
  }

  if (cur.length > 0) tokens.push(cur);
  return { tokens, endsWithTopLevelSpace: lastWasTopLevelSpace };
}

function makeSignatureTriggerChars() {
  const chars = ["(", " ", "[", "]", '"', ")"];
  for (let c = 48; c <= 57; c += 1) chars.push(String.fromCharCode(c));
  for (let c = 97; c <= 122; c += 1) chars.push(String.fromCharCode(c));
  for (let c = 65; c <= 90; c += 1) chars.push(String.fromCharCode(c));
  chars.push("-", "+", "*", "/", "?", "!", "_", ":", ".");
  return Array.from(new Set(chars));
}

function isShellScriptDocument(document) {
  return (
    document &&
    document.uri &&
    document.uri.scheme === "file" &&
    SHELL_LANG_IDS.has(document.languageId)
  );
}

function parseQueHeredocBlocks(text) {
  const lines = text.split(/\r?\n/);
  const blocks = [];
  const openRe = /<<-?\s*(['"]?)([A-Za-z_][A-Za-z0-9_]*)\1/;

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    const match = line.match(openRe);
    if (!match) continue;

    const marker = match[2];
    if (marker !== QUE_HEREDOC_TAG) continue;

    const allowIndent = match[0].includes("<<-");
    const startLine = i + 1;
    let endLine = startLine;

    while (endLine < lines.length) {
      const candidate = lines[endLine];
      const normalized = allowIndent
        ? candidate.replace(/^\t+/, "")
        : candidate;
      if (normalized.trim() === marker) break;
      endLine += 1;
    }

    const hasTerminator = endLine < lines.length;
    const contentLines = hasTerminator
      ? lines.slice(startLine, endLine)
      : lines.slice(startLine);

    blocks.push({
      startLine,
      lineCount: contentLines.length,
      text: contentLines.join("\n"),
    });

    if (!hasTerminator) break;
    i = endLine;
  }

  return blocks;
}

async function fetchQueDiagnosticsForText(text) {
  if (!client) return [];
  try {
    const result = await client.sendRequest("que/analyzeText", { text });
    return Array.isArray(result) ? result : [];
  } catch {
    return [];
  }
}

function lspSeverityToVscode(severity) {
  switch (severity) {
    case 2:
      return vscode.DiagnosticSeverity.Warning;
    case 3:
      return vscode.DiagnosticSeverity.Information;
    case 4:
      return vscode.DiagnosticSeverity.Hint;
    case 1:
    default:
      return vscode.DiagnosticSeverity.Error;
  }
}

function clampLine(line, lineCount) {
  if (!Number.isFinite(line)) return 0;
  if (lineCount <= 0) return 0;
  return Math.min(Math.max(0, line), lineCount - 1);
}

function mapBlockDiagnosticsToDocument(blockDiagnostics, blockStartLine, blockLineCount) {
  if (blockLineCount <= 0) return [];

  const mapped = [];
  for (const diag of blockDiagnostics) {
    if (!diag || !diag.range || !diag.range.start || !diag.range.end) continue;

    const startRelLine = clampLine(diag.range.start.line, blockLineCount);
    const endRelLine = clampLine(diag.range.end.line, blockLineCount);
    const startChar = Math.max(0, diag.range.start.character || 0);
    const endChar = Math.max(0, diag.range.end.character || 0);

    const start = new vscode.Position(blockStartLine + startRelLine, startChar);
    let end = new vscode.Position(blockStartLine + endRelLine, endChar);
    if (end.isBefore(start)) end = start;

    const out = new vscode.Diagnostic(
      new vscode.Range(start, end),
      typeof diag.message === "string" ? diag.message : "Que diagnostic",
      lspSeverityToVscode(diag.severity)
    );
    out.source = diag.source || "que";
    if (diag.code !== undefined) out.code = diag.code;
    mapped.push(out);
  }

  return mapped;
}

async function updateShellHeredocDiagnostics(document) {
  if (!shellDiagnostics || !isShellScriptDocument(document)) return;

  const blocks = parseQueHeredocBlocks(document.getText());
  if (blocks.length === 0) {
    shellDiagnostics.delete(document.uri);
    return;
  }

  const diagnostics = [];
  for (const block of blocks) {
    if (block.text.trim().length === 0) continue;
    const blockDiagnostics = await fetchQueDiagnosticsForText(block.text);
    diagnostics.push(
      ...mapBlockDiagnosticsToDocument(
        blockDiagnostics,
        block.startLine,
        block.lineCount
      )
    );
  }

  shellDiagnostics.set(document.uri, diagnostics);
}

function scheduleShellHeredocDiagnostics(document) {
  if (!shellDiagnostics || !isShellScriptDocument(document)) return;

  const key = document.uri.toString();
  const prev = shellAnalysisTimers.get(key);
  if (prev) clearTimeout(prev);

  const handle = setTimeout(async () => {
    shellAnalysisTimers.delete(key);
    const latest = vscode.workspace.textDocuments.find(
      (doc) => doc.uri.toString() === key
    );
    if (!latest) {
      if (shellDiagnostics) shellDiagnostics.delete(document.uri);
      return;
    }
    await updateShellHeredocDiagnostics(latest);
  }, SHELL_ANALYSIS_DEBOUNCE_MS);

  shellAnalysisTimers.set(key, handle);
}

function registerShellHeredocDiagnostics(context) {
  shellDiagnostics = vscode.languages.createDiagnosticCollection("que-shell");
  context.subscriptions.push(shellDiagnostics);

  for (const doc of vscode.workspace.textDocuments) {
    if (isShellScriptDocument(doc)) scheduleShellHeredocDiagnostics(doc);
  }

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument((doc) => {
      if (isShellScriptDocument(doc)) scheduleShellHeredocDiagnostics(doc);
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidChangeTextDocument((evt) => {
      if (isShellScriptDocument(evt.document))
        scheduleShellHeredocDiagnostics(evt.document);
    })
  );
  context.subscriptions.push(
    vscode.workspace.onDidCloseTextDocument((doc) => {
      const key = doc.uri.toString();
      const timer = shellAnalysisTimers.get(key);
      if (timer) {
        clearTimeout(timer);
        shellAnalysisTimers.delete(key);
      }
      if (shellDiagnostics) shellDiagnostics.delete(doc.uri);
    })
  );
}

async function fetchInferredSignature(document, symbol, position) {
  if (!client) return undefined;
  try {
    const result = await client.sendRequest("que/getSignature", {
      uri: document.uri.toString(),
      symbol,
      position: position
        ? { line: position.line, character: position.character }
        : null,
    });
    if (
      result &&
      typeof result.signature === "string" &&
      result.signature.trim().length > 0
    ) {
      return result.signature.trim();
    }
  } catch {
    // no-op
  }
  return undefined;
}

function registerQueSignatureHelp() {
  const triggerChars = makeSignatureTriggerChars();
  return vscode.languages.registerSignatureHelpProvider(
    "que",
    {
      async provideSignatureHelp(document, position) {
        const cursorOffset = document.offsetAt(position);
        const windowSize = 8000;
        const startOffset = Math.max(0, cursorOffset - windowSize);
        const textBeforeCursor = document.getText(
          new vscode.Range(document.positionAt(startOffset), position)
        );

        const callStartInWindow = findCallStart(textBeforeCursor);
        if (callStartInWindow === -1) return null;

        const content = textBeforeCursor.slice(callStartInWindow + 1);
        const { tokens, endsWithTopLevelSpace } = tokenizeTopLevel(content);
        if (tokens.length === 0) return null;

        const funcName = tokens[0];
        const signature = await fetchInferredSignature(
          document,
          funcName,
          position
        );
        if (!signature) return null;

        const typeSignature = signatureTypePart(signature);
        const allParts = splitSignatureTopLevel(typeSignature);
        const paramLabels = allParts.slice(0, -1);
        if (paramLabels.length === 0) return null;

        const fullLabel = signature.includes(":")
          ? signature
          : `${funcName}: ${typeSignature}`;
        const signatureInfo = new vscode.SignatureInformation(fullLabel);

        const colonIdx = fullLabel.indexOf(":");
        let searchFrom = colonIdx === -1 ? 0 : colonIdx + 1;
        signatureInfo.parameters = paramLabels.map((param) => {
          const idx = fullLabel.indexOf(param, searchFrom);
          if (idx === -1) return new vscode.ParameterInformation(param);
          const start = idx;
          const end = idx + param.length;
          searchFrom = end;
          return new vscode.ParameterInformation([start, end]);
        });

        const help = new vscode.SignatureHelp();
        help.signatures = [signatureInfo];
        help.activeSignature = 0;

        const argsTyped = Math.max(0, tokens.length - 1);
        let activeParam = endsWithTopLevelSpace
          ? argsTyped
          : Math.max(0, argsTyped - 1);
        activeParam = Math.min(
          activeParam,
          Math.max(0, paramLabels.length - 1)
        );
        help.activeParameter = activeParam;
        return help;
      },
    },
    ...triggerChars
  );
}

function executableName() {
  return os.platform() === "win32" ? "quelsp.exe" : "quelsp";
}

function workspaceRoot() {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) {
    return undefined;
  }
  return folders[0].uri.fsPath;
}

function resolveServerCommand(context) {
  const cfg = vscode.workspace.getConfiguration("que");
  const configuredPath = cfg.get("languageServer.path");
  if (typeof configuredPath === "string" && configuredPath.trim().length > 0) {
    return { command: configuredPath.trim(), args: [] };
  }

  const envPath = process.env.QUE_LSP_PATH;
  if (typeof envPath === "string" && envPath.trim().length > 0) {
    return { command: envPath.trim(), args: [] };
  }

  const exe = executableName();
  const root = workspaceRoot();
  const candidateRoots = [];
  if (root) {
    candidateRoots.push(root);
  }
  candidateRoots.push(path.resolve(context.extensionPath, "..", "..", ".."));

  for (const base of candidateRoots) {
    const debugPath = path.join(base, "target", "debug", exe);
    if (fs.existsSync(debugPath)) {
      return { command: debugPath, args: [] };
    }
    const releasePath = path.join(base, "target", "release", exe);
    if (fs.existsSync(releasePath)) {
      return { command: releasePath, args: [] };
    }
  }

  if (root) {
    const cargoManifest = path.join(root, "Cargo.toml");
    const lspBinSource = path.join(root, "src", "bin", "quelsp.rs");
    if (fs.existsSync(cargoManifest) && fs.existsSync(lspBinSource)) {
      return {
        command: "cargo",
        args: ["run", "--quiet", "--bin", "quelsp"],
        cwd: root,
      };
    }
  }

  return { command: exe, args: [] };
}

function createClient(context) {
  const server = resolveServerCommand(context);
  const run = {
    command: server.command,
    args: server.args,
    transport: TransportKind.stdio,
  };
  if (server.cwd) {
    run.options = { cwd: server.cwd };
  }

  const serverOptions = {
    run,
    debug: run,
  };

  const clientOptions = {
    documentSelector: [{ scheme: "file", language: "que" }],
    synchronize: {
      configurationSection: "que",
    },
  };

  return new LanguageClient(
    "queLanguageServer",
    "Que Language Server",
    serverOptions,
    clientOptions
  );
}

async function startClient(context) {
  if (client) {
    return;
  }
  client = createClient(context);
  context.subscriptions.push(client.start());
}

async function restartClient(context) {
  if (client) {
    await client.stop();
    client = undefined;
  }
  await startClient(context);
}

/**
 * @param {vscode.ExtensionContext} context
 */
async function activate(context) {
  await startClient(context);
  const signatureProvider = registerQueSignatureHelp();
  registerShellHeredocDiagnostics(context);

  const restartCommand = vscode.commands.registerCommand(
    "que.restartLanguageServer",
    async () => {
      await restartClient(context);
      for (const doc of vscode.workspace.textDocuments) {
        if (isShellScriptDocument(doc)) scheduleShellHeredocDiagnostics(doc);
      }
      vscode.window.showInformationMessage("Que language server restarted.");
    }
  );

  context.subscriptions.push(signatureProvider);
  context.subscriptions.push(restartCommand);
}

async function deactivate() {
  for (const [, timer] of shellAnalysisTimers) {
    clearTimeout(timer);
  }
  shellAnalysisTimers.clear();
  if (shellDiagnostics) {
    shellDiagnostics.dispose();
    shellDiagnostics = undefined;
  }
  if (!client) {
    return undefined;
  }
  const stopped = client.stop();
  client = undefined;
  return stopped;
}

module.exports = {
  activate,
  deactivate,
};
