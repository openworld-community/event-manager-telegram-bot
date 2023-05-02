FROM rust:1.61.0 as builder
RUN git clone https://github.com/kosmos-954/event-manager-telegram-bot.git /usr/src/event-manager-telegram-bot
WORKDIR /usr/src/event-manager-telegram-bot
COPY . .
ARG RUSTFLAGS="-C target-feature=-crt-static"
RUN cargo install --path .
FROM ubuntu:20.04
RUN apt-get update && apt-get install libssl-dev ca-certificates -y && update-ca-certificates
COPY --from=builder /usr/local/cargo/bin/event-manager-telegram-bot /usr/local/bin/event-manager-telegram-bot
CMD ["event-manager-telegram-bot","-c","/usr/local/etc/event-manager-telegram-bot.toml"]