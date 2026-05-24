use keyring::Entry;

const SERVICE_NAME: &str = "runie";

pub fn store_key(provider: &str, api_key: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, provider)
        .map_err(|e| format!("Keyring error: {}", e))?;
    entry
        .set_password(api_key)
        .map_err(|e| format!("Failed to store key: {}", e))
}

pub fn get_key(provider: &str) -> Option<String> {
    let entry = Entry::new(SERVICE_NAME, provider).ok()?;
    entry.get_password().ok()
}
