# Kube-rs Governance

This document defines project governance for Kube-rs.

## Contributors

Kube-rs is for everyone. Anyone can become a Kube-rs contributor simply by contributing to the project, whether through code, documentation, blog posts, community management, or other means.
As with all Kube-rs community members, contributors are expected to follow the [Kube-rs Code of Conduct][coc].

All contributions to Kube-rs code, documentation, or other components in the Kube-rs GitHub org must follow the guidelines in [CONTRIBUTING.md][contrib].
Whether these contributions are merged into the project is the prerogative of the maintainers.

## Maintainer Expectations

Maintainers have the ability to merge code into the project. Anyone can become a Kube-rs maintainer (see "Becoming a maintainer" below.)

As such, there are certain expectations for maintainers. Kube-rs maintainers are expected to:

* Review pull requests, triage issues, and fix bugs in their areas of expertise, ensuring that all changes go through the project's code review and integration processes.
* Monitor the Kube-rs Discord, and Discussions and help out when possible.
* Rapidly respond to any time-sensitive security release processes.
* Participate on discussions on the roadmap.

If a maintainer is no longer interested in or cannot perform the duties listed above, they should move themselves to emeritus status.
If necessary, this can also occur through the decision-making process outlined below.

### Maintainer decision-making

Ideally, all project decisions are resolved by maintainer consensus.
If this is not possible, maintainers may call a vote.
The voting process is a simple majority in which each maintainer receives one vote.

### Special Tasks

In addition to the outlined abilities and responsibilities outlined above, some maintainer take on additional tasks and responsibilities.

#### Release Tasks

As a maintainer on the release team, you are expected to:

* Cut releases, and update the [CHANGELOG](./CHANGELOG.md)
* Pre-verify big releases against example repos
* Publish and update versions in example repos
* Verify the release

### Becoming a maintainer

Anyone can become a Kube-rs maintainer. Maintainers should be highly proficient in Rust; have relevant domain expertise; have the time and ability to meet the maintainer expectations above; and demonstrate the ability to work with the existing maintainers and project processes.

To become a maintainer, start by expressing interest to existing maintainers.
Existing maintainers will then ask you to demonstrate the qualifications above by contributing PRs, doing code reviews, and other such tasks under their guidance.
After several months of working together, maintainers will decide whether to grant maintainer status.

[coc]: https://github.com/kube-rs/kube-rs/blob/master/code-of-conduct.md
[contrib]: https://github.com/kube-rs/kube-rs/blob/master/CONTRIBUTING.md
