use core::cell::RefCell;

use defmt::info;

use crate::config::CONFIG;
use crate::pins::FLOAT_PIN;
use crate::pump;
use crate::server::{Handler, Response};

pub struct App;

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
                let high = critical_section::with(|cs| {
                    FLOAT_PIN
                        .borrow(cs)
                        .borrow()
                        .as_ref()
                        .map(|pin| pin.is_high())
                        .unwrap_or(false)
                });
                if high {
                    Response::ok("text/plain", b"high")
                } else {
                    Response::ok("text/plain", b"low")
                }
            }
            "/config" => self.handle_config(query),
            "/water" => {
                let duration =
                    critical_section::with(|cs| CONFIG.borrow(cs).borrow().pump_duration_secs);
                info!("Manual watering for {} seconds", duration);
                pump::water_for(duration).await;
                Response::ok("text/plain", b"Watered")
            }
            "/ota" => Response::ok(
                "text/plain",
                b"POST firmware binary to /ota with Authorization: Bearer header",
            ),
            _ => Response::not_found(),
        }
    }
}

static CONFIG_BUF: critical_section::Mutex<RefCell<[u8; 80]>> =
    critical_section::Mutex::new(RefCell::new([0u8; 80]));

impl App {
    fn handle_config(&self, query: Option<&str>) -> Response<'static> {
        if let Some(q) = query {
            if let Some(val) = parse_param(q, "interval")
                && let Ok(secs) = parse_u64(val)
            {
                critical_section::with(|cs| {
                    CONFIG.borrow(cs).borrow_mut().watering_interval_secs = secs;
                });
            }
            if let Some(val) = parse_param(q, "duration")
                && let Ok(secs) = parse_u64(val)
            {
                critical_section::with(|cs| {
                    CONFIG.borrow(cs).borrow_mut().pump_duration_secs = secs;
                });
            }
        }

        let cfg = critical_section::with(|cs| {
            let c = CONFIG.borrow(cs).borrow();
            (c.watering_interval_secs, c.pump_duration_secs)
        });

        let mut num_buf = [0u8; 20];
        critical_section::with(|cs| {
            let buf = &mut *CONFIG_BUF.borrow(cs).borrow_mut();
            let mut pos = 0;

            pos = append_bytes(buf, pos, b"interval=");
            let n = format_u64(cfg.0, &mut num_buf);
            pos = append_bytes(buf, pos, &num_buf[..n]);
            buf[pos] = b'&';
            pos += 1;
            pos = append_bytes(buf, pos, b"duration=");
            let n = format_u64(cfg.1, &mut num_buf);
            pos = append_bytes(buf, pos, &num_buf[..n]);
            buf[pos] = 0;
        });

        Response::ok(
            "text/plain",
            critical_section::with(|cs| {
                let buf = &*CONFIG_BUF.borrow(cs).borrow();
                let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                // SAFETY: the static CONFIG_BUF lives for 'static. The handler runs
                // sequentially so no concurrent reads.
                unsafe { core::slice::from_raw_parts(buf.as_ptr(), end) }
            }),
        )
    }
}

fn append_bytes(buf: &mut [u8], pos: usize, data: &[u8]) -> usize {
    buf[pos..pos + data.len()].copy_from_slice(data);
    pos + data.len()
}

fn parse_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query
        .split('&')
        .find(|p| p.starts_with(key) && p.as_bytes().get(key.len()) == Some(&b'='))
        .and_then(|p| p.get(key.len() + 1..))
}

fn parse_u64(s: &str) -> Result<u64, ()> {
    let mut n: u64 = 0;
    for b in s.bytes() {
        if !b.is_ascii_digit() {
            return Err(());
        }
        n = n.checked_mul(10).ok_or(())? + (b - b'0') as u64;
    }
    Ok(n)
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
