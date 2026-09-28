use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

/// The role of a message, following the OpenAI Chat conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// The content of a message.
#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    Text(String),
}

impl Serialize for Content {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Content::Text(text) => serializer.serialize_str(text),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub role: Role,
    pub content: Content,
}

impl Message {
    /// Constructs a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Content::Text(content.into()),
        }
    }

    /// Constructs a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: Content::Text(content.into()),
        }
    }

    /// Constructs an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: Content::Text(content.into()),
        }
    }
}

impl Serialize for Message {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Message", 2)?;
        s.serialize_field("role", &self.role)?;
        s.serialize_field("content", &self.content)?;
        s.end()
    }
}
