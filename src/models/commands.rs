// SPDX-License-Identifier: Apache-2.0
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use bytes::Bytes;
use tokio::sync::mpsc::UnboundedReceiver;
use crate::models::types::{ManifestSize, MimeType};
use crate::pubsub::command::ChannelId;
use crate::registry::digest::Digest;
use crate::registry::repository::Repository;

pub const SHUTDOWN:&str = "shutdown";
pub const PERSIST_BLOB:&str = "persist_blob";
pub const PERSIST_MANIFEST:&str = "persist_manifest";

#[derive(Debug)]
pub enum RegistryCommand {
    Shutdown,
    PersistBlob(Repository, UnboundedReceiver<Bytes>),
    PersistManifest(Repository, Option<Digest>, ManifestSize, MimeType, UnboundedReceiver<Bytes>),
}

impl RegistryCommand {
    pub fn id(&self) -> String {
        match self {
            RegistryCommand::Shutdown => String::from(SHUTDOWN),
            RegistryCommand::PersistBlob(repo,_) => repo.reference.to_string(),
            RegistryCommand::PersistManifest(repo, _, _, _, _) => repo.reference.to_string(),
        }

    }

    pub fn topic(&self) -> String {
        match self {
            RegistryCommand::Shutdown => String::from(SHUTDOWN),
            RegistryCommand::PersistBlob(_,_) => String::from(PERSIST_BLOB),
            RegistryCommand::PersistManifest(_,_,_,_,_) => String::from(PERSIST_MANIFEST),
        }

    }

}

impl ChannelId for RegistryCommand {
    /// Allows to send specific commands to specific queues
    fn queue_id(&self) -> u64 {

        let mut hasher = DefaultHasher::new();

        // Hash the command ID
        let cmd_id = self.id();
        cmd_id.hash(&mut hasher);

        // Terminate the hashing
        hasher.finish()

    }

    fn topic_id(&self) -> String {
        self.topic()
    }
}