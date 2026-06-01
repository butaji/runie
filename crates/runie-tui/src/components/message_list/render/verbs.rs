use phf::{phf_map, Map};

static PRESENT: Map<&'static str, &'static str> = phf_map! {
    "bash" => "Running",
    "shell" => "Running",
    "read_file" => "Reading",
    "view" => "Reading",
    "write_file" => "Writing",
    "create" => "Writing",
    "edit" => "Editing",
    "str_replace" => "Editing",
    "search" => "Searching",
    "grep" => "Searching",
    "list" => "Listing",
    "ls" => "Listing",
    "delete" => "Deleting",
    "rm" => "Deleting",
    "git" => "Git",
    "test" => "Testing",
    "build" => "Building",
    "run" => "Running",
    "copy" => "Copying",
    "move" => "Moving",
    "mv" => "Moving",
};

static PAST: Map<&'static str, &'static str> = phf_map! {
    "bash" => "Ran",
    "shell" => "Ran",
    "read_file" => "Read",
    "view" => "Read",
    "write_file" => "Wrote",
    "create" => "Wrote",
    "edit" => "Edited",
    "str_replace" => "Edited",
    "search" => "Searched",
    "grep" => "Searched",
    "list" => "Listed",
    "ls" => "Listed",
    "delete" => "Deleted",
    "rm" => "Deleted",
    "git" => "Git",
    "test" => "Tested",
    "build" => "Built",
    "run" => "Ran",
    "copy" => "Copied",
    "move" => "Moved",
    "mv" => "Moved",
};

/// Convert tool name to present participle (-ing form)
pub fn to_present_participle(name: &str) -> String {
    PRESENT.get(name).copied().unwrap_or("Running").to_string()
}

/// Convert tool name to past tense (-ed form)
pub fn to_past_tense(name: &str) -> String {
    PAST.get(name).copied().unwrap_or("Ran").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn present_participle_known() {
        assert_eq!(to_present_participle("bash"), "Running");
        assert_eq!(to_present_participle("read_file"), "Reading");
        assert_eq!(to_present_participle("edit"), "Editing");
    }

    #[test]
    fn past_tense_known() {
        assert_eq!(to_past_tense("bash"), "Ran");
        assert_eq!(to_past_tense("read_file"), "Read");
        assert_eq!(to_past_tense("edit"), "Edited");
    }

    #[test]
    fn unknown_falls_back() {
        assert_eq!(to_present_participle("mystery"), "Running");
        assert_eq!(to_past_tense("mystery"), "Ran");
    }
}
