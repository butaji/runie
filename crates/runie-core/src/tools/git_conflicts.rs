#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitConflictSummary {
    pub conflicted_paths: Vec<String>,
    pub recoverable: bool,
}

impl GitConflictSummary {
    pub fn terminal_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "Git conflicts: {} recoverable={}",
            self.conflicted_paths.len(),
            self.recoverable
        )];
        lines.extend(
            self.conflicted_paths
                .iter()
                .map(|path| format!("Conflict: {path}")),
        );
        lines
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitConflictAction {
    Inspect,
    Resolve,
    Abort,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitConflictRecoveryPlan {
    pub paths: Vec<String>,
    pub actions: Vec<GitConflictAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitConflictRecoveryStatus {
    Ready,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitConflictRecoveryState {
    pub plan: GitConflictRecoveryPlan,
    pub selected_path: Option<String>,
    pub selected_action: Option<GitConflictAction>,
    pub status: GitConflictRecoveryStatus,
}

impl GitConflictRecoveryState {
    /// Stable, data-only rows for TUI/JSON projections.
    pub fn terminal_lines(&self) -> Vec<String> {
        let mut lines = vec![format!("Git recovery: {:?}", self.status)];
        lines.push(format!("Conflicted paths: {}", self.plan.paths.len()));
        lines.extend(
            self.plan
                .paths
                .iter()
                .map(|path| format!("Conflict: {path}")),
        );
        lines.push(format!(
            "Selected path: {}",
            self.selected_path.as_deref().unwrap_or("none")
        ));
        lines.push(format!(
            "Selected action: {}",
            self.selected_action
                .as_ref()
                .map(|action| format!("{action:?}"))
                .as_deref()
                .unwrap_or("none")
        ));
        lines.push(format!("Allowed actions: {}", self.plan.actions.len()));
        lines
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum GitConflictRecoveryEvent {
    PathSelected { path: String },
    ActionSelected { action: GitConflictAction },
    Completed,
    Cancelled,
}

pub fn begin_conflict_recovery(plan: GitConflictRecoveryPlan) -> GitConflictRecoveryState {
    GitConflictRecoveryState {
        plan,
        selected_path: None,
        selected_action: None,
        status: GitConflictRecoveryStatus::Ready,
    }
}

pub fn reduce_conflict_recovery(
    mut state: GitConflictRecoveryState,
    event: GitConflictRecoveryEvent,
) -> Result<GitConflictRecoveryState, String> {
    if state.status != GitConflictRecoveryStatus::Ready {
        return Err("git conflict recovery is no longer active".into());
    }
    match event {
        GitConflictRecoveryEvent::PathSelected { path } => {
            if !state.plan.admits(&GitConflictAction::Inspect, Some(&path)) {
                return Err("selected path is not conflicted".into());
            }
            state.selected_path = Some(path);
        }
        GitConflictRecoveryEvent::ActionSelected { action } => {
            if !state.plan.admits(&action, state.selected_path.as_deref()) {
                return Err("selected conflict action is not allowed".into());
            }
            state.selected_action = Some(action);
        }
        GitConflictRecoveryEvent::Completed => {
            if state.selected_action.is_none() {
                return Err("cannot complete without an action".into());
            }
            state.status = GitConflictRecoveryStatus::Completed;
        }
        GitConflictRecoveryEvent::Cancelled => state.status = GitConflictRecoveryStatus::Cancelled,
    }
    Ok(state)
}

impl GitConflictRecoveryPlan {
    pub fn admits(&self, action: &GitConflictAction, path: Option<&str>) -> bool {
        if !self.actions.contains(action) {
            return false;
        }
        path.is_none_or(|path| self.paths.iter().any(|candidate| candidate == path))
    }
}

pub fn classify_conflicts(status: &str) -> GitConflictSummary {
    let conflicted_paths = status
        .lines()
        .filter_map(|line| {
            let bytes = line.as_bytes();
            (bytes.len() > 3 && is_conflict_code(bytes[0], bytes[1]))
                .then(|| line[3..].trim().to_owned())
        })
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    GitConflictSummary {
        recoverable: !conflicted_paths.is_empty(),
        conflicted_paths,
    }
}

pub fn plan_conflict_recovery(summary: &GitConflictSummary) -> GitConflictRecoveryPlan {
    let actions = summary
        .recoverable
        .then_some(vec![
            GitConflictAction::Inspect,
            GitConflictAction::Resolve,
            GitConflictAction::Abort,
        ])
        .unwrap_or_default();
    GitConflictRecoveryPlan {
        paths: summary.conflicted_paths.clone(),
        actions,
    }
}

fn is_conflict_code(index: u8, worktree: u8) -> bool {
    matches!(
        (index, worktree),
        (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D')
    )
}
