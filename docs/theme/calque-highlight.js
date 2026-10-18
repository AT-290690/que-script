/* Calque syntax for mdBook's bundled Highlight.js instance. */
(function () {
  function calque(hljs) {
    return {
      name: "Calque",
      aliases: ["que", "eclisp"],
      keywords: {
        // Keep these word-level categories aligned with the TextMate grammar
        // used by the Calque VS Code extension.
        keyword: [
          "enum match sig extern block unless when when-not cond cons while alter mut do",
          "letrec let loop if lambda length as get at pop push pull swap emod not and or",
          "identity eq integer fixed boolean que string chars digits vector bigint tuple table",
          "fst snd car cdr has in-bounds apply comp matrix shell Vector String Matrix BigInt",
          "Tuple Table Set Hash Que Integer Decimal Boolean Heap map filter reduce every some",
          "select exclude find sum product slice take drop range flat",
          "set alter capacity serialize deserialize"
        ].join(" "),
        meta: "letmacro quote qq uq uqs gensym macroexpand macroexpand-1",
        built_in: [
          "trim enumerate nl sp ep dq sq bt cr empty copy reverse sort square sqrt expt floor",
          "ceil max min abs first last count points zip unzip permutation window pair lcm gcd",
          "call odd even upper lower digit fill clamp one zero ones zeroes positive negative",
          "invert divisible maximum minimum for subset scan extreme neighborhood unique transpose",
          "interleave intersperse cycle replicate avg mean median each const group append tail head",
          "split join replace list-dir delete mkdir move read write clear print sleep"
        ].join(" "),
        literal: "true false nil infinity",
        type: "Int Dec Bool Char"
      },
      contains: [
        hljs.COMMENT(";", "$"),
        hljs.QUOTE_STRING_MODE,
        {
          className: "string",
          begin: /'/,
          end: /'/,
          contains: [hljs.BACKSLASH_ESCAPE]
        },
        {
          className: "number",
          begin: /-?\b\d+(?:\.\d+)?\b/,
          relevance: 0
        },
        {
          className: "title",
          begin: /\b(?:Table|Set|Vector|String|Matrix|Heap|Que|BigInt|SignedBigInt)\/[A-Za-z0-9_?!*+\-]+/,
          relevance: 0
        },
        {
          className: "operator",
          begin: /(?:=~|\|>|<\||\+\+|--|[+*\/%=-]\.|[+*\/%=<>!&|^~?:.-])+/,
          relevance: 0
        }
      ]
    };
  }

  function install() {
    if (!window.hljs) return;
    window.hljs.registerLanguage("que", calque);
    window.hljs.registerLanguage("eclisp", calque);
    // mdBook 0.5 currently ships Highlight.js 9, whose public method is
    // `highlightBlock`; newer Highlight.js calls it `highlightElement`.
    var highlight = window.hljs.highlightElement || window.hljs.highlightBlock;
    if (!highlight) return;
    document.querySelectorAll("pre code.language-que, pre code.language-eclisp").forEach(function (block) {
      block.removeAttribute("data-highlighted");
      highlight.call(window.hljs, block);
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", install);
  } else {
    install();
  }
}());
