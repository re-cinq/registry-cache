// SPDX-License-Identifier: Apache-2.0
use lazy_static::lazy_static;
use prometheus::{
    HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts,
};

lazy_static! {

    pub static ref INCOMING_REQUESTS: IntCounter =
        IntCounter::new("incoming_requests", "Incoming Requests").expect("incoming_requests metric cannot be created");

    pub static ref CACHED_RESPONSES: IntCounter =
        IntCounter::new("cached_responses", "Cached Responses").expect("cached_responses metric cannot be created");

    pub static ref UPSTREAM_RESPONSES: IntCounter =
        IntCounter::new("upstream_responses", "Upstream Responses").expect("upstream_responses metric cannot be created");

    pub static ref CONNECTED_CLIENTS: IntGauge =
        IntGauge::new("connected_clients", "Connected Clients").expect("connected_clients metric cannot be created");

    pub static ref RESPONSE_CODE_COLLECTOR: IntCounterVec = IntCounterVec::new(
        Opts::new("response_code", "Response Code"),
        &["statuscode", "type", "image"]
    )
    .expect("response_code metric cannot be created");

    pub static ref RESPONSE_TIME_COLLECTOR: HistogramVec = HistogramVec::new(
        HistogramOpts::new("response_time", "Response Times"),
        &["env"]
    )
    .expect("response_time metric cannot be created");
}

pub fn register_metrics() {

    let registry = prometheus::default_registry();

    registry
        .register(Box::new(INCOMING_REQUESTS.clone()))
        .expect("incoming_requests collector can cannot registered");

    registry
        .register(Box::new(CONNECTED_CLIENTS.clone()))
        .expect("connected_clients collector can cannot registered");

    registry
        .register(Box::new(RESPONSE_CODE_COLLECTOR.clone()))
        .expect("response_code collector can cannot registered");

    registry
        .register(Box::new(RESPONSE_TIME_COLLECTOR.clone()))
        .expect("response_time collector can cannot registered");

    registry.register(Box::new(CACHED_RESPONSES.clone()))
        .expect("cached_responses collector can cannot registered");

    registry.register(Box::new(UPSTREAM_RESPONSES.clone()))
        .expect("upstream_responses collector can cannot registered");
}