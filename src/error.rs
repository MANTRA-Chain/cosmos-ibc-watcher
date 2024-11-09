use flex_error::{define_error, TraceError};
use tonic::transport::Error as TransportError;

define_error! {
    Error {
        ConfigIo
            [ TraceError<std::io::Error> ]
            |_| { "config I/O error" },

        ConfigDecode
            [ TraceError<toml::de::Error> ]
            |_| { "invalid configuration" },

        ConfigEncode
            [ TraceError<toml::ser::Error> ]
            |_| { "invalid configuration" },

        ConfigParseU128
            [ TraceError<std::num::ParseIntError> ]
            |_| { "invalid number" },

        ConfigParseU64
            [ TraceError<std::num::ParseIntError> ]
            |_| { "invalid number" },

        GrpcTransport
            [ TraceError<TransportError> ]
            |_| { "error in underlying transport when making gRPC call" },

        GetPacketCommitmentsTotal
            |_| { format_args!(
                "error in getting packet commitments total")
            },

        GetChannelClientState
            |_| { format_args!(
                "error in getting channel client state")
            },

        GetChannelConsensusState
            |_| { format_args!(
                "error in getting channel consensus state")
            },
    }
}
