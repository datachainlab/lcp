FROM ghcr.io/datachainlab/sgx-rust:2004-1.1.5

SHELL ["/bin/bash", "-c", "-l"]

WORKDIR /root

RUN apt-get update -y && apt-get install jq make -y

COPY ./.github/resources/it/entrypoint.sh ./entrypoint.sh

COPY ./ ./lcp
RUN make -C lcp

RUN rm -rf /var/lib/apt/lists/* && \
  rm -rf /var/cache/apt/archives/*

ENTRYPOINT ["/bin/bash","-l","-c","/root/entrypoint.sh"]
