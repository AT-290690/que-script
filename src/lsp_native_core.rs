use crate::infer::{ EffectFlags, infer_with_builtins_typed, InferErrorScope, TypedExpression };
use crate::parser::{ self, Expression };
use crate::types::{ create_builtin_environment, Type, TypeEnv };
use std::collections::{ HashMap, HashSet };

pub const LSP_SPECIAL_KEYWORD_SIGNATURES: [(&str, &str); 5] = [
    ("alter!", "T -> T -> ()"),
    ("vector", "T... -> [T]"),
    ("string", "Char... -> [Char]"),
    ("tuple", "T... -> {T...}"),
    ("loop", "Int -> Int -> (Int -> ()) -> ()"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CorePosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoreRange {
    pub start: CorePosition,
    pub end: CorePosition,
}

pub fn normalize_signature(signature: &str) -> String {
    let trimmed = signature.trim();
    let without_quantifier = if let Some(dot_idx) = trimmed.find('.') {
        let prefix = trimmed[..dot_idx].trim();
        let looks_like_quantifier = is_quantifier_prefix(prefix);
        if looks_like_quantifier {
            trimmed[dot_idx + 1..].trim()
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    strip_type_var_numbers(without_quantifier)
}

fn is_quantifier_prefix(prefix: &str) -> bool {
    if prefix.is_empty() {
        return false;
    }

    let chars: Vec<char> = prefix.chars().collect();
    let mut i = 0usize;
    let mut saw_any = false;

    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ',') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        if chars[i] != 'T' {
            return false;
        }
        saw_any = true;
        i += 1;

        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        let digit_start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i == digit_start {
            return false;
        }
    }

    saw_any
}

pub fn strip_type_var_numbers(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == 'T' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let digit_start = j;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j > digit_start {
                out.push('T');
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

pub fn build_base_environment(
    std_defs: &[Expression]
) -> (TypeEnv, u64, HashMap<String, String>, HashMap<String, EffectFlags>) {
    let (env, next_id) = create_builtin_environment(TypeEnv::new());

    let mut signatures = HashMap::new();
    for scope in &env.scopes {
        for (name, scheme) in scope {
            signatures
                .entry(name.clone())
                .or_insert_with(|| normalize_signature(&scheme.to_string()));
        }
    }

    for (name, signature) in infer_std_signatures(&env, next_id, std_defs) {
        signatures.insert(name, normalize_signature(&signature));
    }
    for (name, signature) in LSP_SPECIAL_KEYWORD_SIGNATURES {
        signatures
            .entry(name.to_string())
            .or_insert_with(|| normalize_signature(signature));
    }

    let std_effects = infer_std_effects(&env, next_id, std_defs);

    (env, next_id, signatures, std_effects)
}

pub fn load_std_definitions() -> Vec<Expression> {
    let ast = crate::baked::load_ast();
    if let Expression::Apply(items) = ast {
        return items.into_iter().skip(1).collect();
    }
    Vec::new()
}

pub fn collect_std_top_level_let_names(std_defs: &[Expression]) -> HashSet<String> {
    let mut names = HashSet::new();
    for expr in std_defs {
        if let Expression::Apply(items) = expr {
            if let [Expression::Word(keyword), Expression::Word(name), _rhs, ..] = &items[..] {
                if keyword == "let" || keyword == "let*" || keyword == "mut" {
                    names.insert(name.clone());
                }
            }
        }
    }
    names
}

pub fn infer_std_signatures(
    base_env: &TypeEnv,
    base_next_id: u64,
    std_defs: &[Expression]
) -> HashMap<String, String> {
    let mut raw_signatures: HashMap<String, Type> = HashMap::new();
    if std_defs.is_empty() {
        return HashMap::new();
    }

    let std_program = Expression::Apply(
        std::iter
            ::once(Expression::Word("do".to_string()))
            .chain(std_defs.iter().cloned())
            .collect()
    );

    if
        let Ok((_typ, typed)) = infer_with_builtins_typed(&std_program, (
            base_env.clone(),
            base_next_id,
        ))
    {
        collect_let_binding_types(&typed, &mut raw_signatures);
    }

    raw_signatures
        .into_iter()
        .map(|(name, typ)| (name, normalize_signature(&typ.to_string())))
        .collect()
}

pub fn infer_std_effects(
    base_env: &TypeEnv,
    base_next_id: u64,
    std_defs: &[Expression]
) -> HashMap<String, EffectFlags> {
    if std_defs.is_empty() {
        return HashMap::new();
    }

    let std_program = Expression::Apply(
        std::iter
            ::once(Expression::Word("do".to_string()))
            .chain(std_defs.iter().cloned())
            .collect()
    );

    let mut effects: HashMap<String, EffectFlags> = HashMap::new();
    if
        let Ok((_typ, typed)) = infer_with_builtins_typed(&std_program, (
            base_env.clone(),
            base_next_id,
        ))
    {
        collect_let_binding_effects(&typed, &mut effects, &HashMap::new());
        let mut fallback = effects.clone();
        for _ in 0..8 {
            let mut next = fallback.clone();
            collect_let_binding_effects(&typed, &mut next, &fallback);
            if next == fallback {
                break;
            }
            fallback = next;
        }
        effects = fallback;
    }

    effects
}

pub fn collect_let_binding_types(node: &TypedExpression, signatures: &mut HashMap<String, Type>) {
    if let Expression::Apply(items) = &node.expr {
        if let [Expression::Word(keyword), Expression::Word(name), _rhs, ..] = &items[..] {
            if keyword == "let" || keyword == "let*" || keyword == "mut" {
                if let Some(rhs_type) = node.children.get(2).and_then(|child| child.typ.as_ref()) {
                    match signatures.get(name) {
                        Some(existing) => {
                            if should_replace_type(existing, rhs_type) {
                                signatures.insert(name.clone(), rhs_type.clone());
                            }
                        }
                        None => {
                            signatures.insert(name.clone(), rhs_type.clone());
                        }
                    }
                }
            }
        }
    }

    for child in &node.children {
        collect_let_binding_types(child, signatures);
    }
}

fn effect_specificity_score(effect: EffectFlags) -> i32 {
    let mut score = 0;
    if effect.contains(EffectFlags::MUTATE) {
        score += 1;
    }
    if effect.contains(EffectFlags::IO) {
        score += 2;
    }
    if effect.contains(EffectFlags::UNKNOWN_CALL) {
        score += 4;
    }
    score
}

fn should_replace_effect(existing: EffectFlags, candidate: EffectFlags) -> bool {
    effect_specificity_score(candidate) > effect_specificity_score(existing)
}

pub fn collect_let_binding_effects(
    node: &TypedExpression,
    effects: &mut HashMap<String, EffectFlags>,
    fallback_effects: &HashMap<String, EffectFlags>
) {
    if let Expression::Apply(items) = &node.expr {
        if let [Expression::Word(keyword), Expression::Word(name), _rhs, ..] = &items[..] {
            if keyword == "let" || keyword == "let*" || keyword == "mut" {
                if let Some(rhs_node) = node.children.get(2) {
                    let mut rhs_effect = rhs_node.effect;
                    if rhs_effect.is_pure() {
                        if let Expression::Word(alias_target) = &rhs_node.expr {
                            if let Some(target_effect) = effects
                                .get(alias_target)
                                .copied()
                                .or_else(|| fallback_effects.get(alias_target).copied())
                                .or_else(|| known_symbol_effect(alias_target))
                            {
                                rhs_effect = target_effect;
                            }
                        }
                    }
                    match effects.get(name).copied() {
                        Some(existing) => {
                            if should_replace_effect(existing, rhs_effect) {
                                effects.insert(name.clone(), rhs_effect);
                            }
                        }
                        None => {
                            effects.insert(name.clone(), rhs_effect);
                        }
                    }
                }
            }
        }
    }

    for child in &node.children {
        collect_let_binding_effects(child, effects, fallback_effects);
    }
}

pub fn known_symbol_effect(symbol: &str) -> Option<EffectFlags> {
    if
        matches!(
            symbol,
            "read!" |
                "write!" |
                "list-dir!" |
                "mkdir!" |
                "delete!" |
                "move!" |
                "print!" |
                "sleep!" |
                "clear!"
        )
    {
        return Some(EffectFlags::IO);
    }
    if matches!(symbol, "set!" | "alter!" | "pop!") {
        return Some(EffectFlags::MUTATE);
    }
    if symbol.ends_with('!') {
        return Some(EffectFlags::MUTATE);
    }
    None
}

pub fn format_effect_flags(effect: EffectFlags) -> Option<String> {
    if effect.is_pure() {
        return None;
    }
    let mut labels = Vec::new();
    if effect.contains(EffectFlags::MUTATE) {
        labels.push("mutate");
    }
    if effect.contains(EffectFlags::IO) {
        labels.push("io");
    }
    if effect.contains(EffectFlags::UNKNOWN_CALL) {
        labels.push("unknown-call");
    }
    if labels.is_empty() {
        None
    } else {
        Some(labels.join(", "))
    }
}

pub fn format_effect_flags_for_symbol(
    symbol: &str,
    effect: EffectFlags,
    externally_impure: Option<bool>
) -> Option<String> {
    if effect.is_pure() {
        return None;
    }
    let mut labels = Vec::new();
    if effect.contains(EffectFlags::MUTATE) {
        if symbol.ends_with('!') || symbol == "set" || externally_impure == Some(true) {
            labels.push("mutate");
        } else {
            labels.push("local-mutate");
        }
    }
    if effect.contains(EffectFlags::IO) {
        labels.push("io");
    }
    if effect.contains(EffectFlags::UNKNOWN_CALL) {
        labels.push("unknown-call");
    }
    if labels.is_empty() {
        None
    } else {
        Some(labels.join(", "))
    }
}

pub fn collect_symbol_types(node: &TypedExpression, symbols: &mut HashMap<String, Type>) {
    if let Expression::Word(name) = &node.expr {
        if let Some(typ) = &node.typ {
            match symbols.get(name) {
                Some(existing) => {
                    if should_replace_type(existing, typ) {
                        symbols.insert(name.clone(), typ.clone());
                    }
                }
                None => {
                    symbols.insert(name.clone(), typ.clone());
                }
            }
        }
    }
    for child in &node.children {
        collect_symbol_types(child, symbols);
    }
}

fn type_specificity_score(typ: &Type) -> i32 {
    match typ {
        Type::Var(_) => 0,
        Type::Int | Type::Float | Type::Bool | Type::Char | Type::Unit => 1,
        Type::List(inner) => 2 + type_specificity_score(inner),
        Type::Tuple(items) => 2 + items.iter().map(type_specificity_score).sum::<i32>(),
        Type::Function(a, b) => 3 + type_specificity_score(a) + type_specificity_score(b),
    }
}

fn should_replace_type(existing: &Type, candidate: &Type) -> bool {
    match (existing, candidate) {
        (Type::Var(_), Type::Var(_)) => false,
        (Type::Var(_), _) => true,
        (_, Type::Var(_)) => false,
        _ => {
            let existing_score = type_specificity_score(existing);
            let candidate_score = type_specificity_score(candidate);
            candidate_score > existing_score
        }
    }
}

pub fn parse_user_exprs_for_symbol_collection(text: &str) -> Option<Vec<Expression>> {
    let masked = mask_literals_for_structural_parse(text);
    if let Ok(exprs) = parser::parse(&masked) {
        return Some(exprs);
    }

    let repaired_masked = repair_source_for_analysis(&masked);
    parser::parse(&repaired_masked).ok()
}

pub fn strip_comment_bodies_preserve_newlines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_comment = false;
    let mut in_string = false;
    let mut in_char = false;

    for ch in text.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
                out.push('\n');
            }
            continue;
        }

        if in_string {
            out.push(ch);
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if in_char {
            out.push(ch);
            if ch == '\'' {
                in_char = false;
            }
            continue;
        }

        match ch {
            ';' => {
                in_comment = true;
                out.push(' ');
            }
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '\'' => {
                in_char = true;
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }

    out
}

pub fn mask_literals_for_structural_parse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_comment = false;
    let mut in_string = false;
    let mut in_char = false;
    let mut string_id = 0usize;
    let mut char_id = 0usize;

    for ch in text.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
                out.push('\n');
            }
            continue;
        }

        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if in_char {
            if ch == '\'' {
                in_char = false;
            }
            continue;
        }

        match ch {
            ';' => {
                in_comment = true;
                out.push(' ');
            }
            '"' => {
                out.push(' ');
                out.push_str(&format!("__STR{}__", string_id));
                out.push(' ');
                string_id += 1;
                in_string = true;
            }
            '\'' => {
                out.push(' ');
                out.push_str(&format!("__CHR{}__", char_id));
                out.push(' ');
                char_id += 1;
                in_char = true;
            }
            _ => out.push(ch),
        }
    }

    out
}

