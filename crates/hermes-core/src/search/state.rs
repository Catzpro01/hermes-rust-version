#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchState {
    Ready,
    Unavailable,
    Corrupt(String),
    NotReady,
}
