#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitConflictSummary {
    pub conflicted_paths: Vec<String>,
    pub recoverable: bool,
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
