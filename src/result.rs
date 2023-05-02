#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    pub line: u32,
    pub column: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Position {
    pub begin: Mark,
    pub end: Mark,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Location {
    pub path: String,
    pub position: Position,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Issue {
    pub issue_text: String,
    pub issue_code: String,
    pub location: Location,
}
