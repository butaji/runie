pub const DEFAULT_MAX_LINES: usize = 2000;
pub const DEFAULT_MAX_BYTES: usize = 50 * 1024;

#[derive(Debug, Clone)]
pub struct TruncationPolicy {
    pub max_lines: usize,
    pub max_bytes: usize,
}

impl Default for TruncationPolicy {
    fn default() -> Self {
        Self::from(&runie_core::config::TruncationSection::default())
    }
}

impl From<&runie_core::config::TruncationSection> for TruncationPolicy {
    fn from(section: &runie_core::config::TruncationSection) -> Self {
        Self {
            max_lines: if section.max_lines == 0 {
                DEFAULT_MAX_LINES
            } else {
                section.max_lines
            },
            max_bytes: if section.max_bytes == 0 {
                DEFAULT_MAX_BYTES
            } else {
                section.max_bytes
            },
        }
    }
}

/// Construct a `TruncationPolicy` from the core `TruncationSection`.
/// Zero values fall back to documented defaults.
pub fn policy_from_section(section: &runie_core::config::TruncationSection) -> TruncationPolicy {
    TruncationPolicy::from(section)
}

#[derive(Debug, Clone)]
pub struct TruncatedOutput {
    pub content: String,
    pub was_truncated: bool,
    pub total_lines: usize,
    pub total_bytes: usize,
    pub output_lines: usize,
    pub output_bytes: usize,
}

impl TruncatedOutput {
    pub fn full(content: String) -> Self {
        let lines = content.lines().count();
        let bytes = content.len();
        Self {
            content,
            was_truncated: false,
            total_lines: lines,
            total_bytes: bytes,
            output_lines: lines,
            output_bytes: bytes,
        }
    }
}

pub fn truncate_head(content: &str, policy: &TruncationPolicy) -> TruncatedOutput {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let total_bytes = content.len();

    if total_lines <= policy.max_lines && total_bytes <= policy.max_bytes {
        return TruncatedOutput::full(content.to_owned());
    }

    let mut output = Vec::new();
    let mut output_bytes = 0;

    for (i, line) in lines.iter().enumerate() {
        if i >= policy.max_lines {
            break;
        }
        let line_bytes = line.len() + 1;
        if output_bytes + line_bytes > policy.max_bytes {
            break;
        }
        output.push(*line);
        output_bytes += line_bytes;
    }

    let out_str = output.join("\n");
    TruncatedOutput {
        content: out_str.clone(),
        was_truncated: true,
        total_lines,
        total_bytes,
        output_lines: output.len(),
        output_bytes: out_str.len(),
    }
}

pub fn truncate_tail(content: &str, policy: &TruncationPolicy) -> TruncatedOutput {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    let total_bytes = content.len();

    if total_lines <= policy.max_lines && total_bytes <= policy.max_bytes {
        return TruncatedOutput::full(content.to_owned());
    }

    let mut output = Vec::new();
    let mut output_bytes = 0;

    for (_i, line) in lines.iter().enumerate().rev() {
        if output.len() >= policy.max_lines {
            break;
        }
        let line_bytes = line.len() + 1;
        if output_bytes + line_bytes > policy.max_bytes {
            break;
        }
        output.push(*line);
        output_bytes += line_bytes;
    }

    output.reverse();
    let out_str = output.join("\n");
    TruncatedOutput {
        content: out_str.clone(),
        was_truncated: true,
        total_lines,
        total_bytes,
        output_lines: output.len(),
        output_bytes: out_str.len(),
    }
}
