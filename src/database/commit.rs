use std::fmt;

use time::OffsetDateTime;

use super::object::Object;

#[derive(Debug, Clone)]
pub struct Commit {
    tree: String,
    author: Author,
    message: String,
    oid: Option<String>,
}

impl Commit {
    pub fn new(tree: &str, author: Author, message: &str) -> Self {
        Self {
            tree: tree.to_owned(),
            author,
            message: message.to_owned(),
            oid: None,
        }
    }
}

impl Object for Commit {
    fn object_type(&self) -> &str {
        "commit"
    }

    fn content(&self) -> Vec<u8> {
        format!(
            "tree {}
author {}
committer {}

{}",
            self.tree, self.author, self.author, self.message
        )
        .as_bytes()
        .to_owned()
    }

    fn set_oid(&mut self, oid: String) {
        assert!(self.oid.is_none());
        self.oid = Some(oid);
    }

    fn get_oid(&self) -> Option<&str> {
        self.oid.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct Author {
    name: String,
    email: String,
    timestamp: OffsetDateTime,
}

impl Author {
    pub fn new(name: &str, email: &str, timestamp: OffsetDateTime) -> Self {
        Self {
            name: name.to_owned(),
            email: email.to_owned(),
            timestamp,
        }
    }
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} <{}> {} {}",
            self.name,
            self.email,
            self.timestamp.timestamp(),
            self.timestamp.offset().lazy_format("%z")
        )
    }
}
