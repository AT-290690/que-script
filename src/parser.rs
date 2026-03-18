use std::collections::{ HashMap, HashSet };

const MAX_MACRO_EXPANSION_DEPTH: usize = 64;

fn is_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "map" |
            "map/i" |
            "filter" |
            "filter/i" |
            "select" |
            "exclude" |
            "reduce" |
            "reduce/i" |
            "reduce/until" |
            "reduce/until/i" |
            "sum" |
            "sum/int" |
            "sum/dec" |
            "product" |
            "product/int" |
            "product/dec" |
            "some?" |
            "some/i?" |
            "every?" |
            "every/i?" |
            "find" |
            "zip" |
            "unzip" |
            "flat" |
            "flat-map" |
            "window" |
            "char" |
            "mean" |
            "mean/int" |
            "mean/dec" |
            "range" |
            "range/int" |
            "range/dec" |
            "slice" |
            "take/first" |
            "drop/first" |
            "take/last" |
            "drop/last" |
            "&mut" |
            "&box" |
            "&get" |
            "&alter!"
    )
}

fn validate_reserved_words_in_binders(expr: &Expression) -> Result<(), String> {
    match expr {
        Expression::Apply(list) if !list.is_empty() => {
            if let Expression::Word(op) = &list[0] {
                match op.as_str() {
                    "let" | "let*" | "mut" => {
                        if let Some(Expression::Word(name)) = list.get(1) {
                            if is_reserved_word(name) {
                                return Err(format!("Variable '{}' is forbidden", name));
                            }
                        }
                    }
                    "lambda" => {
                        for p in &list[1..list.len().saturating_sub(1)] {
                            let mut names = HashSet::new();
                            collect_pattern_words(p, &mut names);
                            for name in names {
                                if is_reserved_word(&name) {
                                    return Err(format!("Variable '{}' is forbidden", name));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            for child in list {
                validate_reserved_words_in_binders(child)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn collect_pattern_words(expr: &Expression, acc: &mut HashSet<String>) {
    match expr {
        Expression::Word(w) => {
            if w != "_" {
                acc.insert(w.clone());
            }
        }
        Expression::Apply(exprs) => {
            for e in exprs {
                collect_pattern_words(e, acc);
            }
        }
        _ => {}
    }
}

fn collect_free_idents(expr: &Expression, bound: &mut HashSet<String>, acc: &mut HashSet<String>) {
    match expr {
        Expression::Word(w) => {
            if !bound.contains(w) {
                acc.insert(w.clone());
            }
        }
        Expression::Apply(exprs) => {
            if exprs.is_empty() {
                return;
            }
            if let Expression::Word(op) = &exprs[0] {
                if op == "lambda" {
                    let mut scoped = bound.clone();
                    for p in &exprs[1..exprs.len().saturating_sub(1)] {
                        collect_pattern_words(p, &mut scoped);
                    }
                    if let Some(body) = exprs.last() {
                        collect_free_idents(body, &mut scoped, acc);
                    }
                    return;
                }
                if op == "do" {
                    for it in &exprs[1..] {
                        if let Expression::Apply(let_items) = it {
                            if
                                let [Expression::Word(kw), Expression::Word(name), rhs] =
                                    &let_items[..]
                            {
                                if kw == "let" || kw == "let*" {
                                    collect_free_idents(rhs, bound, acc);
                                    bound.insert(name.clone());
                                    continue;
                                }
                            }
                        }
                        collect_free_idents(it, bound, acc);
                    }
                    return;
                }
                if op == "let" || op == "let*" {
                    if let [_, Expression::Word(name), rhs] = &exprs[..] {
                        collect_free_idents(rhs, bound, acc);
                        bound.insert(name.clone());
                        return;
                    }
                }
            }
            for e in exprs {
                collect_free_idents(e, bound, acc);
            }
        }
        _ => {}
    }
}

fn tree_shake(
    std_defs: Vec<Expression>,
    used: &HashSet<String>,
    visited: &mut HashSet<String>
) -> Vec<Expression> {
    let mut index = HashMap::new();

    // Index all std let definitions
    for expr in &std_defs {
        if let Expression::Apply(list) = expr {
            if let [Expression::Word(kw), Expression::Word(name), _rest @ ..] = &list[..] {
                if kw == "let" {
                    index.insert(name.clone(), expr.clone());
                }
            }
        }
    }

    let mut kept = Vec::new();

    fn visit(
        name: &str,
        index: &HashMap<String, Expression>,
        kept: &mut Vec<Expression>,
        visited: &mut HashSet<String>
    ) {
        if !visited.insert(name.to_string()) {
            return;
        }
        if let Some(def) = index.get(name) {
            if let Expression::Apply(list) = def {
                if list.len() >= 3 {
                    let mut deps = HashSet::new();
                    let mut scoped = HashSet::new();
                    collect_free_idents(&list[2], &mut scoped, &mut deps);
                    for dep in deps {
                        visit(&dep, index, kept, visited);
                    }
                }
            }
            kept.push(def.clone());
        }
    }

    for name in used {
        visit(name, &index, &mut kept, visited);
    }

    kept
}

fn flush(buf: &mut String, out: &mut Vec<String>) {
    if !buf.is_empty() {
        out.push(std::mem::take(buf));
    }
}

#[derive(Clone, Copy)]
enum DelimiterMode {
    ParenOnly,
    SourceDelimiters,
}

#[derive(Clone, Copy)]
struct OpenDelimiter {
    open: char,
    close: char,
    line: usize,
    col: usize,
}

fn delimiter_close_for_open(ch: char, mode: DelimiterMode) -> Option<char> {
    match mode {
        DelimiterMode::ParenOnly => if ch == '(' { Some(')') } else { None }
        DelimiterMode::SourceDelimiters =>
            match ch {
                '(' => Some(')'),
                '[' => Some(']'),
                '{' => Some('}'),
                _ => None,
            }
    }
}

fn is_delimiter_close(ch: char, mode: DelimiterMode) -> bool {
    match mode {
        DelimiterMode::ParenOnly => ch == ')',
        DelimiterMode::SourceDelimiters => matches!(ch, ')' | ']' | '}'),
    }
}

fn ensure_line_delta(line_deltas: &mut Vec<i32>, line: usize) {
    if line_deltas.len() < line {
        line_deltas.resize(line, 0);
    }
}

fn push_open_stack_lines(out: &mut Vec<String>, stack: &[OpenDelimiter]) {
    if stack.is_empty() {
        out.push("parse.open_stack: <empty>".to_string());
        return;
    }

    for (idx, item) in stack.iter().rev().take(8).enumerate() {
        out.push(
            format!(
                "parse.open_stack[{}]: '{}' opened at {}:{} expects '{}'",
                idx,
                item.open,
                item.line,
                item.col,
                item.close
            )
        );
    }
    if stack.len() > 8 {
        out.push(format!("parse.open_stack_more: {}", stack.len() - 8));
    }
}

fn push_balance_window_lines(
    out: &mut Vec<String>,
    source_lines: &[&str],
    line_deltas: &[i32],
    focus_line: usize
) {
    if line_deltas.is_empty() {
        return;
    }

    let mut cumulative = Vec::with_capacity(line_deltas.len());
    let mut bal = 0i32;
    let mut first_underflow = None;
    for (idx, delta) in line_deltas.iter().enumerate() {
        bal += delta;
        cumulative.push(bal);
        if bal < 0 && first_underflow.is_none() {
            first_underflow = Some(idx + 1);
        }
    }

    let max_line = line_deltas.len().max(1);
    let start = focus_line.saturating_sub(2).max(1);
    let end = (focus_line + 2).min(max_line);
    for line in start..=end {
        let idx = line - 1;
        let src = source_lines.get(idx).copied().unwrap_or("");
        out.push(
            format!(
                "parse.line_balance[{}]: delta={:+} cumulative={:+} | {}",
                line,
                line_deltas[idx],
                cumulative[idx],
                src.trim_end()
            )
        );
    }
    if let Some(line) = first_underflow {
        out.push(format!("parse.first_underflow_line: {}", line));
    }
}

fn delimiter_report_unexpected_closer(
    source_lines: &[&str],
    line_deltas: &[i32],
    stack: &[OpenDelimiter],
    found: char,
    line: usize,
    col: usize
) -> String {
    let mut out = Vec::new();
    out.push("parse.delimiter_error: unexpected_closer".to_string());
    out.push(format!("parse.found: '{}' at {}:{}", found, line, col));
    if let Some(top) = stack.last().copied() {
        out.push(
            format!(
                "parse.expected: '{}' to close '{}' opened at {}:{}",
                top.close,
                top.open,
                top.line,
                top.col
            )
        );
        out.push(
            format!(
                "parse.fix_hint[0]: Replace '{}' with '{}' at {}:{}.",
                found,
                top.close,
                line,
                col
            )
        );
        out.push(
            format!(
                "parse.fix_hint[1]: Or insert '{}' before {}:{} and keep '{}'.",
                top.close,
                line,
                col,
                found
            )
        );
    } else {
        out.push("parse.expected: <no opener in scope>".to_string());
        out.push(format!("parse.fix_hint[0]: Remove '{}' at {}:{}.", found, line, col));
        out.push(format!("parse.fix_hint[1]: Or add matching opener before {}:{}.", line, col));
    }
    push_open_stack_lines(&mut out, stack);
    push_balance_window_lines(&mut out, source_lines, line_deltas, line);
    out.join("\n")
}

fn delimiter_report_unclosed_opener(
    source_lines: &[&str],
    line_deltas: &[i32],
    stack: &[OpenDelimiter],
    opener: OpenDelimiter
) -> String {
    let mut out = Vec::new();
    out.push("parse.delimiter_error: unclosed_opener".to_string());
    out.push(format!("parse.unclosed: '{}' opened at {}:{}", opener.open, opener.line, opener.col));
    out.push(format!("parse.expected_before_eof: '{}'", opener.close));
    out.push(format!("parse.fix_hint[0]: Add '{}' before end of file.", opener.close));
    push_open_stack_lines(&mut out, stack);
    push_balance_window_lines(&mut out, source_lines, line_deltas, opener.line);
    out.join("\n")
}

fn delimiter_report_unclosed_literal(
    source_lines: &[&str],
    line_deltas: &[i32],
    kind: &str,
    line: usize,
    col: usize
) -> String {
    let mut out = Vec::new();
    out.push(format!("parse.delimiter_error: unclosed_{}", kind));
    out.push(format!("parse.unclosed_{}: opened at {}:{}", kind, line, col));
    let closer = if kind == "string_literal" { "\"" } else { "'" };
    out.push(format!("parse.fix_hint[0]: Add closing {}.", closer));
    push_balance_window_lines(&mut out, source_lines, line_deltas, line);
    out.join("\n")
}

fn delimiter_debug_report(source: &str, mode: DelimiterMode) -> Option<String> {
    let source_lines: Vec<&str> = source.lines().collect();
    let mut line_deltas = vec![0i32; 1];

    let mut stack: Vec<OpenDelimiter> = Vec::new();
    let mut line = 1usize;
    let mut col = 1usize;

    let mut in_comment = false;
    let mut in_string_start = None::<(usize, usize)>;
    let mut in_char_start = None::<(usize, usize)>;

    for ch in source.chars() {
        ensure_line_delta(&mut line_deltas, line);

        if in_comment {
            if ch == '\n' {
                in_comment = false;
                line += 1;
                col = 1;
                ensure_line_delta(&mut line_deltas, line);
            } else {
                col += 1;
            }
            continue;
        }

        if in_string_start.is_some() {
            if ch == '"' {
                in_string_start = None;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
                ensure_line_delta(&mut line_deltas, line);
            } else {
                col += 1;
            }
            continue;
        }

        if in_char_start.is_some() {
            if ch == '\'' {
                in_char_start = None;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
                ensure_line_delta(&mut line_deltas, line);
            } else {
                col += 1;
            }
            continue;
        }

        match ch {
            ';' => {
                in_comment = true;
                col += 1;
            }
            '"' => {
                in_string_start = Some((line, col));
                col += 1;
            }
            '\'' => {
                in_char_start = Some((line, col));
                col += 1;
            }
            '\n' => {
                line += 1;
                col = 1;
                ensure_line_delta(&mut line_deltas, line);
            }
            _ => {
                if let Some(close) = delimiter_close_for_open(ch, mode) {
                    line_deltas[line - 1] += 1;
                    stack.push(OpenDelimiter { open: ch, close, line, col });
                    col += 1;
                    continue;
                }

                if is_delimiter_close(ch, mode) {
                    line_deltas[line - 1] -= 1;
                    let Some(top) = stack.last().copied() else {
                        return Some(
                            delimiter_report_unexpected_closer(
                                &source_lines,
                                &line_deltas,
                                &stack,
                                ch,
                                line,
                                col
                            )
                        );
                    };
                    if ch != top.close {
                        return Some(
                            delimiter_report_unexpected_closer(
                                &source_lines,
                                &line_deltas,
                                &stack,
                                ch,
                                line,
                                col
                            )
                        );
                    }
                    stack.pop();
                    col += 1;
                    continue;
                }

                col += 1;
            }
        }
    }

    if let Some((sline, scol)) = in_string_start {
        return Some(
            delimiter_report_unclosed_literal(
                &source_lines,
                &line_deltas,
                "string_literal",
                sline,
                scol
            )
        );
    }
    if let Some((sline, scol)) = in_char_start {
        return Some(
            delimiter_report_unclosed_literal(
                &source_lines,
                &line_deltas,
                "char_literal",
                sline,
                scol
            )
        );
    }

    if let Some(opener) = stack.last().copied() {
        return Some(delimiter_report_unclosed_opener(&source_lines, &line_deltas, &stack, opener));
    }

    None
}

fn tokenize(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            ';' => {
                while let Some(nc) = chars.peek() {
                    if *nc == '\n' {
                        break;
                    }
                    chars.next();
                }
            }
            '(' | ')' => {
                flush(&mut buf, &mut out);
                out.push(ch.to_string());
            }
            c if c.is_whitespace() => {
                flush(&mut buf, &mut out);
            }
            _ => {
                buf.push(ch);
            }
        }
    }
    flush(&mut buf, &mut out);
    out
}

fn parse_expr(tokens: &[String], i: &mut usize) -> Result<Expression, String> {
    if *i >= tokens.len() {
        return Err("Unexpected end of input".into());
    }
    let tok = &tokens[*i];

    if tok == "(" {
        *i += 1;
        let mut exprs = Vec::new();
        while *i < tokens.len() && tokens[*i] != ")" {
            exprs.push(parse_expr(tokens, i)?);
        }
        if *i >= tokens.len() {
            return Err("Unclosed '('".into());
        }
        *i += 1;
        Ok(Expression::Apply(exprs))
    } else if tok == ")" {
        Err("Unexpected ')'".into())
    } else {
        *i += 1;
        if is_integer(tok) {
            let n: i32 = tok.parse().map_err(|e| format!("Bad integer '{}': {}", tok, e))?;
            Ok(Expression::Int(n))
        } else if is_float(tok) {
            let n: f32 = tok.parse().map_err(|e| format!("Bad dec '{}': {}", tok, e))?;
            Ok(Expression::Dec(n))
        } else {
            Ok(Expression::Word(tok.clone()))
        }
    }
}

#[derive(Debug, Clone)]
struct MacroClause {
    params: Vec<String>,
    rest_param: Option<String>,
    body: Expression,
}

#[derive(Debug, Clone)]
struct MacroDef {
    clauses: Vec<MacroClause>,
}

fn parse_macro_param_list(
    params_slice: &[Expression],
    macro_name: &str
) -> Result<(Vec<String>, Option<String>), String> {
    let mut params = Vec::new();
    let mut rest_param = None;
    let mut idx = 0usize;
    while idx < params_slice.len() {
        if let Expression::Word(dot) = &params_slice[idx] {
            if dot == "." {
                if idx + 1 >= params_slice.len() {
                    return Err(
                        format!("letmacro '{}' has '.' without a trailing rest parameter", macro_name)
                    );
                }
                let Expression::Word(rest_name) = &params_slice[idx + 1] else {
                    return Err(
                        format!("letmacro '{}' rest parameter must be a simple word", macro_name)
                    );
                };
                if idx + 2 != params_slice.len() {
                    return Err(
                        format!("letmacro '{}' rest parameter must be the last parameter", macro_name)
                    );
                }
                rest_param = Some(rest_name.clone());
                break;
            }
        }
        let param = &params_slice[idx];
        let Expression::Word(name) = param else {
            return Err(
                format!("letmacro '{}' only supports simple word parameters for now", macro_name)
            );
        };
        params.push(name.clone());
        idx += 1;
    }
    Ok((params, rest_param))
}

fn parse_macro_lambda(expr: &Expression, macro_name: &str) -> Result<MacroClause, String> {
    let Expression::Apply(items) = expr else {
        return Err(format!("letmacro '{}' must be bound to a lambda", macro_name));
    };
    if items.len() < 2 {
        return Err(format!("letmacro '{}' must be bound to a lambda", macro_name));
    }
    let Some(Expression::Word(head)) = items.first() else {
        return Err(format!("letmacro '{}' must be bound to a lambda", macro_name));
    };
    if head != "lambda" {
        return Err(format!("letmacro '{}' must be bound to a lambda", macro_name));
    }
    if items.len() < 2 {
        return Err(format!("letmacro '{}' lambda must have a body", macro_name));
    }
    let params_slice = &items[1..items.len() - 1];
    let (params, rest_param) = parse_macro_param_list(params_slice, macro_name)?;
    let body = items
        .last()
        .cloned()
        .ok_or_else(|| { format!("letmacro '{}' lambda must have a body", macro_name) })?;
    Ok(MacroClause { params, rest_param, body })
}

fn parse_macro_clause(expr: &Expression, macro_name: &str) -> Result<MacroClause, String> {
    let Expression::Apply(items) = expr else {
        return Err(format!("letmacro '{}' clause must look like ((params...) body)", macro_name));
    };
    if items.len() != 2 {
        return Err(
            format!("letmacro '{}' clause must have exactly a parameter list and body", macro_name)
        );
    }
    let params_expr = &items[0];
    let Expression::Apply(param_items) = params_expr else {
        return Err(
            format!("letmacro '{}' clause parameter list must be parenthesized", macro_name)
        );
    };
    let (params, rest_param) = parse_macro_param_list(param_items, macro_name)?;
    Ok(MacroClause {
        params,
        rest_param,
        body: items[1].clone(),
    })
}

fn parse_macro_definition(exprs: &[Expression], macro_name: &str) -> Result<MacroDef, String> {
    if exprs.is_empty() {
        return Err(format!("letmacro '{}' requires a body", macro_name));
    }
    if exprs.len() == 1 {
        return Ok(MacroDef {
            clauses: vec![parse_macro_lambda(&exprs[0], macro_name)?],
        });
    }
    let mut clauses = Vec::new();
    for expr in exprs {
        clauses.push(parse_macro_clause(expr, macro_name)?);
    }
    Ok(MacroDef { clauses })
}

fn split_macro_definitions(
    exprs: Vec<Expression>,
    macros: &mut HashMap<String, MacroDef>
) -> Result<Vec<Expression>, String> {
    let mut out = Vec::new();
    for expr in exprs {
        if let Expression::Apply(items) = &expr {
            if items.len() >= 3 {
                if
                    let (Some(Expression::Word(kw)), Some(Expression::Word(name))) = (
                        items.first(),
                        items.get(1),
                    )
                {
                    if kw == "letmacro" {
                        let macro_def = parse_macro_definition(&items[2..], name)?;
                        macros.insert(name.clone(), macro_def);
                        continue;
                    }
                }
            }
            if let [Expression::Word(kw), Expression::Word(name), _rhs] = &items[..] {
                if kw == "letmacro" {
                    let macro_def = parse_macro_definition(&items[2..], name)?;
                    macros.insert(name.clone(), macro_def);
                    continue;
                }
            }
        }
        out.push(expr);
    }
    Ok(out)
}

fn flatten_top_level_dos(exprs: Vec<Expression>) -> Vec<Expression> {
    let mut out = Vec::new();
    for expr in exprs {
        match expr {
            Expression::Apply(items) if
                !items.is_empty() &&
                matches!(items.first(), Some(Expression::Word(w)) if w == "do")
            => {
                out.extend(flatten_top_level_dos(items.into_iter().skip(1).collect()));
            }
            other => out.push(other),
        }
    }
    out
}

fn eval_macro_expr(
    expr: &Expression,
    bindings: &HashMap<String, Expression>,
    gensym_counter: &mut usize
) -> Result<Expression, String> {
    let mut local_bindings = bindings.clone();
    eval_macro_expr_in_env(expr, &mut local_bindings, gensym_counter)
}

fn eval_macro_expr_in_env(
    expr: &Expression,
    bindings: &mut HashMap<String, Expression>,
    gensym_counter: &mut usize
) -> Result<Expression, String> {
    match expr {
        Expression::Word(w) =>
            Ok(
                bindings
                    .get(w)
                    .cloned()
                    .unwrap_or_else(|| Expression::Word(w.clone()))
            ),
        Expression::Int(n) => Ok(Expression::Int(*n)),
        Expression::Dec(n) => Ok(Expression::Dec(*n)),
        Expression::Apply(items) => {
            if items.is_empty() {
                return Ok(Expression::Apply(Vec::new()));
            }
            match items.first() {
                Some(Expression::Word(head)) if head == "do" => {
                    if items.len() < 2 {
                        return Err("(do ...) requires at least one expression".to_string());
                    }
                    let mut last = None;
                    for item in items.iter().skip(1) {
                        last = Some(eval_macro_expr_in_env(item, bindings, gensym_counter)?);
                    }
                    Ok(last.expect("do body checked to be non-empty"))
                }
                Some(Expression::Word(head)) if head == "let" => {
                    if items.len() != 3 {
                        return Err("(let name value) expects exactly a name and value".to_string());
                    }
                    let Expression::Word(name) = &items[1] else {
                        return Err(
                            "compile-time let only supports simple word bindings for now".to_string()
                        );
                    };
                    let value = eval_macro_expr_in_env(&items[2], bindings, gensym_counter)?;
                    bindings.insert(name.clone(), value.clone());
                    Ok(value)
                }
                Some(Expression::Word(head)) if head == "quote" => {
                    if items.len() != 2 {
                        return Err("(quote ...) expects exactly one expression".to_string());
                    }
                    Ok(items[1].clone())
                }
                Some(Expression::Word(head)) if head == "qq" => {
                    if items.len() != 2 {
                        return Err("(qq ...) expects exactly one expression".to_string());
                    }
                    quasiquote_macro_expr(&items[1], bindings, gensym_counter)
                }
                Some(Expression::Word(head)) if head == "gensym" => {
                    if items.len() != 1 {
                        return Err("(gensym) does not take arguments yet".to_string());
                    }
                    let name = format!("__macro_{}", *gensym_counter);
                    *gensym_counter += 1;
                    Ok(Expression::Word(name))
                }
                _ =>
                    Ok(
                        Expression::Apply(
                            items
                                .iter()
                                .map(|it| eval_macro_expr_in_env(it, bindings, gensym_counter))
                                .collect::<Result<Vec<_>, _>>()?
                        )
                    ),
            }
        }
    }
}

fn quasiquote_macro_expr(
    expr: &Expression,
    bindings: &HashMap<String, Expression>,
    gensym_counter: &mut usize
) -> Result<Expression, String> {
    match expr {
        Expression::Apply(items) => {
            if matches!(items.first(), Some(Expression::Word(w)) if w == "uq") {
                if items.len() != 2 {
                    return Err("(uq ...) expects exactly one expression".to_string());
                }
                return eval_macro_expr(&items[1], bindings, gensym_counter);
            }
            let mut out = Vec::new();
            for it in items {
                if let Expression::Apply(splice_items) = it {
                    if matches!(splice_items.first(), Some(Expression::Word(w)) if w == "uqs") {
                        if splice_items.len() != 2 {
                            return Err("(uqs ...) expects exactly one expression".to_string());
                        }
                        let spliced = eval_macro_expr(&splice_items[1], bindings, gensym_counter)?;
                        match spliced {
                            Expression::Apply(parts) => {
                                out.extend(parts);
                                continue;
                            }
                            other => {
                                return Err(
                                    format!(
                                        "(uqs ...) expected a syntax list to splice, got {}",
                                        other.to_lisp()
                                    )
                                );
                            }
                        }
                    }
                }
                out.push(quasiquote_macro_expr(it, bindings, gensym_counter)?);
            }
            Ok(Expression::Apply(out))
        }
        other => Ok(other.clone()),
    }
}

fn expand_macro_call(
    macro_name: &str,
    macro_def: &MacroDef,
    args: &[Expression],
    gensym_counter: &mut usize
) -> Result<Expression, String> {
    let call_expr = Expression::Apply(
        std::iter
            ::once(Expression::Word(macro_name.to_string()))
            .chain(args.iter().cloned())
            .collect()
    );
    let selected_clause = macro_def.clauses
        .iter()
        .find(|clause| {
            if clause.rest_param.is_some() {
                args.len() >= clause.params.len()
            } else {
                args.len() == clause.params.len()
            }
        })
        .ok_or_else(|| {
            let mut expected = macro_def.clauses
                .iter()
                .map(|clause| {
                    if clause.rest_param.is_some() {
                        format!("{}+", clause.params.len())
                    } else {
                        clause.params.len().to_string()
                    }
                })
                .collect::<Vec<_>>();
            expected.sort();
            expected.dedup();
            format!(
                "Macro '{}' expected one of [{}] args, got {} in call {}",
                macro_name,
                expected.join(", "),
                args.len(),
                call_expr.to_lisp()
            )
        })?;
    let mut bindings = selected_clause.params
        .iter()
        .cloned()
        .zip(args.iter().cloned())
        .collect::<HashMap<_, _>>();
    if let Some(rest_name) = &selected_clause.rest_param {
        bindings.insert(
            rest_name.clone(),
            Expression::Apply(args[selected_clause.params.len()..].to_vec())
        );
    }
    eval_macro_expr(&selected_clause.body, &bindings, gensym_counter).map_err(|e|
        format!("Macro '{}' expansion failed for call {}: {}", macro_name, call_expr.to_lisp(), e)
    )
}

fn expression_to_string_literal_expr(expr: &Expression) -> Expression {
    let rendered = expr.to_lisp();
    let mut items = Vec::with_capacity(rendered.chars().count() + 1);
    items.push(Expression::Word("string".to_string()));
    for ch in rendered.chars() {
        items.push(Expression::Int(ch as u32 as i32));
    }
    Expression::Apply(items)
}

fn macroexpand_once_expr(
    expr: &Expression,
    macros: &HashMap<String, MacroDef>,
    gensym_counter: &mut usize
) -> Result<Expression, String> {
    let Expression::Apply(items) = expr else {
        return Ok(expr.clone());
    };
    let Some(Expression::Word(head)) = items.first() else {
        return Ok(expr.clone());
    };
    let Some(macro_def) = macros.get(head) else {
        return Ok(expr.clone());
    };
    expand_macro_call(head, macro_def, &items[1..], gensym_counter)
}

fn expand_macros_expr(
    expr: &Expression,
    macros: &HashMap<String, MacroDef>,
    gensym_counter: &mut usize,
    depth: usize
) -> Result<Expression, String> {
    if depth > MAX_MACRO_EXPANSION_DEPTH {
        return Err("Macro expansion exceeded maximum depth".to_string());
    }
    match expr {
        Expression::Apply(items) if !items.is_empty() => {
            if let Some(Expression::Word(head)) = items.first() {
                match head.as_str() {
                    "quote" | "qq" | "uq" | "uqs" | "gensym" => {
                        return Err(
                            format!("Compile-time form '{}' can only appear inside letmacro bodies", head)
                        );
                    }
                    "macroexpand-1" => {
                        if items.len() != 2 {
                            return Err(
                                "(macroexpand-1 ...) expects exactly one expression".to_string()
                            );
                        }
                        let expanded = macroexpand_once_expr(&items[1], macros, gensym_counter)?;
                        return Ok(expression_to_string_literal_expr(&expanded));
                    }
                    "macroexpand" => {
                        if items.len() != 2 {
                            return Err(
                                "(macroexpand ...) expects exactly one expression".to_string()
                            );
                        }
                        let expanded = expand_macros_expr(
                            &items[1],
                            macros,
                            gensym_counter,
                            depth + 1
                        )?;
                        return Ok(expression_to_string_literal_expr(&expanded));
                    }
                    _ => {}
                }
                if let Some(macro_def) = macros.get(head) {
                    let expanded = expand_macro_call(head, macro_def, &items[1..], gensym_counter)?;
                    return expand_macros_expr(&expanded, macros, gensym_counter, depth + 1);
                }
            }
            Ok(
                Expression::Apply(
                    items
                        .iter()
                        .map(|item| expand_macros_expr(item, macros, gensym_counter, depth))
                        .collect::<Result<Vec<_>, _>>()?
                )
            )
        }
        _ => Ok(expr.clone()),
    }
}

fn expand_macros_in_program(
    exprs: Vec<Expression>,
    macros: &HashMap<String, MacroDef>
) -> Result<Vec<Expression>, String> {
    let mut gensym_counter = 0usize;
    exprs
        .iter()
        .map(|expr| expand_macros_expr(expr, macros, &mut gensym_counter, 0))
        .collect()
}

fn prepare_program_with_macros(
    program_exprs: Vec<Expression>,
    std_exprs: Vec<Expression>
) -> Result<(Vec<Expression>, Vec<Expression>), String> {
    let mut macros = HashMap::new();
    let runtime_std = split_macro_definitions(std_exprs, &mut macros)?;
    let runtime_program = split_macro_definitions(
        flatten_top_level_dos(program_exprs),
        &mut macros
    )?;
    let expanded_std = expand_macros_in_program(runtime_std, &macros)?;
    let expanded_program = expand_macros_in_program(runtime_program, &macros)?;
    Ok((expanded_program, expanded_std))
}

pub fn parse(src: &str) -> Result<Vec<Expression>, String> {
    if let Some(report) = delimiter_debug_report(src, DelimiterMode::ParenOnly) {
        return Err(report);
    }
    let tokens = tokenize(src);
    let mut i = 0;
    let mut exprs = Vec::new();

    while i < tokens.len() {
        match parse_expr(&tokens, &mut i) {
            Ok(expr) => exprs.push(expr),
            Err(e) => {
                return Err(format!("Error parsing expression at token index {}: {}", i, e));
            }
        }
    }
    Ok(exprs)
}
fn preprocess(source: &str) -> Result<String, String> {
    if let Some(report) = delimiter_debug_report(source, DelimiterMode::SourceDelimiters) {
        return Err(report);
    }
    let mut out = String::new();
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            ';' => {
                while let Some(nc) = chars.peek() {
                    if *nc == '\n' {
                        break;
                    }
                    chars.next();
                }
            }
            '[' => {
                out.push('(');
                out.push_str("vector ");
            }
            ']' => out.push(')'),
            '{' => {
                out.push('(');
                out.push_str("tuple ");
            }
            '}' => out.push(')'),
            '"' => {
                let mut s = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '"' {
                        break;
                    } else {
                        s.push(next);
                    }
                }
                out.push_str("(string ");
                for (i, c) in s.chars().enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    out.push_str(&(c as u32).to_string());
                }
                out.push(')');
            }

            '\'' => {
                let mut s = String::new();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '\'' {
                        break;
                    } else {
                        s.push(next);
                    }
                }
                if s.len() != 1 {
                    return Err(format!("Char should be of length 1"));
                }
                out.push_str("(char ");
                for c in s.chars() {
                    out.push_str(&(c as u32).to_string());
                    break;
                }
                out.push(')');
            }

            _ => out.push(ch),
        }
    }

    Ok(out)
}
fn next_destructure_temp(prefix: &str, arg_index: usize, binding_counter: &mut usize) -> String {
    let id = *binding_counter;
    *binding_counter += 1;
    format!("_{}_{}_{}", prefix, arg_index, id)
}

fn desugar_with_counter(
    expr: Expression,
    binding_counter: &mut usize
) -> Result<Expression, String> {
    match expr {
        Expression::Apply(exprs) if !exprs.is_empty() => {
            let mut desugared_exprs = Vec::new();
            for expr in exprs {
                match desugar_with_counter(expr, binding_counter) {
                    Ok(expr) => desugared_exprs.push(expr),
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            let exprs = desugared_exprs;
            if let Expression::Word(ref name) = exprs[0] {
                match name.as_str() {
                    "<|" => Ok(pipe_data_first_curry_transform(exprs)),
                    "|>" => Ok(pipe_curry_transform(exprs)),
                    "if" => Ok(if_transform(exprs)),
                    "-" => Ok(minus_transform(exprs)),
                    "-." => Ok(minusf_transform(exprs)),
                    "+" => Ok(plus_transform(exprs)),
                    "+." => Ok(plusf_transform(exprs)),
                    "*" => Ok(mult_transform(exprs)),
                    "*." => Ok(multf_transform(exprs)),
                    "/" => Ok(div_transform(exprs)),
                    "/." => Ok(divf_transform(exprs)),
                    "and" => Ok(and_transform(exprs)),
                    "or" => Ok(or_transform(exprs)),
                    "get" => Ok(accessor_transform(exprs)?),
                    "cdr" => Ok(cdr_transform(exprs)?),
                    "set!" => Ok(setter_transform(exprs)?),
                    "&mut" | "variable" => Ok(variable_transform(exprs)),
                    "integer" => Ok(integer_transform(exprs)),
                    "fixed" => Ok(float_transform(exprs)),
                    "boolean" => boolean_transform(exprs),
                    "while" => Ok(loop_while_transform(exprs)?),
                    "lambda" => lambda_destructure_transform(exprs, binding_counter),
                    "cons" => Ok(cons_transform(exprs)),
                    "apply" => Ok(apply_transform(exprs)?),
                    "comp" => Ok(combinator_transform_rev(exprs)?),
                    "do" => Ok(transform_do(exprs, binding_counter)?),
                    _ => Ok(Expression::Apply(exprs)),
                }
            } else {
                Ok(Expression::Apply(exprs))
            }
        }
        other => Ok(other),
    }
}

fn destructure_pattern(
    pattern: &Expression,
    value_expr: Expression,
    arg_index: usize,
    binding_counter: &mut usize
) -> Result<(Vec<Expression>, Expression), String> {
    match pattern {
        Expression::Word(name) => {
            if name == "_" {
                // skip
                Ok((vec![], value_expr))
            } else {
                Ok((
                    vec![
                        Expression::Apply(
                            vec![
                                Expression::Word("let".to_string()),
                                Expression::Word(name.clone()),
                                value_expr.clone()
                            ]
                        )
                    ],
                    Expression::Word(name.clone()),
                ))
            }
        }
        // Tuple or vector destructuring
        Expression::Apply(tuple_exprs) => {
            if tuple_exprs.is_empty() {
                return Err("Empty pattern not allowed".to_string());
            }

            if let [Expression::Word(ref vector_kw), ref elements @ ..] = &tuple_exprs[..] {
                if vector_kw == "vector" {
                    // Recursively calling so value_expr should be the vector element
                    let temp_var = next_destructure_temp("temp_vec", arg_index, binding_counter);

                    let mut bindings = vec![];
                    bindings.push(
                        Expression::Apply(
                            vec![
                                Expression::Word("let".to_string()),
                                Expression::Word(temp_var.clone()),
                                value_expr
                            ]
                        )
                    );

                    let vector_bindings = destructure_vector_pattern(
                        pattern,
                        temp_var.clone(),
                        arg_index,
                        binding_counter
                    )?;
                    bindings.extend(vector_bindings);

                    Ok((bindings, Expression::Word(temp_var)))
                } else if vector_kw == "tuple" {
                    // (tuple a b)
                    if elements.len() != 2 {
                        return Err(
                            format!(
                                "Tuple pattern must have exactly 2 elements, got {}",
                                elements.len()
                            )
                        );
                    }

                    let mut bindings = vec![];
                    let temp_var = next_destructure_temp("temp_tuple", arg_index, binding_counter);

                    bindings.push(
                        Expression::Apply(
                            vec![
                                Expression::Word("let".to_string()),
                                Expression::Word(temp_var.clone()),
                                value_expr
                            ]
                        )
                    );

                    let (fst_bindings, _fst_expr) = destructure_pattern(
                        &elements[0],
                        Expression::Apply(
                            vec![
                                Expression::Word("fst".to_string()),
                                Expression::Word(temp_var.clone())
                            ]
                        ),
                        arg_index,
                        binding_counter
                    )?;
                    bindings.extend(fst_bindings);

                    let (snd_bindings, _) = destructure_pattern(
                        &elements[1],
                        Expression::Apply(
                            vec![
                                Expression::Word("snd".to_string()),
                                Expression::Word(temp_var.clone())
                            ]
                        ),
                        arg_index,
                        binding_counter
                    )?;
                    bindings.extend(snd_bindings);

                    Ok((bindings, Expression::Word(temp_var)))
                } else {
                    // Check if second element is a tuple pattern
                    if elements.len() >= 1 {
                        if let Expression::Apply(inner_exprs) = &elements[0] {
                            if let [Expression::Word(ref inner_kw), ..] = &inner_exprs[..] {
                                if inner_kw == "tuple" {
                                    let mut bindings = vec![];
                                    let temp_var = next_destructure_temp(
                                        "temp_tuple",
                                        arg_index,
                                        binding_counter
                                    );

                                    bindings.push(
                                        Expression::Apply(
                                            vec![
                                                Expression::Word("let".to_string()),
                                                Expression::Word(temp_var.clone()),
                                                value_expr.clone()
                                            ]
                                        )
                                    );

                                    // Get the inner tuple from snd
                                    let inner_tuple_var = next_destructure_temp(
                                        "temp_tuple",
                                        arg_index,
                                        binding_counter
                                    );
                                    bindings.push(
                                        Expression::Apply(
                                            vec![
                                                Expression::Word("let".to_string()),
                                                Expression::Word(inner_tuple_var.clone()),
                                                Expression::Apply(
                                                    vec![
                                                        Expression::Word("snd".to_string()),
                                                        Expression::Word(temp_var.clone())
                                                    ]
                                                )
                                            ]
                                        )
                                    );

                                    if inner_exprs.len() >= 3 {
                                        let (fst_bindings, _) = destructure_pattern(
                                            &inner_exprs[1],
                                            Expression::Apply(
                                                vec![
                                                    Expression::Word("fst".to_string()),
                                                    Expression::Word(inner_tuple_var.clone())
                                                ]
                                            ),
                                            arg_index,
                                            binding_counter
                                        )?;
                                        bindings.extend(fst_bindings);

                                        let (snd_bindings, _) = destructure_pattern(
                                            &inner_exprs[2],
                                            Expression::Apply(
                                                vec![
                                                    Expression::Word("snd".to_string()),
                                                    Expression::Word(inner_tuple_var.clone())
                                                ]
                                            ),
                                            arg_index,
                                            binding_counter
                                        )?;
                                        bindings.extend(snd_bindings);
                                    }

                                    Ok((bindings, Expression::Word(temp_var)))
                                } else {
                                    Ok((vec![], value_expr))
                                }
                            } else {
                                Ok((vec![], value_expr))
                            }
                        } else if elements.len() == 2 {
                            let mut bindings = vec![];
                            let temp_var = next_destructure_temp(
                                "temp_tuple",
                                arg_index,
                                binding_counter
                            );

                            bindings.push(
                                Expression::Apply(
                                    vec![
                                        Expression::Word("let".to_string()),
                                        Expression::Word(temp_var.clone()),
                                        value_expr
                                    ]
                                )
                            );

                            let (fst_bindings, _) = destructure_pattern(
                                &elements[0],
                                Expression::Apply(
                                    vec![
                                        Expression::Word("fst".to_string()),
                                        Expression::Apply(
                                            vec![
                                                Expression::Word("snd".to_string()),
                                                Expression::Word(temp_var.clone())
                                            ]
                                        )
                                    ]
                                ),
                                arg_index,
                                binding_counter
                            )?;
                            bindings.extend(fst_bindings);

                            let (snd_bindings, _) = destructure_pattern(
                                &elements[1],
                                Expression::Apply(
                                    vec![
                                        Expression::Word("snd".to_string()),
                                        Expression::Apply(
                                            vec![
                                                Expression::Word("snd".to_string()),
                                                Expression::Word(temp_var.clone())
                                            ]
                                        )
                                    ]
                                ),
                                arg_index,
                                binding_counter
                            )?;
                            bindings.extend(snd_bindings);

                            Ok((bindings, Expression::Word(temp_var)))
                        } else {
                            Ok((vec![], value_expr))
                        }
                    } else {
                        Ok((vec![], value_expr))
                    }
                }
            } else {
                Ok((vec![], value_expr))
            }
        }
        _ => Ok((vec![], value_expr)),
    }
}

fn destructure_vector_pattern(
    pattern: &Expression,
    vector_var: String,
    arg_index: usize,
    binding_counter: &mut usize
) -> Result<Vec<Expression>, String> {
    match pattern {
        Expression::Apply(vector_exprs) => {
            if let [Expression::Word(ref vector_kw), ref elements @ ..] = &vector_exprs[..] {
                if vector_kw == "vector" {
                    let mut bindings = vec![];
                    let mut element_index = 0;
                    let mut rest_name = None;
                    let mut elements_to_process = elements;

                    if
                        let Some(dot_idx) = elements
                            .iter()
                            .position(|elem| matches!(elem, Expression::Word(name) if name == "."))
                    {
                        if dot_idx + 2 != elements.len() {
                            return Err(
                                "Vector pattern rest must look like '[... . rest]' with '.' before the final binding".to_string()
                            );
                        }
                        let Expression::Word(name) = &elements[dot_idx + 1] else {
                            return Err(
                                "Vector pattern rest binding must be a simple word".to_string()
                            );
                        };
                        if name == "." {
                            return Err("Vector pattern rest binding cannot be '.'".to_string());
                        }
                        rest_name = Some(name.clone());
                        elements_to_process = &elements[..dot_idx];
                    }

                    for elem in elements_to_process {
                        match elem {
                            Expression::Word(name) => {
                                if name == "_" {
                                    // skip
                                    element_index += 1;
                                } else {
                                    let (elem_bindings, _) = destructure_pattern(
                                        elem,
                                        Expression::Apply(
                                            vec![
                                                Expression::Word("get".to_string()),
                                                Expression::Word(vector_var.clone()),
                                                Expression::Int(element_index as i32)
                                            ]
                                        ),
                                        arg_index,
                                        binding_counter
                                    )?;
                                    bindings.extend(elem_bindings);
                                    element_index += 1;
                                }
                            }
                            // Nested pattern (vector or tuple)
                            Expression::Apply(_) => {
                                // Recursively destructure nested pattern
                                let (elem_bindings, _) = destructure_pattern(
                                    elem,
                                    Expression::Apply(
                                        vec![
                                            Expression::Word("get".to_string()),
                                            Expression::Word(vector_var.clone()),
                                            Expression::Int(element_index as i32)
                                        ]
                                    ),
                                    arg_index,
                                    binding_counter
                                )?;
                                bindings.extend(elem_bindings);
                                element_index += 1;
                            }
                            _ => {
                                return Err(
                                    "Vector pattern element must be a word, '_', '.', or nested pattern".to_string()
                                );
                            }
                        }
                    }

                    if let Some(rest_name) = rest_name {
                        bindings.push(
                            Expression::Apply(
                                vec![
                                    Expression::Word("let".to_string()),
                                    Expression::Word(rest_name),
                                    Expression::Apply(
                                        vec![
                                            Expression::Word("cdr".to_string()),
                                            Expression::Word(vector_var.clone()),
                                            Expression::Int(element_index as i32)
                                        ]
                                    )
                                ]
                            )
                        );
                    }

                    Ok(bindings)
                } else {
                    Ok(vec![])
                }
            } else {
                Ok(vec![])
            }
        }
        _ => Ok(vec![]),
    }
}

fn lambda_destructure_transform(
    exprs: Vec<Expression>,
    binding_counter: &mut usize
) -> Result<Expression, String> {
    // Check if valid body
    if exprs.len() < 2 {
        return Err("lambda expects at least a body".to_string());
    }
    let args = &exprs[1..exprs.len() - 1];
    let body = exprs.last().unwrap().clone();

    // look for destructuring patterns in args
    let mut new_bindings = vec![];
    let mut new_args = Vec::new();
    for (j, arg) in args.iter().enumerate() {
        match arg {
            Expression::Apply(array_exprs) => {
                if let [Expression::Word(ref array_kw), ref _elements @ ..] = &array_exprs[..] {
                    match array_kw.as_str() {
                        "vector" => {
                            // Vector destructuring pattern
                            let temp_arg_name = format!(
                                "_args{}_{}",
                                j,
                                next_destructure_temp("arg", j, binding_counter)
                            );
                            let bindings = destructure_vector_pattern(
                                arg,
                                temp_arg_name.clone(),
                                j,
                                binding_counter
                            )?;
                            new_bindings.extend(bindings);
                            new_args.push(Expression::Word(temp_arg_name));
                            continue;
                        }
                        "tuple" => {
                            // Tuple destructuring pattern - use recursive destructuring
                            let temp_arg_name = format!(
                                "_args{}_{}",
                                j,
                                next_destructure_temp("arg", j, binding_counter)
                            );
                            let (bindings, _) = destructure_pattern(
                                arg,
                                Expression::Word(temp_arg_name.clone()),
                                j,
                                binding_counter
                            )?;
                            new_bindings.extend(bindings);
                            new_args.push(Expression::Word(temp_arg_name));
                            continue;
                        }
                        _ => new_args.push(arg.clone()),
                    }
                } else {
                    // It was NOT a destructuring pattern - skip
                    new_args.push(arg.clone());
                }
            }
            _ => new_args.push(arg.clone()),
        }
    }

    // wrap body with new bindings
    let new_body = if !new_bindings.is_empty() {
        let mut do_exprs = new_bindings;
        do_exprs.push(body);
        Expression::Apply(
            std::iter
                ::once(Expression::Word("do".to_string()))
                .chain(do_exprs.into_iter())
                .collect()
        )
    } else {
        body
    };

    // rebuild lambda with transformed args and body
    let mut lambda_exprs = vec![Expression::Word("lambda".to_string())];
    lambda_exprs.extend(new_args);
    lambda_exprs.push(new_body);
    Ok(Expression::Apply(lambda_exprs))
}

fn ensure_do_body_with_trailing_nil(body_expr: Expression) -> Expression {
    match body_expr {
        Expression::Apply(mut body_items) if
            matches!(body_items.first(), Some(Expression::Word(head)) if head == "do")
        => {
            body_items.push(Expression::Word("nil".to_string()));
            Expression::Apply(body_items)
        }
        other =>
            Expression::Apply(
                vec![Expression::Word("do".to_string()), other, Expression::Word("nil".to_string())]
            ),
    }
}

fn normalize_loop_while_body_from_arg(body_arg: &Expression) -> Result<Expression, String> {
    match body_arg {
        Expression::Apply(items) if !items.is_empty() => {
            if let Expression::Word(head) = &items[0] {
                if head == "lambda" {
                    if items.len() < 2 {
                        return Err(
                            format!(
                                "loop condition form lambda missing body\n{}",
                                body_arg.to_lisp()
                            )
                        );
                    }
                    let params = &items[1..items.len() - 1];
                    let param_count = params.len();
                    if param_count != 0 {
                        return Err(
                            format!(
                                "loop condition form expects lambda with 0 parameters, found {}\n{}",
                                param_count,
                                body_arg.to_lisp()
                            )
                        );
                    }
                    return Ok(Expression::Apply(vec![body_arg.clone()]));
                }
            }
            Ok(body_arg.clone())
        }
        Expression::Word(name) => Ok(Expression::Apply(vec![Expression::Word(name.clone())])),
        _ => Ok(body_arg.clone()),
    }
}

fn loop_while_transform(mut exprs: Vec<Expression>) -> Result<Expression, String> {
    exprs.remove(0);
    let len = exprs.len();
    if len != 2 {
        return Err(
            format!(
                "while expects exactly 2 arguments: condition and body expression, got {}\n{}",
                len,
                exprs
                    .into_iter()
                    .map(|e| e.to_lisp())
                    .collect::<Vec<String>>()
                    .join(" ")
            )
        );
    }

    let condition = exprs[0].clone();
    let raw_body = normalize_loop_while_body_from_arg(&exprs[1])?;
    let body_with_unit = ensure_do_body_with_trailing_nil(raw_body);
    Ok(Expression::Apply(vec![Expression::Word("while".to_string()), condition, body_with_unit]))
}

fn cdr_transform(mut exprs: Vec<Expression>) -> Result<Expression, String> {
    exprs.remove(0);
    let len = exprs.len();
    let mut iter = exprs.into_iter();
    if len == 0 {
        return Err("cdr requires at least 1 argument".to_string());
    }
    let first = iter.next().unwrap();
    if len == 1 {
        return Ok(
            Expression::Apply(vec![Expression::Word("cdr".to_string()), first, Expression::Int(1)])
        );
    }
    Ok(Expression::Apply(vec![Expression::Word("cdr".to_string()), first, iter.next().unwrap()]))
}

fn accessor_transform(mut exprs: Vec<Expression>) -> Result<Expression, String> {
    exprs.remove(0);
    let len = exprs.len();
    let mut iter = exprs.into_iter();
    if len == 0 {
        return Err("get requires at least 1 argument".to_string());
    }
    let first = iter.next().unwrap();
    if len == 1 {
        return Ok(
            Expression::Apply(vec![Expression::Word("get".to_string()), first, Expression::Int(0)])
        );
    }
    let mut acc = first;
    for e in iter {
        acc = Expression::Apply(vec![Expression::Word("get".to_string()), acc, e]);
    }
    Ok(acc)
}
fn setter_transform(mut exprs: Vec<Expression>) -> Result<Expression, String> {
    if exprs.len() == 4 {
        return Ok(Expression::Apply(exprs));
    }
    exprs.remove(0);
    let len = exprs.len();
    let last = exprs.pop().unwrap();
    let set_idx = exprs.pop().unwrap();
    let mut iter = exprs.into_iter();
    if len < 3 {
        return Err("set! requires at least 3 arguments".to_string());
    }
    let first = iter.next().unwrap();
    let mut acc = first;
    for e in iter {
        acc = Expression::Apply(vec![Expression::Word("get".to_string()), acc, e]);
    }
    Ok(Expression::Apply(vec![Expression::Word("set!".to_string()), acc, set_idx, last]))
}
fn variable_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);
    Expression::Apply(
        vec![
            Expression::Word("let".to_string()),
            exprs[0].clone(),
            Expression::Apply(vec![Expression::Word("box".to_string()), exprs[1].clone()])
        ]
    )
}
fn integer_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);
    Expression::Apply(
        vec![
            Expression::Word("let".to_string()),
            exprs[0].clone(),
            Expression::Apply(vec![Expression::Word("int".to_string()), exprs[1].clone()])
        ]
    )
}
fn float_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);
    Expression::Apply(
        vec![
            Expression::Word("let".to_string()),
            exprs[0].clone(),
            Expression::Apply(vec![Expression::Word("dec".to_string()), exprs[1].clone()])
        ]
    )
}
fn boolean_transform(mut exprs: Vec<Expression>) -> Result<Expression, String> {
    exprs.remove(0);
    match &exprs[1] {
        Expression::Word(x) => {
            if x != "true" && x != "false" {
                return Err(
                    format!("Booleans variables only be assigned to true or false but got: {}", x)
                );
            }
        }
        Expression::Apply(x) =>
            match &x[0] {
                Expression::Word(y) => {
                    if
                        y != "=" &&
                        y != ">" &&
                        y != "<" &&
                        y != "<=" &&
                        y != ">=" &&
                        y != "not" &&
                        y != "or" &&
                        y != "and"
                    {
                        return Err(
                            format!("Booleans variables only be assigned to results of boolean expressions but got: {}", y)
                        );
                    }
                }
                _ => {
                    return Err(
                        format!(
                            "Booleans variables only be assigned to true or false but got: {:?}",
                            x[0]
                        )
                    );
                }
            }
        x => {
            return Err(
                format!("Booleans variables only be assigned to true or false but got : {:?}", x)
            );
        }
    }
    Ok(
        Expression::Apply(
            vec![
                Expression::Word("let".to_string()),
                exprs[0].clone(),
                Expression::Apply(vec![Expression::Word("bool".to_string()), exprs[1].clone()])
            ]
        )
    )
}
fn minus_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Int(0),
        1 =>
            Expression::Apply(
                vec![Expression::Word("*".to_string()), exprs.remove(0), Expression::Int(-1)]
            ),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("-".to_string()), acc, next])
                })
        }
    }
}
fn minusf_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Int(0),
        1 =>
            Expression::Apply(
                vec![Expression::Word("*.".to_string()), exprs.remove(0), Expression::Dec(-1.0)]
            ),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("-.".to_string()), acc, next])
                })
        }
    }
}
fn plus_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Int(1),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("+".to_string()), acc, next])
                })
        }
    }
}

