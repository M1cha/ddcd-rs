pub const BRIGHTNESS_UP: u8 = 0x00;
pub const BRIGHTNESS_DOWN: u8 = 0x01;
pub const INPUT_SOURCE: u8 = 0x02;

#[derive(Debug, serde::Deserialize, serde::Serialize, clap::Parser)]
pub enum Command {
    BrightnessUp,
    BrightnessDown,
    InputSource { id: u16 },
}

#[derive(Debug, serde::Deserialize, serde::Serialize, clap::Parser)]
pub struct SocketPayload {
    pub model: Option<String>,

    #[clap(subcommand)]
    pub cmd: Command,
}
