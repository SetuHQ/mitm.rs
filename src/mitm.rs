use std::collections::HashMap;
use std::convert::{Infallible, TryInto};
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use colored::*;
use futures::future::{self, Future};
use futures_util::future::try_join;
use futures_util::{FutureExt, TryFutureExt};
use hyper::client::client::ClientError;
use hyper::client::pool::Key as PoolKey;
use hyper::client::HttpConnector;
use hyper::http::uri::{self, Authority, Scheme};
use hyper::http::{StatusCode, Uri};
use hyper::server::conn::{AddrStream, Http};
use hyper::service::{make_service_fn, service_fn, Service};
use hyper::upgrade::Upgraded;
use hyper::{http, upgrade, Body, Client, Method, Request, Response, Server};
use hyper_rustls::{HttpsConnector, MaybeHttpsStream};
use lazy_static::{__Deref, lazy_static};
use lru_cache::LruCache;
// TLS
use openssl::pkey::{PKey, PKeyRef, Private};
use openssl::x509::X509;
use rustls::ClientConfig;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufStream};
use tokio::net::TcpStream;
use tokio_native_tls::{native_tls, TlsConnector};
use tokio_rustls::server::TlsStream;
use tokio_rustls::{Accept, TlsAcceptor};

use crate::util::args::Args;
use crate::util::cert::{mk_ca_cert, mk_ca_signed_cert, read_cert, read_pkey, verify, CertPair, CERTIFICATES};
use crate::util::tls::{client_config, tls_config};
use crate::util::util::print_type_of;


lazy_static! {
  // pub static ref ARGS: Mutex<Args> = Mutex::new(Args::new());
  static ref ARGS: Mutex<LruCache<String, Arc<Args>>> = Mutex::new(LruCache::new(1000));
  static ref CA: Mutex<LruCache<String, Arc<CertPair>>> = Mutex::new(LruCache::new(1000));
}

#[tokio::main]
// Create an mitm listener
pub async fn listen(args: Args, ca: CertPair) {
  let addr = SocketAddr::new(args.host.parse::<IpAddr>().unwrap(), args.port.try_into().unwrap());
  ARGS.lock().unwrap().insert("args".to_string(), Arc::new(args.clone()));
  CA.lock().unwrap().insert("ca".to_string(), Arc::new(ca.clone()));

  let make_svc = make_service_fn(|socket: &AddrStream| {
    let remote_addr = socket.remote_addr();
    let args = args.clone();
    let ca = ca.clone();

    return async move {
      // Callback for handling every incoming request
      return Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
        println!("Handling request {:?}", req);

        let args = args.clone();
        let ca = ca.clone();

        async move {
          // Construct the failure response body
          let mut failure = Response::new(Body::from("Error: Server error occurred."));
          *failure.status_mut() = StatusCode::BAD_REQUEST;
          // MITM the request
          let response = mitm(req, args, ca).await.unwrap_or(failure);
          return Ok::<_, Infallible>(response);
        }
      }));
    };
  });

  // Bind to given port and start server
  let server = Server::bind(&addr).serve(make_svc);
  println!("Listening on http://{}", addr.to_string().red());

  if let Err(e) = server.await {
    eprintln!("server error: {}", e);
  }
}

async fn mitm(req: Request<Body>, args: Args, ca: CertPair) -> Result<Response<Body>, Error> {
  let host: &str = &format!("{}://{}", req.uri().scheme_str().unwrap_or("https"), req.uri().authority().unwrap());

  // Service a TLS CONNECT request
  if Method::CONNECT == req.method() {
    println!("Received a CONNECT request for {}", host.red());
    handle_connect_request(req, args, ca).await
  } else {
    println!("MITM-ing request from {}", host.red());
    let args = Arc::new(args);
    Ok(handle_proxy_request(req).await)
  }
}

