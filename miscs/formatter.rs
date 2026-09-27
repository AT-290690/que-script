#[derive(Clone, Debug, PartialEq, Eq)]
enum Form {
    Atom(String),
    Comment(String),
    Group {
        open: char,
        close: char,
        items: Vec<Form>,
    },
}

const DEFAULT_WIDTH: usize = 88;
const INDENT: usize = 2;

pub fn format_source(source: &str) -> Result<String, String> {
    let before = crate::parser::build(source)
        .map_err(|error| format!("cannot format invalid Que source: {error}"))?;
    let forms = parse_forms(source)?;
    let mut rendered = Vec::new();
    for form in &forms {
        rendered.push(render(form, 0, DEFAULT_WIDTH));
    }
    let formatted = rendered.join("\n") + "\n";
    let after = crate::parser::build(&formatted)
        .map_err(|error| format!("formatter produced invalid Que source: {error}"))?;
    if before.to_lisp() != after.to_lisp() {
        return Err("formatter changed the parsed Que program".to_string());
    }
    Ok(formatted)
}

fn parse_forms(source: &str) -> Result<Vec<Form>, String> {
    let chars = source.chars().collect::<Vec<_>>();
    let mut at = 0;
    parse_until(&chars, &mut at, None)
}

fn parse_until(
    chars: &[char],
    at: &mut usize,
    expected_close: Option<char>,
) -> Result<Vec<Form>, String> {
    let mut forms = Vec::new();
    while *at < chars.len() {
        while *at < chars.len() && chars[*at].is_whitespace() {
            *at += 1;
        }
        if *at >= chars.len() {
            break;
        }
        let ch = chars[*at];
        if Some(ch) == expected_close {
            *at += 1;
            return Ok(forms);
        }
        if matches!(ch, ')' | ']' | '}') {
            return Err(format!("unexpected '{}' while formatting", ch));
        }
        if ch == ';' {
            let start = *at;
            while *at < chars.len() && chars[*at] != '\n' {
                *at += 1;
            }
            forms.push(Form::Comment(chars[start..*at].iter().collect()));
            continue;
        }
        if let Some(close) = match ch {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            _ => None,
        } {
            *at += 1;
            let items = parse_until(chars, at, Some(close))?;
            forms.push(Form::Group {
                open: ch,
                close,
                items,
            });
            continue;
        }
        let start = *at;
        if ch == '"' || ch == '\'' {
            let quote = ch;
            *at += 1;
            let mut escaped = false;
            while *at < chars.len() {
                let current = chars[*at];
                *at += 1;
                if escaped {
                    escaped = false;
                } else if current == '\\' {
                    escaped = true;
                } else if current == quote {
                    break;
                }
            }
            if chars.get(at.saturating_sub(1)) != Some(&quote) {
                return Err(format!(
                    "unterminated {} literal",
                    if quote == '"' { "string" } else { "character" }
                ));
            }
        } else {
            while *at < chars.len()
                && !chars[*at].is_whitespace()
                && !matches!(chars[*at], '(' | ')' | '[' | ']' | '{' | '}' | ';')
            {
                *at += 1;
            }
        }
        forms.push(Form::Atom(chars[start..*at].iter().collect()));
    }
    if let Some(close) = expected_close {
        Err(format!("unclosed form; expected '{}'", close))
    } else {
        Ok(forms)
    }
}

fn flat(form: &Form) -> Option<String> {
    match form {
        Form::Atom(value) => Some(value.clone()),
        Form::Comment(_) => None,
        Form::Group { open, close, items } => {
            let children = items.iter().map(flat).collect::<Option<Vec<_>>>()?;
            Some(format!("{}{}{}", open, children.join(" "), close))
        }
    }
}

fn head(items: &[Form]) -> &str {
    match items.first() {
        Some(Form::Atom(value)) => value,
        _ => "",
    }
}

