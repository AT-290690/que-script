#!/usr/bin/env node
// One-shot source migration for the breaking data-last core collection API.
// It preserves surrounding text/comments and only canonicalizes the rewritten
// call itself.  Kept in-tree so downstream libraries can use the same
// migration when moving from the old target-first spelling.
import fs from "node:fs";

const target = process.argv.slice(2).find((arg) => !arg.startsWith("--"));
if (!target) throw new Error("usage: migrate-data-last-core.mjs <file>");
const publicOnly = process.argv.includes("--public-only");
const rustMode = process.argv.includes("--rust");
const legacyOnly = process.argv.includes("--legacy-only");
const repairOnly = process.argv.includes("--repair-only");
// This mode is deliberately for a pristine pre-data-last source file.  It
// performs one unambiguous rotation, including multi-index `get` sugar and
// the public receiver-last helpers below.  Do not run it twice on a file.
const fromLegacy = process.argv.includes("--from-legacy");
const source = fs.readFileSync(target, "utf8");

const receiverLastHelpers = new Set([
  "Vector/append/raw!", "Matrix/in-bounds/raw?", "Matrix/set/raw!",
  "Que/add-to-left!", "Que/add-to-right!", "Que/append/raw!",
  "Que/prepend/raw!", "Que/enqueue/raw!",
  "Set/resize!", "Set/add!", "Set/remove!", "Set/intersection",
  "Set/difference", "Set/xor", "Set/union",
  "Table/update!", "Table/update-or!", "Table/push-or!", "Table/set!",
  "Table/remove!", "Table/drop!", "Table/merge!", "Table/resize!",
  "Table/hash", "Table/find-index", "Table/for-each", "Table/set/unchecked!",
  "Table/resize/raw!", "Table/has/raw?", "Table/set/raw!", "Table/remove/raw!",
  "Table/get/raw", "Table/drop/raw!", "Table/keep/raw", "Table/merge/raw!",
  "Table/merge/raw", "Table/omit/raw",
  "Set/hash", "Set/find-index", "Set/for-each", "Set/add/raw!",
  "Vector/set!", "Vector/append!", "Vector/cons!", "Vector/swap!",
  "Vector/sort-partition!", "Vector/sort!", "Matrix/for/i",
  "Heap/swap!", "Heap/sift-up!", "Heap/sift-down!", "Heap/push!",
  "Heap/pop!", "Heap/replace!",
  "String/damerau-levenshtein", "at", "in-bounds?",
  "cartesian-product", "interleave",
]);

const lambdaReceiverLast = [
  ["Vector/append/raw!", "xs x", "x xs"],
  ["Matrix/in-bounds/raw?", "matrix y x", "y x matrix"],
  ["Matrix/set/raw!", "matrix y x value", "y x value matrix"],
  ["Que/add-to-left!", "q item", "item q"],
  ["Que/add-to-right!", "q item", "item q"],
  ["Que/append/raw!", "q item", "item q"],
  ["Que/prepend/raw!", "q item", "item q"],
  ["Que/enqueue/raw!", "queue item", "item queue"],
  ["Set/resize!", "table new-capacity", "new-capacity table"],
  ["Set/add!", "table item", "item table"],
  ["Set/remove!", "table item", "item table"],
  ["Set/intersection", "a b", "b a"],
  ["Set/difference", "a b", "b a"],
  ["Set/xor", "a b", "b a"],
  ["Set/union", "a b", "b a"],
  ["Table/update!", "table key init f", "key init f table"],
  ["Table/update-or!", "table key missing present", "key missing present table"],
  ["Table/push-or!", "table key value", "key value table"],
  ["Table/set!", "table key value", "key value table"],
  ["Table/remove!", "table key", "key table"],
  ["Table/drop!", "table keys", "keys table"],
  ["Table/merge!", "table other", "other table"],
  ["Table/resize!", "table n", "n table"],
  ["Table/hash", "table key", "key table"],
  ["Table/find-index", "bucket key", "key bucket"],
  ["Table/for-each", "table fn", "fn table"],
  ["Table/set/unchecked!", "table key value", "key value table"],
  ["Table/resize/raw!", "table new-capacity", "new-capacity table"],
  ["Table/has/raw?", "table key", "key table"],
  ["Table/set/raw!", "table key value", "key value table"],
  ["Table/remove/raw!", "table key", "key table"],
  ["Table/get/raw", "table key", "key table"],
  ["Table/drop/raw!", "table keys", "keys table"],
  ["Table/keep/raw", "table keys", "keys table"],
  ["Table/merge/raw!", "a b", "b a"],
  ["Table/merge/raw", "a b", "b a"],
  ["Table/omit/raw", "table keys", "keys table"],
  ["Set/hash", "table key", "key table"],
  ["Set/find-index", "bucket key", "key bucket"],
  ["Set/for-each", "table fn", "fn table"],
  ["Set/add/raw!", "table key", "key table"],
  ["Vector/set!", "xs i v", "i v xs"],
  ["Vector/append!", "xs x", "x xs"],
  ["Vector/cons!", "xs x", "x xs"],
  ["Vector/swap!", "xs i j", "i j xs"],
  ["Vector/sort-partition!", "arr start end fn", "start end fn arr"],
  ["Vector/sort!", "arr fn", "fn arr"],
  ["Matrix/for/i", "matrix fn", "fn matrix"],
  ["Heap/swap!", "heap i j", "i j heap"],
  ["Heap/sift-up!", "heap fn", "fn heap"],
  ["Heap/sift-down!", "heap fn", "fn heap"],
  ["Heap/push!", "heap value fn", "value fn heap"],
  ["Heap/pop!", "heap fn", "fn heap"],
  ["Heap/replace!", "heap value fn", "value fn heap"],
  ["String/damerau-levenshtein", "a b", "b a"],
  ["at", "xs i", "i xs"],
  ["in-bounds?", "xs index", "index xs"],
  ["cartesian-product", "a b", "b a"],
  ["interleave", "xs ys", "ys xs"],
  ["get*", "xs i some none", "i some none xs"],
];

