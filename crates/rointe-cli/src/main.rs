use clap::{Parser, Subcommand};
use rointe_core::{HvacMode, Preset, RointeClient};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "rointe-cli", about = "Control Rointe WiFi radiators")]
struct Cli {
    /// Rointe account email
    #[arg(long, env = "ROINTE_EMAIL")]
    email: String,

    /// Rointe account password
    #[arg(long, env = "ROINTE_PASSWORD")]
    password: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all installations and their devices
    Devices,

    /// Show current state of a device
    Status {
        device_id: String,
    },

    /// Set target temperature (switches to manual/heat mode)
    SetTemp {
        device_id: String,
        /// Temperature in °C
        temp: f64,
    },

    /// Set a comfort preset
    SetPreset {
        device_id: String,
        /// comfort, eco, or ice
        preset: String,
    },

    /// Set HVAC mode
    SetMode {
        device_id: String,
        /// off, heat, or auto
        mode: String,
    },

    /// Show energy statistics
    Energy {
        device_id: String,
    },

    /// Stream real-time device updates
    Watch {
        device_id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let cli = Cli::parse();

    let client = RointeClient::new(&cli.email, &cli.password).await?;

    match cli.command {
        Command::Devices => cmd_devices(&client).await?,
        Command::Status { device_id } => cmd_status(&client, &device_id).await?,
        Command::SetTemp { device_id, temp } => cmd_set_temp(&client, &device_id, temp).await?,
        Command::SetPreset { device_id, preset } => {
            cmd_set_preset(&client, &device_id, &preset).await?
        }
        Command::SetMode { device_id, mode } => cmd_set_mode(&client, &device_id, &mode).await?,
        Command::Energy { device_id } => cmd_energy(&client, &device_id).await?,
        Command::Watch { device_id } => cmd_watch(&client, &device_id).await?,
    }

    Ok(())
}

async fn cmd_devices(client: &RointeClient) -> anyhow::Result<()> {
    let installations = client.get_installations().await?;

    if installations.is_empty() {
        println!("No installations found.");
        return Ok(());
    }

    for inst in &installations {
        println!("Installation: {} ({})", inst.name.as_deref().unwrap_or("—"), inst.id);

        let device_ids = client.discover_devices(&inst.id).await?;
        if device_ids.is_empty() {
            println!("  (no devices)");
            continue;
        }

        for id in &device_ids {
            match client.get_device(id).await {
                Ok(d) => println!(
                    "  {} — {} | {} | {:.1}°C | {}",
                    id,
                    d.data.name,
                    d.data.device_type,
                    d.data.temp,
                    if d.data.power { "ON" } else { "OFF" },
                ),
                Err(e) => println!("  {} — error: {e}", id),
            }
        }
    }

    Ok(())
}

async fn cmd_status(client: &RointeClient, device_id: &str) -> anyhow::Result<()> {
    let d = client.get_device(device_id).await?;
    let data = &d.data;

    println!("Device:      {} ({})", data.name, device_id);
    println!("Type:        {} v{}", data.device_type, data.product_version.as_deref().unwrap_or("?"));
    println!("Serial:      {}", d.serialnumber.as_deref().unwrap_or("—"));
    println!("Firmware:    {}", d.firmware.as_ref().and_then(|f| f.firmware_version_device.as_deref()).unwrap_or("—"));
    println!("Power:       {}", if data.power { "ON" } else { "OFF" });
    println!("Mode:        {:?}", data.mode);
    println!("Status:      {:?}", data.status);
    println!("Target temp: {:.1}°C", data.temp);
    if let Some(t) = data.temp_probe {
        println!("Probe temp:  {:.1}°C", t);
    }
    if let Some(t) = data.temp_calc {
        println!("Calc temp:   {:.1}°C", t);
    }
    println!("Comfort:     {:.1}°C", data.comfort);
    println!("Eco:         {:.1}°C", data.eco);
    println!("Ice:         {:.1}°C", data.ice);
    println!("Ice mode:    {}", data.ice_mode);
    if let Some(w) = data.nominal_power {
        println!("Power (W):   {w}");
    }

    Ok(())
}

async fn cmd_set_temp(client: &RointeClient, device_id: &str, temp: f64) -> anyhow::Result<()> {
    client.set_temperature(device_id, temp).await?;
    println!("Temperature set to {temp:.1}°C");
    Ok(())
}

async fn cmd_set_preset(
    client: &RointeClient,
    device_id: &str,
    preset: &str,
) -> anyhow::Result<()> {
    let p = match preset.to_lowercase().as_str() {
        "comfort" => Preset::Comfort,
        "eco" => Preset::Eco,
        "ice" => Preset::Ice,
        other => anyhow::bail!("Unknown preset '{other}'. Use: comfort, eco, ice"),
    };
    client.set_preset(device_id, p).await?;
    println!("Preset set to {preset}");
    Ok(())
}

async fn cmd_set_mode(client: &RointeClient, device_id: &str, mode: &str) -> anyhow::Result<()> {
    let m = match mode.to_lowercase().as_str() {
        "off" => HvacMode::Off,
        "heat" => HvacMode::Heat,
        "auto" => HvacMode::Auto,
        other => anyhow::bail!("Unknown mode '{other}'. Use: off, heat, auto"),
    };
    client.set_mode(device_id, m).await?;
    println!("Mode set to {mode}");
    Ok(())
}

async fn cmd_energy(client: &RointeClient, device_id: &str) -> anyhow::Result<()> {
    let stats = client.get_energy_stats(device_id).await?;
    match (stats.kw_h, stats.effective_power) {
        (Some(kwh), Some(w)) => println!("Energy: {kwh:.3} kWh | Effective power: {w} W"),
        (Some(kwh), None) => println!("Energy: {kwh:.3} kWh"),
        _ => println!("No energy data available for this device."),
    }
    Ok(())
}

async fn cmd_watch(client: &RointeClient, device_id: &str) -> anyhow::Result<()> {
    println!("Watching {device_id} (polling every 5s) — press Ctrl+C to stop\n");

    loop {
        match client.get_device(device_id).await {
            Ok(d) => println!(
                "[{}] power={} mode={:?} status={:?} temp={:.1}°C probe={}",
                chrono::Utc::now().format("%H:%M:%S"),
                d.data.power,
                d.data.mode,
                d.data.status,
                d.data.temp,
                d.data.temp_probe.map_or("—".to_string(), |t| format!("{t:.1}°C")),
            ),
            Err(e) => eprintln!("Error: {e}"),
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
