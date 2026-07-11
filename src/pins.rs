use core::cell::RefCell;

use esp_hal::gpio::Input;

pub static FLOAT_PIN: critical_section::Mutex<RefCell<Option<Input<'static>>>> =
    critical_section::Mutex::new(RefCell::new(None));
