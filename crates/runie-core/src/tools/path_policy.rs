//! Pure workspace path policy shared by filesystem tools.

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathOperation {
    Read,
    Write,
    Search,
}

pub fn validate(path: &str, operation: PathOperation) -> Result<PathBuf, String> {
    let raw = Path::new(path);
    if raw
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("path traversal is not allowed".into());
    }
    let name = raw
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if sensitive_name(name) {
        return Err(format!("refusing access to sensitive path {path:?}"));
    }
    if matches!(operation, PathOperation::Write) && raw.is_absolute() {
        return Err("writes must stay in the working directory".into());
    }
    Ok(raw.to_path_buf())
}

fn sensitive_name(name: &str) -> bool {
    name == ".env" || name.starts_with(".env.") || name.ends_with(".pem") || name == "id_rsa"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_and_sensitive_files() {
        assert!(validate("../secret", PathOperation::Read).is_err());
        assert!(validate(".env", PathOperation::Read).is_err());
        assert!(validate("keys.pem", PathOperation::Read).is_err());
    }

    #[test]
    fn allows_workspace_reads_but_not_absolute_writes() {
        assert_eq!(
            validate("src/lib.rs", PathOperation::Read).unwrap(),
            PathBuf::from("src/lib.rs")
        );
        assert!(validate("/tmp/file", PathOperation::Write).is_err());
    }
}
