#!/bin/bash
# A script that runs the integration test with the running nodes
# Prerequisites: scripts/setup_test_nodes.sh is already executed
set -eux

TEST_STORE_DIR=${TEST_STORE_DIR:-data/test}
source ${TEST_STORE_DIR}/binary-channels.env
source ${TEST_STORE_DIR}/binary-chains.env

SETUP_NODES=false TEST_NODE_CHAIN_ID=$CHAIN_ID_A TEST_NODE_RPC_ADDR=$NODE_A_RPC_ADDR TEST_NODE_GRPC_ADDR=$NODE_A_GRPC_ADDR make integration-test
