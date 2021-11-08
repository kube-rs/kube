# Contributing Guide

This document describes the requirements for committing to this repository.

## Developer Certificate of Origin (DCO)

In order to contribute to this project, you must sign each of your commits to
attest that you have the right to contribute that code. This is done with the
`-s`/`--signoff` flag on `git commit`. More information about DCO can be found
[here](https://developercertificate.org/)

## Pull Request Management

All code that is contributed to kube-rs must go through the Pull Request (PR)
process. To contribute a PR, fork this project, create a new branch, make
changes on that branch, and then use GitHub to open a pull request with your
changes.

Every PR must be reviewed by at least one [Maintainer](./maintainers.md) of the project. Once
a PR has been marked "Approved" by a Maintainer (and no other
Maintainer has an open "Rejected" vote), the PR may be merged. While it is fine
for non-maintainers to contribute their own code reviews, those reviews do not
satisfy the above requirement.

## Code of Conduct

This project has adopted the [CNCF Code of
Conduct](https://github.com/cncf/foundation/blob/master/code-of-conduct.md).

## Rust Guidelines

- **Channel**: Code is built and tested using the **stable** channel of Rust, but documented and formatted with **nightly**
- **Formatting**: To format the codebase, run `make fmt`
- **Documentation** To check documentation, run `make doc`
- **Testing**: To run tests, run `make test`.

## Testing

Most tests can be run with `cargo test --all`, but because of features, some tests must be run a little more precisely.
For the complete variations see the `make test` target in the `Makefile`.

Some tests and examples require an accessible kubernetes cluster via a `KUBECONFIG` environment variable.

- unit tests marked as `#[ignore]` run via `cargo test --all --lib -- --ignored`
- examples run with `cargo run --example=...`
- [integration tests](https://github.com/kube-rs/kube-rs/tree/master/integration)

The easiest way set up a minimal kubernetes cluster for these is with [`k3d`](https://k3d.io/).

## Support
### Documentation
The [high-level architecture document](./architecture.md) is written for contributors.

### Contact
You can ask general questions / share ideas / query the community at the [kube-rs discussions forum](https://github.com/kube-rs/kube-rs/discussions).
You can reach the maintainers of this project at [#kube](https://discord.gg/tokio) channel on the Tokio discord.
