use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(
    name = "awpro",
    about = "Control the Dell Alienware Pro Wireless Gaming Headset"
)]
pub struct Cli {
    /// Output results as JSON
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

// ── Top-level commands ────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum Command {
    /// Control ANC and transparency mode
    Anc(AncArgs),
    /// Control microphone settings
    Mic(MicArgs),
    /// Set sidetone (local monitoring) level
    Sidetone(SidetoneArgs),
    /// Control power-related settings
    Power(PowerArgs),
    /// Query battery level
    Battery,
    /// Query current device state
    Get(GetArgs),
}

// ── anc ───────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct AncArgs {
    #[command(subcommand)]
    pub command: AncCommand,
}

#[derive(Subcommand)]
pub enum AncCommand {
    /// Disable ANC and transparency
    Off,
    /// Enable ANC
    On,
    /// Enable transparency mode
    Transparency {
        /// Transparency level (1–5); omit to keep current level
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=5))]
        level: Option<u8>,
    },
}

// ── mic ───────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct MicArgs {
    #[command(subcommand)]
    pub command: MicCommand,
}

#[derive(Subcommand)]
pub enum MicCommand {
    /// Mute the microphone
    Mute,
    /// Unmute the microphone
    Unmute,
    /// Control microphone noise cancellation
    NoiseCancel {
        /// Enable or disable noise cancellation
        toggle: Toggle,
    },
}

#[derive(ValueEnum, Clone)]
pub enum Toggle {
    On,
    Off,
}

// ── sidetone ──────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct SidetoneArgs {
    /// Sidetone level
    pub level: SidetoneLevel,
}

#[derive(ValueEnum, Clone)]
pub enum SidetoneLevel {
    Off,
    #[value(name = "1")]
    L1,
    #[value(name = "2")]
    L2,
    #[value(name = "3")]
    L3,
    #[value(name = "4")]
    L4,
    #[value(name = "5")]
    L5,
}

impl SidetoneLevel {
    pub fn as_byte(&self) -> u8 {
        match self {
            Self::Off => 0x00,
            Self::L1 => 0x01,
            Self::L2 => 0x02,
            Self::L3 => 0x03,
            Self::L4 => 0x04,
            Self::L5 => 0x05,
        }
    }
}

// ── power ─────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct PowerArgs {
    #[command(subcommand)]
    pub command: PowerCommand,
}

#[derive(Subcommand)]
pub enum PowerCommand {
    /// Configure auto power-off timer
    AutoOff {
        /// Timer interval in minutes, or off
        interval: AutoOffInterval,
    },
    /// Configure power saving mode
    Saving {
        #[command(subcommand)]
        cmd: SavingCommand,
    },
}

#[derive(ValueEnum, Clone)]
pub enum AutoOffInterval {
    Off,
    #[value(name = "15")]
    Min15,
    #[value(name = "30")]
    Min30,
    #[value(name = "45")]
    Min45,
    #[value(name = "60")]
    Min60,
}

#[derive(Subcommand)]
pub enum SavingCommand {
    /// Disable power saving
    Off,
    /// Enable power saving
    On {
        /// Battery threshold percentage to activate saving mode (5–100)
        #[arg(long, value_parser = clap::value_parser!(u8).range(5..=100))]
        threshold: u8,
    },
}

// ── get ───────────────────────────────────────────────────────────────────────

#[derive(Args)]
pub struct GetArgs {
    #[command(subcommand)]
    pub command: GetCommand,
}

#[derive(Subcommand)]
pub enum GetCommand {
    /// Query ANC / transparency state
    Anc,
    /// Query microphone state
    Mic {
        #[command(subcommand)]
        field: Option<MicField>,
    },
    /// Query sidetone level
    Sidetone,
    /// Query power settings
    Power {
        #[command(subcommand)]
        field: PowerField,
    },
    /// Query battery level
    Battery,
}

#[derive(Subcommand, Clone)]
pub enum MicField {
    /// Query noise cancellation state
    NoiseCancel,
}

#[derive(Subcommand, Clone)]
pub enum PowerField {
    /// Query power saving state
    Saving,
    /// Query auto power-off state
    AutoOff,
}
