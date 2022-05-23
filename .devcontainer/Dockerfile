FROM docker.io/rust:1.60.0-bullseye

ENV DEBIAN_FRONTEND=noninteractive
RUN apt update && apt upgrade -y
RUN apt install -y \
    clang \
    cmake \
    jq \
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

COPY scurl /usr/local/bin/scurl

# Install a Docker client that uses the parent host's Docker daemon
ARG USE_MOBY=false
ENV DOCKER_BUILDKIT=1
RUN scurl https://raw.githubusercontent.com/microsoft/vscode-dev-containers/main/script-library/docker-debian.sh \
    | bash -s -- true /var/run/docker-host.sock /var/run/docker.sock "${USER}" "${USE_MOBY}" latest

USER $USER
ENV HOME=/home/$USER
RUN mkdir -p $HOME/bin
ENV PATH=$HOME/bin:$PATH

# Install `kubectl`
RUN export K8S_VERSION="$(scurl https://dl.k8s.io/release/stable.txt)" \
    && scurl -o $HOME/bin/kubectl "https://dl.k8s.io/release/${K8S_VERSION}/bin/linux/amd64/kubectl" \
    && chmod 755 $HOME/bin/kubectl

# Install `k3d`
RUN scurl https://raw.githubusercontent.com/rancher/k3d/main/install.sh \
    | USE_SUDO=false K3D_INSTALL_DIR=$HOME/bin bash

RUN rustup component add clippy rls rust-src rustfmt

# Install cargo-deny
ARG CARGO_DENY_VERSION=0.11.4
RUN scurl "https://github.com/EmbarkStudios/cargo-deny/releases/download/${CARGO_DENY_VERSION}/cargo-deny-${CARGO_DENY_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar zvxf - --strip-components=1 -C $HOME/bin "cargo-deny-${CARGO_DENY_VERSION}-x86_64-unknown-linux-musl/cargo-deny"

# Install cargo-tarpaulin
ARG CARGO_TARPAULIN_VERSION=0.20.0
RUN scurl "https://github.com/xd009642/tarpaulin/releases/download/${CARGO_TARPAULIN_VERSION}/cargo-tarpaulin-${CARGO_TARPAULIN_VERSION}-travis.tar.gz" \
    | tar xzvf - -C $HOME/bin

ARG JUST_VERSION=1.1.3
RUN scurl https://github.com/casey/just/releases/download/${JUST_VERSION}/just-${JUST_VERSION}-x86_64-unknown-linux-musl.tar.gz \
    | tar xzvf - -C $HOME/bin

ENTRYPOINT ["/usr/local/share/docker-init.sh"]
CMD ["sleep", "infinity"]
