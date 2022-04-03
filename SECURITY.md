# Security Policy

## Supported Versions

We provide security updates for the two most recent minor versions released on `crates.io`.

For example, if `0.70.1` is the most recent stable version, we will address security updates for `0.69` and later.
Once `0.71.1` is released, we will no longer provide updates for `0.69` releases.

## Reporting a Vulnerability

To report a security problem in Kube-rs, please contact at least two [maintainers](https://kube.rs/maintainers/)

These people will help diagnose the severity of the issue and determine how to address the issue.
Issues deemed to be non-critical will be filed as GitHub issues.
Critical issues will receive immediate attention and be fixed as quickly as possible.

## Security Advisories

When serious security problems in Kube-rs are discovered and corrected, we issue a security advisory, describing the problem and containing a pointer to the fix.

These are announced the [RustSec Advisory Database](https://github.com/rustsec/advisory-db), to our github issues under the label `critical`, as well as discord and other primary communication channels.

Security issues are fixed as soon as possible, and the fixes are propagated to the stable branches as fast as possible. However, when a vulnerability is found during a code audit, or when several other issues are likely to be spotted and fixed in the near future, the security team may delay the release of a Security Advisory, so that one unique, comprehensive Security Advisory covering several vulnerabilities can be issued.
Communication with vendors and other distributions shipping the same code may also cause these delays.
