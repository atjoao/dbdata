//! Generated protobuf code for Ubisoft services
//!
//! This module includes the protobuf definitions compiled by prost-build.

/// Demux protocol messages (mg.protocol.demux)
pub mod demux {
    include!(concat!(env!("OUT_DIR"), "/mg.protocol.demux.rs"));
}

/// Ownership service messages (mg.protocol.ownership)
pub mod ownership {
    include!(concat!(env!("OUT_DIR"), "/mg.protocol.ownership.rs"));
}

/// Denuvo service messages (mg.protocol.denuvo_service)
pub mod denuvo {
    include!(concat!(env!("OUT_DIR"), "/mg.protocol.denuvo_service.rs"));
}