function skipQuoted(text, index, quote) {
  let i = index + 1;
  while (i < text.length) {
    if (text[i] === "\\") i += 2;
    else if (text[i++] === quote) break;
  }
  return i;
}

function skipComment(text, index) {
  const end = text.indexOf("\n", index);
  return end < 0 ? text.length : end;
}

function skipSpaceAndComments(text, index) {
  let i = index;
  while (i < text.length) {
    if (/\s/.test(text[i])) i += 1;
    else if (text[i] === ";") i = skipComment(text, i);
    else break;
  }
  return i;
}

function atomEnd(text, index) {
  let i = index;
  while (i < text.length && !/[\s()\[\]{};]/.test(text[i])) i += 1;
  return i;
}

function listEnd(text, index, open, close) {
  let i = index + 1;
  while (i < text.length) {
    const c = text[i];
    if (c === ";") i = skipComment(text, i);
    else if (c === '"' || c === "'") i = skipQuoted(text, i, c);
    else if (c === "(") i = listEnd(text, i, "(", ")");
    else if (c === "[") i = listEnd(text, i, "[", "]");
    else if (c === "{") i = listEnd(text, i, "{", "}");
    else if (c === close) return i + 1;
    else i += 1;
  }
  throw new Error(`unclosed ${open} at ${index}`);
}

function topLevelItems(text) {
  const items = [];
  let i = 1;
  while (i < text.length - 1) {
    i = skipSpaceAndComments(text, i);
    if (i >= text.length - 1) break;
    const start = i;
    if (text[i] === "(") i = listEnd(text, i, "(", ")");
    else if (text[i] === "[") i = listEnd(text, i, "[", "]");
    else if (text[i] === "{") i = listEnd(text, i, "{", "}");
    else if (text[i] === '"' || text[i] === "'") i = skipQuoted(text, i, text[i]);
    else i = atomEnd(text, i);
    items.push(text.slice(start, i));
  }
  return items;
}

