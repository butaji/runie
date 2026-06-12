pub const DEFAULT_MAX_LINES: usize = 2000;
pub const DEFAULT_MAX_BYTES: usize = 50 * 1024;

#[derive(Debug, Clone)]
pub struct TruncationPolicy {
    pub max_lines: usize,
    pub max_bytes: usize,
}

impl Default for TruncationPolicy {
    fn default() -> Self {
        Self::from(&TruncationConfig::default())
    }
}

impl From<&TruncationConfig> for TruncationPolicy {
    fn from(c: &TruncationConfig) -> Self {
        Self {
            max_lines: if c.max_lines == 0 { DEFAULT_MAX_LINES } else { c.max_lines },
            max_bytes: if c.max_bytes == 0 { DEFAULT_MAX_BYTES } else { c.max_bytes },
        }
    }
}

/// Truncation settings, parsed from the `[truncation]` section of
/// `config.toml`. Missing fields fall back to the documented defaults.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct TruncationConfig {
    pub max_lines: usize,
    pub max_bytes: usize,
}

impl Default for TruncationConfig {
    fn default() -> Self {
        Self {
            max_lines: DEFAULT_MAX_LINES,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }
}

/// Construct a `TruncationPolicy` from the core `TruncationSection` (the
/// type used by `runie-core::config_reload::TruncationSection`). This lets
/// the binary wire the parsed config into the agent without a circular
/// dependency between `runie-core` and `runie-agent`.
pub fn policy_from_section(max_lines: usize, max_bytes: usize) -> TruncationPolicy {
    TruncationPolicy {
        max_lines: if max_lines == 0 { DEFAULT_MAX_LINES } else { max_lines },
        max_bytes: if max_bytes == 0 { DEFAULT_MAX_BYTES } else { max_bytes },
    }
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
        return TruncatedOutput::full(content.to_string());
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
        return TruncatedOutput::full(content.to_string());
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
