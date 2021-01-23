use ddcd::*;
use tokio::io::AsyncReadExt;
use tokio::stream::StreamExt;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    DDCUtil(#[from] ddcutil::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

struct Context {
    max_brightness: u16,
    brightness: u16,
    dh: ddcutil::DisplayHandle,
}

async fn handle_client(mut stream: tokio::net::UnixStream, ctx: &mut Context) -> Result<(), Error> {
    let cmd = stream.read_u8().await?;
    match cmd {
        BRIGHTNESS_UP => {
            ctx.brightness = ctx.max_brightness.min(ctx.brightness + 5);
            ctx.dh.set_non_table_vcp_value(0x10, ctx.brightness)?;
        }
        BRIGHTNESS_DOWN => {
            if ctx.brightness >= 5 {
                ctx.brightness -= 5;
            } else {
                ctx.brightness = 0;
            }
            ctx.dh.set_non_table_vcp_value(0x10, ctx.brightness)?;
        }
        INPUT_SOURCE => {
            let val = stream.read_u16().await?;
            ctx.dh.set_non_table_vcp_value(0x60, val)?;
        }
        _ => eprintln!("invalid command: {}", cmd),
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let dispno: usize = std::env::var("DDCD_DISPNO").unwrap().parse().unwrap();

    let mut listenfd = listenfd::ListenFd::from_env();
    let listener = listenfd
        .take_unix_listener(0)
        .expect("invalid fd type")
        .expect("can't get unix listener");
    let mut listener = tokio::net::UnixListener::from_std(listener).unwrap();

    let did = ddcutil::DisplayIdentifier::from_dispno(dispno).expect("can't get display id");
    let dref = did.get_display_ref().expect("can't get display ref");
    let mut dh = dref.open_display2(true).expect("can't open display");
    let (max_brightness, brightness) = dh.non_table_vcp_value(0x10).expect("can't get brightness");
    let mut ctx = Context {
        max_brightness,
        brightness,
        dh,
    };

    while let Some(stream) = listener.next().await {
        match stream {
            Ok(stream) => match handle_client(stream, &mut ctx).await {
                Ok(_) => (),
                Err(e) => eprintln!("handle_client: {}", e),
            },
            Err(e) => {
                eprintln!("listener error: {}", e);
            }
        }
    }
}
