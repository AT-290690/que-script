import init, { run, check, js } from "./pkg/fez_rs.js";
init();
const makeEditor = (el, theme) => {
  const editor = ace.edit(el);
  editor.setOptions({
    fontFamily: "Monaco9",
    fontSize: "10pt",
    copyWithEmptySelection: true,
  });
  editor.setKeyboardHandler("ace/keyboard/vscode");
  editor.renderer.setShowGutter(true);
  editor.setTheme(`ace/theme/${theme}`);
  editor.setShowPrintMargin(false);
  editor.session.setMode("ace/mode/lisp");
  editor.renderer.setScrollMargin(10, 10);
  editor.session.setUseWrapMode(true);
  return editor;
};
const THEME = "terminal";
const editor = makeEditor("editor", THEME);
const terminal = makeEditor("terminal", THEME);
terminal.renderer.setShowGutter(false);
terminal.setValue("; To run press cmd/ctrl + S or the run button");
terminal.clearSelection();
const initial = new URLSearchParams(location.search).get("l") ?? "";
if (initial) {
  try {
    const decompressed = LZString.decompressFromBase64(initial);
    const source = decodeURIComponent(decompressed);
    editor.setValue(source);
    editor.clearSelection();
  } catch (e) {
    alert(e instanceof Error ? e.message : e);
  }
}
const serialise = (arg) => {
  if (typeof arg === "number" || typeof arg === "string") return arg.toString();
  else if (Array.isArray(arg))
    return arg.length ? `[${arg.map((a) => serialise(a)).join(" ")}]` : "[]";
  else if (arg === true || arg === false) return arg.toString();
  else return "(lambda)";
};
const link = (value) => {
  const compressed = LZString.compressToBase64(value);
  const newurl =
    window.location.protocol +
    "//" +
    window.location.host +
    window.location.pathname +
    `?l=${encodeURIComponent(compressed)}`;
  window.history.pushState({ path: newurl }, "", newurl);
};
const comp = (value) => {
  const out = run(value);
  if (out && out[0] === '"') {
    terminal.setValue(out.substring(1, out.length - 1));
  } else terminal.setValue(out);
};
const type = (value) => {
  const out = check(value);
  if (out && out[0] === '"') {
    terminal.setValue(out.substring(1, out.length - 1));
  } else terminal.setValue(out);
};
const javascript = (value) =>
  terminal.setValue(serialise(new Function(`return ${js(value)}`)()));

document.addEventListener("keydown", (e) => {
  if (e.key.toLowerCase() === "s" && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
    e.preventDefault();
    e.stopPropagation();
    const value = editor.getValue();
    if (value.trim()) {
      comp(value);
      link(value);
      terminal.clearSelection();
    }
  }
});
document.getElementById("run").addEventListener("click", () => {
  const value = editor.getValue();
  if (value.trim()) {
    comp(value);
    link(value);
    terminal.clearSelection();
  }
});
document.getElementById("check").addEventListener("click", () => {
  const value = editor.getValue();
  if (value.trim()) {
    type(value);
    link(value);
    terminal.clearSelection();
  }
});
document.getElementById("js").addEventListener("click", () => {
  const value = editor.getValue();
  if (value.trim()) {
    javascript(value);
    link(value);
    terminal.clearSelection();
  }
});
