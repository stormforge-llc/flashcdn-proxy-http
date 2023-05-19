use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server, Uri, Error};
use http::uri::{Scheme};
use hyper::service::{make_service_fn, service_fn};
use hyper::Client;
use html5ever::{parse_document, serialize, Parser, ParseOpts};
use markup5ever_rcdom::RcDom;
use html5ever::tendril::TendrilSink;

async fn hello_world(req_incoming: Request<Body>) -> Result<Response<Body>, Error> {
    let client = Client::new();
    let method = req_incoming.method().clone();
    let http1_scheme = Scheme::try_from("http").unwrap();
    let uri = Uri::builder()
        .scheme(req_incoming.uri().scheme().unwrap_or(&http1_scheme).clone())
        .authority("localhost:8000")
        .path_and_query(req_incoming.uri().path_and_query().unwrap().clone())
        .build()
        .unwrap();
    // TODO: stream instead of await
    let req = Request::builder()
        .uri(uri.clone())
        .method(method)
        .body(Body::from(hyper::body::to_bytes(req_incoming.into_body()).await.unwrap()))
        .unwrap();
    let (parts, resp_body_stream) = client.request(req).await?.into_parts();
    if !parts.status.is_success() {
        println!("Skipping transform of non-successful status {:?}", parts.status);
        return Ok(Response::from_parts(parts, resp_body_stream));
    }
    let content_type = parts.headers.get(http::header::CONTENT_TYPE);
    if content_type.is_none() {
        println!("Skipping transform of non-HTML content type {:?}", content_type);
        return Ok(Response::from_parts(parts, resp_body_stream));
    }

    println!("Scanning {:?} for transformable HTML", uri);
    let body_orig = hyper::body::to_bytes(resp_body_stream).await.unwrap();
    let body_bytes: Vec<u8> = body_orig.to_vec();
    let html = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut body_bytes)
        .unwrap();
    return Ok(Response::from_parts(parts, Body::from(body_orig)));
}

#[tokio::main]
async fn main() {
    // We'll bind to 127.0.0.1:3000
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // A `Service` is needed for every connection, so this
    // creates one from our `hello_world` function.
    let make_svc = make_service_fn(|_conn| async {
        // service_fn converts our function into a `Service`
        Ok::<_, Error>(service_fn(hello_world))
    });

    let server = Server::bind(&addr).serve(make_svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
