FROM bluele/sgx-rust:2004-1.1.6

RUN apt-get update -y && apt-get install -y libclang-dev

ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m -s /bin/bash $USERNAME

USER dev
RUN cd /home/dev && \
curl 'https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init' --output /home/dev/rustup-init && \
chmod +x /home/dev/rustup-init && \
echo '1' | /home/dev/rustup-init --default-toolchain ${rust_toolchain} && \
echo 'source /home/dev/.cargo/env' >> /home/dev/.bashrc && \
/home/dev/.cargo/bin/rustup component add rust-src rls rust-analysis clippy rustfmt && \
/home/dev/.cargo/bin/cargo install xargo && \
rm /home/dev/rustup-init && rm -rf /home/dev/.cargo/registry && rm -rf /home/dev/.cargo/git
RUN echo 'source /opt/sgxsdk/environment' >> /home/dev/.bashrc
