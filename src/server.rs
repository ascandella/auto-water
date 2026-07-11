use defmt::info;
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use httparse::{EMPTY_HEADER, Request, Status};

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

pub trait Handler {
    fn handle(&self, method: &str, path: &str) -> Response<'static>;
}

pub struct Server<H: Handler> {
    handler: H,
}

impl<H: Handler> Server<H> {
    pub const fn new(handler: H) -> Self {
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

            handle_connection(&self.handler, &mut socket).await;
            socket.close();
            Timer::after(Duration::from_millis(100)).await;
        }
    }
}

async fn handle_connection<H: Handler>(handler: &H, socket: &mut TcpSocket<'_>) {
    let mut req_buf = [0u8; 512];
    let n = match socket.read(&mut req_buf).await {
        Ok(n) => n,
        Err(_) => return,
    };

    let mut headers = [EMPTY_HEADER; MAX_HEADERS];
    let mut req = Request::new(&mut headers);

    match req.parse(&req_buf[..n as usize]) {
        Ok(Status::Complete(_)) => {
            let method = req.method.unwrap_or("GET");
            let path = req.path.unwrap_or("/");
            info!("HTTP {} {}", method, path);

            let resp = handler.handle(method, path);
            send_response(socket, &resp).await;
        }
        _ => {
            info!("Failed to parse HTTP request");
            let _ = socket.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
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
