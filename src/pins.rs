use core::cell::RefCell;

use esp_hal::gpio::{Input, Output};

pub static RELAY_PIN: critical_section::Mutex<RefCell<Option<Output<'static>>>> =
    critical_section::Mutex::new(RefCell::new(None));
pub static FLOAT_PIN: critical_section::Mutex<RefCell<Option<Input<'static>>>> =
    critical_section::Mutex::new(RefCell::new(None));
