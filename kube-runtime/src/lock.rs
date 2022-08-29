//! Leader election support.
//! # Primitives
//! TODO.
//! # Requirements and guarantees
//! In general, one may talk about two properties of distributed locking protocol:
//! 1. Safety (should not be confused with safety in rust). Protocol is safe if it provides
//! strict "at most once guarantee", i.e. situation when several instances execute critical section is impossible.
//! 2. Liveness. Protocol has liveness if it tolerates partial failures without human intervention.
//!
//! It is impossible for generic locking protocol to have both propertise,
//! and `kube`-supplied locks implemented in this module choose liveness over safety.
//! Therefore your code must tolerate situation when critical section is entered concurrently.
//!
//! To minimize chances that this situation will occur:
//! - Use distinct `identity`-es for different instances.
//! - Stop your leader-only components right after being notified about lost leadership, in timely fashion.
//! - Do not violate runtime compatibility policy (documented below)
//! - Do not delete or edit `Lease` object used by locks internally.
//! - Ensure that cluster has no clock skew (i.e. at any moment difference between time on any two nodes is negligible compared
//! to expiration timeout).
//! - Ensure that task which renews lock does not starve (if it does not get executed, it won't have chance to observe that lock has expired).
//! # Runtime compatiblity policy
//! Sometimes underlying protocol may be changed in such a way that actors compiled with newer kube version do not understand actors compiled with older kube version.
//! This may lead to various nasty situations (such as lock being constantly stolen, multiple leaders elected at once, and so on). However, kube guarantees that
//! 1. This may not happen if all actors share one or two consecutive `kube` minor versions. (E.g. if all your instances use `v0.378.0`, `v0.278.3` or `v0.379.1`, this is supported version skew)
//! 2. Each time new kube minor release drops compatibility with some older releases, it is reflected in release notes as a breaking change.
pub mod raw;
