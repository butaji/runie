//! Deny path lists for sandbox enforcement.
//!
//! Provides `ReadDenyList` and `WriteDenyList` for path-based access control.
//! Paths can be specified as exact matches or regex patterns.

use std::path::{Path, PathBuf};
use regex::Regex;

use super::profiles::Profile;

/// A deny list for file read operations.
#[derive(Debug, Clone, Default)]
pub struct ReadDenyList {
    /// Exact path matches to deny.
    exact: Vec<PathBuf>,
    /// Regex patterns to deny.
    patterns: Vec<Regex>,
    /// Profile-specific default denies.
    profile_defaults: bool,
}

impl ReadDenyList {
    /// Create an empty deny list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from a profile.
    pub fn from_profile(profile: Profile) -> Self {
        let mut list = Self::default();
        list.add_profile_defaults(profile);
        list
    }

    /// Add an exact path to deny.
    pub fn add_exact(&mut self, path: impl Into<PathBuf>) {
        self.exact.push(path.into());
    }

    /// Add a regex pattern to deny.
    pub fn add_pattern(&mut self, pattern: &str) {
        if let Ok(p) = Regex::new(pattern) {
            self.patterns.push(p);
        }
    }

    /// Add profile-specific default denies.
    fn add_profile_defaults(&mut self, profile: Profile) {
        self.profile_defaults = true;

        match profile {
            Profile::Strict => {
                // Strict mode denies sensitive system paths for reading
                self.add_exact("/etc/shadow");
                self.add_exact("/etc/sudoers");
                self.add_pattern("/root/.*");
                self.add_pattern(".*\\.pem$");
                self.add_pattern(".*\\.key$");
            }
            Profile::Workspace => {
                // Workspace mode is permissive for reads
            }
            Profile::Devbox | Profile::Custom | Profile::Off => {
                // No default read denies
            }
        }
    }

    /// Check if a path should be denied for reading.
    pub fn is_denied(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check exact matches
        for exact in &self.exact {
            let exact_str = Self::exact_match_str(exact);
            if path == exact {
                return true;
            }
            // Check if path is a child directory of an exact match (e.g., /etc/shadow/foo under /etc/shadow)
            if path_str.starts_with(&format!("{}/", exact_str)) {
                return true;
            }
            // Check if path is a same-named file with extension (e.g., /etc/shadow.bak under /etc/shadow)
            // This catches backup files and variants of the denied path
            if path_str.starts_with(&format!("{}.", exact_str)) {
                return true;
            }
        }

        // Check regex patterns
        for pattern in &self.patterns {
            if pattern.is_match(&path_str) {
                return true;
            }
        }

        false
    }

