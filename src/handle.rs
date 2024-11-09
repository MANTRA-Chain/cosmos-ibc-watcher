use crate::{config, query, telemetry::*};
use duration_str::parse;
use ibc_relayer_types::Height;
use log::{error, info, warn};
use std::time::Duration;
use tendermint_rpc::Url;

pub async fn ibc_status_collector(config: config::Config) {
    for chain_config in config.chains.iter() {
        let grpc_addr = chain_config.grpc_addr.clone();
        let chain_id = chain_config.id.clone();
        for chain_channel in chain_config.channels.clone().iter() {
            tokio::task::spawn(track_ibc_status(
                grpc_addr.clone(),
                chain_id.clone(),
                chain_channel.clone(),
            ));
            tokio::task::spawn(track_ibc_client_status(
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

pub async fn track_ibc_client_status(
    grpc_addr: Url,
    chain_id: String,
    chain_channel: config::Channel,
) {
    let port_id = &chain_channel.port_id;
    let channel_id = &chain_channel.channel_id;
    let destination_chain_id = &chain_channel.destination_chain_id;
    let refresh = &chain_channel.refresh;
    let mut min_time_before_client_expiration: Option<Duration> = chain_channel
        .min_time_before_client_expiration
        .as_ref()
        .map(|x| parse(x).unwrap());
    let mut collect_interval = tokio::time::interval(refresh.to_owned());
    let mut last_channel_client_state_height = Height::new(0, 1).unwrap();
    let mut last_channel_client_consensus_state_duration: Option<Duration> = None;
    let mut trusting_period: Option<Duration> = None;

    loop {
        collect_interval.tick().await;

        if trusting_period.is_none() {
            info!("The trusting_period is not set, fetching from the chain");
            trusting_period = match query::get_trusting_period(
                port_id.into(),
                channel_id.into(),
                grpc_addr.to_string(),
            )
            .await
            {
                Ok(d) => {
                    ibc_query_status_setter(
                        &chain_id,
                        port_id,
                        channel_id,
                        destination_chain_id,
                        &grpc_addr.to_string(),
                        0,
                    );
                    info!("The trusting_period={:?} with channel_id ({}) with destination_chain_id {} on ({})", d, channel_id, destination_chain_id, chain_id);
                    Some(d)
                }
                Err(e) => {
                    error!("{} and retry next refresh", e);
                    ibc_query_status_setter(
                        &chain_id,
                        port_id,
                        channel_id,
                        destination_chain_id,
                        &grpc_addr.to_string(),
                        1,
                    );
                    continue;
                }
            }
        }

        if min_time_before_client_expiration.is_none() {
            info!("The min_time_before_client_expiration is not set, set it as 1/3 of trusting_period");
            min_time_before_client_expiration = Some(trusting_period.unwrap() / 3);
        }

        let channel_client_state_height = match query::get_latest_channel_client_state_height(
            port_id.into(),
            channel_id.into(),
            grpc_addr.to_string(),
        )
        .await
        {
            Ok(height) => {
                ibc_query_status_setter(
                    &chain_id,
                    port_id,
                    channel_id,
                    destination_chain_id,
                    &grpc_addr.to_string(),
                    0,
                );
                height
            }
            Err(e) => {
                error!("{} and retry next refresh", e);
                ibc_query_status_setter(
                    &chain_id,
                    port_id,
                    channel_id,
                    destination_chain_id,
                    &grpc_addr.to_string(),
                    1,
                );
                continue;
            }
        };

        if channel_client_state_height.revision_height()
            > last_channel_client_state_height.revision_height()
        {
            let channel_client_consensus_state_duration =
                match query::get_latest_channel_client_consensus_state_duration(
                    port_id.into(),
                    channel_id.into(),
                    channel_client_state_height,
                    grpc_addr.to_string(),
                )
                .await
                {
                    Ok(duration) => {
                        ibc_query_status_setter(
                            &chain_id,
                            port_id,
                            channel_id,
                            destination_chain_id,
                            &grpc_addr.to_string(),
                            0,
                        );
                        info!(
                            "The channel_client_consensus_state_duration={:?} with channel_id ({}) with destination_chain_id {} on ({})",
                            duration, channel_id, destination_chain_id, chain_id
                        );
                        last_channel_client_state_height = channel_client_state_height;
                        duration
                    }
                    Err(e) => {
                        error!("{} and retry next refresh", e);
                        ibc_query_status_setter(
                            &chain_id,
                            port_id,
                            channel_id,
                            destination_chain_id,
                            &grpc_addr.to_string(),
                            1,
                        );
                        continue;
                    }
                };

            last_channel_client_consensus_state_duration =
                Some(channel_client_consensus_state_duration);
        }

        update_ibc_client_status(
            &chain_id,
            port_id,
            channel_id,
            destination_chain_id,
            min_time_before_client_expiration.unwrap(),
            trusting_period.unwrap(),
            last_channel_client_consensus_state_duration.unwrap(),
        );
    }
}

fn update_ibc_client_status(
    chain_id: &str,
    port_id: &str,
    channel_id: &str,
    destination_chain_id: &str,
    min_time_before_client_expiration: Duration,
    trusting_period: Duration,
    last_channel_client_consensus_state_duration: Duration,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();

    let expiry_time = last_channel_client_consensus_state_duration + trusting_period;
    let min_time_before_client_expiration_str =
        min_time_before_client_expiration.as_secs().to_string() + "s";

    if expiry_time.gt(&now) {
        let time_before_expire = (expiry_time - now).as_secs().try_into().unwrap();
        ibc_client_time_before_expire_setter(
            chain_id,
            port_id,
            channel_id,
            destination_chain_id,
            &min_time_before_client_expiration_str,
            time_before_expire,
        );
        if (expiry_time - now).gt(&min_time_before_client_expiration) {
            ibc_client_status_setter(
                chain_id,
                port_id,
                channel_id,
                destination_chain_id,
                &min_time_before_client_expiration_str,
                0,
            );
        } else {
            ibc_client_status_setter(
                chain_id,
                port_id,
                channel_id,
                destination_chain_id,
                &min_time_before_client_expiration_str,
                1,
            );
        }
    } else {
        ibc_client_time_before_expire_setter(
            chain_id,
            port_id,
            channel_id,
            destination_chain_id,
            &min_time_before_client_expiration_str,
            0,
        );
        ibc_client_status_setter(
            chain_id,
            port_id,
            channel_id,
            destination_chain_id,
            &min_time_before_client_expiration_str,
            1,
        );
    }
}

pub async fn track_ibc_status(grpc_addr: Url, chain_id: String, chain_channel: config::Channel) {
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
            grpc_addr.to_string(),
        )
        .await
        {
            Ok(total) => {
                ibc_query_status_setter(
                    &chain_id,
                    port_id,
                    channel_id,
                    destination_chain_id,
                    &grpc_addr.to_string(),
                    0,
                );
                total
            }
            Err(e) => {
                error!("{} and retry next refresh", e);
                ibc_query_status_setter(
                    &chain_id,
                    port_id,
                    channel_id,
                    destination_chain_id,
                    &grpc_addr.to_string(),
                    1,
                );
                continue;
            }
        };
        info!(
            "The latest total={} with channel_id ({}) with destination_chain_id {} on ({})",
            total, channel_id, destination_chain_id, chain_id
        );
        if total < min_total.parse::<u64>().unwrap() {
            ibc_status_setter(
                &chain_id,
                port_id,
                channel_id,
                destination_chain_id,
                min_total,
                0,
            );
        } else {
            warn!("The current total {} with channel_id ({}) is higher than {} with destination_chain_id {} on ({})", total, channel_id, min_total, destination_chain_id, chain_id);
            ibc_status_setter(
                &chain_id,
                port_id,
                channel_id,
                destination_chain_id,
                min_total,
                1,
            );
        }
        ibc_count_setter(
            &chain_id,
            port_id,
            channel_id,
            destination_chain_id,
            min_total,
            total.try_into().unwrap(),
        );
    }
}
