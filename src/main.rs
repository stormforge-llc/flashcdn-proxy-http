use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server, Uri, Error};
use http::uri::{Scheme};
use hyper::service::{make_service_fn, service_fn};
use hyper::Client;


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
    let body = hyper::body::to_bytes(req_incoming.into_body()).await.unwrap();
    let req = Request::builder()
        .uri(uri)
        .method(method)
        .body(Body::from(body))
        .unwrap();
    return client.request(req).await;
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
