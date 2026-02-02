import React, { useState, useEffect } from "react";
import Editor, { useMonaco } from "@monaco-editor/react";
import { Button } from "./components/ui/button.js";
import themes from "./theme.ts";
import { LogoVariant } from "./LogoVariant.js";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "./components/ui/dialog.js";
import { Input } from "./components/ui/input.js";
import { standardLibrary } from "./standardLibrary.js";
import { ScrollArea } from "./components/ui/scroll-area.js";
import {
  Play,
  BookOpen,
  Regex,
  Zap,
  Type,
  BugIcon,
  BugOff,
  Link,
  Pause,
  Copy,
  Check,
  Trash2,
  Sparkles,
  BookMarked,
  Flower,
} from "lucide-react";
import init, {
  // exec,
  // comp,
  // cons,
  // run,
  // format,
  signatures,
  evaluate,
  check,
  js,
  get_output_len,
} from "./pkg/web/fez_rs.js";
import { Que, QueConf } from "./que.js";
import {
  compressToBase64,
  decompressFromBase64,
} from "lz-string/libs/lz-string.js";
import { Toggle } from "@radix-ui/react-toggle";
// @ts-ignore
import sparklesUrl from "./sparkles.gif";
import learnText from "./learn.ts";

let wasm;
let timeout: ReturnType<typeof setTimeout> | null = null;
const initial = new URLSearchParams(location.search).get("l") ?? "";
const SPARKLE_TIMEOUT = 2000;
const isUnsafe = new URLSearchParams(location.search).get("u") === "t";
const isMinimal = new URLSearchParams(location.search).get("m") === "t";
const autoRun = new URLSearchParams(location.search).get("r") === "t";
const initIsSparkling = new URLSearchParams(location.search).get("s") === "t";
const selectedTheme =
  new URLSearchParams(location.search).get("theme") ?? "dark";

const tools = new URLSearchParams(location.search).get("t");
const displayButtons = tools
  ? tools.split("").reduce(
      (a, b) => {
        a[b] = true;
        return a;
      },
      {
        p: false,
        c: false,
        j: false,
        m: false,
        u: false,
        d: false,
        s: false,
      }
    )
  : {
      p: true,
      c: true,
      j: true,
      m: true,
      u: true,
      d: true,
      s: true,
    };
const toEncoded = (value) => encodeURIComponent(compressToBase64(value));
const toNewUrl = ({ isMinimal, autoRun, isUnsafe, value }) =>
  `${window.location.protocol}//${window.location.host}${
    window.location.pathname
  }?m=${isMinimal ? "t" : "f"}&r=${autoRun ? "t" : "f"}&u=${
    isUnsafe ? "t" : "f"
  }&l=${toEncoded(value)}`;
const serialise = (arg) => {
  if (typeof arg === "number") return arg.toString();
  else if (typeof arg === "string") return `"${arg.toString()}"`;
  else if (Array.isArray(arg))
    return arg.length ? `[${arg.map((a) => serialise(a)).join(" ")}]` : "[]";
  else if (arg === true || arg === false) return arg.toString();
  else return "Function";
};
const serialiseHandledStrings = (arg) => {
  if (typeof arg === "number") return arg.toString();
  else if (typeof arg === "string") return `${arg.toString()}`;
  else if (Array.isArray(arg))
    return arg.length
      ? `[${arg.map((a) => serialiseHandledStrings(a)).join(" ")}]`
      : "[]";
  else if (arg === true || arg === false) return arg.toString();
  else return "Function";
};
const charCodesToString = (codes) => {
  if (Array.isArray(codes)) {
    if (typeof codes[0] === "number")
      return codes.map((x) => String.fromCharCode(x)).join("");
    else return codes.map(charCodesToString);
  } else return codes;
};
const charCodeToChar = (code) => `'${String.fromCharCode(code)}'`;
const readWasmString = (ptr, len) =>
  new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, ptr, len));
// Use these
const typeCheck = (program) => readWasmString(check(program), get_output_len());
const compileJs = (program) => readWasmString(js(program), get_output_len());
// const formatCode = (program) =>
//   readWasmString(format(program), get_output_len());

const generateSignatures = (program) =>
  readWasmString(signatures(program), get_output_len());
// const compileBiteCode = (program) =>
//   readWasmString(comp(program), get_output_len());
// const execBiteCode = (program) =>
//   readWasmString(exec(program), get_output_len());
const typeCheckAndRun = (program) =>
  readWasmString(evaluate(program), get_output_len());
