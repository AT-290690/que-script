use crate::parser::Expression;

pub fn format(exprs: &[Expression]) -> String {
    exprs
        .iter()
        .map(|e| format_expr(e, 0))
        .collect::<Vec<_>>()
        .join("\n")
}
fn format_expr(expr: &Expression, indent: usize) -> String {
    match expr {
        Expression::Word(w) => w.clone(),
        Expression::Int(n) => n.to_string(),
        Expression::Float(f) => f.to_string(),
        Expression::Apply(xs) => format_apply(xs, indent),
    }
}
fn format_let(xs: &[Expression], indent: usize) -> String {
    if xs.len() != 3 {
        return format_paren(xs, indent);
    }

    let name = &xs[1];
    let value = &xs[2];

    let value_str = format_expr(value, indent + 2);

    if !value_str.contains('\n') {
        format!("(let {} {})", format_expr(name, 0), value_str)
    } else {
        format!(
            "(let {}{}\n{}{})\n",
            format_expr(name, 0),
            "",
            indent_str(indent + 2),
            value_str.replace('\n', &format!("\n{}", indent_str(indent + 2)))
        )
    }
}

fn format_do(xs: &[Expression], indent: usize) -> String {
    format!(
        "(do\n{})",
        xs[1..]
            .iter()
            .map(|e| format!("     {}", format_expr(e, indent)))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn format_if(xs: &[Expression], indent: usize) -> String {
    format!(
        "(if {}\n{})",
        format_expr(&xs[1], indent),
        xs[2..]
            .iter()
            .map(|e| format!("    {}", format_expr(e, indent)))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn format_cond(xs: &[Expression], indent: usize) -> String {
    format!(
        "(cond\n{})",
        xs[1..]
            .iter()
            .map(|e| format!("    {}", format_expr(e, indent)))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn format_lambda(xs: &[Expression], indent: usize) -> String {
    if xs.len() < 3 {
        return format_paren(xs, indent);
    }

    let args = &xs[1..xs.len() - 1];
    let body = xs.last().unwrap();

    let mut out = String::new();
    out.push_str("(lambda");

    for arg in args {
        out.push(' ');
        out.push_str(&format_expr(arg, 0));
    }

    let body_str = format_expr(body, indent + 2);

    if !body_str.contains('\n') {
        out.push(' ');
        out.push_str(&body_str);
        out.push(')');
    } else {
        out.push('\n');
        out.push_str(&indent_str(indent + 2));
        out.push_str(&body_str.replace('\n', &format!("\n{}", indent_str(indent + 2))));
        out.push('\n');
        out.push_str(&indent_str(indent));
        out.push(')');
    }

    out
}
fn format_apply(xs: &[Expression], indent: usize) -> String {
    if xs.is_empty() {
        return "()".to_string();
    }

    if is_vector(xs) {
        return format_bracketed(xs, indent, "[", "]");
    }

    if is_tuple(xs) {
        return format_bracketed(xs, indent, "{", "}");
    }

    if matches!(xs[0], Expression::Word(ref w) if w == "let") {
        return format_let(xs, indent);
    }

    if matches!(xs[0], Expression::Word(ref w) if w == "lambda") {
        return format_lambda(xs, indent);
    }

    if matches!(xs[0], Expression::Word(ref w) if w == "do") {
        return format_do(xs, indent);
    }

    if matches!(xs[0], Expression::Word(ref w) if w == "if") {
        return format_if(xs, indent);
    }

    if matches!(xs[0], Expression::Word(ref w) if w == "cond") {
        return format_cond(xs, indent);
    }

    if matches!(xs[0], Expression::Word(ref w) if w == "|>" || w == "comp") {
        return format_pipe_or_comp(xs, indent);
    }

    format_paren(xs, indent)
}
fn is_vector(xs: &[Expression]) -> bool {
    matches!(xs.first(), Some(Expression::Word(w)) if w == "vector")
}

fn is_tuple(xs: &[Expression]) -> bool {
    matches!(xs.first(), Some(Expression::Word(w)) if w == "tuple")
}
fn format_bracketed(xs: &[Expression], indent: usize, open: &str, close: &str) -> String {
    let elements = &xs[1..];

    if elements.is_empty() {
        return format!("{}{}", open, close);
    }

    if fits_on_one_line(elements) {
        let inner = elements
            .iter()
            .map(|e| format_expr(e, 0))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{}{}{}", open, inner, close)
    } else {
        let mut out = String::new();
        out.push_str(open);
        for e in elements {
            out.push('\n');
            out.push_str(&indent_str(indent + 2));
            out.push_str(&format_expr(e, indent + 2));
        }
        out.push('\n');
        out.push_str(&indent_str(indent));
        out.push_str(close);
        out
    }
}
fn format_paren(xs: &[Expression], indent: usize) -> String {
    if fits_on_one_line(xs) {
        let inner = xs
            .iter()
            .map(|e| format_expr(e, 0))
            .collect::<Vec<_>>()
            .join(" ");
        return format!("({})", inner);
    }

    let mut out = String::new();
    out.push('(');
    out.push_str(&format_expr(&xs[0], indent));

    for arg in &xs[1..] {
        out.push('\n');
        out.push_str(&indent_str(indent + 2));
        out.push_str(&format_expr(arg, indent + 2));
    }

    out.push('\n');
    out.push_str(&indent_str(indent));
    out.push(')');
    out
}

fn fits_on_one_line(xs: &[Expression]) -> bool {
    let mut len = 0;

    for e in xs {
        match e {
            Expression::Word(w) => {
                if is_string_atom(w) || is_char_atom(w) || is_bracket_atom(w) {
                    return true;
                }
                len += w.len() + 1;
            }
            Expression::Int(n) => {
                len += n.to_string().len() + 1;
            }
            Expression::Float(f) => {
                len += f.to_string().len() + 1;
            }
            Expression::Apply(inner) => {
                if inner.iter().any(|x| {
                    matches!(x,
                        Expression::Word(w) if is_bracket_atom(w)
                    )
                }) {
                    return true;
                }
                len += inner.len() * 2 + 2;
            }
        }

        if len > 60 {
            return false;
        }
    }

    true
}

fn indent_str(n: usize) -> String {
    " ".repeat(n)
}
fn is_string_atom(w: &str) -> bool {
    w.starts_with('"') && w.ends_with('"')
}

fn is_char_atom(w: &str) -> bool {
    w.starts_with('\'') && w.ends_with('\'')
}
pub fn compact_dangling_parens(input: &str) -> String {
    let lines: Vec<String> = input.lines().map(|l| l.to_string()).collect();
    let mut out: Vec<String> = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];

        if is_only_closing_parens(line) {
            let mut parens = count_parens(line);
            let mut j = i + 1;

            while j < lines.len() && is_only_closing_parens(&lines[j]) {
                parens += count_parens(&lines[j]);
                j += 1;
            }

            if let Some(prev) = out.last_mut() {
                prev.push_str(&")".repeat(parens));
            } else {
                out.push(")".repeat(parens));
            }

            i = j;
            continue;
        }

        out.push(line.clone());
        i += 1;
    }

    out.join("\n")
}

fn is_only_closing_parens(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.chars().all(|c| c == ')')
}

fn count_parens(line: &str) -> usize {
    line.chars().filter(|&c| c == ')').count()
}
fn is_bracket_atom(w: &str) -> bool {
    matches!(w, "[" | "]" | "{" | "}")
}

#[derive(Debug)]
pub enum Frozen {
    String(String),
    Char(String),
    Comment(String),
}
pub struct FreezeResult {
    pub rewritten: String,
    pub frozen: Vec<Frozen>,
}
pub fn freeze_literals(src: &str) -> FreezeResult {
    let mut out = String::new();
    let mut frozen = Vec::new();

    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '"' => {
                let start = i;
                i += 1;
                while i < chars.len() {
                    if chars[i] == '"' && chars[i - 1] != '\\' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let id = frozen.len();
                frozen.push(Frozen::String(s));
                out.push_str(&format!("⟦STR#{}⟧", id));
            }

            '\'' => {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
                let s: String = chars[start..i].iter().collect();
                let id = frozen.len();
                frozen.push(Frozen::Char(s));
                out.push_str(&format!("⟦CHR#{}⟧", id));
            }

            ';' => {
                let start = i;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                let s: String = chars[start..i].iter().collect();
                let id = frozen.len();
                frozen.push(Frozen::Comment(format!("{}\n", s)));
                out.push_str(&format!("⟦CMT#{}⟧", id));
            }

            c => {
                out.push(c);
                i += 1;
            }
        }
    }

    FreezeResult {
        rewritten: out,
        frozen,
    }
}
pub fn restore_literals(src: &str, frozen: &[Frozen]) -> String {
    let mut out = src.to_string();

    for (i, lit) in frozen.iter().enumerate() {
        let placeholder = match lit {
            Frozen::String(_) => format!("⟦STR#{}⟧", i),
            Frozen::Char(_) => format!("⟦CHR#{}⟧", i),
            Frozen::Comment(_) => format!("⟦CMT#{}⟧", i),
        };

        let value = match lit {
            Frozen::String(s) => s,
            Frozen::Char(s) => s,
            Frozen::Comment(s) => s,
        };

        out = out.replace(&placeholder, value);
    }

    out
}
pub fn format_source(src: &str) -> String {
    let frozen = freeze_literals(src);
    let exprs = crate::parser::parse(&frozen.rewritten).unwrap();
    let formatted = format(&exprs);
    let compacted = compact_dangling_parens(&formatted);
    restore_literals(&compacted, &frozen.frozen)
}
fn format_pipe_or_comp(xs: &[Expression], indent: usize) -> String {
    let head = &xs[0];
    let first = &xs[1];
    let rest = &xs[2..];

    let mut out = String::new();
    out.push('(');
    out.push_str(&format_expr(head, indent));
    out.push(' ');
    out.push_str(&format_expr(first, 0));

    for expr in rest {
        out.push('\n');
        out.push_str(&indent_str(indent + 2));
        out.push_str(&format_expr(expr, indent + 2));
    }

    out.push(')');
    out
}
