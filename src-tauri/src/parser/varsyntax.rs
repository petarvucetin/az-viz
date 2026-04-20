//! Recognize shell-style variable assignments and `$NAME` references
//! inside command tokens. This is intentionally a small subset — no
//! `${NAME}`, no `${NAME:-default}`, no nested expansions.

use crate::model::{VarBody, VarOrigin, Variable};

/// Checks that `name` is a syntactically valid variable identifier:
/// letter/underscore followed by letters/digits/underscores.
pub fn is_valid_name(name: &str) -> bool {
    let mut it = name.chars();
    match it.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    it.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// If `line` begins with `NAME=...` (POSIX-ish assignment), split into
/// `(name, rhs)`. The RHS keeps its leading/trailing whitespace trimmed.
pub fn split_assignment(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    let eq = trimmed.find('=')?;
    if eq == 0 { return None; }
    let (name, rest) = trimmed.split_at(eq);
    let rhs = &rest[1..];
    if !is_valid_name(name) { return None; }
    Some((name.to_string(), rhs.trim().to_string()))
}

/// Parse a right-hand side into a `VarBody`.
///
/// `$(az ...)` → Command with argv (includes the leading `az` token).
/// `az ...`    → Command (user may omit the `$(...)` wrapper).
/// anything else → Literal value (outer single/double quotes stripped).
pub fn body_from_rhs(rhs: &str) -> VarBody {
    let rhs = rhs.trim();
    if rhs.is_empty() { return VarBody::Unset; }

    // $(az ...) form
    if let Some(inner) = rhs.strip_prefix("$(").and_then(|s| s.strip_suffix(')')) {
        let inner = inner.trim();
        if inner.starts_with("az ") || inner == "az" {
            return tokens_to_command(inner);
        }
        // $() of something other than `az` isn't supported; treat as literal.
        return VarBody::Literal { value: rhs.to_string() };
    }
    // Bare `az ...` (unwrapped shell form).
    if rhs.starts_with("az ") || rhs == "az" {
        return tokens_to_command(rhs);
    }
    // Literal — strip outer matching quotes if present.
    let stripped = strip_matched_quotes(rhs);
    VarBody::Literal { value: stripped.to_string() }
}

fn tokens_to_command(az_line: &str) -> VarBody {
    match crate::parser::tokenize::tokenize(az_line) {
        Ok(tokens) => {
            let argv: Vec<String> = std::iter::once("az".to_string()).chain(tokens).collect();
            VarBody::Command { argv }
        }
        // Fallback: store as a whitespace-split argv so the user can still
        // edit it; resolution will likely fail but won't crash.
        Err(_) => {
            let argv: Vec<String> = az_line.split_whitespace().map(|s| s.to_string()).collect();
            VarBody::Command { argv }
        }
    }
}

fn strip_matched_quotes(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

/// Extract every `$NAME` reference that appears anywhere in `token`.
/// `$` must be followed by a valid identifier start character; if not, the
/// `$` is treated as a literal (no match).
pub fn scan_var_refs(token: &str) -> Vec<String> {
    let bytes = token.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
            { end += 1; }
            if end > start {
                // Must start with letter or underscore.
                let first = bytes[start];
                if first.is_ascii_alphabetic() || first == b'_' {
                    // Safe: ASCII range only.
                    out.push(std::str::from_utf8(&bytes[start..end]).unwrap().to_string());
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    out
}

/// Substitute `$NAME` occurrences in `token` using `resolve`. Unresolved
/// names are left in place (so the consumer can detect missing values).
pub fn substitute(token: &str, resolve: &dyn Fn(&str) -> Option<String>) -> String {
    let bytes = token.as_bytes();
    let mut out = String::with_capacity(token.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
            { end += 1; }
            if end > start && (bytes[start].is_ascii_alphabetic() || bytes[start] == b'_') {
                let name = std::str::from_utf8(&bytes[start..end]).unwrap();
                match resolve(name) {
                    Some(v) => out.push_str(&v),
                    None => out.push_str(&token[i..end]),
                }
                i = end;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Given an assignment `NAME=RHS`, produce a `Variable` with `Declared` origin.
pub fn variable_from_assignment(name: String, rhs: &str) -> Variable {
    Variable {
        name,
        body: body_from_rhs(rhs),
        origin: VarOrigin::Declared,
        resolved: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_assignment_basic() {
        let (n, r) = split_assignment("FOO=bar").unwrap();
        assert_eq!(n, "FOO"); assert_eq!(r, "bar");
    }
    #[test]
    fn split_assignment_trims_rhs() {
        let (n, r) = split_assignment("  FOO=  hello world  ").unwrap();
        assert_eq!(n, "FOO"); assert_eq!(r, "hello world");
    }
    #[test]
    fn split_assignment_rejects_non_identifier() {
        assert!(split_assignment("9A=bar").is_none());
        assert!(split_assignment("=bar").is_none());
        assert!(split_assignment("no equals").is_none());
    }

    #[test]
    fn body_from_rhs_command_subshell() {
        let b = body_from_rhs("$(az network vnet show -g rg -n v --query id -o tsv)");
        match b {
            VarBody::Command { argv } => assert_eq!(argv[0], "az"),
            _ => panic!("expected Command"),
        }
    }
    #[test]
    fn body_from_rhs_literal_with_quotes() {
        assert!(matches!(body_from_rhs("\"hello\""), VarBody::Literal { value } if value == "hello"));
    }

    #[test]
    fn scan_refs_inside_complex_token() {
        let t = "[{private-ip-allocation-method:'Dynamic',id:'$SUBNET_ID'}]";
        assert_eq!(scan_var_refs(t), vec!["SUBNET_ID"]);
    }
    #[test]
    fn scan_refs_multi() {
        assert_eq!(scan_var_refs("$A$B"), vec!["A", "B"]);
    }
    #[test]
    fn scan_refs_ignores_trailing_dollar() {
        assert!(scan_var_refs("price$").is_empty());
    }

    #[test]
    fn substitute_replaces_when_resolved() {
        let map = |n: &str| if n == "X" { Some("42".to_string()) } else { None };
        assert_eq!(substitute("id=$X;y=$Y", &map), "id=42;y=$Y");
    }
}
