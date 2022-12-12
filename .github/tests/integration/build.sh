#!/usr/bin/env bash

SCRIPT_DIR=$(cd $(dirname $0);pwd)

docker build --target aesm \
--tag sgx_aesm:latest \
-f ${SCRIPT_DIR}/Dockerfile .

docker build --target lcp \
--tag lcp-ci-it:${TAG} \
--build-arg USERNAME=${USERNAME} \
--build-arg GROUPNAME=${GROUPNAME} \
--build-arg UID=$(id -u) \
--build-arg GID=$(id -g) \
-f ${SCRIPT_DIR}/Dockerfile .
