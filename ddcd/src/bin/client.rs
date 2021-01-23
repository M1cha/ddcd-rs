use ddcd::*;
use std::convert::TryInto;
use tokio::io::AsyncWriteExt;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = &args[1];
    let cmd = &args[2];

    let mut stream = tokio::net::UnixStream::connect("/run/ddcd/socket")
        .await
        .unwrap();

    stream
        .write_u8(model.len().try_into().unwrap())
        .await
        .unwrap();

    stream.write_all(model.as_bytes()).await.unwrap();

    match cmd.as_str() {
        "bl-up" => stream.write_u8(BRIGHTNESS_UP).await.unwrap(),
        "bl-down" => stream.write_u8(BRIGHTNESS_DOWN).await.unwrap(),
        "set-input" => {
            let input = u16::from_str_radix(&args[2], 16).unwrap();
            stream.write_u8(INPUT_SOURCE).await.unwrap();
            stream.write_u16(input).await.unwrap();
        }
        _ => panic!("unsupported command: {}", cmd),
    }
}
