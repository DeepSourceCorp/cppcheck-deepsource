use crate::result::Mark;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub file: String,
    pub begin: Mark,
    pub end: Mark,
}