fn plusf_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Dec(1.0),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("+.".to_string()), acc, next])
                })
        }
    }
}

fn cons_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Apply(vec![Expression::Word("vector".to_string())]),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("cons".to_string()), acc, next])
                })
        }
    }
}
// fn combinator_transform(mut exprs: Vec<Expression>) -> Result<Expression, String> {
//     // Remove the "\"
//     exprs.remove(0);

//     if exprs.is_empty() {
//         return Err("(\\) requires at least one function".into());
//     }

//     // First item is the function being partially applied
//     let func: Expression = exprs.remove(0);

//     Ok(Expression::Apply(
//         vec![
//             Expression::Word(format!("std/fn/combinator/{}", exprs.len() + 1)),
//             func,
//         ]
//         .into_iter()
//         .chain(exprs)
//         .collect(),
//     ))
// }

fn normalize_apply(expr: Expression) -> Expression {
    match expr {
        Expression::Apply(items) if items.len() == 2 => {
            let f = normalize_apply(items[0].clone());
            let arg = normalize_apply(items[1].clone());

            match f {
                Expression::Apply(mut inner) => {
                    inner.push(arg);
                    Expression::Apply(inner)
                }
                other => Expression::Apply(vec![other, arg]),
            }
        }

        Expression::Apply(items) => {
            Expression::Apply(items.into_iter().map(normalize_apply).collect())
        }

        other => other,
    }
}
enum DestructuringKind {
    Tuple,
    Vector,
}
fn destructuring_kind(pattern: &Expression) -> Option<DestructuringKind> {
    match pattern {
        Expression::Apply(exprs) =>
            match exprs.as_slice() {
                [Expression::Word(kw), ..] if kw == "tuple" => { Some(DestructuringKind::Tuple) }
                [Expression::Word(kw), ..] if kw == "vector" => { Some(DestructuringKind::Vector) }
                _ => None,
            }
        _ => None,
    }
}
fn transform_let_destructuring_in_do(
    exprs: Vec<Expression>,
    binding_counter: &mut usize
) -> Result<Vec<Expression>, String> {
    let mut new_exprs = Vec::new();

    for expr in exprs {
        if let Expression::Apply(let_items) = &expr {
            if let_items.len() >= 3 {
                if let Expression::Word(kw) = &let_items[0] {
                    if kw == "let" {
                        let pattern = &let_items[1];
                        let value_expr = &let_items[2];

                        if let Some(kind) = destructuring_kind(pattern) {
                            let prefix = match kind {
                                DestructuringKind::Tuple => "tuple",
                                DestructuringKind::Vector => "vec",
                            };

                            let temp_var = next_destructure_temp(
                                &format!("let_temp_{}", prefix),
                                10000,
                                binding_counter
                            );

                            let temp_binding = Expression::Apply(
                                vec![
                                    Expression::Word(kw.clone()),
                                    Expression::Word(temp_var.clone()),
                                    value_expr.clone()
                                ]
                            );

                            // We use a high arg_index to avoid clashes with lambda's _args prefix
                            // Nested temp variables will be _temp_10000_X which won't conflict
                            let (destructured_bindings, _) = destructure_pattern(
                                pattern,
                                Expression::Word(temp_var),
                                10000, // Use high index to avoid conflicts with lambda's small indices
                                binding_counter
                            )?;

                            // Add temp binding first, then destructured bindings
                            new_exprs.push(temp_binding);
                            new_exprs.extend(destructured_bindings);
                        } else {
                            // Not a destructuring pattern, keep as is
                            new_exprs.push(expr);
                        }

                        continue;
                    }
                }
            }
        }

        // Not a let binding, keep as is
        new_exprs.push(expr);
    }

    Ok(new_exprs)
}

