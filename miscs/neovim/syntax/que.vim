if exists("b:current_syntax")
  finish
endif

syn case match

syn match queComment /;.*/

syn region queString start=/"/ skip=/\\"/ end=/"/
syn region queChar start=/'/ skip=/\\'/ end=/'/

syn match queNumber /\v<[-+]?\d+(\.\d+)?>/

syn keyword queKeyword
      \ lambda if let letrec letmacro mut do block while
      \ cond unless and or not quote qq uq uqs gensym
      \ macroexpand macroexpand-1 as

syn match queBuiltin /\v(\&alter!|\&get|\&mut|alter!|set!|push!|pop!|pop-val!)/
syn match queBuiltin /\v(<\||\|>)/

syn match queBoolean /\v<(true|false|nil)>/
syn match queDelimiter /[()\[\]{}]/

hi def link queComment Comment
hi def link queString String
hi def link queChar Character
hi def link queNumber Number
hi def link queKeyword Keyword
hi def link queBuiltin Function
hi def link queBoolean Boolean
hi def link queDelimiter Delimiter

let b:current_syntax = "que"
