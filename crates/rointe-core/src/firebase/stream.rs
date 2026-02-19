//! Phase 2: Firebase RTDB Server-Sent Events (SSE) streaming.
//!
//! Will use `eventsource-client` to subscribe to real-time device state
//! changes, eliminating the need for polling.
//!
//! Planned API:
//! ```ignore
//! let stream = client.watch_device(device_id).await?;
//! while let Some(event) = stream.next().await {
//!     match event? {
//!         DeviceEvent::StateChanged(data) => { /* full state */ }
//!         DeviceEvent::FieldUpdated(path, value) => { /* partial update */ }
//!         DeviceEvent::KeepAlive => {}
//!     }
//! }
//! ```
