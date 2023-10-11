// SPDX-License-Identifier: Apache-2.0
use std::fmt;

pub struct RepositoryError {
    pub message: String,
}

pub fn from(message: String) -> RepositoryError {
    RepositoryError { message }
}

/// Display implementation
impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Debug implementation
impl fmt::Debug for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RepositoryError {{ message: {} }}", self.message)
    }
}