use std::convert::{Infallible, TryInto};
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use colored::*;
use hyper::client::HttpConnector;
use hyper::http::uri::Scheme;
// use futures_util::future::try_join;
use hyper::http::{StatusCode, Uri};
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper::{Body, Client, Method, Request, Response, Server};
use hyper_tls::HttpsConnector;
use openssl::x509::X509;
use tokio::net::TcpStream;
use tokio_native_tls::TlsConnector;


#[tokio::main]
pub async fn listen(host: String, port: u32, client_cert: &'static str) {
  let address = format!("{}:{}", host, port);
  let addr = SocketAddr::new(address.parse::<IpAddr>().unwrap(), port.try_into().unwrap());

  let make_service =
    make_service_fn(move |_| async move { Ok::<_, Infallible>(service_fn(move |req| mitm(req, client_cert))) });

  let server = Server::bind(&addr).serve(make_service);
  println!("Listening on http://{}", addr.to_string().red());
}

async fn mitm(req: Request<Body>, cert: &str) -> Result<Response<Body>, hyper::Error> {
  let host: Option<String> = req.uri().authority().and_then(|auth| auth.as_str().parse().ok());

  if Method::CONNECT == req.method() {
    if let Some(addr) = host {
      let connect_response =
        handle_connect_request(req).unwrap_or("Error: Could not perform the CONNECT protocol handshake.".to_string());

      Ok(Response::new(Body::from(connect_response)))
    } else {
      eprintln!("CONNECT host is not socket addr: {:?}", req.uri());
      let mut resp = Response::new(Body::from("CONNECT must be to a socket address"));
      *resp.status_mut() = StatusCode::BAD_REQUEST;

      Ok(resp)
    }
  } else {
    // match req.uri().scheme() {
    //     Some(&Scheme::HTTPS) => {
    //         let mut tls_conn_builder = TlsConnector::builder().identity();
    //         tls_conn_builder.identity(cert);
    //         // let tls_conn = tls_conn_builder.build().unwrap();
    //     }
    //     Some(&Scheme::HTTP) => {
    //         let client = Client::new();
    //         client.request(req).await
    //     }
    // }
    Ok(Response::new(Body::from("Not implemented yet")))
  }
}


fn handle_connect_request(req: Request<Body>) -> Option<String> { Some("lol".to_string()) }
