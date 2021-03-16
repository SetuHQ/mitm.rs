############################
# STEP 1 build executable binary
############################
FROM rust:1-slim-buster as builder

ENV CODEDIR /app

COPY . ${CODEDIR}
WORKDIR ${CODEDIR}

RUN make release

############################
# STEP 2 build a small image
############################
FROM debian:buster-slim

RUN mkdir /app
WORKDIR /app

COPY --from=builder /app/mitm.rs /app/mitm.rs
COPY config.json /app/config.json

RUN groupadd -r appgroup && useradd -r -g appgroup appuser
RUN mkdir -p /home/appuser && chown -R appuser /home/appuser

USER appuser
EXPOSE 8080

CMD ["/app/mitm.rs --config_file /app/confg.json"]
