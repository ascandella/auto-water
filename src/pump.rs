use defmt::info;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{Input, Output};

use crate::config::CONFIG;

pub fn is_float_high(pin: &Input<'static>) -> bool {
    pin.is_high()
}

pub async fn run_pump(pin: &mut Output<'static>, secs: u64) {
    pin.set_high();
    Timer::after(Duration::from_secs(secs)).await;
    pin.set_low();
}

pub async fn watering_loop(float_pin: &Input<'static>, relay_pin: &mut Output<'static>) {
    info!("Watering task started");
    loop {
        let interval =
            critical_section::with(|cs| CONFIG.borrow(cs).borrow().watering_interval_secs);
        let duration = critical_section::with(|cs| CONFIG.borrow(cs).borrow().pump_duration_secs);

        Timer::after(Duration::from_secs(interval)).await;

        if is_float_high(float_pin) {
            info!(
                "Scheduled watering: float is high, running pump for {} seconds",
                duration
            );
            run_pump(relay_pin, duration).await;
        } else {
            info!("Scheduled watering: float is low, skipping");
        }
    }
}
