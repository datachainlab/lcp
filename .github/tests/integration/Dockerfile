FROM ghcr.io/datachainlab/sgx-rust:2004-1.1.5

SHELL ["/bin/bash", "-c", "-l"]

WORKDIR /root

COPY ./.github/tests/integration/entrypoint.sh ./entrypoint.sh

COPY ./ ./lcp

ENTRYPOINT ["/bin/bash","-l","-c","/root/entrypoint.sh"]
