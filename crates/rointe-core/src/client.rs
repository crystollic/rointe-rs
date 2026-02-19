use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Duration as ChronoDuration, Timelike, Utc};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tracing::debug;

use crate::auth::FirebaseAuth;
use crate::error::{Result, RointeError};
use crate::firebase::rtdb::RtdbClient;
use crate::models::{
    device::RointeDevice,
    energy::EnergyConsumptionData,
    enums::{HvacMode, Preset},
    installation::{Installation, Zone},
};

/// High-level API for authenticating, discovering, and controlling Rointe devices.
///
/// All methods are `async` and require a Tokio runtime. Create an instance
/// with [`RointeClient::new`] and reuse it across requests — it maintains an
/// internal HTTP connection pool and handles token refresh automatically.
pub struct RointeClient {
    auth: Arc<Mutex<FirebaseAuth>>,
    rtdb: RtdbClient,
}

impl RointeClient {
    /// Authenticate with Rointe / Firebase and return a ready client.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rointe_core::RointeClient;
    /// # async fn run() -> rointe_core::Result<()> {
    /// let client = RointeClient::new("user@example.com", "s3cr3t").await?;
    /// # Ok(()) }
    /// ```
    pub async fn new(email: &str, password: &str) -> Result<Self> {
        let client = Client::builder()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .map_err(RointeError::Network)?;

        let auth = FirebaseAuth::login(client.clone(), email, password).await?;
        let rtdb = RtdbClient::new(client);

        Ok(Self {
            auth: Arc::new(Mutex::new(auth)),
            rtdb,
        })
    }

    // ── Internal helpers ────────────────────────────────────────────────────

    async fn token(&self) -> Result<String> {
        self.auth.lock().await.ensure_valid_token().await
    }

    async fn local_id(&self) -> String {
        self.auth.lock().await.local_id.clone()
    }

    // ── Public API ───────────────────────────────────────────────────────────

    /// Return all installations belonging to the authenticated user.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rointe_core::RointeClient;
    /// # async fn run(client: &RointeClient) -> rointe_core::Result<()> {
    /// let installations = client.get_installations().await?;
    /// for inst in &installations {
    ///     println!("{}: {}", inst.id, inst.name.as_deref().unwrap_or("—"));
    /// }
    /// # Ok(()) }
    /// ```
    pub async fn get_installations(&self) -> Result<Vec<Installation>> {
        let token = self.token().await?;
        let local_id = self.local_id().await;

        // Firebase orderBy / equalTo values must be quoted JSON strings.
        let order_by = "\"userid\"";
        let equal_to = format!("\"{}\"", local_id);

        let raw: Value = self
            .rtdb
            .get(
                "/installations2.json",
                &token,
                &[("orderBy", order_by), ("equalTo", &equal_to)],
            )
            .await?;

        if raw.is_null() {
            return Ok(vec![]);
        }

        let map: std::collections::HashMap<String, Value> =
            serde_json::from_value(raw).map_err(|e| {
                RointeError::Firebase(format!("Failed to parse installations: {e}"))
            })?;

        let installations = map
            .into_iter()
            .map(|(id, value)| {
                let mut inst: Installation = serde_json::from_value(value).map_err(|e| {
                    RointeError::Firebase(format!("Failed to parse installation {id}: {e}"))
                })?;
                inst.id = id;
                Ok(inst)
            })
            .collect::<Result<Vec<_>>>()?;

        debug!("Found {} installation(s)", installations.len());
        Ok(installations)
    }

