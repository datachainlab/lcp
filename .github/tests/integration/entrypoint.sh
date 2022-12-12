#!/usr/bin/env bash

set -e

export CARGO_INCREMENTAL=1
make -C lcp
make -C lcp integration-test
