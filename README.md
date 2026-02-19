# rointe-rs

A Rust SDK for controlling [Rointe](https://www.rointe.com/) WiFi-enabled radiators via the Firebase Realtime Database API.

## Crates

| Crate | Description |
|-------|-------------|
| [`rointe-core`](crates/rointe-core) | Core library: Firebase auth, device discovery, state reads, and control commands |
| [`rointe-cli`](crates/rointe-cli) | CLI tool for manual control and debugging (not published) |

## Quick start

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

## CLI

```bash
cargo install --path crates/rointe-cli

rointe-cli --email user@example.com --password secret devices
rointe-cli --email user@example.com --password secret status <device-id>
rointe-cli --email user@example.com --password secret set-temp <device-id> 21.5
rointe-cli --email user@example.com --password secret set-preset <device-id> comfort
rointe-cli --email user@example.com --password secret set-mode <device-id> auto
rointe-cli --email user@example.com --password secret energy <device-id>
rointe-cli --email user@example.com --password secret watch <device-id>
```

Credentials can also be provided via `ROINTE_EMAIL` and `ROINTE_PASSWORD` environment variables.

## Attribution

Protocol knowledge derived from [rointe-sdk](https://github.com/tggm/rointe-sdk) by Tiago Matias (MIT License). See [`LICENSE-THIRD-PARTY`](LICENSE-THIRD-PARTY).

## License

MIT — see [`LICENSE`](LICENSE).