fn transform_do(
    mut exprs: Vec<Expression>,
    binding_counter: &mut usize
) -> Result<Expression, String> {
    exprs.remove(0);
    let exprs_with_destructured_lets = transform_let_destructuring_in_do(exprs, binding_counter)?;
    Ok(
        Expression::Apply(
            vec![Expression::Word("do".to_string())]
                .into_iter()
                .chain(exprs_with_destructured_lets)
                .collect()
        )
    )
}
fn combinator_transform_rev(mut exprs: Vec<Expression>) -> Result<Expression, String> {
    exprs.remove(0);
    if exprs.is_empty() {
        return Err("(comp) requires at least one function".into());
    }
    // generate fresh parameter
    let arg = Expression::Word("_x".to_string());
    // Build nested application: fns applied right-to-left
    let body = exprs.into_iter().fold(arg.clone(), |acc, func| Expression::Apply(vec![func, acc]));

    Ok(normalize_apply(Expression::Apply(vec![Expression::Word("lambda".to_string()), arg, body])))
}

fn apply_transform(mut exprs: Vec<Expression>) -> Result<Expression, String> {
    // Remove the "apply"
    exprs.remove(0);

    if exprs.is_empty() {
        return Err("(apply) requires at least one function".into());
    }

    let func: Expression = exprs.remove(0);
    Ok(exprs.into_iter().fold(func, |acc, arg| Expression::Apply(vec![acc, arg])))
}

