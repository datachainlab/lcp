#!/bin/bash
set -eox pipefail

if [ $# -eq 0 ]; then
    echo "No arguments supplied"
    exit 1
fi
SDK_DIR_PREFIX=$1

DCAP_VERSION=1.21.100.3-jammy1
# create tmp dir
TMP_DIR=$(mktemp -d)
echo "Created temp dir: $TMP_DIR"
cd $TMP_DIR
# clone the repo
git clone --recursive https://github.com/intel/SGXDataCenterAttestationPrimitives  -b dcap_1.21_reproducible --depth 1

wget https://download.01.org/intel-sgx/sgx-dcap/1.21/linux/distro/ubuntu22.04-server/sgx_linux_x64_sdk_2.24.100.3.bin -O sgx_linux_x64_sdk.bin
chmod a+x sgx_linux_x64_sdk.bin
./sgx_linux_x64_sdk.bin --prefix=$SDK_DIR_PREFIX
rm -rf ./sgx_linux_x64_sdk.bin

wget https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key
cat intel-sgx-deb.key | tee /etc/apt/keyrings/intel-sgx-keyring.asc > /dev/null
echo 'deb [signed-by=/etc/apt/keyrings/intel-sgx-keyring.asc arch=amd64] https://download.01.org/intel-sgx/sgx_repo/ubuntu jammy main' | tee /etc/apt/sources.list.d/intel-sgx.list

apt-get update -y
apt-get install -y libsgx-dcap-ql=$DCAP_VERSION libsgx-dcap-ql-dev=$DCAP_VERSION