async fn handle_connect_request(req: Request<Body>, args: Args, ca: CertPair) -> Result<Response<Body>, Error> {
  let host: &str = &format!("{}://{}", req.uri().scheme_str().unwrap_or("https"), req.uri().authority().unwrap());

  // Prepare: get TLS config with impersonating certs, sort out the target URI
  let authority = req.uri().authority().unwrap();
  let tls_conf = tls_config(authority, |auth| mk_ca_signed_cert(&ca.cert, &ca.key, authority));

  // Build a client with TLS config
  let client_conf = client_config(host);
  let mut http = HttpConnector::new();
  http.enforce_http(false);
  // let mut connector = ;
  let mut client: Client<HttpsConnector<HttpConnector>, Body> =
    Client::builder().build(HttpsConnector::from((http, client_conf)));

  // 1. Perform TLS handshake with the target
  return client.connection_for((
    req.uri().scheme().unwrap_or(&Scheme::from_str("https").unwrap()).clone(),
    req.uri().authority().unwrap().clone()
  ))
  .map_err(|e| {
    Error::new(ErrorKind::Other, format!("Could not obtain TLS connection to {}", host.red()))
  })
  .await
  // 2. Upgrade the HTTP connection from the client
  .map(move |_whatever| {
    let job = hyper::upgrade::on(req)
      .map_err(|e| {
        Error::new(ErrorKind::Other, format!("Could not upgrade connection"))
      })
      // 3. Perform the TLS handshake with the client
      .and_then(|upgraded:Upgraded| {
        println!("Connection upgraded with client");
        TlsAcceptor::from(tls_conf).accept(upgraded)
      })
      // 4. Handle the underlying HTTP request
      .map(|accepted| {
        println!("Established CONNECT TLS connection with client");
        handle_https_proxy_request(accepted)
      });

    tokio::task::spawn(job);
    Response::builder().status(200).body(Body::empty()).unwrap()
  })
  .or_else(|x| {
    println!("Failed to perform the CONNECT protocol for {}", host.red());
    Ok(Response::builder().status(502).body(Body::empty()).unwrap())
  });
}

async fn handle_proxy_request(req: Request<Body>) -> Response<Body> {
  let host: &str = &format!("{}://{}", req.uri().scheme_str().unwrap_or("https"), req.uri().authority().unwrap());

  let args = ARGS.lock().unwrap().get_mut("args").unwrap().clone();
  let ca = CA.lock().unwrap().get_mut("ca").unwrap().clone();

  // Prepare: get TLS config with impersonating certs, sort out the target URI
  let authority = req.uri().authority().unwrap();
  let tls_conf = tls_config(authority, |auth| mk_ca_signed_cert(&ca.cert, &ca.key, authority));

  // Build a client with TLS config
  let client_conf = client_config(host);
  let mut http = HttpConnector::new();
  http.enforce_http(false);
  // let mut connector = ;
  let mut client: Client<HttpsConnector<HttpConnector>, Body> =
    Client::builder().build(HttpsConnector::from((http, client_conf)));

  // 1. Perform TLS handshake with the target
  let mut pool = client
    .connection_for((
      req.uri().scheme().unwrap_or(&Scheme::from_str("https").unwrap()).clone(),
      req.uri().authority().unwrap().clone(),
    ))
    .await
    .ok()
    .unwrap();

  let uri = req.uri();
  let (parts, body) = req.into_parts();
  println!("MITMed request: {:?} {:?}", parts, body);
  let modified_req = Request::from_parts(parts, body);

  let response = pool.send_request_retryable(modified_req).await.ok().unwrap();
  let (resp_parts, resp_body) = response.into_parts();
  println!("MITMed response: {:?} {:?}", resp_parts, resp_body);
  let modified_resp = Response::from_parts(resp_parts, resp_body);

  modified_resp
}

// Option<Body>
fn handle_https_proxy_request(stream: Result<TlsStream<Upgraded>, Error>) -> () {
  let svc = service_fn(move |req: Request<Body>| {
    async move {
      let authority = req.headers().get("host").unwrap().to_str().unwrap();

      let uri = http::uri::Builder::new()
        .scheme("https")
        .authority(authority)
        .path_and_query(&req.uri().to_string() as &str)
        .build()
        .unwrap();

      let (mut parts, body) = req.into_parts();
      parts.uri = uri;
      let req = Request::from_parts(parts, body);

      let response: Response<Body> = handle_proxy_request(req).await;
      Ok::<_, hyper::Error>(response)
    }
  });

  Http::new().serve_connection(stream.unwrap(), svc).map_err(|e: hyper::Error| {
    println!("Error in serving http conection inside TLS tunnel");
  });
}
