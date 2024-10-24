#[macro_use]
extern crate lazy_static;
use cosmos_ibc_watcher::{config, query, DEFAULT_CONFIG_PATH};
use env_logger::Builder;
use log::{error, info, warn, LevelFilter};
use prometheus::{IntGaugeVec, Opts, Registry};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::result::Result;
use std::str::FromStr;
use structopt::StructOpt;
use tendermint_rpc::Url;
use warp::{Filter, Rejection, Reply};

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
        Opts::new("ibc_query_status", "IBC Query Status show the ibc total query is successful or not. 0: can access, 1: cannot access"),
        &["chain_id", "port_id", "channel_id", "destination_chain_id", "min_total"]
    )
    .expect("metric can be created");

    pub static ref REGISTRY: Registry = Registry::new();
}

/// Helper sub-commands
#[derive(Debug, StructOpt)]
#[structopt(
    name = "ibc-watcher",
    about = "watcher for total no. of cosmos-sdk chain ibc not yet relayed packet commitments associated with a channel and expose as prometheus metrics"
)]
enum IbcWatcher {
    #[structopt(name = "start", about = "start ibc watcher process")]
    Start {
        #[structopt(short)]
        config_path: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() {
    let mut builder = Builder::new();
    builder.filter_level(LevelFilter::Info).init();

    let opt = IbcWatcher::from_args();
    let result = match opt {
        IbcWatcher::Start { config_path } => start(config_path).await,
    };
    if let Err(e) = result {
        error!("{}", e);
        std::process::exit(1);
    }
}

async fn start(config_path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let default_path = format!(
        "{}/{}",
        std::env::current_exe()?.parent().unwrap().to_str().unwrap(),
        DEFAULT_CONFIG_PATH
    );
    let cp = config_path.unwrap_or_else(|| default_path.into());
    info!("config file: {}", cp.display());
    if !cp.exists() {
        Err("missing chains.toml file".into())
    } else {
        let config = config::load(cp).expect("could not parse config");

        register_custom_metrics();
        let metrics_route = warp::path!("metrics").and_then(metrics_handler);
        tokio::task::spawn(ibc_status_collector(config.clone()));

        info!(
            "Started prometheus metrics server: http://{}:{}/metrics",
            &config.prometheus.host, &config.prometheus.port
        );
        warp::serve(metrics_route)
            .run((
                Ipv4Addr::from_str(&config.prometheus.host)?,
                config.prometheus.port as u16,
            ))
            .await;
        Ok(())
    }
}

fn register_custom_metrics() {
    REGISTRY
        .register(Box::new(IBC_STATUS_COLLECTOR.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(IBC_COUNT_COLLECTOR.clone()))
        .expect("collector can be registered");
    REGISTRY
        .register(Box::new(IBC_QUERY_STATUS_COLLECTOR.clone()))
        .expect("collector can be registered");
}

async fn ibc_status_collector(config: config::Config) {
    for chain_config in config.chains.iter() {
        let grpc_addr = chain_config.grpc_addr.clone();
        let chain_id = chain_config.id.clone();
        for chain_channel in chain_config.channels.clone().iter() {
            tokio::task::spawn(track_ibc_status(
                grpc_addr.clone(),
                chain_id.clone(),
                chain_channel.clone(),
            ));
        }
    }
    if let Some(interval) = config.prometheus.reset {
        let mut reset_interval = tokio::time::interval(interval);
        loop {
            reset_interval.tick().await;
            info!("reset metrics!");
            IBC_STATUS_COLLECTOR.reset();
            IBC_COUNT_COLLECTOR.reset();
            IBC_QUERY_STATUS_COLLECTOR.reset();
        }
    }
}

async fn track_ibc_status(gprc_addr: Url, chain_id: String, chain_channel: config::Channel) {
    let port_id = &chain_channel.port_id;
    let channel_id = &chain_channel.channel_id;
    let destination_chain_id = &chain_channel.destination_chain_id;
    let refresh = &chain_channel.refresh;
    let min_total = &chain_channel.min_total;
    let mut total: u64;
    let mut collect_interval = tokio::time::interval(refresh.to_owned());

    loop {
        collect_interval.tick().await;
        total = match query::get_packet_commitments_total(
            port_id.into(),
            channel_id.into(),
            gprc_addr.to_string(),
        )
        .await
        {
            Ok(total) => {
                IBC_QUERY_STATUS_COLLECTOR
                    .with_label_values(&[
                        &chain_id.clone(),
                        port_id,
                        channel_id,
                        destination_chain_id,
                        min_total,
                    ])
                    .set(0);
                total
            }
            Err(error) => {
                error!("{} and retry next refresh", error);
                IBC_QUERY_STATUS_COLLECTOR
                    .with_label_values(&[
                        &chain_id.clone(),
                        port_id,
                        channel_id,
                        destination_chain_id,
                        min_total,
                    ])
                    .set(1);
                continue;
            }
        };
        info!(
            "The latest total={} with channel_id ({}) with destination_chain_id {} on ({})",
            total, channel_id, destination_chain_id, chain_id
        );
        if total < min_total.parse::<u64>().unwrap() {
            IBC_STATUS_COLLECTOR
                .with_label_values(&[
                    &chain_id.clone(),
                    port_id,
                    channel_id,
                    destination_chain_id,
                    min_total,
                ])
                .set(0);
        } else {
            warn!("The current total {} with channel_id ({}) is higher than {} with destination_chain_id {} on ({})", total, channel_id, min_total, destination_chain_id, chain_id);
            IBC_STATUS_COLLECTOR
                .with_label_values(&[
                    &chain_id.clone(),
                    port_id,
                    channel_id,
                    destination_chain_id,
                    min_total,
                ])
                .set(1);
        }
        IBC_COUNT_COLLECTOR
            .with_label_values(&[
                &chain_id.clone(),
                port_id,
                channel_id,
                destination_chain_id,
                min_total,
            ])
            .set(total as i64);
    }
}

async fn metrics_handler() -> Result<impl Reply, Rejection> {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        error!("could not encode custom metrics: {:?}", e);
    };
    let mut res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            error!("custom metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        error!("could not encode prometheus metrics: {:?}", e);
    };
    let res_custom = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            error!("prometheus metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();

    res.push_str(&res_custom);
    Ok(res)
}
