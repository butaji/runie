/// Read-only name lookup shared by normalized tool records and legacy test
/// fixtures. Projection code depends on this narrow data contract rather than
/// rebuilding a second name index.
pub trait ToolNameLookup {
    fn tool_name(&self, id: &str) -> Option<&str>;
}

impl ToolNameLookup for HashMap<String, String> {
    fn tool_name(&self, id: &str) -> Option<&str> {
        self.get(id).map(String::as_str)
    }
}

impl ToolNameLookup for HashMap<String, ToolRecord> {
    fn tool_name(&self, id: &str) -> Option<&str> {
        self.get(id).and_then(|record| record.name.as_deref())
    }
}
