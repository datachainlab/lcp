#!/bin/sh
set -eux
CONFIG_DIR=./tests/e2e/cases/tm2tm/configs/demo
MRENCLAVE=$(./bin/lcp enclave metadata --enclave=./bin/enclave.signed.so | jq -r .mrenclave)
jq --arg MRENCLAVE ${MRENCLAVE} -r '.prover.mrenclave = $MRENCLAVE' ${CONFIG_DIR}/ibc-0.json.tpl > ${CONFIG_DIR}/ibc-0.json
jq --arg MRENCLAVE ${MRENCLAVE} -r '.prover.mrenclave = $MRENCLAVE' ${CONFIG_DIR}/ibc-1.json.tpl > ${CONFIG_DIR}/ibc-1.json
