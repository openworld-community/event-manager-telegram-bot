FROM ubuntu:20.04
RUN apt-get update && apt-get install libssl-dev ca-certificates -y && update-ca-certificates
ARG REPOSITORY_ADDRESS
ARG RELEASE_VERSION
COPY cfg/event-manager-telegram-bot.toml /usr/local/etc/
ADD $REPOSITORY_ADDRESS/releases/download/$RELEASE_VERSION/event-manager-telegram-bot /usr/local/bin/
RUN chmod +x /usr/local/bin/event-manager-telegram-bot
RUN mkdir /data
CMD ["event-manager-telegram-bot","-c","/usr/local/etc/event-manager-telegram-bot.toml"]