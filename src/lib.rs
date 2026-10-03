//! Conversion logic shared by the HTTP server (src/main.rs, feature "server")
//! and the wasm-bindgen crate (wasm/), which has no server and never opens a socket.

pub mod codec;
pub mod convert;
pub mod schema;
pub mod wire;