pub fn repair_source_for_analysis(text: &str) -> String {
    let mut repaired = text.to_string();
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut in_comment = false;

    for ch in text.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }

        if !in_string && ch == ';' {
            in_comment = true;
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            continue;
        }

        if in_string {
            continue;
        }

        match ch {
            '(' => stack.push(')'),
            '[' => stack.push(']'),
            ')' | ']' => {
                if let Some(expected) = stack.last().copied() {
                    if ch == expected {
                        let _ = stack.pop();
                    }
                }
            }
            _ => {}
        }
    }

    if in_string {
        repaired.push('"');
    }
    while let Some(close) = stack.pop() {
        repaired.push(close);
    }
    repaired
}

pub fn diagnostic_summary_without_snippet(message: &str) -> String {
    let mut lines: Vec<&str> = message
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    if let Some(last) = lines.last() {
        if last.starts_with('(') && last.ends_with(')') {
            lines.pop();
        }
    }

    if lines.is_empty() {
        return message.trim().to_string();
    }

    lines.join(" | ")
}

pub fn collect_user_bound_symbols_from_exprs(exprs: &[Expression], out: &mut HashSet<String>) {
    for expr in exprs {
        collect_user_bound_symbols(expr, out);
    }
}

fn collect_user_bound_symbols(expr: &Expression, out: &mut HashSet<String>) {
    if let Expression::Apply(items) = expr {
        if let Some(Expression::Word(head)) = items.first() {
            if head == "let" || head == "let*" || head == "mut" {
                if let Some(Expression::Word(name)) = items.get(1) {
                    out.insert(name.clone());
                }
            } else if head == "lambda" && items.len() >= 2 {
                for param in items.iter().skip(1).take(items.len().saturating_sub(2)) {
                    collect_pattern_words(param, out);
                }
            }
        }
        for item in items {
            collect_user_bound_symbols(item, out);
        }
    }
}

