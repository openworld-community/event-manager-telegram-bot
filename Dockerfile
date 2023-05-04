FROM ubuntu:20.04
RUN apt-get update && apt-get install libssl-dev ca-certificates -y && update-ca-certificates
RUN wget
CMD ["event-manager-telegram-bot","-c","/usr/local/etc/event-manager-telegram-bot.toml"]