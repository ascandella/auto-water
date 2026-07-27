use core::cell::RefCell;

use defmt::info;
use esp_nvs::Key;
use esp_storage::FlashStorage;

use crate::partitions::find_partition;

const NVS: (usize, usize) = find_partition("nvs");
const NVS_PARTITION_OFFSET: usize = NVS.0;
const NVS_PARTITION_SIZE: usize = NVS.1;

const NS: Key = Key::from_str("config");
const KEY_INTERVAL: Key = Key::from_str("interval");
const KEY_DURATION: Key = Key::from_str("duration");

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

pub static FLASH: critical_section::Mutex<RefCell<Option<FlashStorage<'static>>>> =
    critical_section::Mutex::new(RefCell::new(None));

pub fn store_flash(flash: FlashStorage<'static>) {
    critical_section::with(|cs| {
        *FLASH.borrow(cs).borrow_mut() = Some(flash);
    });
}

pub fn take_flash() -> FlashStorage<'static> {
    critical_section::with(|cs| {
        FLASH
            .borrow(cs)
            .borrow_mut()
            .take()
            .expect("flash not initialized")
    })
}

pub fn load_from_nvs() {
    let flash = take_flash();

    match esp_nvs::Nvs::new(NVS_PARTITION_OFFSET, NVS_PARTITION_SIZE, flash) {
        Ok(mut nvs) => {
            critical_section::with(|cs| {
                let mut cfg = CONFIG.borrow(cs).borrow_mut();

                if let Ok(val) = nvs.get::<u64>(&NS, &KEY_INTERVAL) {
                    info!("NVS: loaded interval = {} secs", val);
                    cfg.watering_interval_secs = val;
                }
                if let Ok(val) = nvs.get::<u64>(&NS, &KEY_DURATION) {
                    info!("NVS: loaded duration = {} secs", val);
                    cfg.pump_duration_secs = val;
                }
            });

            store_flash(nvs.into_inner());
        }
        Err(_) => {
            info!("NVS: failed to open for reading, using defaults");
        }
    }
}

pub fn save_to_nvs() {
    let flash = take_flash();

    match esp_nvs::Nvs::new(NVS_PARTITION_OFFSET, NVS_PARTITION_SIZE, flash) {
        Ok(mut nvs) => {
            critical_section::with(|cs| {
                let cfg = CONFIG.borrow(cs).borrow();

                if nvs
                    .set::<u64>(&NS, &KEY_INTERVAL, cfg.watering_interval_secs)
                    .is_err()
                {
                    info!("NVS: failed to write interval");
                }
                if nvs
                    .set::<u64>(&NS, &KEY_DURATION, cfg.pump_duration_secs)
                    .is_err()
                {
                    info!("NVS: failed to write duration");
                }
            });

            info!("NVS: config saved");
            store_flash(nvs.into_inner());
        }
        Err(_) => {
            info!("NVS: failed to open for writing");
        }
    }
}