    /// Helper to get string from path for exact matches.
    fn exact_match_str(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    /// Get the number of deny entries.
    pub fn len(&self) -> usize {
        self.exact.len() + self.patterns.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A deny list for file write operations.
#[derive(Debug, Clone, Default)]
pub struct WriteDenyList {
    /// Exact path matches to deny.
    exact: Vec<PathBuf>,
    /// Regex patterns to deny.
    patterns: Vec<Regex>,
    /// Profile-specific default denies.
    profile_defaults: bool,
}

impl WriteDenyList {
    /// Create an empty deny list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from a profile.
    pub fn from_profile(profile: Profile) -> Self {
        let mut list = Self::default();
        list.add_profile_defaults(profile);
        list
    }

    /// Add an exact path to deny.
    pub fn add_exact(&mut self, path: impl Into<PathBuf>) {
        self.exact.push(path.into());
    }

    /// Add a regex pattern to deny.
    pub fn add_pattern(&mut self, pattern: &str) {
        if let Ok(p) = Regex::new(pattern) {
            self.patterns.push(p);
        }
    }

    /// Add profile-specific default denies.
    fn add_profile_defaults(&mut self, profile: Profile) {
        self.profile_defaults = true;

        match profile {
            Profile::Strict => {
                // Strict mode denies writes to most places except workspace
                self.add_exact("/etc");
                self.add_exact("/usr");
                self.add_exact("/bin");
                self.add_exact("/sbin");
                self.add_exact("/lib");
                self.add_exact("/System");
                self.add_pattern("/root/.*");
            }
            Profile::Workspace => {
                // Workspace mode denies writes outside workspace
                self.add_exact("/etc");
                self.add_exact("/usr");
                self.add_exact("/bin");
                self.add_exact("/sbin");
                self.add_exact("/var");
                self.add_exact("/opt");
            }
            Profile::Devbox => {
                // Devbox mode denies writes to /data
                self.add_exact("/data");
            }
            Profile::Custom | Profile::Off => {
                // No default write denies
            }
        }
    }

    /// Check if a path should be denied for writing.
    pub fn is_denied(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Check exact matches
        for exact in &self.exact {
            if path == exact || path.starts_with(exact) {
                return true;
            }
        }

        // Check regex patterns
        for pattern in &self.patterns {
            if pattern.is_match(&path_str) {
                return true;
            }
        }

        false
    }

    /// Get the number of deny entries.
    pub fn len(&self) -> usize {
        self.exact.len() + self.patterns.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Combined deny list for both read and write operations.
#[derive(Debug, Clone, Default)]
pub struct DenyList {
    /// Read deny list.
    pub read: ReadDenyList,
    /// Write deny list.
    pub write: WriteDenyList,
}

impl DenyList {
    /// Create from a profile.
    pub fn from_profile(profile: Profile) -> Self {
        Self {
            read: ReadDenyList::from_profile(profile),
            write: WriteDenyList::from_profile(profile),
        }
    }

    /// Check if a path should be denied for any operation.
    pub fn is_denied(&self, path: &Path, write: bool) -> bool {
        if write {
            self.read.is_denied(path) || self.write.is_denied(path)
        } else {
            self.read.is_denied(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_deny_exact() {
        let mut list = ReadDenyList::new();
        list.add_exact("/etc/shadow");

        assert!(list.is_denied(Path::new("/etc/shadow")));
        assert!(list.is_denied(Path::new("/etc/shadow.bak")));
        assert!(!list.is_denied(Path::new("/etc/passwd")));
    }

    #[test]
    fn read_deny_pattern() {
        let mut list = ReadDenyList::new();
        list.add_pattern(".*\\.key$");
        list.add_pattern("/root/.*");

        assert!(list.is_denied(Path::new("/home/user/.ssh/id_rsa.key")));
        assert!(list.is_denied(Path::new("/root/.bashrc")));
        assert!(!list.is_denied(Path::new("/home/user/script.sh")));
    }

    #[test]
    fn read_deny_strict_profile() {
        let list = ReadDenyList::from_profile(Profile::Strict);
        assert!(!list.is_empty());
        assert!(list.is_denied(Path::new("/etc/shadow")));
    }

    #[test]
    fn read_deny_workspace_profile() {
        let list = ReadDenyList::from_profile(Profile::Workspace);
        // Workspace is permissive for reads
        assert!(list.is_empty());
    }

    #[test]
    fn write_deny_exact() {
        let mut list = WriteDenyList::new();
        list.add_exact("/etc");
        list.add_exact("/usr");

        assert!(list.is_denied(Path::new("/etc")));
        assert!(list.is_denied(Path::new("/etc/passwd")));
        assert!(list.is_denied(Path::new("/usr/local/bin")));
        assert!(!list.is_denied(Path::new("/tmp")));
    }

    #[test]
    fn write_deny_strict_profile() {
        let list = WriteDenyList::from_profile(Profile::Strict);
        assert!(!list.is_empty());
        assert!(list.is_denied(Path::new("/etc")));
        assert!(list.is_denied(Path::new("/usr")));
    }

    #[test]
    fn write_deny_devbox_profile() {
        let list = WriteDenyList::from_profile(Profile::Devbox);
        assert!(list.is_denied(Path::new("/data")));
        assert!(list.is_denied(Path::new("/data/project")));
        assert!(!list.is_denied(Path::new("/workspace")));
    }

    #[test]
    fn write_deny_workspace_profile() {
        let list = WriteDenyList::from_profile(Profile::Workspace);
        assert!(list.is_denied(Path::new("/etc")));
        assert!(list.is_denied(Path::new("/var")));
        assert!(list.is_denied(Path::new("/opt")));
    }

    #[test]
    fn deny_list_combined() {
        let list = DenyList::from_profile(Profile::Strict);

        // Read checks
        assert!(list.is_denied(Path::new("/etc/shadow"), false));

        // Write checks
        assert!(list.is_denied(Path::new("/etc/passwd"), true));
    }
}
