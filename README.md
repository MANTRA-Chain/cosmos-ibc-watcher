# Cosmos IBC Watcher 🧑🏻‍🏭

This project monitors IBC (Inter-Blockchain Communication) packet commitments for Cosmos-SDK chains and exposes their status as Prometheus metrics. It allows users to track the number of unrelayed IBC packet commitments and the status of IBC clients, providing insights into the health and performance of IBC channels.

## Features

- **IBC Packet Commitments Monitoring**: Track the total number of unrelayed IBC packet commitments for each channel.
- **IBC Client Status Monitoring**: Monitor the time left before IBC clients expire and their current status.
- **Prometheus Metrics**: Expose IBC metrics in a format compatible with Prometheus for easy integration with monitoring and alerting systems.
- **Grafana Dashboard**: A pre-configured Grafana dashboard to visualize the IBC metrics.

## Build

```bash
make all
```

## Prepare config file

example
```toml
[prometheus]
host = '127.0.0.1'
port = 9090

[[chains]]
id = 'chain_A'
grpc_addr = 'http://127.0.0.1:9090'
[[chains.channels]]
port_id = 'transfer'
channel_id = 'channel-0'
destination_chain_id = 'devnet-1'
min_time_before_client_expiration = '307200s' # default is 1/3 trusting_period
min_total = '20'
refresh = '300s'
[[chains.channels]]
port_id = 'transfer'
channel_id = 'channel-1'
destination_chain_id = 'devnet-33'
min_total = '40'


[[chains]]
id = 'chain_B'
grpc_addr = 'http://127.0.0.1:9090'
[[chains.channels]]
port_id = 'transfer'
channel_id = 'channel-33'
destination_chain_id = 'testnet-1'
min_total = '20'
refresh = '300s'
```

## Run

```bash
./target/debug/ibc-watcher start -c YOUR_CONFIG_PATH
```

## Show prometheus metrics
```bash
$ curl http://127.0.0.1:9090/metrics

# HELP ibc_client_status IBC client status. 0: > min_time_before_client_expiration, 1: < min_time_before_client_expiration
# TYPE ibc_client_status gauge
ibc_client_status{chain_id="mantra-1",channel_id="channel-0",destination_chain_id="osmosis-1",min_time_before_client_expiration="537600",port_id="transfer"} 0
ibc_client_status{chain_id="mantra-1",channel_id="channel-1",destination_chain_id="noble-1",min_time_before_client_expiration="403200",port_id="transfer"} 0
ibc_client_status{chain_id="noble-1",channel_id="channel-101",destination_chain_id="mantra-1",min_time_before_client_expiration="307200",port_id="transfer"} 0
ibc_client_status{chain_id="osmosis-1",channel_id="channel-85077",destination_chain_id="mantra-1",min_time_before_client_expiration="307200",port_id="transfer"} 0
# HELP ibc_client_time_before_expire the times left before client reach ibc client expiration in seconds
# TYPE ibc_client_time_before_expire gauge
ibc_client_time_before_expire{chain_id="mantra-1",channel_id="channel-0",destination_chain_id="osmosis-1",min_time_before_client_expiration="537600s",port_id="transfer"} 806317
ibc_client_time_before_expire{chain_id="mantra-1",channel_id="channel-1",destination_chain_id="noble-1",min_time_before_client_expiration="403200s",port_id="transfer"} 1208147
ibc_client_time_before_expire{chain_id="noble-1",channel_id="channel-101",destination_chain_id="mantra-1",min_time_before_client_expiration="307200s",port_id="transfer"} 457449
ibc_client_time_before_expire{chain_id="osmosis-1",channel_id="channel-85077",destination_chain_id="mantra-1",min_time_before_client_expiration="307200s",port_id="transfer"} 460723
# HELP ibc_count no of ibc packet commitments
# TYPE ibc_count gauge
ibc_count{chain_id="mantra-1",channel_id="channel-0",destination_chain_id="osmosis-1",min_total="10",port_id="transfer"} 0
ibc_count{chain_id="mantra-1",channel_id="channel-1",destination_chain_id="noble-1",min_total="10",port_id="transfer"} 0
ibc_count{chain_id="noble-1",channel_id="channel-101",destination_chain_id="mantra-1",min_total="10",port_id="transfer"} 0
ibc_count{chain_id="osmosis-1",channel_id="channel-85077",destination_chain_id="mantra-1",min_total="10",port_id="transfer"} 1
# HELP ibc_query_status IBC Query Status show the ibc total query is successful or not. 0: can access, 1: cannot access
# TYPE ibc_query_status gauge
ibc_query_status{chain_id="mantra-1",channel_id="channel-0",destination_chain_id="osmosis-1",min_total="10",port_id="transfer"} 0
ibc_query_status{chain_id="mantra-1",channel_id="channel-1",destination_chain_id="noble-1",min_total="10",port_id="transfer"} 0
ibc_query_status{chain_id="noble-1",channel_id="channel-101",destination_chain_id="mantra-1",min_total="10",port_id="transfer"} 0
ibc_query_status{chain_id="osmosis-1",channel_id="channel-85077",destination_chain_id="mantra-1",min_total="10",port_id="transfer"} 0
# HELP ibc_status IBC Status. 0: < min_total, 1: > min_total
# TYPE ibc_status gauge
ibc_status{chain_id="mantra-1",channel_id="channel-0",destination_chain_id="osmosis-1",min_total="10",port_id="transfer"} 0
ibc_status{chain_id="mantra-1",channel_id="channel-1",destination_chain_id="noble-1",min_total="10",port_id="transfer"} 0
ibc_status{chain_id="noble-1",channel_id="channel-101",destination_chain_id="mantra-1",min_total="10",port_id="transfer"} 0
ibc_status{chain_id="osmosis-1",channel_id="channel-85077",destination_chain_id="mantra-1",min_total="10",port_id="transfer"} 0
```

Grafana Dashboard
A pre-configured Grafana dashboard is available to visualize the IBC metrics. Import the [ibc-watcher.json](./grafana/ibc-watcher.json) file into your Grafana instance to get started.
