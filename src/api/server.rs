// SPDX-License-Identifier: Apache-2.0
use std::{fs::File, io::BufReader};
use std::sync::Arc;
use std::time::Duration;
use actix_web::{App, HttpServer, middleware, web};
use actix_web::http::KeepAlive;
use actix_web::middleware::{Logger, TrailingSlash};
use reqwest::ClientBuilder;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use tracing::log;
use crate::api::routes;
use crate::api::metrics::metrics_handler;
use crate::api::state::AppState;
use crate::config::app::AppConfig;
use crate::handlers::command::blob::service::ManifestService;
use crate::metrics::register_metrics;
use crate::pubsub::command_bus::CommandBus;
use crate::repository::filesystem::FilesystemStorage;

pub async fn start(config: AppConfig, command_bus: Arc<CommandBus>, manifest_service: Arc<ManifestService>) -> std::io::Result<()> {

    // TODO: 1. expose the timeout settings to the config
    // TODO: 2. expose the possibility to skip TLS verification
    // TODO: 3. allow to pass a proxy configuration
    // TODO: 4. allow to pass a custom DNS resolver
    // Http client for the upstream requests
    let reqwest_client = ClientBuilder::new()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .tcp_nodelay(true)
        .build().expect("Failed to create upstream http client");

    // Upstream hostname
    let app_config = config.clone();

    // Tls config
    let tls_config = load_tls(&config);

    // Storage
    let filesystem_storage = FilesystemStorage::new(app_config.clone());

    // Host and port
    let api_config = config.api.clone();
    let host_port = format!("{}:{}", api_config.hostname, api_config.port.unwrap_or_else(|| String::from("8080")));

    // Upstreams
    for (host, upstream) in config.clone().upstreams() {
        let forward_url = format!("{}://{}", upstream.schema, upstream.registry);
        log::info!("forwarding from {} to {}", host, forward_url);
    }


    // Init the command bus
    let bus = command_bus.clone();

    // Application state
    let state = web::Data::new(AppState::new(reqwest_client, command_bus.clone(), app_config.clone(),
                                             filesystem_storage, manifest_service));

    log::info!("starting HTTP server at https://{}", config.api.hostname,);

    // Prometheus
    register_metrics();

    // Create the actix web server
    let server = HttpServer::new(move || {
        App::new()
            //.app_data(state.clone())
            .app_data(state.clone())
            // .app_data(web::Data::new(forward_url.clone()))
            .wrap(middleware::NormalizePath::new(TrailingSlash::MergeOnly))
            .wrap(middleware::Compress::default())
            .wrap(Logger::default())
            // Container Registry Scope
            .service(metrics_handler)
            .service(web::scope("/v2").configure(routes::registry_api_config))
    }).keep_alive(KeepAlive::Timeout(Duration::from_secs(75)));

    // let stop_handle = StopHandle::new(bus);

    let server = if let Some(tls) = tls_config {
        server.bind_rustls_021(host_port, tls)?
            .run()

    } else {
        server.bind(host_port)?
            .run()
    };

    // Listen for the HTTP requests
    server.await?;

    // Call the stop handle
    // stop_handle.stop(true).await;
    tracing::info!("Shutting down persistence bus...");
    bus.shutdown().await;

    Ok(())
}

fn load_tls(config: &AppConfig) -> Option<ServerConfig> {

    if config.api.tls_cert.is_none() || config.api.tls_key.is_none() {
        return None;
    }

    let cert_file_path = config.api.tls_cert.clone().unwrap();
    let key_file_path = config.api.tls_key.clone().unwrap();

    // init server config builder with safe defaults
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();

    // load TLS key/cert files
    let cert_file = &mut BufReader::new(File::open(&cert_file_path).unwrap_or_else(|_| panic!("failed to open certificate file {:?}", cert_file_path)));
    let key_file = &mut BufReader::new(File::open(&key_file_path).unwrap_or_else(|_| panic!("failed to open certificate private key file {:?}", key_file_path)));

    // convert files to key/cert objects
    let cert_chain = certs(cert_file)
        .unwrap()
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
        .unwrap()
        .into_iter()
        .map(PrivateKey)
        .collect();

    // exit if no keys could be parsed
    if keys.is_empty() {
        eprintln!("Could not locate PKCS 8 private keys.");
        std::process::exit(1);
    }

    Some(config.with_single_cert(cert_chain, keys.remove(0)).unwrap())
}