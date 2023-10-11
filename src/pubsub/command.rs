// SPDX-License-Identifier: Apache-2.0
pub trait ChannelId {
    fn queue_id(&self) -> u64;
    fn topic_id(&self) -> String;
}