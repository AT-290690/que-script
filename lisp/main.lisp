; ; --
; (let *INPUT* 
; "2 3 4 5
; 3 4 5 2
; 4 5 2 3
; 5 2 3 4")
; ; --

; (let parse (lambda input (|> input (std/convert/string->vector std/int/char/new-line) (std/vector/map (lambda line (|> line (std/convert/string->vector std/int/char/space) (std/vector/map std/convert/chars->integer)))))))
; (|> *INPUT* (parse) (std/vector/map std/vector/int/sum) (std/convert/integer->string std/int/char/space))


(1 2)