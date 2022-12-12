FROM ubuntu:20.04 as aesm

ENV VERSION 2.17.100.3-focal1

SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN apt-get update && apt-get install -y gnupg2 software-properties-common wget

RUN wget -qO - https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key | apt-key add -
RUN add-apt-repository "deb https://download.01.org/intel-sgx/sgx_repo/ubuntu focal main"

RUN apt-get update && apt-get install -y \
    libsgx-aesm-launch-plugin=$VERSION \
    libsgx-aesm-quote-ex-plugin=$VERSION

RUN rm -rf /var/lib/apt/lists/* && rm -rf /var/cache/apt/archives/*

WORKDIR /opt/intel/sgx-aesm-service/aesm/
ENV LD_LIBRARY_PATH=.
CMD ./aesm_service --no-daemon

FROM ghcr.io/datachainlab/sgx-rust:2004-1.1.5 as lcp

ARG USERNAME=test-user
ARG GROUPNAME=test-user
ARG UID=1000
ARG GID=1000

ENV RUST_VERSION nightly-2022-02-23

RUN apt-get update -y && apt-get install jq make libclang-dev -y && \
    rm -rf /var/lib/apt/lists/* && rm -rf /var/cache/apt/archives/*

RUN groupadd -g $GID $GROUPNAME && \
    useradd -m -s /bin/bash -u $UID -g $GID $USERNAME

USER $USERNAME

WORKDIR /home/$USERNAME

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --profile default \
    --default-toolchain $RUST_VERSION \
    --component rust-src rls rust-analysis clippy rustfmt

RUN echo 'source /opt/sgxsdk/environment' >> $HOME/.bashrc

COPY --chown=$UID:$GID ./.github/tests/integration/entrypoint.sh /home/$USERNAME/entrypoint.sh
COPY --chown=$UID:$GID . ./lcp

ENTRYPOINT ["/bin/bash","-l","-c","/home/$USERNAME/entrypoint.sh"]
