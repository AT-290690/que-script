use crate::parser::Expression;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const PROJECT_CONFIG_FILE: &str = "que.toml";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectConfig {
    pub entry: Option<String>,
    pub deps: Vec<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedProjectConfig {
    pub path: PathBuf,
    pub root_dir: PathBuf,
    pub config: ProjectConfig,
}

pub fn parse_project_config(source: &str, label: &str) -> Result<ProjectConfig, String> {
    let mut cfg = ProjectConfig::default();
    let mut lines = source.lines().enumerate().peekable();
    let mut section: Option<String> = None;

    while let Some((line_no, raw_line)) = lines.next() {
        let line = strip_inline_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].trim();
            match name {
                "env" => section = Some(name.to_string()),
                other => {
                    return Err(format!(
                        "failed to parse project config '{}': unknown section '{}' on line {}",
                        label,
                        other,
                        line_no + 1
                    ));
                }
            }
            continue;
        }

        let Some((raw_key, raw_value)) = line.split_once('=') else {
            return Err(format!(
                "failed to parse project config '{}': line {} must be `key = value`",
                label,
                line_no + 1
            ));
        };

        let key = raw_key.trim();
        let mut value = raw_value.trim().to_string();
        match section.as_deref() {
            None => match key {
                "entry" => {
                    cfg.entry = Some(parse_quoted_string(&value, label, line_no + 1, "entry")?);
                }
                "deps" => {
                    while !array_literal_is_complete(&value) {
                        let Some((_, next_line)) = lines.next() else {
                            return Err(format!(
                                "failed to parse project config '{}': unterminated deps array",
                                label
                            ));
                        };
                        let next = strip_inline_comment(next_line).trim();
                        if !next.is_empty() {
                            if !value.ends_with('[') && !value.ends_with(',') {
                                value.push(' ');
                            }
                            value.push_str(next);
                        }
                    }
                    cfg.deps = parse_string_array(&value, label, line_no + 1, "deps")?;
                }
                other => {
                    return Err(format!(
                        "failed to parse project config '{}': unknown key '{}' on line {}",
                        label,
                        other,
                        line_no + 1
                    ));
                }
            },
            Some("env") => {
                cfg.env.insert(
                    key.to_string(),
                    parse_quoted_string(&value, label, line_no + 1, key)?,
                );
            }
            Some(other) => {
                return Err(format!(
                    "failed to parse project config '{}': unsupported section '{}'",
                    label,
                    other
                ));
            }
        }
    }

    Ok(cfg)
}

pub fn load_project_config_from_path(path: &Path) -> Result<LoadedProjectConfig, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read project config '{}': {}", path.display(), e))?;
    let config = parse_project_config(&raw, &path.display().to_string())?;
    let root_dir = path
        .parent()
        .map(Path::to_path_buf)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(LoadedProjectConfig {
        path: path.to_path_buf(),
        root_dir,
        config,
    })
}

pub fn discover_project_config(start_dir: &Path) -> Result<Option<LoadedProjectConfig>, String> {
    let mut current = match fs::canonicalize(start_dir) {
        Ok(path) => path,
        Err(_) => start_dir.to_path_buf(),
    };

    loop {
        let candidate = current.join(PROJECT_CONFIG_FILE);
        if candidate.is_file() {
            return load_project_config_from_path(&candidate).map(Some);
        }
        let Some(parent) = current.parent() else {
            break;
        };
        if parent == current {
            break;
        }
        current = parent.to_path_buf();
    }

    Ok(None)
}

pub fn parse_bundle_definitions(source: &str, label: &str) -> Result<Vec<Expression>, String> {
    let root = crate::parser::build(source)
        .map_err(|e| format!("failed to parse bundle '{}': {}", label, e))?;
    let defs = crate::baked::ast_to_definitions(root, label)?;
    for (idx, item) in defs.iter().enumerate() {
        let Expression::Apply(form) = item else {
            return Err(format!(
                "bundle '{}' must contain only top-level definitions; found non-definition at form {}: {}",
                label,
                idx,
                item.to_lisp()
            ));
        };
        if form.is_empty() {
            return Err(format!(
                "bundle '{}' must contain only top-level definitions; malformed form {}: {}",
                label,
                idx,
                item.to_lisp()
            ));
        }
        let Expression::Word(kw) = &form[0] else {
            return Err(format!(
                "bundle '{}' must contain only top-level definitions; malformed form {}: {}",
                label,
                idx,
                item.to_lisp()
            ));
        };
        if kw != "let" && kw != "letrec" && kw != "mut" && kw != "extern" && kw != "letype" {
            return Err(format!(
                "bundle '{}' must contain only top-level definitions; found '{}' at form {}",
                label,
                kw,
                idx
            ));
        }
    }
    Ok(defs)
}

pub fn load_bundle_definitions(
    base_dir: &Path,
    bundle_paths: &[String],
) -> Result<Vec<Expression>, String> {
    let mut out = Vec::new();
    for bundle_path in bundle_paths {
        let raw = Path::new(bundle_path);
        let resolved = if raw.is_absolute() {
            raw.to_path_buf()
        } else {
            base_dir.join(raw)
        };
        if resolved.extension().and_then(|e| e.to_str()) != Some("que") {
            return Err(format!("bundle '{}' must be a .que file", resolved.display()));
        }
        let source = fs::read_to_string(&resolved)
            .map_err(|e| format!("failed to read bundle '{}': {}", resolved.display(), e))?;
        let mut defs = parse_bundle_definitions(&source, &resolved.display().to_string())?;
        out.append(&mut defs);
    }
    Ok(out)
}

