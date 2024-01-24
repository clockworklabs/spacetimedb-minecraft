//! The configuration for the server, given from environment variables and lazy 
//! initialized when needed.

use glam::DVec3;
use once_cell::race::OnceBool;
use std::env;


/// Return true if fast entity tracking is enabled on the server. 
/// 
/// To enable this feature, set `MC173_FAST_ENTITY=1`.
pub fn fast_entity() -> bool {
    static ENV: OnceBool = OnceBool::new();
    ENV.get_or_init(|| {
        env::var_os("MC173_FAST_ENTITY")
            .map(|s| s.as_encoded_bytes() == b"1")
            .unwrap_or(false)
    })
}


/// Server world seed is currently hardcoded.
pub const SEED: i64 = 9999;

/// The spawn position is currently hardcoded.
pub const SPAWN_POS: DVec3 = DVec3::new(0.0, 100.0, 0.0);
// pub const SPAWN_POS: DVec3 = DVec3::new(12550800.0, 100.0, 12550800.0);
