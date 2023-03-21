#![allow(unsafe_code)]

use tracing::info;

/// make sure this is private so no reference should be created
static mut STOP_NOW: bool = false;

/// # Safety
/// we don't care the atomicity of STOP_NOW
///
/// there should not be any reference to STOP_NOW at any time, so it's always safe to call this function
pub fn stop() {
    unsafe {
        info!("received ctrl-c command");

        STOP_NOW = true;
    }
}
/// # Safety
/// we don't care the atomicity of STOP_NOW
pub fn start() {
    unsafe {
        STOP_NOW = false;
    }
}

/// # Safety
/// we don't care the atomicity of STOP_NOW
pub fn read() -> bool {
    unsafe { STOP_NOW }
}
