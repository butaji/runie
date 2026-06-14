pub fn check_bash_safety(command: &str) -> Option<&'static str> {
    let cmd = command.trim().to_lowercase();
    check_rm_rf(&cmd)
        .or_else(|| check_dd(&cmd))
        .or_else(|| check_block_write(&cmd))
        .or_else(|| check_mkfs(&cmd))
        .or_else(|| check_fork_bomb(&cmd))
        .or_else(|| check_chmod_root(&cmd))
        .or_else(|| check_sudo_destructive(&cmd))
}

fn check_rm_rf(cmd: &str) -> Option<&'static str> {
    if cmd.contains("rm -rf /") || cmd.contains("rm -rf /*") || cmd.contains("rm -rf ~") {
        Some("rm -rf on system directories or home is blocked")
    } else {
        None
    }
}

fn check_dd(cmd: &str) -> Option<&'static str> {
    if cmd.starts_with("dd ") && cmd.contains("of=/dev/") {
        Some("dd writing to block devices is blocked")
    } else {
        None
    }
}

fn check_block_write(cmd: &str) -> Option<&'static str> {
    if cmd.contains("> /dev/sda") || cmd.contains("> /dev/nvme") || cmd.contains("> /dev/hd") {
        Some("writing directly to block devices is blocked")
    } else {
        None
    }
}

fn check_mkfs(cmd: &str) -> Option<&'static str> {
    if cmd.starts_with("mkfs") || cmd.starts_with("mkfs.") {
        Some("mkfs is blocked")
    } else {
        None
    }
}

fn check_fork_bomb(cmd: &str) -> Option<&'static str> {
    if cmd.contains(":|:") && cmd.contains("};") {
        Some("fork bombs are blocked")
    } else {
        None
    }
}

fn check_chmod_root(cmd: &str) -> Option<&'static str> {
    if cmd.contains("chmod -r 777 /") || cmd.contains("chmod -r 000 /") {
        Some("recursive chmod on root is blocked")
    } else {
        None
    }
}

fn check_sudo_destructive(cmd: &str) -> Option<&'static str> {
    if cmd.starts_with("sudo ") && (cmd.contains(" rm ") || cmd.contains(" dd ")) {
        Some("sudo with destructive commands is blocked")
    } else {
        None
    }
}
