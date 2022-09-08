ARG GOLAGN_VER=1.18.4
ARG ALPINE_VER=3.16

FROM golang:${GOLAGN_VER}-alpine${ALPINE_VER} AS scaffold
WORKDIR /root

COPY ./ ./

RUN go build -mod readonly -o /root/build/simd ./simapp/simd

FROM alpine:${ALPINE_VER} AS initializer

RUN apk add --no-cache jq bash

WORKDIR /root

COPY --from=scaffold /root/build/simd /usr/bin/simd
COPY ./scripts ./scripts

ARG CHAINID
ARG CHAINDIR=./data
ARG RPCPORT=26657
ARG P2PPORT=26656
ARG PROFPORT=6060
ARG GRPCPORT=9090

RUN ./scripts/tm-chain simd $CHAINID $CHAINDIR $RPCPORT $P2PPORT $PROFPORT $GRPCPORT

FROM alpine:${ALPINE_VER}

WORKDIR /root

ARG CHAINID
ARG CHAINDIR=./data
ARG RPCPORT=26657
ARG P2PPORT=26656
ARG PROFPORT=6060
ARG GRPCPORT=9090

ENV CHAINID=$CHAINID
ENV CHAINDIR=$CHAINDIR
ENV GRPCPORT=$GRPCPORT

COPY --from=scaffold /root/build/simd /usr/bin/simd
COPY --from=initializer /root/$CHAINDIR /root/$CHAINDIR
COPY ./scripts/entrypoint.sh /root/entrypoint.sh

EXPOSE $RPCPORT $P2PPORT $PROFPORT $GRPCPORT

ENTRYPOINT ["./entrypoint.sh"]