pub fn default_project_config_text() -> String {
    r#"entry = "main.que"
deps = []

[env]
QUE_WASM_OPT = "speed"
QUE_DEVIRTUALIZE = "aggressive"
QUE_TCO = "aggressive"
"#
    .to_string()
}

fn strip_inline_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..idx],
            _ => {}
        }
    }
    line
}

fn array_literal_is_complete(value: &str) -> bool {
    let mut in_string = false;
    let mut escaped = false;
    let mut depth = 0i32;
    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '[' if !in_string => depth += 1,
            ']' if !in_string => depth -= 1,
            _ => {}
        }
    }
    depth <= 0 && value.contains(']')
}

fn parse_quoted_string(value: &str, label: &str, line_no: usize, key: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if !(trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2) {
        return Err(format!(
            "failed to parse project config '{}': {} on line {} must be a quoted string",
            label,
            key,
            line_no
        ));
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let Some(next) = chars.next() else {
                return Err(format!(
                    "failed to parse project config '{}': invalid escape in {} on line {}",
                    label,
                    key,
                    line_no
                ));
            };
            match next {
                '\\' | '"' => out.push(next),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => {
                    return Err(format!(
                        "failed to parse project config '{}': unsupported escape '\\{}' in {} on line {}",
                        label,
                        other,
                        key,
                        line_no
                    ));
                }
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

fn parse_string_array(
    value: &str,
    label: &str,
    line_no: usize,
    key: &str,
) -> Result<Vec<String>, String> {
    let trimmed = value.trim();
    if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
        return Err(format!(
            "failed to parse project config '{}': {} on line {} must be a string array",
            label,
            key,
            line_no
        ));
    }
    let inner = trimmed[1..trimmed.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    let mut cursor = inner;
    while !cursor.trim_start().is_empty() {
        cursor = cursor.trim_start();
        if !cursor.starts_with('"') {
            return Err(format!(
                "failed to parse project config '{}': {} on line {} must contain only quoted strings",
                label,
                key,
                line_no
            ));
        }
        let end = find_string_end(cursor).ok_or_else(|| {
            format!(
                "failed to parse project config '{}': unterminated string in {} on line {}",
                label,
                key,
                line_no
            )
        })?;
        out.push(parse_quoted_string(&cursor[..=end], label, line_no, key)?);
        cursor = cursor[end + 1..].trim_start();
        if cursor.is_empty() {
            break;
        }
        let Some(rest) = cursor.strip_prefix(',') else {
            return Err(format!(
                "failed to parse project config '{}': {} on line {} must separate values with commas",
                label,
                key,
                line_no
            ));
        };
        cursor = rest;
    }

    Ok(out)
}

fn find_string_end(input: &str) -> Option<usize> {
    let mut escaped = false;
    for (idx, ch) in input.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return Some(idx),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        default_project_config_text, discover_project_config, load_bundle_definitions,
        parse_project_config, PROJECT_CONFIG_FILE,
    };
    use std::fs;

    #[test]
    fn parse_project_config_reads_entry_and_deps() {
        let cfg = parse_project_config(
            "entry = \"main.que\"\ndeps = [\"./utils.que\", \"./lib/math.que\"]\n[env]\nQUE_WASM_OPT = \"speed\"",
            "que.toml",
        )
        .expect("config should parse");
        assert_eq!(cfg.entry.as_deref(), Some("main.que"));
        assert_eq!(cfg.deps.len(), 2);
        assert_eq!(cfg.env.get("QUE_WASM_OPT").map(String::as_str), Some("speed"));
    }

    #[test]
    fn discover_project_config_walks_upward() {
        let base = std::env::temp_dir().join(format!(
            "que-project-config-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos()
        ));
        let nested = base.join("a").join("b");
        fs::create_dir_all(&nested).expect("temp dirs should exist");
        fs::write(
            base.join(PROJECT_CONFIG_FILE),
            "entry = \"main.que\"\ndeps = [\"./utils.que\"]",
        )
        .expect("config should be written");

        let loaded = discover_project_config(&nested)
            .expect("discovery should succeed")
            .expect("config should be found");
        assert_eq!(loaded.config.entry.as_deref(), Some("main.que"));
        assert_eq!(loaded.config.deps, vec!["./utils.que".to_string()]);
    }

    #[test]
    fn load_bundle_definitions_accepts_relative_bundle_from_project_root() {
        let base = std::env::temp_dir().join(format!(
            "que-project-bundle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos()
        ));
        let nested = base.join("utils");
        fs::create_dir_all(&nested).expect("temp dirs should exist");
        fs::write(nested.join("util.que"), "(let inc (lambda x (+ x 1)))")
            .expect("bundle should be written");
        let defs = load_bundle_definitions(&base, &["./utils/util.que".to_string()])
            .expect("bundle defs should load");
        assert_eq!(defs.len(), 1);
    }

    #[test]
    fn default_project_config_contains_entry() {
        let text = default_project_config_text();
        assert!(text.contains("entry = \"main.que\""));
        assert!(text.contains("deps = []"));
        assert!(text.contains("[env]"));
    }
}