fn and_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Int(1),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("and".to_string()), acc, next])
                })
        }
    }
}

fn or_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Int(1),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("or".to_string()), acc, next])
                })
        }
    }
}
fn mult_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Int(1),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("*".to_string()), acc, next])
                })
        }
    }
}
fn multf_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Dec(1.0),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("*.".to_string()), acc, next])
                })
        }
    }
}
fn div_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Int(1),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("/".to_string()), acc, next])
                })
        }
    }
}
fn divf_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);

    match exprs.len() {
        0 => Expression::Dec(1.0),
        _ => {
            let first = exprs.remove(0);
            exprs
                .into_iter()
                .fold(first, |acc, next| {
                    Expression::Apply(vec![Expression::Word("/.".to_string()), acc, next])
                })
        }
    }
}
fn if_transform(mut exprs: Vec<Expression>) -> Expression {
    exprs.remove(0);
    if exprs.len() == 0 {
        return Expression::Apply(
            vec![
                Expression::Word("if".to_string()),
                Expression::Word("nil".to_string()),
                Expression::Word("nil".to_string()),
                Expression::Word("nil".to_string())
            ]
        );
    }
    if exprs.len() == 1 {
        return Expression::Apply(
            vec![
                Expression::Word("if".to_string()),
                exprs[0].clone(),
                Expression::Word("nil".to_string()),
                Expression::Word("nil".to_string())
            ]
        );
    }
    return Expression::Apply(
        vec![Expression::Word("if".to_string()), exprs[0].clone(), exprs[1].clone(), if
            exprs.len() == 2
        {
            Expression::Word("nil".to_string())
        } else {
            exprs[2].clone()
        }]
    );
}
fn pipe_data_first_curry_transform(mut exprs: Vec<Expression>) -> Expression {
    let mut inp = exprs.remove(1); // piped value

    for stage in exprs.into_iter().skip(1) {
        match stage {
            // normal |>, data-first
            Expression::Apply(mut inner) if !inner.is_empty() => {
                let func = inner.remove(0);
                let mut new_stage = vec![func, inp];
                new_stage.extend(inner);
                inp = Expression::Apply(new_stage);
            }

            // simple function
            stage => {
                inp = Expression::Apply(vec![stage, inp]);
            }
        }
    }

    inp
}