    /// Return all device IDs found within an installation (recurses through zones).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rointe_core::RointeClient;
    /// # async fn run(client: &RointeClient) -> rointe_core::Result<()> {
    /// let device_ids = client.discover_devices("installation-id").await?;
    /// println!("Found {} device(s)", device_ids.len());
    /// # Ok(()) }
    /// ```
    pub async fn discover_devices(&self, installation_id: &str) -> Result<Vec<String>> {
        let installations = self.get_installations().await?;

        let installation = installations
            .into_iter()
            .find(|i| i.id == installation_id)
            .ok_or_else(|| RointeError::DeviceNotFound(installation_id.to_string()))?;

        let mut device_ids = Vec::new();
        if let Some(zones) = &installation.zones {
            for zone in zones.values() {
                collect_device_ids(zone, &mut device_ids);
            }
        }

        debug!(
            "Discovered {} device(s) in installation {installation_id}",
            device_ids.len()
        );
        Ok(device_ids)
    }

    /// Fetch the current state of a single device.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rointe_core::RointeClient;
    /// # async fn run(client: &RointeClient) -> rointe_core::Result<()> {
    /// let device = client.get_device("device-id").await?;
    /// println!("{}: {:.1}°C ({})", device.data.name, device.data.temp,
    ///     if device.data.power { "ON" } else { "OFF" });
    /// # Ok(()) }
    /// ```
    pub async fn get_device(&self, device_id: &str) -> Result<RointeDevice> {
        let token = self.token().await?;
        let path = format!("/devices/{device_id}.json");
        self.rtdb.get(&path, &token, &[]).await
    }

    /// Set the target temperature (switches to manual mode, powers on).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rointe_core::RointeClient;
    /// # async fn run(client: &RointeClient) -> rointe_core::Result<()> {
    /// client.set_temperature("device-id", 21.5).await?;
    /// # Ok(()) }
    /// ```
    pub async fn set_temperature(&self, device_id: &str, temp: f64) -> Result<()> {
        let token = self.token().await?;
        let path = format!("/devices/{device_id}/data.json");
        let now = Utc::now().timestamp_millis();

        let body = json!({
            "temp": temp,
            "mode": "manual",
            "power": true,
            "last_sync_datetime_app": now,
        });

        self.rtdb.patch(&path, &token, &body).await
    }

    /// Activate a comfort preset (comfort / eco / ice).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rointe_core::{RointeClient, Preset};
    /// # async fn run(client: &RointeClient) -> rointe_core::Result<()> {
    /// client.set_preset("device-id", Preset::Eco).await?;
    /// # Ok(()) }
    /// ```
    pub async fn set_preset(&self, device_id: &str, preset: Preset) -> Result<()> {
        let device = self.get_device(device_id).await?;
        let token = self.token().await?;
        let path = format!("/devices/{device_id}/data.json");
        let now = Utc::now().timestamp_millis();

        let (temp, status) = match preset {
            Preset::Comfort => (device.data.comfort, "comfort"),
            Preset::Eco => (device.data.eco, "eco"),
            Preset::Ice => (device.data.ice, "ice"),
        };

        let body = json!({
            "power": true,
            "mode": "manual",
            "temp": temp,
            "status": status,
            "last_sync_datetime_app": now,
        });

        self.rtdb.patch(&path, &token, &body).await
    }

