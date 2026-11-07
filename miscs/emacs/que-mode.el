;;; que-mode.el --- Major mode and Eglot support for Calque -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Calque contributors

;; Author: Calque contributors
;; Keywords: languages, lisp, tools
;; Package-Requires: ((emacs "29.1"))

;;; Commentary:

;; `que-mode' provides editing support for Calque source files.  It supplies
;; font locking, Lisp-style indentation, formatter and runner commands, and
;; connects Emacs's built-in Eglot client to `quelsp' when it is available.
;;
;; Add this directory to `load-path' and use:
;;
;;   (require 'que-mode)
;;
;; See README.md in this directory for a complete setup example.

;;; Code:

(require 'compile)
(require 'lisp-mode)

(defgroup que nil
  "Editing Calque source files."
  :group 'languages
  :prefix "que-")

(defcustom que-lsp-server-command '("quelsp")
  "Command and arguments used to start the Calque language server."
  :type '(repeat string)
  :group 'que)

(defcustom que-eglot-auto-start t
  "Whether `que-mode' starts Eglot when `quelsp' is available.

Set this to nil when another package manages the language server."
  :type 'boolean
  :group 'que)

(defcustom que-run-command "que"
  "Calque command used by formatter and runner commands."
  :type 'string
  :group 'que)

(defvar que-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?( "()" table)
    (modify-syntax-entry ?) ")(" table)
    (modify-syntax-entry ?[ "(]" table)
    (modify-syntax-entry ?] ")[" table)
    (modify-syntax-entry ?{ "(}" table)
    (modify-syntax-entry ?} "){" table)
    (modify-syntax-entry ?\" "\"" table)
    ;; Calque characters use single quotes.  Giving them string syntax also
    ;; keeps comments and delimiters inside a character literal unambiguous.
    (modify-syntax-entry ?' "\"" table)
    (modify-syntax-entry ?\\ "\\" table)
    (modify-syntax-entry ?; "< b" table)
    (modify-syntax-entry ?\n "> b" table)
    ;; Punctuation in names such as `Table/set!' and `safe?' is intentional.
    (dolist (character '(?_ ?/ ?! ?? ?& ?* ?+ ?- ?= ?< ?> ?.))
      (modify-syntax-entry character "w" table))
    table)
  "Syntax table used by `que-mode'.")

(defconst que--keywords
  '("lambda" "comp" "if" "let" "letrec" "letmacro" "mut" "variable"
    "do" "block" "while" "enum" "record" "match" "match/string" "cond"
    "unless" "when" "when-not" "loop" "and" "or" "not" "quote" "qq"
    "uq" "uqs" "gensym" "macroexpand" "macroexpand-1" "as" "sig")
  "Calque special forms and control-flow words.")

(defconst que--builtins
  '("length" "car" "cdr" "cons" "get" "set!" "push!" "pop!" "pop-val!"
    "&get" "&mut" "&alter!" "alter!" "first" "last" "fst" "snd"
    "map" "filter" "reduce" "sum" "some" "every" "range" "join"
    "split" "print!" "println!" "read!" "write!" "sleep!" "serialize"
    "deserialize")
  "Common Calque built-ins for basic font locking.")

(defconst que-font-lock-keywords
  `((,(regexp-opt que--keywords 'symbols) . font-lock-keyword-face)
    (,(regexp-opt que--builtins 'symbols) . font-lock-builtin-face)
    ("\\_<\\(?:true\\|false\\|nil\\)\\_>" . font-lock-constant-face)
    ("\\(?:|>\\|<|\\|=~\\|[+*/%~-]\\.?\\)" . font-lock-operator-face)
    ("\\_<[[:upper:]][[:alnum:]_/]*\\_>" . font-lock-type-face)
    ("'\\(?:\\\\.\\|[^'\\\\]\\)'" . font-lock-constant-face))
  "Font-lock rules for `que-mode'.")

(defun que--file-name ()
  "Return the current buffer's file name or signal a useful user error."
  (or buffer-file-name
      (user-error "Save this Calque buffer before running it")))

(defun que--command (&rest arguments)
  "Build a shell-safe command using `que-run-command' and ARGUMENTS."
  (mapconcat #'shell-quote-argument (cons que-run-command arguments) " "))

;;;###autoload
(defun que-format-buffer ()
  "Format the current buffer with `que fmt --stdin'."
  (interactive)
  (let ((output (generate-new-buffer " *que-format*"))
        (point-before (point))
        (status nil))
    (unwind-protect
        (progn
          (setq status
                (call-process-region (point-min) (point-max) que-run-command
                                     nil output nil "fmt" "--stdin"))
          (if (equal status 0)
              (let ((formatted (with-current-buffer output (buffer-string))))
                (erase-buffer)
                (insert formatted)
                (goto-char (min point-before (point-max)))
                (message "Formatted Calque buffer"))
            (with-current-buffer output
              (compilation-mode))
            (display-buffer output)
            (user-error "Calque formatter failed")))
      (unless (and (buffer-live-p output) (get-buffer-window output))
        (when (buffer-live-p output)
          (kill-buffer output))))))

;;;###autoload
(defun que-run ()
  "Run the current Calque file with optimized execution in a compilation buffer."
  (interactive)
  (compilation-start (que--command (que--file-name) "--opt") 'compilation-mode))

;;;###autoload
(defun que-debug ()
  "Run the current Calque file with debug safety checks."
  (interactive)
  (compilation-start (que--command (que--file-name) "--debug") 'compilation-mode))

;;;###autoload
(defun que-wat ()
  "Show optimized WebAssembly text for the current Calque file."
  (interactive)
  (compilation-start (que--command (que--file-name) "--opt" "--emit" "wat")
                     'compilation-mode))

;;;###autoload
(defun que-explain ()
  "Show Calque's static-analysis explanation for the current file."
  (interactive)
  (compilation-start (que--command "explain" (que--file-name)) 'compilation-mode))

(defun que-eglot-ensure ()
  "Configure and start Eglot for this Calque buffer when possible."
  (when (and que-eglot-auto-start
             (executable-find (car que-lsp-server-command))
             (require 'eglot nil t))
    (setf (alist-get 'que-mode eglot-server-programs)
          que-lsp-server-command)
    (eglot-ensure)))

(defvar que-mode-map
  (let ((map (make-sparse-keymap)))
    (define-key map (kbd "C-c C-f") #'que-format-buffer)
    (define-key map (kbd "C-c C-r") #'que-run)
    (define-key map (kbd "C-c C-d") #'que-debug)
    (define-key map (kbd "C-c C-w") #'que-wat)
    (define-key map (kbd "C-c C-e") #'que-explain)
    map)
  "Keymap for `que-mode'.")

;;;###autoload
(define-derived-mode que-mode prog-mode "Calque"
  "Major mode for editing Calque / Eclisp source."
  :syntax-table que-mode-syntax-table
  (setq-local font-lock-defaults '(que-font-lock-keywords))
  (setq-local comment-start ";")
  (setq-local comment-end "")
  (setq-local indent-line-function #'lisp-indent-line)
  (setq-local lisp-indent-function #'lisp-indent-function)
  (que-eglot-ensure))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.que\\'" . que-mode))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.eclisp\\'" . que-mode))

(provide 'que-mode)

;;; que-mode.el ends here