fn pipe_curry_transform(mut exprs: Vec<Expression>) -> Expression {
    let mut inp = exprs.remove(1);

    for stage in exprs.into_iter().skip(1) {
        match stage {
            Expression::Apply(items) if !items.is_empty() => {
                let func = items[0].clone();
                let mut args: Vec<Expression> = items[1..].to_vec();
                args.push(inp);
                let mut new_list = vec![func];
                new_list.extend(args);
                inp = Expression::Apply(new_list);
            }

            stage => {
                inp = Expression::Apply(vec![stage, inp]);
            }
        }
    }

    inp
}

// fn pipe_transform(mut exprs: Vec<Expression>) -> Expression {
//     let mut inp = exprs.remove(1);

//     for stage in exprs.into_iter().skip(1) {
//         if let Expression::Apply(mut inner) = stage {
//             if inner.is_empty() {
//                 continue;
//             }
//             let func = inner.remove(0);
//             let mut new_stage = vec![func, inp];
//             new_stage.extend(inner);
//             inp = Expression::Apply(new_stage);
//         } else {
//             inp = Expression::Apply(vec![stage, inp]);
//         }
//     }

//     inp
// }

fn is_float(s: &str) -> bool {
    if s == "-" || s == "+" {
        return false;
    }

    let trimmed = if let Some(stripped) = s.strip_prefix('-') { stripped } else { s };

    if !trimmed.contains('.') {
        return false;
    }

    let mut parts = trimmed.split('.');

    let before = parts.next().unwrap_or("");
    let after = parts.next().unwrap_or("");

    if parts.next().is_some() {
        return false;
    }

    let has_digit =
        before.chars().any(|c| c.is_ascii_digit()) || after.chars().any(|c| c.is_ascii_digit());
    if !has_digit {
        return false;
    }

    for c in before.chars() {
        if !c.is_ascii_digit() {
            return false;
        }
    }

    for c in after.chars() {
        if !c.is_ascii_digit() {
            return false;
        }
    }

    true
}

