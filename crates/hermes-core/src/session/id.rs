use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);
impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}
impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl FromStr for SessionId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}
