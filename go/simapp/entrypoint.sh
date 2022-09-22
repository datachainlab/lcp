#!/bin/sh
set -x

simd --home ${CHAINDIR}/${CHAINID} start --pruning=nothing