    /// Set the HVAC operating mode.
    ///
    /// - [`HvacMode::Off`]  — two-step power-off sequence
    /// - [`HvacMode::Heat`] — two-step power-on (heats to comfort temperature)
    /// - [`HvacMode::Auto`] — two-step switch to schedule-following auto mode
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rointe_core::{RointeClient, HvacMode};
    /// # async fn run(client: &RointeClient) -> rointe_core::Result<()> {
    /// client.set_mode("device-id", HvacMode::Auto).await?;
    /// # Ok(()) }
    /// ```
    pub async fn set_mode(&self, device_id: &str, mode: HvacMode) -> Result<()> {
        let device = self.get_device(device_id).await?;
        let token = self.token().await?;
        let path = format!("/devices/{device_id}/data.json");

        match mode {
            HvacMode::Off => {
                let now = Utc::now().timestamp_millis();
                let step1 = json!({ "temp": 20, "last_sync_datetime_app": now });
                self.rtdb.patch(&path, &token, &step1).await?;

                let now2 = Utc::now().timestamp_millis();
                let step2 = json!({
                    "power": false,
                    "mode": "manual",
                    "status": "off",
                    "last_sync_datetime_app": now2,
                });
                self.rtdb.patch(&path, &token, &step2).await
            }

            HvacMode::Heat => {
                let now = Utc::now().timestamp_millis();
                let step1 = json!({
                    "temp": device.data.comfort,
                    "last_sync_datetime_app": now,
                });
                self.rtdb.patch(&path, &token, &step1).await?;

                let now2 = Utc::now().timestamp_millis();
                let step2 = json!({
                    "mode": "manual",
                    "power": true,
                    "status": "none",
                    "last_sync_datetime_app": now2,
                });
                self.rtdb.patch(&path, &token, &step2).await
            }

            HvacMode::Auto => {
                let schedule_temp = schedule_temp_for_now(&device);
                let now = Utc::now().timestamp_millis();
                let step1 = json!({
                    "temp": schedule_temp,
                    "last_sync_datetime_app": now,
                });
                self.rtdb.patch(&path, &token, &step1).await?;

                let now2 = Utc::now().timestamp_millis();
                let step2 = json!({
                    "mode": "auto",
                    "power": true,
                    "last_sync_datetime_app": now2,
                });
                self.rtdb.patch(&path, &token, &step2).await
            }
        }
    }

