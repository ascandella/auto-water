use defmt::info;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use esp_hal::system::software_reset;
use httparse::{EMPTY_HEADER, Request, Status};

use crate::config;
use crate::ota;

const MAX_HEADERS: usize = 16;
const RX_BUF_SIZE: usize = 2048;
const TX_BUF_SIZE: usize = 2048;

pub struct Response<'a> {
    pub status: &'a str,
    pub content_type: &'a str,
    pub body: &'a [u8],
}

impl<'a> Response<'a> {
    pub const fn ok(content_type: &'a str, body: &'a [u8]) -> Self {
        Self {
            status: "200 OK",
            content_type,
            body,
        }
    }

    pub const fn not_found() -> Self {
        Self {
            status: "404 Not Found",
            content_type: "text/plain",
            body: b"Not Found",
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait Handler {
    async fn handle(&self, method: &str, path: &str) -> Response<'static>;
}

pub struct Server<H: Handler> {
    handler: H,
}

impl<H: Handler> Server<H> {
    pub fn new(handler: H) -> Self {
        Self { handler }
    }

    pub async fn run(&self, stack: embassy_net::Stack<'static>) {
        let mut rx_buf = [0u8; RX_BUF_SIZE];
        let mut tx_buf = [0u8; TX_BUF_SIZE];

        loop {
            let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);

            if socket.accept(80).await.is_err() {
                continue;
            }

            self.handle_connection(&mut socket).await;
            socket.close();
            Timer::after(Duration::from_millis(100)).await;
        }
    }

    async fn handle_connection(&self, socket: &mut TcpSocket<'_>) {
        let mut req_buf = [0u8; 512];
        let n = match socket.read(&mut req_buf).await {
            Ok(n) => n,
            Err(_) => return,
        };

        let mut headers = [EMPTY_HEADER; MAX_HEADERS];
        let mut req = Request::new(&mut headers);

        match req.parse(&req_buf[..n as usize]) {
            Ok(Status::Complete(header_len)) => {
                let method = req.method.unwrap_or("GET");
                let path = req.path.unwrap_or("/");
                info!("HTTP {} {}", method, path);

                if method == "POST" && path == "/ota" {
                    let auth_header = req
                        .headers
                        .iter()
                        .find(|h| h.name.eq_ignore_ascii_case("Authorization"))
                        .map(|h| h.value);

                    let content_length = req
                        .headers
                        .iter()
                        .find(|h| h.name.eq_ignore_ascii_case("Content-Length"))
                        .and_then(|h| core::str::from_utf8(h.value).ok())
                        .and_then(|s| s.parse::<usize>().ok());

                    let initial_body = &req_buf[header_len..n as usize];
                    self.handle_ota(socket, auth_header, content_length, initial_body)
                        .await;
                } else {
                    let resp = self.handler.handle(method, path).await;
                    send_response(socket, &resp).await;
                }
            }
            _ => {
                info!("Failed to parse HTTP request");
                let _ = socket.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
            }
        }
    }

    async fn handle_ota(
        &self,
        socket: &mut TcpSocket<'_>,
        auth_header: Option<&[u8]>,
        content_length: Option<usize>,
        initial_body: &[u8],
    ) {
        if !ota::check_auth(auth_header) {
            info!("OTA: auth failed");
            let _ = socket
                .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Type: text/plain\r\nContent-Length: 16\r\n\r\nInvalid password")
                .await;
            return;
        }

        let content_length = match content_length {
            Some(len) if len > 0 => len,
            _ => {
                info!("OTA: missing or invalid Content-Length");
                let _ = socket
                    .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nContent-Length: 22\r\n\r\nMissing Content-Length")
                    .await;
                return;
            }
        };

        info!("OTA: starting, {} bytes", content_length);

        let mut flash = config::take_flash();
        match ota::perform_ota(&mut flash, socket, content_length, initial_body).await {
            Ok(()) => {
                info!("OTA: write complete, sending response");
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 35\r\n\r\nOTA update successful, rebooting...",
                    )
                    .await;
                software_reset();
            }
            Err(e) => {
                defmt::error!("OTA failed: {:?}", e);
                config::store_flash(flash);
                let _ = socket
                    .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: 9\r\n\r\nOTA failed")
                    .await;
            }
        }
    }
}

async fn send_response(socket: &mut TcpSocket<'_>, resp: &Response<'_>) {
    let _ = socket.write_all(b"HTTP/1.1 ").await;
    let _ = socket.write_all(resp.status.as_bytes()).await;
    let _ = socket.write_all(b"\r\nContent-Type: ").await;
    let _ = socket.write_all(resp.content_type.as_bytes()).await;
    let _ = socket.write_all(b"\r\nContent-Length: ").await;

    let mut len_buf = [0u8; 10];
    let len_str = format_u32(resp.body.len() as u32, &mut len_buf);
    let _ = socket.write_all(len_str).await;

    let _ = socket.write_all(b"\r\n\r\n").await;
    let _ = socket.write_all(resp.body).await;
}

fn format_u32(mut n: u32, buf: &mut [u8; 10]) -> &[u8] {
    if n == 0 {
        return b"0";
    }
    let mut i = buf.len();
    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    &buf[i..]
}
