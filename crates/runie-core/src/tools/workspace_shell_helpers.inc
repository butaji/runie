fn complete_shell_text(
    stdout: &[u8],
    stderr: &[u8],
    update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
) -> String {
    let mut text = String::from_utf8_lossy(stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(stderr));
    if let Some(update) = update {
        update(serde_json::json!({"text": text, "complete": true}));
    }
    text
}
