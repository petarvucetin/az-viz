#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TokenizeError {
    #[error("empty input")]
    Empty,
    #[error("expected 'az' as first token")]
    MissingAz,
    #[error("expected 'network' as second token")]
    MissingNetwork,
    #[error("shell tokenization failed: {0}")]
    Shell(String),
}

/// Tokenizes a line and verifies it begins with `az network`.
/// Returns the tokens **after** `az` (i.e., starting with `network`).
pub fn tokenize(line: &str) -> Result<Vec<String>, TokenizeError> {
    let trimmed = line.trim();
    if trimmed.is_empty() { return Err(TokenizeError::Empty); }
    let joined = trimmed.replace("\\\n", " ").replace("\\\r\n", " ");
    let tokens = shell_words::split(&joined).map_err(|e| TokenizeError::Shell(e.to_string()))?;
    if tokens.first().map(|s| s.as_str()) != Some("az") {
        return Err(TokenizeError::MissingAz);
    }
    if tokens.get(1).map(|s| s.as_str()) != Some("network") {
        return Err(TokenizeError::MissingNetwork);
    }
    Ok(tokens.into_iter().skip(1).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_basic_command() {
        let toks = tokenize("az network vnet create --name v --resource-group rg").unwrap();
        assert_eq!(toks, ["network","vnet","create","--name","v","--resource-group","rg"]);
    }

    #[test]
    fn supports_quoted_values_with_spaces() {
        let toks = tokenize(r#"az network vnet create --name "my vnet" --resource-group rg"#).unwrap();
        assert_eq!(toks[3], "--name");
        assert_eq!(toks[4], "my vnet");
    }

    #[test]
    fn supports_line_continuations() {
        let input = "az network vnet create \\\n  --name v \\\n  --resource-group rg";
        let toks = tokenize(input).unwrap();
        assert!(toks.contains(&"--name".to_string()));
        assert!(toks.contains(&"v".to_string()));
    }

    #[test]
    fn rejects_non_az() {
        assert_eq!(tokenize("pwsh script.ps1").unwrap_err(), TokenizeError::MissingAz);
    }

    #[test]
    fn rejects_non_network() {
        assert_eq!(tokenize("az group create").unwrap_err(), TokenizeError::MissingNetwork);
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(tokenize("   ").unwrap_err(), TokenizeError::Empty);
    }
}
