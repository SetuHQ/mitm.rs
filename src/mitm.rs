use std::convert::{Infallible, TryInto};
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use base64;
use colored::*;
use futures_util::{FutureExt, TryFutureExt};
use hyper::client::HttpConnector;
use hyper::http::uri::Scheme;
use hyper::http::StatusCode;
use hyper::server::conn::{AddrStream, Http};
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper::{http, Body, Client, Method, Request, Response, Server};
use hyper_rustls::HttpsConnector;
use lazy_static::lazy_static;
use log::info;
use lru_cache::LruCache;
// TLS
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;

use crate::util::args::Args;
use crate::util::cert::{mk_ca_signed_cert, CertPair};
use crate::util::tls::{client_config, tls_config};


lazy_static! {
  static ref ARGS: Mutex<LruCache<String, Arc<Args>>> = Mutex::new(LruCache::new(1000));
  static ref CA: Mutex<LruCache<String, Arc<CertPair>>> = Mutex::new(LruCache::new(1000));
}

#[tokio::main]
// Create an mitm listener
pub async fn listen(args: Args, ca: CertPair) {
  let addr = SocketAddr::new(args.host.parse::<IpAddr>().unwrap(), args.port.try_into().unwrap());
  ARGS.lock().unwrap().insert("args".to_string(), Arc::new(args.clone()));
  CA.lock().unwrap().insert("ca".to_string(), Arc::new(ca.clone()));

  let make_svc = make_service_fn(|_socket: &AddrStream| {
    return async move {
      // Callback for handling every incoming request
      return Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
        println!("✅ {}: Handling request {:?}", req.uri().to_string().red(), req);

        async move {
          // Construct the failure response body
          let mut failure = Response::new(Body::from("Error: Server error occurred."));
          *failure.status_mut() = StatusCode::BAD_REQUEST;
          // MITM the request
          let response = mitm(req).await.unwrap_or(failure);
          return Ok::<_, Infallible>(response);
        }
      }));
    };
  });

  // Bind to given port and start server
  let server = Server::bind(&addr).serve(make_svc);
  println!("✅ Listening on http://{}", addr.to_string().red());

  if let Err(e) = server.await {
    eprintln!("❌ server error: {}", e);
  }
}

// basic auth for the proxy server
fn ensure_auth(headers: &http::header::HeaderMap<http::header::HeaderValue>) -> bool {
  let args = ARGS.lock().unwrap().get_mut("args").unwrap().clone();
  let default_creds = &"".to_string();
  let username = args.basic_auth_user.as_ref().unwrap_or(default_creds);
  let password = args.basic_auth_password.as_ref().unwrap_or(default_creds);

  if headers.contains_key("Proxy-Authorization") {
    let encoded_creds = headers["Proxy-Authorization"].to_str().unwrap_or("Basic ").replace("Basic ", "");
    let creds = base64::decode(encoded_creds);

    return match creds {
      Ok(c) => {
        let x = String::from_utf8_lossy(&c);
        let mut s = x.split(":");
        let username_match = s.next().unwrap() == username;
        let password_match = s.next().unwrap() == password;

        username_match & password_match
      }
      Err(_e) => false,
    };
  } else {
    return false;
  }
}

async fn mitm(req: Request<Body>) -> Result<Response<Body>, Error> {
  let host: &str = &format!("{}://{}", req.uri().scheme_str().unwrap_or("https"), req.uri().authority().unwrap());

  if !ensure_auth(req.headers()) {
    println!("❌ {}: Authentication failure", host.red());
    return Ok(Response::builder().status(400).body(Body::from("Authentication Failure")).unwrap());
  }

  // Service a TLS CONNECT request
  if Method::CONNECT == req.method() {
    println!("✅ {}: Received a CONNECT request", host.red());
    handle_connect_request(req).await
  } else {
    println!("✅ {}: MITM-ing request", host.red());
    Ok(handle_proxy_request(req).await)
  }
}

