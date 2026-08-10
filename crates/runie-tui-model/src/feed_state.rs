#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedState {
    pub lines: Vec<Line>,
    pub navigation: FeedNavigation,
}
