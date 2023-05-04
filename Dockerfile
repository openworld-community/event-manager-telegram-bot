FROM ubuntu:20.04
RUN apt-get update && apt-get install libssl-dev ca-certificates -y && update-ca-certificates
ARG ASSET_NAME
ARG REPOSITORY_ADDRESS
ARG RELEASE_VERSION
COPY cfg/$ASSET_NAME.toml /usr/local/etc/
ADD $REPOSITORY_ADDRESS/releases/download/$RELEASE_VERSION/$ASSET_NAME /usr/local/bin/
CMD ["$ASSET_NAME","-c","/usr/local/etc/$ASSET_NAME.toml"]