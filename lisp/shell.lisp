; --- Lazy Command Builders (Strings Only) ---
(let str/quote (lambda str (cons "'" str "'")))
(let str/dquote (lambda str (cons [dq] str [dq])))
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
(let cmd/eval (lambda file (cons "que" " " file)))
(let cmd/args (lambda args (Vector->String ' ' (map str/dquote args))))
; --- Lazy Command Builders ---
; Builds: curl -X POST -d 'data' url
(let cmd/curl-post (lambda url data 
  (cons "curl -X POST -d " (str/quote data) " " (str/quote url))))
; Builds: curl -o filename url
(let cmd/download (lambda url file 
  (cons "curl -L -o " file " " (str/quote url))))
; Check if a directory exists
(let cmd/dir-exists? (lambda dir (cons "test -d " dir)))
; Get file size in bytes
(let cmd/file-size (lambda file (cons "stat -c%s " file)))
; Touch a file (create empty or update timestamp)
(let cmd/touch (lambda file (cons "touch " file)))
; curl with a custom header (e.g., Content-Type: application/json)
(let cmd/curl-header (lambda url header (cons "curl -sL -H " (str/quote header) " " (str/quote url))))
; Simple Bearer Auth fetch
(let cmd/curl-auth (lambda url token (cons "curl -sL -H " (str/quote (cons "Authorization: Bearer " token)) " " (str/quote url))))
; Search for a string in a file or stream
(let cmd/grep (lambda pattern (cons "grep " (str/quote pattern))))
; Count lines (great for checking if a list is empty)
(let cmd/count-lines (lambda "wc -l"))
; Sort output alphabetically
(let cmd/sort (lambda "sort"))
; Remove duplicate lines
(let cmd/uniq (lambda "uniq"))
; Get the current user
(let cmd/whoami (lambda "whoami"))
; Sleep/Delay (useful for retrying fetches)
(let cmd/sleep (lambda sec (cons "sleep " (Integer->String sec))))
(let cmd/head (lambda file n 
  (cons "head -n " (Integer->String n) " " file)))
; Print the last N lines of a file
(let cmd/tail (lambda file n (cons "tail -n " (Integer->String n) " " file)))
; Wait/Delay for N seconds
(let cmd/sleep (lambda n 
  (cons "sleep " (Integer->String n))))
; curl with a max timeout in seconds
(let fetch/timeout (lambda url sec 
  (cons "curl -sL --max-time " (Integer->String sec) " " (str/quote url))))
; curl that retries N times if it fails
(let fetch/retry (lambda url n 
  (cons "curl -sL --retry " (Integer->String n) " " (str/quote url))))
; Get a specific line number from a stream (using sed)
(let cmd/line-at (lambda n 
  (cons "sed -n " (Integer->String n) "p")))
; Limit output to N characters (using cut)
(let cmd/limit-chars (lambda n 
  (cons "cut -c1-" (Integer->String n))))
; Check if a file is LARGER than N bytes
(let cmd/file-min-size? (lambda file n 
  (cons "[ $(stat -c%s " file ") -gt " (Integer->String n) " ]")))
; Run a command N times (Bash loop builder)
(let cmd/repeat (lambda n cmd 
  (cons "for i in {1.." (Integer->String n) "}; do " cmd "; done")))
; Change permissions to a specific octal (e.g. 755)
(let cmd/chmod (lambda mode file 
  (cons "chmod " (Integer->String mode) " " file)))
