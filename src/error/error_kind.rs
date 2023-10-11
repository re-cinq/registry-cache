// SPDX-License-Identifier: Apache-2.0
use std::fmt;
use serde::{Deserialize, Serialize};

const BLOB_ERROR:&str = "BLOB_ERROR";
const BLOB_UNKNOWN:&str = "BLOB_UNKNOWN";
const BLOB_UPLOAD_INVALID:&str = "BLOB_UPLOAD_INVALID";
const BLOB_UPLOAD_UNKNOWN:&str = "BLOB_UPLOAD_UNKNOWN";
const DIGEST_INVALID:&str = "DIGEST_INVALID";
const MANIFEST_BLOB_UNKNOWN:&str = "MANIFEST_BLOB_UNKNOWN";
const MANIFEST_INVALID:&str = "MANIFEST_INVALID";
const MANIFEST_UNKNOWN:&str = "MANIFEST_UNKNOWN";
const MANIFEST_UNVERIFIED:&str = "MANIFEST_UNVERIFIED";
const NAME_INVALID:&str = "NAME_INVALID";
const NAME_UNKNOWN:&str = "NAME_UNKNOWN";
const SIZE_INVALID:&str = "SIZE_INVALID";
const TAG_INVALID:&str = "TAG_INVALID";
const UNAUTHORIZED:&str = "UNAUTHORIZED";
const INTERNAL_SERVER_ERROR:&str = "INTERNAL_SERVER_ERROR";
const JWT_SIGN_ERROR:&str = "JWT_SIGN_ERROR";
const NOT_FOUND:&str = "NOT_FOUND";
const MAX_PAYLOAD_REACHED:&str = "PAYLOAD_REACHED_MAX_SIZE_LIMIT";
const CONFIG_ERROR: &str = "CONFIG_ERROR";
const INVALID_SESSION:&str = "INVALID_SESSION";

const SESSION_ERROR:&str = "SESSION_ERROR";
const JWT_TOKEN_VALIDATION_ERROR:&str = "JWT_TOKEN_VALIDATION_ERROR";
const AUTHENTICATION_ERROR:&str = "AUTHENTICATION_ERROR";
const AUTHORIZATION_ERROR:&str = "AUTHORIZATION_ERROR";
const SQL_ERROR:&str = "SQL_ERROR";
const JSON_ERROR:&str = "JSON_ERROR";


/// Enum representing the various kinds of DB errors
#[derive(Serialize, Deserialize, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ErrorKind {

    RegistryBlobError,

    // =============================================================================================
    // REGISTRY ERRORS
    /// This error MAY be returned when a blob is unknown to the registry in a specified repository.
    /// This can be returned with a standard get or if a manifest references an unknown layer during upload.
    RegistryBlobUnknown,

    /// The blob upload encountered an error and can no longer proceed.
    RegistryBlobUploadInvalid,

    ///  If a blob upload has been cancelled or was never started, this error code MAY be returned.
    RegistryBlobUploadUnknown,

    /// Invalid digest
    RegistryDigestInvalid,

    /// Unknown manifest blob
    RegistryManifestBlobUnknown,

    /// Invalid Manifest
    RegistryManifestInvalid,

    /// Unknown manifest
    RegistryManifestUnknown,

    /// Unverified manifest
    RegistryManifestUnverified,

    /// Invalid container image name
    RegistryNameInvalid,

    /// Unknown container image
    RegistryNameUnknown,

    /// Invalid size
    RegistrySizeInvalid,

    /// Invalid Tag
    RegistryTagInvalid,

    /// Unauthorized registry access
    RegistryUnauthorized,

    // =============================================================================================

    /// In case a session cannot be read or written
    SessionError,

    /// In case a session is invalid
    InvalidSession,

    /// Returned when the user is not authorized to execute an API calls
    Unauthorized,

    /// Returned when there is an internal API error
    InternalError,

    /// Returned when there is an error with the JWT token
    JWTokenValidationError,

    /// Error signing the JWT token
    JWTokenSignError,

    /// Returned when a resource is not found
    NotFound,

    // =============================================================================================

    /// The upload has overflown
    MaxPayloadError,

    /// Authentication error
    AuthenticationError,

    /// Authorization error
    AuthorizationError,

    /// SQLError error
    SQLError,

    /// Json Serialization/DeSerialization error
    JSONError,

    /// Database record not found
    RecordNotFound,

    /// Error loading config
    ConfigError,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        let kind = match *self {
            ErrorKind::RegistryBlobError => BLOB_ERROR,
            ErrorKind::RegistryBlobUnknown => BLOB_UNKNOWN,
            ErrorKind::RegistryBlobUploadInvalid => BLOB_UPLOAD_INVALID,
            ErrorKind::RegistryBlobUploadUnknown => BLOB_UPLOAD_UNKNOWN,
            ErrorKind::RegistryDigestInvalid => DIGEST_INVALID,
            ErrorKind::RegistryManifestBlobUnknown => MANIFEST_BLOB_UNKNOWN,
            ErrorKind::RegistryManifestInvalid => MANIFEST_INVALID,
            ErrorKind::RegistryManifestUnknown => MANIFEST_UNKNOWN,
            ErrorKind::RegistryManifestUnverified => MANIFEST_UNVERIFIED,
            ErrorKind::RegistryNameInvalid => NAME_INVALID,
            ErrorKind::RegistryNameUnknown => NAME_UNKNOWN,
            ErrorKind::RegistrySizeInvalid => SIZE_INVALID,
            ErrorKind::RegistryTagInvalid => TAG_INVALID,
            ErrorKind::RegistryUnauthorized => UNAUTHORIZED,
            ErrorKind::SessionError => SESSION_ERROR,
            ErrorKind::InvalidSession => INVALID_SESSION,
            ErrorKind::Unauthorized => UNAUTHORIZED,
            ErrorKind::InternalError => INTERNAL_SERVER_ERROR,
            ErrorKind::JWTokenValidationError => JWT_TOKEN_VALIDATION_ERROR,
            ErrorKind::JWTokenSignError => JWT_SIGN_ERROR,
            ErrorKind::NotFound => NOT_FOUND,
            ErrorKind::AuthenticationError => AUTHENTICATION_ERROR,
            ErrorKind::AuthorizationError => AUTHORIZATION_ERROR,
            ErrorKind::SQLError => SQL_ERROR,
            ErrorKind::JSONError => JSON_ERROR,
            ErrorKind::RecordNotFound => NOT_FOUND,
            ErrorKind::MaxPayloadError => MAX_PAYLOAD_REACHED,
            ErrorKind::ConfigError => CONFIG_ERROR,
        };

        write!(f, "{}", kind)
    }
}