fn collect_pattern_words(expr: &Expression, out: &mut HashSet<String>) {
    match expr {
        Expression::Word(word) => {
            if word != "." {
                out.insert(word.clone());
            }
        }
        Expression::Apply(items) => {
            for item in items {
                collect_pattern_words(item, out);
            }
        }
        _ => {}
    }
}

pub fn is_standalone_symbol_expr_at_range(text: &str, range: CoreRange, symbol: &str) -> bool {
    let Some(line_text) = text.lines().nth(range.start.line as usize) else {
        return false;
    };
    let start = range.start.character as usize;
    let end = range.end.character as usize;
    if start > end || end > line_text.len() {
        return false;
    }
    let Some(slice) = line_text.get(start..end) else {
        return false;
    };
    if slice != symbol {
        return false;
    }

    let before = &line_text[..start];
    let after = &line_text[end..];
    let after_trimmed = after.trim_start();
    before.trim().is_empty() && (after_trimmed.is_empty() || after_trimmed.starts_with(';'))
}

pub fn infer_error_ranges(
    text: &str,
    message: &str,
    scope: Option<&InferErrorScope>
) -> Vec<CoreRange> {
    if message.contains("Char should be of length 1") {
        if let Some(range) = find_invalid_char_literal_range(text) {
            return vec![range];
        }
    }

    if let Some(snippet) = extract_error_snippet(message) {
        let mut ranges = find_snippet_ranges(text, &snippet);
        if ranges.is_empty() {
            ranges = find_call_prefix_ranges(text, &snippet);
        }
        if !ranges.is_empty() {
            if let Some(scope_meta) = scope {
                let scoped = filter_ranges_to_scope(text, &ranges, scope_meta);
                if !scoped.is_empty() {
                    return scoped;
                }
                if let Some(scope_range) = find_scope_range(text, scope_meta) {
                    return vec![scope_range];
                }
            }
            return ranges;
        }
    }

    if let Some(symbol) = extract_symbol_from_error(message) {
        let ranges = find_symbol_ranges(text, &symbol);
        if !ranges.is_empty() {
            if let Some(scope_meta) = scope {
                let scoped = filter_ranges_to_scope(text, &ranges, scope_meta);
                if !scoped.is_empty() {
                    return scoped;
                }
                if let Some(scope_range) = find_scope_range(text, scope_meta) {
                    return vec![scope_range];
                }
            }
            return ranges;
        }
    }

    if let Some(scope_meta) = scope {
        if let Some(scope_range) = find_scope_range(text, scope_meta) {
            return vec![scope_range];
        }
    }

    find_first_call_range(text).into_iter().collect()
}

