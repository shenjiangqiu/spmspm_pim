// if it's not wasm32, use file_server
#[cfg(not(target_arch = "wasm32"))]
mod unix_file_server;
#[cfg(not(target_arch = "wasm32"))]
pub use unix_file_server::{file_reader, FileServer};

#[cfg(target_arch = "wasm32")]
mod wasm_file_server;
#[cfg(target_arch = "wasm32")]
pub use wasm_file_server::{file_reader, FileServer};
