use std::env;
use std::fs;
use std::io;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const LEARN: &str = r#"Que quick reference

Bindings: (let x value), (mut x value), (alter! x value), (letrec f (lambda ...))
Functions: (lambda (a b) body...), called as (f a b); extra body forms are sequenced.
Data: vectors [1 2 3], tuples {a b}, strings "text", characters 'x'.
Control: (if test yes no), (cond test value ... default), (while test body...), (loop i test body...).
Mutation: (push! xs x), (set! xs i x), (pop! xs), (&mut cell), (&alter! cell value).
Pipelines pass data last: (|> xs (map f) (select pred)).
Use (block ...) for lexical branch/loop scopes so repeated local names do not collide.
Comments begin with ; and continue to the end of the line.
Run `que --examples`, `que --style`, or `que --pitfalls` for more."#;

const EXAMPLES: &str = r#"Que examples

; Functional
(|> [1 2 3 4] (map (lambda (x) (* x x))))

; Imperative implementation
(let sum (lambda (xs)
  (mut total 0)
  (loop i (< i (length xs))
    (alter! total (+ total (get xs i))))
  total))
(sum [10 20 30])

; Scoped branch locals
(if true
    (block (let value 1) value)
    (block (let value 2) value))"#;

const STYLE: &str = r#"Que style

- Prefer clear functional composition for transformations and loops/mutation for hot code.
- Public collection helpers normally take data last so they compose with |>.
- Put caller-visible mutated values first.
- Use block around branch-local and loop-local bindings.
- Cache lengths and repeated computations in hot loops.
- Use --debug while checking correctness and --opt for release execution.
- Use ; use-strict-warnings! at the start of a file for LSP correctness warnings."#;

const PITFALLS: &str = r#"Que pitfalls

- do sequences expressions but does not create a lexical scope; block does.
- get is unchecked under --opt unless the compiler proves or you guard the index.
- Int is signed i32; guard arithmetic in strict/debug code when overflow is possible.
- Mutation visible to a caller must be rooted in the function's first parameter.
- deserialize needs a concrete expected type.
- A partial application is a function value, so a missing argument can look like closure capture.
- Use grouped lambda parameters: (lambda (a b) body...)."#;

fn nvim_lua() -> &'static str {
    r#"lua do
local function run(extra, terminal, filetype)
  vim.cmd('silent write')
  local cmd
  if extra[1] == '__explain' then
    cmd = { vim.env.QUE_NVIM_EXE, 'explain', vim.api.nvim_buf_get_name(0), '--opt' }
  else
    cmd = { vim.env.QUE_NVIM_EXE, vim.api.nvim_buf_get_name(0) }
    local i, count = 0, tonumber(vim.env.QUE_NVIM_ARG_COUNT or '0')
    while i < count do
      local value = vim.env['QUE_NVIM_ARG_' .. i]
      if value == '--debug' then
        local following = vim.env['QUE_NVIM_ARG_' .. (i + 1)]
        if following == 'basic' or following == 'code' or following == 'types' or following == 'all' then i = i + 1 end
      elseif value == '--emit' or value == '--out' then
        i = i + 1
      elseif value ~= '--opt' and value ~= '--emit-source' then
        table.insert(cmd, value)
      end
      i = i + 1
    end
    for _, value in ipairs(extra) do table.insert(cmd, value) end
  end
  vim.cmd('botright new')
  if terminal then vim.fn.termopen(cmd); vim.cmd('startinsert'); return end
  local buf = vim.api.nvim_get_current_buf()
  vim.bo[buf].buftype='nofile'; vim.bo[buf].bufhidden='wipe'; vim.bo[buf].swapfile=false
  vim.bo[buf].filetype=filetype or ''; vim.api.nvim_buf_set_lines(buf,0,-1,false,{})
  vim.fn.jobstart(cmd,{stdout_buffered=true,stderr_buffered=true,
    on_stdout=function(_,d) if d then vim.schedule(function() if vim.api.nvim_buf_is_valid(buf) then vim.api.nvim_buf_set_lines(buf,-1,-1,false,d) end end) end end,
    on_stderr=function(_,d) if d then vim.schedule(function() if vim.api.nvim_buf_is_valid(buf) then vim.api.nvim_buf_set_lines(buf,-1,-1,false,d) end end) end end})
end
local function open_library_source(name)
  local lines = vim.fn.systemlist({vim.env.QUE_NVIM_EXE,'--lib','source',name})
  if vim.v.shell_error ~= 0 then vim.notify(table.concat(lines,'\n'),vim.log.levels.ERROR); return end
  vim.cmd('botright new')
  local buf=vim.api.nvim_get_current_buf()
  vim.bo[buf].buftype='nofile'; vim.bo[buf].bufhidden='wipe'; vim.bo[buf].swapfile=false; vim.bo[buf].filetype='que'
  vim.api.nvim_buf_set_lines(buf,0,-1,false,lines)
  vim.api.nvim_buf_set_name(buf,'que-lib://'..name)
