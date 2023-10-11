/// Repository is the implementation of the repository spec:
/// https://github.com/opencontainers/distribution-spec/blob/master/spec.md#overview
/// 1. A repository name is broken up into path components.
/// 2. A component of a repository name MUST begin with one or more lowercase alpha-numeric characters.
/// 3. Subsequent lowercase alpha-numeric characters are OPTIONAL and MAY be separated by periods, dashes or underscores.
/// More strictly, it MUST match the regular expression [a-z0-9]+(?:[._-][a-z0-9]+)*.

// SPDX-License-Identifier: Apache-2.0
use lazy_static::lazy_static;
use regex::Regex;

use serde::{Deserialize, Serialize};
use crate::error::error_kind::ErrorKind;
use crate::error::registry::RegistryError;
use crate::registry::digest::{Digest, DigestAlgorithm};

lazy_static! {
    static ref REGEX_COMPONENT: Regex = Regex::new(r"^[a-z0-9]+(?:[._-][a-z0-9]+)*").unwrap();
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Repository {
    // This is the whole name(space)
    #[serde(default)]
    pub name: String,

    // This is the whole reference
    #[serde(default)]
    pub reference: String,

    // This is the parsed namespace
    #[serde(default)]
    pub components: Vec<String>,

    // If the reference is a digest then it's also parsed
    #[serde(default)]
    pub digest: Option<Digest>,
}

impl Repository {
    /// New repository with reference
    pub fn new_with_reference(name: &str, reference: &str) -> Result<Repository, RegistryError> {
        // parse the name(space)
        let mut repository = Repository::new(name)?;

        // set the reference
        repository.reference = reference.to_string();

        // if the reference contains a :, then check if it is a digest
        if reference.contains(':') && (reference.starts_with(&DigestAlgorithm::Sha256.to_string()) ||
            reference.starts_with(&DigestAlgorithm::Sha512.to_string())){
            repository.digest = Some(Digest::parse(reference)?);

        } else if !REGEX_COMPONENT.is_match(reference) {
            return Err(RegistryError::new(ErrorKind::RegistryDigestInvalid).with_error(format!(
                "Repository reference/tag is invalid: {}",
                &reference
            )));
        }


        // return it
        Ok(repository)
    }

    /// New repository
    pub fn new(name: &str) -> Result<Repository, RegistryError> {
        // check that the maximum amount of chars for the name is 255
        if name.len() > 255 {
            return Err(RegistryError::new(ErrorKind::RegistryNameInvalid).with_error(format!(
                "Repository name max length should be less than 255 chars - we got: {}",
                name.len()
            )));
        }

        // split the repository name into components via the: `/` char
        let components = name
            .split('/')
            .map(String::from)
            .collect::<Vec<String>>();

        // verify now that each component is valid
        for component in &components {
            // if it does not match then return an error!
            if !REGEX_COMPONENT.is_match(component) {
                return Err(RegistryError::new(ErrorKind::RegistryNameInvalid).with_error(format!(
                    "Repository component is invalid: {}",
                    &name
                )));
            }
        }

        Ok(Repository {
            name: name.to_string(),
            reference: "".to_string(),
            components,
            digest: None
        })
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn repository_no_tag_test() {
        let repo_name = String::from("library/nginx");
        let repo = super::Repository::new(&repo_name).expect(&*format!("Failed to parse repo: {}", &repo_name));
        assert_eq!(2, repo.components.len());
        assert_eq!("library", repo.components[0]);
        assert_eq!("nginx", repo.components[1]);
        assert_eq!(repo_name, repo.name);
        // assert_eq!("", repo.tag.name);
        // assert_eq!(DigestAlgorithm::default(), repo.tag.algo);
        // assert_eq!("", repo.tag.version);
        // assert_eq!(false, repo.tag.is_digest);
    }

    #[test]
    fn repository_with_empty_tag_test() {
        let repo_name = String::from("library/nginx");
        let repo = super::Repository::new(&repo_name)
            .expect(&format!("Failed to parse repo: {}", &repo_name));
        assert_eq!(2, repo.components.len());
        assert_eq!("library", repo.components[0]);
        assert_eq!("nginx", repo.components[1]);
        assert_eq!(repo_name, repo.name);
    }

    #[test]
    fn repository_with_tag_test() {
        let repo_name = String::from("library/nginx");
        let reference = "nginx:1.18";
        let repo = super::Repository::new_with_reference(&repo_name, reference)
            .expect(&*format!("Failed to parse repo: {}", &repo_name));
        assert_eq!(2, repo.components.len());
        assert_eq!("library", repo.components[0]);
        assert_eq!("nginx", repo.components[1]);
        assert_eq!(repo_name, repo.name);
        assert_eq!(reference, repo.reference);
        // assert_eq!("nginx", repo.tag.name);
        // assert_eq!(DigestAlgorithm::default(), repo.tag.algo);
        // assert_eq!("1.18", repo.tag.version);
        // assert_eq!(false, repo.tag.is_digest);
    }

    #[test]
    fn repository_basic_test() {
        let repo_name = String::from("library");
        let reference = "nginx:latest";
        let repo = super::Repository::new_with_reference(&repo_name, reference)
            .expect(&*format!("Failed to parse repo: {}", &repo_name));
        assert_eq!(1, repo.components.len());
        assert_eq!("library", repo.components[0]);
        assert_eq!(repo_name, repo.name);
        assert_eq!(reference, repo.reference);
        // assert_eq!("nginx", repo.tag.name);
        // assert_eq!("latest", repo.tag.version);
        // assert_eq!(false, repo.tag.is_digest);
    }

    #[test]
    fn repository_test() {
        let repo_name = String::from("library");
        let reference = "debian:unstable-20200803-slim";
        let repo = super::Repository::new_with_reference(&repo_name, reference)
            .expect(&*format!("Failed to parse repo: {}", &repo_name));
        assert_eq!(1, repo.components.len());
        assert_eq!("library", repo.components[0]);
        assert_eq!(repo_name, repo.name);
        assert_eq!(reference, repo.reference);
        // assert_eq!("debian", repo.tag.name);
        // assert_eq!("unstable-20200803-slim", repo.tag.version);
        // assert_eq!(false, repo.tag.is_digest);
    }

    #[test]
    fn repository_image_version_and_digest_test() {
        let repo_name = String::from("frolvlad");
        let reference = "alpine-miniconda3:python3.7@sha256:9bc9c096713a6e47ca1b4a0d354ea3f2a1f67669c9a2456352d28481a6ce2fbe";
        let repo = super::Repository::new_with_reference(&repo_name, reference)
            .expect(&*format!("Failed to parse repo: {}", &repo_name));
        assert_eq!(1, repo.components.len());
        assert_eq!("frolvlad", repo.components[0]);
        assert_eq!(repo_name, repo.name);
        assert_eq!(reference, repo.reference);
        // assert_eq!("alpine-miniconda3", repo.tag.name);
        // assert_eq!(
        //     "9bc9c096713a6e47ca1b4a0d354ea3f2a1f67669c9a2456352d28481a6ce2fbe",
        //     repo.tag.version
        // );
        // assert!(repo.tag.is_digest);
    }

    #[test]
    fn repository_basic_with_slash_prefix_test() {
        let repo_name = String::from("/library");
        let repo = super::Repository::new(&repo_name);
        assert!(
            repo.is_err(),
            "repo should not start with a forward slash /"
        );
    }

    #[test]
    fn repository_complex_test() {
        let repo_name = String::from("lib/crane/reg/test/amd64/nginx");
        let repo = super::Repository::new(&repo_name);
        assert!(repo.is_ok(), "complex repo should be parsed fine");
        let repo = repo.unwrap();
        assert_eq!(6, repo.components.len());
        assert_eq!("lib", repo.components[0]);
        assert_eq!("crane", repo.components[1]);
        assert_eq!("reg", repo.components[2]);
        assert_eq!("test", repo.components[3]);
        assert_eq!("amd64", repo.components[4]);
        assert_eq!("nginx", repo.components[5]);
    }

    #[test]
    fn repository_complex_with_space_test() {
        let repo_name = String::from("lib/crane/reg/test rust/amd64/nginx");
        let repo = super::Repository::new(&repo_name);
        assert!(repo.is_ok(), "complex repo should be parsed fine");
        let repo = repo.unwrap();
        assert_eq!(6, repo.components.len());
        assert_eq!("lib", repo.components[0]);
        assert_eq!("crane", repo.components[1]);
        assert_eq!("reg", repo.components[2]);
        assert_eq!("test rust", repo.components[3]);
        assert_eq!("amd64", repo.components[4]);
        assert_eq!("nginx", repo.components[5]);
    }
}