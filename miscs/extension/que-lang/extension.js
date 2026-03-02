const standardLibrary = [
  ["ARGV", "[T]", "Command line arguments"],
  ["shell", "[Char] -> [Char]", "Execute shell command", [`(shell "ls .")`]],
  [
    "String/quote",
    "[Char] -> [Char]",
    "Wrap string in single quotes",
    [`(String/quote "hello world"); => 'hello world'`],
  ],
  [
    "String/dquote",
    "[Char] -> [Char]",
    "Wrap string in double quotes",
    [`(String/dquote "hello world"); => "hello world"`],
  ],
  [
    "cmd/ls",
    "[Char] -> [Char]",
    "Lists files and directories in a given path",
    ['(cmd/ls "/home/user"); => ls /home/user'],
  ],
  [
    "cmd/rm",
    "[Char] -> [Char]",
    "Removes a specific file from the filesystem",
    ['(cmd/rm "temp.log"); => rm temp.log'],
  ],
  [
    "cmd/mkdir",
    "[Char] -> [Char]",
    "Creates a directory path, including parents if missing",
    ['(cmd/mkdir "build/logs"); => mkdir -p build/logs'],
  ],
  [
    "cmd/owr",
    "[Char] -> [Char] -> [Char]",
    "Overwrites a file with the provided data string",
    ['(cmd/owr "version.txt" "1.0.0"); => echo \'1.0.0\' > version.txt'],
  ],
  [
    "cmd/app",
    "[Char] -> [Char] -> [Char]",
    "Appends data to the end of an existing file",
    ['(cmd/app "events.log" "New Event"); => echo \'New Event\' >> events.log'],
  ],
  [
    "cmd/echo",
    "[Char] -> [Char]",
    "Prints a string to the standard output",
    ['(cmd/echo "Hello World"); => echo Hello World'],
  ],
  [
    "cmd/and",
    "[Char] -> [Char] -> [Char]",
    "Chains two commands: runs the second only if the first succeeds",
    ['(cmd/and (cmd/mkdir "d") (cmd/ls "d")); => mkdir -p d && ls d'],
  ],
  [
    "cmd/or",
    "[Char] -> [Char] -> [Char]",
    "Chains two commands: runs the second only if the first fails",
    ['(cmd/or (cmd/ls "d") (cmd/echo "fail")); => ls d || echo fail'],
  ],
  [
    "cmd/pipe",
    "[Char] -> [Char] -> [Char]",
    "Pipes the output of the first command as input to the second",
    ['(cmd/pipe (cmd/ls ".") (cmd/count-lines)); => ls . | wc -l'],
  ],
  [
    "fetch",
    "[Char] -> [Char]",
    "Downloads the content of a URL (GET request)",
    ["(fetch \"http://api.com\"); => curl -sL 'http://api.com'"],
  ],
  [
    "cmd/console-log",
    "() -> [Char]",
    "Streams standard input directly to the console via cat",
    ["(cmd/console-log); => cat"],
  ],
  [
    "cmd/if-exists",
    "[Char] -> [Char] -> [Char]",
    "Executes a command only if the specified file exists",
    [
      '(cmd/if-exists "config.que" (cmd/eval "config.que")); => test -f config.que && que config.que',
    ],
  ],
  [
    "cmd/eval",
    "[Char] -> [Char]",
    "Runs the 'que' interpreter on a specific file",
    ['(cmd/eval "script.que"); => que script.que'],
  ],
  [
    "cmd/args",
    "[[Char]] -> [Char]",
    "Formats a list of strings into quoted shell arguments",
    [`(cmd/args ["a" "b"]); => "a" "b"`],
  ],
  [
    "cmd/curl-post",
    "[Char] -> [Char] -> [Char]",
    "Sends a POST request with a data body to a URL",
    ['(cmd/curl-post "url" "data"); => curl -X POST -d \'data\' url'],
  ],
  [
    "cmd/download",
    "[Char] -> [Char] -> [Char]",
    "Downloads a file from a URL to a local destination",
    ['(cmd/download "url" "file"); => curl -L -o file \'url\''],
  ],
  [
    "cmd/dir-exists?",
    "[Char] -> [Char]",
    "Tests if a directory exists at the given path",
    ['(cmd/dir-exists? "/bin"); => test -d /bin'],
  ],
  [
    "cmd/file-size",
    "[Char] -> [Char]",
    "Returns the size of a file in bytes",
    ['(cmd/file-size "img.png"); => stat -c%s img.png'],
  ],
  [
    "cmd/touch",
    "[Char] -> [Char]",
    "Creates an empty file or updates the timestamp of an existing one",
    ['(cmd/touch "keep-alive"); => touch keep-alive'],
  ],
  [
    "cmd/curl-header",
    "[Char] -> [Char] -> [Char]",
    "Performs a GET request with a custom HTTP header",
    [
      "(cmd/curl-header \"url\" \"X-Api-Key: 123\"); => curl -sL -H 'X-Api-Key: 123' 'url'",
    ],
  ],
  [
    "cmd/curl-auth",
    "[Char] -> [Char] -> [Char]",
    "Performs a GET request using a Bearer token for authentication",
    [
      "(cmd/curl-auth \"url\" \"token123\"); => curl -sL -H 'Authorization: Bearer token123' 'url'",
    ],
  ],
  [
    "cmd/grep",
    "[Char] -> [Char]",
    "Filters input lines that match a specific pattern",
    ["(cmd/grep \"error\"); => grep 'error'"],
  ],
  [
    "cmd/count-lines",
    "() -> [Char]",
    "Counts the total number of lines in the input",
    ["(cmd/count-lines); => wc -l"],
  ],
  [
    "cmd/sort",
    "() -> [Char]",
    "Sorts input lines alphabetically",
    ["(cmd/sort); => sort"],
  ],
  [
    "cmd/uniq",
    "() -> [Char]",
    "Filters out adjacent duplicate lines from the input",
    ["(cmd/uniq); => uniq"],
  ],
  [
    "cmd/whoami",
    "() -> [Char]",
    "Returns the username of the current effective user",
    ["(cmd/whoami); => whoami"],
  ],
  [
    "cmd/sleep",
    "Int -> [Char]",
    "Suspends execution for a specific number of seconds",
    ["(cmd/sleep 5); => sleep 5"],
  ],
  [
    "cmd/head",
    "[Char] -> Int -> [Char]",
    "Returns the first N lines of a file",
    ['(cmd/head "log.txt" 10); => head -n 10 log.txt'],
  ],
  [
    "cmd/tail",
    "[Char] -> Int -> [Char]",
    "Returns the last N lines of a file",
    ['(cmd/tail "log.txt" 5); => tail -n 5 log.txt'],
  ],
  [
    "fetch/timeout",
    "[Char] -> Int -> [Char]",
    "Fetches a URL with a maximum time limit in seconds",
    ["(fetch/timeout \"url\" 2); => curl -sL --max-time 2 'url'"],
  ],
  [
    "fetch/retry",
    "[Char] -> Int -> [Char]",
    "Fetches a URL and retries N times on failure",
    ["(fetch/retry \"url\" 3); => curl -sL --retry 3 'url'"],
  ],
  [
    "cmd/line-at",
    "Int -> [Char]",
    "Extracts a specific line number from the input stream",
    ["(cmd/line-at 2); => sed -n 2p"],
  ],
  [
    "cmd/limit-chars",
    "Int -> [Char]",
    "Truncates input lines to a maximum number of characters",
    ["(cmd/limit-chars 80); => cut -c1-80"],
  ],
  [
    "cmd/file-min-size?",
    "[Char] -> Int -> [Char]",
    "Checks if a file is larger than the specified byte size",
    ['(cmd/file-min-size? "db.sql" 1024); => [ $(stat -c%s db.sql) -gt 1024 ]'],
  ],
  [
    "cmd/repeat",
    "Int -> [Char] -> [Char]",
    "Generates a shell loop to run a command N times",
    ['(cmd/repeat 3 (cmd/echo "hi")); => for i in {1..3}; do echo hi; done'],
  ],
  [
    "cmd/chmod",
    "Int -> [Char] -> [Char]",
    "Changes file permissions using an octal mode",
    ['(cmd/chmod 755 "run.sh"); => chmod 755 run.sh'],
  ],
  [
    "cmd/rm-f",
    "[Char] -> [Char]",
    "Forcefully removes a file, ignoring if it doesn't exist",
    ['(cmd/rm-f "maybe.tmp"); => rm -f maybe.tmp'],
  ],
  [
    "cmd/rm-rf",
    "[Char] -> [Char]",
    "Forcefully and recursively removes a directory and its contents",
    ['(cmd/rm-rf "dist"); => rm -rf dist'],
  ],
  [
    "cmd/if-not-exists",
    "[Char] -> [Char] -> [Char]",
    "Executes a command only if the specified file does not exist",
    [
      '(cmd/if-not-exists "lock" (cmd/touch "lock")); => ! test -f lock && touch lock',
    ],
  ],
  [
    "cmd/lower",
    "() -> [Char]",
    "Converts all input text to lowercase",
    ["(cmd/lower); => tr '[:upper:]' '[:lower:]'"],
  ],
  [
    "cmd/column",
    "Int -> [Char]",
    "Extracts the Nth column from whitespace-separated input",
    ["(cmd/column 2); => awk '{print $2}'"],
  ],
  [
    "cmd/replace",
    "[Char] -> [Char] -> [Char]",
    "Replaces all occurrences of a string with another",
    ['(cmd/replace "foo" "bar"); => sed \'s/foo/bar/g\''],
  ],
  [
    "cmd/my-ip",
    "() -> [Char]",
    "Retrieves the public IP address of the machine",
    ["(cmd/my-ip); => curl -s https://ifconfig.me"],
  ],
  [
    "cmd/http-status",
    "[Char] -> [Char]",
    "Returns only the HTTP status code of a URL request",
    [
      "(cmd/http-status \"url\"); => curl -s -o /dev/null -w '%{http_code}' 'url'",
    ],
  ],
  [
    "cmd/which",
    "[Char] -> [Char]",
    "Locates the executable path of a command in the PATH",
    ['(cmd/which "que"); => which que'],
  ],
  [
    "cmd/background",
    "[Char] -> [Char]",
    "Runs a command in the background (detached from terminal)",
    [
      '(cmd/background (cmd/eval "s.que")); => nohup que s.que > /dev/null 2>&1 &',
    ],
  ],
  [
    "cmd/pkill",
    "[Char] -> [Char]",
    "Terminates all processes matching the given name",
    ['(cmd/pkill "que"); => pkill -9 -f que'],
  ],
  [
    "cmd/mem-free",
    "() -> [Char]",
    "Displays system memory usage in a human-readable format",
    ["(cmd/mem-free); => free -h"],
  ],
  [
    "cmd/pgrep",
    "[Char] -> [Char]",
    "Searches for processes by name and returns their PIDs",
    ['(cmd/pgrep "que"); => pgrep -f que'],
  ],
  [
    "cmd/top-cpu",
    "() -> [Char]",
    "Returns the top 5 CPU-consuming processes",
    ["(cmd/top-cpu); => ps -eo pcpu,comm --sort=-pcpu | head -n 6"],
  ],
  [
    "cmd/df",
    "() -> [Char]",
    "Displays disk space usage for the current partition",
    ["(cmd/df); => df -h ."],
  ],
  [
    "cmd/find-large",
    "[Char] -> Int -> [Char]",
    "Finds files in a directory larger than N megabytes",
    ['(cmd/find-large "." 50); => find . -type f -size +50M'],
  ],
  [
    "cmd/file-count",
    "[Char] -> [Char]",
    "Counts the number of files within a directory",
    ['(cmd/file-count "src"); => ls src | wc -l'],
  ],
  [
    "cmd/port-in-use?",
    "Int -> [Char]",
    "Checks if a specific network port is currently active",
    ["(cmd/port-in-use? 8080); => lsof -i :8080"],
  ],
  [
    "cmd/local-ip",
    "() -> [Char]",
    "Retrieves the internal (LAN) IP address of the machine",
    ["(cmd/local-ip); => hostname -I | awk '{print $1}'"],
  ],
  [
    "cmd/ping-once",
    "[Char] -> [Char]",
    "Sends a single ICMP echo request to a host",
    ['(cmd/ping-once "google.com"); => ping -c 1 google.com'],
  ],
  [
    "cmd/wait-for-file",
    "[Char] -> [Char]",
    "Polls until a file exists, checking every second for 10 seconds",
    [
      '(cmd/wait-for-file "done"); => for i in {1..10}; do test -f done && echo Found || sleep 1; done',
    ],
  ],
  [
    "cmd/kernel",
    "() -> [Char]",
    "Returns the current operating system kernel version",
    ["(cmd/kernel); => uname -r"],
  ],
  [
    "cmd/shell-type",
    "() -> [Char]",
    "Returns the path of the current user's shell",
    ["(cmd/shell-type); => echo $SHELL"],
  ],
  [
    "cmd/cpu-cores",
    "() -> [Char]",
    "Returns the number of available CPU cores (for Linux only)",
    ["(cmd/cpu-cores); => nproc"],
  ],
  ["+", "Int -> Int -> Int"],
  ["+#", "Char -> Char -> Char"],
  ["+.", "Float -> Float -> Float"],
  ["-", "Int -> Int -> Int"],
  ["-#", "Char -> Char -> Char"],
  ["-.", "Float -> Float -> Float"],
  ["/", "Int -> Int -> Int"],
  ["/#", "Char -> Char -> Char"],
  ["/.", "Float -> Float -> Float"],
  ["*", "Int -> Int -> Int"],
  ["*#", "Char -> Char -> Char"],
  ["*.", "Float -> Float -> Float"],
  ["mod", "Int -> Int -> Int"],
  ["mod.", "Float -> Float -> Float"],
  ["=", "Int -> Int -> Bool"],
  ["=?", "Bool -> Bool -> Bool"],
  ["=#", "Char -> Char -> Bool"],
  ["=.", "Float -> Float -> Bool"],
  ["<", "Int -> Int -> Bool"],
  ["<#", "Char -> Char -> Bool"],
  ["<.", "Float -> Float -> Bool"],
  [">", "Int -> Int -> Bool"],
  [">#", "Char -> Char -> Bool"],
  [">.", "Float -> Float -> Bool"],
  ["<=", "Int -> Int -> Bool"],
  ["<=#", "Char -> Char -> Bool"],
  ["<=.", "Float -> Float -> Bool"],
  [">=", "Int -> Int -> Bool"],
  [">=#", "Char -> Char -> Bool"],
  [">=.", "Float -> Float -> Bool"],
  ["not", "Bool -> Bool"],
  ["and", "Bool -> Bool -> Bool"],
  ["or", "Bool -> Bool -> Bool"],
  ["^", "Int -> Int -> Int"],
  [">>", "Int -> Int -> Int"],
  ["<<", "Int -> Int -> Int"],
  ["|", "Int -> Int -> Int"],
  ["&", "Int -> Int -> Int"],
  ["~", "Int -> Int"],
  ["true", "Bool"],
  ["false", "Bool"],
  [
    "length",
    "[T13] -> Int",
    "Returns the number of elements in a list or vector",
    ["(length [1 2 3]); => 3"],
  ],
  [
    "car",
    "[T13] -> T13",
    "Returns the first element (head) of a list",
    ["(car [10 20 30]); => 10"],
  ],
  [
    "cdr",
    "[T13] -> Int -> [T13]",
    "Returns a new list containing the remaining elements after skipping N elements",
    ["(cdr [1 2 3 4] 1); => [2 3 4]", "(cdr [1 2 3 4] 2); => [3 4]"],
  ],
  [
    "Int->Float",
    "Int -> Float",
    "Converts an integer to a floating-point number",
    ["(Int->Float 5); => 5.0"],
  ],
  [
    "Float->Int",
    "Float -> Int",
    "Truncates a floating-point number to an integer",
    ["(Float->Int 5.9); => 5"],
  ],
  ["*Id*", "[Int]"],
  ["Id", "() -> Int"],
  [
    "const/int/max-safe",
    "Int",
    "The maximum value for a signed 32-bit integer (2^31 - 1)",
    ["const/int/max-safe; => 2147483647"],
  ],
  [
    "const/int/min-safe",
    "Int",
    "The minimum value for a signed 32-bit integer -(2^31)",
    ["const/int/min-safe; => -2147483648"],
  ],
  [
    "const/float/max-safe",
    "Float",
    "The largest finite 32-bit floating-point value",
    ["const/float/max-safe; => 3.4028235e+38"],
  ],
  [
    "const/float/min-safe",
    "Float",
    "The smallest positive 32-bit floating-point value",
    ["const/float/min-safe; => 1.17549435e-38"],
  ],
  [
    "const/float/pi",
    "Float",
    "The mathematical constant π (pi) as a 32-bit float",
    ["const/float/pi; => 3.1415927"],
  ],
  [
    "infinity",
    "Int",
    "Representation of positive infinity (typically 32-bit float INF cast to Int)",
    ["infinity; => 2147483647"],
  ],
  [
    "-infinity",
    "Int",
    "Representation of negative infinity (typically 32-bit float -INF cast to Int)",
    ["-infinity; => -2147483648"],
  ],
  ["Int", "Int"],
  ["Float", "Float"],
  ["Char", "Char"],
  ["Bool", "Bool"],
  [
    "nil",
    "()",
    "The value representation of unit type. Returns 0 but can't be used really.",
  ],
  ["Nil", "()"],
  ["as", "T15 -> T16 -> T16"],
  [":", "T15 -> T15 -> T15"],
  ["eq", "Bool -> Bool -> Bool"],
  [
    "identity",
    "T14 -> T14",
    "Returns the argument exactly as provided without modification",
    ["(identity 42); => 42", "(identity [1 2]); => [1 2]"],
  ],
  ["Char/nil", "Char"],
  ["Char/start", "Char"],
  ["Char/end", "Char"],
  ["nl", "Char", "New-line character"],
  ["sp", "Char", "Space character"],
  ["ep", "Char", "Empty character"],
  ["dq", "Char", "Double-quote character"],
  ["sq", "Char", "Single-quote character"],
  ["bt", "Char", "Backtick character"],
  [
    "box",
    "T14 -> [T14]",
    "Creates a new mutable box containing the initial value",
    ["(let x (box 10)); => [10]"],
  ],
  [
    "set",
    "[T20] -> T20 -> ()",
    "Updates the value inside a box to a new value",
    ["(let x (box 10)) (set x 20); => ()", "x; => [20]"],
  ],
  [
    "=!",
    "[T20] -> T20 -> ()",
    "Alias for set; performs an immediate destructive update to a box",
    ["(let x (box 1)) (=! x 2); => ()", "x; => [2]"],
  ],
  [
    "boole-set",
    "[Bool] -> Bool -> ()",
    "Mutates a boolean box with a new boolean value",
    ["(let b (box true)) (boole-set b false); => ()"],
  ],
  [
    "boole-eqv",
    "[Bool] -> [Bool] -> Bool",
    "Checks if the values inside two boolean boxes are equivalent",
    ["(boole-eqv (box true) (box true)); => true"],
  ],
  [
    "boolean/set",
    "[Bool] -> Bool -> ()",
    "Namespace alias for boole-set; mutates a boolean box",
    ["(boolean/set (box false) true); => ()"],
  ],
  [
    "true?",
    "[Bool] -> Bool",
    "Checks if the value inside a boolean box is true",
    ["(true? (box true)); => true"],
  ],
  [
    "false?",
    "[Bool] -> Bool",
    "Checks if the value inside a boolean box is false",
    ["(false? (box true)); => false"],
  ],
  [
    "+=",
    "[Int] -> Int -> ()",
    "Increments the value in an integer box by the given amount",
    ["(let n (box 5)) (+= n 3); => ()", "n; => [8]"],
  ],
  [
    "-=",
    "[Int] -> Int -> ()",
    "Decrements the value in an integer box by the given amount",
    ["(let n (box 10)) (-= n 4); => ()", "n; => [6]"],
  ],
  [
    "*=",
    "[Int] -> Int -> ()",
    "Multiplies the value in an integer box by the given amount",
    ["(let n (box 3)) (*= n 4); => ()", "n; => [12]"],
  ],
  [
    "/=",
    "[Int] -> Int -> ()",
    "Divides the value in an integer box by the given amount",
    ["(let n (box 12)) (/= n 3); => ()", "n; => [4]"],
  ],
  [
    "++",
    "[Int] -> ()",
    "Increments the value in an integer box by 1",
    ["(let n (box 0)) (++); => ()", "n; => [1]"],
  ],
  [
    "--",
    "[Int] -> ()",
    "Decrements the value in an integer box by 1",
    ["(let n (box 5)) (-- n); => ()", "n; => [4]"],
  ],
  [
    "**",
    "[Int] -> ()",
    "Squares the value in an integer box",
    ["(let n (box 4)) (** n); => ()", "n; => [16]"],
  ],
  [
    "+=.",
    "[Float] -> Float -> ()",
    "Increments a float box by the given float amount",
    ["(let f (box 1.5)) (+=. f 2.5); => ()", "f; => [4.0]"],
  ],
  [
    "-=.",
    "[Float] -> Float -> ()",
    "Decrements a float box by the given float amount",
    ["(let f (box 5.0)) (-=. f 1.0); => ()", "f; => [4.0]"],
  ],
  [
    "*=.",
    "[Float] -> Float -> ()",
    "Multiplies a float box by the given float amount",
    ["(let f (box 2.0)) (*=. f 3.0); => ()", "f; => [6.0]"],
  ],
  [
    "/=.",
    "[Float] -> Float -> ()",
    "Divides a float box by the given float amount",
    ["(let f (box 10.0)) (/=. f 2.0); => ()", "f; => [5.0]"],
  ],
  [
    "++.",
    "[Float] -> ()",
    "Increments the value in a float box by 1.0",
    ["(let f (box 1.1)) (++. f); => ()", "f; => [2.1]"],
  ],
  [
    "--.",
    "[Float] -> ()",
    "Decrements the value in a float box by 1.0",
    ["(let f (box 2.5)) (--. f); => ()", "f; => [1.5]"],
  ],
  [
    "**.",
    "[Float] -> ()",
    "Squares the value in a float box",
    ["(let f (box 3.0)) (**. f); => ()", "f; => [9.0]"],
  ],
  [
    "Bool->Int",
    "Bool -> Int",
    "Converts a boolean to its integer representation",
    ["(Bool->Int true); => 1", "(Bool->Int false); => 0"],
  ],
  [
    "Bool->Char",
    "Bool -> Char",
    "Converts a boolean to a character representation",
    ["(Bool->Char true); => '1'", "(Bool->Char false); => '0'"],
  ],
  [
    "Char->Int",
    "Char -> Int",
    "Returns the 32-bit integer ASCII/Unicode value of a character",
    ["(Char->Int 'A'); => 65", "(Char->Int nl); => 10"],
  ],
  [
    "Char->Bool",
    "Char -> Bool",
    "Converts a character to a boolean (typically non-null is true)",
    ["(Char->Bool 'a'); => true", "(Char->Bool ep); => false"],
  ],
  [
    "Int->Bool",
    "Int -> Bool",
    "Converts an integer to a boolean (0 is false, others are true)",
    ["(Int->Bool 1); => true", "(Int->Bool 0); => false"],
  ],
  [
    "Int->Char",
    "Int -> Char",
    "Converts a 32-bit integer code to its corresponding character",
    ["(Int->Char 97); => 'a'"],
  ],
  [
    "int",
    "Int -> [Int]",
    "Creates a new mutable boxed i32 Int containing the initial Int",
  ],
  [
    "float",
    "Float -> [Float]",
    "Creates a new mutable boxed f32 Float containing the initial Float",
  ],
  ["I/comb", "T15 -> T15"],
  ["K/comb", "T17 -> T18 -> T17"],
  ["KI/comb", "T18 -> T17 -> T17"],
  ["W/comb", "(T20 -> T20 -> T19) -> T20 -> T19"],
  ["B/comb", "(T21 -> T23) -> (T22 -> T21) -> T22 -> T23"],
  ["C/comb", "(T22 -> T23 -> T21) -> T23 -> T22 -> T21"],
  ["S/comb", "(T22 -> T23 -> T24) -> (T22 -> T23) -> T22 -> T24"],
  ["D/comb", "(T25 -> T27 -> T24) -> (T26 -> T27) -> T25 -> T26 -> T24"],
  ["B1/comb", "(T25 -> T24) -> (T26 -> T27 -> T25) -> T26 -> T27 -> T24"],
  ["PSI/comb", "(T24 -> T24 -> T26) -> (T25 -> T24) -> T25 -> T25 -> T26"],
  [
    "PHI/comb",
    "(T25 -> T26) -> (T26 -> T28 -> T27) -> (T25 -> T28) -> T25 -> T27",
  ],
  ["Rec/return", "Int"],
  ["Rec/push", "Int"],
  ["Rec/none", "Int"],
  ["Rec", "T54 -> (T54 -> {Int * [T54]}) -> [T54]"],
  ["all-equal/int?", "[Int] -> Bool"],
  ["all-equal/float?", "[Float] -> Bool"],
  ["all-equal/char?", "[Char] -> Bool"],
  ["all-equal/bool?", "[Bool] -> Bool"],
  ["iexpt", "Int -> Int -> Int"],
  ["get*", "[T28] -> Int -> (T28 -> T27) -> (() -> T26) -> ()"],
  [
    "loop/repeat",
    "Int -> (() -> T18) -> ()",
    "Executes a provided body function a specific number of times",
    ['(loop/repeat 3 (lambda (cmd/echo "Hi"))); => ()'],
  ],
  [
    "loop/some-range?",
    "Int -> Int -> (Int -> Bool) -> Bool",
    "Loops until condition is false in the range [start, end) satisfies the predicate",
    ["(loop/some-range? 0 10 (lambda n (> n 5))); => true"],
  ],
  [
    "loop/some-n?",
    "Int -> (Int -> Bool) -> Bool",
    "Loops until condition is false from 0 to N-1",
    ["(loop/some-n? 5 (lambda n (== n 3))); => true"],
  ],
  [
    "push!",
    "[T17] -> T17 -> ()",
    "Appends an element to the end of a mutable vector in-place",
    ["(let xs [1 2]) (push! xs 3); => ()", "xs; => [1 2 3]"],
  ],
  [
    "pull!",
    "[T20] -> T20",
    "Removes and returns the last element from a mutable vector",
    ["(let xs [1 2 3]) (pull! xs); => 3", "xs; => [1 2]"],
  ],
  [
    "swap!",
    "Int -> Int -> [T28] -> ()",
    "Swaps the elements at two specified indices within a vector",
    ["(let xs [10 20 30]) (swap! 0 2 xs); => ()", "xs; => [30 20 10]"],
  ],
  [
    "scan!",
    "(T35 -> T35 -> T35) -> [T35] -> ()",
    "Performs an in-place prefix sum (scan) on a vector using the provided operator",
    ["(let xs [1 2 3]) (scan! + xs); => ()", "xs; => [1 3 6]"],
  ],
  [
    "empty!",
    "[T27] -> ()",
    "Removes all elements from a mutable vector, leaving it with length 0",
    ["(let xs [1 2 3]) (empty! xs); => ()", "xs; => []"],
  ],
  [
    "reverse!",
    "[T31] -> ()",
    "Reverses the order of elements in a vector in-place",
    ["(let xs [1 2 3]) (reverse! xs); => ()", "xs; => [3 2 1]"],
  ],
  [
    "sort!",
    "(T77 -> T77 -> Bool) -> [T77] -> [T77]",
    "Sorts a vector in-place using a comparator function and returns the vector",
    ["(let xs [3 1 2]) (sort! < xs); => [1 2 3]"],
  ],

  [
    "emod",
    "Int -> Int -> Int",
    "Returns the Euclidean remainder of two integers; ensures the result always has the same sign as the divisor",
    ["(emod -5 3); => 1", "(emod 5 -3); => -1"],
  ],
  ["mul", "Int -> Int -> Int"],
  ["div", "Int -> Int -> Int"],
  ["add", "Int -> Int -> Int"],
  ["sub", "Int -> Int -> Int"],
  [
    "const",
    "T17 -> T18 -> T17",
    "A higher-order function that returns a constant function; always returns the first argument regardless of the second",
    [
      "(apply (const 42) 99); => 42",
      '(let always-true (const true)) (always-true "anything"); => true',
    ],
  ],
  [
    "floor",
    "Float -> Float",
    "Returns the largest integer less than or equal to a 32-bit float",
    ["(floor 3.9); => 3.0", "(floor -3.1); => -4.0"],
  ],
  [
    "ceil",
    "Float -> Float",
    "Returns the smallest integer greater than or equal to a 32-bit float",
    ["(ceil 3.1); => 4.0", "(ceil -3.9); => -3.0"],
  ],
  [
    "extreme",
    "[Int] -> {Int * Int}",
    "Returns a tuple containing both the minimum and maximum values found in a vector of integers",
    ["(let xs [5 12 3 8]) (extreme xs); => {3 12}"],
  ],
  [
    "map/tuple",
    "(T27 -> T28 -> T26) -> {T27 * T28} -> T26",
    "Applies a binary function to both elements of a tuple and returns the result",
    [
      "(map/tuple + {10 * 20}); => 30",
      '(map/tuple (lambda a b (cons a "-" b)) {"que" "lisp"}); => "que-lisp"',
    ],
  ],
  [
    "map/fst",
    "(T24 -> T23) -> {T24 * T25} -> T23",
    "Applies a transformation function specifically to the first element of a tuple",
    [
      "(map/fst (lambda n (* n 2)) {10 5}); => 20",
      '(map/fst cmd/ls {"/tmp" "ignore"}); => "ls /tmp"',
    ],
  ],
  [
    "map/snd",
    "(T24 -> T23) -> {T25 * T24} -> T23",
    "Applies a transformation function specifically to the second element of a tuple",
    [
      "(map/snd (lambda n (+ n 1)) {5 10}); => 11",
      '(map/snd str/quote {"key" "value"}); => "\'value\'"',
    ],
  ],
  [
    "flat-map",
    "(T80 -> T79) -> [[T80]] -> [T79]",
    "Maps a function over a vector of vectors and flattens the result into a single vector",
    ["(let xs [[1 2] [3 4]]) (flat-map identity xs); => [1 2 3 4]"],
  ],
  [
    "map",
    "(T35 -> T36) -> [T35] -> [T36]",
    "Applies a transformation function to each element and returns a new vector",
    ["(map (lambda n (* n 2)) [1 2 3]); => [2 4 6]"],
  ],
  [
    "for",
    "(T24 -> T25) -> [T24] -> ()",
    "Iterates over a vector and applies a function to each element for side effects",
    ["(for (lambda n (cmd/echo (Integer->String n))) [10 20]); => ()"],
  ],
  [
    "for/i",
    "(T26 -> Int -> T25) -> [T26] -> ()",
    "Iterates over a vector with an index; function receives (element, index)",
    ["(for/i (lambda val i (cmd/echo (Integer->String i))) [10 20]); => ()"],
  ],
  [
    "each",
    "[T25] -> (T25 -> T24) -> [T25]",
    "Applies a function to each element and returns the original vector (Fluent/Chainable)",
    ['(let xs [1 2]) (each xs (lambda n (cmd/echo "hi"))); => [1 2]'],
  ],
  [
    "each/i",
    "[T26] -> (T26 -> Int -> T25) -> [T26]",
    "Applies a function with index to each element and returns the original vector",
    [
      "(let xs [10 20]) (each/i xs (lambda val i (cmd/echo (Integer->String i)))); => [10 20]",
    ],
  ],
  [
    "filter",
    "(T35 -> Bool) -> [T35] -> [T35]",
    "Returns a new vector containing only the elements that satisfy the predicate",
    ["(filter (lambda n (> n 5)) [2 8 4 10]); => [8 10]"],
  ],
  [
    "reduce",
    "(T34 -> T35 -> T34) -> T34 -> [T35] -> T34",
    "Combines all elements into a single value using an accumulator and a binary function",
    ["(reduce + 0 [1 2 3 4]); => 10"],
  ],
  [
    "reduce/until",
    "(T38 -> T37 -> T38) -> (T38 -> Bool) -> T38 -> [T37] -> T38",
    "Reduces a vector but stops early if the accumulator satisfies the 'until' predicate",
    ["(reduce/until + (lambda acc (> acc 5)) 0 [1 2 3 4 5]); => 6"],
  ],
  ["interleave", "[T35] -> [T35] -> [T35]"],
  ["intersperse", "T40 -> [T40] -> [T40]"],

  [
    "transpose",
    "[[T45]] -> [[T45]]",
    "Flips a vector of vectors by switching rows and columns",
    ["(transpose [[1 2] [3 4]]); => [[1 3] [2 4]]"],
  ],
  [
    "every?",
    "(T31 -> Bool) -> [T31] -> Bool",
    "Returns true if every element in the vector satisfies the predicate",
    ["(every? (lambda n (> n 0)) [1 2 3]); => true"],
  ],
  [
    "some?",
    "(T31 -> Bool) -> [T31] -> Bool",
    "Returns true if at least one element in the vector satisfies the predicate",
    ["(some? (lambda n (< n 0)) [1 -2 3]); => true"],
  ],
  [
    "empty?",
    "[T16] -> Bool",
    "Returns true if the vector contains no elements",
    ["(empty? []); => true", "(empty? [1]); => false"],
  ],
  [
    "not-empty?",
    "[T16] -> Bool",
    "Returns true if the vector contains one or more elements",
    ["(not-empty? [1 2]); => true"],
  ],
  [
    "exclude",
    "(T37 -> Bool) -> [T37] -> [T37]",
    "Returns a new vector containing only elements that do NOT satisfy the predicate",
    ["(exclude (lambda n (== n 2)) [1 2 3 2]); => [1 3]"],
  ],
  [
    "select",
    "(T35 -> Bool) -> [T35] -> [T35]",
    "Alias for filter; returns elements that satisfy the predicate",
    ["(select (lambda n (> n 10)) [5 15 8 20]); => [15 20]"],
  ],
  [
    "find",
    "(T34 -> Bool) -> [T34] -> Int",
    "Returns the zero-based index of the first element that satisfies the predicate",
    ["(find (lambda n (== n 30)) [10 20 30 40]); => 2"],
  ],
  [
    "partition",
    "Int -> [T40] -> [[T40]]",
    "Splits a vector into sub-vectors of a specific size",
    ["(partition 2 [1 2 3 4]); => [[1 2] [3 4]]"],
  ],
  [
    "reverse",
    "[T29] -> [T29]",
    "Returns a new vector with the elements in reverse order",
    ["(reverse [1 2 3]); => [3 2 1]"],
  ],
  [
    "slice",
    "Int -> Int -> [T33] -> [T33]",
    "Returns a sub-vector from the start index up to (but not including) the end index",
    ["(slice 1 3 [10 20 30 40]); => [20 30]"],
  ],
  [
    "cons",
    "[T37] -> [T37] -> [T37]",
    "Joins two vectors together into a single new vector",
    ["(cons [1 2] [3 4]); => [1 2 3 4]"],
  ],
  [
    "range",
    "Int -> Int -> [Int]",
    "Generates a vector of integers from start to end (exclusive)",
    ["(range 0 3); => [0 1 2]"],
  ],
  [
    "range/int",
    "Int -> Int -> [Int]",
    "Explicitly generates a vector of 32-bit integers in a range",
    ["(range/int 5 8); => [5 6 7]"],
  ],
  [
    "range/float",
    "Int -> Int -> [Float]",
    "Generates a vector of 32-bit floats based on the integer range",
    ["(range/float 0 3); => [0.0 1.0 2.0]"],
  ],

  [
    "square",
    "Int -> Int",
    "Multiplies an integer by itself",
    ["(square 4); => 16"],
  ],
  [
    "expt",
    "Int -> Int -> Int",
    "Returns the base (last arg) raised to the power of the exponent (first arg)",
    ["(expt 10 2); => 1024", "(expt 3 2); => 8"],
  ],
  [
    "sqrt",
    "Int -> Int",
    "Returns the integer square root of a 32-bit integer",
    ["(sqrt 16); => 4"],
  ],
  [
    "expt/int",
    "Int -> Int -> Int",
    "Calculates integer power with data-last (base-last) ordering",
    ["(expt/int 8 2); => 256"],
  ],
  [
    "sqrt/int",
    "Int -> Int",
    "Explicit 32-bit integer square root",
    ["(sqrt/int 100); => 10"],
  ],
  [
    "expt/float",
    "Float -> Float -> Float",
    "Returns the base (last arg) raised to the power of the exponent (first arg) as 32-bit floats",
    ["(expt/float 3.0 2.0); => 8.0"],
  ],
  [
    "sqrt/float",
    "Float -> Float",
    "Returns the square root of a 32-bit float",
    ["(sqrt/float 16.0); => 4.0"],
  ],

  [
    "odd?",
    "Int -> Bool",
    "Returns true if the 32-bit integer is not divisible by 2",
    ["(odd? 3); => true", "(odd? 4); => false"],
  ],
  [
    "even?",
    "Int -> Bool",
    "Returns true if the 32-bit integer is divisible by 2",
    ["(even? 4); => true", "(even? 5); => false"],
  ],
  [
    "odd/int?",
    "Int -> Bool",
    "Explicitly checks if an Int is odd; useful for clear type intent",
    ["(odd/int? 7); => true"],
  ],
  [
    "even/int?",
    "Int -> Bool",
    "Explicitly checks if an Int is even; useful for clear type intent",
    ["(even/int? 8); => true"],
  ],
  [
    "odd/float?",
    "Float -> Bool",
    "Checks if a 32-bit float represents an odd integer value",
    ["(odd/float? 3.0); => true", "(odd/float? 3.1); => false"],
  ],
  [
    "even/float?",
    "Float -> Bool",
    "Checks if a 32-bit float represents an even integer value",
    ["(even/float? 2.0); => true", "(even/float? 2.1); => false"],
  ],
  [
    "one?",
    "Int -> Bool",
    "Returns true if the integer value is exactly 1",
    ["(one? 1); => true", "(one? 0); => false"],
  ],
  [
    "zero?",
    "Int -> Bool",
    "Returns true if the integer value is exactly 0",
    ["(zero? 0); => true", "(zero? 1); => false"],
  ],
  [
    "one/int?",
    "Int -> Bool",
    "Explicitly checks if an Int is 1",
    ["(one/int? 1); => true"],
  ],
  [
    "zero/int?",
    "Int -> Bool",
    "Explicitly checks if an Int is 0",
    ["(zero/int? 0); => true"],
  ],
  [
    "one/float?",
    "Float -> Bool",
    "Returns true if the 32-bit float is exactly 1.0",
    ["(one/float? 1.0); => true", "(one/float? 1.0001); => false"],
  ],
  [
    "zero/float?",
    "Float -> Bool",
    "Returns true if the 32-bit float is exactly 0.0",
    ["(zero/float? 0.0); => true"],
  ],

  [
    "map/until",
    "(T55 -> T54) -> (T55 -> Bool) -> [T55] -> [T54]",
    "Maps a function over a vector but stops and returns the result as soon as an element satisfies the until-predicate",
    ["(map/until (lambda n (* n 2)) (lambda n (> n 2)) [1 2 3 4]); => [2 4]"],
  ],
  [
    "map/until/i",
    "(T56 -> Int -> T57) -> (T56 -> Int -> Bool) -> [T56] -> [T57]",
    "Indexed map that terminates early when the until-predicate (element, index) is met",
    [
      "(map/until/i (lambda n i (+ n i)) (lambda n i (== i 2)) [10 20 30 40]); => [10 21]",
    ],
  ],
  [
    "reduce/until",
    "(T37 -> T38 -> T37) -> (T37 -> Bool) -> T37 -> [T38] -> T37",
    "Reduces a vector into a single value, stopping early if the accumulator satisfies the until-predicate",
    ["(reduce/until + (lambda acc (> acc 5)) 0 [1 2 3 4]); => 6"],
  ],
  [
    "reduce/until/i",
    "(T58 -> T57 -> Int -> T58) -> (T58 -> T57 -> Int -> Bool) -> T58 -> [T57] -> T58",
    "Indexed reduction that terminates early based on the accumulator, current element, and index",
    [
      "(reduce/until/i (lambda acc n i (+ acc n)) (lambda acc n i (> i 1)) 0 [1 2 3]); => 3",
    ],
  ],
  [
    "for/until",
    "(T48 -> T49) -> (T48 -> Bool) -> [T48] -> ()",
    "Iterates for side effects but breaks the loop when the until-predicate is satisfied",
    ["(for/until (lambda n (cmd/echo n)) (lambda n (== n 2)) [1 2 3]); => ()"],
  ],
  [
    "for/until/i",
    "(T50 -> Int -> T51) -> (T50 -> Int -> Bool) -> [T50] -> ()",
    "Indexed iteration for side effects with an early exit condition",
    [
      "(for/until/i (lambda n i (cmd/echo i)) (lambda n i (> i 0)) [10 20 30]); => ()",
    ],
  ],
  [
    "each/until",
    "(T49 -> T48) -> (T49 -> Bool) -> [T49] -> [T49]",
    "Applies side effects and returns the original vector, stopping early if the predicate is met",
    [
      "(each/until (lambda n (cmd/echo n)) (lambda n (> n 1)) [1 2 3]); => [1 2 3]",
    ],
  ],
  [
    "each/until/i",
    "(T50 -> Int -> T51) -> (T50 -> Int -> Bool) -> [T50] -> [T50]",
    "Fluent indexed iteration that returns the original vector but terminates processing early",
    [
      "(each/until/i (lambda n i (cmd/echo i)) (lambda n i (== i 1)) [10 20 30]); => [10 20 30]",
    ],
  ],
  [
    "map/i",
    "(T37 -> Int -> T36) -> [T37] -> [T36]",
    "Standard indexed map; transforms elements based on their value and position",
    ["(map/i (lambda n i (* n i)) [1 2 3]); => [0 2 6]"],
  ],
  [
    "reduce/i",
    "(T36 -> T35 -> Int -> T36) -> T36 -> [T35] -> T36",
    "Standard indexed reduction; combines elements using value, index, and accumulator",
    ["(reduce/i (lambda acc n i (+ acc i)) 0 [10 10 10]); => 3"],
  ],
  [
    "filter/i",
    "(T36 -> Int -> Bool) -> [T36] -> [T36]",
    "Filters a vector keeping only elements where the predicate (value, index) is true",
    ["(filter/i (lambda n i (even? i)) [10 20 30 40]); => [10 30]"],
  ],
  [
    "some/i?",
    "(T33 -> Int -> Bool) -> [T33] -> Bool",
    "Returns true if at least one element/index pair satisfies the predicate",
    ["(some/i? (lambda n i (== n i)) [0 2 2]); => true"],
  ],
  [
    "every/i?",
    "(T33 -> Int -> Bool) -> [T33] -> Bool",
    "Returns true only if every element/index pair satisfies the predicate",
    ["(every/i? (lambda n i (>= n i)) [10 20 30]); => true"],
  ],
  [
    "ones",
    "Int -> [Int]",
    "Generates a vector of N integers, all initialized to 1",
    ["(ones 3); => [1 1 1]"],
  ],
  [
    "zeroes",
    "Int -> [Int]",
    "Generates a vector of N integers, all initialized to 0",
    ["(zeroes 3); => [0 0 0]"],
  ],
  [
    "ones/int",
    "Int -> [Int]",
    "Explicit 32-bit integer vector filled with 1s",
    ["(ones/int 2); => [1 1]"],
  ],
  [
    "zeroes/int",
    "Int -> [Int]",
    "Explicit 32-bit integer vector filled with 0s",
    ["(zeroes/int 2); => [0 0]"],
  ],
  [
    "ones/float",
    "Int -> [Float]",
    "Generates a vector of N 32-bit floats, all initialized to 1.0",
    ["(ones/float 2); => [1.0 1.0]"],
  ],
  [
    "zeroes/float",
    "Int -> [Float]",
    "Generates a vector of N 32-bit floats, all initialized to 0.0",
    ["(zeroes/float 2); => [0.0 0.0]"],
  ],

  [
    "positive?",
    "Int -> Bool",
    "Checks if a 32-bit integer is greater than 0",
    ["(positive? 5); => true", "(positive? -1); => false"],
  ],
  [
    "negative?",
    "Int -> Bool",
    "Checks if a 32-bit integer is less than 0",
    ["(negative? -5); => true"],
  ],
  [
    "invert",
    "Int -> Int",
    "Returns the additive inverse of an integer",
    ["(invert 10); => -10"],
  ],
  [
    "negative-one?",
    "Int -> Bool",
    "Checks if the integer is exactly -1",
    ["(negative-one? -1); => true"],
  ],
  [
    "divisible?",
    "Int -> Int -> Bool",
    "Returns true if the second argument is divisible by the first",
    ["(divisible? 2 10); => true"],
  ],
  [
    "positive/int?",
    "Int -> Bool",
    "Explicit 32-bit integer positivity check",
    ["(positive/int? 1); => true"],
  ],
  [
    "negative/int?",
    "Int -> Bool",
    "Explicit 32-bit integer negativity check",
    ["(negative/int? -1); => true"],
  ],
  [
    "invert/int",
    "Int -> Int",
    "Explicit 32-bit integer additive inversion",
    ["(invert/int -5); => 5"],
  ],
  [
    "negative-one/int?",
    "Int -> Bool",
    "Explicit check for the integer value -1",
    ["(negative-one/int? -1); => true"],
  ],
  [
    "divisible/int?",
    "Int -> Int -> Bool",
    "Explicit 32-bit integer divisibility check (divisor first)",
    ["(divisible/int? 3 9); => true"],
  ],
  [
    "positive/float?",
    "Float -> Bool",
    "Checks if a 32-bit float is greater than 0.0",
    ["(positive/float? 0.1); => true"],
  ],
  [
    "negative/float?",
    "Float -> Bool",
    "Checks if a 32-bit float is less than 0.0",
    ["(negative/float? -0.1); => true"],
  ],
  [
    "invert/float",
    "Float -> Float",
    "Returns the additive inverse of a 32-bit float",
    ["(invert/float 1.5); => -1.5"],
  ],
  [
    "negative-one/float?",
    "Float -> Bool",
    "Checks if a 32-bit float is exactly -1.0",
    ["(negative-one/float? -1.0); => true"],
  ],
  [
    "divisible/float?",
    "Float -> Float -> Bool",
    "Checks if the second float is divisible by the first with no remainder",
    ["(divisible/float? 0.5 2.0); => true"],
  ],
  [
    "upper",
    "Char -> Char",
    "Converts a character to its uppercase equivalent",
    ["(upper #\\a); => #\\A"],
  ],
  [
    "lower",
    "Char -> Char",
    "Converts a character to its lowercase equivalent",
    ["(lower #\\A); => #\\a"],
  ],
  [
    "match?",
    "[Char] -> [Char] -> Bool",
    "Checks if a string (vector of chars) matches a pattern or another string",
    ['(match? "que" "que"); => true'],
  ],
  [
    "digit?",
    "Char -> Bool",
    "Checks if a character is a numeric digit (0-9)",
    ["(digit? #\\5); => true"],
  ],
  [
    "fill",
    "Int -> (Int -> T40) -> [T40]",
    "Generates a vector of size N by applying a function to each index",
    ["(fill 3 (lambda i (* i 10))); => [0 10 20]"],
  ],
  [
    "max",
    "Int -> Int -> Int",
    "Returns the larger of two 32-bit integers",
    ["(max 10 20); => 20"],
  ],
  [
    "min",
    "Int -> Int -> Int",
    "Returns the smaller of two 32-bit integers",
    ["(min 10 20); => 10"],
  ],
  [
    "max/int",
    "Int -> Int -> Int",
    "Explicit 32-bit integer maximum",
    ["(max/int 5 2); => 5"],
  ],
  [
    "min/int",
    "Int -> Int -> Int",
    "Explicit 32-bit integer minimum",
    ["(min/int 5 2); => 2"],
  ],
  [
    "max/float",
    "Float -> Float -> Float",
    "Returns the larger of two 32-bit floats",
    ["(max/float 1.1 1.2); => 1.2"],
  ],
  [
    "min/float",
    "Float -> Float -> Float",
    "Returns the smaller of two 32-bit floats",
    ["(min/float 1.1 1.2); => 1.1"],
  ],
  [
    "maximum",
    "[Int] -> Int",
    "Returns the largest element in a vector of integers",
    ["(maximum [1 5 3]); => 5"],
  ],
  [
    "minimum",
    "[Int] -> Int",
    "Returns the smallest element in a vector of integers",
    ["(minimum [1 5 3]); => 1"],
  ],
  [
    "maximum/int",
    "[Int] -> Int",
    "Explicit 32-bit integer vector maximum",
    ["(maximum/int [10 20]); => 20"],
  ],
  [
    "minimum/int",
    "[Int] -> Int",
    "Explicit 32-bit integer vector minimum",
    ["(minimum/int [10 20]); => 10"],
  ],
  [
    "maximum/float",
    "[Float] -> Float",
    "Returns the largest element in a vector of 32-bit floats",
    ["(maximum/float [1.1 2.2]); => 2.2"],
  ],
  [
    "minimum/float",
    "[Float] -> Float",
    "Returns the smallest element in a vector of 32-bit floats",
    ["(minimum/float [1.1 2.2]); => 1.1"],
  ],
  [
    "gt?",
    "Bool -> Bool -> Bool",
    "Checks if the first boolean is greater than the last (true > false)",
    ["(gt? true false); => true"],
  ],
  [
    "lt?",
    "Bool -> Bool -> Bool",
    "Checks if the first boolean is less than the last (false < true)",
    ["(lt? false true); => true"],
  ],
  [
    "and?",
    "Bool -> Bool -> Bool",
    "Performs a logical AND on two booleans",
    ["(and? true false); => false"],
  ],
  [
    "or?",
    "Bool -> Bool -> Bool",
    "Performs a logical OR on two booleans",
    ["(or? true false); => true"],
  ],
  [
    "not?",
    "Bool -> Bool",
    "Returns the logical inverse of a boolean",
    ["(not? true); => false"],
  ],
  [
    "abs",
    "Int -> Int",
    "Returns the absolute value of a 32-bit integer",
    ["(abs -50); => 50"],
  ],
  [
    "abs/int",
    "Int -> Int",
    "Explicit 32-bit integer absolute value",
    ["(abs/int -10); => 10"],
  ],
  [
    "abs/float",
    "Float -> Float",
    "Returns the absolute value of a 32-bit float",
    ["(abs/float -3.14); => 3.14"],
  ],
  [
    "first",
    "[T16] -> T16",
    "Returns the first element of a vector (index 0)",
    ["(first [10 20 30]); => 10"],
  ],
  [
    "last",
    "[T17] -> T17",
    "Returns the last element of a vector",
    ["(last [10 20 30]); => 30"],
  ],
  [
    "pair",
    "T16 -> T15 -> {T16 * T15}",
    "Constructs a tuple from two values",
    ['(pair 1 "a"); => {1 * "a"}'],
  ],
  [
    "product",
    "[Int] -> Int",
    "Calculates the result of multiplying all integers in a vector",
    ["(product [2 3 4]); => 24"],
  ],
  [
    "product/int",
    "[Int] -> Int",
    "Explicit 32-bit integer vector product",
    ["(product/int [5 5]); => 25"],
  ],
  [
    "product/float",
    "[Float] -> Float",
    "Calculates the product of a vector of 32-bit floats",
    ["(product/float [1.5 2.0]); => 3.0"],
  ],
  [
    "sum",
    "[Int] -> Int",
    "Calculates the total sum of all integers in a vector",
    ["(sum [1 2 3]); => 6"],
  ],
  [
    "sum/int",
    "[Int] -> Int",
    "Explicit 32-bit integer vector sum",
    ["(sum/int [10 10]); => 20"],
  ],
  [
    "sum/float",
    "[Float] -> Float",
    "Calculates the sum of a vector of 32-bit floats",
    ["(sum/float [1.1 2.2]); => 3.3"],
  ],
  [
    "avg",
    "Int -> Int -> Int",
    "Calculates the arithmetic mean of two integers",
    ["(avg 10 20); => 15"],
  ],
  [
    "avg/int",
    "Int -> Int -> Int",
    "Explicit 32-bit integer average of two values",
    ["(avg/int 4 6); => 5"],
  ],
  [
    "avg/float",
    "Float -> Float -> Float",
    "Calculates the average of two 32-bit floats",
    ["(avg/float 1.0 2.0); => 1.5"],
  ],
  [
    "mean",
    "[Int] -> Int",
    "Returns the arithmetic mean of a vector of integers",
    ["(mean [10 20 30]); => 20"],
  ],
  [
    "median",
    "[Int] -> Int",
    "Returns the middle value of a sorted vector of integers",
    ["(median [1 3 5]); => 3"],
  ],
  [
    "mean/int",
    "[Int] -> Int",
    "Explicit 32-bit integer vector mean",
    ["(mean/int [2 4]); => 3"],
  ],
  [
    "mean/float",
    "[Float] -> Float",
    "Returns the arithmetic mean of a vector of 32-bit floats",
    ["(mean/float [1.0 2.0]); => 1.5"],
  ],
  [
    "median/int",
    "[Int] -> Int",
    "Explicit 32-bit integer vector median",
    ["(median/int [10 20 30]); => 20"],
  ],
  [
    "median/float",
    "[Float] -> Float",
    "Returns the median of a vector of 32-bit floats",
    ["(median/float [1.1 2.2 3.3]); => 2.2"],
  ],

  [
    "zip",
    "{[T37] * [T38]} -> [{T37 * T38}]",
    "Combines a tuple of two vectors into a single vector of tuples",
    ["(zip {[1 2] * [10 20]}); => [{1 * 10} {2 * 20}]"],
  ],
  [
    "unzip",
    "[{T45 * T44}] -> {[T45] * [T44]}",
    "Splits a vector of tuples into a tuple containing two separate vectors",
    ["(unzip [{1 * 10} {2 * 20}]); => {[1 2] * [10 20]}"],
  ],
  [
    "zip-with",
    "(T33 -> T34 -> T32) -> [T33] -> [T34] -> [T32]",
    "Combines two vectors using a binary function to produce a new vector",
    ["(zip-with + [1 2] [10 20]); => [11 22]"],
  ],
  [
    "window",
    "[T74] -> Int -> [[T74]]",
    "Creates a sliding window of size N over a vector",
    ["(window [1 2 3 4] 2); => [[1 2] [2 3] [3 4]]"],
  ],
  [
    "flat",
    "[[T60]] -> [T60]",
    "Flattens a vector of vectors into a single-level vector",
    ["(flat [[1 2] [3 4]]); => [1 2 3 4]"],
  ],
  [
    "enumerate",
    "[T46] -> [{Int * T46}]",
    "Pairs each element of a vector with its zero-based index",
    ['(enumerate ["a" "b"]); => [{0 * "a"} {1 * "b"}]'],
  ],
  [
    "clamp",
    "Int -> Int -> Int",
    "Restricts a value to be no greater than the provided maximum",
    ["(clamp 100 50); => 50", "(clamp 20 50); => 20"],
  ],
  [
    "clamp-range",
    "Int -> Int -> Int -> Int",
    "Constrains a value within a specific [min, max] range",
    ["(clamp-range 0 100 50); => 50", "(clamp-range 0 100 -10); => 0"],
  ],
  [
    "clamp/int",
    "Int -> Int -> Int",
    "Explicit 32-bit integer maximum constraint",
    ["(clamp/int 10 5); => 5"],
  ],
  [
    "clamp-range/int",
    "Int -> Int -> Int -> Int",
    "Explicit 32-bit integer range constraint",
    ["(clamp-range/int 1 10 15); => 10"],
  ],
  [
    "clamp/float",
    "Float -> Float -> Float",
    "Restricts a 32-bit float to be no greater than the provided maximum",
    ["(clamp/float 1.5 1.0); => 1.0"],
  ],
  [
    "clamp-range/float",
    "Float -> Float -> Float -> Float",
    "Constrains a 32-bit float within a specific [min, max] range",
    ["(clamp-range/float 0.0 1.0 0.5); => 0.5"],
  ],

  [
    "at",
    "[T19] -> Int -> T19",
    "Returns the element at the specified index; supports negative integers to count back from the end",
    ["(at [10 20 30] 0); => 10", "(at [10 20 30] -1); => 30"],
  ],
  [
    "scan",
    "(T57 -> T57 -> T57) -> [T57] -> [T57]",
    "Returns a vector of cumulative results (prefix sums) using a binary operator",
    ["(scan + [1 2 3]); => [1 3 6]"],
  ],
  [
    "cycle",
    "Int -> [T30] -> [T30]",
    "Repeats a vector N times to create a new, larger vector",
    ["(cycle 3 [1 2]); => [1 2 1 2 1 2]"],
  ],
  [
    "replicate",
    "Int -> T28 -> [T28]",
    "Creates a vector of length N containing only the provided value",
    ['(replicate 3 "hi"); => ["hi" "hi" "hi"]'],
  ],
  [
    "cartesian-product",
    "[T73] -> [T74] -> [{T73 * T74}]",
    "Returns a vector of tuples representing all possible combinations of elements from two vectors",
    [
      '(cartesian-product [1 2] ["a" "b"]); => [{1 * "a"} {1 * "b"} {2 * "a"} {2 * "b"}]',
    ],
  ],
  [
    "lcm",
    "Int -> Int -> Int",
    "Calculates the Least Common Multiple of two 32-bit integers",
    ["(lcm 4 6); => 12"],
  ],
  [
    "gcd",
    "Int -> Int -> Int",
    "Calculates the Greatest Common Divisor of two 32-bit integers",
    ["(gcd 12 8); => 4"],
  ],
  ["delta", "Int -> Int -> Int"],
  ["delta/int", "Int -> Int -> Int"],
  ["delta/float", "Float -> Float -> Float"],
  [
    "map/adjacent",
    "(T41 -> T41 -> T40) -> [T41] -> [T40]",
    "Applies a binary function to every overlapping pair of elements in a vector",
    ["(map/adjacent - [10 8 5]); => [2 3]"],
  ],
  [
    "buckets",
    "Int -> [[T25]]",
    "Initializes a vector containing N empty sub-vectors",
    ["(buckets 3); => [[] [] []]"],
  ],
  [
    "count/char",
    "Char -> [Char] -> Int",
    "Counts how many times a specific character appears in a string or vector",
    ['(count/char #\\a "banana"); => 3'],
  ],
  [
    "count/int",
    "Int -> [Int] -> Int",
    "Counts occurrences of a specific 32-bit integer in a vector",
    ["(count/int 5 [1 5 2 5]); => 2"],
  ],
  [
    "count/float",
    "Float -> [Float] -> Int",
    "Counts occurrences of a specific 32-bit float in a vector",
    ["(count/float 1.1 [1.1 2.0 1.1]); => 2"],
  ],
  [
    "count/bool",
    "Bool -> [Bool] -> Int",
    "Counts how many times a boolean value appears in a vector",
    ["(count/bool true [true false true]); => 2"],
  ],
  [
    "count",
    "(T39 -> Bool) -> [T39] -> Int",
    "Returns the total number of elements that satisfy the predicate",
    ["(count even? [1 2 3 4 5]); => 2"],
  ],
  [
    "points",
    "(T52 -> Bool) -> [[T52]] -> [[Int]]",
    "Returns a vector of [row col] coordinate vectors for elements matching the predicate",
    ["(points (lambda x (== x 1)) [[0 1] [1 0]]); => [[0 1] [1 0]]"],
  ],
  [
    "unique/int",
    "[Int] -> [Int]",
    "Removes duplicates from a vector of integers, preserving original order",
    ["(unique/int [1 2 1 3 2]); => [1 2 3]"],
  ],
  [
    "unique/char",
    "[Char] -> [Char]",
    "Removes duplicate characters from a string or character vector",
    ['(unique/char "mississippi"); => "misp"'],
  ],
  [
    "permutation",
    "[T76] -> [[T76]]",
    "Generates all possible ordered arrangements of the vector elements",
    ["(permutation [1 2]); => [[1 2] [2 1]]"],
  ],
  [
    "combination/pairs",
    "[T43] -> [{T43 * T43}]",
    "Generates all unique non-repeating pairs (2-combinations) as tuples",
    ["(combination/pairs [1 2 3]); => [{1 * 2} {1 * 3} {2 * 3}]"],
  ],
  [
    "combination",
    "[T63] -> [[T63]]",
    "Generates all possible unique combinations of the vector elements",
    ["(combination [1 2]); => [[] [1] [2] [1 2]]"],
  ],
  [
    "combination/n",
    "Int -> [T63] -> [[T63]]",
    "Generates all unique combinations of size N from the vector",
    ["(combination/n 2 [1 2 3]); => [[1 2] [1 3] [2 3]]"],
  ],
  [
    "subset",
    "[T96] -> [[T96]]",
    "Alias for combination; returns the power set of the vector",
    ["(subset [1 2]); => [[] [1] [2] [1 2]]"],
  ],
  [
    "in-bounds?",
    "[T17] -> Int -> Bool",
    "Checks if the provided index is valid for the given vector length",
    ["(in-bounds? [10 20] 1); => true", "(in-bounds? [10 20] 5); => false"],
  ],
  [
    "take/first",
    "Int -> [T31] -> [T31]",
    "Returns a new vector containing only the first N elements",
    ["(take/first 2 [10 20 30]); => [10 20]"],
  ],
  [
    "drop/first",
    "Int -> [T32] -> [T32]",
    "Returns a new vector with the first N elements removed",
    ["(drop/first 1 [10 20 30]); => [20 30]"],
  ],
  [
    "take/last",
    "Int -> [T33] -> [T33]",
    "Returns a new vector containing only the last N elements",
    ["(take/last 2 [10 20 30]); => [20 30]"],
  ],
  [
    "drop/last",
    "Int -> [T32] -> [T32]",
    "Returns a new vector with the last N elements removed",
    ["(drop/last 1 [10 20 30]); => [10 20]"],
  ],

  ["true/option", "T15 -> {Bool * T15}"],
  ["false/option", "T15 -> {Bool * T15}"],
  ["resolve/option", "([T59] -> T58) -> T58 -> [{Bool * T59}] -> {Bool * T58}"],
  ["call", "(T21 -> T20) -> T21 -> T20"],
  [
    "copy",
    "[T37] -> [T37]",
    "Creates a shallow clone of a vector to prevent mutation of the original",
    [
      "(let xs [1 2]) (let ys (copy xs)) (push! ys 3); => ys: [1 2 3], xs: [1 2]",
    ],
  ],
  [
    "sort",
    "(T97 -> T97 -> Bool) -> [T97] -> [T97]",
    "Returns a new sorted vector using a comparator function without modifying the original",
    ["(sort < [3 1 2]); => [1 2 3]"],
  ],
  [
    "neighborhood",
    "[[Int]] -> Int -> Int -> (T58 -> [Int] -> Int -> Int -> ()) -> [[T58]] -> ()",
    "Iterates over neighbors in a 2D grid based on an offset kernel, running a callback for each",
    [
      "(neighborhood neighborhood/moore 1 1 (lambda val pos r c (cmd/echo val)) grid); => ()",
    ],
  ],
  [
    "neighborhood/moore",
    "[[Int]]",
    "Constant defining the 8 surrounding cells in a 2D grid (Moore neighborhood)",
    [
      "neighborhood/moore; => [[-1 -1] [-1 0] [-1 1] [0 -1] [0 1] [1 -1] [1 0] [1 1]]",
    ],
  ],
  [
    "neighborhood/diagonal",
    "[[Int]]",
    "Constant defining only the 4 diagonal neighbor offsets",
    ["neighborhood/diagonal; => [[-1 -1] [-1 1] [1 -1] [1 1]]"],
  ],
  [
    "neighborhood/kernel",
    "[[Int]]",
    "An identity kernel offset (typically [[0 0]])",
    ["neighborhood/kernel; => [[0 0]]"],
  ],
  [
    "neighborhood/von-neumann",
    "[[Int]]",
    "Constant defining the 4 orthogonal neighbor cells (Up, Down, Left, Right)",
    ["neighborhood/von-neumann; => [[-1 0] [1 0] [0 -1] [0 1]]"],
  ],
  [
    "group",
    "(T191 -> [Char]) -> [T191] -> [[[{[Char] * [T191]}]]]",
    "Groups vector elements into a hashtable on a key-extraction function",
    [
      '(group (lambda s (slice 0 1 s)) ["apple" "ant" "banana"]); => [[[{"a" ["apple" "ant"]}]] [] [] [] [] [[{"b" ["banana"]}]]]',
    ],
  ],
  [
    "append!",
    "T20 -> [T20] -> [T20]",
    "Adds an element to the end of a vector and returns the modified vector",
    ["(let xs [1 2]) (append! 3 xs); => [1 2 3]"],
  ],
  [
    "tail",
    "[T32] -> [T32]",
    "Returns a new vector containing all elements except the first",
    ["(tail [1 2 3]); => [2 3]"],
  ],
  [
    "head",
    "[T32] -> [T32]",
    "Returns a new vector containing only the first element (as a vector)",
    ["(head [1 2 3]); => [1]"],
  ],

  ["fp/mul", "Int -> Int -> Int"],
  ["fp/div", "Int -> Int -> Int"],
  ["fp/add", "Int -> Int -> Int"],
  ["fp/sub", "Int -> Int -> Int"],
  ["fp/emod", "Int -> Int -> Int"],
  ["fp/mod", "Int -> Int -> Int"],
  ["cond/dispatch", "(T19 -> Bool) -> T18 -> T18 -> T19 -> T18"],
  [
    "split",
    "[Char] -> [Char] -> [[Char]]",
    "Splits a string into a vector of strings based on a provided delimiter",
    [
      '(split "," "a,b,c"); => ["a" "b" "c"]',
      '(split " " "que lisp"); => ["que" "lisp"]',
    ],
  ],
  [
    "Tuple/swap",
    "{T21 * T20} -> {T20 * T21}",
    "Swaps the first and second elements of a tuple",
    ['(Tuple/swap {1 "a"}); => {"a" 1}'],
  ],
  [
    "Tuple/map/fst",
    "{T21 * T23} -> (T21 -> T22) -> T22",
    "Applies a function to the first element of a tuple and returns the transformed value",
    ["(Tuple/map/fst {10 20} (lambda n (* n 2))); => 20"],
  ],
  [
    "Tuple/map/snd",
    "{T22 * T23} -> (T23 -> T21) -> T21",
    "Applies a function to the second element of a tuple and returns the transformed value",
    ['(Tuple/map/snd {"a" 5} (lambda n (+ n 1))); => 6'],
  ],
  [
    "Tuple/map",
    "{T24 * T26} -> (T24 -> T26 -> T25) -> T25",
    "Deconstructs a tuple and applies its elements to a binary function",
    ["(Tuple/map {10 5} +); => 15"],
  ],
  [
    "Tuple/int/add",
    "{Int * Int} -> Int",
    "Adds the two 32-bit integers contained within a tuple",
    ["(Tuple/int/add {10 20}); => 30"],
  ],
  [
    "Tuple/int/sub",
    "{Int * Int} -> Int",
    "Subtracts the second integer from the first within a tuple",
    ["(Tuple/int/sub {50 10}); => 40"],
  ],
  [
    "Tuple/int/mul",
    "{Int * Int} -> Int",
    "Multiplies the two 32-bit integers contained within a tuple",
    ["(Tuple/int/mul {6 7}); => 42"],
  ],
  [
    "Tuple/int/div",
    "{Int * Int} -> Int",
    "Performs integer division of the first element by the second within a tuple",
    ["(Tuple/int/div {10 2}); => 5"],
  ],
  [
    "Tuple/bool/eq?",
    "{Bool * Bool} -> Bool",
    "Returns true if both boolean values in the tuple are identical",
    ["(Tuple/bool/eq? {true true}); => true"],
  ],
  [
    "Tuple/int/eq?",
    "{Int * Int} -> Bool",
    "Returns true if both 32-bit integers in the tuple are equal",
    ["(Tuple/int/eq? {5 5}); => true"],
  ],
  [
    "Tuple/char/eq?",
    "{Char * Char} -> Bool",
    "Returns true if both characters in the tuple are identical",
    ["(Tuple/char/eq? {'a' 'a'}); => true"],
  ],
  [
    "Tuple/float/eq?",
    "{Float * Float} -> Bool",
    "Returns true if both 32-bit floats in the tuple are exactly equal",
    ["(Tuple/float/eq? {1.1 1.1}); => true"],
  ],
  [
    "Tuple/bool/not-eq?",
    "{Bool * Bool} -> Bool",
    "Returns true if the boolean values in the tuple are different",
    ["(Tuple/bool/not-eq? {true false}); => true"],
  ],
  [
    "Tuple/int/not-eq?",
    "{Int * Int} -> Bool",
    "Returns true if the 32-bit integers in the tuple are not equal",
    ["(Tuple/int/not-eq? {10 20}); => true"],
  ],
  [
    "Tuple/char/not-eq?",
    "{Char * Char} -> Bool",
    "Returns true if the characters in the tuple are different",
    ["(Tuple/char/not-eq? {'a' 'b'}); => true"],
  ],
  [
    "Tuple/float/not-eq?",
    "{Float * Float} -> Bool",
    "Returns true if the 32-bit floats in the tuple are not equal",
    ["(Tuple/float/not-eq? {1.1 1.2}); => true"],
  ],
  [
    "Vector->Tuple",
    "(T24 -> T25) -> (T24 -> T26) -> T24 -> {T25 T26}",
    "Applies two different functions to a single value to create a result tuple",
    ["(Vector->Tuple identity (lambda n (* n 2)) 10); => {10 20}"],
  ],
  [
    "Int->Alphabet",
    "Int -> Char -> Char",
    "Shifts a base character by an integer offset to find another character in the alphabet",
    ["(Int->Alphabet 1 'a'); => 'b'"],
  ],
  [
    "Integer->Chars",
    "Int -> [Char]",
    "Converts a 32-bit integer into a vector of characters",
    ["(Integer->Chars 123); => ['1' '2' '3']"],
  ],
  [
    "Integer->String",
    "Int -> [Char]",
    "Converts a 32-bit integer into a string (vector of chars)",
    ["(Integer->String -42); => ['-' '4' '2']"],
  ],
  [
    "Float->Chars",
    "Float -> [Char]",
    "Converts a 32-bit float into a vector of characters",
    ["(Float->Chars 3.14); => ['3' '.' '1' '4']"],
  ],
  [
    "Float->String",
    "Float -> [Char]",
    "Converts a 32-bit float into a string (vector of chars)",
    ["(Float->String 1.5); => ['1' '.' '5']"],
  ],
  [
    "String->Vector",
    "Char -> [Char] -> [[Char]]",
    "Splits a string into a vector of strings based on a delimiter character",
    ['(String->Vector \',\' "a,b,c"); => ["a" "b" "c"]'],
  ],
  [
    "Vector->String",
    "Char -> [[Char]] -> [Char]",
    "Joins a vector of strings into a single string using a separator character",
    ['(Vector->String \'-\' ["que" "lisp"]); => "que-lisp"'],
  ],
  [
    "Chars->Digits",
    "[Char] -> [Int]",
    "Converts a vector of numeric characters into a vector of their integer values",
    ["(Chars->Digits ['1' '2']); => [1 2]"],
  ],
  [
    "Digits->Integer",
    "[Int] -> Int",
    "Combines a vector of digits into a single 32-bit integer",
    ["(Digits->Integer [1 2 3]); => 123"],
  ],
  [
    "Chars->Integer",
    "[Char] -> Int",
    "Parses a vector of characters directly into a 32-bit integer",
    ['(Chars->Integer "1024"); => 1024'],
  ],
  [
    "String->Integer",
    "[Char] -> Int",
    "Parses a string into a 32-bit integer",
    ['(String->Integer "-50"); => -50'],
  ],
  [
    "String->Float",
    "[Char] -> Float",
    "Parses a string into a 32-bit floating-point number",
    ['(String->Float "0.25"); => 0.25'],
  ],
  [
    "Chars->Float",
    "[Char] -> Float",
    "Parses a vector of characters into a 32-bit float",
    ['(Chars->Float "1.1"); => 1.1'],
  ],
  [
    "Integer->Bits",
    "Int -> [Int]",
    "Converts a 32-bit integer into its binary representation (vector of 0s and 1s)",
    ["(Integer->Bits 5); => [0 0 ... 1 0 1]"],
  ],
  [
    "Char->Digit",
    "Char -> Int",
    "Converts a single numeric character to its integer value",
    ["(Char->Digit '9'); => 9"],
  ],
  [
    "Digit->Char",
    "Int -> Char",
    "Converts a single-digit integer to its character equivalent",
    ["(Digit->Char 7); => '7'"],
  ],
  [
    "Integer->Digits",
    "Int -> [Int]",
    "Breaks a 32-bit integer into a vector of its individual digits",
    ["(Integer->Digits 502); => [5 0 2]"],
  ],
  [
    "Digits->Chars",
    "[Int] -> [Char]",
    "Converts a vector of integer digits into a vector of characters",
    ["(Digits->Chars [1 0]); => ['1' '0']"],
  ],
  [
    "Set->Vector",
    "[[[T79]]] -> [[T79]]",
    "Flattens a Hashtable-based Set into a standard Vector of its present elements",
    ["(Set->Vector [[['a']] [[]] [['b']]]); => ['a' 'b']"],
  ],
  [
    "Vector->Set",
    "[[Char]] -> [[[Char]]]",
    "Converts a Vector of strings into a Hashtable Set; handles collisions and wraps items in buckets",
    ['(Vector->Set ["que" "lisp"]); => [[["que"]] [[]] [["lisp"]]]'],
  ],
  ["Vector/equal?", "[Int] -> [Int] -> Bool"],
  ["Set/size", "[[T63]] -> Int"],
  ["Table/size", "[[T63]] -> Int"],
  ["Set/new", "() -> [[T25]]"],
  ["Table/new", "() -> [[T25]]"],

  [
    "Set/intersection",
    "[[[Char]]] -> [[[Char]]] -> [[[Char]]]",
    "Returns a new Set containing only the elements present in both input Sets",
    ['(Set/intersection s1 s2); => [[["common"]] [[]]]'],
  ],
  [
    "Set/difference",
    "[[[Char]]] -> [[[Char]]] -> [[[Char]]]",
    "Returns a new Set containing elements from the first Set that are not in the second",
    ['(Set/difference s1 s2); => [[["unique-to-s1"]] [[]]]'],
  ],
  [
    "Set/xor",
    "[[[Char]]] -> [[[Char]]] -> [[[Char]]]",
    "Returns a new Set containing elements present in either Set, but not both (Symmetric Difference)",
    ['(Set/xor s1 s2); => [[["only-s1"]] [["only-s2"]]]'],
  ],
  [
    "Set/union",
    "[[[Char]]] -> [[[Char]]] -> [[[Char]]]",
    "Returns a new Set containing all unique elements from both input Sets",
    ['(Set/union s1 s2); => [[["a"]] [["b"]] [["c"]]]'],
  ],
  [
    "Set/add!",
    "[Char] -> [[[Char]]] -> ()",
    "Inserts a string into the Set in-place; handles hashing and bucket placement",
    ['(Set/add! "new-item" my-set); => ()'],
  ],
  [
    "Set/remove!",
    "[Char] -> [[[Char]]] -> ()",
    "Removes a string from the Set in-place if it exists",
    ['(Set/remove! "old-item" my-set); => ()'],
  ],
  [
    "Set/has?",
    "[Char] -> [[[Char]]] -> Bool",
    "Returns true if the string exists within the Hashtable buckets",
    ['(Set/has? "search-term" my-set); => true'],
  ],

  [
    "Table/entries",
    "[[[{T85 * T86}]]] -> [{T85 * T86}]",
    "Extracts all present key-value pairs from the hashtable buckets into a single vector of tuples",
    ['(Table/entries tbl); => [{1 "a"} {2 "b"}]'],
  ],
  [
    "Table/keys",
    "[[[{T83 * T82}]]] -> [T83]",
    "Returns a vector containing only the keys present in the hashtable",
    ["(Table/keys tbl); => [1 2]"],
  ],
  [
    "Table/values",
    "[[[{T82 * T83}]]] -> [T83]",
    "Returns a vector containing only the values present in the hashtable",
    ['(Table/values tbl); => ["a" "b"]'],
  ],
  [
    "Set/values",
    "[[T60]] -> [T60]",
    "Flattens a set's buckets to return a simple vector of all unique elements",
    ['(Set/values s); => ["que" "lisp"]'],
  ],

  [
    "Table/get",
    "[Char] -> [[[{[Char] * T104}]]] -> [{[Char] * T104}]",
    "Retrieves a key-value bucket for a given string key; returns an empty vector [] if not found",
    [
      '(Table/get "id" user-table); => [{"id" 101}]',
      '(Table/get "missing" user-table); => []',
    ],
  ],
  [
    "Table/get*",
    "[Char] -> [[[{[Char] * T138}]]] -> {Bool * [T138]}",
    "Returns a tuple containing a existence boolean and a vector containing the value (if present)",
    [
      '(Table/get* "name" user-table); => {true ["que"]}',
      '(Table/get* "age" user-table); => {false []}',
    ],
  ],
  [
    "Table/get!",
    "[Char] -> [[[{[Char] * T107}]]] -> T107",
    "Forcefully retrieves a value from the table; may crash or return undefined if the key is missing",
    ['(Table/get! "id" user-table); => 101'],
  ],
  [
    "Table/has?",
    "[Char] -> [[[{[Char] * T115}]]] -> Bool",
    "Checks if a specific string key exists within any bucket of the hashtable",
    ['(Table/has? "id" user-table); => true'],
  ],
  [
    "Table/set!",
    "[Char] -> T107 -> [[[{[Char] * T107}]]] -> ()",
    "Inserts or updates a key-value pair in the table in-place",
    ['(Table/set! "status" "active" config-table); => ()'],
  ],
  [
    "Table/remove!",
    "[Char] -> [[[{[Char] * T112}]]] -> ()",
    "Deletes a key and its associated value from the table in-place",
    ['(Table/remove! "temp-key" my-table); => ()'],
  ],
  [
    "Table/count",
    "[[Char]] -> [[[{[Char] * Int}]]]",
    "Creates a frequency table from a vector of strings, counting occurrences of each unique item",
    ['(Table/count ["a" "b" "a"]); => [[[{"a" 2}]] [[]] [[{"b" 1}]]]'],
  ],
  [
    "Table/drop!",
    "[[Char]] -> [[[{[Char] * T119}]]] -> ()",
    "Batch removes multiple keys from the table at once using a vector of strings",
    ['(Table/drop! ["tmp1" "tmp2"] my-table); => ()'],
  ],
  [
    "Table/keep",
    "[[Char]] -> [[[{[Char] * T147}]]] -> [[[{[Char] * [Char]}]]]",
    "Returns a new Table containing only the key-value pairs whose keys are present in the provided vector",
    [
      '(Table/keep ["id" "user"] my-table); => [[[{"id" 1}]] [[]] [[{"user" "que"}]]]',
    ],
  ],
  [
    "Table/omit",
    "[[Char]] -> [[[{[Char] * T140}]]] -> [[[{[Char] * T140}]]]",
    "Returns a new Table with all specified keys and their associated values removed",
    ['(Table/omit ["tmp" "secret"] my-table); => [[[{"id" 1}]] [[]]]'],
  ],
  [
    "Table/merge!",
    "[[[{[Char] * T189}]]] -> [[[{[Char] * T189}]]] -> ()",
    "Merges the first Table into the second in-place; overwrites existing keys in the target Table",
    ["(Table/merge! update-tbl main-tbl); => ()"],
  ],
  [
    "Table/merge",
    "[[[{[Char] * T207}]]] -> [[[{[Char] * T207}]]] -> [[[{[Char] * T207}]]]",
    "Combines two Tables into a new third Table; if keys collide, the value from the second Table takes precedence",
    ['(Table/merge t1 t2); => [[[{"k" "new-val"}]] [[]]]'],
  ],
  ["Que/new", "T16 -> [[T16]]"],
  ["Que/empty?", "[[T21]] -> Bool"],
  ["Que/not-empty?", "[[T23]] -> Bool"],
  ["Que/empty!", "[[T23]] -> ()"],
  ["Que/enque!", "T29 -> [[T29]] -> ()"],
  ["Que/deque!", "[[T100]] -> ()"],
  ["Que/peek", "[[T26]] -> T26"],
  ["Que/push!", "T26 -> [[T26]] -> ()"],
  ["Que/pop!", "[[T100]] -> ()"],
  ["Que/prepend!", "T26 -> [[T26]] -> ()"],
  ["Que/first", "[[T26]] -> T26"],
  ["Que/last", "[[T32]] -> T32"],
  ["Que/tail!", "[[T100]] -> ()"],
  ["Que/append!", "T26 -> [[T26]] -> ()"],
  ["Vector->Que", "[T42] -> [[T42]]"],
  ["Que->Vector", "[[T45]] -> [T45]"],
  ["Que/get", "[[T24]] -> Int -> T24"],
  ["Que/length", "[[T19]] -> Int"],
  ["Que/at", "Int -> [[T28]] -> T28"],

  [
    "BigInt/add",
    "[Int] -> [Int] -> [Int]",
    "Adds two BigInt vectors and returns the sum as a new vector",
    ["(BigInt/add [9 9] [1]); => [1 0 0]"],
  ],
  [
    "BigInt/sub",
    "[Int] -> [Int] -> [Int]",
    "Subtracts the second BigInt from the first",
    ["(BigInt/sub [1 0] [1]); => [9]"],
  ],
  [
    "BigInt/mul",
    "[Int] -> [Int] -> [Int]",
    "Multiplies two BigInt vectors",
    ["(BigInt/mul [1 2] [1 0]); => [1 2 0]"],
  ],
  [
    "BigInt/lte?",
    "[Int] -> [Int] -> Bool",
    "Checks if the first BigInt is less than or equal to the second",
    ["(BigInt/lte? [1 0] [2 0]); => true"],
  ],
  [
    "BigInt/gte?",
    "[Int] -> [Int] -> Bool",
    "Checks if the first BigInt is greater than or equal to the second",
    ["(BigInt/gte? [5] [5]); => true"],
  ],
  [
    "BigInt/lt?",
    "[Int] -> [Int] -> Bool",
    "Checks if the first BigInt is strictly less than the second",
    ["(BigInt/lt? [5] [1 0]); => true"],
  ],
  [
    "BigInt/gt?",
    "[Int] -> [Int] -> Bool",
    "Checks if the first BigInt is strictly greater than the second",
    ["(BigInt/gt? [1 0] [5]); => true"],
  ],
  [
    "BigInt/equal?",
    "[Int] -> [Int] -> Bool",
    "Checks if two BigInt vectors represent the same value",
    ["(BigInt/equal? [0 1] [1]); => true"],
  ],
  [
    "BigInt/div",
    "[Int] -> [Int] -> [Int]",
    "Performs integer division of two BigInts",
    ["(BigInt/div [1 0] [3]); => [3]"],
  ],
  [
    "BigInt/square",
    "[Int] -> [Int]",
    "Calculates the square of a BigInt vector",
    ["(BigInt/square [1 2]); => [1 4 4]"],
  ],
  [
    "BigInt/div/floor",
    "[Int] -> [Int] -> [Int]",
    "Divides two BigInts and rounds the result down",
    ["(BigInt/div/floor [7] [2]); => [3]"],
  ],
  [
    "BigInt/div/ceal",
    "[Int] -> [Int] -> [Int]",
    "Divides two BigInts and rounds the result up",
    ["(BigInt/div/ceal [7] [2]); => [4]"],
  ],
  [
    "BigInt/sum",
    "[[Int]] -> [Int]",
    "Calculates the total sum of a vector containing multiple BigInt vectors",
    ["(BigInt/sum [[1 0] [2 0] [3 0]]); => [6 0]"],
  ],
  [
    "BigInt/product",
    "[[Int]] -> [Int]",
    "Calculates the total product of a vector containing multiple BigInt vectors",
    ["(BigInt/product [[2] [5] [1 0]]); => [1 0 0]"],
  ],
  [
    "BigInt/new",
    "[Char] -> [Int]",
    "Parses a string into a BigInt vector representation",
    ['(BigInt/new "123456789"); => [1 2 3 4 5 6 7 8 9]'],
  ],
  [
    "BigInt/pow",
    "[Int] -> Int -> [Int]",
    "Raises a BigInt base to a standard 32-bit integer power",
    ["(BigInt/pow [2] 10); => [1 0 2 4]"],
  ],
  [
    "BigInt/expt",
    "[Int] -> [Int] -> [Int]",
    "Raises a BigInt base to a BigInt power",
    ["(BigInt/expt [2] [1 0]); => [1 0 2 4]"],
  ],
  [
    "BigInt/range",
    "Int -> Int -> [[Int]]",
    "Generates a vector of BigInts between two 32-bit integer bounds",
    ["(BigInt/range 8 11); => [[8] [9] [1 0]]"],
  ],
  [
    "Vector/push!",
    "[T22] -> T22 -> [T22]",
    "Appends an element to the end of a mutable vector in-place and returns the vector",
    ["(let xs [1 2]) (Vector/push! xs 3); => [1 2 3]"],
  ],
  [
    "Vector/pop!",
    "[T16] -> ()",
    "Removes the last element from a mutable vector in-place (returns nothing)",
    ["(let xs [1 2 3]) (Vector/pop! xs); => ()", "xs; => [1 2]"],
  ],
  [
    "Vector/last",
    "[T17] -> T17",
    "Returns the final element of a vector without modifying it",
    ["(Vector/last [10 20 30]); => 30"],
  ],
  [
    "Vector/first",
    "[T16] -> T16",
    "Returns the initial element (index 0) of a vector",
    ["(Vector/first [10 20 30]); => 10"],
  ],
  [
    "Vector/pull!",
    "[T20] -> T20",
    "Removes the last element from a vector in-place and returns that element",
    ["(let xs [1 2 3]) (Vector/pull! xs); => 3", "xs; => [1 2]"],
  ],
  [
    "Vector/get!",
    "Int -> [T16] -> T16",
    "Forcefully retrieves the element at index N; may crash if index is out of bounds",
    ["(Vector/get! 1 [10 20 30]); => 20"],
  ],
  [
    "Vector/get*",
    "Int -> [T18] -> {Bool * [T18]}",
    "Safely retrieves an element; returns a tuple of existence and the value wrapped in a bucket",
    [
      "(Vector/get* 1 [10 20]); => {true [20]}",
      "(Vector/get* 5 [10 20]); => {false []}",
    ],
  ],
  [
    "Vector/at!",
    "Int -> [T21] -> T21",
    "Retrieves an element using JS-style indexing (supports negative integers to count from end)",
    ["(Vector/at! -1 [10 20 30]); => 30"],
  ],
  [
    "Vector/at*",
    "Int -> [T23] -> {Bool * [T23]}",
    "Safe version of at!; returns a existence tuple and a bucketed value",
    ["(Vector/at* -1 [10 20]); => {true [20]}"],
  ],
  [
    "Vector/set!",
    "[T18] -> Int -> T18 -> ()",
    "Updates the element at a specific index in-place",
    ["(let xs [10 20]) (Vector/set! xs 0 99); => ()", "xs; => [99 20]"],
  ],
  [
    "Vector/length",
    "[T16] -> Int",
    "Returns the total number of elements currently in the vector",
    ["(Vector/length [1 2 3]); => 3"],
  ],
  [
    "Vector/append!",
    "T27 -> [T27] -> [T27]",
    "Alias for push!; adds an element to the end and returns the vector",
    ["(Vector/append! 3 [1 2]); => [1 2 3]"],
  ],

  ["Vector/2d/set!", "[T22] -> [T22] -> ()"],
  ["Vector/2d/add", "[Int] -> [Int] -> [Int]"],
  ["Vector/2d/sub", "[Int] -> [Int] -> [Int]"],
  ["Vector/2d/mul", "[Int] -> [Int] -> [Int]"],
  ["Vector/2d/div", "[Int] -> [Int] -> [Int]"],
  ["Vector/2d/mag/sqrt", "[Int] -> Int"],
  ["Vector/2d/mag", "[Int] -> Int"],
  ["Vector/2d/dot", "[Int] -> [Int] -> Int"],
  ["Vector/2d/cross", "[Int] -> [Int] -> [Int]"],
  ["Vector/2d/dist", "[Int] -> [Int] -> Int"],
  ["Vector/2d/norm", "[Int] -> [Int]"],
  ["Vector/2d/limit", "[Int] -> Int -> [Int]"],
  ["Vector/2d/add!", "[Int] -> [Int] -> ()"],
  ["Vector/2d/sub!", "[Int] -> [Int] -> ()"],
  ["Vector/2d/mul!", "[Int] -> [Int] -> ()"],
  ["Vector/2d/div!", "[Int] -> [Int] -> ()"],
  ["Vector/2d/norm!", "[Int] -> ()"],
  ["Vector/2d/limit!", "[Int] -> Int -> ()"],
  ["Vector/2d/mag!", "[Int] -> [Int] -> ()"],
  ["Vector/3d/set!", "[T24] -> [T24] -> ()"],
  ["Vector/3d/add", "[Int] -> [Int] -> [Int]"],
  ["Vector/3d/sub", "[Int] -> [Int] -> [Int]"],
  ["Vector/3d/mul", "[Int] -> [Int] -> [Int]"],
  ["Vector/3d/div", "[Int] -> [Int] -> [Int]"],
  ["Vector/3d/mag/sqrt", "[Int] -> Int"],
  ["Vector/3d/mag", "[Int] -> Int"],
  ["Vector/3d/dot", "[Int] -> [Int] -> Int"],
  ["Vector/3d/cross", "[Int] -> [Int] -> [Int]"],
  ["Vector/3d/dist", "[Int] -> [Int] -> Int"],
  ["Vector/3d/norm", "[Int] -> [Int]"],
  ["Vector/3d/limit", "[Int] -> Int -> [Int]"],
  ["Vector/3d/add!", "[Int] -> [Int] -> ()"],
  ["Vector/3d/sub!", "[Int] -> [Int] -> ()"],
  ["Vector/3d/mul!", "[Int] -> [Int] -> ()"],
  ["Vector/3d/div!", "[Int] -> [Int] -> ()"],
  ["Vector/3d/norm!", "[Int] -> ()"],
  ["Vector/3d/limit!", "[Int] -> Int -> ()"],
  ["Vector/3d/mag!", "[Int] -> [Int] -> ()"],

  [
    "Vector/new",
    "(Int -> T42) -> Int -> [T42]",
    "Generates a new vector of size N by calling a function for each index i",
    ["(Vector/new (lambda i (* i 10)) 3); => [0 10 20]"],
  ],
  [
    "Matrix/new",
    "(Int -> Int -> T45) -> Int -> Int -> [[T45]]",
    "Generates a 2D vector (Matrix) of size R x C using a generator function (row, col)",
    ["(Matrix/new (lambda r c (+ r c)) 2 2); => [[0 1] [1 2]]"],
  ],
  [
    "String/equal?",
    "[Char] -> [Char] -> Bool",
    "Returns true if two strings (vectors of chars) are identical",
    ['(String/equal? "que" "que"); => true'],
  ],
  [
    "String/lte?",
    "[Char] -> [Char] -> Bool",
    "Checks if the first string is lexicographically less than or equal to the second",
    ['(String/lte? "apple" "banana"); => true'],
  ],
  [
    "String/gte?",
    "[Char] -> [Char] -> Bool",
    "Checks if the first string is lexicographically greater than or equal to the second",
    ['(String/gte? "cat" "bat"); => true'],
  ],
  [
    "String/lt?",
    "[Char] -> [Char] -> Bool",
    "Checks if the first string is strictly less than the second in alphabetical order",
    ['(String/lt? "abc" "abd"); => true'],
  ],
  [
    "String/gt?",
    "[Char] -> [Char] -> Bool",
    "Checks if the first string is strictly greater than the second in alphabetical order",
    ['(String/gt? "zoo" "apple"); => true'],
  ],
  [
    "Char/eq?",
    "Char -> Char -> Bool",
    "Strict equality check for two 32-bit characters",
    ["(Char/eq? 'a' 'a'); => true"],
  ],
  [
    "Int/eq?",
    "Int -> Int -> Bool",
    "Strict equality check for two 32-bit integers",
    ["(Int/eq? 10 10); => true"],
  ],
  [
    "Bool/eq?",
    "Bool -> Bool -> Bool",
    "Strict equality check for two boolean values",
    ["(Bool/eq? true true); => true"],
  ],
  [
    "Float/eq?",
    "Float -> Float -> Bool",
    "Strict equality check for two 32-bit floating point numbers",
    ["(Float/eq? 1.5 1.5); => true"],
  ],
];