fn is_integer(s: &str) -> bool {
    if s == "-" || s == "+ " {
        return false;
    }
    if !s.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    let trimmed = if let Some(stripped) = s.strip_prefix('-') { stripped } else { s };
    for c in trimmed.chars() {
        if !c.is_ascii_digit() {
            return false;
        }
    }
    true
}
#[allow(unused_macros)]
macro_rules! s {
    ($s:expr) => {
        $s.to_string()
    };
}

#[derive(Debug, Clone)]
pub enum Expression {
    Int(i32),
    Dec(f32),
    Word(String),
    Apply(Vec<Expression>),
}

impl Expression {
    // pub fn to_rust(&self) -> String {
    //     match self {
    //         Expression::Int(n) => format!("Int({})", n),
    //         Expression::Dec(n) => format!("Dec({:?})", n),
    //         Expression::Word(w) => format!("Word(s!({:?}))", w),
    //         Expression::Apply(exprs) => {
    //             let inner: Vec<String> = exprs.iter().map(|e| e.to_rust()).collect();
    //             format!("Apply(vec![{}])", inner.join(", "))
    //         }
    //     }
    // }
    pub fn to_lisp(&self) -> String {
        match self {
            Expression::Word(w) => w.clone(),
            Expression::Int(a) => a.to_string(),
            Expression::Dec(a) => format!("{:?}", a),
            Expression::Apply(items) => {
                if items.is_empty() {
                    return "()".to_string();
                }
                let parts: Vec<String> = items
                    .iter()
                    .map(|e| e.to_lisp())
                    .collect();
                format!("({})", parts.join(" "))
            }
        }
    }
}

