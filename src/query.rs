use anyhow::Result;
use http::uri::Uri;
use ibc_proto::cosmos::base::query::v1beta1::PageRequest;
use ibc_proto::cosmos::base::tendermint::v1beta1::{
    service_client::ServiceClient, GetLatestBlockRequest,
};
use ibc_proto::ibc::core::channel::v1::{
    query_client::QueryClient, QueryChannelClientStateRequest, QueryChannelConsensusStateRequest,
    QueryPacketCommitmentsRequest,
};
use log::warn;

use ibc_relayer::client_state::IdentifiedAnyClientState;
use ibc_relayer::consensus_state::AnyConsensusState;
use ibc_relayer_types::Height;
use std::time::Duration;

/// The outcome of a query attempted across a list of fallback gRPC endpoints:
/// the final result, plus the endpoint that produced it (the one that
/// succeeded, or the last one tried if all of them failed).
pub struct QueryOutcome<T> {
    pub result: Result<T>,
    pub grpc_addr: String,
}

/// Runs `attempt` against each entry of `grpc_addrs` in order, returning as
/// soon as one succeeds. Every failed attempt is logged with its endpoint
/// before moving on to the next one.
async fn query_with_failover<T, F, Fut>(grpc_addrs: &[String], mut attempt: F) -> QueryOutcome<T>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_addr = grpc_addrs.first().cloned().unwrap_or_default();
    let mut last_err: Option<anyhow::Error> = None;

    for grpc_addr in grpc_addrs {
        last_addr = grpc_addr.clone();
        match attempt(grpc_addr.clone()).await {
            Ok(value) => {
                return QueryOutcome {
                    result: Ok(value),
                    grpc_addr: last_addr,
                }
            }
            Err(err) => {
                warn!(
                    "gRPC query via {} failed: {}; trying next endpoint if available",
                    grpc_addr, err
                );
                last_err = Some(err);
            }
        }
    }

    QueryOutcome {
        result: Err(last_err.unwrap_or_else(|| anyhow::anyhow!("grpc_addrs is empty"))),
        grpc_addr: last_addr,
    }
}

/// Fetches on-chain data of given port_id, channel_id and chain
pub async fn get_packet_commitments_total(
    port_id: String,
    channel_id: String,
    grpc_addrs: &[String],
) -> QueryOutcome<u64> {
    query_with_failover(grpc_addrs, |grpc_addr| {
        let port_id = port_id.clone();
        let channel_id = channel_id.clone();
        async move {
            let mut query_client =
                create_grpc_client(grpc_addr.parse::<Uri>()?, QueryClient::new).await?;

            let page_request = PageRequest {
                key: vec![],
                offset: 1,
                limit: 100,
                count_total: true,
                reverse: true,
            };
            let request = QueryPacketCommitmentsRequest {
                port_id,
                channel_id,
                pagination: Some(page_request),
            };

            Ok(query_client
                .packet_commitments(request)
                .await?
                .into_inner()
                .pagination
                .map(|x| x.total)
                .ok_or_else(crate::error::Error::get_packet_commitments_total)?)
        }
    })
    .await
}

/// Fetches trusting period of the channel
pub async fn get_trusting_period(
    port_id: String,
    channel_id: String,
    grpc_addrs: &[String],
) -> QueryOutcome<Duration> {
    query_with_failover(grpc_addrs, |grpc_addr| {
        let port_id = port_id.clone();
        let channel_id = channel_id.clone();
        async move {
            let mut query_client =
                create_grpc_client(grpc_addr.parse::<Uri>()?, QueryClient::new).await?;

            let request = QueryChannelClientStateRequest {
                port_id,
                channel_id,
            };

            Ok(query_client
                .channel_client_state(request)
                .await?
                .into_inner()
                .identified_client_state
                .ok_or_else(crate::error::Error::get_channel_client_state)
                .map(IdentifiedAnyClientState::try_from)??
                .client_state
                .trusting_period())
        }
    })
    .await
}

/// Fetch the latest client state height of the channel
pub async fn get_latest_channel_client_state_height(
    port_id: String,
    channel_id: String,
    grpc_addrs: &[String],
) -> QueryOutcome<Height> {
    query_with_failover(grpc_addrs, |grpc_addr| {
        let port_id = port_id.clone();
        let channel_id = channel_id.clone();
        async move {
            let mut query_client =
                create_grpc_client(grpc_addr.parse::<Uri>()?, QueryClient::new).await?;

            let request = QueryChannelClientStateRequest {
                port_id,
                channel_id,
            };

            Ok(query_client
                .channel_client_state(request)
                .await?
                .into_inner()
                .identified_client_state
                .ok_or_else(crate::error::Error::get_channel_client_state)
                .map(IdentifiedAnyClientState::try_from)??
                .client_state
                .latest_height())
        }
    })
    .await
}