const vscode = require("vscode");

function splitSignatureTopLevel(sig) {
  const parts = [];
  let buf = "";

  let paren = 0;
  let bracket = 0;
  let inString = false;

  for (let i = 0; i < sig.length; i++) {
    const ch = sig[i];
    const prev = i > 0 ? sig[i - 1] : "";
    const next = i + 1 < sig.length ? sig[i + 1] : "";

    if (ch === '"' && prev !== "\\") {
      inString = !inString;
      buf += ch;
      continue;
    }

    if (!inString) {
      if (ch === "(") paren++;
      else if (ch === ")") paren = Math.max(0, paren - 1);
      else if (ch === "[") bracket++;
      else if (ch === "]") bracket = Math.max(0, bracket - 1);

      // split only on top-level "->"
      if (ch === "-" && next === ">" && paren === 0 && bracket === 0) {
        parts.push(buf.trim());
        buf = "";
        i++; // skip '>'
        continue;
      }
    }

    buf += ch;
  }

  if (buf.trim().length) parts.push(buf.trim());
  return parts;
}

function registerQueSignatureHelp(cleanedLibrary) {
  // Trigger on a bunch of characters so signature help updates while typing arguments.
  // VS Code only calls provideSignatureHelp when a trigger char is typed (or user manually retriggers).
  const triggerChars = makeTriggerChars();

  return vscode.languages.registerSignatureHelpProvider(
    "que",
    {
      provideSignatureHelp(document, position) {
        // Read a window of text before the cursor (works across lines)
        const cursorOffset = document.offsetAt(position);
        const WINDOW = 8000;
        const startOffset = Math.max(0, cursorOffset - WINDOW);
        const textBeforeCursor = document.getText(
          new vscode.Range(document.positionAt(startOffset), position)
        );

        // Find start of the current call: the last unmatched '(' at top level
        const callStartInWindow = findCallStart(textBeforeCursor);
        if (callStartInWindow === -1) return null;

        const content = textBeforeCursor.slice(callStartInWindow + 1); // after '(' up to cursor
        const { tokens, endsWithTopLevelSpace } = tokenizeTopLevel(content);
        if (tokens.length === 0) return null;

        const funcName = tokens[0];
        const libEntry = cleanedLibrary.find((x) => x.name === funcName);
        if (!libEntry) return null;

        // Split signature by top-level "->" (so "(T -> T)" stays ONE parameter)
        const allParts = splitSignatureTopLevel(libEntry.signature);
        const paramLabels = allParts.slice(0, -1); // inputs only

        const fullLabel = `${libEntry.name}: ${libEntry.signature}`;

        const signatureInfo = new vscode.SignatureInformation(
          fullLabel,
          new vscode.MarkdownString(libEntry.description ?? "")
        );

        // Build ParameterInformation using [start,end] ranges inside fullLabel
        let searchFrom = 0;
        signatureInfo.parameters = paramLabels.map((p) => {
          const idx = fullLabel.indexOf(p, searchFrom);
          if (idx === -1) {
            // fallback: if not found, still return string label
            return new vscode.ParameterInformation(p);
          }
          const start = idx;
          const end = idx + p.length;
          searchFrom = end; // IMPORTANT: move forward so the 2nd "Int" finds the next one
          return new vscode.ParameterInformation([start, end]);
        });

        const help = new vscode.SignatureHelp();
        help.signatures = [signatureInfo];
        help.activeSignature = 0;

        // tokens = [func, arg1, arg2, ...]
        const argsTyped = Math.max(0, tokens.length - 1);

        // If cursor is after a top-level space, user is starting NEXT arg.
        // Otherwise they are typing current arg.
        let activeParam = endsWithTopLevelSpace
          ? argsTyped
          : Math.max(0, argsTyped - 1);

        activeParam = Math.min(
          activeParam,
          Math.max(0, paramLabels.length - 1)
        );
        help.activeParameter = activeParam;
        return help;
      },
    },
    ...triggerChars
  );
}

