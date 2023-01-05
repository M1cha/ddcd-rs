use futures::stream::TryStreamExt as _;
use std::convert::TryInto as _;

const VCP_INPUT: u8 = 0x60;
const VCP_BRIGHTNESS: u8 = 0x10;
const VCP_GAIN_RED: u8 = 0x16;
const VCP_GAIN_GREEN: u8 = 0x18;
const VCP_GAIN_BLUE: u8 = 0x1A;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    DDCUtil(#[from] ddcutil::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),
    #[error(transparent)]
    TryFromIntError(#[from] std::num::TryFromIntError),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error("display not found")]
    DisplayNotFound,
}

struct Calibration {
    red: u16,
    green: u16,
    blue: u16,
    max: u16,
}

struct VcpValue {
    id: u8,
    current: u16,
    max: u16,
}

impl VcpValue {
    pub fn from_display(dh: &mut ddcutil::DisplayHandle, id: u8) -> Result<Self, Error> {
        let (max, current) = dh.non_table_vcp_value(id)?;
        Ok(Self { id, current, max })
    }

    pub fn set(&mut self, dh: &mut ddcutil::DisplayHandle, value: u16) -> Result<(), Error> {
        if self.current != value {
            dh.set_non_table_vcp_value(self.id, value)?;
            self.current = value;
        }

        Ok(())
    }
}

struct Display<'a> {
    di: ddcutil::DisplayInfo<'a>,
    dh: ddcutil::DisplayHandle,

    calibration: Calibration,

    brightness: VcpValue,
    gain_red: VcpValue,
    gain_green: VcpValue,
    gain_blue: VcpValue,

    /// a value between -100 and +100 in %
    ///
    /// A positive number denotes the backlight strength percentage.
    /// A negative number denotes the software brightness decrease.
    luminance: i8,
}

impl<'a> Display<'a> {
    pub fn set_input(&mut self, id: u16) -> Result<(), Error> {
        self.dh.set_non_table_vcp_value(VCP_INPUT, id)?;
        Ok(())
    }

    pub fn set_luminance(&mut self, new_luminance: i8) -> Result<(), Error> {
        eprintln!("set luminance to {}", new_luminance);

        if new_luminance >= 0 {
            let new_luminance: u16 = new_luminance.try_into()?;

            self.gain_red.set(&mut self.dh, self.gain_red.max)?;
            self.gain_green.set(&mut self.dh, self.gain_green.max)?;
            self.gain_blue.set(&mut self.dh, self.gain_blue.max)?;

            let new_brightness =
                (self.brightness.max as f32 * new_luminance as f32 / 100.0).round() as u16;

            self.brightness.set(&mut self.dh, new_brightness)?;
        } else {
            let new_luminance: f32 = (100 - new_luminance.abs()).try_into()?;

            let new_gain_red_internal = self.calibration.red as f32 * new_luminance / 100.0;
            let new_gain_green_internal = self.calibration.green as f32 * new_luminance / 100.0;
            let new_gain_blue_internal = self.calibration.blue as f32 * new_luminance / 100.0;

            let new_gain_red = (self.gain_red.max as f32 * new_gain_red_internal
                / self.calibration.max as f32)
                .round() as u16;
            let new_gain_green = (self.gain_green.max as f32 * new_gain_green_internal
                / self.calibration.max as f32)
                .round() as u16;
            let new_gain_blue = (self.gain_blue.max as f32 * new_gain_blue_internal
                / self.calibration.max as f32)
                .round() as u16;

            self.brightness.set(&mut self.dh, 0)?;
            self.gain_red.set(&mut self.dh, new_gain_red)?;
            self.gain_green.set(&mut self.dh, new_gain_green)?;
            self.gain_blue.set(&mut self.dh, new_gain_blue)?;
        }

        self.luminance = new_luminance;

        Ok(())
    }

    pub fn increase_luminance(&mut self) -> Result<(), Error> {
        self.set_luminance((self.luminance + 5).min(100))
    }

    pub fn decrease_luminance(&mut self) -> Result<(), Error> {
        self.set_luminance((self.luminance - 5).max(-100))
    }
}

async fn handle_client_cmd(display: &mut Display<'_>, cmd: &ddcd::Command) -> Result<(), Error> {
    match cmd {
        ddcd::Command::BrightnessUp => display.increase_luminance()?,
        ddcd::Command::BrightnessDown => display.decrease_luminance()?,
        ddcd::Command::InputSource { id } => display.set_input(*id)?,
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
        tokio_serde::formats::SymmetricalBincode::<ddcd::SocketPayload>::default(),
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

        let calibration = match di.model() {
            "DELL P3221D" => Calibration {
                red: 215,
                green: 219,
                blue: 210,
                max: 219,
            },
            model => panic!("unsupported model: {}", model),
        };

        eprintln!("add display dispno={} model={}", di.dispno(), di.model());
        displays.push(Display {
            brightness: VcpValue::from_display(&mut dh, VCP_BRIGHTNESS).unwrap(),
            gain_red: VcpValue::from_display(&mut dh, VCP_GAIN_RED).unwrap(),
            gain_green: VcpValue::from_display(&mut dh, VCP_GAIN_GREEN).unwrap(),
            gain_blue: VcpValue::from_display(&mut dh, VCP_GAIN_BLUE).unwrap(),

            di,
            dh,

            calibration,

            luminance: 0,
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
