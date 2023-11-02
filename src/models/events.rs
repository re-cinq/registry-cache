// SPDX-License-Identifier: Apache-2.0
use strum::Display;

#[derive(Clone, Display, Debug)]
pub enum RegistryEvent {
    BlobPersisted
}