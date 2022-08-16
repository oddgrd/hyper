#![feature(test)]
#![deny(warnings)]
extern crate test;

use test::Bencher;

use futures_util::future::join_all;
use hyper::body::HttpBody;
use hyper::client::conn::SendRequest;
use hyper::{server::conn::Http, service::service_fn};
use hyper::{Body, Method, Request, Response};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tower::ServiceExt;

// // HTTP1
#[bench]
fn http1_consecutive_x1_empty(b: &mut Bencher) {
    opts().bench(b)
}

#[bench]
fn http1_consecutive_x1_req_10b(b: &mut test::Bencher) {
    opts()
        .method(Method::POST)
        .request_body(&[b's'; 10])
        .bench(b)
}

#[bench]
fn http1_consecutive_x1_both_100kb(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 100];
    opts()
        .method(Method::POST)
        .request_body(body)
        .response_body(body)
        .bench(b)
}

#[bench]
fn http1_consecutive_x1_both_10mb(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 1024 * 10];
    opts()
        .method(Method::POST)
        .request_body(body)
        .response_body(body)
        .bench(b)
}

#[bench]
fn http1_parallel_x10_empty(b: &mut test::Bencher) {
    opts().parallel(10).bench(b)
}

#[bench]
fn http1_parallel_x10_req_10mb(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 1024 * 10];
    opts()
        .parallel(10)
        .method(Method::POST)
        .request_body(body)
        .bench(b)
}

#[bench]
fn http1_parallel_x10_req_10kb_100_chunks(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 10];
    opts()
        .parallel(10)
        .method(Method::POST)
        .request_chunks(body, 100)
        .bench(b)
}

#[bench]
fn http1_parallel_x10_res_1mb(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 1024 * 1];
    opts().parallel(10).response_body(body).bench(b)
}

#[bench]
fn http1_parallel_x10_res_10mb(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 1024 * 10];
    opts().parallel(10).response_body(body).bench(b)
}

// // HTTP2

const HTTP2_MAX_WINDOW: u32 = std::u32::MAX >> 1;

#[bench]
fn http2_consecutive_x1_empty(b: &mut test::Bencher) {
    opts().http2().bench(b)
}

#[bench]
fn http2_consecutive_x1_req_10b(b: &mut test::Bencher) {
    opts()
        .http2()
        .method(Method::POST)
        .request_body(&[b's'; 10])
        .bench(b)
}

#[bench]
fn http2_consecutive_x1_req_100kb(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 100];
    opts()
        .http2()
        .method(Method::POST)
        .request_body(body)
        .bench(b)
}

#[bench]
fn http2_parallel_x10_empty(b: &mut test::Bencher) {
    opts().http2().parallel(10).bench(b)
}

#[bench]
fn http2_parallel_x10_req_10mb(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 1024 * 10];
    opts()
        .http2()
        .parallel(10)
        .method(Method::POST)
        .request_body(body)
        .http2_stream_window(HTTP2_MAX_WINDOW)
        .http2_conn_window(HTTP2_MAX_WINDOW)
        .bench(b)
}

#[bench]
fn http2_parallel_x10_req_10kb_100_chunks(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 10];
    opts()
        .http2()
        .parallel(10)
        .method(Method::POST)
        .request_chunks(body, 100)
        .bench(b)
}

#[bench]
fn http2_parallel_x10_req_10kb_100_chunks_adaptive_window(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 10];
    opts()
        .http2()
        .parallel(10)
        .method(Method::POST)
        .request_chunks(body, 100)
        .http2_adaptive_window()
        .bench(b)
}

#[bench]
fn http2_parallel_x10_req_10kb_100_chunks_max_window(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 10];
    opts()
        .http2()
        .parallel(10)
        .method(Method::POST)
        .request_chunks(body, 100)
        .http2_stream_window(HTTP2_MAX_WINDOW)
        .http2_conn_window(HTTP2_MAX_WINDOW)
        .bench(b)
}

#[bench]
fn http2_parallel_x10_res_1mb(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 1024 * 1];
    opts()
        .http2()
        .parallel(10)
        .response_body(body)
        .http2_stream_window(HTTP2_MAX_WINDOW)
        .http2_conn_window(HTTP2_MAX_WINDOW)
        .bench(b)
}

#[bench]
fn http2_parallel_x10_res_10mb(b: &mut test::Bencher) {
    let body = &[b'x'; 1024 * 1024 * 10];
    opts()
        .http2()
        .parallel(10)
        .response_body(body)
        .http2_stream_window(HTTP2_MAX_WINDOW)
        .http2_conn_window(HTTP2_MAX_WINDOW)
        .bench(b)
}

// // ==== Benchmark Options =====

#[derive(Clone)]
struct Opts {
    http2: bool,
    http2_stream_window: Option<u32>,
    http2_conn_window: Option<u32>,
    http2_adaptive_window: bool,
    parallel_cnt: u32,
    request_method: Method,
    request_body: Option<&'static [u8]>,
    request_chunks: usize,
    response_body: &'static [u8],
}

fn opts() -> Opts {
    Opts {
        http2: false,
        http2_stream_window: None,
        http2_conn_window: None,
        http2_adaptive_window: false,
        parallel_cnt: 1,
        request_method: Method::GET,
        request_body: None,
        request_chunks: 0,
        response_body: b"",
    }
}

impl Opts {
    fn http2(mut self) -> Self {
        self.http2 = true;
        self
    }

    fn http2_stream_window(mut self, sz: impl Into<Option<u32>>) -> Self {
        assert!(!self.http2_adaptive_window);
        self.http2_stream_window = sz.into();
        self
    }

