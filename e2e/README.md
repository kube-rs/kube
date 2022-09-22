# E2E tests

Small set of tests complete e2e flows, like local vs. in-cluster configs, incluster namespace defaulting, and non-standard feature flags that are hard to test within kube.

**[You probably do not want to make an E2E test](../CONTRIBUTING.md#test-guidelines)**.

## boot

Simple executable that lists pods.

Intended as a compilation target to ensure kube builds with any k8s-openapi version feature selection greater than or equal to our MK8SV.

## job

A more advanced application that is containerised and deployed into a cluster on CI during the `e2e` job via [our ci workflow](https://github.com/kube-rs/kube/blob/2b5e4ad788366125448ad40eadaf68cf9ceeaf31/.github/workflows/ci.yml#L58-L107).

Functionally equivalent to the `job_api` example. Creates a noop job, waits for it to complete, then deletes it.

Intended as a safety mechanism to ensure in-cluster authenication is working, not hanging, and its minimal work is is verifiable out-of-band.

## Testing Strategy

### job

Compile the `job` binary (via [muslrust](https://github.com/clux/muslrust)) and put the it into a [distroless:static](https://github.com/GoogleContainerTools/distroless) image.

Then, import the image into `k3d` (to avoid pushing), and apply the [test yaml](./deployment.yaml). We can observe the job completes.

Running these locally requires a local cluster. Use `just k3d` to start a simple one.

Then, run `just e2e-incluster openssl,latest` or `just e2e-incluster rustls,latest`.

### boot

Build the `boot` bin against various `k8s-openapi` version features, and check that it runs. Uses local auth; not dockerised.

To run this with all feature combinations combinations, run `just e2e-mink8s`.
