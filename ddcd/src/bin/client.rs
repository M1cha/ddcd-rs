use clap::Parser as _;
use futures::prelude::sink::SinkExt as _;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let payload = ddcd::SocketPayload::parse();

    let stream = tokio::net::UnixStream::connect("/run/ddcd/socket")
        .await
        .unwrap();

    let frames =
        tokio_util::codec::FramedWrite::new(stream, tokio_util::codec::LengthDelimitedCodec::new());
    let mut payloads = tokio_serde::SymmetricallyFramed::new(
        frames,
        tokio_serde::formats::SymmetricalBincode::default(),
    );

    payloads.send(payload).await.unwrap();
}
