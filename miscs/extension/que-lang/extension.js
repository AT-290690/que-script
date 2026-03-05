const fs = require("fs");
const os = require("os");
const path = require("path");
const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

/** @type {LanguageClient | undefined} */
let client;

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
    if (result && typeof result.signature === "string" && result.signature.trim().length > 0) {
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
        const signature = await fetchInferredSignature(document, funcName, position);
        if (!signature) return null;

        const allParts = splitSignatureTopLevel(signature);
        const paramLabels = allParts.slice(0, -1);
        if (paramLabels.length === 0) return null;

        const fullLabel = `${funcName}: ${signature}`;
        const signatureInfo = new vscode.SignatureInformation(fullLabel);

        let searchFrom = 0;
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
        let activeParam = endsWithTopLevelSpace ? argsTyped : Math.max(0, argsTyped - 1);
        activeParam = Math.min(activeParam, Math.max(0, paramLabels.length - 1));
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
  const run = { command: server.command, args: server.args, transport: TransportKind.stdio };
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

  const restartCommand = vscode.commands.registerCommand(
    "que.restartLanguageServer",
    async () => {
      await restartClient(context);
      vscode.window.showInformationMessage("Que language server restarted.");
    }
  );

  context.subscriptions.push(signatureProvider);
  context.subscriptions.push(restartCommand);
}

async function deactivate() {
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
