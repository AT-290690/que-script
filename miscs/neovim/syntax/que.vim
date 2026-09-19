if exists("b:current_syntax")
  finish
endif

syn case match

syn match queComment /;.*/

syn region queString start=/"/ skip=/\\"/ end=/"/
syn region queChar start=/'/ skip=/\\'/ end=/'/

syn match queNumber /\v<[-+]?\d+(\.\d+)?>/

" --- Keywords (Control Flow & Structure) ---
syn keyword queKeyword
      \ lambda comp if let letrec letmacro mut do block while
      \ cond unless when when-not loop and or not quote qq uq uqs gensym
      \ fst snd
      \ macroexpand macroexpand-1 as sig

" Keyword forms containing punctuation cannot be expressed reliably with
" :syn keyword because characters such as &, ! and . are not keyword chars.
syn match queKeyword /\%(^\|\s\|(\|\[\|{\)\zs\%(&mut\|alter!\|&alter!\|mod\.\|mod\)\ze\%($\|\s\|)\|\]\|}\)/

" --- Builtins (Functions, Core Operations & Mutations) ---
syn keyword queBuiltin
      \ length car cdr cons get

" Regexp matching for builtins containing symbols like ! & |
syn match queBuiltin /\v(\&get|set!|push!|pop!|pop-val!)/
syn match queBuiltin /\v(<\||\|>)/

" --- Literals & Delimiters ---
syn match queBoolean /\v<(true|false|nil)>/
syn match queDelimiter /[()\[\]{}]/

" --- Highlighting Links ---
hi def link queComment Comment
hi def link queString String
hi def link queChar Character
hi def link queNumber Number
hi def link queKeyword Keyword
hi def link queBuiltin Function
hi def link queBoolean Boolean
hi def link queDelimiter Delimiter

let b:current_syntax = "que"
