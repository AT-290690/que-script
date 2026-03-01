; --- Lazy Command Builders (Strings Only) ---
(let str/quote (lambda str (cons "'" str "'")))

(let cmd/ls (lambda dir (cons "ls " dir)))
(let cmd/rm (lambda file (cons "rm " file)))
(let cmd/mkdir (lambda dir (cons "mkdir -p " dir)))

; Writing/Appending - Notice the flat cons!
(let cmd/owr (lambda file data (cons "echo " (str/quote data) " > " file)))
(let cmd/app (lambda file data (cons "echo " (str/quote data) " >> " file)))
(let cmd/echo (lambda x (cons "echo " x)))
; Logic/Chaining
(let cmd/and (lambda a b (cons a " && " b)))
(let cmd/or  (lambda a b (cons a " || " b)))

; The Pipe Operator ( | )
(let cmd/pipe (lambda a b (cons a " | " b)))

; curl -sL (silent, follow redirects)
(let fetch (lambda url (cons "curl -sL " (str/quote url))))

; A "log" helper (using cat to print from stdin)
(let cmd/console-log (lambda "cat")) 

; Logic Helpers
(let cmd/if-exists (lambda file cmd (cons "test -f " file " && " cmd)))
(let cmd/run-fez   (lambda file (cons "./fez " file)))

; --- Lazy Command Builders ---
; Builds: curl -X POST -d 'data' url
(let cmd/curl-post (lambda url data 
  (cons "curl -X POST -d " (str/quote data) " " (str/quote url))))

; Builds: curl -o filename url
(let cmd/download (lambda url file 
  (cons "curl -L -o " file " " (str/quote url))))
