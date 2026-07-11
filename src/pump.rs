use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;

use crate::config::CONFIG;
use crate::pins::FLOAT_PIN;

pub enum PumpCommand {
    Water { duration_secs: u64 },
}

pub static PUMP_CHANNEL: Channel<CriticalSectionRawMutex, PumpCommand, 4> = Channel::new();

pub async fn water_for(secs: u64) {
    PUMP_CHANNEL
        .send(PumpCommand::Water {
            duration_secs: secs,
        })
        .await;
}

pub async fn pump_task(mut pin: Output<'static>) {
    info!("Pump task started");
    loop {
        match PUMP_CHANNEL.receive().await {
            PumpCommand::Water { duration_secs } => {
                info!("Pump ON for {} seconds", duration_secs);
                pin.set_high();
                Timer::after(Duration::from_secs(duration_secs)).await;
                pin.set_low();
                info!("Pump OFF");
            }
        }
    }
}

pub async fn watering_loop() {
    info!("Watering task started");
    loop {
        let (interval, duration) = critical_section::with(|cs| {
            let cfg = CONFIG.borrow(cs).borrow();
            (cfg.watering_interval_secs, cfg.pump_duration_secs)
        });

        Timer::after(Duration::from_secs(interval)).await;

        let float_high = critical_section::with(|cs| {
            FLOAT_PIN
                .borrow(cs)
                .borrow()
                .as_ref()
                .map(|pin| pin.is_high())
                .unwrap_or(false)
        });

        if float_high {
            info!(
                "Scheduled watering: float is high, running pump for {} seconds",
                duration
            );
            water_for(duration).await;
        } else {
            info!("Scheduled watering: float is low, skipping");
        }
    }
}