pub fn infer_error_range(text: &str, message: &str) -> Option<CoreRange> {
    infer_error_ranges(text, message, None).into_iter().next()
}

pub fn extract_error_snippet(message: &str) -> Option<String> {
    for line in message.lines().map(str::trim) {
        if line.starts_with('(') && line.ends_with(')') && line.len() >= 2 {
            return Some(line.to_string());
        }
    }

    let bytes = message.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let start = i;
            let mut depth = 1usize;
            i += 1;
            while i < bytes.len() {
                match bytes[i] {
                    b'(' => {
                        depth += 1;
                    }
                    b')' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            if let Some(slice) = message.get(start..=i) {
                                return Some(slice.trim().to_string());
                            }
                            return None;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            break;
        }
        i += 1;
    }
    None
}

pub fn symbol_at_position(text: &str, position: CorePosition) -> Option<(String, CoreRange)> {
    let line_text = text.lines().nth(position.line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut idx = (position.character as usize).min(chars.len().saturating_sub(1));
    if !is_symbol_char(chars[idx]) {
        if idx > 0 && is_symbol_char(chars[idx - 1]) {
            idx -= 1;
        } else {
            return None;
        }
    }

    let mut left = idx;
    while left > 0 && is_symbol_char(chars[left - 1]) {
        left -= 1;
    }
    let mut right = idx;
    while right + 1 < chars.len() && is_symbol_char(chars[right + 1]) {
        right += 1;
    }

    let symbol: String = chars[left..=right].iter().collect();
    if symbol.is_empty() {
        return None;
    }

    let range = CoreRange {
        start: CorePosition {
            line: position.line,
            character: left as u32,
        },
        end: CorePosition {
            line: position.line,
            character: (right + 1) as u32,
        },
    };
    Some((symbol, range))
}

pub fn literal_type_at_position(text: &str, position: CorePosition) -> Option<(String, CoreRange)> {
    let offset = position_to_byte_offset(text, position)?;

    if let Some((start, end)) = find_enclosing_string_literal(text, offset) {
        let range = CoreRange {
            start: byte_offset_to_position(text, start),
            end: byte_offset_to_position(text, end),
        };
        return Some(("[Char]".to_string(), range));
    }

    if let Some((start, end)) = find_enclosing_char_literal(text, offset) {
        let range = CoreRange {
            start: byte_offset_to_position(text, start),
            end: byte_offset_to_position(text, end),
        };
        return Some(("Char".to_string(), range));
    }

    if let Some((token, start, end)) = numeric_token_at_offset(text, offset) {
        let typ = if is_int_token(&token) {
            Some("Int")
        } else if is_float_token(&token) {
            Some("Float")
        } else {
            None
        };
        if let Some(typ) = typ {
            let range = CoreRange {
                start: byte_offset_to_position(text, start),
                end: byte_offset_to_position(text, end),
            };
            return Some((typ.to_string(), range));
        }
    }

    None
}

pub fn format_literal_hover(text: &str, range: CoreRange, literal_type: &str) -> String {
    if literal_type == "[Char]" {
        if let Some((preview, len, truncated)) = preview_string_literal(text, range, 16) {
            let suffix = if truncated { "..." } else { "" };
            return format!("\"{}{}\" : [Char] length : {}", preview, suffix, len);
        }
    }

    let literal_text = text_for_range(text, range).unwrap_or_default();
    format!("{} : {}", literal_text, literal_type)
}

fn preview_string_literal(
    text: &str,
    range: CoreRange,
    max_chars: usize
) -> Option<(String, usize, bool)> {
    let start = position_to_byte_offset(text, range.start)?;
    let end = position_to_byte_offset(text, range.end)?;
    let raw = text.get(start..end)?;
    let content = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);

    let mut preview = String::new();
    let mut len = 0usize;
    let mut truncated = false;

    for ch in content.chars() {
        len += 1;
        if len <= max_chars {
            append_escaped_preview_char(&mut preview, ch);
        } else {
            truncated = true;
        }
    }

    Some((preview, len, truncated))
}

fn append_escaped_preview_char(out: &mut String, ch: char) {
    match ch {
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        '"' => out.push_str("\\\""),
        '\\' => out.push_str("\\\\"),
        _ => out.push(ch),
    }
}

