use super::*;

/// Event wait state exposed to UI projections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WaitingReason {
    Model,
    Subagent,
    TaskOutput {
        task_ids: Vec<String>,
        subject: String,
    },
    TasksComplete,
    Sleep,
}

impl WaitingReason {
    pub fn label(&self) -> String {
        match self {
            Self::Model => "Waiting for response…".to_owned(),
            Self::Subagent => "Waiting on subagent…".to_owned(),
            Self::TaskOutput { subject, .. } if !subject.trim().is_empty() => {
                format!("{}…", clamp_wait_subject(subject))
            }
            Self::TaskOutput { .. } => "Waiting on task output…".to_owned(),
            Self::TasksComplete => "Waiting on tasks…".to_owned(),
            Self::Sleep => "Sleeping…".to_owned(),
        }
    }
}

fn clamp_wait_subject(subject: &str) -> String {
    const MAX_WAIT_SUBJECT_CHARS: usize = 40;
    let subject = subject.trim();
    if subject.chars().count() <= MAX_WAIT_SUBJECT_CHARS {
        subject.to_owned()
    } else {
        subject.chars().take(MAX_WAIT_SUBJECT_CHARS).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeKind {
    #[default]
    GrokNight,
    GrokDay,
    TokyoNight,
    RosePineMoon,
    OscuraMidnight,
    Auto,
    TerminalNative,
    AyuDark,
    AyuLight,
    AyuMirage,
    CatppuccinFrappe,
    CatppuccinLatte,
    CatppuccinMacchiato,
    CatppuccinMocha,
    Dracula,
    EverforestDark,
    EverforestLight,
    FlexokiDark,
    FlexokiLight,
    GithubDarkDimmed,
    GithubLight,
    GruvboxDark,
    GruvboxLight,
    KanagawaDragon,
    KanagawaLotus,
    KanagawaWave,
    LightOwl,
    MonokaiPro,
    Nord,
    OneDark,
    OneLight,
    Palenight,
    RosePine,
    RosePineDawn,
    SilkCircuitDawn,
    SilkCircuitGlow,
    SilkCircuitNeon,
    SilkCircuitSoft,
    SilkCircuitVibrant,
    SolarizedDark,
    SolarizedLight,
    TokyoNightMoon,
    TokyoNightStorm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolDisplayMode {
    Collapsed,
    Truncated,
    Expanded,
}
