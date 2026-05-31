/// Convert tool name to present participle (-ing form)
pub fn to_present_participle(name: &str) -> String {
    match name {
        "bash" | "shell" => "Running",
        "read_file" | "view" => "Reading",
        "write_file" | "create" => "Writing",
        "edit" | "str_replace" => "Editing",
        "search" | "grep" => "Searching",
        "list" | "ls" => "Listing",
        "delete" | "rm" => "Deleting",
        "git" => "Git",
        "test" => "Testing",
        "build" => "Building",
        "run" => "Running",
        "copy" => "Copying",
        "move" | "mv" => "Moving",
        _ => "Running",
    }
    .to_string()
}

/// Convert tool name to past tense (-ed form)
pub fn to_past_tense(name: &str) -> String {
    match name {
        "bash" | "shell" => "Ran",
        "read_file" | "view" => "Read",
        "write_file" | "create" => "Wrote",
        "edit" | "str_replace" => "Edited",
        "search" | "grep" => "Searched",
        "list" | "ls" => "Listed",
        "delete" | "rm" => "Deleted",
        "git" => "Git",
        "test" => "Tested",
        "build" => "Built",
        "run" => "Ran",
        "copy" => "Copied",
        "move" | "mv" => "Moved",
        _ => "Ran",
    }
    .to_string()
}
