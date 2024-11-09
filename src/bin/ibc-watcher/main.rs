use cosmos_ibc_watcher::{
    config,
    handle::ibc_status_collector,
    telemetry::{metrics_handler, register_custom_metrics},
    DEFAULT_CONFIG_PATH,
};
use env_logger::Builder;
use log::{error, info, LevelFilter};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::result::Result;
use std::str::FromStr;
use structopt::StructOpt;
use warp::Filter;

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
