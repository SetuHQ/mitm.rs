use std::collections::HashMap;
use std::convert::{Infallible, TryInto};
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use colored::*;
use futures_util::future::try_join;
use hyper::client::HttpConnector;
use hyper::http::uri::Scheme;
use hyper::http::{StatusCode, Uri};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper::{Body, Client, Method, Request, Response, Server};
use hyper_tls::HttpsConnector;
use openssl::x509::X509;
use tokio::net::TcpStream;
use tokio_native_tls::TlsConnector;

use crate::util::cert::{mk_ca_cert, mk_ca_signed_cert, read_cert, read_pkey, verify, CertPair};


#[tokio::main]
pub async fn listen(host: String, port: u32, client_certs: HashMap<String, CertPair>) {
  let addr = SocketAddr::new(host.parse::<IpAddr>().unwrap(), port.try_into().unwrap());
  let certs = client_certs.clone();

  let make_svc = make_service_fn(|socket: &AddrStream| {
    let remote_addr = socket.remote_addr();
    // let c = &certs;

    return async move {
      return Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
        async move {
          let mut resp = Response::new(Body::from("Error: Server error occurred."));
          *resp.status_mut() = StatusCode::BAD_REQUEST;

          let response = mitm(req).unwrap_or(resp);

          return Ok::<_, Infallible>(response);
        }
      }));
    };
  });

  let server = Server::bind(&addr).serve(make_svc);
  println!("Listening on http://{}", addr.to_string().red());

  if let Err(e) = server.await {
    eprintln!("server error: {}", e);
  }
}

fn mitm(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
  let host: Option<String> = req.uri().authority().and_then(|auth| auth.as_str().parse().ok());

  if Method::CONNECT == req.method() {
    if let Some(addr) = host {
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
    println!("adnfhkfvfsdvdfufvbvbeuvugegvfedrbbgebvhe--------------------");
    Ok(Response::new(Body::from("Not implemented yet")))
  }
}

fn handle_connect_request(req: Request<Body>) -> Option<Body> {
  // upgrade the connection
  // see https://en.wikipedia.org/wiki/HTTP/1.1_Upgrade_header

  let host: String = req.uri().authority().and_then(|auth| auth.as_str().parse().ok()).unwrap();
  // println!("{} =========================================", host);

  // let x:SocketAddr = host.parse().unwrap();

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

// Create a TCP connection to host:port, build a tunnel between the connection and
// the upgraded connection
async fn tunnel(upgraded: Upgraded, addr: String) -> std::io::Result<()> {
  // Connect to remote server
  let mut server = TcpStream::connect(addr).await?;

  // Proxying data
  let amounts = {
    let (mut server_rd, mut server_wr) = server.split();
    let (mut client_rd, mut client_wr) = tokio::io::split(upgraded);

    let client_to_server = tokio::io::copy(&mut client_rd, &mut server_wr);
    let server_to_client = tokio::io::copy(&mut server_rd, &mut client_wr);

    try_join(client_to_server, server_to_client).await
  };

  // Print message when done
  match amounts {
    Ok((from_client, from_server)) => {
      println!("client wrote {} bytes and received {} bytes", from_client, from_server);
    }
    Err(e) => {
      println!("tunnel error: {}", e);
    }
  };
  Ok(())
}
