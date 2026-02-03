use bytes::Bytes;
use derive_more::From;
use http::{Request, Response};
use wstd::http::{Body, Client};
use wstd::io::AsyncRead;

#[derive(Debug, From)]
pub enum Error {
    #[from]
    Http(http::Error),
    #[from]
    WstdHttp(wstd::http::Error),
    Generic(String),
}

pub struct WstdHttpClient {
    client: Client,
}

impl WstdHttpClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn execute(&self, request: Request<Bytes>) -> Result<Response<Vec<u8>>, Error> {
        let (parts, body) = request.into_parts();
        
        let mut wasi_req = wstd::http::Request::builder()
            .uri(parts.uri)
            .method(parts.method)
            .version(parts.version)
            .body(BytesCursor::new(body))
            .map_err(Error::Http)?;
            
        *wasi_req.headers_mut() = parts.headers;

        let mut wasi_resp = self.client.send(wasi_req).await.map_err(Error::WstdHttp)?;
        
        let status = wasi_resp.status();
        let headers = wasi_resp.headers().clone();
        let body = wasi_resp.body_mut().bytes().await.map_err(Error::WstdHttp)?;

        let mut response = Response::builder()
            .status(status)
            .body(body)
            .map_err(Error::Http)?;
            
        *response.headers_mut() = headers;
        
        Ok(response)
    }
}

pub struct BytesCursor {
    cursor: wstd::io::Cursor<Bytes>,
}

impl BytesCursor {
    fn new(bytes: Bytes) -> Self {
        Self {
            cursor: wstd::io::Cursor::new(bytes),
        }
    }
}

impl AsyncRead for BytesCursor {
    async fn read(&mut self, buf: &mut [u8]) -> wstd::io::Result<usize> {
        self.cursor.read(buf).await
    }
}

impl Body for BytesCursor {
    fn len(&self) -> Option<usize> {
        Some(self.cursor.get_ref().len())
    }
}