pub fn position_to_byte_offset(text: &str, position: CorePosition) -> Option<usize> {
    let mut line = 0u32;
    let mut col = 0u32;
    for (idx, ch) in text.char_indices() {
        if line == position.line && col == position.character {
            return Some(idx);
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    if line == position.line && col == position.character {
        return Some(text.len());
    }
    None
}

pub fn byte_offset_to_position(text: &str, target: usize) -> CorePosition {
    let mut line = 0u32;
    let mut col = 0u32;
    for (idx, ch) in text.char_indices() {
        if idx >= target {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    CorePosition {
        line,
        character: col,
    }
}

pub fn text_for_range(text: &str, range: CoreRange) -> Option<String> {
    let start = position_to_byte_offset(text, range.start)?;
    let end = position_to_byte_offset(text, range.end)?;
    text.get(start..end).map(|s| s.to_string())
}

pub fn full_range(text: &str) -> CoreRange {
    let end = end_position(text);
    CoreRange {
        start: CorePosition {
            line: 0,
            character: 0,
        },
        end,
    }
}

fn end_position(text: &str) -> CorePosition {
    let mut line = 0u32;
    let mut character = 0u32;
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    CorePosition { line, character }
}

#[derive(Clone, Debug)]
struct ScopeRegion {
    top_form_idx: usize,
    lambda_path: Vec<usize>,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug)]
struct ListNode {
    start: usize,
    end: usize,
    head: Option<String>,
    children: Vec<ListNode>,
}

pub fn top_level_form_ranges(text: &str) -> Vec<CoreRange> {
    top_level_form_byte_ranges(text)
        .into_iter()
        .map(|(start, end)| CoreRange {
            start: byte_offset_to_position(text, start),
            end: byte_offset_to_position(text, end),
        })
        .collect()
}

fn top_level_form_byte_ranges(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut ranges = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        while i < bytes.len() {
            if bytes[i].is_ascii_whitespace() {
                i += 1;
                continue;
            }
            if bytes[i] == b';' {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            break;
        }

        if i >= bytes.len() {
            break;
        }

        let start = i;
        match bytes[i] {
            b'(' | b'[' => {
                let end = find_matching_list_end_byte(text, i)
                    .map(|idx| idx + 1)
                    .unwrap_or(text.len());
                ranges.push((start, end));
                i = end;
            }
            b'"' => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                ranges.push((start, i));
            }
            _ => {
                i += 1;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b';' {
                    i += 1;
                }
                ranges.push((start, i));
            }
        }
    }

    ranges
}

fn find_matching_list_end_byte(text: &str, open_idx: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let open = *bytes.get(open_idx)?;
    if open != b'(' && open != b'[' {
        return None;
    }

    let mut stack = vec![if open == b'(' { b')' } else { b']' }];
    let mut in_string = false;
    let mut in_comment = false;
    let mut i = open_idx + 1;

    while i < bytes.len() {
        let b = bytes[i];

        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }

        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }

        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            i += 1;
            continue;
        }

        if in_string {
            i += 1;
            continue;
        }

        match b {
            b'(' => stack.push(b')'),
            b'[' => stack.push(b']'),
            b')' | b']' => {
                let Some(expected) = stack.pop() else {
                    return None;
                };
                if b != expected {
                    return None;
                }
                if stack.is_empty() {
                    return Some(i);
                }
            }
            _ => {}
        }

        i += 1;
    }

    None
}

fn find_scope_range(text: &str, scope: &InferErrorScope) -> Option<CoreRange> {
    collect_scope_regions(text)
        .into_iter()
        .find(|region| region.top_form_idx == scope.user_top_form && region.lambda_path == scope.lambda_path)
        .map(|region| CoreRange {
            start: byte_offset_to_position(text, region.start),
            end: byte_offset_to_position(text, region.end),
        })
}

fn filter_ranges_to_scope(text: &str, ranges: &[CoreRange], scope: &InferErrorScope) -> Vec<CoreRange> {
    let Some(scope_range) = find_scope_range(text, scope) else {
        return Vec::new();
    };
    let Some(scope_start) = position_to_byte_offset(text, scope_range.start) else {
        return Vec::new();
    };
    let Some(scope_end) = position_to_byte_offset(text, scope_range.end) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for range in ranges {
        let Some(start) = position_to_byte_offset(text, range.start) else {
            continue;
        };
        let Some(end) = position_to_byte_offset(text, range.end) else {
            continue;
        };
        if start >= scope_start && end <= scope_end && seen.insert((start, end)) {
            out.push(*range);
        }
    }
    out
}

fn collect_scope_regions(text: &str) -> Vec<ScopeRegion> {
    let mut out = Vec::new();
    for (top_form_idx, (start, end)) in top_level_form_byte_ranges(text).into_iter().enumerate() {
        out.push(ScopeRegion {
            top_form_idx,
            lambda_path: Vec::new(),
            start,
            end,
        });

        let Some(open) = text.as_bytes().get(start) else {
            continue;
        };
        if *open != b'(' && *open != b'[' {
            continue;
        }
        let Some(root) = parse_list_node_at(text, start) else {
            continue;
        };
        collect_lambda_scope_regions(&root, top_form_idx, &[], &mut out);
    }
    out
}

fn collect_lambda_scope_regions(
    node: &ListNode,
    top_form_idx: usize,
    parent_path: &[usize],
    out: &mut Vec<ScopeRegion>
) {
    let mut next_lambda_idx = 0usize;
    collect_lambda_descendants(node, top_form_idx, parent_path, &mut next_lambda_idx, out);
}