    /// Return the most recent hourly energy stats for a device.
    ///
    /// Tries the current hour and walks back up to 5 hours to find a non-null
    /// record. Returns an empty [`EnergyConsumptionData`] if no data is found
    /// (not all device models report energy statistics).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rointe_core::RointeClient;
    /// # async fn run(client: &RointeClient) -> rointe_core::Result<()> {
    /// let stats = client.get_energy_stats("device-id").await?;
    /// if let Some(kwh) = stats.kw_h {
    ///     println!("Last hour: {kwh:.3} kWh");
    /// }
    /// # Ok(()) }
    /// ```
    pub async fn get_energy_stats(
        &self,
        device_id: &str,
    ) -> Result<EnergyConsumptionData> {
        let token = self.token().await?;
        let now = Utc::now();

        for hours_back in 0..=5i64 {
            let dt = now - ChronoDuration::hours(hours_back);
            let path = format!(
                "/history_statistics/{}/daily/{}/{}/{}/energy/{}0000.json",
                device_id,
                dt.format("%Y"),
                dt.format("%m"),
                dt.format("%d"),
                dt.format("%H"),
            );

            if let Ok(data) = self
                .rtdb
                .get::<EnergyConsumptionData>(&path, &token, &[])
                .await
            {
                if data.kw_h.is_some() {
                    return Ok(data);
                }
            }
        }

        Ok(EnergyConsumptionData {
            kw_h: None,
            effective_power: None,
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Recursively collect all device IDs from a zone and its sub-zones.
fn collect_device_ids(zone: &Zone, device_ids: &mut Vec<String>) {
    if let Some(devices) = &zone.devices {
        device_ids.extend(devices.keys().cloned());
    }
    if let Some(sub_zones) = &zone.zones {
        for sub_zone in sub_zones.values() {
            collect_device_ids(sub_zone, device_ids);
        }
    }
}

/// Determine the temperature the device should target right now based on its
/// weekly schedule, ice mode, and the current UTC time.
///
/// Split into a testable inner function that takes explicit day and hour.
fn schedule_temp_for_now(device: &RointeDevice) -> f64 {
    let now = Utc::now();
    let day = now.weekday().num_days_from_monday() as usize; // 0 = Monday
    let hour = now.hour() as usize;
    schedule_temp(device, day, hour)
}

fn schedule_temp(device: &RointeDevice, day: usize, hour: usize) -> f64 {
    if device.data.ice_mode {
        return device.data.ice;
    }

    if let Some(schedule) = &device.data.schedule {
        if let Some(day_str) = schedule.get(day) {
            if let Some(slot) = day_str.chars().nth(hour) {
                return match slot {
                    'C' => device.data.comfort,
                    'E' => device.data.eco,
                    _ => 20.0,
                };
            }
        }
    }

    20.0
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        device::{DeviceData, FirmwareInfo, RointeDevice},
        enums::{DeviceMode, DeviceStatus},
    };

    fn make_device(ice_mode: bool) -> RointeDevice {
        RointeDevice {
            data: DeviceData {
                name: "Test Radiator".to_string(),
                device_type: "radiator".to_string(),
                product_version: Some("v2".to_string()),
                nominal_power: Some(1500),
                power: true,
                mode: DeviceMode::Manual,
                status: DeviceStatus::Comfort,
                temp: 21.0,
                temp_calc: None,
                temp_probe: None,
                comfort: 21.0,
                eco: 18.0,
                ice: 8.0,
                ice_mode,
                // "CCCCCCCCEEEEEEEEEEEEEECC": hours 0-7 = C, 8-21 = E, 22-23 = C
                schedule: Some(vec![
                    "CCCCCCCCEEEEEEEEEEEEEECC".to_string(),
                    "CCCCCCCCEEEEEEEEEEEEEECC".to_string(),
                    "CCCCCCCCEEEEEEEEEEEEEECC".to_string(),
                    "CCCCCCCCEEEEEEEEEEEEEECC".to_string(),
                    "CCCCCCCCEEEEEEEEEEEEEECC".to_string(),
                    "CCCCCCCCCCCCCCCCCCCCCCCC".to_string(),
                    "CCCCCCCCCCCCCCCCCCCCCCCC".to_string(),
                ]),
                schedule_day: Some(0),
                schedule_hour: Some(0),
                um_max_temp: Some(30.0),
                um_min_temp: Some(7.0),
                user_mode: Some(false),
                last_sync_datetime_app: 1708360000000,
                last_sync_datetime_device: None,
            },
            serialnumber: Some("ROINTE12345".to_string()),
            firmware: Some(FirmwareInfo {
                firmware_version_device: Some("3.2.1".to_string()),
            }),
        }
    }

    #[test]
    fn test_schedule_temp_comfort_hour() {
        let device = make_device(false);
        // Monday hour 0 → 'C' → comfort (21.0)
        assert_eq!(schedule_temp(&device, 0, 0), 21.0);
        assert_eq!(schedule_temp(&device, 0, 7), 21.0);
    }

    #[test]
    fn test_schedule_temp_eco_hour() {
        let device = make_device(false);
        // Monday hour 8 → 'E' → eco (18.0)
        assert_eq!(schedule_temp(&device, 0, 8), 18.0);
        assert_eq!(schedule_temp(&device, 0, 21), 18.0);
    }

    #[test]
    fn test_schedule_temp_comfort_late_evening() {
        let device = make_device(false);
        // Monday hour 22 → 'C' → comfort (21.0)
        assert_eq!(schedule_temp(&device, 0, 22), 21.0);
        assert_eq!(schedule_temp(&device, 0, 23), 21.0);
    }

    #[test]
    fn test_schedule_temp_ice_mode_overrides() {
        let device = make_device(true);
        // Ice mode always returns ice temp regardless of schedule
        assert_eq!(schedule_temp(&device, 0, 0), 8.0);
        assert_eq!(schedule_temp(&device, 5, 12), 8.0);
    }

    #[test]
    fn test_schedule_temp_weekend_all_comfort() {
        let device = make_device(false);
        // Saturday (day 5) is all C's
        for hour in 0..24 {
            assert_eq!(schedule_temp(&device, 5, hour), 21.0);
        }
    }

    #[test]
    fn test_schedule_temp_no_schedule_fallback() {
        let mut device = make_device(false);
        device.data.schedule = None;
        // No schedule → fallback 20.0
        assert_eq!(schedule_temp(&device, 0, 12), 20.0);
    }
}
