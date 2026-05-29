use super::{StatusBar, StatusItem, BackgroundJob, JobStatus};
use crate::theme::ThemeWrapper;

pub struct StatusBarBuilder {
    items: Vec<StatusItem>,
    jobs: Vec<BackgroundJob>,
    theme: ThemeWrapper,
}

impl StatusBarBuilder {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            jobs: Vec::new(),
            theme: ThemeWrapper::default(),
        }
    }

    pub fn hotkey(mut self, key: &str, desc: &str) -> Self {
        self.items.push(StatusItem {
            key: key.to_string(),
            description: desc.to_string(),
        });
        self
    }

    pub fn job(mut self, name: &str, status: JobStatus) -> Self {
        self.jobs.push(BackgroundJob {
            name: name.to_string(),
            status,
        });
        self
    }

    pub fn build(self) -> StatusBar {
        StatusBar {
            items: self.items,
            theme: self.theme,
            background_jobs: self.jobs,
        }
    }
}

impl Default for StatusBarBuilder {
    fn default() -> Self {
        Self::new()
    }
}
