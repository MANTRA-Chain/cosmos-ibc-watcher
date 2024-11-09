use anyhow::Result;
use http::uri::Uri;
use ibc_proto::cosmos::base::query::v1beta1::PageRequest;
use ibc_proto::ibc::core::channel::v1::{
    query_client::QueryClient, QueryChannelClientStateRequest, QueryChannelConsensusStateRequest,
    QueryPacketCommitmentsRequest,
};
use ibc_relayer::client_state::IdentifiedAnyClientState;
use ibc_relayer::consensus_state::AnyConsensusState;
use ibc_relayer_types::Height;
use std::time::Duration;

/// Fetches on-chain data of given port_id, channel_id and chain
pub async fn get_packet_commitments_total(
    port_id: String,
    channel_id: String,
    grpc_addr: String,
) -> Result<u64> {
    let mut query_client = create_grpc_client(grpc_addr.parse::<Uri>()?, QueryClient::new).await?;

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

/// Fetches trusting period of the channel
pub async fn get_trusting_period(
    port_id: String,
    channel_id: String,
    grpc_addr: String,
) -> Result<Duration> {
    let mut query_client = create_grpc_client(grpc_addr.parse::<Uri>()?, QueryClient::new).await?;

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

/// Fetch the latest client state height of the channel
pub async fn get_latest_channel_client_state_height(
    port_id: String,
    channel_id: String,
    grpc_addr: String,
) -> Result<Height> {
    let mut query_client = create_grpc_client(grpc_addr.parse::<Uri>()?, QueryClient::new).await?;

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

/// Fetch the duration of the latest ibc client consensus state by height
pub async fn get_latest_channel_client_consensus_state_duration(
    port_id: String,
    channel_id: String,
    height: Height,
    grpc_addr: String,
) -> Result<Duration> {
    let mut query_client = create_grpc_client(grpc_addr.parse::<Uri>()?, QueryClient::new).await?;

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
        let grpc_addr = "https://grpc.mantrachain.io".to_string();
        let total = get_packet_commitments_total(port_id, channel_id, grpc_addr)
            .await
            .unwrap();
        println!("{:?}", total);
        assert_ge!(total, 0);
    }

    #[actix_rt::test]
    async fn test_get_trusting_period() {
        let port_id = "transfer".to_string();
        let channel_id = "channel-0".to_string();
        let grpc_addr = "https://grpc.mantrachain.io".to_string();
        let duration = get_trusting_period(port_id, channel_id, grpc_addr)
            .await
            .unwrap();
        println!("{:?}", duration);
        assert_ge!(duration.as_secs(), 0);
    }

    #[actix_rt::test]
    async fn test_get_latest_channel_client_state_height() {
        let port_id = "transfer".to_string();
        let channel_id = "channel-0".to_string();
        let grpc_addr = "https://grpc.mantrachain.io".to_string();
        let height = get_latest_channel_client_state_height(port_id, channel_id, grpc_addr)
            .await
            .unwrap();
        println!("{:?}", height);
        assert_ge!(height.revision_height(), 0);
    }

    #[actix_rt::test]
    async fn test_get_latest_channel_client_consensus_state_duration() {
        let port_id = "transfer".to_string();
        let channel_id = "channel-0".to_string();
        let grpc_addr = "https://grpc.mantrachain.io".to_string();
        let height = get_latest_channel_client_state_height(
            port_id.clone(),
            channel_id.clone(),
            grpc_addr.clone(),
        )
        .await
        .unwrap();
        let duration = get_latest_channel_client_consensus_state_duration(
            port_id, channel_id, height, grpc_addr,
        )
        .await
        .unwrap();
        println!("{:?}", duration);
        assert_ge!(duration.as_secs(), 0);
    }
}
