use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server, Uri, Error, Client};
use hyper::body::Buf;
use hyper::service::{make_service_fn, service_fn};
use http::uri::{Scheme};
use html5ever::{parse_document, serialize, ParseOpts};
use html5ever::serialize::SerializeOpts;
use markup5ever_rcdom::{RcDom, SerializableHandle};
use html5ever::tendril::TendrilSink;
use bytes::{BytesMut, BufMut};
use std::borrow::BorrowMut;

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
    let (mut parts, resp_body_stream) = client.request(req).await?.into_parts();
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
    let body_orig = hyper::body::aggregate(resp_body_stream).await.unwrap();
    let html = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut body_orig.reader())
        .unwrap();
    let mut w = BytesMut::new().writer();
    serialize(w.borrow_mut(), &SerializableHandle::from(html.document), SerializeOpts::default());
    let body_transformed = w.into_inner().freeze();
    parts.headers.remove(http::header::CONTENT_LENGTH);
    return Ok(Response::from_parts(parts, Body::from(body_transformed)));
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
