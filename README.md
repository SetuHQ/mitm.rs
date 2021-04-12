# mitm.rs

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
## Table of Contents

- [TLDR](#tldr)
- [Run](#run)
  - [1. JSON file](#1-json-file)
  - [2. Command-line parameters](#2-command-line-parameters)
- [Testing](#testing)
  - [Prelude](#prelude)
  - [GET](#get)
  - [POST](#post)
  - [PUT](#put)
  - [PATCH](#patch)
  - [DELETE](#delete)
  - [Client certificates](#client-certificates)
- [Docker](#docker)
- [Develop](#develop)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

MITM-enabled auditing proxy server for outbound traffic.
Logs all requests going out of a network.

## TLDR

```bash
 docker run -it --rm -p 8080:8080 --mount type=bind,source="$(pwd)"/certs,target=/certs quay.io/setuinfra/mitm.rs:latest /app/mitm.rs \
  --ca_cert /certs/ca.pem \
  --ca_privkey /certs/ca_priv.pem \
  --host 0.0.0.0 \
  --port 8080 \
  --log_file "/tmp/mitm.log" \
  --basic_auth_user user \
  --basic_auth_password password
```

## Run

The server can be configured using:

### 1. JSON file

```json
{
  "ca_cert"             : "./ca.pem",
  "ca_privkey"          : "./ca_priv.pem",
  "host"                : "0.0.0.0",
  "port"                : 8080,
  "client_key"          : "./key1.pem,./key2.pem,./key3.pem",
  "client_cert"         : "./cert1.pem,./cert2.pem,./cert3.pem",
  "client_host"         : "website1.com,website2.com,website3.com",
  "log_file"            : "/var/log/mitm.log",
  "basic_auth_user"     : "username",
  "basic_auth_password" : "password",
}
```

```bash
mitm.rs --config_file ./confg.json
```

### 2. Command-line parameters

```markdown
mitm.rs 0.1
infra@setu.co

USAGE:
    mitm [OPTIONS]

FLAGS:
        --help
            Prints help information

    -V, --version
            Prints version information


OPTIONS:
    -w, --basic_auth_password <basic_auth_password>
            Password for basic auth [Optional]

    -u, --basic_auth_user <basic_auth_user>
            Username for basic auth [Optional]

        --ca_cert <ca_cert>
            Path of the CA Certificate to use [Optional]

        --ca_privkey <ca_privkey>
            Path of the CA private key to use [Optional]

        --client_cert <client_cert>
            Client certificate location (comma separated) [Optional]

        --client_host <client_host>
            Websites / endpoint for which corresponding client certificate applies (comma separated) [Optional]

        --client_key <client_key>
            Client certificate's private key location (comma separated) [Optional]

    -f, --config <FILE>
            Pick command line options from a config file

    -h, --host <host>
            Hostname to listen to [default: 127.0.0.1]

    -l, --log_file <log_file>
            Log requests to file [default: requests.log.json]

    -p, --port <port>
            Port to listen to [default: 8080]
```

For example:

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
  --basic_auth_password password
```

## Testing

The mitm proxy can be tested for REST APIs like so:

### Prelude

```bash
export USER=<proxy creds - username>
export PASS=<proxy creds - password>
export HOST=<proxy - host name>
export PORT=<proxy port (8080)>
```

Place the CA cert in current directory (`cert.pem`).

### GET

```bash
curl --cacert ./cert.pem -vvv\
  -x "http://${USER}:${PASS}@${HOST}:${PORT}/" \
  https://jsonplaceholder.typicode.com/posts/1
```

expected response:

```json
{
  "userId": 1,
  "id": 1,
  "title": "sunt aut facere repellat provident occaecati excepturi optio reprehenderit",
  "body": "quia et suscipit\nsuscipit recusandae consequuntur expedita et cum\nreprehenderit molestiae ut ut quas totam\nnostrum rerum est autem sunt rem eveniet architecto"
}
```

### POST

```bash
curl --cacert ./cert.pem -vvv\
  -x "http://${USER}:${PASS}@${HOST}:${PORT}/" \
  --request POST -sL \
  --url 'https://jsonplaceholder.typicode.com/posts' \
  --data 'title=foo' \
  --data 'body=bar' \
  --data 'userId=1'
```

expected response:

```json
{
  "title": "foo",
  "body": "bar",
  "userId": "1",
  "id": 101
}
```

### PUT

```bash
curl --cacert ./cert.pem -vvv\
  -x "http://${USER}:${PASS}@${HOST}:${PORT}/" \
  --request PUT -sL \
  --url 'https://jsonplaceholder.typicode.com/posts/1' \
  --data 'title=foo' \
  --data 'body=bar' \
  --data 'userId=1' \
  --data 'id=1'
```

expected response:

```json
{
  "title": "foo",
  "body": "bar",
  "userId": "1",
  "id": 1
}
```

### PATCH

```bash
curl --cacert ./cert.pem -vvv\
  -x "http://${USER}:${PASS}@${HOST}:${PORT}/" \
  --request PATCH -sL \
  --url 'https://jsonplaceholder.typicode.com/posts/1' \
  --data 'title=lolfoo'
```

expected response:

```json
{
  "userId": 1,
  "id": 1,
  "title": "lolfoo",
  "body": "quia et suscipit\nsuscipit recusandae consequuntur expedita et cum\nreprehenderit molestiae ut ut quas totam\nnostrum rerum est autem sunt rem eveniet architecto"
}
```

### DELETE

```bash
curl --cacert ./cert.pem -vvv\
  -x "http://${USER}:${PASS}@${HOST}:${PORT}/" \
  --request DELETE -sL \
  --url 'https://jsonplaceholder.typicode.com/posts/1'
```

expected response:

```json
{}
```

### Client certificates

Download client certificates from https://badssl.com/download/

```bash
mitm.rs \
  --ca_cert ./certs/ca.pem \
  --ca_privkey ./certs/ca_priv.pem \
  --host 0.0.0.0 \
  --port 8080 \
  --client_key "./certs/badssl-key.pem" \
  --client_cert "./certs/badssl-cert.pem" \
  --client_host "client.badssl.com" \
  --log_file "./mitm.log" \
  --basic_auth_user username \
  --basic_auth_password password

# Enter certificate passkey: `badssl.com`
```

Test curl on another terminal:

```bash
export USER=username
export PASS=password
export HOST=localhost
export PORT=8080

curl --cacert ./cert.pem -vvv\
  -x "http://${USER}:${PASS}@${HOST}:${PORT}/" \
  https://client.badssl.com/
```

## Docker

A docker container can be built using

```bash
make docker
```

and run like:

```bash
docker run -it --rm -p8080:8080 mitm.rs:latest /app/mitm.rs --config /app/config.json
```

## Develop

All commands required for development are captured in the makefile:

```bash
$ make help

build                          Build the server
run                            Run the server
watch                          Build, watch for changes and restart
watch-run                      Build and run, watch for changes and restart
release                        Create a release binary `mitm.rs`
publish                        Publish into crates.io
help                           Dislay this help
```