fn prefix_count(open: char, items: &[Form]) -> usize {
    if open != '(' {
        return 0;
    }
    match head(items) {
        "let" | "letrec" | "mut" | "sig" | "letype" => 3,
        "lambda" => 2,
        "if" => 3,
        "while" => 2,
        "loop" => 3,
        "loop/range" => 4,
        "loop/in" | "map" | "select" | "reduce" => 2,
        "cond" => 3,
        "block" | "do" => 1,
        _ => 1,
    }
    .min(items.len())
}

fn append_rendered(lines: &mut Vec<String>, rendered: String, append_first: bool, indent: usize) {
    let mut incoming = rendered.lines();
    if let Some(first) = incoming.next() {
        if append_first {
            if let Some(last) = lines.last_mut() {
                if !last.ends_with(['(', '[', '{']) {
                    last.push(' ');
                }
                last.push_str(first.trim_start());
            }
        } else {
            lines.push(format!("{}{}", " ".repeat(indent), first.trim_start()));
        }
    }
    if append_first {
        let prefix = " ".repeat(INDENT);
        lines.extend(incoming.map(|line| line.strip_prefix(&prefix).unwrap_or(line).to_string()));
    } else {
        lines.extend(incoming.map(str::to_string));
    }
}

fn render(form: &Form, indent: usize, width: usize) -> String {
    if let Some(one_line) = flat(form) {
        if indent + one_line.chars().count() <= width {
            return format!("{}{}", " ".repeat(indent), one_line);
        }
    }
    match form {
        Form::Atom(value) => format!("{}{}", " ".repeat(indent), value),
        Form::Comment(value) => format!("{}{}", " ".repeat(indent), value.trim_end()),
        Form::Group { open, close, items } => {
            if items.is_empty() {
                return format!("{}{}{}", " ".repeat(indent), open, close);
            }
            let prefix = prefix_count(*open, items);
            let mut lines = vec![format!("{}{}", " ".repeat(indent), open)];
            for (index, item) in items.iter().enumerate() {
                let child_indent = indent + INDENT;
                let rendered = render(item, child_indent, width);
                let can_append = index < prefix
                    && !matches!(item, Form::Comment(_))
                    && lines.last().is_some_and(|line| {
                        let first = rendered.lines().next().unwrap_or("").trim_start();
                        line.chars().count() + 1 + first.chars().count() <= width
                    });
                append_rendered(&mut lines, rendered, can_append, child_indent);
            }
            if matches!(items.last(), Some(Form::Comment(_))) {
                lines.push(format!("{}{}", " ".repeat(indent), close));
            } else if let Some(last) = lines.last_mut() {
                last.push(*close);
            }
            lines.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_nested_que_in_compact_lisp_style() {
        let source = "(let search? (lambda (target xs) (letrec bs? (lambda (left right) (if (or (< left 0) (> left right)) false (block (let index (+ left (/ (- right left) 2))) (bs? (+ index 1) right))))) (bs? 0 (- (length xs) 1))))";
        let formatted = format_source(source).expect("formatting should work");
        assert!(
            formatted.starts_with("(let search? (lambda (target xs)\n"),
            "{formatted}"
        );
        assert!(formatted.contains("\n  (letrec bs? (lambda (left right)\n"));
        assert!(formatted.contains("\n    (if (or (< left 0) (> left right)) false"));
        assert!(formatted.ends_with("\n"));
    }

    #[test]
    fn preserves_comments_strings_and_character_literals() {
        let source = "; heading\n(let x \"; not comment\") ; tail\n(let slash '\\\\')\n(block 1 ; before close\n)";
        let formatted = format_source(source).expect("formatting should work");
        assert!(formatted.contains("; heading"));
        assert!(formatted.contains("\"; not comment\""));
        assert!(formatted.contains("; tail"));
        assert!(formatted.contains("'\\\\'"));
        assert!(formatted.contains("; before close\n)"));
    }

    #[test]
    fn formatting_is_idempotent() {
        let once =
            format_source("(let xs [78 9 20 10 30 2])\n(loop i (< i (length xs)) (get xs i))")
                .expect("first format");
        let twice = format_source(&once).expect("second format");
        assert_eq!(once, twice);
    }
}
