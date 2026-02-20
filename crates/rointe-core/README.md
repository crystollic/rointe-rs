# rointe-core

Rust SDK for controlling [Rointe](https://www.rointe.com/) WiFi-enabled radiators via the Firebase Realtime Database API.

## Usage

```toml
[dependencies]
rointe-core = "0.1"
```

```rust
use rointe_core::{RointeClient, Preset};

#[tokio::main]
async fn main() -> rointe_core::Result<()> {
    let client = RointeClient::new("user@example.com", "password").await?;

    let installations = client.get_installations().await?;
    for inst in &installations {
        let ids = client.discover_devices(&inst.id).await?;
        for id in ids {
            let device = client.get_device(&id).await?;
            println!("{}: {:.1}°C ({})",
                device.data.name,
                device.data.temp,
                if device.data.power { "ON" } else { "OFF" },
            );
        }
    }

    Ok(())
}
```

## Features

- **Authentication** — email/password login with automatic token refresh
- **Device discovery** — recursively enumerate installations and zones
- **State reads** — fetch full device state
- **Control** — set temperature, presets (`comfort` / `eco` / `ice`), and HVAC mode (`off` / `heat` / `auto`)
- **Energy stats** — query hourly consumption data

## Attribution

Protocol knowledge derived from [rointe-sdk](https://github.com/tggm/rointe-sdk) by Tiago Matias (MIT License).

## License

MIT
