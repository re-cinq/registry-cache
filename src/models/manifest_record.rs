use crate::models::types::MimeType;
use crate::registry::digest::Digest;

/// ManifestRecord keeps an index between the container image manifest tag and its reference
pub struct ManifestRecord {
    pub name: String,
    pub tag: String,
    pub reference: Option<Digest>,
    pub size: i32,
    pub mime: MimeType,
}

impl ManifestRecord {
    pub fn new(name: String, tag: String, reference: Option<Digest>, size: i32, mime: MimeType) -> ManifestRecord {
        ManifestRecord {
            name,
            tag,
            reference,
            size,
            mime
        }
    }

    // /// Whether we do have a reference in the record
    // pub fn is_present(&self) -> bool {
    //     self.reference.is_some()
    // }
}