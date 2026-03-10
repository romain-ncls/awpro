use clap::{Parser, Subcommand};
use hidapi::{HidApi, HidResult};
use std::error::Error;
use zbus::{connection, interface, Connection};

const VENDOR_ID: u16 = 0x413c;
const PRODUCT_ID: u16 = 0xa529;
const DBUS_NAME: &str = "com.awpro.Headset";
const DBUS_PATH: &str = "/com/awpro/Headset";

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "awpro", about = "Control Alienware Pro Wireless Gaming Headset")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the daemon, listening for DBus commands
    Daemon,

    /// Microphone controls
    Mic {
        #[command(subcommand)]
        action: MicCommand,
    },

    /// ANC controls
    Anc {
        #[command(subcommand)]
        action: AncCommand,
    },

    /// Set transparency mode
    Transparency {
        /// Transparency level (0–5)
        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=5))]
        level: u8,
    },
}

#[derive(Subcommand)]
enum MicCommand {
    /// Mute the microphone
    Mute,
    /// Unmute the microphone
    Unmute,
    /// Microphone ANC controls
    Anc {
        #[command(subcommand)]
        action: AncCommand,
    },
}

#[derive(Subcommand)]
enum AncCommand {
    /// Enable ANC
    On,
    /// Disable ANC
    Off,
}

// ── HID helpers ───────────────────────────────────────────────────────────────

fn set_transparency(api: &HidApi, level: u8) -> HidResult<()> {
    let device = api.open(VENDOR_ID, PRODUCT_ID)?;
    let mut data = [0u8; 62];
    data[..7].copy_from_slice(&[0x06, 0x40, 0x75, 0x02, 0x00, 0x02, level]);
    device.send_feature_report(&data)?;
    Ok(())
}

fn set_anc_off(api: &HidApi) -> HidResult<()> {
    let device = api.open(VENDOR_ID, PRODUCT_ID)?;
    let mut data = [0u8; 62];
    data[..7].copy_from_slice(&[0x06, 0x40, 0x75, 0x02, 0x00, 0x00, 0x03]);
    device.send_feature_report(&data)?;
    Ok(())
}

fn set_anc_on(api: &HidApi) -> HidResult<()> {
    let device = api.open(VENDOR_ID, PRODUCT_ID)?;
    let mut data = [0u8; 62];
    data[..7].copy_from_slice(&[0x06, 0x40, 0x75, 0x02, 0x00, 0x01, 0x03]);
    device.send_feature_report(&data)?;
    Ok(())
}

fn mic_on(api: &HidApi) -> HidResult<()> {
    let device = api.open(VENDOR_ID, PRODUCT_ID)?;
    let mut data = [0u8; 62];
    data[..6].copy_from_slice(&[0x06, 0x40, 0x73, 0x01, 0x00, 0x01]);
    device.send_feature_report(&data)?;
    Ok(())
}

fn mic_off(api: &HidApi) -> HidResult<()> {
    let device = api.open(VENDOR_ID, PRODUCT_ID)?;
    let mut data = [0u8; 62];
    data[..6].copy_from_slice(&[0x06, 0x40, 0x73, 0x01, 0x00, 0x00]);
    device.send_feature_report(&data)?;
    Ok(())
}

// ── DBus interface (daemon side) ──────────────────────────────────────────────

struct HeadsetInterface;

#[interface(name = "com.awpro.Headset1")]
impl HeadsetInterface {
    fn mic_mute(&self) -> zbus::fdo::Result<()> {
        let api = HidApi::new().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        mic_off(&api).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    fn mic_unmute(&self) -> zbus::fdo::Result<()> {
        let api = HidApi::new().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        mic_on(&api).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    fn mic_anc_on(&self) -> zbus::fdo::Result<()> {
        let api = HidApi::new().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        set_anc_on(&api).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    fn mic_anc_off(&self) -> zbus::fdo::Result<()> {
        let api = HidApi::new().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        set_anc_off(&api).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    fn anc_on(&self) -> zbus::fdo::Result<()> {
        let api = HidApi::new().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        set_anc_on(&api).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    fn anc_off(&self) -> zbus::fdo::Result<()> {
        let api = HidApi::new().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        set_anc_off(&api).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    fn set_transparency(&self, level: u8) -> zbus::fdo::Result<()> {
        let api = HidApi::new().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        set_transparency(&api, level).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }
}

// ── DBus client helpers ───────────────────────────────────────────────────────

async fn call(method: &str) -> Result<(), Box<dyn Error>> {
    let conn = Connection::session().await?;
    let proxy = zbus::Proxy::new(&conn, DBUS_NAME, DBUS_PATH, "com.awpro.Headset1").await?;
    proxy.call_method(method, &()).await?;
    Ok(())
}

async fn call_with_level(method: &str, level: u8) -> Result<(), Box<dyn Error>> {
    let conn = Connection::session().await?;
    let proxy = zbus::Proxy::new(&conn, DBUS_NAME, DBUS_PATH, "com.awpro.Headset1").await?;
    proxy.call_method(method, &(level,)).await?;
    Ok(())
}

// ── Daemon ────────────────────────────────────────────────────────────────────

async fn run_daemon() -> Result<(), Box<dyn Error>> {
    let _conn = connection::Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(DBUS_PATH, HeadsetInterface)?
        .build()
        .await?;

    println!("awpro daemon running on DBus as {DBUS_NAME}");
    println!("Press Ctrl-C to stop.");

    std::future::pending::<()>().await;
    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Daemon => run_daemon().await?,

        Command::Anc { action } => match action {
            AncCommand::On  => call("AncOn").await?,
            AncCommand::Off => call("AncOff").await?,
        },

        Command::Mic { action } => match action {
            MicCommand::Mute   => call("MicMute").await?,
            MicCommand::Unmute => call("MicUnmute").await?,
            MicCommand::Anc { action } => match action {
                AncCommand::On  => call("MicAncOn").await?,
                AncCommand::Off => call("MicAncOff").await?,
            },
        },

        Command::Transparency { level } => call_with_level("SetTransparency", level).await?,
    }

    Ok(())
}