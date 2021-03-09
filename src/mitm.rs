use std::collections::HashMap;
use std::convert::{Infallible, TryInto};
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use bytes::Bytes;
use colored::*;
use futures_util::future::try_join;
use hyper::client::HttpConnector;
use hyper::http::uri::{Authority, Scheme};
use hyper::http::{StatusCode, Uri};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper::{Body, Client, Method, Request, Response, Server};
use hyper_tls::HttpsConnector;
// TLS
use openssl::pkey::{PKey, PKeyRef, Private};
use openssl::x509::X509;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufStream};
use tokio::net::TcpStream;
use tokio_native_tls::{native_tls, TlsConnector};

use crate::util::args::Args;
use crate::util::cert::{mk_ca_cert, mk_ca_signed_cert, read_cert, read_pkey, verify, CertPair, CERTIFICATES};
use crate::util::tls::tls_config;


#[tokio::main]
// Create an mitm listener
pub async fn listen(args: Args, ca: CertPair) {
  let addr = SocketAddr::new(args.host.parse::<IpAddr>().unwrap(), args.port.try_into().unwrap());

  let make_svc = make_service_fn(|socket: &AddrStream| {
    let remote_addr = socket.remote_addr();
    let args = args.clone();
    let ca = ca.clone();

    return async move {
      // Callback for handling every incoming request
      return Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
        println!("MITM handling request {:?}", req);

        let args = args.clone();
        let ca = ca.clone();

        async move {
          // Construct the failure response body
          let mut failure = Response::new(Body::from("Error: Server error occurred."));
          *failure.status_mut() = StatusCode::BAD_REQUEST;
          // MITM the request
          let response = mitm(req, args, ca).unwrap_or(failure);

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

fn mitm(req: Request<Body>, args: Args, ca: CertPair) -> Result<Response<Body>, hyper::Error> {
  let host: Option<String> = req.uri().authority().and_then(|auth| auth.as_str().parse().ok());

  if Method::CONNECT == req.method() {
    if let Some(addr) = host {
      println!("Received a CONNECT request from {}", addr.red());
      let connect_response =
        handle_connect_request(req).unwrap_or(Body::from("Error: Could not perform the CONNECT protocol handshake."));

      Ok(Response::new(connect_response))
    } else {
      eprintln!("CONNECT host is not socket addr: {:?}", req.uri());
      let mut resp = Response::new(Body::from("CONNECT must be to a socket address"));
      *resp.status_mut() = StatusCode::BAD_REQUEST;

      Ok(resp)
    }
  } else {
    // Bare HTTP request, just log and move on
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

fn handle_connect_request(req: Request<Body>) -> Option<Body> {
  let host: String = req.uri().authority().and_then(|auth| auth.as_str().parse().ok()).unwrap();

  // upgrade the connection
  // see https://en.wikipedia.org/wiki/HTTP/1.1_Upgrade_header
  tokio::task::spawn(async move {
    match hyper::upgrade::on(req).await {
      Ok(upgraded) => {
        if let Err(e) = tunnel(upgraded, host).await {
          eprintln!("server io error: {}", e);
        };
      }
      Err(e) => eprintln!("upgrade error: {}", e),
    }
  });

  Some(Body::from("lol"))
}

// async fn mitm_tls_connection(host: &str, port:&str) {
//   let authority = Authority::from_shared(Bytes::from(connect_req.uri().to_string())).unwrap();
// }

// Create a TCP connection to host:port, build a tunnel between the connection and
// the upgraded connection
async fn tunnel(upgraded: Upgraded, host: String) -> std::io::Result<()> {
  // Connect to remote server
  let mut server = TcpStream::connect(host.clone()).await.unwrap();

  // set up TLS connection:
  // 1. request TLS cert from target
  // 2. create and sign cert by spoofing target cert using mitm CA cert
  // let connector = native_tls::TlsConnector::builder().build().unwrap();
  // let tokio_connector = tokio_native_tls::TlsConnector::from(connector);
  // let mut target_stream = tokio_connector.connect(&host, server).await.unwrap();
  // let certificate = &target_stream.get_ref().peer_certificate().unwrap();
  // let certificate = match certificate {
  //   Some(cert) => cert,
  //   None => {
  //     return Err(Error::new(ErrorKind::Other, "oh no!"))
  //   }
  // };
  // let certificate = X509::from_der(&certificate.to_der().unwrap()).unwrap();

  // println!("{}", String::from_utf8_lossy(certificate.to_pem().unwrap().as_slice()));

  // Proxying data
  // let amounts = {
  //   let (mut server_rd, mut server_wr) = server.split();
  //   let (mut client_rd, mut client_wr) = tokio::io::split(upgraded);

  //   // FIXME: arbitrary static size
  //   let mut request: Vec<u8> = Vec::with_capacity(10000);
  //   let mut response: Vec<u8> = Vec::with_capacity(10000);
  //   // let mut _request = BufStream::new();
  //   // let mut _response = BufStream::new();
  //   // let (mut request_r, mut request_w) = io::duplex(1000);
  //   // let (mut response_r, mut response_w) = io::duplex(1000);
  //   // let response = io::empty();

  //   let client_to_mitm = client_rd.read_buf(&mut request).await.unwrap();
  //   // let client_to_server = server_wr.write_buf(&mut request.as_slice()).await.unwrap();
  //   // let client_to_mitm = tokio::io::copy(&mut client_rd, &mut request_w);
  //   // let client_to_server = tokio::io::copy(&mut request_r, &mut server_wr);
  //   // let from_client = try_join(client_to_mitm, client_to_server).await;

  //   // let server_to_mitm = server_rd.read_buf(&mut response).await.unwrap();
  //   // let server_to_client = client_wr.write_buf(&mut response.as_slice()).await.unwrap();
  //   // let server_to_mitm = tokio::io::copy(&mut server_rd, &mut response_w);
  //   // let server_to_client = tokio::io::copy(&mut response_r, &mut client_wr);
  //   // let from_server = try_join(server_to_mitm, server_to_client).await;


  //   // println!("{} {} ", client_to_mitm, client_to_server);
  //   // println!("{} {} {} {}", client_to_mitm, client_to_server, server_to_mitm, server_to_client);
  //   // let req = String::from_utf8_lossy(&request);
  //   // let resp = String::from_utf8_lossy(&response);
  //   // println!("result: {} \n====================================\n {}", req, resp);

  //   // let s = String::from_utf8_lossy(&request);
  //   // println!("result: {}", s);

  //   let client_to_server = tokio::io::copy(&mut client_rd, &mut server_wr);
  //   // let server_to_client = tokio::io::copy(&mut server_rd, &mut client_wr);
  //   // let copied_data = try_join(client_to_server, server_to_client).await;

  //   // // let request = BufStream::new()
  //   // copied_data.map(|(_out, _in)| {
  //   //   let mut request:&[u8] = Vec::with_capacity(_out as usize).as_slice();
  //   //   let mut response:&[u8] = Vec::with_capacity(_in as usize).as_slice();

  //   //   // tokio::io::copy(&mut client_rd, &mut request);
  //   //   let mut buffer = String::new();
  //   //   client_rd.read_to_string(&mut buffer);
  //   //   println!("{} -------------------------------", buffer);

  //   //   (_out, _in)
  //   // })
  // };

  // Print message when done
  // match amounts {
  //   Ok((from_client, from_server)) => {
  //     println!("client wrote {} bytes and received {} bytes", from_client, from_server);
  //   }
  //   Err(e) => {
  //     println!("tunnel error: {}", e);
  //   }
  // };

  Ok(())
}