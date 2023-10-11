// SPDX-License-Identifier: Apache-2.0
use std::fmt;
use actix_web::{error, HttpResponse, HttpResponseBuilder};
use actix_web::http::{header, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::log;
use crate::error::error_kind::ErrorKind;

// =================================================================================================

#[derive(Serialize, Deserialize, Clone)]
struct RegistryErrorResponse {
    code: String,
    message: String,
    details: String,
}

#[derive(Serialize, Deserialize)]
struct ErrorResponse {
    errors: Vec<RegistryErrorResponse>
}

// =================================================================================================

/// Manages the registry internal errors, that can be logged
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RegistryError {
    /// The kind of error
    pub kind: ErrorKind,

    /// General description of the error
    pub message: String,

    /// The original error we might want to log
    pub error: String,

    /// Realm for authentication of the registry
    realm: String
}

impl fmt::Debug for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "RegistryError {{ kind: ErrorKind::{:#?}, message: {:?}, error: {:?} }}",
            self.kind, self.message, self.error
        )
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} {}: {}", self.kind, self.message, self.error)
    }
}

impl From<ErrorKind> for RegistryError {
    fn from(kind: ErrorKind) -> RegistryError {
        RegistryError::new(kind)
    }
}

/// Converts from serde_json::Error to module error
impl From<serde_json::Error> for RegistryError {
    fn from(e: serde_json::Error) -> RegistryError {
        RegistryError::new(ErrorKind::JSONError)
            .with_context("failed to serialize/deserialize object")
            .with_error(e.to_string())
    }
}

impl RegistryError {

    pub fn log(&self) {
        log::error!("{}", self)
    }

    /// Creates a new [`Error`](struct.Error.html)
    pub fn new(kind: ErrorKind) -> RegistryError {
        RegistryError { kind, message: Default::default(), error: Default::default(), realm: Default::default() }
    }

    /// Adds additional context to the [`Error`](struct.Error.html). The additional context will be appended to
    /// the end of the [`Error`](struct.Error.html)'s display string
    pub fn with_context<S>(mut self, context: S) -> RegistryError
        where
            S: AsRef<str>
    {
        self.message = context.as_ref().to_string();
        self
    }

    /// Add the original error as string to the RegistryError
    pub fn with_error<S>(mut self, error: S) -> RegistryError where S: AsRef<str> {
        self.error = error.as_ref().to_string();
        self
    }

