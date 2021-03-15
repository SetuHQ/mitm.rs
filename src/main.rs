// Compiler config
#![allow(warnings)]
#![deny(unused_variables)]
#![allow(dead_code)]
#[macro_use]

mod mitm;
mod util;

use std::collections::HashMap;
use std::str::from_utf8;

use clap::{load_yaml, App};
use colored::*;
use openssl::pkey::{PKey, PKeyRef, Private};
use openssl::x509::X509;

// use crate::mitm::listen;
use crate::util::args::{parse_args, Args};
use crate::util::cert::{mk_ca_cert, mk_ca_signed_cert, read_cert, read_pkey, verify, CertPair, CERTIFICATES};


fn main() {
  println!("{}", "              d8b 888                                       ".truecolor(200, 100, 250).bold());
  println!("{}", "              Y8P 888                                       ".truecolor(200, 100, 250).bold());
  println!("{}", "                  888                                       ".truecolor(200, 100, 250).bold());
  println!("{}", "88888b.d88b.  888 888888 88888b.d88b.      888d888 .d8888b  ".truecolor(200, 100, 250).bold());
  println!("{}", "888 '888 '88b 888 888    888 '888 '88b     888P'   88K 888".truecolor(200, 100, 250).bold());
  println!("{}", "888  888  888 888 Y88b.  888  888  888 d8b 888          X88 ".truecolor(200, 100, 250).bold());
  println!("{}", "888  888  888 888  'Y888 888  888  888 Y8P 888      88888P' ".truecolor(200, 100, 250).bold());
  println!("{}", "                                                            ".truecolor(200, 100, 250).bold());
  println!("{}", "████████████████████████████████████████████████████████████".truecolor(200, 100, 250).bold());

  let yaml = load_yaml!("../config/cmd_args.yml");
  let app: App = App::from(yaml);

  let args: Args = parse_args(app);
  // make two copies, one for our ownership, one to pass around
  let args = args.clone();
  let args_other = args.clone();

  // load or generate a CA cert
  let (cert, pkey) = mk_ca_cert().unwrap();
  let ca_cert: X509 = args.ca_cert.map(|x| read_cert(x).unwrap()).unwrap_or(cert);
  let ca_privkey: PKey<Private> = args.ca_privkey.map(|x| read_pkey(x).unwrap()).unwrap_or(pkey);
  let ca_pair = CertPair { key: ca_privkey.clone(), cert: ca_cert.clone() };

  // load client certificates
  // let mut certs: Box<HashMap<String, CertPair>> = Box::new(HashMap::new());
  // let mut certs: HashMap<String, CertPair> = HashMap::new();
  let hosts = args.client_host.unwrap_or(vec![]);
  for (i, host) in hosts.iter().enumerate() {
    println!("✅ Configuring host no. {}: {} for client authentication", i, host.red());
    let cert_paths = args.client_cert.clone().unwrap_or(vec![]);
    let key_paths = args.client_key.clone().unwrap_or(vec![]);

    if cert_paths.len() > i && key_paths.len() > i {
      let cert_path = cert_paths[i].clone();
      let key_path = key_paths[i].clone();

      let cert = read_cert(cert_path.clone()).unwrap();
      let key = read_pkey(key_path).unwrap();

      CERTIFICATES.lock().unwrap().insert(host.clone(), CertPair { key, cert });
    }
  }

  println!("{}", format!("✅ CA certificate: \n{}", from_utf8(&ca_cert.to_pem().unwrap()).unwrap().magenta()));
  println!(
    "{}",
    format!("✅ CA private key: \n{}", from_utf8(&ca_privkey.private_key_to_pem_pkcs8().unwrap()).unwrap().magenta())
  );

  mitm::listen(args_other, ca_pair);
  // mitm::listen(args.host, args.port, &certs, ca_cert, ca_privkey);
}
