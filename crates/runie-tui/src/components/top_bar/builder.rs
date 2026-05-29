use super::TopBarViewModel;

pub struct TopBarBuilder {
    repo: String,
    branch: String,
    path: String,
    model: String,
    context_window: usize,
    estimated_tokens: usize,
}

impl TopBarBuilder {
    pub fn new() -> Self {
        Self {
            repo: String::new(),
            branch: String::new(),
            path: String::new(),
            model: String::new(),
            context_window: 128_000,
            estimated_tokens: 0,
        }
    }

    pub fn repo(mut self, repo: &str) -> Self {
        self.repo = repo.to_string();
        self
    }

    pub fn branch(mut self, branch: &str) -> Self {
        self.branch = branch.to_string();
        self
    }

    pub fn path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }

    pub fn model(mut self, model: &str, context_window: usize) -> Self {
        self.model = model.to_string();
        self.context_window = context_window;
        self
    }

    pub fn tokens(mut self, estimated: usize) -> Self {
        self.estimated_tokens = estimated;
        self
    }

    pub fn build(self) -> TopBarViewModel {
        TopBarViewModel {
            repo: self.repo,
            branch: self.branch,
            path: self.path,
            model: self.model,
            context_window: self.context_window,
            estimated_tokens: self.estimated_tokens,
        }
    }
}

impl Default for TopBarBuilder {
    fn default() -> Self {
        Self::new()
    }
}
