use serde::{Deserialize, Serialize};

/// Hourly energy consumption data returned by the `history_statistics` endpoint.
///
/// Values may be `None` if the device has not reported data for the requested
/// time slot. [`crate::RointeClient::get_energy_stats`] walks back up to five
/// hours to find the most recent non-null record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyConsumptionData {
    /// Energy consumed during the hour in kilowatt-hours.
    pub kw_h: Option<f64>,

    /// Effective (actual) power draw during the hour in watts.
    pub effective_power: Option<u32>,
}