// const uncheckRun = (program) => readWasmString(run(program), get_output_len());
// const concatenateBiteCode = (a, b) =>
//   readWasmString(cons(a, b), get_output_len());
const formatType = (typ) => typ.replaceAll(new RegExp(/\d/g), "");
// const reduceErrorNoise = (err) => {
//   if (err.length > 100) {
//     const arr = [...err];
//     const lastLine = err.split("\n").pop();
//     return `${arr.slice(0, 97).join("")}...\n${lastLine}`;
//   }
//   return err;
// };
const compile = (source: string) => {
  const out = typeCheckAndRun(source);
  // To debug memory leaks
  // console.log(wasm.memory.buffer);
  if (out[0] !== "0") {
    return {
      // reduceErrorNoise()
      err: out.slice(2).replaceAll("Error! ", "").trim(),
      typ: null,
      res: null,
    };
  } else {
    const [_, typ, res] = out.split("\n");
    return { err: null, typ: formatType(typ), res };
  }
};
const fastCompile = (source: string) => {
  try {
    const comp = compileJs(source);
    if (comp.includes("return var ")) {
      return {
        err: null,
        typ: "()",
        res: 0,
      };
    }
    return {
      err: null,
      typ: "()",
      res: new Function(`return ${compileJs(source)}`)(),
    };
  } catch (err) {
    return {
      err: `Something went horribly wrong executing JavaScript!\n${err}`,
      typ: "()",
      res: null,
    };
  }
};
const checkTypes = (source: string) => {
  const out = typeCheck(source);
  // To debug memory leaks
  // console.log(wasm.memory.buffer);
  if (out[0] !== "0") {
    return {
      // reduceErrorNoise
      err: out.slice(2).replaceAll("Error! ", "").trim(),
      typ: null,
      res: null,
    };
  } else {
    return {
      err: null,
      typ: formatType(out.slice(2)),
      res: null,
    };
  }
};
interface TerminalLine {
  type: "output" | "error";
  content: string;
}
const handleError = (res) => {
  try {
    return JSON.parse(
      res.trim().split("\n").at(-1)?.replaceAll(" ", ",") ?? res
    );
  } catch (err) {
    return err;
  }
};

const showSpark = (editor, spark, setSpark) => {
  if (!editor) return;
  const position = editor.getPosition();
  const visiblePos = editor.getScrolledVisiblePosition(position);
  if (!visiblePos) return;
  // Get editor DOM rect to convert to page coordinates (optional: keep relative to editor)
  const editorRect = editor.getDomNode().getBoundingClientRect();
  const top = editorRect.top + visiblePos.top;
  const left = editorRect.left + visiblePos.left;
  // Clear previous timer
  if (timeout) {
    clearTimeout(timeout);
    timeout = null;
  }

  setSpark({
    visible: true,
    result: false,
    failed: false,
    top,
    left,
    height: visiblePos.height,
  });

  // Hide after 2 seconds
  timeout = setTimeout(() => {
    setSpark({ ...spark, visible: false, result: false, failed: false });
    timeout = null;
  }, SPARKLE_TIMEOUT);

  return { top, left, height: visiblePos.height };
};

