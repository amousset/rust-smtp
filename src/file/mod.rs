//! The file transport writes the emails to the given directory. The name of the file will be
//! `message_id.txt`.
//! It can be useful for testing purposes, or if you want to keep track of sent messages.
//!

use crate::file::error::FileResult;
use crate::Email;
use crate::Envelope;
use crate::Transport;
use serde_json;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

pub mod error;

/// Writes the content and the envelope information to a file
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FileTransport {
    path: PathBuf,
}

impl FileTransport {
    /// Creates a new transport to the given directory
    pub fn new<P: AsRef<Path>>(path: P) -> FileTransport {
        FileTransport {
            path: PathBuf::from(path.as_ref()),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct SerializableEmail {
    envelope: Envelope,
    message_id: String,
    message: String,
}

impl<'a> Transport<'a> for FileTransport {
    type Result = FileResult;

    fn send<E: Into<Email>>(&mut self, email: E) -> FileResult {
        let email = email.into();

        let message_id = email.message_id().to_string();
        let envelope = email.envelope().clone();

        let mut file = self.path.clone();
        file.push(format!("{}.json", message_id));

        let serialized = serde_json::to_string(&SerializableEmail {
            envelope,
            message_id,
            message: email.message_to_string()?,
        })?;

        File::create(file.as_path())?.write_all(serialized.as_bytes())?;
        Ok(())
    }
}
