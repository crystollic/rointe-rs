//! Low-level Firebase Realtime Database (RTDB) client.
//!
//! - [`rtdb`] — REST client for GET and PATCH operations against the Firebase
//!   RTDB REST API. This is the only transport layer used by [`crate::RointeClient`].
//! - [`stream`] — SSE streaming (planned; not yet implemented).

pub mod rtdb;
pub mod stream;