    fn http2_conn_window(mut self, sz: impl Into<Option<u32>>) -> Self {
        assert!(!self.http2_adaptive_window);
        self.http2_conn_window = sz.into();
        self
    }

    fn http2_adaptive_window(mut self) -> Self {
        assert!(self.http2_stream_window.is_none());
        assert!(self.http2_conn_window.is_none());
        self.http2_adaptive_window = true;
        self
    }

    fn method(mut self, m: Method) -> Self {
        self.request_method = m;
        self
    }

    fn request_body(mut self, body: &'static [u8]) -> Self {
        self.request_body = Some(body);
        self
    }

    fn request_chunks(mut self, chunk: &'static [u8], cnt: usize) -> Self {
        assert!(cnt > 0);
        self.request_body = Some(chunk);
        self.request_chunks = cnt;
        self
    }

    fn response_body(mut self, body: &'static [u8]) -> Self {
        self.response_body = body;
        self
    }

    fn parallel(mut self, cnt: u32) -> Self {
        assert!(cnt > 0, "parallel count must be larger than 0");
        self.parallel_cnt = cnt;
        self
    }

    fn bench(self, b: &mut test::Bencher) {
        let _ = pretty_env_logger::try_init();
        // Create a runtime of current thread.
        let rt = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("rt build"),
        );

        let req_len = self.request_body.map(|b| b.len()).unwrap_or(0) as u64
            * u64::max(1, self.request_chunks as u64);
        let bytes_per_iter = (req_len + self.response_body.len() as u64) * self.parallel_cnt as u64;
        b.bytes = bytes_per_iter;

        let addr = spawn_server(&rt, &self);

        let client_count = match self.http2 {
            true => 1,
            false => self.parallel_cnt,
        };

        let mut request_senders: Vec<SendRequest<crate::Body>> = (0..client_count)
            .map(|_| prepare_client(&rt, &self, &addr))
            .collect();

        let mut send_request = |req: Request<Body>, idx: usize| {
            if self.http2 {
                rt.block_on(request_senders[idx].ready()).unwrap();
            }
            let fut = request_senders[idx].send_request(req);
            async {
                let res = fut.await.expect("Client wait");
                let mut body = res.into_body();
                while let Some(_chunk) = body.data().await {}
            }
        };

        if self.parallel_cnt == 1 {
            b.iter(|| {
                let req = make_request(&self, &rt, &addr);
                rt.block_on(send_request(req, 0));
            });
        } else {
            b.iter(|| {
                let futs = (0..self.parallel_cnt as usize).map(|idx| {
                    let i = if self.http2 { 0 } else { idx };
                    let req = make_request(&self, &rt, &addr);
                    send_request(req, i)
                });
                rt.spawn(join_all(futs));
            });
        }
    }
}

/// Creates a request, for being sent via the request_sender
fn make_request(opts: &Opts, rt: &tokio::runtime::Runtime, addr: &SocketAddr) -> Request<Body> {
    let url: hyper::Uri = format!("http://{}/hello", addr).parse().unwrap();
    let chunk_cnt = opts.request_chunks;
    let body = if chunk_cnt > 0 {
        let (mut tx, body) = Body::channel();
        let chunk = opts
            .request_body
            .expect("request_chunks means request_body");
        rt.spawn(async move {
            for _ in 0..chunk_cnt {
                tx.send_data(chunk.into()).await.expect("send_data");
            }
        });
        body
    } else {
        opts.request_body
            .map(Body::from)
            .unwrap_or_else(Body::empty)
    };
    let mut req = Request::new(body);
    *req.method_mut() = opts.request_method.clone();
    *req.uri_mut() = url.clone();
    req
}

/// Prepares a client (request_sender) for sending requests to the given address
fn prepare_client(
    rt: &tokio::runtime::Runtime,
    opts: &Opts,
    addr: &SocketAddr,
) -> SendRequest<crate::Body> {
    let target_stream = rt.block_on(TcpStream::connect(addr)).unwrap();
    let (client, conn) = rt
        .block_on(
            hyper::client::conn::Builder::new()
                .http2_only(opts.http2)
                .http2_initial_stream_window_size(opts.http2_stream_window)
                .http2_initial_connection_window_size(opts.http2_conn_window)
                .http2_adaptive_window(opts.http2_adaptive_window)
                .handshake::<_, Body>(target_stream),
        )
        .unwrap();

    rt.spawn(async move {
        if let Err(e) = conn.await {
            eprintln!("Error in connection: {}", e);
        }
    });
    client
}

/// Spawns a server in the background
fn spawn_server(rt: &tokio::runtime::Runtime, opts: &Opts) -> SocketAddr {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let tcp_listener = rt.block_on(async move { TcpListener::bind(addr).await.unwrap() });
    let local_addr = tcp_listener.local_addr().unwrap();

    let opts = opts.clone();
    let body = opts.response_body;
    rt.spawn(async move {
        loop {
            let (stream, _addr) = tcp_listener.accept().await.unwrap();
            eprintln!("New incoming connection: {:?}", addr);
            tokio::task::spawn(
                Http::new()
                    .http2_only(opts.http2)
                    .http2_initial_stream_window_size(opts.http2_stream_window)
                    .http2_initial_connection_window_size(opts.http2_conn_window)
                    .http2_adaptive_window(opts.http2_adaptive_window)
                    .serve_connection(
                        stream,
                        service_fn(move |req: Request<Body>| async move {
                            let mut req_body = req.into_body();
                            while let Some(_chunk) = req_body.data().await {}
                            Ok::<_, hyper::Error>(Response::new(Body::from(body)))
                        }),
                    ),
            );
        }
    });
    local_addr
}
