#!/bin/sh

simd --home /root/${CHAINDIR}/${CHAINID} start --pruning=nothing --grpc.address="0.0.0.0:${GRPCPORT}"
