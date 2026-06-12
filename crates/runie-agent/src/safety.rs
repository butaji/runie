pub fn check_bash_safety(command: &str) -> Option<&'static str> {
    let cmd = command.trim().to_lowercase();
    if cmd.contains("rm -rf /") || cmd.contains("rm -rf /*") || cmd.contains("rm -rf ~") {
        return Some("rm -rf on system directories or home is blocked");
    }
    if cmd.starts_with("dd ") && cmd.contains("of=/dev/") {
        return Some("dd writing to block devices is blocked");
    }
    if cmd.contains("> /dev/sda") || cmd.contains("> /dev/nvme") || cmd.contains("> /dev/hd") {
        return Some("writing directly to block devices is blocked");
    }
    if cmd.starts_with("mkfs") || cmd.starts_with("mkfs.") {
        return Some("mkfs is blocked");
    }
    if cmd.contains(":|:") && cmd.contains("};") {
        return Some("fork bombs are blocked");
    }
    if cmd.contains("chmod -r 777 /") || cmd.contains("chmod -r 000 /") {
        return Some("recursive chmod on root is blocked");
    }
    if cmd.starts_with("sudo ") && (cmd.contains(" rm ") || cmd.contains(" dd ")) {
        return Some("sudo with destructive commands is blocked");
    }
    None
}
