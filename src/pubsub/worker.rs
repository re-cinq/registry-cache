// SPDX-License-Identifier: Apache-2.0
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use crate::models::commands::RegistryCommand;
use crate::pubsub::subscriber::CommandSubscriber;

/// Worker of the worker pool which process the commands and executes them
pub struct Worker {
    /// The size of the channel buffer
    buffer_size: usize,

    /// The subscriber for this worker
    handler: CommandSubscriber,
}

impl Worker {

    /// New worker instance for the specific Handler
    pub fn new(buffer_size: usize, handler: CommandSubscriber) -> Self {
        // New instance
        Worker {
            buffer_size,
            handler
        }
    }

    /// Start processing the messages and return the channel needed to communicate with it
    pub async fn start(&self) -> Sender<RegistryCommand> {
        // Build the channel
        let (sender, mut receiver) = mpsc::channel(self.buffer_size);

        // Clone the worker reference (behind an Arc)
        let local_worker = self.handler.clone();

        // Start the processing of the commands in a different task
        tokio::spawn(async move {

            // await for a command
            while let Some(cmd) = receiver.recv().await {

                // Shutdown
                if let RegistryCommand::Shutdown = cmd {
                    receiver.close();
                    return;
                }

                // check if the worker supports concurrency
                if local_worker.supports_concurrency() {
                    // If so execute the method in a different task

                    // Clone the worker ARC
                    let async_worker = local_worker.clone();

                    // run the method in a different task
                    tokio::spawn(async move {
                        async_worker.run(cmd).await;
                    });
                } else {
                    // run the method in the current task
                    // WARNING: this blocks reading other commands, so the execution should be fast
                    local_worker.run(cmd).await;
                }
            }
        });

        // return the channel sender
        sender
    }

}