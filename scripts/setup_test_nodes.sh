#!/bin/bash
# A script that runs two nodes(gaiad) in background and setup IBC connection and channel between the nodes
set -eux

TEST_HEALTH_CHECK_ADDR=${TEST_HEALTH_CHECK_ADDR:-127.0.0.1:7878}
TEST_STORE_DIR=${TEST_STORE_DIR:-data/test}
MAX_ATTEMPTS=${MAX_ATTEMPTS:-60}
RETRY_INTERVAL=${RETRY_INTERVAL:-1}

addr=(${TEST_HEALTH_CHECK_ADDR//:/ })
attempt_num=1
retry_interval=1

TEST_STORE_DIR=${TEST_STORE_DIR} make test-setup-nodes > /dev/null &

until [[ $(nc -d ${addr[0]} ${addr[1]}) == 'ok' ]]
do
    if (( attempt_num == MAX_ATTEMPTS )); then
        echo "Attempt $attempt_num failed and there are no more attempts left!"
        exit 1
    else
        sleep $RETRY_INTERVAL
        attempt_num=$(( $attempt_num+1 ))
    fi

done