fn wrap_top_level_do(exprs: Vec<Expression>) -> Expression {
    Expression::Apply(std::iter::once(Expression::Word("do".to_string())).chain(exprs).collect())
}

fn wrap_runtime_top_level_do(exprs: Vec<Expression>) -> Expression {
    if exprs.is_empty() {
        return Expression::Apply(
            vec![Expression::Word("do".to_string()), Expression::Word("nil".to_string())]
        );
    }
    wrap_top_level_do(exprs)
}

pub fn merge_std_and_program(program: &str, std: Vec<Expression>) -> Result<Expression, String> {
    match preprocess(&program) {
        Ok(preprocessed) =>
            match parse(&preprocessed) {
                Ok(exprs) => {
                    let (exprs, std) = prepare_program_with_macros(exprs, std)?;
                    let mut desugared = Vec::new();
                    let mut desugared_std = Vec::new();
                    let mut binding_counter = 0usize;
                    for expr in std {
                        match desugar_with_counter(expr, &mut binding_counter) {
                            Ok(expr) => desugared_std.push(expr),
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }
                    for expr in exprs {
                        match desugar_with_counter(expr, &mut binding_counter) {
                            Ok(expr) => desugared.push(expr),
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }
                    for expr in &desugared {
                        validate_reserved_words_in_binders(expr)?;
                    }
                    let mut used: HashSet<String> = HashSet::new();
                    for e in &desugared {
                        let mut scoped = HashSet::new();
                        collect_free_idents(e, &mut scoped, &mut used);
                    }
                    let mut definitions: HashSet<String> = HashSet::new();
                    for expr in &desugared {
                        if let Expression::Apply(list) = expr {
                            if
                                let [Expression::Word(kw), Expression::Word(name), _rest @ ..] =
                                    &list[..]
                            {
                                if kw == "let" || kw == "let*" || kw == "mut" {
                                    if is_reserved_word(name) {
                                        return Err(format!("Variable '{}' is forbidden", name));
                                    }
                                    definitions.insert(name.to_string());
                                }
                            }
                        }
                    }

                    let shaken_std = tree_shake(desugared_std, &used, &mut definitions);
                    let top_level = transform_let_destructuring_in_do(
                        desugared.to_vec(),
                        &mut binding_counter
                    )?;
                    let wrapped = wrap_runtime_top_level_do(
                        shaken_std.into_iter().chain(top_level).collect()
                    );
                    Ok(wrapped)
                }
                Err(e) => {
                    return Err(e);
                }
            }
        Err(e) => {
            return Err(e);
        }
    }
}

pub fn build(program: &str) -> Result<Expression, String> {
    let preprocessed = preprocess(program)?;
    let exprs = parse(&preprocessed)?;
    let (exprs, _std) = prepare_program_with_macros(exprs, Vec::new())?;

    let mut desugared = Vec::new();
    let mut binding_counter = 0usize;
    for expr in exprs {
        desugared.push(desugar_with_counter(expr, &mut binding_counter)?);
    }

    let top_level = transform_let_destructuring_in_do(desugared, &mut binding_counter)?;
    Ok(wrap_runtime_top_level_do(top_level))
}

pub fn build_library(program: &str) -> Result<Expression, String> {
    let preprocessed = preprocess(program)?;
    let exprs = parse(&preprocessed)?;
    Ok(wrap_top_level_do(flatten_top_level_dos(exprs)))
}
