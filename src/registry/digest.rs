// SPDX-License-Identifier: Apache-2.0
use lazy_static::lazy_static;
use regex::Regex;
use std::str::FromStr;
use std::fmt;
use std::fs::File;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Sha256, Sha512};
use sha2::Digest as Sha2Digest;
use crate::error::error_kind::ErrorKind;
use crate::error::error_kind::ErrorKind::RegistryDigestInvalid;
use crate::error::registry::RegistryError;
use crate::registry::repository_error;
use crate::registry::repository_error::RepositoryError;

// These regex are used to do a simple validation of the tag fields
lazy_static! {
    static ref REGEX_ALGO: Regex = Regex::new(r"^[A-Za-z0-9_+.-]+$").unwrap();
    static ref REGEX_DIGEST: Regex = Regex::new(r"^[A-Fa-f0-9]+$").unwrap();
}

#[derive(Hash, Serialize, Deserialize, Debug, Clone, Copy, PartialOrd, Ord, Eq, PartialEq, Default)]
pub enum DigestAlgorithm {
    #[default]
    Sha256,
    Sha512,
}

impl FromStr for DigestAlgorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sha256" => Ok(DigestAlgorithm::Sha256),
            "sha512" => Ok(DigestAlgorithm::Sha512),
            "SHA256" => Ok(DigestAlgorithm::Sha256),
            "SHA512" => Ok(DigestAlgorithm::Sha512),
            _ => Err(format!("'{}' is not a valid DigestAlgorithm", s)),
        }
    }
}

impl fmt::Display for DigestAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DigestAlgorithm::Sha256 => write!(f, "sha256"),
            DigestAlgorithm::Sha512 => write!(f, "sha512"),
        }
    }
}

/// This contains the algorithm and the hashed value
#[derive(Debug, PartialEq, Clone, PartialOrd, Eq, Hash)]
pub struct Digest {
    pub algo: DigestAlgorithm,
    pub hash: String,
}

impl Default for Digest {
    fn default() -> Self {
        Digest {
            algo: Default::default(),
            hash: "".to_string()
        }
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.algo, self.hash)
    }
}

/// Implemented custom deserializer from string to Enum
impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?.to_lowercase();
        Digest::parse(s.as_str()).map_err(|e| de::Error::custom(format!("error parsing digest: {}", e)))
    }
}

/// Implemented custom serializer from Enum to String
impl Serialize for Digest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl Digest {

    /// Parse the digest with the form: algo:hash
    pub fn parse(component: &str) -> Result<Digest, RegistryError> {
        let algo_digest = component
            .split(':')
            .map(String::from)
            .collect::<Vec<String>>();

        // Parse it now
        Digest::parse_parts(algo_digest).map_err(|e|
            RegistryError::new(RegistryDigestInvalid)
                .with_context(format!("failed to parse digest {}", component)).with_error(e.to_string()))
    }


    pub async fn hash_digest_file(algo: DigestAlgorithm, mut file: File) -> Result<Digest, RegistryError> {
        match algo {
            DigestAlgorithm::Sha256 => {
                let handle = tokio::task::spawn_blocking(move || async move {
                    let mut hasher = Sha256::new();
                    let _n = std::io::copy(&mut file, &mut hasher);
                    let hash = hasher.finalize();
                    Ok(Digest {
                        algo,
                        hash: hex::encode(hash),
                    })
                });


                match handle.await {
                    Ok(result) => result.await,
                    Err(e) => {
                        Err(RegistryError::new(ErrorKind::RegistryBlobUploadInvalid)
                            .with_context("failed to calculate sha256 digest").with_error(format!("{:?}", e)))
                    }
                }
            }
            DigestAlgorithm::Sha512 => {
                let handle = tokio::task::spawn_blocking(move || async move {
                    let mut hasher = Sha512::new();
                    let _n = std::io::copy(&mut file, &mut hasher);
                    let hash = hasher.finalize();
                    Ok(Digest {
                        algo,
                        hash: hex::encode(hash),
                    })
                });


                match handle.await {
                    Ok(result) => result.await,
                    Err(e) => {
                        Err(RegistryError::new(ErrorKind::RegistryBlobUploadInvalid)
                            .with_context("failed to calculate sha512 digest").with_error(format!("{:?}", e)))
                    }
                }
            }
        }
    }

    // /// Returns a hash in the form of: hash
    // pub async fn hash_reference(algo: DigestAlgorithm, data: &[u8]) -> Result<String, RegistryError> {
    //     let digest = Self::hash_digest(algo, data).await?;
    //     Ok(digest.to_string())
    // }

    // =============================================================================================
    // Private functions

    /// Parses the split parts: algo and digest
    fn parse_parts(algo_digest: Vec<String>) -> Result<Digest, RepositoryError> {
        // check that we have both parts: algo and digest
        if algo_digest.len() < 2 {
            return Err(repository_error::from(format!(
                "Component cannot be parsed into a digest: {:#?}",
                &algo_digest
            )));
        }

        // Do a simple verification
        let algo = String::from(&algo_digest[0]);
        let digest = String::from(&algo_digest[1]);

        if !REGEX_ALGO.is_match(&algo) {
            return Err(repository_error::from(format!(
                "Component cannot be parsed into a TAG wrong digest algorithm: {:#?} - {}",
                &algo_digest, &algo
            )));
        }

        if !REGEX_DIGEST.is_match(&digest) {
            return Err(repository_error::from(format!(
                "Component cannot be parsed into a TAG wrong digest format: {:#?} - {}",
                &algo_digest, &digest
            )));
        }

        let algo_enum = DigestAlgorithm::from_str(algo.as_str()).map_err(|e| RepositoryError {
            message: e,
        })?;

        Ok(Digest {
            algo: algo_enum,
            hash: digest,
        })
    }
}

#[cfg(test)]
mod test {

    use serde_json::json;
    use crate::registry::digest::{Digest, DigestAlgorithm};


    #[tokio::test]
    async fn digest_test() {
        let digest = Digest {
            algo: DigestAlgorithm::Sha256,
            hash: "05c6e08f1d9fdafa03147fcb8f82f124c76d2f70e3d989dc8aadb5e7d7450bec".to_string()
        };

        assert_eq!(
            digest.to_string(),
            "sha256:05c6e08f1d9fdafa03147fcb8f82f124c76d2f70e3d989dc8aadb5e7d7450bec"
        );
    }

    #[tokio::test]
    async fn digest_serde_test() {
        let digest = Digest {
            algo: DigestAlgorithm::Sha256,
            hash: "05c6e08f1d9fdafa03147fcb8f82f124c76d2f70e3d989dc8aadb5e7d7450bec".to_string()
        };

        let serialised = json!(&digest);
        assert_eq!("sha256:05c6e08f1d9fdafa03147fcb8f82f124c76d2f70e3d989dc8aadb5e7d7450bec", serialised);

        let parsed_digest: Digest = serde_json::from_value(serde_json::Value::String("sha256:05c6e08f1d9fdafa03147fcb8f82f124c76d2f70e3d989dc8aadb5e7d7450bec".to_string())).expect("failed to parse digest");
        assert_eq!(parsed_digest, digest);

    }
}