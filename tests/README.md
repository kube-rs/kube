# integration tests

This is a working example of a kubernetes application `dapp`, that is deployed on CI during circle's `kind_k8s` and `kind_compile` job. See [this part of the .circleci/config](https://github.com/kube-rs/kube-rs/blob/9837d60246a0528f8d810243fe544014d1e51dca/.circleci/config.yml#L32-L84).

## Behavior
The app currently only does what the `job_api` example does, but from within the cluster, so it needs the rbac permissions to `create` a `job` in `batch`.

## CircleCI
It's a slightly complicated process to do this on CI due to cache boundaries and switching between machine and docker executors on circleci, so this explains the process:

### kind_compile
The first job compiles the app with `cargo build -p tests --release` using [muslrust](https://github.com/clux/muslrust) for the musl cross compile.

The static binary is then persisted to circleci's workspace.

### kind_k8s
This resumes with a machine excutor (which can run `docker` commands).

It carries on building the tiny image with the mounted binary from the workspace into a [distroless:static](https://github.com/GoogleContainerTools/distroless) image.

This is then pushed to [dockerhub/clux/kube-dapp](https://hub.docker.com/repository/docker/clux/kube-dapp/tags) (if we have creds, wont work on pr builds), otherwise the image is loaded into [kind](https://kind.sigs.k8s.io/).

We install `kind` using the direct binary install from their [github releases](https://github.com/kubernetes-sigs/kind/releases), and use the [circleci kubernetes orb](https://circleci.com/orbs/registry/orb/circleci/kubernetes) to apply our [test yaml](./deployment.yaml).

It's successful if the app exits successfully, without encountering errors.

## Locally
Start a cluster first. Say, via `make minikube-create && make minikube` or `make kind-create && make kind`.

### Building Yourself
Run `make integration-test` to cross compile `dapp` with `muslrust` locally using the same docker image, and then deploy it to the current active cluster.

### Using CI built image
Switch to the same git sha used on CI, and use `make integration-pull`.
