// SPDX-License-Identifier: Apache-2.0
use actix_web::{dev::PeerAddr, http::Method, web, HttpRequest, HttpResponse};
use actix_web::http::header;
use actix_web::http::header::HeaderValue;
use futures_util::{pin_mut, StreamExt as _, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use url::Url;
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
                   method: Method, peer_addr: Option<PeerAddr>,
                   state: web::Data<AppState>) -> Result<HttpResponse, RegistryError> {

    // Increase the requests counter
    metrics::INCOMING_REQUESTS.inc();

    // parse the name from the request
    let repository = blob_request.into_inner();

    // Image info
    let image_name = repository.name.clone();

    // validate the repository
    let repository = repository.is_valid().await?;

    // Make sure we have the digest in the request
    if repository.digest.is_none() {
        let err = RegistryError::new(RegistryBlobUnknown).with_error(format!("Failed to parse digest: {}", repository.reference));
        err.log();
        return Err(err);
    }

    // Try to open the repository now
    let existing = state.storage.read(repository.clone()).await;

    // TODO: handoff the write operations to a service which stores the blob as tmp and then verifies the sha256
    // and moves it atomically when it matches

    match existing {
        Ok(_blob) => {

            let file = actix_files::NamedFile::open_async(state.storage.blob_path(repository)).await
                .map_err(|e| RegistryError::new(ErrorKind::NotFound).with_error(e.to_string()))?;

            log::info!("*** Cached: {} {}", req.method(), req.uri());

            let response = file.into_response(&req);

            metrics::CACHED_RESPONSES.inc();
            metrics::RESPONSE_CODE_COLLECTOR.with_label_values(&[response.status().as_str(), req.method().as_str(), &image_name]).inc();

            Ok(response)

        }
        Err(_e) => {

            let host_header = req.headers().get(header::HOST).cloned().unwrap_or_else(|| HeaderValue::from_static(""));
            let host = host_header.to_str().unwrap_or("");
            let upstream = state.upstreams.get(host);

            if upstream.is_none() {
                tracing::error!("Upstream not found for host {}", host);
                return Err(RegistryError::new(ErrorKind::NotFound));
            }
            let upstream = upstream.unwrap();
            let forward_url = format!("{}://{}", upstream.schema, upstream.registry);

            // Rewrite the URL
            let mut new_url = Url::parse(&forward_url).unwrap();

            // Convert the original request URI to string
            let path = req.uri().path();

            // Set the URL path
            new_url.set_path(path);

            // Set the URL query string parameters
            new_url.set_query(req.uri().query());


            // Create the upstream request
            let mut forwarded_req = state.client
                .request(method, new_url);

            // Append the client request headers to the upstream request
            for (header_name, header_value) in req.headers().iter().filter(|(h, _)| *h != "host") {
                forwarded_req = forwarded_req.header(header_name, header_value);
            }

            // TODO: This forwarded implementation is incomplete as it only handles the unofficial
            // X-Forwarded-For header but not the official Forwarded one.
            let forwarded_req = match peer_addr {
                Some(PeerAddr(addr)) => forwarded_req.header("X-Forwarded-For", addr.ip().to_string()),
                None => forwarded_req,
            };

            let upstream_request = forwarded_req.build().map_err(|e| RegistryError::new(ErrorKind::NotFound).with_error(e.to_string()))?;

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