end
local function browse_library()
  vim.ui.input({prompt='Que library pattern: ',default='*'},function(pattern)
    if not pattern or pattern=='' then return end
    local lines=vim.fn.systemlist({vim.env.QUE_NVIM_EXE,'--lib','types',pattern})
    if vim.v.shell_error ~= 0 then vim.notify(table.concat(lines,'\n'),vim.log.levels.ERROR); return end
    local entries={}
    for _,line in ipairs(lines) do
      if line~='' then table.insert(entries,{name=line:match('^([^%s]+)%s+:') or line,text=line}) end
    end
    if #entries==0 then vim.notify('No Que library symbols match '..pattern,vim.log.levels.INFO); return end
    local ok,pickers=pcall(require,'telescope.pickers')
    local fok,finders=pcall(require,'telescope.finders')
    local sok,sorters=pcall(require,'telescope.sorters')
    if ok and fok and sok then
      pickers.new({}, {
        prompt_title='Que library types: '..pattern,
        finder=finders.new_table({results=entries,entry_maker=function(entry) return {value=entry,display=entry.text,ordinal=entry.text} end}),
        sorter=sorters.get_generic_fuzzy_sorter(),
        attach_mappings=function(prompt,map)
          local actions=require('telescope.actions'); local state=require('telescope.actions.state')
          local function open_selected()
            local selected=state.get_selected_entry(); actions.close(prompt)
            if selected and selected.value then open_library_source(selected.value.name) end
          end
          map('i','<CR>',open_selected); map('n','<CR>',open_selected); return true
        end,
      }):find()
      return
    end
    vim.ui.select(entries,{prompt='Que library types: '..pattern,format_item=function(entry) return entry.text end},function(entry)
      if entry then open_library_source(entry.name) end
    end)
  end)
end
local modes={
  QueRun={{'--opt'},true,''}, QueDebug={{'--debug'},true,''},
  QueWat={{'--opt','--emit','wat'},false,'wat'}, QueTypes={{'--opt','--emit','types'},false,'que'},
  QueExplain={{'__explain'},false,'markdown'}, QueSource={{'--opt','--emit','source'},false,'que'}}
local keys={QueRun='r',QueDebug='d',QueWat='w',QueTypes='a',QueExplain='e',QueSource='z'}
for name,mode in pairs(modes) do
  vim.api.nvim_create_user_command(name,function() run(mode[1],mode[2],mode[3]) end,{})
  vim.keymap.set('n','<leader>'..keys[name],'<cmd>'..name..'<CR>',{silent=true,buffer=true})
end
vim.api.nvim_create_user_command('QueLib',browse_library,{desc='Browse Que library types'})
vim.keymap.set('n','<leader>g','<cmd>QueLib<CR>',{silent=true,buffer=true,desc='Browse Que library types'})
end"#
}

fn run_nvim(mut args: Vec<String>) -> Result<(), String> {
    if matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
        println!("Usage: que nvim [--code <source>] [program arguments and flags]\n\n<leader>r runs optimized; d debugs; w emits WAT; a types; e explains; z emits source; g browses library types.");
        return Ok(());
    }
    let initial = if let Some(index) = args.iter().position(|arg| arg == "--code") {
        if index + 1 >= args.len() {
            return Err("que nvim --code requires Que source".into());
        }
        let source = args.remove(index + 1);
        args.remove(index);
        source
    } else {
        String::new()
    };
    let cwd = env::current_dir().map_err(|error| error.to_string())?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_nanos())
        .unwrap_or(0);
    let source = cwd.join(format!("que-script-{nonce}.que"));
    let marker = source.with_extension("run");
    fs::write(&source, initial).map_err(|error| error.to_string())?;
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let mut editor = Command::new("nvim");
    editor
        .env("QUE_NVIM_EXE", &executable)
        .env("QUE_NVIM_ARG_COUNT", args.len().to_string());
    for (index, arg) in args.iter().enumerate() {
        editor.env(format!("QUE_NVIM_ARG_{index}"), arg);
    }
    let status = editor
        .arg("-c").arg("setlocal filetype=que")
        .arg("-c").arg(format!("lua vim.api.nvim_create_autocmd('BufWritePost',{{buffer=0,callback=function() vim.fn.writefile({{'run'}},'{}') end}})", marker.display()))
        .arg("-c").arg(nvim_lua()).arg("-c").arg("setlocal modified").arg(&source).status();
    let result = match status {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err("nvim was not found in PATH".into())
        }
        Err(error) => Err(format!("failed to launch nvim: {error}")),
        Ok(status) if !status.success() || !marker.exists() => Ok(()),
        Ok(_) => Command::new(&executable)
            .arg(&source)
            .args(&args)
            .status()
            .map_err(|error| format!("failed to run scratch program: {error}"))
            .and_then(|status| {
                if status.success() {
                    Ok(())
                } else {
                    Err("scratch program failed; see the Que diagnostic above".into())
                }
            }),
    };
    let _ = fs::remove_file(source);
    let _ = fs::remove_file(marker);
    result
}

fn main() {
    match env::args().nth(1).as_deref() {
        Some("--learn") => {
            println!("{LEARN}");
            return;
        }
        Some("--examples") => {
            println!("{EXAMPLES}");
            return;
        }
        Some("--style") => {
            println!("{STYLE}");
            return;
        }
        Some("--pitfalls") => {
            println!("{PITFALLS}");
            return;
        }
        Some("nvim") => {
            if let Err(error) = run_nvim(env::args().skip(2).collect()) {
                eprintln!("\x1b[31mException: {error}\x1b[0m");
                std::process::exit(1);
            }
            return;
        }
        _ => {}
    }
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let compiler_command = args.first().is_some_and(|arg| {
        matches!(
            arg.as_str(),
            "--help"
                | "-h"
                | "--version"
                | "-V"
                | "compile"
                | "run-wasi"
                | "wat"
                | "explain"
                | "fmt"
                | "--eval"
                | "-e"
                | "--lib"
                | "--env"
        )
    }) || args.iter().any(|arg| arg == "--emit");
    if !compiler_command {
        args.insert(0, "run-wasi".into());
    }
    if let Err(error) = que::compiler_cli::run_with_args(args) {
        eprintln!("\x1b[31mException: {error}\x1b[0m");
        std::process::exit(1);
    }
}
