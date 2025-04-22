# Build with:
#
#     docker buildx build --platform linux/arm/v7,linux/arm64 -t your-image-name ./docker/multiarch-node/
#
###############################################################################
#
FROM --platform=$BUILDPLATFORM ubuntu:22.04

RUN apt-get update && apt-get -y upgrade && apt-get -y autoremove && apt-get -y clean && apt-get -f -y install build-essential wget pkg-config curl sudo

# Specify Node.js version with:
#
#    --build-arg NODE_VERSION=22
#
# Valid values: 18, 20, 22
ARG NODE_VERSION=18
RUN curl -sL https://deb.nodesource.com/setup_$NODE_VERSION.x | sudo -E bash -
RUN apt-get install -y nodejs
RUN apt-get install -y git
