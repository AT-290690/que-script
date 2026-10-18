# Syntax and literals

Calque uses S-expressions. Whitespace separates forms; parentheses make a
call or special form; square brackets make a vector; braces make a tuple.

```que
(+ 1 2)
(let xs [1 2 3])
(let point {10 20})
```

## Literals

```que
42             ; Int
-7             ; Int
3.14           ; Dec
true           ; Bool
false          ; Bool
nil            ; unit, type ()
'a'            ; Char
'\n'           ; newline Char
"hello"        ; [Char]
```

Comments begin with `;` and end at the line boundary:

```que
; this is a comment
(let answer 42) ; so is this
```

## Forms and patterns

```que
(lambda (left right) (+ left right))
(let {x y} {10 20})
(let [first second] [10 20])
(let [head . tail] [1 2 3])
```

The final rest pattern is primarily useful in parameter/vector destructuring.
When destructuring requires elements, static analysis may warn unless it can
prove the source vector has sufficient length.

## Operators are names

Operators are ordinary callable names in prefix position. Their suffix
communicates their type family:

```que
(+ 1 2)        ; Int
(+. 1.5 2.0)   ; Dec
(+# 'A' 1)     ; Char arithmetic
(= 2 2)        ; Int equality
(=. 2.0 2.0)   ; Dec equality
(=# 'x' 'x')   ; Char equality
(=? true true) ; Bool equality
```

Enum equality is intentionally separate: `(=~ A B)`.
