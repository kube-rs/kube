FROM docker.io/rust:1.56-bullseye

ENV DEBIAN_FRONTEND=noninteractive
RUN apt update && apt upgrade -y
RUN apt install -y \
    clang \
    cmake \
    lldb \
    lsb-release \
    sudo \
    time

ARG USER=code
ARG USER_UID=1000
ARG USER_GID=1000
RUN groupadd --gid=$USER_GID $USER \
    && useradd --uid=$USER_UID --gid=$USER_GID -m $USER \
    && echo "$USER ALL=(root) NOPASSWD:ALL" >/etc/sudoers.d/$USER \
    && chmod 0440 /etc/sudoers.d/$USER

# Install a Docker client that uses the parent host's Docker daemon
ARG USE_MOBY=false
ENV DOCKER_BUILDKIT=1
RUN curl --proto '=https' --tlsv1.3 -vsSfL https://raw.githubusercontent.com/microsoft/vscode-dev-containers/main/script-library/docker-debian.sh \
    | bash -s -- true /var/run/docker-host.sock /var/run/docker.sock "${USER}" "${USE_MOBY}" latest

USER $USER
ENV HOME=/home/$USER
RUN mkdir -p $HOME/bin
ENV PATH=$HOME/bin:$PATH

# Install `kubectl`
RUN curl --proto '=https' --tlsv1.3 -vsSfLo $HOME/kubectl "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl" \
    && chmod 755 $HOME/kubectl

# Install `k3d`
RUN curl --proto '=https' --tlsv1.3 -vsSfL https://raw.githubusercontent.com/rancher/k3d/main/install.sh \
    | USE_SUDO=false K3D_INSTALL_DIR=$HOME/bin bash

RUN rustup toolchain add nightly \
    && rustup default nightly \
    && rustup component add clippy rls rust-src rustfmt

# Install cargo-deny
RUN mkdir /tmp/cargo-deny && cd /tmp/cargo-deny && \
    curl --proto '=https' --tlsv1.3 -vsSfL https://github.com/EmbarkStudios/cargo-deny/releases/download/0.11.0/cargo-deny-0.11.0-x86_64-unknown-linux-musl.tar.gz \
        | tar zxf - \
    && mv cargo-deny-0.11.0-x86_64-unknown-linux-musl/cargo-deny $HOME/bin \
    && cd .. && rm -rf /tmp/cargo-deny

# Install cargo-tarpaulin
ARG CARGO_TARPAULIN_VERSION=0.18.5
RUN cd $HOME/bin && curl --proto '=https' --tlsv1.3 -sSfL "https://github.com/xd009642/tarpaulin/releases/download/${CARGO_TARPAULIN_VERSION}/cargo-tarpaulin-${CARGO_TARPAULIN_VERSION}-travis.tar.gz" | tar xzvf -

ENTRYPOINT ["/usr/local/share/docker-init.sh"]
CMD ["sleep", "infinity"]
