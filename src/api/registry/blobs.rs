// SPDX-License-Identifier: Apache-2.0
use actix_web::{http::Method, web, HttpRequest, HttpResponse};
use futures_util::{pin_mut, StreamExt as _, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use crate::api::registry::{build_upstream_req, serve_from_cache, validate_repository};
use crate::api::state::AppState;
use crate::driver::RepositoryTrait;
use crate::error::error_kind::ErrorKind;
use crate::error::error_kind::ErrorKind::RegistryBlobUnknown;
use crate::error::registry::RegistryError;
use crate::metrics;
use crate::models::commands::RegistryCommand;
use crate::registry::repository::Repository;

// This struct is used for the blobs requests
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RepositoryRequest {
    #[serde(default)]
    pub name: String,

    // there reference format is: algo:hex_encoding
    #[serde(default)]
    pub reference: String,
}

impl RepositoryRequest {
    // Parse and verify
    pub async fn is_valid(&self) -> Result<Repository, RegistryError> {
        is_valid(&self.name, &self.reference)
    }
}

// Parse and verify
fn is_valid(name: &str, reference: &str) -> Result<Repository, RegistryError> {
    // Does the request have a non empty reference ?
    let has_reference = !reference.is_empty();

    // Validate the repository
    let repository = if has_reference {
        Repository::new_with_reference(name, reference)
    } else {
        Repository::new(name)
    }?;

    Ok(repository)
}

/// Forward the request to upstream
pub async fn cache(blob_request: web::Path<RepositoryRequest>,
                   req: HttpRequest,
                   method: Method,
                   state: web::Data<AppState>) -> Result<HttpResponse, RegistryError> {

    // Increase the requests counter
    metrics::INCOMING_REQUESTS.inc();

    // parse the name from the request
    let repository = validate_repository(blob_request).await?;

    // Make sure we have the digest in the request
    if repository.digest.is_none() {
        let err = RegistryError::new(RegistryBlobUnknown).with_error(format!("Failed to parse digest: {}", repository.reference));
        err.log();
        return Err(err);
    }

    // Image info
    let image_name = repository.name.clone();

    // Try to open the repository now
    let existing = state.storage.read(repository.clone()).await;

    // Check whether the blob exists
    match existing {
        Ok(_blob) => {

            // Serve the content from cache
            serve_from_cache(req, repository, None, &state).await
        }
        Err(_e) => {

            // Build the upstream URL
            let upstream_request = build_upstream_req(&req, method, &state)?;

            // Build the request
            let upstream_request = upstream_request.build().map_err(|e| RegistryError::new(ErrorKind::NotFound).with_error(e.to_string()))?;

            log::info!("Upstream: {} {}", upstream_request.method(), upstream_request.url());

            // Execute the request against the upstream
            let upstream_response = state.client.execute(upstream_request).await
                .map_err(|e|RegistryError::new(ErrorKind::RegistryBlobError).with_error(e.to_string()))?;

            // Build the response for the client
            let mut client_resp = HttpResponse::build(upstream_response.status());

            // Remove `Connection` as per
            // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
            for (header_name, header_value) in upstream_response.headers().iter().filter(|(h, _)| *h != "connection") {
                client_resp.insert_header((header_name.clone(), header_value.clone()));
                // tracing::info!("Response header: {}: {:?}", header_name, header_value);
            }

            // Create the client response channel
            let (mut response_tx, response_rx) = tokio::io::duplex(8192); //mpsc::unbounded_channel();
            let stream = tokio_util::codec::FramedRead::new(response_rx, tokio_util::codec::BytesCodec::new()).map_ok(|b| b.freeze());

            // Create the persistence channels
            let (persist_tx,persist_rx) = mpsc::unbounded_channel();

            // Ask the bus to store the data
            let persist_command = RegistryCommand::PersistBlob(repository, persist_rx);
            state.command_bus.publish(persist_command).await;

            // Status code
            let status = upstream_response.status().to_string();

            // Consume the stream and send it to 2 channels:
            // - the response channel to send to the client
            // - the persist channel to persist the blob
            let _handle = tokio::spawn(async move {
                let stream = upstream_response.bytes_stream();
                pin_mut!(stream);

                while let Some(chunk) = stream.next().await {
                    if let Ok(ref chunk) = chunk {
                        if let Err(e) = persist_tx.send(chunk.clone()) {
                            tracing::error!("Failed to send blob chunk for persistence: {}", e.to_string());
                        }
                        if let Err(e) = response_tx.write_all(chunk).await {
                            tracing::error!("Failed to send blob chunk for client response: {}", e.to_string());
                        }
                    }
                    // response_tx.write_all(chunk).unwrap();
                }
            });

            metrics::UPSTREAM_RESPONSES.inc();
            metrics::RESPONSE_CODE_COLLECTOR.with_label_values(&[&status, req.method().as_str(), &image_name]).inc();

            // Ok(client_resp.streaming(response_stream))
            Ok(client_resp.streaming(stream))


        }
    }

}