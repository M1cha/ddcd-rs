use ddcd::*;
use futures::stream::TryStreamExt;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    DDCUtil(#[from] ddcutil::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error("display not found")]
    DisplayNotFound,
}

struct Display<'a> {
    max_brightness: u16,
    di: ddcutil::DisplayInfo<'a>,
    dh: ddcutil::DisplayHandle,
    max_luminance: usize,
}

impl<'a> Display<'a> {
    pub fn set_brightness(&mut self, brightness: u16) -> Result<(), Error> {
        eprintln!(
            "update display dispno={} model={} _brightness={}",
            self.di.dispno(),
            self.di.model(),
            brightness
        );

        self.dh
            .set_non_table_vcp_value(0x10, self.max_brightness.min(brightness))?;
        Ok(())
    }

    pub fn set_luminance(&mut self, luminance: u16) -> Result<(), Error> {
        self.set_brightness(
            ((self.max_brightness as f64) / (self.max_luminance as f64) * (luminance as f64))
                .round() as u16,
        )?;
        Ok(())
    }
}

struct Context {
    luminance: u16,
}

async fn handle_client_cmd(
    ctx: &mut Context,
    displays: &mut [Display<'_>],
    cmd: &Command,
) -> Result<(), Error> {
    match cmd {
        Command::BrightnessUp => {
            ctx.luminance += 5;

            for display in displays {
                display.set_luminance(ctx.luminance)?;
            }
        }
        Command::BrightnessDown => {
            if ctx.luminance >= 5 {
                ctx.luminance -= 5;
            } else {
                ctx.luminance = 0;
            }

            for display in displays {
                display.set_luminance(ctx.luminance)?;
            }
        }
        Command::InputSource { id } => {
            for display in displays {
                display.dh.set_non_table_vcp_value(0x60, *id)?;
            }
        }
    }

    Ok(())
}

async fn handle_client(
    ctx: &mut Context,
    stream: tokio::net::UnixStream,
    displays: &mut [Display<'_>],
) -> Result<(), Error> {
    let frames =
        tokio_util::codec::FramedRead::new(stream, tokio_util::codec::LengthDelimitedCodec::new());
    let mut payloads = tokio_serde::SymmetricallyFramed::new(
        frames,
        tokio_serde::formats::SymmetricalBincode::<SocketPayload>::default(),
    );

    while let Some(payload) = payloads.try_next().await? {
        if let Some(model) = payload.model {
            let i = displays
                .iter()
                .position(|d| d.di.model() == model)
                .ok_or(Error::DisplayNotFound)?;
            handle_client_cmd(ctx, &mut displays[i..i + 1], &payload.cmd).await?;
        } else {
            handle_client_cmd(ctx, displays, &payload.cmd).await?;
        }
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut listenfd = listenfd::ListenFd::from_env();
    let listener = listenfd
        .take_unix_listener(0)
        .expect("invalid fd type")
        .expect("can't get unix listener");
    let listener = tokio::net::UnixListener::from_std(listener).unwrap();

    let dil = ddcutil::DisplayInfoList::new(false).expect("can't get display info list");
    let mut displays = Vec::with_capacity(dil.len());

    for di in dil.iter() {
        let dref = di.display_ref();
        let mut dh = match dref.open_display2(true) {
            Err(e) => {
                eprintln!("can't get display {}: {}", di.dispno(), e);
                continue;
            }
            Ok(dh) => dh,
        };

        let (max_brightness, _brightness) = match dh.non_table_vcp_value(0x10) {
            Err(e) => {
                eprintln!("can't get brightness of display {}: {}", di.dispno(), e);
                continue;
            }
            Ok(r) => r,
        };

        let max_luminance = match di.model() {
            "DELL P3221D" => 350,
            "DELL P2417H" => 250,
            model => panic!("unsupported model: {}", model),
        };

        eprintln!(
            "add display dispno={} model={} max_brightness={}",
            di.dispno(),
            di.model(),
            max_brightness
        );
        displays.push(Display {
            max_brightness,
            di,
            dh,
            max_luminance,
        });
    }

    let mut ctx = Context { luminance: 0 };
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                if let Err(e) = handle_client(&mut ctx, stream, &mut displays).await {
                    eprintln!("handle_client: {}", e)
                }
            }
            Err(e) => eprintln!("listener error: {}", e),
        }
    }
}
