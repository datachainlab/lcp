#!/usr/bin/env bash

set -e

LD_LIBRARY_PATH=/opt/intel/sgx-aesm-service/aesm/
/opt/intel/sgx-aesm-service/aesm/aesm_service

make -C lcp integration-test
