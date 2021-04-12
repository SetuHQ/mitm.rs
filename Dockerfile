############################
# STEP 1 build executable binary
############################
FROM rust:1-slim-buster as builder

ENV CODEDIR /app

COPY . ${CODEDIR}
WORKDIR ${CODEDIR}

RUN apt-get update && apt-get install -y build-essential libssl-dev pkg-config

RUN make release

############################
# STEP 2 build a small image
############################
FROM debian:buster-slim

RUN mkdir /app
WORKDIR /app

RUN apt-get update && apt-get install -y libssl-dev pkg-config

COPY --from=builder /app/mitm.rs /app/mitm.rs
COPY config/config.json /app/config.json

# Copy certs
COPY certs /app/

RUN groupadd -r appgroup && useradd -r -g appgroup appuser
RUN mkdir -p /home/appuser && chown -R appuser /home/appuser

USER appuser
EXPOSE 8080

CMD ["RUST_BACKTRACE=1 /app/mitm.rs --config /app/config.json"]
