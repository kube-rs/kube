# Contributing Guide

This document describes the requirements for committing to this repository.

## Developer Certificate of Origin (DCO)

In order to contribute to this project, you must sign each of your commits to attest that you have the right to contribute that code.
This is done with the `-s`/`--signoff` flag on `git commit`.
More information about `DCO` can be found [here](https://developercertificate.org/)

## Pull Request Management

All code that is contributed to kube-rs must go through the Pull Request (PR) process.
To contribute a PR, fork this project, create a new branch, make changes on that branch, and then use GitHub to open a pull request with your changes.

Every PR must be reviewed by at least one [Maintainer](https://kube.rs/maintainers/) of the project.
Once a PR has been marked "Approved" by a Maintainer (and no other Maintainer has an open "Rejected" vote), the PR may be merged.
While it is fine for non-maintainers to contribute their own code reviews, those reviews do not satisfy the above requirement.

## Code of Conduct

This project has adopted the [CNCF Code of
Conduct](https://github.com/cncf/foundation/blob/master/code-of-conduct.md).

## Rust Guidelines

- **Channel**: Code is built and tested using the **stable** channel of Rust, but documented and formatted with **nightly** <sup>[*](https://github.com/kube-rs/kube/issues/707)</sup>
- **Formatting**: To format the codebase, run `just fmt`
- **Documentation** To check documentation, run `just doc`
- **Testing**: To run tests, run `just test` and see below.

For a list of tooling that we glue together everything see [TOOLS.md](https://kube.rs/tools/).

## Testing

We have 3 classes of tests.

- Unit tests & Documentation Tests
- Integration tests (requires Kubernetes)
- End to End tests (requires Kubernetes)

The last two will try to access the Kubernetes cluster that is your `current-context`; i.e. via your local `KUBECONFIG` evar or `~/.kube/config` file.

The easiest way set up a minimal Kubernetes cluster for these is with [`k3d`](https://k3d.io/) (`just k3d`).

### Unit Tests & Documentation Tests

**Most** unit/doc tests are run from `cargo test --lib --doc --all`, but because of feature-sets, and examples, you will need a couple of extra invocations to replicate our CI.

For the complete variations, run the `just test` target in the `justfile`.

All public interfaces must be documented, and most should have minor documentation examples to show usage.

### Integration Tests

Slower set of tests within the crates marked with an **`#[ignore]`** attribute.

:warning: These  **WILL** try to modify resources in your current cluster :warning:

Most integration tests are run with `cargo test --all --lib -- --ignored`, but because of feature-sets, you will need a few invocations of these to replicate our CI. See `just test-integration`

### End to End Tests

We have a small set of [e2e tests](https://github.com/kube-rs/kube/tree/main/e2e) that tests difference between in-cluster and local configuration.

These tests are the heaviest tests we have because they require a full `docker build`, image import (or push/pull flow), yaml construction, and `kubectl` usage to verify that the outcome was sufficient.

To run E2E tests, use (or follow) `just e2e` as appropriate.

### Test Guidelines

#### When to add a test

All public interfaces should have doc tests with examples for [docs.rs](https://docs.rs/kube).

When adding new non-trivial pieces of logic that results in a drop in coverage you should add a test.

Cross-reference with the coverage build [![coverage build](https://codecov.io/gh/kube-rs/kube/branch/main/graph/badge.svg?token=9FCqEcyDTZ)](https://codecov.io/gh/kube-rs/kube) and go to your branch. Coverage can also be run locally with [`cargo tarpaulin`](https://github.com/xd009642/tarpaulin) at project root. This will use our [tarpaulin.toml](https://github.com/kube-rs/kube/blob/main/tarpaulin.toml) config, and **will run both unit and integration** tests.

#### What type of test

- Unit tests **MUST NOT** try to contact a Kubernetes cluster
- Doc tests **MUST** be marked as `no_run` when they need to contact a Kubernetes cluster
- Integration tests **MUST NOT** be used when a unit test is sufficient
- Integration tests **MUST NOT** assume existence of non-standard objects in the cluster
- Integration tests **MUST NOT** cross-depend on other unit tests completing (and installing what you need)
- E2E tests **MUST NOT** be used where an integration test is sufficient

In general: **use the least powerful method** of testing available to you:

- use unit tests in `kube-core`
- use unit tests in `kube-client` (and in rare cases integration tests)
- use unit tests in `kube-runtime` (and occassionally integration tests)
- use e2e tests when testing differences between in-cluster and local configuration

## Support
### Documentation
The [high-level architecture document](https://kube.rs/architecture/) is written for contributors.

### Contact
You can ask general questions / share ideas / query the community at the [kube-rs discussions forum](https://github.com/kube-rs/kube/discussions).
You can reach the maintainers of this project at [#kube](https://discord.gg/tokio) channel on the Tokio discord.
