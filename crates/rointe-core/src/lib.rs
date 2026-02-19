//! # rointe-core
//!
//! Rust SDK for controlling [Rointe](https://www.rointe.com/) WiFi-enabled
//! radiators via the Firebase Realtime Database REST API.
//!
//! This crate handles authentication, device discovery, state queries, and
//! control commands. It is derived from protocol knowledge documented in
//! [rointe-sdk](https://github.com/tggm/rointe-sdk) by Tiago Matias (MIT).
//!
//! ## Quick start
//!
//! ```no_run
//! use rointe_core::{RointeClient, Preset};
//!
//! #[tokio::main]
//! async fn main() -> rointe_core::Result<()> {
//!     let client = RointeClient::new("user@example.com", "password").await?;
//!
//!     // Discover all installations and devices
//!     let installations = client.get_installations().await?;
//!     for inst in &installations {
//!         let ids = client.discover_devices(&inst.id).await?;
//!         for id in ids {
//!             let device = client.get_device(&id).await?;
//!             println!("{}: {:.1}°C", device.data.name, device.data.temp);
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **Authentication** — email/password login with automatic token refresh
//!   ([`auth::FirebaseAuth`])
//! - **Device discovery** — recursively enumerate installations and zones
//!   ([`RointeClient::get_installations`], [`RointeClient::discover_devices`])
//! - **State reads** — fetch full device state ([`RointeClient::get_device`])
//! - **Control** — set temperature, presets, and HVAC mode
//!   ([`RointeClient::set_temperature`], [`RointeClient::set_preset`],
//!   [`RointeClient::set_mode`])
//! - **Energy stats** — query hourly consumption data
//!   ([`RointeClient::get_energy_stats`])

pub mod auth;
pub mod client;
pub mod error;
pub mod firebase;
pub mod models;

// Convenient top-level re-exports
pub use client::RointeClient;
pub use error::{Result, RointeError};
pub use models::{
    device::{DeviceData, FirmwareInfo, RointeDevice},
    energy::EnergyConsumptionData,
    enums::{DeviceMode, DeviceStatus, HvacMode, Preset, RointeProduct, ScheduleMode},
    installation::{Installation, Zone},
};
