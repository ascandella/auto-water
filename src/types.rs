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
