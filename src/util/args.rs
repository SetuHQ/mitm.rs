use std::fmt::Display;
use std::{fmt, fs};

use clap::App;
use colored::*;
use serde::{Deserialize, Serialize};
use serde_json;


#[derive(Serialize, Deserialize, Debug)]
pub struct Args {
  #[serde(default)]
  pub ca_cert:             Option<String>,
  #[serde(default)]
  pub ca_privkey:          Option<String>,
  #[serde(default = "default_host")]
  pub host:                String,
  #[serde(default = "default_port")]
  pub port:                u32,
  #[serde(default)]
  pub client_key:          Option<Vec<String>>,
  #[serde(default)]
  pub client_cert:         Option<Vec<String>>,
  #[serde(default)]
  pub client_host:         Option<Vec<String>>,
  #[serde(default = "default_log_file")]
  pub log_file:            String,
  #[serde(default)]
  pub basic_auth_user:     Option<String>,
  #[serde(default)]
  pub basic_auth_password: Option<String>,
}

fn default_host() -> String { "127.0.0.1".to_string() }

fn default_port() -> u32 { 8080 }

fn default_log_file() -> String { "mitm_log.json".to_string() }

pub fn parse_args(app: App) -> Args {
  let matches = app.get_matches();

  if let Some(config_file) = matches.value_of("config") {
    // read and parse the config file
    let config_contents = fs::read_to_string(config_file).expect(&"Something went wrong reading the config file".red());

    // parse arguments from file
    let mut args: Args = serde_json::from_str(&config_contents).unwrap();

    // load file contents
    args.ca_cert = args.ca_cert.map(|x| fs::read_to_string(x).unwrap());
    args.ca_privkey = args.ca_privkey.map(|x| fs::read_to_string(x).unwrap());
    args.client_key = args.client_key.map(|a| a.iter().map(|x| fs::read_to_string(x).unwrap()).collect());
    args.client_cert = args.client_cert.map(|a| a.iter().map(|x| fs::read_to_string(x).unwrap()).collect());
    args
  } else {
    println!("{}", "Command line config file not present, proceeding to parse cmd params".magenta());

    // load certificate file contents
    let ca_cert: Option<String> = matches.value_of("ca_cert").map(|x| {
      let cert = fs::read_to_string(x).expect(&format!("Could not read ca cert {}", x.red().bold()));
      cert
    });
    let ca_privkey: Option<String> = matches.value_of("ca_privkey").map(|x| {
      let cert = fs::read_to_string(x).expect(&format!("Could not read ca cert {}", x.red().bold()));
      cert
    });

    let client_keys: Option<Vec<String>> = matches.value_of("client_key").map(|x| {
      let files: Vec<String> = x
        .to_string()
        .split(",")
        .map(|x| {
          let file_contents: String =
            fs::read_to_string(x).expect(&format!("Could not read client key {}", x.red().bold()));
          file_contents
        })
        .collect();

      files
    });

    let client_certs: Option<Vec<String>> = matches.value_of("client_cert").map(|x| {
      let files: Vec<String> = x
        .to_string()
        .split(",")
        .map(|x| {
          let file_contents: String =
            fs::read_to_string(x).expect(&format!("Could not read client cert {}", x.red().bold()));
          file_contents
        })
        .collect();

      files
    });

    let client_hosts: Option<Vec<String>> = matches.value_of("client_host").map(|x| {
      let hosts: Vec<String> = x.to_string().split(",").map(|x| x.to_string()).collect();

      hosts
    });

    // read other info
    let host: Option<String> = matches.value_of("host").map(|x| x.to_string());
    let port: Option<u32> = matches.value_of("port").map(|x| x.parse::<u32>().unwrap());
    let log_file: Option<String> = matches.value_of("log_file").map(|x| x.to_string());
    let basic_auth_user: Option<String> = matches.value_of("basic_auth_user").map(|x| x.to_string());
    let basic_auth_password: Option<String> = matches.value_of("basic_auth_password").map(|x| x.to_string());

    Args {
      ca_cert:             ca_cert,
      ca_privkey:          ca_privkey,
      host:                host.unwrap_or(default_host()),
      port:                port.unwrap_or(default_port()),
      client_key:          client_keys,
      client_cert:         client_certs,
      client_host:         client_hosts,
      log_file:            log_file.unwrap_or(default_log_file()),
      basic_auth_user:     basic_auth_user,
      basic_auth_password: basic_auth_password,
    }
  }
}