/// Fetch the duration of the latest ibc client consensus state by height
pub async fn get_latest_channel_client_consensus_state_duration(
    port_id: String,
    channel_id: String,
    height: Height,
    grpc_addrs: &[String],
) -> QueryOutcome<Duration> {
    query_with_failover(grpc_addrs, |grpc_addr| {
        let port_id = port_id.clone();
        let channel_id = channel_id.clone();
        async move {
            let mut query_client =
                create_grpc_client(grpc_addr.parse::<Uri>()?, QueryClient::new).await?;

            let request = QueryChannelConsensusStateRequest {
                port_id,
                channel_id,
                revision_height: height.revision_height(),
                revision_number: height.revision_number(),
            };

            Ok(Duration::from_nanos(
                query_client
                    .channel_consensus_state(request)
                    .await?
                    .into_inner()
                    .consensus_state
                    .ok_or_else(crate::error::Error::get_channel_consensus_state)
                    .map(AnyConsensusState::try_from)??
                    .timestamp()
                    .nanoseconds(),
            ))
        }
    })
    .await
}

/// fetches the latest block height of the chain
pub async fn get_latest_height(grpc_addrs: &[String]) -> QueryOutcome<i64> {
    query_with_failover(grpc_addrs, |grpc_addr| async move {
        let mut query_client =
            create_grpc_client(grpc_addr.parse::<Uri>()?, ServiceClient::new).await?;

        Ok(query_client
            .get_latest_block(GetLatestBlockRequest {})
            .await?
            .into_inner()
            .block
            .ok_or_else(crate::error::Error::get_latest_block)?
            .header
            .ok_or_else(crate::error::Error::get_latest_block)?
            .height)
    })
    .await
}

/// Helper function to create a gRPC client.
pub async fn create_grpc_client<T>(
    grpc_addr: Uri,
    client_constructor: impl FnOnce(tonic::transport::Channel) -> T,
) -> Result<T, crate::error::Error> {
    let tls_config = tonic::transport::ClientTlsConfig::new().with_native_roots();
    let channel = tonic::transport::Channel::builder(grpc_addr)
        .tls_config(tls_config)
        .map_err(crate::error::Error::grpc_transport)?
        .connect()
        .await
        .map_err(crate::error::Error::grpc_transport)?;
    Ok(client_constructor(channel))
}

#[cfg(test)]
mod tests {
    use super::*;
    // TODO: use mock server instead
    #[actix_rt::test]
    async fn test_get_packet_commitments_total() {
        let port_id = "transfer".to_string();
        let channel_id = "channel-0".to_string();
        let grpc_addrs = vec!["https://grpc.mantrachain.io".to_string()];
        let total = get_packet_commitments_total(port_id, channel_id, &grpc_addrs)
            .await
            .result
            .unwrap();
        println!("{:?}", total);
        assert_ge!(total, 0);
    }

    #[actix_rt::test]
    async fn test_get_trusting_period() {
        let port_id = "transfer".to_string();
        let channel_id = "channel-0".to_string();
        let grpc_addrs = vec!["https://grpc.mantrachain.io".to_string()];
        let duration = get_trusting_period(port_id, channel_id, &grpc_addrs)
            .await
            .result
            .unwrap();
        println!("{:?}", duration);
        assert_ge!(duration.as_secs(), 0);
    }

    #[actix_rt::test]
    async fn test_get_latest_channel_client_state_height() {
        let port_id = "transfer".to_string();
        let channel_id = "channel-0".to_string();
        let grpc_addrs = vec!["https://grpc.mantrachain.io".to_string()];
        let height = get_latest_channel_client_state_height(port_id, channel_id, &grpc_addrs)
            .await
            .result
            .unwrap();
        println!("{:?}", height);
        assert_ge!(height.revision_height(), 0);
    }

    #[actix_rt::test]
    async fn test_get_latest_channel_client_consensus_state_duration() {
        let port_id = "transfer".to_string();
        let channel_id = "channel-0".to_string();
        let grpc_addrs = vec!["https://grpc.mantrachain.io".to_string()];
        let height = get_latest_channel_client_state_height(
            port_id.clone(),
            channel_id.clone(),
            &grpc_addrs,
        )
        .await
        .result
        .unwrap();
        let duration = get_latest_channel_client_consensus_state_duration(
            port_id,
            channel_id,
            height,
            &grpc_addrs,
        )
        .await
        .result
        .unwrap();
        println!("{:?}", duration);
        assert_ge!(duration.as_secs(), 0);
    }

    #[actix_rt::test]
    async fn test_get_latest_height() {
        let grpc_addrs = vec!["https://grpc.mantrachain.io".to_string()];
        let height = get_latest_height(&grpc_addrs).await.result.unwrap();
        println!("{:?}", height);
        assert_ge!(height, 0);
    }

    #[actix_rt::test]
    async fn test_get_latest_height_falls_back_to_working_endpoint() {
        // The first endpoint is unreachable, the second is valid; the query
        // should fail over and report the endpoint that actually succeeded.
        let grpc_addrs = vec![
            "http://127.0.0.1:1".to_string(),
            "https://grpc.mantrachain.io".to_string(),
        ];
        let outcome = get_latest_height(&grpc_addrs).await;
        assert!(outcome.result.is_ok());
        assert_eq!(outcome.grpc_addr, "https://grpc.mantrachain.io");
    }
}