fn collect_lambda_descendants(
    node: &ListNode,
    top_form_idx: usize,
    parent_path: &[usize],
    next_lambda_idx: &mut usize,
    out: &mut Vec<ScopeRegion>
) {
    for child in &node.children {
        if child.head.as_deref() == Some("lambda") {
            let mut child_path = parent_path.to_vec();
            child_path.push(*next_lambda_idx);
            *next_lambda_idx += 1;

            out.push(ScopeRegion {
                top_form_idx,
                lambda_path: child_path.clone(),
                start: child.start,
                end: child.end,
            });

            collect_lambda_scope_regions(child, top_form_idx, &child_path, out);
        } else {
            collect_lambda_descendants(child, top_form_idx, parent_path, next_lambda_idx, out);
        }
    }
}

fn parse_list_node_at(text: &str, open_idx: usize) -> Option<ListNode> {
    let close_idx = find_matching_list_end_byte(text, open_idx)?;
    let mut node = ListNode {
        start: open_idx,
        end: close_idx + 1,
        head: None,
        children: Vec::new(),
    };
    let bytes = text.as_bytes();
    let mut i = open_idx + 1;

    while i < close_idx {
        i = skip_ws_and_comments(text, i, close_idx);
        if i >= close_idx {
            break;
        }

        match bytes[i] {
            b'(' | b'[' => {
                let child = parse_list_node_at(text, i)?;
                i = child.end;
                node.children.push(child);
            }
            b'"' => {
                i = skip_string_literal(text, i, close_idx);
            }
            b'\'' => {
                i = skip_char_literal(text, i, close_idx);
            }
            _ => {
                let token_end = skip_token(text, i, close_idx);
                if node.head.is_none() {
                    node.head = text.get(i..token_end).map(|s| s.to_string());
                }
                i = token_end;
            }
        }
    }

    Some(node)
}

fn skip_ws_and_comments(text: &str, mut i: usize, limit: usize) -> usize {
    let bytes = text.as_bytes();
    while i < limit {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b == b';' {
            i += 1;
            while i < limit && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        break;
    }
    i
}

fn skip_string_literal(text: &str, start: usize, limit: usize) -> usize {
    let bytes = text.as_bytes();
    let mut i = start + 1;
    while i < limit {
        if bytes[i] == b'"' && bytes[i - 1] != b'\\' {
            return i + 1;
        }
        i += 1;
    }
    limit
}

fn skip_char_literal(text: &str, start: usize, limit: usize) -> usize {
    let bytes = text.as_bytes();
    let mut i = start + 1;
    while i < limit {
        if bytes[i] == b'\'' {
            return i + 1;
        }
        if bytes[i] == b'\n' {
            return i;
        }
        i += 1;
    }
    limit
}

fn skip_token(text: &str, mut i: usize, limit: usize) -> usize {
    let bytes = text.as_bytes();
    while i < limit {
        let b = bytes[i];
        if
            b.is_ascii_whitespace() ||
            b == b';' ||
            b == b'(' ||
            b == b')' ||
            b == b'[' ||
            b == b']'
        {
            break;
        }
        i += 1;
    }
    i
}

fn find_invalid_char_literal_range(text: &str) -> Option<CoreRange> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_comment = false;

    while i < text.len() {
        let ch = text[i..].chars().next()?;
        let ch_len = ch.len_utf8();

        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            i += ch_len;
            continue;
        }

        if !in_string && ch == ';' {
            in_comment = true;
            i += ch_len;
            continue;
        }

        if ch == '"' {
            let escaped = i > 0 && bytes[i - 1] == b'\\';
            if !escaped {
                in_string = !in_string;
            }
            i += ch_len;
            continue;
        }

        if !in_string && ch == '\'' {
            let start = i;
            i += ch_len;
            let mut char_count = 0usize;
            let mut closed = false;

            while i < text.len() {
                let inner = text[i..].chars().next()?;
                let inner_len = inner.len_utf8();
                if inner == '\'' {
                    closed = true;
                    i += inner_len;
                    break;
                }
                char_count += 1;
                i += inner_len;
            }

            if !closed {
                return Some(CoreRange {
                    start: byte_offset_to_position(text, start),
                    end: byte_offset_to_position(text, (start + 1).min(text.len())),
                });
            }

            if char_count != 1 {
                return Some(CoreRange {
                    start: byte_offset_to_position(text, start),
                    end: byte_offset_to_position(text, i),
                });
            }

            continue;
        }

        i += ch_len;
    }

    None
}

fn find_snippet_ranges(text: &str, snippet: &str) -> Vec<CoreRange> {
    if snippet.is_empty() {
        return Vec::new();
    }
    let bytes = text.as_bytes();
    let needle = snippet.as_bytes();
    if needle.len() > bytes.len() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_comment = false;
    while i + needle.len() <= bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }
        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if in_string {
            i += 1;
            continue;
        }

        if &bytes[i..i + needle.len()] == needle {
            let start = i;
            let end = i + needle.len();
            out.push(CoreRange {
                start: byte_offset_to_position(text, start),
                end: byte_offset_to_position(text, end),
            });
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

fn find_call_prefix_ranges(text: &str, snippet: &str) -> Vec<CoreRange> {
    let all_tokens = extract_call_prefix_tokens(snippet, 3);
    if all_tokens.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for token_count in (1..=all_tokens.len()).rev() {
        let prefix_tokens = &all_tokens[..token_count];
        let bytes = text.as_bytes();
        let mut i = 0usize;
        let mut in_string = false;
        let mut in_comment = false;
        while i < bytes.len() {
            if in_comment {
                if bytes[i] == b'\n' {
                    in_comment = false;
                }
                i += 1;
                continue;
            }
            if !in_string && bytes[i] == b';' {
                in_comment = true;
                i += 1;
                continue;
            }
            if bytes[i] == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
                in_string = !in_string;
                i += 1;
                continue;
            }
            if in_string {
                i += 1;
                continue;
            }

            if bytes[i] == b'(' && match_call_prefix_at(text, i, prefix_tokens) {
                if let Some(close) = find_matching_paren_byte(text, i) {
                    out.push(CoreRange {
                        start: byte_offset_to_position(text, i),
                        end: byte_offset_to_position(text, close + 1),
                    });
                    i = close + 1;
                    continue;
                }
            }
            i += 1;
        }
        if !out.is_empty() {
            break;
        }
    }
    out
}

