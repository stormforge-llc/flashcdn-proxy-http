use std::{
    rc::Rc,
    net::SocketAddr,
    vec::Vec,
};
use hyper::{Body, Request, Response, Server, Uri, Error, Client};
use hyper::body::Buf;
use hyper::service::{make_service_fn, service_fn};
use http::uri::{Scheme};
use html5ever::{parse_document, serialize, ParseOpts};
use html5ever::serialize::SerializeOpts;
use markup5ever_rcdom::{RcDom, SerializableHandle, Node, NodeData};
use html5ever::tendril::TendrilSink;
use bytes::{BytesMut, BufMut};
use std::borrow::BorrowMut;

struct NodeIterator {
    root: Rc<Node>,
    idx: usize,
    children: Vec<NodeIterator>,
    me: bool,
}

impl NodeIterator {
    fn new(root: Rc<Node>) -> NodeIterator {
        let r = Rc::clone(&root);
        let ch = r.children.borrow();
        let mut children = Vec::with_capacity(ch.len());
        for n in ch.iter() {
            children.push(NodeIterator::new(n.clone()));
        }
        return NodeIterator {
            root: r.clone(),
            idx: 0,
            children: children,
            me: false,
        }
    }
}

impl Iterator for NodeIterator {
    type Item = Rc<Node>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.me {
            self.me = true;
            return Some(Rc::clone(&self.root));
        }

        // Level-order traversal
        while self.idx < self.children.len() {
            let ch = &mut self.children[self.idx];
            let n = ch.next();
            if let Some(c) = n {
                return Some(c);
            }
            self.idx += 1;
        }

        return None;
    }
}

fn flash_transform(doc: Rc<Node>) -> Rc<Node> {
    let it = NodeIterator::new(Rc::clone(&doc));
    for n in it {
        if let NodeData::Element { name, attrs, template_contents, mathml_annotation_xml_integration_point } = &n.as_ref().data {
            println!("visiting {}", name.local.to_string());
        }
    }
    return doc;
}

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
    let document_transformed = flash_transform(html.document);
    serialize(w.borrow_mut(), &SerializableHandle::from(document_transformed), SerializeOpts::default());
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
