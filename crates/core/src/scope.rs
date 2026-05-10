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

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "publish" => Some(Self::Publish),
            "read" => Some(Self::Read),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}
