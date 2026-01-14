use std::io::Result;

fn main() -> Result<()> {
    // Compile protobuf files
    prost_build::compile_protos(
        &[
            "proto/proto_demux/demux.proto",
            "proto/proto_ownership/ownership.proto",
            "proto/proto_denuvo_service/denuvo_service.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}