function makeTriggerChars() {
  const chars = ["(", " ", "[", "]", '"', ")"];

  // digits
  for (let c = 48; c <= 57; c++) chars.push(String.fromCharCode(c));
  // a-z
  for (let c = 97; c <= 122; c++) chars.push(String.fromCharCode(c));
  // A-Z
  for (let c = 65; c <= 90; c++) chars.push(String.fromCharCode(c));

  // common symbol starters in lisps
  chars.push("-", "+", "*", "/", "?", "!", "_", ":", ".");

  // de-dupe
  return Array.from(new Set(chars));
}

function findCallStart(text) {
  let inString = false;
  let bracketDepth = 0;
  let parenDepth = 0;

  for (let i = text.length - 1; i >= 0; i--) {
    const ch = text[i];
    const prev = i > 0 ? text[i - 1] : "";

    if (ch === '"' && prev !== "\\") {
      inString = !inString;
      continue;
    }
    if (inString) continue;

    if (ch === "]") {
      bracketDepth++;
      continue;
    }
    if (ch === "[") {
      bracketDepth = Math.max(0, bracketDepth - 1);
      continue;
    }
    if (bracketDepth > 0) continue;

    if (ch === ")") {
      parenDepth++;
      continue;
    }
    if (ch === "(") {
      if (parenDepth === 0) return i;
      parenDepth--;
      continue;
    }
  }

  return -1;
}

