use core::cell::RefCell;

pub struct Config {
    pub watering_interval_secs: u64,
    pub pump_duration_secs: u64,
}

impl Config {
    pub const fn default() -> Self {
        Self {
            watering_interval_secs: 48 * 60 * 60,
            pump_duration_secs: 5,
        }
    }
}

pub static CONFIG: critical_section::Mutex<RefCell<Config>> =
    critical_section::Mutex::new(RefCell::new(Config::default()));