async fn handle_connect_request(req: Request<Body>) -> Result<Response<Body>, Error> {
  let host: &str = &format!("{}://{}", req.uri().scheme_str().unwrap_or("https"), req.uri().authority().unwrap());
  let ca = CA.lock().unwrap().get_mut("ca").unwrap().clone();

  // Prepare: get TLS config with impersonating certs, sort out the target URI
  let authority = req.uri().authority().unwrap();
  let tls_conf = tls_config(authority, |_auth| mk_ca_signed_cert(&ca.cert, &ca.key, authority));

  // Build a client with TLS config
  let client_conf = client_config(req.uri().host().unwrap());
  let mut http = HttpConnector::new();
  http.enforce_http(false);
  let client: Client<HttpsConnector<HttpConnector>, Body> =
    Client::builder().build(HttpsConnector::from((http, client_conf)));

  // 1. Get TCP connection pool for target
  return client.connection_for((
    req.uri().scheme().unwrap_or(&Scheme::from_str("https").unwrap()).clone(),
    req.uri().authority().unwrap().clone()
  ))
  .map_err(|_e| {
    Error::new(ErrorKind::Other, format!("{}: Could not obtain TLS connection", host.red()))
  })
  .await
  // 2. Upgrade the HTTP connection from the client
  .map(move |_whatever| {
    // 3. Service the HTTP request inside the TCP tunnel
    let job = hyper::upgrade::on(req)
      .map_err(|e| {
        Error::new(ErrorKind::Other, format!("Could not upgrade connection: {}", e))
      })
      // 4. Perform the TLS handshake with the client
      .and_then(|upgraded:Upgraded| {
        println!("✅ Connection upgraded with client");
        TlsAcceptor::from(tls_conf).accept(upgraded)
      })
      // 5. Handle the underlying HTTP request
      .map(|accepted| {
        println!("✅ Established CONNECT TLS connection with client: {:?}", accepted);
        handle_https_proxy_request(accepted)
      })
      .flatten();

    tokio::task::spawn(job);
    println!("✅ {}: Accepting the CONNECT request", host.red());
    Response::builder().status(200).body(Body::empty()).unwrap()
  })
  .or_else(|e| {
    println!("❌ {}: Failed to perform the CONNECT handshake: {}", host.red(), e);
    Ok(Response::builder().status(502).body(Body::empty()).unwrap())
  });
}

async fn handle_proxy_request(req: Request<Body>) -> Response<Body> {
  // Build a client with TLS config
  let client_conf = client_config(req.uri().host().unwrap());
  let mut http = HttpConnector::new();
  http.enforce_http(false);
  let client: Client<HttpsConnector<HttpConnector>, Body> =
    Client::builder().build(HttpsConnector::from((http, client_conf)));

  // 1. Get TCP connection pool for target
  let mut pool = client
    .connection_for((
      req.uri().scheme().unwrap_or(&Scheme::from_str("https").unwrap()).clone(),
      req.uri().authority().unwrap().clone(),
    ))
    .await
    .ok()
    .unwrap();

  // 2. Log / MITM the request received from source
  let (parts, body) = req.into_parts();
  info!("✅ MITMed request: {:?} {:?}", parts, body);
  let modified_req = Request::from_parts(parts, body);

  // 3. Use the modified request to request the target
  let response = pool.send_request_retryable(modified_req).await.ok().unwrap();

  // 4. Log / MITM the response received from target
  let (resp_parts, resp_body) = response.into_parts();
  info!("✅ MITMed response: {:?} {:?}", resp_parts, resp_body);
  let modified_resp = Response::from_parts(resp_parts, resp_body);

  // 5. Pass on the modified response to source
  modified_resp
}

async fn handle_https_proxy_request(stream: Result<TlsStream<Upgraded>, Error>) -> () {
  let stream = Box::new(stream);

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

  let mut h = Http::new();
  h.http1_only(true);
  h.serve_connection(stream.unwrap(), svc)
    .map_err(|e: hyper::Error| {
      println!("❌ Error in serving http conection inside TLS tunnel {:?}", e);
    })
    .await
    .unwrap_or(());
}
