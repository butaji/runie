use super::TopBarViewModel;

pub(crate) struct TopBarBuilder {
    repo: String,
    branch: String,
    path: String,
    context_window: usize,
    estimated_tokens: usize,
}

impl TopBarBuilder {
    pub(crate) fn new() -> Self {
        Self {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            context_window: 128_000,
            estimated_tokens: 0,
        }
    }

    pub(crate) fn repo(mut self, repo: &str) -> Self {
        self.repo = repo.to_string();
        self
    }

    pub(crate) fn branch(mut self, branch: &str) -> Self {
        self.branch = branch.to_string();
        self
    }

    pub(crate) fn path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }


    pub(crate) fn tokens(mut self, estimated: usize) -> Self {
        self.estimated_tokens = estimated;
        self
    }

    pub(crate) fn build(self) -> TopBarViewModel {
        TopBarViewModel {
            repo: self.repo,
            branch: self.branch,
            path: self.path,
            context_window: self.context_window,
            estimated_tokens: self.estimated_tokens,
            agent_running: false,
            braille_frame: 0,
        }
    }
}

impl Default for TopBarBuilder {
    fn default() -> Self {
        Self::new()
    }
}
