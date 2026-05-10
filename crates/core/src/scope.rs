use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Publish,
    Read,
    Delete,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Publish => "publish",
            Scope::Read => "read",
            Scope::Delete => "delete",
        }
    }
}

impl FromStr for Scope {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "publish" => Ok(Self::Publish),
            "read" => Ok(Self::Read),
            "delete" => Ok(Self::Delete),
            _ => Err(()),
        }
    }
}
