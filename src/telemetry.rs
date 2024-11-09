use lazy_static::lazy_static;
use log::error;
use prometheus::{IntGaugeVec, Opts, Registry};
use warp::{Rejection, Reply};

lazy_static! {
    pub static ref IBC_STATUS_COLLECTOR: IntGaugeVec = IntGaugeVec::new(
        Opts::new("ibc_status", "IBC Status. 0: < min_total, 1: > min_total"),
        &["chain_id", "port_id", "channel_id", "destination_chain_id", "min_total"]
    )
    .expect("metric can be created");
    pub static ref IBC_COUNT_COLLECTOR: IntGaugeVec = IntGaugeVec::new(
        Opts::new("ibc_count", "no of ibc packet commitments"),
        &["chain_id", "port_id", "channel_id", "destination_chain_id", "min_total"]
    )
    .expect("metric can be created");
    pub static ref IBC_QUERY_STATUS_COLLECTOR: IntGaugeVec = IntGaugeVec::new(
        Opts::new("ibc_query_status", "IBC Query Status show the ibc query is successful or not. 0: can access, 1: cannot access"),
        &["chain_id", "port_id", "channel_id", "destination_chain_id", "query_endpoint_url"]
    )
    .expect("metric can be created");
    pub static ref IBC_CLIENT_STATUS_COLLECTOR: IntGaugeVec = IntGaugeVec::new(
        Opts::new("ibc_client_status", "IBC client status. 0: (expiry_time - now) > min_time_before_client_expiration, 1: (expiry_time - now) < min_time_before_client_expiration"),
        &["chain_id", "port_id", "channel_id", "destination_chain_id", "min_time_before_client_expiration"]
    )
    .expect("metric can be created");
    pub static ref IBC_CLIENT_TIME_BEFORE_EXPIRE_COLLECTOR: IntGaugeVec = IntGaugeVec::new(
        Opts::new("ibc_client_time_before_expire", "the times left before client reach min_time_before_client_expiration in seconds"),
        &["chain_id", "port_id", "channel_id", "destination_chain_id", "min_time_before_client_expiration"]
    )
    .expect("metric can be created");

    pub static ref REGISTRY: Registry = Registry::new();
}

/// A setter for IBC_STATUS_COLLECTOR, make sure all the labels are set and types are correct
pub fn ibc_status_setter(
    chain_id: &str,
    port_id: &str,
    channel_id: &str,
    destination_chain_id: &str,
    min_total: &str,
    status: i64,
) {
    IBC_STATUS_COLLECTOR
        .with_label_values(&[
            chain_id,
            port_id,
            channel_id,
            destination_chain_id,
            min_total,
        ])
        .set(status);
}

/// A setter for IBC_COUNT_COLLECTOR, make sure all the labels are set and types are correct
pub fn ibc_count_setter(
    chain_id: &str,
    port_id: &str,
    channel_id: &str,
    destination_chain_id: &str,
    min_total: &str,
    count: i64,
) {
    IBC_COUNT_COLLECTOR
        .with_label_values(&[
            chain_id,
            port_id,
            channel_id,
            destination_chain_id,
            min_total,
        ])
        .set(count);
}

/// A setter for IBC_QUERY_STATUS_COLLECTOR, make sure all the labels are set and types are correct
pub fn ibc_query_status_setter(
    chain_id: &str,
    port_id: &str,
    channel_id: &str,
    destination_chain_id: &str,
    query_endpoint_url: &str,
    status: i64,
) {
    IBC_QUERY_STATUS_COLLECTOR
        .with_label_values(&[
            chain_id,
            port_id,
            channel_id,
            destination_chain_id,
            query_endpoint_url,
        ])
        .set(status);
}

/// A setter for IBC_CLIENT_STATUS_COLLECTOR, make sure all the labels are set and types are correct
pub fn ibc_client_status_setter(
    chain_id: &str,
    port_id: &str,
    channel_id: &str,
    destination_chain_id: &str,
    min_time_before_client_expiration: &str,
    status: i64,
) {
    IBC_CLIENT_STATUS_COLLECTOR
        .with_label_values(&[
            chain_id,
            port_id,
            channel_id,
            destination_chain_id,
            min_time_before_client_expiration,
        ])
        .set(status);
}

/// A setter for IBC_CLIENT_TIME_BEFORE_EXPIRE_COLLECTOR, make sure all the labels are set and types are correct
pub fn ibc_client_time_before_expire_setter(
    chain_id: &str,
    port_id: &str,
    channel_id: &str,
    destination_chain_id: &str,
    min_time_before_client_expiration: &str,
    time_before_expire: i64,
) {
    IBC_CLIENT_TIME_BEFORE_EXPIRE_COLLECTOR
        .with_label_values(&[
            chain_id,
            port_id,
            channel_id,
            destination_chain_id,
            min_time_before_client_expiration,
        ])
        .set(time_before_expire);
}

pub fn register_custom_metrics() {
    REGISTRY
        .register(Box::new(IBC_STATUS_COLLECTOR.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(IBC_COUNT_COLLECTOR.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(IBC_QUERY_STATUS_COLLECTOR.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(IBC_CLIENT_STATUS_COLLECTOR.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(IBC_CLIENT_TIME_BEFORE_EXPIRE_COLLECTOR.clone()))
        .expect("collector can be registered");
}

pub async fn metrics_handler() -> Result<impl Reply, Rejection> {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        error!("could not encode custom metrics: {:?}", e);
    };
    let mut res = String::from_utf8(buffer.clone()).unwrap_or_else(|e| {
        error!("custom metrics could not be from_utf8'd: {}", e);
        String::default()
    });
    buffer.clear();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        error!("could not encode prometheus metrics: {:?}", e);
    };
    let res_custom = String::from_utf8(buffer.clone()).unwrap_or_else(|e| {
        error!("prometheus metrics could not be from_utf8'd: {}", e);
        String::default()
    });
    buffer.clear();

    res.push_str(&res_custom);
    Ok(res)
}