function tokenizeTopLevel(content) {
  const tokens = [];
  let cur = "";

  let inString = false;
  let bracketDepth = 0;
  let parenDepth = 0;

  let lastWasTopLevelSpace = false;

  for (let i = 0; i < content.length; i++) {
    const ch = content[i];
    const prev = i > 0 ? content[i - 1] : "";

    if (ch === '"' && prev !== "\\") {
      inString = !inString;
      cur += ch;
      lastWasTopLevelSpace = false;
      continue;
    }

    if (!inString) {
      if (ch === "[") bracketDepth++;
      else if (ch === "]") bracketDepth = Math.max(0, bracketDepth - 1);
      else if (ch === "(") parenDepth++;
      else if (ch === ")") parenDepth = Math.max(0, parenDepth - 1);
    }

    const atTopLevel = !inString && bracketDepth === 0 && parenDepth === 0;

    if (atTopLevel && /\s/.test(ch)) {
      if (cur.length > 0) {
        tokens.push(cur);
        cur = "";
      }
      lastWasTopLevelSpace = true;
      continue;
    }

    cur += ch;
    lastWasTopLevelSpace = false;
  }

  if (cur.length > 0) tokens.push(cur);

  return { tokens, endsWithTopLevelSpace: lastWasTopLevelSpace };
}
/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
  // 1. Clean the data (Removes numbers from signatures like T14 -> T)
  const cleanedLibrary = standardLibrary.map(
    ([name, signature, description, examples]) => {
      return {
        name,
        signature: signature.replace(/\d+/g, ""),
        description,
        examples,
      };
    }
  );

  const items = cleanedLibrary.map(({ name, signature, description }) => {
    // 1. Constructor ONLY takes the label object and the Kind
    const item = new vscode.CompletionItem(
      {
        label: name,
        description: signature, // Shows signature on the right
      },
      signature.includes("->")
        ? vscode.CompletionItemKind.Function
        : vscode.CompletionItemKind.Constant
    );

    // 2. These MUST be set on the instance
    item.documentation = new vscode.MarkdownString(description);
    item.detail = signature; // Shows in the focus bar
    item.insertText = new vscode.SnippetString(name);

    return item;
  });

  // 3. Register the provider
  const provider = vscode.languages.registerCompletionItemProvider("que", {
    provideCompletionItems() {
      return items;
    },
  });
  vscode.languages.registerHoverProvider("que", {
    provideHover(document, position) {
      // 1. Get the range of the word at the mouse position
      const range = document.getWordRangeAtPosition(position, /[\w\/\-\?]+/);

      // SAFETY CHECK: If no word is found (whitespace/symbols), stop here
      if (!range) {
        return null;
      }

      const word = document.getText(range);

      // 2. Find the item in your library
      const libEntry = cleanedLibrary.find((item) => item.name === word);

      if (libEntry) {
        const hoverContent = new vscode.MarkdownString();

        // Formatting the hover nicely
        hoverContent.appendMarkdown(`### ${libEntry.name}\n`);
        hoverContent.appendCodeblock(libEntry.signature, "que");
        if (libEntry.description)
          hoverContent.appendMarkdown(`---\n${libEntry.description}\n`);
        if (libEntry.examples) {
          hoverContent.appendMarkdown(`\n**Examples:**\n`);
          libEntry.examples.forEach((ex) =>
            hoverContent.appendCodeblock(ex, "que")
          );
        }
        // 3. Return the Hover WITH the valid range
        return new vscode.Hover(hoverContent, range);
      }
      return null;
    },
  });

  const signatureProvider = registerQueSignatureHelp(cleanedLibrary);
  context.subscriptions.push(provider);
  context.subscriptions.push(signatureProvider);
}

// VS Code calls this when your extension is shut down
function deactivate() {}

module.exports = {
  activate,
  deactivate,
};
