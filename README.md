# mitm.rs

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
## Table of Contents

- [Run](#run)
  - [1. JSON file](#1-json-file)
  - [2. Command-line parameters](#2-command-line-parameters)
- [Deploy](#deploy)
- [Develop](#develop)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

MITM-enabled auditing proxy server for outbound traffic.
Logs all requests going out of a network.

## Run

The server can be configured using:

### 1. JSON file

```json
{
  "ca_cert"             : ./ca.pem,
  "ca_privkey"          : ./ca_priv.pem,
  "host"                : 0.0.0.0,
  "port"                : 8080,
  "client_key"          : "./key1.pem,./key2.pem,./key3.pem",
  "client_cert"         : "./cert1.pem,./cert2.pem,./cert3.pem",
  "client_host"         : "website1.com,website2.com,website3.com",
  "log_file"            : "/var/log/mitm.log",
  "basic_auth_user"     : username,
  "basic_auth_password" : password,
}
```

```bash
mitm.rs --config_file ./confg.json
```

### 2. Command-line parameters

```bash
mitm.rs \
  --ca_cert ./ca.pem                                     \
  --ca_privkey ./ca_priv.pem                             \
  --host 0.0.0.0                                         \
  --port 8080                                            \
  --client_key "./key1.pem,./key2.pem,./key3.pem"        \
  --client_cert "./cert1.pem,./cert2.pem,./cert3.pem"    \
  --client_host "website1.com,website2.com,website3.com" \
  --log_file "/var/log/mitm.log"                         \
  --basic_auth_user username                             \
  --basic_auth_password password                         \
```

## Deploy

## Develop
