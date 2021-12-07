# E2E tests

Small set of tests to verify differences between local and in-cluster development.

**[You probably do not want to make a new E2E test](../CONTRIBUTING.md#test-guidelines)**.

## dapp

A working example of a kubernetes application `dapp` deployed on CI during the `e2e` job via [our ci workflow](https://github.com/kube-rs/kube-rs/blob/2b5e4ad788366125448ad40eadaf68cf9ceeaf31/.github/workflows/ci.yml#L58-L107). It is here to ensure in-cluster configuration is working.

### Behavior
The app currently only does what the `job_api` example does, but from within the cluster, so it needs the rbac permissions to `create` a `job` in `batch`.

### Github Actions
General process, optimized for time.

- compile the image with [muslrust](https://github.com/clux/muslrust)
- put the static binary into a [distroless:static](https://github.com/GoogleContainerTools/distroless) image
- import the image into `k3d` (to avoid pushing)
- `kubectl apply` the [test yaml](./deployment.yaml)
- wait for the job to complete for up to 60s

It's successful if the app exits successfully, without encountering errors.

### Running
Start a cluster first, e.g. `make k3d`.

Run `make integration` to cross compile `dapp` with `muslrust` locally using the same docker image, and then deploy it to the current active cluster.
