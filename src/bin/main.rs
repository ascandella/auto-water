#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use core::cell::RefCell;

use defmt::info;
use embassy_executor::Spawner;
use embassy_net::{DhcpConfig, Runner, StackResources};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_radio::wifi::{Config, WifiController, sta::StationConfig};

use auto_water::config::CONFIG;
use auto_water::server::{Handler, Response, Server};

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");

static RELAY_PIN: critical_section::Mutex<RefCell<Option<Output<'static>>>> =
    critical_section::Mutex::new(RefCell::new(None));

static FLOAT_PIN: critical_section::Mutex<RefCell<Option<Input<'static>>>> =
    critical_section::Mutex::new(RefCell::new(None));

macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write($val);
        x
    }};
}

fn is_float_high() -> bool {
    critical_section::with(|cs| {
        FLOAT_PIN
            .borrow(cs)
            .borrow()
            .as_ref()
            .map(|pin| pin.is_high())
            .unwrap_or(false)
    })
}

async fn run_pump(secs: u64) {
    critical_section::with(|cs| {
        if let Some(ref mut pin) = RELAY_PIN.borrow(cs).borrow_mut().as_mut() {
            pin.set_high();
        }
    });
    Timer::after(Duration::from_secs(secs)).await;
    critical_section::with(|cs| {
        if let Some(ref mut pin) = RELAY_PIN.borrow(cs).borrow_mut().as_mut() {
            pin.set_low();
        }
    });
}

fn parse_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    let mut params = query.split('&');
    params
        .find(|p| p.starts_with(key) && p.as_bytes().get(key.len()) == Some(&b'='))
        .and_then(|p| p.get(key.len() + 1..))
}

fn format_u64(mut n: u64, buf: &mut [u8; 20]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut i = buf.len();
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    let len = buf.len() - i;
    buf.copy_within(i.., 0);
    len
}

struct App;

impl Handler for App {
    async fn handle(&self, _method: &str, path: &str) -> Response<'static> {
        let (path_base, query) = match path.split_once('?') {
            Some((p, q)) => (p, Some(q)),
            None => (path, None),
        };

        match path_base {
            "/" => Response::ok("text/plain", b"Hello from auto-water!"),
            "/status" => Response::ok("text/plain", b"OK"),
            "/float" => {
                let high = is_float_high();
                if high {
                    Response::ok("text/plain", b"high")
                } else {
                    Response::ok("text/plain", b"low")
                }
            }
            "/config" => {
                let q = query.unwrap_or("");
                let mut interval_updated = false;
                let mut duration_updated = false;

                if let Some(val) = parse_param(q, "interval") {
                    if let Ok(secs) = parse_u64(val) {
                        critical_section::with(|cs| {
                            CONFIG.borrow(cs).borrow_mut().watering_interval_secs = secs;
                        });
                        interval_updated = true;
                    }
                }
                if let Some(val) = parse_param(q, "duration") {
                    if let Ok(secs) = parse_u64(val) {
                        critical_section::with(|cs| {
                            CONFIG.borrow(cs).borrow_mut().pump_duration_secs = secs;
                        });
                        duration_updated = true;
                    }
                }

                if interval_updated || duration_updated {
                    let cfg = critical_section::with(|cs| {
                        let c = CONFIG.borrow(cs).borrow();
                        (c.watering_interval_secs, c.pump_duration_secs)
                    });

                    let mut buf = [0u8; 80];
                    let mut pos = 0;
                    let s = b"interval=";
                    buf[pos..pos + s.len()].copy_from_slice(s);
                    pos += s.len();
                    let mut num_buf = [0u8; 20];
                    let n = format_u64(cfg.0, &mut num_buf);
                    buf[pos..pos + n].copy_from_slice(&num_buf[..n]);
                    pos += n;
                    buf[pos] = b'&';
                    pos += 1;
                    let s = b"duration=";
                    buf[pos..pos + s.len()].copy_from_slice(s);
                    pos += s.len();
                    let n = format_u64(cfg.1, &mut num_buf);
                    buf[pos..pos + n].copy_from_slice(&num_buf[..n]);
                    pos += n;

                    Response::ok("text/plain", &buf[..pos])
                } else {
                    let cfg = critical_section::with(|cs| {
                        let c = CONFIG.borrow(cs).borrow();
                        (c.watering_interval_secs, c.pump_duration_secs)
                    });

                    let mut buf = [0u8; 80];
                    let mut pos = 0;
                    let s = b"interval=";
                    buf[pos..pos + s.len()].copy_from_slice(s);
                    pos += s.len();
                    let mut num_buf = [0u8; 20];
                    let n = format_u64(cfg.0, &mut num_buf);
                    buf[pos..pos + n].copy_from_slice(&num_buf[..n]);
                    pos += n;
                    buf[pos] = b'&';
                    pos += 1;
                    let s = b"duration=";
                    buf[pos..pos + s.len()].copy_from_slice(s);
                    pos += s.len();
                    let n = format_u64(cfg.1, &mut num_buf);
                    buf[pos..pos + n].copy_from_slice(&num_buf[..n]);
                    pos += n;

                    Response::ok("text/plain", &buf[..pos])
                }
            }
            "/water" => {
                let duration =
                    critical_section::with(|cs| CONFIG.borrow(cs).borrow().pump_duration_secs);
                info!("Manual watering for {} seconds", duration);
                run_pump(duration).await;
                Response::ok("text/plain", b"Watered")
            }
            _ => Response::not_found(),
        }
    }
}

