use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::sync::{Arc, Mutex};

use colored::*;
use hyper::http::uri::Authority;
use lazy_static::lazy_static;
use lru_cache::LruCache;
use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, PKeyRef, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{
  AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectAlternativeName, SubjectKeyIdentifier
};
use openssl::x509::{X509NameBuilder, X509Ref, X509Req, X509ReqBuilder, X509VerifyResult, X509};
use rustls;
use rustls::{Certificate, ClientConfig, PrivateKey};


// The global SSL certificates cache
lazy_static! {
  static ref TLS_CONFIG_CACHE: Mutex<LruCache<String, Arc<rustls::ServerConfig>>> = Mutex::new(LruCache::new(1000));
}

// Either load an existing TLS server configuration from cache or build a new
// one (and cache it) for the provided authority.
pub fn tls_config<T>(authority: &Authority, cert_creator: T) -> Arc<rustls::ServerConfig>
where
  T: Fn(&Authority) -> Result<(X509, PKey<Private>), ErrorStack>, {
  if !TLS_CONFIG_CACHE.lock().unwrap().contains_key(authority.host()) {
    let tls_cfg: Arc<rustls::ServerConfig> = {
      // create certificates for current target domain and convert into rustls types
      let (rustls_cert, rustls_key) = cert_creator(&authority)
        .map(|(cert, key)| {
          let c = cert.to_der().map(|x| Certificate(x)).unwrap();
          let k = key.private_key_to_der().map(|x| PrivateKey(x)).unwrap();
          (c, k)
        })
        .unwrap();

      let certs = vec![rustls_cert; 1];
      let mut result = rustls::ServerConfig::new(rustls::NoClientAuth::new());
      result
        .set_single_cert(certs, rustls_key)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e)))
        .unwrap();
      Arc::new(result)
    };

    TLS_CONFIG_CACHE.lock().unwrap().insert(authority.host().to_owned(), tls_cfg);
  }

  TLS_CONFIG_CACHE.lock().unwrap().get_mut(authority.host()).unwrap().clone()
}

// Generate TLS config for a client, with client certificates if host is provided
pub fn client_config(host: &str) -> ClientConfig {
  let mut client_config = ClientConfig::new();
  // .set_single_client_cert() // TODO: Add client certificates here
  // Load system certificates
  client_config.root_store = match rustls_native_certs::load_native_certs() {
    Ok(store) => store,
    Err((Some(store), err)) => {
      println!("❌ Could not load all certificates: {:?}", err);
      store
    }
    Err((None, err)) => Err(err).expect("cannot access native cert store"),
  };
  client_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

  client_config
}
