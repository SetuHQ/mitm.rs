# mitm.rs

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
## Table of Contents

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

## Deploy

## Develop
