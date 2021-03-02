// Compiler config
#![allow(warnings)]
#![allow(unused_variables)]
#![allow(dead_code)]

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
use crate::util::cert::{mk_ca_cert, mk_ca_signed_cert, read_cert, read_pkey, verify};


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

  // load or generate a CA cert
  let (cert, pkey) = mk_ca_cert().unwrap();

  let ca_cert: X509 = args.ca_cert.map(|x| read_cert(x).unwrap()).unwrap_or(cert);

  let ca_privkey: PKey<Private> = args.ca_privkey.map(|x| read_pkey(x).unwrap()).unwrap_or(pkey);

  // load client certificates
  let client_keys: Option<Vec<PKey<Private>>> =
    args.client_key.map(|a| a.iter().map(|x| read_pkey(x.to_string()).unwrap()).collect());
  let client_certs: Option<Vec<X509>> =
    args.client_cert.map(|a| a.iter().map(|x| read_cert(x.to_string()).unwrap()).collect());

  println!("{}", format!("CA certificate: \n{}", from_utf8(&ca_cert.to_pem().unwrap()).unwrap().magenta()));
  println!(
    "{}",
    format!("CA private key: \n{}", from_utf8(&ca_privkey.private_key_to_pem_pkcs8().unwrap()).unwrap().magenta())
  );
}