function formatTuple(type, res) {
  // Helper: split top-level space-separated tokens while respecting nested brackets
  function splitTopLevel(str) {
    const parts: string[] = [];
    let depth = 0,
      current = "";
    for (let i = 0; i < str.length; i++) {
      const c = str[i];
      if (c === "{" || c === "[") depth++;
      if (c === "}" || c === "]") depth--;
      if (c === " " && depth === 0) {
        if (current.trim()) parts.push(current.trim());
        current = "";
      } else current += c;
    }
    if (current.trim()) parts.push(current.trim());
    return parts;
  }

  // Parse the result structure into nested arrays/numbers
  function parseValue(input) {
    input = input.trim();
    if (input.startsWith("[") && input.endsWith("]")) {
      const inner = input.slice(1, -1).trim();
      if (!inner) return [];
      return splitTopLevel(inner).map(parseValue);
    }
    if (input.startsWith("{") && input.endsWith("}")) {
      const inner = input.slice(1, -1).trim();
      const parts = splitTopLevel(inner);
      return parts.map(parseValue);
    }
    if (/^\d+$/.test(input)) return Number(input);
    if (input === "true") return true;
    if (input === "false") return false;
    return input;
  }

  // Parse type structure to decide how to format chars
  function formatWithType(typeDesc, val) {
    typeDesc = typeDesc.trim();

    if (Array.isArray(val)) {
      // Handle array types
      if (typeDesc.startsWith("[") && typeDesc.endsWith("]")) {
        const innerType = typeDesc.slice(1, -1).trim();
        // Special case: [Char]
        if (innerType === "Char") {
          return `"${String.fromCharCode(...val)}"`;
        }
        return val.map((v) => formatWithType(innerType, v));
      }

      // Handle tuple {A * B}
      if (typeDesc.includes("*")) {
        const [leftType, rightType] = typeDesc
          .replace(/[{}]/g, "")
          .split("*")
          .map((t) => t.trim());
        return [
          formatWithType(leftType, val[0]),
          formatWithType(rightType, val[1]),
        ];
      }
    }

    // Char case
    if (typeDesc === "Char") {
      return `'${String.fromCharCode(val)}'`;
    }

    // Primitive passthrough
    return val;
  }

  const parsedRes = parseValue(res);
  return formatWithType(type, parsedRes);
}
const theme = themes[selectedTheme] ?? themes["dark"];
export default function CodeEditor() {
  const monaco = useMonaco();
  // if (!monaco) return;
  const [editor, setEditor] = useState(null);
  // const editorRef = useRef(null);
  const [terminalLines, setTerminalLines] = useState({
    content: "",
    type: "output",
  });
  const [typeLibrary, setsTypeLibrary] = useState([]);
  const [typeInfo, setTypeInfo] = useState("()");
  const [libraryOpen, setLibraryOpen] = useState(false);
  const [typelibraryOpen, settypeLibraryOpen] = useState(false);
  const [shareOpen, setShareOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [regexMode, setRegexMode] = useState(false);
  const [isUnsafeMode, setIsUnsafeMode] = useState(isUnsafe);
  const [isAutoRun, setIsAutoRun] = useState(autoRun);
  const [isCopied, setIsCopied] = useState(false);
  const [shareLink, setShareLink] = useState("");
  const [isSparkling, setIsSparkling] = useState(initIsSparkling);
  const [spark, setSpark] = useState({
    failed: false,
    visible: false,
    result: false,
    top: 0,
    left: 0,
    height: 50,
  });

  useEffect(() => {
    if (!monaco) return;
    const editorInit = monaco.editor.getEditors()[0];
    if (!editorInit) return;
    setEditor(editorInit);

    const themeName = "my";
    // defineTheme is idempotent but avoid re-defining unnecessarily
    try {
      monaco.editor.defineTheme(themeName, theme as any);
    } catch (e) {
      // ignore if it's already defined (optional)
    }
    monaco.editor.setTheme(themeName);
    // Register a new language
    monaco.languages.register({ id: "que" });

    // Register a tokens provider for the language
    monaco.languages.setMonarchTokensProvider("que", Que as any);
    monaco.languages.setLanguageConfiguration("que", QueConf as any);

    // Register a completion item provider for the "que" language
    const completionProvider = monaco.languages.registerCompletionItemProvider(
      "que",
      {
        provideCompletionItems: (model, position) => {
          const word = model.getWordUntilPosition(position);
          const range = {
            startLineNumber: position.lineNumber,
            endLineNumber: position.lineNumber,
            startColumn: word.startColumn,
            endColumn: word.endColumn,
          };
          const suggestions = standardLibrary.map(({ name, signature }) => ({
            label: name,
            kind: monaco.languages.CompletionItemKind.Function,
            // documentation: signature,
            detail: signature,
            insertText: name,
            range: range,
          }));

          return { suggestions: suggestions };
        },
      }
    );

    (async () => {
      // globalThis._mprint_m = (data) =>
      //   setTimeout(
      //     () =>
      //       setTerminalLines({
      //         type: "content",
      //         content: serialise(data),
      //       }),
      //     1
      //   );
      wasm = await init();
      if (initial) {
        try {
          const source = decompressFromBase64(initial);
          if (source) {
            editorInit.setValue(source);
            if (autoRun) isUnsafe ? handleFastRun(source) : handleRun(source);
          }
        } catch (e) {
          setTerminalLines({
            type: "error",
            content: e instanceof Error ? e.message : e,
          });
        }
      }
    })();
    // Cleanup provider
    return () => {
      completionProvider.dispose();
    };
  }, [monaco]); // runs once when monaco becomes available

  const filteredLibrary =
    searchQuery === ""
      ? standardLibrary
      : standardLibrary.filter((item) => {
          let matchesSearch = false;
          if (regexMode) {
            try {
              const regex = new RegExp(searchQuery);
              matchesSearch =
                regex.test(item.name) || regex.test(item.signature);
            } catch (e) {
              // Invalid regex, fall back to normal search
              matchesSearch =
                item.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                item.signature.includes(searchQuery);
            }
          } else {
            matchesSearch =
              item.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
              item.signature.includes(searchQuery);
          }
          return matchesSearch;
        });
  const clearSpark = isSparkling
    ? () => {
        if (timeout) {
          clearTimeout(timeout);
          timeout = null;
        }
        setSpark({ ...spark, visible: false, result: false, failed: true });
      }
    : () => {};
  const sparkResult = isSparkling
    ? () => {
        // Clear previous timer
        if (timeout) {
          clearTimeout(timeout);
          timeout = null;
        }
        setSpark({ ...spark, visible: false, failed: false, result: true });
        timeout = setTimeout(() => {
          setSpark({
            ...spark,
            visible: false,
            result: false,
            failed: false,
          });
          timeout = null;
        }, SPARKLE_TIMEOUT);
      }
    : () => {};

  const handleRun = (source: string = "") => {
    const { err, typ, res } = compile(source);
    if (err != null) {
      clearSpark();
      setTypeInfo("()");
      return setTerminalLines({ type: "error", content: err });
    }
    if (typ) {
      if (typ.includes("*")) {
        setTerminalLines({
          type: "output",
          content: serialiseHandledStrings(formatTuple(typ, res)),
        });
      } else
        setTerminalLines({
          type: "output",
          content:
            typ === "Char"
              ? charCodeToChar(res)
              : typ.includes("[Char]")
              ? serialise(charCodesToString(handleError(res)))
              : res,
        });
      // setTerminalLines({
      //   type: "output",
      //   content:
      //     typ === "Char"
      //       ? charCodeToChar(res)
      //       : !typ.includes("*") && typ.includes("[Char]")
      //       ? serialise(charCodesToString(handleError(res)))
      //       : res,
      // });
      setTypeInfo(formatType(typ));
    }
    sparkResult();
  };
  const handleFastRun = (source: string = "") => {
    const { err, res } = fastCompile(source);
    setTypeInfo("()");
    if (err != null) {
      return setTerminalLines({ type: "error", content: err });
    }
    setTerminalLines({
      type: "output",
      content: serialise(res),
    });
    sparkResult();
  };
  const handleCheck = (source: string = "") => {
    // const selection =
    //   editor && editor.getModel().getValueInRange(editor.getSelection());
    // let { err, typ } = checkTypes(
    //   selection ? `${source}\n${selection}` : source
    // );

    let { err, typ } = checkTypes(source);
    if (err != null) {
      clearSpark();
      setTypeInfo("()");
      return setTerminalLines({ type: "error", content: err });
    }

    // if (!selection) {
    setsTypeLibrary(
      generateSignatures(source)
        .split("\n")
        .map((x) => x.split(","))
        .map(
          ([name, signature]) => ({
            name,
            signature: signature
              .replaceAll(/\d+/g, "")
              .split(".")
              .at(-1)
              ?.trim(),
          }),
          []
        )
    );
    setSearchQuery("");
    settypeLibraryOpen(true);
    // }
    setTerminalLines({
      type: "output",
      content: "",
    });
    if (typ) setTypeInfo(typ.replaceAll(new RegExp(/T+\d+/g), "T"));
    sparkResult();
  };
  // Keyboard shortcut: Ctrl/Cmd+S to run code
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (
        e.key.toLowerCase() === "s" &&
        (e.ctrlKey || e.metaKey) &&
        !e.shiftKey &&
        !isUnsafeMode
      ) {
        e.preventDefault();
        e.stopPropagation();
        handleRun(editor.getValue());
      } else if (
        e.key.toLowerCase() === "s" &&
        (e.ctrlKey || e.metaKey) &&
        (e.shiftKey || isUnsafeMode)
      ) {
        e.preventDefault();
        e.stopPropagation();
        handleFastRun(editor.getValue());
      }
    };

    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [handleRun, handleFastRun]);

  // const getCursorPosition = () => {
  //   const pos = editor?.getPosition(); // { lineNumber, column } or null
  //   return pos;
  // };

  // const handleChange = (value) => {
  //   setCode(value || "");
  // };

  return (
    <div className="flex flex-col h-screen bg-slate-950">
      {/* Header */}
      {isMinimal ? (
        <></>
      ) : (
        <div className="flex items-center justify-between px-6 py-4 bg-slate-950 border-b border-slate-800">
          <LogoVariant variant={0} />
          {/* <div className="flex items-center justify-between gap-4">
            <Button
              onClick={() => {
                const selection =
                  editor &&
                  editor.getModel().getValueInRange(editor.getSelection());
                if (selection) {
                  editor.setValue(
                    editor
                      .getValue()
                      .replaceAll(selection, formatCode(selection))
                  );
                }
              }}
              variant="outline"
              className="bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
            >
              <Flower className="w-4 h-4" />
            </Button>
        
          </div> */}
          <Button
            onClick={() =>
              editor.setValue(
                editor
                  .getValue()
                  .replaceAll(/\s*;+.*/g, "")
                  .trim() + learnText
              )
            }
            variant="outline"
            className="bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
          >
            <BookMarked className="w-4 h-4" />
          </Button>
          {/* <h1 className="text-slate-100">Playground</h1> */}
        </div>
      )}
      {/* Main Content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Code Editor Section */}
        <div className="flex-1 flex flex-col min-h-0">
          <div className="flex-1 min-h-0">
            {/* Spark image positioned absolutely over the page */}
            <img
              src={sparklesUrl}
              alt="spark"
              style={{
                position: "absolute",
                top: spark.top + 10,
                left: spark.left - 10,
                pointerEvents: "none",
                filter: `hue-rotate(${Math.round(Math.random() * 360)}deg)`,
                transform: "translate(-50%, -50%)", // center on the cursor; adjust as needed
                display: spark.visible ? "block" : "none",
                width: spark.height * Math.round(Math.random() * 5 + 5), // optional sizing tied to line height
                height: "auto",
                zIndex: 9999,
              }}
            />
            <Editor
              defaultLanguage="que"
              language="que"
              theme="my"
              onChange={
                isSparkling
                  ? () => showSpark(editor, spark, setSpark)
                  : () => {}
              }
              options={{
                fontFamily: "JetBrains Mono",
                fontSize: 14,
                bracketPairColorization: { enabled: true },
                minimap: { enabled: false },
                scrollBeyondLastLine: false,
                fontLigatures: true,
                lineNumbers: "on",
                tabSize: 2,
                automaticLayout: true,
                renderWhitespace: "boundary",
                guides: {
                  indentation: false,
                  bracketPairs: true,
                  bracketPairsHorizontal: true,
                  highlightActiveBracketPair: true,
                  highlightActiveIndentation: true,
                },
              }}
            />
          </div>
        </div>
        <img
          src={sparklesUrl}
          alt="spark"
          style={{
            position: "absolute",
            bottom: 0,
            left: 0,
            // transform: "translate(-50%, -50%)",
            filter: `hue-rotate(${Math.round(Math.random() * 360)}deg)`,
            pointerEvents: "none",
            display: spark.result ? "block" : "none",
            width: 200, // optional sizing tied to line height
            height: "auto",
            zIndex: 9999,
          }}
        />
        {/* Terminal Section */}
        <div className="h-32 flex flex-col bg-slate-950 border-t border-slate-800">
          <div className="flex items-center justify-between px-6 py-2 bg-slate-950 border-b border-slate-800">
            {/* <span className="text-slate-400 text-sm">
              Terminal
            </span> */}
            {/* Type Display */}
            <textarea
              value={typeInfo}
              className="text-emerald-400 rounded text-sm"
              style={{
                resize: "none",
                width: "100%",
                fontFamily: '"JetBrains Mono", monospace',
              }}
            ></textarea>

            <div className="flex gap-2">
              {displayButtons.s && (
                <Button
                  // disabled={!editor?.getValue()}
                  onClick={() => {
                    const value = editor.getValue();
                    setShareLink(
                      toNewUrl({
                        isMinimal,
                        autoRun,
                        isUnsafe,
                        value,
                      })
                    );
                    setTerminalLines({
                      type: "output",
                      content: toEncoded(value),
                    });
                    setShareOpen(true);
                  }}
                  variant="outline"
                  className="bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                >
                  <Link className="w-4 h-4 mr-1" />
                  {/* Share */}
                </Button>
              )}
              {displayButtons.d && (
                <Button
                  onClick={() => setLibraryOpen(true)}
                  variant="outline"
                  className="bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                >
                  <BookOpen className="w-4 h-4 mr-1" />
                  {/* Library */}
                </Button>
              )}
              {displayButtons.u && (
                <Toggle
                  defaultPressed={isUnsafeMode}
                  onPressedChange={(flag) => {
                    setIsUnsafeMode(flag);
                  }}
                >
                  {isUnsafeMode ? (
                    <Button
                      variant="outline"
                      className="bg-red-400 border-red-400 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                    >
                      <BugIcon className="w-4 h-4 mr-1 " />
                    </Button>
                  ) : (
                    <Button
                      variant="outline"
                      className="bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                    >
                      <BugOff className="w-4 h-4 mr-1 " />
                    </Button>
                  )}
                </Toggle>
              )}
              {displayButtons.m && (
                <Toggle
                  defaultPressed={isSparkling}
                  onPressedChange={(flag) => setIsSparkling(flag)}
                >
                  <Button
                    variant="outline"
                    className={
                      isSparkling
                        ? "bg-purple-400 border-purple-400 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                        : "bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                    }
                  >
                    <Sparkles className="w-4 h-4 mr-1 " />
                  </Button>
                </Toggle>
              )}
              {displayButtons.j && (
                <Button
                  onClick={() => handleFastRun(editor.getValue())}
                  className="bg-yellow-600 hover:bg-slate-700 text-white cursor-pointer"
                >
                  <Zap className="w-4 h-4 mr-1 " />
                </Button>
              )}

              {displayButtons.c && (
                <Button
                  disabled={isUnsafeMode}
                  onClick={() => handleCheck(editor.getValue())}
                  className="bg-blue-600 hover:bg-slate-700 text-white cursor-pointer"
                >
                  <Type className="w-4 h-4 mr-1 " />
                </Button>
              )}

              {displayButtons.p && (
                <Button
                  disabled={isUnsafeMode}
                  onClick={() => handleRun(editor.getValue())}
                  className="bg-emerald-600 hover:bg-slate-700 text-white cursor-pointer"
                >
                  <Play className="w-4 h-4 mr-1 " />
                  {/* Run */}
                </Button>
              )}
            </div>
          </div>
          {/* <ScrollArea className="flex-1 px-6 py-2"> */}
          <div
            className="px-6 py-2 text-sm space-y-1 overflow-hidden"
            style={{ fontFamily: '"JetBrains Mono", monospace' }}
          >
            <textarea
              value={terminalLines.content}
              style={{ resize: "none", width: "100%" }}
              className={
                terminalLines.type === "error"
                  ? "text-red-400"
                  : "text-slate-300"
              }
            ></textarea>
          </div>
          {/* </ScrollArea> */}
        </div>
      </div>

      {/* Library Search Dialog */}
      <Dialog open={libraryOpen} onOpenChange={setLibraryOpen}>
        <DialogContent className="w-full max-w-2xl sm:max-w-4xl md:max-w-6xl bg-slate-950 border-slate-700 text-slate-100">
          <DialogHeader>
            <DialogTitle
              className="text-slate-100"
              style={{ fontFamily: '"JetBrains Mono", monospace' }}
            >
              Library
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            {/* Search row */}
            <div className="flex items-center gap-2">
              <div className="relative flex-1">
                {/* Search icon inside input */}
                {/* <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-400 pointer-events-none" /> */}
                <Input
                  style={{ fontFamily: '"JetBrains Mono", monospace' }}
                  placeholder="Search functions..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-10 pr-3 h-9 w-full bg-slate-900 border border-slate-700 text-slate-100 placeholder:text-slate-500 rounded-lg"
                />
              </div>

              {/* Regex toggle button */}
              <Button
                variant="outline"
                onClick={() => setRegexMode((s) => !s)}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs h-9 transition-all bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                title={regexMode ? "Regex mode enabled" : "Regex mode disabled"}
              >
                <Regex
                  className={`h-3.5 w-3.5 ${
                    regexMode ? "text-slate-200" : "text-slate-500"
                  }`}
                />
              </Button>
            </div>

            {/* Results */}
            <ScrollArea className="h-64">
              <div className="space-y-2 p-1">
                {filteredLibrary.length > 0 ? (
                  filteredLibrary.map((item, index) => (
                    <div
                      key={index}
                      className="p-3 bg-slate-900 rounded-lg border border-slate-700 hover:border-slate-600 cursor-pointer transition-colors"
                      onClick={() => {
                        editor.setValue(
                          editor.getValue() +
                            (item.signature.includes("->")
                              ? `\n(${item.name} )`
                              : `\n${item.name}`)
                        );
                        setLibraryOpen(false);
                        setSearchQuery("");
                      }}
                    >
                      <div className="flex items-baseline justify-between mb-1">
                        <span
                          className="text-slate-400 font-mono"
                          style={{ fontFamily: '"JetBrains Mono", monospace' }}
                        >
                          {item.name}
                        </span>
                        {/* optional signature */}
                        {/* <code className="text-xs text-slate-400 font-mono">{item.signature}</code> */}
                      </div>
                      <code
                        className="text-sm text-emerald-400 font-mono"
                        style={{ fontFamily: '"JetBrains Mono", monospace' }}
                      >
                        {item.signature}
                      </code>
                    </div>
                  ))
                ) : (
                  <div
                    className="text-center py-8 text-slate-500 font-mono"
                    style={{ fontFamily: '"JetBrains Mono", monospace' }}
                  >
                    No functions found matching "{searchQuery}"
                  </div>
                )}
              </div>
            </ScrollArea>
          </div>
        </DialogContent>
      </Dialog>
      {/* Share Dialog */}

      <Dialog open={typelibraryOpen} onOpenChange={settypeLibraryOpen}>
        <DialogContent className="w-full max-w-2xl sm:max-w-4xl md:max-w-6xl bg-slate-950 border-slate-700 text-slate-100">
          <DialogHeader>
            <DialogTitle
              className="text-slate-100"
              style={{ fontFamily: '"JetBrains Mono", monospace' }}
            >
              Types
            </DialogTitle>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-4">
              {/* Search row */}
              <div className="flex items-center gap-2">
                <div className="relative flex-1">
                  {/* Search icon inside input */}
                  {/* <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-400 pointer-events-none" /> */}
                  <Input
                    style={{ fontFamily: '"JetBrains Mono", monospace' }}
                    placeholder="Search types..."
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="pl-10 pr-3 h-9 w-full bg-slate-900 border border-slate-700 text-slate-100 placeholder:text-slate-500 rounded-lg"
                  />
                </div>

                {/* Regex toggle button */}
                <Button
                  variant="outline"
                  onClick={() => setRegexMode((s) => !s)}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs h-9 transition-all bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                  title={
                    regexMode ? "Regex mode enabled" : "Regex mode disabled"
                  }
                >
                  <Regex
                    className={`h-3.5 w-3.5 ${
                      regexMode ? "text-slate-200" : "text-slate-500"
                    }`}
                  />
                </Button>
              </div>

              {/* Results */}
              <ScrollArea className="h-64">
                <div className="space-y-2 p-1">
                  {typeLibrary.length > 0 ? (
                    (searchQuery === ""
                      ? typeLibrary
                      : typeLibrary.filter((item) => {
                          let matchesSearch = false;
                          if (regexMode) {
                            try {
                              const regex = new RegExp(searchQuery);
                              matchesSearch =
                                regex.test(item.name) ||
                                regex.test(item.signature);
                            } catch (e) {
                              // Invalid regex, fall back to normal search
                              matchesSearch =
                                item.name
                                  .toLowerCase()
                                  .includes(searchQuery.toLowerCase()) ||
                                item.signature.includes(searchQuery);
                            }
                          } else {
                            matchesSearch =
                              item.name
                                .toLowerCase()
                                .includes(searchQuery.toLowerCase()) ||
                              item.signature.includes(searchQuery);
                          }
                          return matchesSearch;
                        })
                    ).map((item, index) => (
                      <div
                        key={index}
                        className="p-3 bg-slate-900 rounded-lg border border-slate-700 hover:border-slate-600 cursor-pointer transition-colors"
                        // onClick={() => {
                        //   editor.setValue(
                        //     editor.getValue() +
                        //       (item.signature.includes("->")
                        //         ? `\n(${item.name} )`
                        //         : `\n${item.name}`)
                        //   );
                        //   settypeLibraryOpen(false);
                        // }}
                      >
                        <div className="flex items-baseline justify-between mb-1">
                          <span
                            className="text-slate-400 font-mono"
                            style={{
                              fontFamily: '"JetBrains Mono", monospace',
                            }}
                          >
                            {item.name}
                          </span>
                          {/* optional signature */}
                          {/* <code className="text-xs text-slate-400 font-mono">{item.signature}</code> */}
                        </div>
                        <code
                          className="text-sm text-emerald-400 font-mono"
                          style={{
                            fontFamily: '"JetBrains Mono", monospace',
                          }}
                        >
                          {item.signature}
                        </code>
                      </div>
                    ))
                  ) : (
                    <div
                      className="text-center py-8 text-slate-500 font-mono"
                      style={{ fontFamily: '"JetBrains Mono", monospace' }}
                    >
                      No types found
                    </div>
                  )}
                </div>
              </ScrollArea>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <Dialog open={shareOpen} onOpenChange={setShareOpen}>
        <DialogContent className="w-full max-w-2xl sm:max-w-4xl md:max-w-6xl bg-slate-950 border-slate-700 text-slate-100">
          <DialogHeader>
            <DialogTitle className="text-slate-100">Share</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            {/* Search row */}
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                className="bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                onClick={(e) => {
                  e.stopPropagation();
                  navigator.clipboard.writeText(shareLink);
                  setIsCopied(true);
                  setTimeout(() => setIsCopied(false), 1000);
                }}
              >
                {isCopied ? (
                  <>
                    <Check className="h-3 w-3" />
                  </>
                ) : (
                  <>
                    <Copy className="h-3 w-3" />
                  </>
                )}
              </Button>
              <div className="relative flex-1">
                {/* Search icon inside input */}
                {/* <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-400 pointer-events-none" /> */}
                <Input
                  value={shareLink}
                  placeholder="No code to share..."
                  className="pl-10 pr-3 h-9 w-full bg-slate-900 border border-slate-700 text-slate-100 placeholder:text-slate-500 rounded-lg"
                />
              </div>
              <Button
                onClick={() => {
                  window.history.pushState(
                    {},
                    "",
                    `${window.location.protocol}//${window.location.host}${window.location.pathname}`
                  );
                  setShareLink("");
                }}
                variant="outline"
                className="bg-slate-900 hover:bg-slate-700 hover:text-slate-200 border-slate-700 text-slate-200 cursor-pointer"
              >
                <Trash2 className="w-4 h-4 mr-1 " />
              </Button>
              {/* isUnsafeMode */}
              <Toggle
                defaultPressed={isUnsafeMode}
                onPressedChange={(flag) => {
                  setIsUnsafeMode(flag);
                  setShareLink(
                    toNewUrl({
                      isMinimal,
                      isUnsafe: flag,
                      autoRun: isAutoRun,
                      value: editor.getValue(),
                    })
                  );
                }}
              >
                {isUnsafeMode ? (
                  <Button
                    variant="outline"
                    className="bg-red-400 border-red-400 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                  >
                    <BugIcon className="w-4 h-4 mr-1 " />
                  </Button>
                ) : (
                  <Button
                    variant="outline"
                    className="bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                  >
                    <BugOff className="w-4 h-4 mr-1 " />
                  </Button>
                )}
              </Toggle>

              <Toggle
                defaultPressed={isAutoRun}
                onPressedChange={(flag) => {
                  setIsAutoRun(flag);
                  setShareLink(
                    toNewUrl({
                      isMinimal,
                      isUnsafe: isUnsafeMode,
                      autoRun: flag,
                      value: editor.getValue(),
                    })
                  );
                }}
              >
                {isAutoRun ? (
                  <Button
                    variant="outline"
                    className="bg-emerald-600 hover:bg-slate-700 hover:text-slate-200 text-white cursor-pointer"
                  >
                    <Play className="w-4 h-4 mr-1 " />
                  </Button>
                ) : (
                  <Button
                    variant="outline"
                    className="bg-slate-900 border-slate-700 text-slate-200 hover:text-slate-200 hover:bg-slate-700 cursor-pointer"
                  >
                    <Pause className="w-4 h-4 mr-1 " />
                  </Button>
                )}
              </Toggle>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
