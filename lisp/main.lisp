(let greet (lambda name (do
    (let message (cons "Hello, " name "!"))
    message)))

(greet "Que")
; Returns: "Hello, Que!"