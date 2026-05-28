use crate::provider_factory::validate_api_key;

#[test]
fn test_validate_garbage_api_key_rejected() {
    let key = "RUST_BACKTRACE=full cargo run -p runie-cli";
    let result = validate_api_key(key, "minimax");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("shell command"), "Expected 'shell command' in error: {}", err);
}

#[test]
fn test_validate_placeholder_rejected() {
    let key = "YOUR_MINIMAX_API_KEY_HERE";
    let result = validate_api_key(key, "minimax");
    assert!(result.is_err());
}

#[test]
fn test_validate_empty_rejected() {
    let key = "";
    let result = validate_api_key(key, "minimax");
    assert!(result.is_err());
}

#[test]
fn test_validate_short_rejected() {
    let key = "abc123";
    let result = validate_api_key(key, "minimax");
    assert!(result.is_err());
}

#[test]
fn test_validate_valid_key_accepted() {
    let key = "sk-minimax-valid-key-1234567890";
    let result = validate_api_key(key, "minimax");
    assert!(result.is_ok());
}