fn extract_call_prefix_tokens(snippet: &str, max_tokens: usize) -> Vec<String> {
    let trimmed = snippet.trim();
    let inner = if let Some(stripped) = trimmed.strip_prefix('(') { stripped } else { trimmed };
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    let mut in_string = false;
    for ch in inner.chars() {
        if in_string {
            cur.push(ch);
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                in_string = true;
                cur.push(ch);
            }
            '(' | '[' | '{' => {
                depth += 1;
                cur.push(ch);
            }
            ')' | ']' | '}' => {
                if depth == 0 {
                    break;
                }
                depth = depth.saturating_sub(1);
                cur.push(ch);
            }
            ' ' | '\t' | '\n' | '\r' if depth == 0 => {
                if !cur.trim().is_empty() {
                    tokens.push(cur.trim().to_string());
                    if tokens.len() >= max_tokens {
                        break;
                    }
                    cur.clear();
                }
            }
            _ => cur.push(ch),
        }
    }
    if tokens.len() < max_tokens && !cur.trim().is_empty() {
        tokens.push(cur.trim().to_string());
    }
    tokens
}

fn match_call_prefix_at(text: &str, open_idx: usize, tokens: &[String]) -> bool {
    let bytes = text.as_bytes();
    let mut i = open_idx + 1;

    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    for (idx, token) in tokens.iter().enumerate() {
        if token.is_empty() {
            return false;
        }
        let token_bytes = token.as_bytes();
        if i + token_bytes.len() > bytes.len() || &bytes[i..i + token_bytes.len()] != token_bytes {
            return false;
        }
        i += token_bytes.len();

        if idx + 1 < tokens.len() {
            if i >= bytes.len() || !bytes[i].is_ascii_whitespace() {
                return false;
            }
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
        }
    }
    true
}

fn find_first_call_range(text: &str) -> Option<CoreRange> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_comment = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }
        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if in_string {
            i += 1;
            continue;
        }

        if b == b'(' {
            if let Some(close) = find_matching_paren_byte(text, i) {
                return Some(CoreRange {
                    start: byte_offset_to_position(text, i),
                    end: byte_offset_to_position(text, close + 1),
                });
            }
        }
        i += 1;
    }
    None
}

fn find_symbol_ranges(text: &str, symbol: &str) -> Vec<CoreRange> {
    if symbol.is_empty() {
        return Vec::new();
    }
    let bytes = text.as_bytes();
    let needle = symbol.as_bytes();
    if needle.len() > bytes.len() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_comment = false;
    while i + needle.len() <= bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }
        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if in_string {
            i += 1;
            continue;
        }

        if &bytes[i..i + needle.len()] == needle {
            let left_ok = i == 0 || !is_ident_char(bytes[i - 1] as char);
            let right_idx = i + needle.len();
            let right_ok = right_idx >= bytes.len() || !is_ident_char(bytes[right_idx] as char);
            if left_ok && right_ok {
                out.push(CoreRange {
                    start: byte_offset_to_position(text, i),
                    end: byte_offset_to_position(text, i + needle.len()),
                });
                i += needle.len();
                continue;
            }
        }
        i += 1;
    }

    out
}

fn extract_symbol_from_error(message: &str) -> Option<String> {
    for prefix in ["Undefined variable: ", "Variable '"] {
        if let Some(rest) = message.strip_prefix(prefix) {
            if prefix == "Variable '" {
                if let Some(end) = rest.find('\'') {
                    return Some(rest[..end].to_string());
                }
            } else {
                let token = rest.split_whitespace().next()?.trim();
                if !token.is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

pub fn extract_undefined_variable_name(message: &str) -> Option<String> {
    const PREFIX: &str = "Undefined variable: ";
    for line in message.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix(PREFIX) {
            let symbol = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c: char| matches!(c, ',' | ';' | ')' | '('));
            if !symbol.is_empty() {
                return Some(symbol.to_string());
            }
        }
    }
    None
}

fn damerau_levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let n = a_chars.len();
    let m = b_chars.len();
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }

    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 0..=n {
        dp[i][0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }

    for i in 1..=n {
        for j in 1..=m {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            let deletion = dp[i - 1][j] + 1;
            let insertion = dp[i][j - 1] + 1;
            let substitution = dp[i - 1][j - 1] + cost;
            let mut best = deletion.min(insertion).min(substitution);

            if
                i > 1 &&
                j > 1 &&
                a_chars[i - 1] == b_chars[j - 2] &&
                a_chars[i - 2] == b_chars[j - 1]
            {
                best = best.min(dp[i - 2][j - 2] + 1);
            }

            dp[i][j] = best;
        }
    }

    dp[n][m]
}

fn max_typo_distance_for_len(len: usize) -> usize {
    match len {
        0..=3 => 1,
        4..=7 => 2,
        8..=12 => 3,
        _ => 4,
    }
}

pub fn suggest_undefined_variable_candidates<'a, I>(
    message: &str,
    candidates: I,
    limit: usize
) -> Vec<String>
    where I: IntoIterator<Item = &'a str>
{
    let Some(missing) = extract_undefined_variable_name(message) else {
        return Vec::new();
    };
    if limit == 0 {
        return Vec::new();
    }

    let missing_lc = missing.to_ascii_lowercase();
    let missing_len = missing_lc.chars().count();
    let base_threshold = max_typo_distance_for_len(missing_len);
    let prefix_len = missing_lc.chars().take(2).count();
    let missing_prefix: String = missing_lc.chars().take(prefix_len).collect();

    let mut scored: Vec<(usize, usize, usize, String)> = Vec::new();
    let mut seen = HashSet::new();
    for cand in candidates {
        if cand.is_empty() || cand.starts_with('_') || cand == missing {
            continue;
        }
        let cand_lc = cand.to_ascii_lowercase();
        if !seen.insert(cand_lc.clone()) {
            continue;
        }

        let cand_len = cand_lc.chars().count();
        let distance = damerau_levenshtein_distance(&missing_lc, &cand_lc);
        let len_threshold = max_typo_distance_for_len(cand_len);
        let threshold = base_threshold.max(len_threshold);
        if distance > threshold {
            continue;
        }

        let prefix_penalty = if
            !missing_prefix.is_empty() && cand_lc.starts_with(&missing_prefix)
        {
            0
        } else {
            1
        };
        let len_diff = missing_len.abs_diff(cand_len);
        scored.push((distance, prefix_penalty, len_diff, cand.to_string()));
    }

    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.3.cmp(&b.3))
    });
    scored.into_iter().take(limit).map(|(_, _, _, name)| name).collect()
}

