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
    brightness: u16,
    di: ddcutil::DisplayInfo<'a>,
    dh: ddcutil::DisplayHandle,
}

impl<'a> Display<'a> {
    pub fn set_brightness(&mut self, brightness: u16) -> Result<(), Error> {
        eprintln!(
            "update display dispno={} model={} _brightness={}",
            self.di.dispno(),
            self.di.model(),
            brightness
        );

        self.dh.set_non_table_vcp_value(0x10, brightness)?;
        self.brightness = brightness;
        Ok(())
    }

    pub fn increase_brightness(&mut self) -> Result<(), Error> {
        self.set_brightness(self.max_brightness.min(self.brightness + 5))
    }

    pub fn decrease_brightness(&mut self) -> Result<(), Error> {
        let new_brightness = if self.brightness >= 5 {
            self.brightness - 5
        } else {
            0
        };
        self.set_brightness(new_brightness)
    }

    pub fn set_input(&mut self, id: u16) -> Result<(), Error> {
        self.dh.set_non_table_vcp_value(0x60, id)?;
        Ok(())
    }
}

async fn handle_client_cmd(display: &mut Display<'_>, cmd: &Command) -> Result<(), Error> {
    match cmd {
        Command::BrightnessUp => display.increase_brightness()?,
        Command::BrightnessDown => display.decrease_brightness()?,
        Command::InputSource { id } => display.set_input(*id)?,
    }

    Ok(())
}

async fn handle_client(
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
            let display = displays
                .iter_mut()
                .find(|d| d.di.model() == model)
                .ok_or(Error::DisplayNotFound)?;
            handle_client_cmd(display, &payload.cmd).await?;
        } else {
            for display in displays.iter_mut() {
                handle_client_cmd(display, &payload.cmd).await?;
            }
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

        let (max_brightness, brightness) = match dh.non_table_vcp_value(0x10) {
            Err(e) => {
                eprintln!("can't get brightness of display {}: {}", di.dispno(), e);
                continue;
            }
            Ok(r) => r,
        };

        eprintln!("add display dispno={} model={}", di.dispno(), di.model());
        displays.push(Display {
            max_brightness,
            brightness,
            di,
            dh,
        });
    }

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                if let Err(e) = handle_client(stream, &mut displays).await {
                    eprintln!("handle_client: {}", e)
                }
            }
            Err(e) => eprintln!("listener error: {}", e),
        }
    }
}
