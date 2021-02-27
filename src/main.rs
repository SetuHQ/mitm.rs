extern crate clap;

// Compiler config
#[allow(unused_variables)]
#[allow(dead_code)]
mod util;

use clap::{load_yaml, App};
use openssl::pkey::{PKey, PKeyRef, Private};
use openssl::x509::X509;

use crate::util::args::{parse_args, Args};
use crate::util::cert::{mk_ca_cert, mk_ca_signed_cert, read_cert, read_pkey, verify};

fn main() {
    let yaml = load_yaml!("../config/cmd_args.yml");
    let app: App = App::from(yaml);

    let args: Args = parse_args(app);

    // load or generate a CA cert
    let (cert, pkey) = mk_ca_cert().unwrap();

    let ca_cert: X509 = args.ca_cert.map(|x| read_cert(x).unwrap()).unwrap_or(cert);

    let ca_privkey: PKey<Private> = args
        .ca_privkey
        .map(|x| read_pkey(x).unwrap())
        .unwrap_or(pkey);

    // load client certificates
    let client_keys: Option<Vec<X509>> = args.client_key.map(|a| {
        a.iter()
            .map(|x| read_cert(x.to_string()).unwrap())
            .collect()
    });
    let client_certs: Option<Vec<PKey<Private>>> = args.client_cert.map(|a| {
        a.iter()
            .map(|x| read_pkey(x.to_string()).unwrap())
            .collect()
    });
}
