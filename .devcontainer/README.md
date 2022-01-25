# `kube-rs` Development container

This directory provides a [_devcontainer_][dc] configuration that configures a
reproducible development environment for this project. This base image should
contain only the bare necessities to get up and running with `kube-rs`.
Customizations should be made in per-user configuration.

## Usage

Install the VS Code [Remote Development extension pack][remote-exts] after which
VS Code should build (first use) then run the container in the background.

## Docker

This configuration currently uses the parent host's Docker daemon (rather than
running a separate docker daemon within in the container). It creates
devcontainers on the host network so it's easy to use k3d clusters hosted in the
parent host's docker daemon.

## Personalizing

This configuration is supposed to provide a minimal setup without catering to
any one developer's personal tastes. Devcontainers can be extended with per-user
configuration.

To add your own extensions to the devcontainer, configure default extensions in
your VS Code settings:

```jsonc
    "remote.containers.defaultExtensions": [
        "eamodio.gitlens",
        "GitHub.copilot",
        "GitHub.vscode-pull-request-github",
        "mutantdino.resourcemonitor",
    ],
```

Furthermore, you can configure a [_dotfiles_ repository][df] to perform
customizations with a VS Code setting like:

```jsonc
    "dotfiles.repository": "https://github.com/olix0r/dotfiles.git",
```

[dc]: https://code.visualstudio.com/docs/remote/containers
[df]: https://dotfiles.github.io/
[remote-exts]: https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.vscode-remote-extensionpack