fn parse_u64(s: &str) -> Result<u64, ()> {
    let mut n: u64 = 0;
    for b in s.bytes() {
        if b < b'0' || b > b'9' {
            return Err(());
        }
        n = n.checked_mul(10).ok_or(())? + (b - b'0') as u64;
    }
    Ok(n)
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let _ = peripherals.GPIO6;
    let _ = peripherals.GPIO7;
    let _ = peripherals.GPIO8;
    let _ = peripherals.GPIO9;
    let _ = peripherals.GPIO10;
    let _ = peripherals.GPIO11;
    let _ = peripherals.GPIO16;
    let _ = peripherals.GPIO20;

    let relay_pin = Output::new(peripherals.GPIO26, Level::Low, OutputConfig::default());
    critical_section::with(|cs| {
        RELAY_PIN.borrow(cs).replace(Some(relay_pin));
    });

    let float_pin = Input::new(
        peripherals.GPIO13,
        InputConfig::default().with_pull(Pull::Up),
    );
    critical_section::with(|cs| {
        FLOAT_PIN.borrow(cs).replace(Some(float_pin));
    });

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    let rng = Rng::new();

    let (wifi_controller, interfaces) = esp_radio::wifi::new(peripherals.WIFI, Default::default())
        .expect("Failed to initialize Wi-Fi controller");

    let net_seed = rng.random() as u64 | ((rng.random() as u64) << 32);
    let net_config = embassy_net::Config::dhcpv4(DhcpConfig::default());

    let (stack, runner) = embassy_net::new(
        interfaces.station,
        net_config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        net_seed,
    );

    spawner.spawn(connection(wifi_controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());
    spawner.spawn(watering_task().unwrap());

    println!("Waiting for link up...");
    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        println!("IP: {}", config.address);
    }

    let server = Server::new(App);
    server.run(stack).await;
}

#[embassy_executor::task]
async fn watering_task() {
    info!("Watering task started");
    loop {
        let interval =
            critical_section::with(|cs| CONFIG.borrow(cs).borrow().watering_interval_secs);
        let duration = critical_section::with(|cs| CONFIG.borrow(cs).borrow().pump_duration_secs);

        Timer::after(Duration::from_secs(interval)).await;

        if is_float_high() {
            info!(
                "Scheduled watering: float is high, running pump for {} seconds",
                duration
            );
            run_pump(duration).await;
        } else {
            info!("Scheduled watering: float is low, skipping");
        }
    }
}

#[embassy_executor::task]
#[allow(clippy::large_stack_frames)]
async fn connection(mut controller: WifiController<'static>) {
    info!("Starting WiFi connection task");
    loop {
        if !controller.is_connected() {
            let client_config = Config::Station(
                StationConfig::default()
                    .with_ssid(SSID)
                    .with_password(PASSWORD.into()),
            );
            controller.set_config(&client_config).unwrap();
            info!("Connecting to WiFi...");
            match controller.connect_async().await {
                Ok(_) => info!("WiFi connected!"),
                Err(e) => {
                    info!("Failed to connect: {:?}", e);
                    Timer::after(Duration::from_secs(5)).await;
                }
            }
        } else {
            Timer::after(Duration::from_secs(10)).await;
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, esp_radio::wifi::Interface<'static>>) {
    runner.run().await;
}
