; Lists files and directories. In que, this is usually piped into 
; other commands to process file lists.
(let cmd/ls (lambda dir (cons "ls " dir)))
; Deletes a file. Caution: this is permanent and does not use a trash bin.
(let cmd/rm (lambda file (cons "rm " file)))
; Creates a directory path. The '-p' flag ensures that parent directories 
; are created if they don't exist and prevents errors if the folder already exists.
(let cmd/mkdir (lambda dir (cons "mkdir -p " dir)))
; Writing/Appending - Notice the flat cons!
(let cmd/owr (lambda file data (cons "echo " (String/quote data) " > " file)))
; Appends data to the end of a file without overwriting existing content.
; Uses the '>>' operator to ensure the file grows rather than being replaced.
(let cmd/app (lambda file data (cons "echo " (String/quote data) " >> " file)))
; Print a string directly to the terminal for status updates or logging progress within a 'que' pipel
(let cmd/echo (lambda x (cons "echo " x)))
; Logic/Chaining
(let cmd/and (lambda a b (cons a " && " b)))
(let cmd/or  (lambda a b (cons a " || " b)))
; The Pipe Operator ( | )
(let cmd/pipe (lambda a b (cons a " | " b)))
; curl -sL (silent, follow redirects)
(let fetch (lambda url (cons "curl -sL " (String/quote url))))
; A "log" helper (using cat to print from stdin)
(let cmd/console-log (lambda "cat")) 
; Logic Helpers
(let cmd/if-exists (lambda file cmd (cons "test -f " file " && " cmd)))
; Evaluate que file
(let cmd/eval (lambda file (cons "que" " " file)))
; Create args string from vector of args
(let cmd/args (lambda args (Vector->String ' ' (map String/dquote args))))
; --- Lazy Command Builders ---
; Builds: curl -X POST -d 'data' url
(let cmd/curl-post (lambda url data 
  (cons "curl -X POST -d " (String/quote data) " " (String/quote url))))
; Builds: curl -o filename url
(let cmd/download (lambda url file 
  (cons "curl -L -o " file " " (String/quote url))))
; Check if a directory exists
(let cmd/dir-exists? (lambda dir (cons "test -d " dir)))
; Get file size in bytes
(let cmd/file-size (lambda file (cons "stat -c%s " file)))
; Touch a file (create empty or update timestamp)
(let cmd/touch (lambda file (cons "touch " file)))
; curl with a custom header (e.g., Content-Type: application/json)
(let cmd/curl-header (lambda url header (cons "curl -sL -H " (String/quote header) " " (String/quote url))))
; Simple Bearer Auth fetch
(let cmd/curl-auth (lambda url token (cons "curl -sL -H " (String/quote (cons "Authorization: Bearer " token)) " " (String/quote url))))
; Search for a string in a file or stream
(let cmd/grep (lambda pattern (cons "grep " (String/quote pattern))))
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
; Extracts the first N lines of a file. 
(let cmd/head (lambda file n 
  (cons "head -n " (Integer->String n) " " file)))
; Extracts the last N lines of a file
(let cmd/tail (lambda file n (cons "tail -n " (Integer->String n) " " file)))
; Wait/Delay for N seconds
(let cmd/sleep (lambda n 
  (cons "sleep " (Integer->String n))))
; curl with a max timeout in seconds
(let fetch/timeout (lambda url sec 
  (cons "curl -sL --max-time " (Integer->String sec) " " (String/quote url))))
; curl that retries N times if it fails
(let fetch/retry (lambda url n 
  (cons "curl -sL --retry " (Integer->String n) " " (String/quote url))))
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
; Remove a file ONLY if it exists (avoids "no such file" errors)
(let cmd/rm-f (lambda file (cons "rm -f " file)))
; Remove a directory and everything inside it (Recursive + Force)
(let cmd/rm-rf (lambda dir (cons "rm -rf " dir)))
; Logical NOT (Check if a file does NOT exist)
(let cmd/if-not-exists (lambda file cmd (cons "! test -f " file " && " cmd)))
; Convert all text to lowercase (great for normalization)
(let cmd/lower (lambda "tr '[:upper:]' '[:lower:]'"))
; Extract the Nth word/column from a line (using awk)
(let cmd/column (lambda n (cons "awk '{print $" (Integer->String n) "}'")))
; Replace a string with another (simple sed replacement)
(let cmd/replace (lambda old new (cons "sed 's/" old "/" new "/g'")))
; Get your public IP address
(let cmd/my-ip (lambda "curl -s https://ifconfig.me"))
; Check if a website is "up" (returns only the HTTP status code like 200)
(let cmd/http-status (lambda url (cons "curl -s -o /dev/null -w '%{http_code}' " (String/quote url))))
; Find out where a command is located (e.g. 'which que')
(let cmd/which (lambda cmd (cons "which " cmd)))
; Run a command in the background (disown so it lives after que exits)
(let cmd/background (lambda cmd (cons "nohup " cmd " > /dev/null 2>&1 &")))
; Kill a process by name (dangerous, but useful for 'que' upgrades)
(let cmd/pkill (lambda name (cons "pkill -f " name)))
; Get memory usage of the system (using free)
(let cmd/mem-free (lambda "free -h"))
; Check if a specific process is running (returns PID or empty)
(let cmd/pgrep (lambda name (cons "pgrep -f " name)))
; Force kill a process by name
(let cmd/pkill (lambda name (cons "pkill -9 -f " name)))
; Top 5 CPU consuming processes
(let cmd/top-cpu (lambda "ps -eo pcpu,comm --sort=-pcpu | head -n 6"))
; Check disk space usage on the current partition
(let cmd/df (lambda "df -h ."))
; Find files larger than N megabytes
(let cmd/find-large (lambda dir n 
  (cons "find " dir " -type f -size +" (Integer->String n) "M")))
; Count how many files are in a directory
(let cmd/file-count (lambda dir 
  (cmd/pipe (cmd/ls dir) (cmd/count-lines))))
; Check if a local port is in use (e.g. 8080)
(let cmd/port-in-use? (lambda port 
  (cons "lsof -i :" (Integer->String port))))
; Get the local IP address (Internal network)
(let cmd/local-ip (lambda "hostname -I | awk '{print $1}'"))
; Ping a host once to check latency
(let cmd/ping-once (lambda host (cons "ping -c 1 " host)))
; Wait for a file to appear (checks every second for 10 seconds)
(let cmd/wait-for-file (lambda file 
  (cmd/repeat 10 
    (cmd/or (cmd/if-exists file (cmd/echo "Found!")) 
            (cmd/sleep 1)))))
; Get the Kernel version
(let cmd/kernel (lambda "uname -r"))
; Check if the current shell is Zsh or Bash
(let cmd/shell-type (lambda "echo $SHELL"))
; Get the number of CPU cores (useful for parallel builds)
(let cmd/cpu-cores (lambda "nproc"))