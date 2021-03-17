use std::sync::{Arc, Mutex};

use hyper::http::uri::Authority;
use lazy_static::lazy_static;
use lru_cache::LruCache;
use openssl::error::ErrorStack;
use openssl::pkey::{PKey, Private};
// use openssl::x509::extension::{
//   AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectAlternativeName, SubjectKeyIdentifier
// };
use openssl::x509::X509;
use rustls;
use rustls::{Certificate, ClientConfig, PrivateKey};

use crate::CERTIFICATES;

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
  let certs = CERTIFICATES.lock().unwrap();
  let client_cert = certs.get(host);

  // set client certificate if configured
  if !client_cert.is_none() {
    let c = client_cert.unwrap();
    // let c_cert = Certificate::from(c.cert.to_der().unwrap());
    let cert = c.cert.to_der().map(|x| Certificate(x)).unwrap();
    let key = c.key.private_key_to_der().map(|x| PrivateKey(x)).unwrap();
    client_config.set_single_client_cert(vec![cert], key).unwrap();
  }

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