const likelyTarget = (value) => /^(xs|ys|zs|out|arr|matrix|rows|cols|table|bucket|heap|queue|path|flags|cells|marks|stack|history|alt|a|b|env|check|destructured-vector|[A-Z][A-Z0-9_-]*)$/u.test(value.trim());
const likelyIndex = (value) => /^(i|j|k|x|y|z|r|c|n|idx|index|row|col|start|end|left|right|mid|half|offset|[+-]?\d+)$/u.test(value.trim());
const likelyReceiver = (value) => {
  const trimmed = value.trim();
  return likelyTarget(trimmed)
    || /^[\[{]/u.test(trimmed)
    || (/^[A-Za-z_][A-Za-z0-9_\/?*!-]*$/u.test(trimmed) && !likelyIndex(trimmed));
};
const likelyData = (value) => likelyReceiver(value) || /^\(/u.test(value.trim());

function transform(text) {
  let out = "";
  let i = 0;
  while (i < text.length) {
    const c = text[i];
    if (c === ";") {
      const end = skipComment(text, i);
      out += text.slice(i, end);
      i = end;
    } else if (c === '"' || c === "'") {
      const end = skipQuoted(text, i, c);
      out += text.slice(i, end);
      i = end;
    } else if (c === "(" || c === "[" || c === "{") {
      const close = c === "(" ? ")" : c === "[" ? "]" : "}";
      const end = listEnd(text, i, c, close);
      const raw = text.slice(i, end);
      const items = topLevelItems(raw).map(transform);
      const head = items[0];
      let reordered = items;
      if (c === "(") {
        if (fromLegacy && head === "get" && items.length >= 3) {
          reordered = [head, ...items.slice(2), items[1]];
        } else if (fromLegacy && ["push!", "capacity!", "cdr"].includes(head) && items.length === 3) {
          reordered = [head, items[2], items[1]];
        } else if (fromLegacy && head === "set!" && items.length === 4) {
          reordered = [head, items[2], items[3], items[1]];
        } else if (items.length >= 3 && receiverLastHelpers.has(head)
          && (fromLegacy || (legacyOnly && likelyData(items[1]) && !likelyReceiver(items.at(-1))))) {
          reordered = [head, ...items.slice(2), items[1]];
        } else {
        const mayRewrite = (kind) => {
          if (!legacyOnly) return true;
          // In the legacy spelling the receiver is the first argument.  The
          // new spelling places it last.  Looking at the receiver position,
          // rather than the index/value position, also handles expressions
          // such as `(get xs (+ i 1))` reliably.
          if (kind === "get" || kind === "cdr" || kind === "capacity!" || kind === "push!") {
            return likelyData(items[1]) && !likelyReceiver(items[2]);
          }
          if (kind === "set!") return likelyData(items[1]) && !likelyReceiver(items[3]);
          return true;
        };
        const appendedTo = items[3]?.trim().match(/^\(length\s+(.+)\)$/su);
        if (head === "set!" && items.length === 4 && appendedTo && appendedTo[1].trim() === items[2].trim()) {
          // Repair the only ambiguous mechanical case: an earlier migration
          // may have rotated `(set! xs (length xs) value)` twice.
          reordered = [head, items[3], items[1], items[2]];
        } else if (head === "set!" && items.length === 4 && likelyIndex(items[3]) && likelyData(items[2])) {
          // The other mechanically detectable twice-rotated shape is
          // `(set! value vector index)`.
          reordered = [head, items[3], items[1], items[2]];
        } else if (!repairOnly && [
          "Vector/push!", "Vector/append!", "Vector/cons!",
          "Que/enque!", "Que/push!", "Que/prepend!", "Que/append!",
        ].includes(head) && items.length === 3 && mayRewrite("push!")) {
          reordered = [head, items[2], items[1]];
        } else if (head === "Vector/set!" && items.length === 4) {
          reordered = [head, items[2], items[3], items[1]];
        } else if (!publicOnly && ["get", "push!", "capacity!"].includes(head) && items.length === 3 && mayRewrite(head)) {
          reordered = [head, items[2], items[1]];
        } else if (!publicOnly && head === "set!" && items.length === 4 && mayRewrite(head)) {
          reordered = [head, items[2], items[3], items[1]];
        } else if (!publicOnly && head === "cdr" && items.length === 3 && mayRewrite(head)) {
          reordered = [head, items[2], items[1]];
        }
        }
      }
      out += reordered === items ? c + transform(raw.slice(1, -1)) + close : `(${reordered.join(" ")})`;
      i = end;
    } else {
      out += c;
      i += 1;
    }
  }
  return out;
}

function transformRustStrings(text) {
  let out = "";
  let i = 0;
  while (i < text.length) {
    const raw = text.slice(i).match(/^r(#+)?"/);
    if (raw) {
      const hashes = raw[1] ?? "";
      const start = i + raw[0].length;
      const terminator = `"${hashes}`;
      const end = text.indexOf(terminator, start);
      if (end < 0) throw new Error(`unterminated raw Rust string at ${i}`);
      let migrated = text.slice(start, end);
      try { migrated = transform(migrated); } catch (error) {
        if (process.env.QUE_MIGRATE_VERBOSE === "1") console.error(`skipped raw string at ${i}: ${error.message}`);
      }
      out += raw[0] + migrated + terminator;
      i = end + terminator.length;
      continue;
    }
    if (text[i] === '"') {
      const end = skipQuoted(text, i, '"');
      const rawLiteral = text.slice(i, end);
      let decoded;
      try {
        decoded = JSON.parse(rawLiteral);
      } catch {
        // Rust permits literal newlines in ordinary strings, while JSON does
        // not.  Calque fixture programs commonly use those strings; their
        // contents need no unescaping to reorder calls safely.
        let migrated = rawLiteral.slice(1, -1);
        try { migrated = transform(migrated); } catch (error) {
          if (process.env.QUE_MIGRATE_VERBOSE === "1") console.error(`skipped multiline string at ${i}: ${error.message}`);
        }
        out += `"${migrated}"`;
        i = end;
        continue;
      }
      let migrated = decoded;
      try { migrated = transform(decoded); } catch (error) {
        if (process.env.QUE_MIGRATE_VERBOSE === "1") console.error(`skipped string at ${i}: ${error.message}`);
      }
      out += JSON.stringify(migrated);
      i = end;
      continue;
    }
    out += text[i++];
  }
  return out;
}

let migrated = rustMode ? transformRustStrings(source) : transform(source);
if (fromLegacy && !rustMode) {
  for (const [name, oldParams, newParams] of lambdaReceiverLast) {
    // Definitions are often formatted across several lines.  Match only the
    // declaration prefix and preserve its whitespace before the parameter
    // list, while accepting arbitrary whitespace between parameters.
    const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const oldPattern = oldParams.trim().split(/\s+/u)
      .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
      .join("\\s+");
    const pattern = new RegExp(`(\\(let\\s+${escapedName}\\s+\\(lambda\\s+)${oldPattern}(?=\\s|\\()`, "gu");
    migrated = migrated.replace(pattern, `$1${newParams}`);
  }
}
fs.writeFileSync(target, migrated);
