FROM ghcr.io/datachainlab/sgx-rust:2004-1.1.5

SHELL ["/bin/bash", "-c", "-l"]

WORKDIR /root

RUN apt-get update -y && apt-get install jq make -y

COPY ./.ci/tests/integration/entrypoint.sh ./entrypoint.sh

COPY ./ ./lcp

RUN rm -rf /var/lib/apt/lists/* && \
  rm -rf /var/cache/apt/archives/*

ENTRYPOINT ["/bin/bash","-l","-c","/root/entrypoint.sh"]