pub fn append_undefined_variable_suggestions<'a, I>(
    message: &str,
    candidates: I,
    limit: usize
) -> String
    where I: IntoIterator<Item = &'a str>
{
    if extract_undefined_variable_name(message).is_none() || message.contains("Did you mean:") {
        return message.to_string();
    }

    let suggestions = suggest_undefined_variable_candidates(message, candidates, limit);
    if suggestions.is_empty() {
        return message.to_string();
    }

    format!("{}\nDid you mean: {}", message.trim_end(), suggestions.join(", "))
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() ||
        matches!(ch, '_' | '-' | '/' | '?' | '!' | '*' | '+' | '<' | '>' | '=' | '.')
}

pub fn find_matching_paren_byte(text: &str, open_idx: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if *bytes.get(open_idx)? != b'(' {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut in_comment = false;
    let mut i = open_idx;

    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }
        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if in_string {
            i += 1;
            continue;
        }

        if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn find_enclosing_string_literal(text: &str, offset: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut in_comment = false;
    let mut in_string = false;
    let mut start = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }

        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }

        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            if !in_string {
                in_string = true;
                start = i;
            } else {
                let end = i + 1;
                if offset >= start && offset < end {
                    return Some((start, end));
                }
                in_string = false;
            }
        }

        i += 1;
    }

    if in_string && offset >= start {
        return Some((start, bytes.len()));
    }
    None
}

fn find_enclosing_char_literal(text: &str, offset: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut in_comment = false;
    let mut in_string = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }

        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }

        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            i += 1;
            continue;
        }

        if !in_string && b == b'\'' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    let end = i + 1;
                    if offset >= start && offset < end {
                        return Some((start, end));
                    }
                    i = end;
                    break;
                }
                if bytes[i] == b'\n' {
                    break;
                }
                i += 1;
            }
            continue;
        }

        i += 1;
    }
    None
}

fn numeric_token_at_offset(text: &str, offset: usize) -> Option<(String, usize, usize)> {
    if text.is_empty() {
        return None;
    }
    let bytes = text.as_bytes();
    let mut idx = offset.min(bytes.len().saturating_sub(1));

    if idx < bytes.len() && !is_numeric_token_byte(bytes[idx]) {
        if idx > 0 && is_numeric_token_byte(bytes[idx - 1]) {
            idx -= 1;
        } else {
            return None;
        }
    }

    let mut start = idx;
    while start > 0 && is_numeric_token_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = idx + 1;
    while end < bytes.len() && is_numeric_token_byte(bytes[end]) {
        end += 1;
    }

    let token = text.get(start..end)?.to_string();
    Some((token, start, end))
}

fn is_numeric_token_byte(b: u8) -> bool {
    b.is_ascii_digit() || b == b'.' || b == b'-'
}

fn is_int_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let start = if bytes[0] == b'-' { 1 } else { 0 };
    start < bytes.len() && bytes[start..].iter().all(|b| b.is_ascii_digit())
}

fn is_float_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let start = if bytes[0] == b'-' { 1 } else { 0 };
    if start >= bytes.len() {
        return false;
    }
    let slice = &bytes[start..];
    let dot_count = slice
        .iter()
        .filter(|&&b| b == b'.')
        .count();
    if dot_count != 1 {
        return false;
    }
    #[allow(unused_parens)]
    if !slice.iter().all(|b| (b.is_ascii_digit() || *b == b'.')) {
        return false;
    }
    let dot_idx = slice
        .iter()
        .position(|&b| b == b'.')
        .unwrap_or(0);
    let left = &slice[..dot_idx];
    let right = &slice[dot_idx + 1..];
    (!left.is_empty() || !right.is_empty()) &&
        left.iter().all(|b| b.is_ascii_digit()) &&
        right.iter().all(|b| b.is_ascii_digit())
}

fn is_symbol_char(ch: char) -> bool {
    !ch.is_whitespace() && !matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '"' | ';' | ',')
}
