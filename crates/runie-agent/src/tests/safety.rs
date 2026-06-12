use crate::safety::check_bash_safety;

#[test]
fn test_bash_safety_rm_rf_root() {
    assert!(check_bash_safety("rm -rf /").is_some());
    assert!(check_bash_safety("rm -rf /*").is_some());
}

#[test]
fn test_bash_safety_rm_rf_home() {
    assert!(check_bash_safety("rm -rf ~").is_some());
}

#[test]
fn test_bash_safety_dd_block_device() {
    assert!(check_bash_safety("dd if=/dev/zero of=/dev/sda").is_some());
}

#[test]
fn test_bash_safety_mkfs() {
    assert!(check_bash_safety("mkfs.ext4 /dev/sda1").is_some());
}

#[test]
fn test_bash_safety_fork_bomb() {
    assert!(check_bash_safety(":(){ :|:& };:").is_some());
}

#[test]
fn test_bash_safety_dangerous_chmod() {
    assert!(check_bash_safety("chmod -R 777 /").is_some());
}

#[test]
fn test_bash_safety_sudo_rm() {
    assert!(check_bash_safety("sudo rm -rf / important").is_some());
}

#[test]
fn test_bash_safety_safe_commands() {
    assert!(check_bash_safety("echo hello").is_none());
    assert!(check_bash_safety("ls -la").is_none());
    assert!(check_bash_safety("cat file.txt").is_none());
    assert!(check_bash_safety("git status").is_none());
}
