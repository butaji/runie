use std::path::PathBuf;
use std::process::Command;

fn workspace_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

#[test]
fn dev_sh_syntax_is_valid() {
    let root = workspace_root();
    let dev_sh = root.join("dev.sh");
    let output = Command::new("bash")
        .args(["-n", dev_sh.to_str().unwrap()])
        .output()
        .expect("bash available");
    assert!(
        output.status.success(),
        "dev.sh must have valid bash syntax"
    );
}

#[test]
fn dev_sh_uses_cargo_watch_for_hot_reload() {
    let root = workspace_root();
    let content = std::fs::read_to_string(root.join("dev.sh")).expect("dev.sh readable");
    assert!(
        content.contains("cargo watch"),
        "dev.sh must use cargo-watch"
    );
    assert!(
        content.contains("-x") && content.contains("run"),
        "dev.sh must run the binary via cargo watch"
    );
}

#[test]
fn dev_sh_executable_bit_set() {
    use std::os::unix::fs::PermissionsExt;
    let root = workspace_root();
    let meta = std::fs::metadata(root.join("dev.sh")).expect("dev.sh exists");
    let mode = meta.permissions().mode();
    assert!(mode & 0o111 != 0, "dev.sh must be executable");
}
