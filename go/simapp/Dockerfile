FROM golang:1.18.6-bullseye

COPY . /root/
WORKDIR /root/simapp

RUN go build -mod readonly -o /usr/bin/simd ./simd
RUN apt-get update -y && apt-get install -y jq

WORKDIR /root/simapp

ARG CHAINID
ARG CHAINDIR=/root/data
ARG RPCPORT=26657
ARG P2PPORT=26656
ARG PROFPORT=6060
ARG GRPCPORT=9090

RUN ./tm-chain simd $CHAINID $CHAINDIR $RPCPORT $P2PPORT $PROFPORT $GRPCPORT

ENV CHAINID=$CHAINID
ENV CHAINDIR=$CHAINDIR
EXPOSE $RPCPORT $P2PPORT $PROFPORT $GRPCPORT
ENTRYPOINT ["./entrypoint.sh"]