    /// Returns the status code
    fn status_code(&self) -> StatusCode {
        match self.kind {

            // Invalid requests
            ErrorKind::RegistryDigestInvalid => StatusCode::BAD_REQUEST,
            ErrorKind::RegistryManifestInvalid => StatusCode::BAD_REQUEST,
            ErrorKind::RegistryBlobUploadInvalid => StatusCode::BAD_REQUEST,
            ErrorKind::RegistrySizeInvalid => StatusCode::BAD_REQUEST,
            ErrorKind::RegistryTagInvalid => StatusCode::BAD_REQUEST,

            // Not found requests
            ErrorKind::RegistryNameInvalid => StatusCode::NOT_FOUND,
            ErrorKind::RegistryNameUnknown => StatusCode::NOT_FOUND,
            ErrorKind::RegistryManifestUnknown => StatusCode::NOT_FOUND,
            ErrorKind::RegistryBlobUnknown => StatusCode::NOT_FOUND,
            ErrorKind::RegistryBlobUploadUnknown => StatusCode::NOT_FOUND,
            ErrorKind::RegistryManifestBlobUnknown => StatusCode::NOT_FOUND,
            ErrorKind::RecordNotFound => StatusCode::NOT_FOUND,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,

            // Failed expectation
            ErrorKind::RegistryManifestUnverified => StatusCode::EXPECTATION_FAILED,

            // Unauthorized
            ErrorKind::RegistryUnauthorized => StatusCode::UNAUTHORIZED,
            ErrorKind::AuthenticationError => StatusCode::UNAUTHORIZED,
            ErrorKind::AuthorizationError => StatusCode::UNAUTHORIZED,
            ErrorKind::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorKind::JWTokenValidationError => StatusCode::UNAUTHORIZED,
            ErrorKind::JWTokenSignError => StatusCode::UNAUTHORIZED,

            // 413 max request size
            ErrorKind::MaxPayloadError => StatusCode::PAYLOAD_TOO_LARGE,

            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl error::ResponseError for RegistryError {

    /// Returns the status code
    fn status_code(&self) -> StatusCode {
        match self.kind {

            // Invalid requests
            ErrorKind::RegistryDigestInvalid => StatusCode::BAD_REQUEST,
            ErrorKind::RegistryManifestInvalid => StatusCode::BAD_REQUEST,
            ErrorKind::RegistryBlobUploadInvalid => StatusCode::BAD_REQUEST,
            ErrorKind::RegistrySizeInvalid => StatusCode::BAD_REQUEST,
            ErrorKind::RegistryTagInvalid => StatusCode::BAD_REQUEST,

            // Not found requests
            ErrorKind::RegistryNameInvalid => StatusCode::NOT_FOUND,
            ErrorKind::RegistryNameUnknown => StatusCode::NOT_FOUND,
            ErrorKind::RegistryManifestUnknown => StatusCode::NOT_FOUND,
            ErrorKind::RegistryBlobUnknown => StatusCode::NOT_FOUND,
            ErrorKind::RegistryBlobUploadUnknown => StatusCode::NOT_FOUND,
            ErrorKind::RegistryManifestBlobUnknown => StatusCode::NOT_FOUND,
            ErrorKind::RecordNotFound => StatusCode::NOT_FOUND,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,

            // Failed expectation
            ErrorKind::RegistryManifestUnverified => StatusCode::EXPECTATION_FAILED,

            // Unauthorized
            ErrorKind::RegistryUnauthorized => StatusCode::UNAUTHORIZED,
            ErrorKind::AuthenticationError => StatusCode::UNAUTHORIZED,
            ErrorKind::AuthorizationError => StatusCode::UNAUTHORIZED,
            ErrorKind::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorKind::JWTokenValidationError => StatusCode::UNAUTHORIZED,
            ErrorKind::JWTokenSignError => StatusCode::UNAUTHORIZED,

            // 413 max request size
            ErrorKind::MaxPayloadError => StatusCode::PAYLOAD_TOO_LARGE,

            // Internal server error
            ErrorKind::JSONError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorKind::SQLError => StatusCode::INTERNAL_SERVER_ERROR,

            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Return the HTTP error response
    fn error_response(&self) -> HttpResponse {
        // calculate the status code
        let status_code = self.status_code();

        // put together the array of errors: in our case always 1
        // but this is the format of the spec
        let errors: Vec<RegistryErrorResponse> = vec![RegistryErrorResponse {
            code: self.kind.to_string(),
            message: self.message.to_string(),
            details: self.error.to_string(),
        }];

        let error_response = ErrorResponse {
            errors
        };

        let body = serde_json::to_string_pretty(&error_response);

        if body.is_err() {
            return HttpResponseBuilder::new(StatusCode::INTERNAL_SERVER_ERROR)
                .insert_header((header::CONTENT_TYPE, "text/html; charset=utf-8"))
                .body("Internal Server error!");
        }

        // if we got here then we are fine
        let mut builder = HttpResponseBuilder::new(status_code)
            .insert_header((header::CONTENT_TYPE, "application/json; charset=utf-8")).take();

        if status_code == StatusCode::UNAUTHORIZED {
            let realm = self.realm.clone();
            if !realm.is_empty() {
                builder.insert_header((header::WWW_AUTHENTICATE, realm));
            }
        }


        builder.body(body.unwrap())
    }
}
