use anyhow::Result;
use http::uri::Uri;
use ibc_proto::cosmos::base::query::v1beta1::PageRequest;
use ibc_proto::ibc::core::channel::v1::{query_client::QueryClient, QueryPacketCommitmentsRequest};

/// Fetches on-chain balance of given port_id, channel_id and chain
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
}
