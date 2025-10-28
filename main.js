import init, {
  //   exec,
  //   comp,
  //   cons,
  evaluate,
  run,
  check,
  js,
  get_output_len,
} from "./pkg/web/fez_rs.js";
let lib = "";
let wasm;
(async () => {
  wasm = await init();
  const response = await fetch("./lib.json");
  lib = await response.json();
})();
const readWasmString = (ptr, len) =>
  new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, ptr, len));
// Use these
const typeCheck = (program) => readWasmString(check(program), get_output_len());
const compileJs = (program) => readWasmString(js(program), get_output_len());
// const compileBiteCode = (program) =>
//   readWasmString(comp(program), get_output_len());
// const execBiteCode = (program) =>
//   readWasmString(exec(program), get_output_len());
const typeCheckAndRun = (program) =>
  readWasmString(evaluate(program), get_output_len());
const uncheckRun = (program) => readWasmString(run(program), get_output_len());
// const concatenateBiteCode = (a, b) =>
//   readWasmString(cons(a, b), get_output_len());

const makeEditor = (el, theme) => {
  const editor = ace.edit(el);
  editor.setOptions({
    fontFamily: "Fantastic",
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
const compile = (value) => {
  const out = typeCheckAndRun(value);
  if (out && out[0] === '"') {
    terminal.setValue(out.substring(1, out.length - 1));
  } else terminal.setValue(out);
};
const runCode = (value) => {
  const out = uncheckRun(value);
  if (out && out[0] === '"') {
    terminal.setValue(out.substring(1, out.length - 1));
  } else terminal.setValue(out);
};
const type = (value) => {
  const out = typeCheck(value);
  if (out && out[0] === '"') {
    terminal.setValue(out.substring(1, out.length - 1));
  } else terminal.setValue(out);
};
const javascript = (value) =>
  terminal.setValue(serialise(new Function(`return ${compileJs(value)}`)()));

document.addEventListener("keydown", (e) => {
  if (e.key.toLowerCase() === "s" && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
    e.preventDefault();
    e.stopPropagation();
    const value = editor.getValue();
    if (value.trim()) {
      compile(value);
      link(value);
      terminal.clearSelection();
      //   console.log(wasm.memory.buffer);
    }
  } else if (
    e.key.toLowerCase() === "s" &&
    (e.ctrlKey || e.metaKey) &&
    e.shiftKey
  ) {
    e.preventDefault();
    e.stopPropagation();
    try {
      const arr = JSON.parse(
        terminal.getValue().trim().split("\n").at(-1).replaceAll(" ", ",")
      );
      terminal.setValue(serialise(charCodesToString(arr)));
    } catch (e) {
      terminal.setValue("Can't serialise - not a String output");
    }
  }
});
document.getElementById("run").addEventListener("click", () => {
  const value = editor.getValue();
  if (value.trim()) {
    runCode(value);
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
const charCodesToString = (codes) => {
  if (typeof codes[0] === "number")
    return codes.map((x) => String.fromCharCode(x)).join("");
  else return codes.map(charCodesToString);
};
document.getElementById("str").addEventListener("click", () => {
  try {
    const arr = JSON.parse(
      terminal.getValue().trim().split("\n").at(-1).replaceAll(" ", ",")
    );
    terminal.setValue(serialise(charCodesToString(arr)));
  } catch (e) {
    terminal.setValue("Can't serialise - not a String output");
  }
});
const dialog = document.getElementById("modal");
const dialogCloseButton = document.getElementById("closeModal");
document.getElementById("std").addEventListener("click", () => {
  dialog.showModal();
});
dialogCloseButton.addEventListener("click", () => {
  dialog.close();
});
const field = document.getElementById("libs");
document.getElementById("search").addEventListener("input", (e) => {
  field.value = lib
    .filter(([x]) => x.toLowerCase().includes(e.target.value.toLowerCase()))
    .map((x) => x.join(" "))
    .join("\n");
});
document.getElementById("help").addEventListener("click", () => {
  terminal.setValue(
    `Help (scroll down for more)

; To run press cmd/ctrl + S or click the run button
; To type check only click the check button
; To execute faster JavaScript code click the js button
; To display last line in terminal as text click the char button or press cmd/ctrl + shift + S
; To search functions in library click the lib button

; How Que Script works

; Hindley–Milner Type Inference

; - No type annotations required: the compiler figures everything out.
; - Supports polymorphism and higher-order functions.
; - Only 5 types - functions, booleans, integers, characters and vectors.
; - Guarantees soundness: if your program compiles, it won’t have type errors at runtime.
; - Example:

(let sum-odd-squares (lambda xs
    (|> xs
        (std/vector/filter std/int/odd?)
        (std/vector/map std/int/square)
        (std/vector/int/sum))))

(sum-odd-squares [ 1 2 3 4 5 6 7 8 9 10 ])
; Int
; 165

; - filter, map and sum will be tree shaked from std.
; - Pipe (|> ... ) will be desuggered to:

(std/vector/int/sum (std/vector/map (std/vector/filter xs std/int/odd?) std/int/square))

; - Argument type of the function will be [Int].
; - Return type of the function will be Int.
; - filter will only work with [Int] and callback of type Int -> Bool
; - map will only work with [Int] and callback of type Int -> Int
; - sum will only work with [Int]

; Short names can be extracted from std using import:

(import filter map std/vector)
(import odd? square std/int)
(import sum std/vector/int)

(let sum-odd-squares (lambda xs
    (|> xs
        (filter odd?)
        (map square)
        (sum))))

(sum-odd-squares [ 1 2 3 4 5 6 7 8 9 10 ])

; sum is a sub import of vector under int:

; import int/sum directly from std/vector
(import int/sum filter map std/vector)
(import odd? square std/int)

(let xs [ 1 2 3 4 5 6 7 8 9 10 ])
; here we use int/sum
(int/sum (map (filter xs odd?) square))

; Tail Call Optimization
; A call is said to be in tail position if it is the last instruction executed before returning from the current function. Compilers can optimize such calls by discarding the caller frame and replacing the call with a jump.
; This is especially useful for recursive functions. For instance, take this function that sums the elements of a vector:

(let sum (lambda xs acc
    (if (= (length xs) 0) acc
        (sum (std/vector/drop xs 1) (+ acc (get xs 0))))))

(sum [ 1 2 3 4 5 ] 0)
; Int
; 15

; With a regular call, this consumes 𝒪(n) stack space: each element of the vector adds a new frame on the call stack. With a long enough vector, this could very quickly overflow the stack. By replacing the call with a jump, tail call optimization effectively turns this recursive function into a loop which uses 𝒪(1) stack space:

(let sum (lambda xs acc (do
    (let _acc [ acc ])
    (let _xs [ xs ])
    (let _new_xs [])
    (let _new_acc [])
    (loop (not (= (length (get _xs 0)) 0)) (lambda (do
        (set! _new_xs 0 (std/vector/drop (get _xs 0) 1))
        (set! _new_acc 0 (+ (get _acc 0) (get (get _xs 0) 0)))
        (set! _xs 0 (get _new_xs 0))
        (set! _acc 0 (get _new_acc 0)))))
    (get _acc 0))))

; This optimization is particularly important for functional languages. They rely heavily on recursive functions, and pure ones like Haskell don’t even provide loop control structures. Any kind of custom iteration typically uses recursion one way or another. Without tail call optimization, this would very quickly run into a stack overflow for any non-trivial program:

; TCO recursion
(let k-mod (lambda n k (if (< k n) k (k-mod n (- k n)))))
; taking advantage of partial apply
(let mod2 (k-mod 2))
; TCO recursion
(let collatz (lambda n steps
               (if (= n 1)
                    steps
                    (collatz (if (= (mod2 n) 0)
                                 (/ n 2)
                                 (+ (* n 3) 1))
                                 (+ steps 1)))))

(collatz 27 0)
; Int
; 111

; Tail Call Optimization Convention

; When a function is transformed for tail-call optimization, the last parameter is treated as the accumulator/result parameter.
; On entry, the compiler initializes the result register from this last parameter.
; On each tail-call, the accumulator is updated before the next iteration.
; This guarantees that type inference can resolve the result type statically.
; Users are encouraged to design tail-recursive functions so that the accumulator is the last argument.

; Cast
; An empty vector has a polymorphic type (it can contain anything):

(let xs [])
xs
; [t7]
; []

; To enforce a type we can use as:

(let xs (as [] [Int]))
xs
; [Int]
; []

; Now the vector can only have Ints and will error out if anything else is pushed to it.
; We can also cast Int to Char:

(let x (as 64 Char))
x
; Char
; 64

; Note: This works nicely for vectors of any depth with concrete types Int, Char or Bool. Lambdas are a bit awkward to cast and are therefore discouraged for now.

; Loop Limit
; Loops are capped at 5,000,000 (five million) total iterations for the entire program.
; To ensure programs remain safe when running locally or on shared servers, loops must be "safe" and unable to hang or block the main thread. This limit also applies to tail-call optimized recursion.

; Starting in the top left corner of a 2x2 grid, and only being able to move to the right and down, there are exactly 6 routes to the bottom right corner:

(let factorial (lambda n total
   (if (= n 0)
       total
       (factorial (- n 1) (* total n)))))

(let bionomial-coefficient (lambda a b
    (/ (factorial a 1)
            (*
                (factorial b 1)
                (factorial (- a b) 1)))))

(let m 2)
(let n 2)
(bionomial-coefficient (+ m n) m)
; Int
; 6

; How many such routes are there through a 20x20 grid?
; Unfortunately, we can't fit that number in 32 big integers.
; Instead we have to use Big integers (or numbers as a vectors with arbitrary precision):

(let factorial (lambda n total
        (if (= (get n 0) 0)
            total
            (factorial (std/int/big/sub n [ 1 ]) (std/int/big/mul total n)))))

(let bionomial-coefficient (lambda a b
    (std/int/big/div (factorial a [ 1 ])
            (std/int/big/mul
                (factorial b [ 1 ])
                (factorial (std/int/big/sub a b) [ 1 ])))))

(let m [ 2 0 ])
(let n [ 2 0 ])
(bionomial-coefficient (std/int/big/add m n) m)
; [Int]
; [1 3 7 8 4 6 5 2 8 8 2 0]

; Advent of Code 2015

; --- Day 1: Not Quite Lisp ---

; Santa is trying to deliver presents in a large apartment building, but he can't find the right floor - the directions he got are a little confusing. He starts on the ground floor (floor 0) and then follows the instructions one character at a time.
; An opening parenthesis, (, means he should go up one floor, and a closing parenthesis, ), means he should go down one floor.
; The apartment building is very tall, and the basement is very deep; he will never find the top or bottom floors.


; For example:

; (()) and ()() both result in floor 0.
; ((( and (()(()( both result in floor 3.
; ))((((( also results in floor 3.
; ()) and ))( both result in floor -1 (the first basement level).
; ))) and )())()) both result in floor -3.
; To what floor do the instructions take Santa?

(let samples [
    "(())"    ; result in floor 0.
    "()()"    ; result in floor 0.
    "((("     ; result in floor 3.
    "(()(()(" ; result in floor 3.
    "))(((((" ; also results in floor 3.
    "())"     ; result in floor -1 (the first basement level).
    "))("     ; result in floor -1 (the first basement level).
    ")))"     ; result in floor -3.
    ")())())" ; result in floor -3.
])
(let solve (lambda input (- (std/vector/char/count input std/char/left-brace) (std/vector/char/count input std/char/right-brace))))
(std/vector/map samples solve)
; [Int]
; [0 0 3 3 3 -1 -1 -3 -3]
`
  );
  terminal.clearSelection();
  terminal.focus();
});
