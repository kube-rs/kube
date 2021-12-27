use std::{cmp::Reverse, convert::Infallible, str::FromStr};

/// Version parser for Kubernetes version patterns
///
/// This type implements two orderings for sorting by:
///
/// - [`Version::priority`] for [Kubernetes/kubectl version priority](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definition-versioning/#version-priority)
/// - [`Version::generation`] for sorting strictly by version generation in a semver style
///
/// To get the api versions sorted by `kubectl` priority:
///
/// ```
/// use kube_core::Version;
/// use std::cmp::Reverse; // for DESCENDING sort
/// let mut versions = vec![
///     "v10beta3",
///     "v2",
///     "foo10",
///     "v1",
///     "v3beta1",
///     "v11alpha2",
///     "v11beta2",
///     "v12alpha1",
///     "foo1",
///     "v10",
/// ];
/// versions.sort_by_cached_key(|v| Reverse(Version::parse(v).priority()));
/// assert_eq!(versions, vec![
///     "v10",
///     "v2",
///     "v1",
///     "v11beta2",
///     "v10beta3",
///     "v3beta1",
///     "v12alpha1",
///     "v11alpha2",
///     "foo1",
///     "foo10",
/// ]);
/// ```
///
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Version {
    /// A major/GA release
    ///
    /// Always considered higher priority than a beta release.
    Stable(u32),

    /// A beta release for a specific major version
    ///
    /// Always considered higher priority than an alpha release.
    Beta(u32, Option<u32>),

    /// An alpha release for a specific major version
    ///
    /// Always considered higher priority than a nonconformant version
    Alpha(u32, Option<u32>),
    /// An non-conformant api string
    ///
    /// CRDs and APIServices can use arbitrary strings as versions.
    Nonconformant(String),
}

impl Version {
    fn try_parse(v: &str) -> Option<Version> {
        let v = v.strip_prefix('v')?;
        let major = v.split_terminator(|ch: char| !ch.is_ascii_digit()).next()?;
        let v = &v[major.len()..];
        let major: u32 = major.parse().ok()?;
        if v.is_empty() {
            return Some(Version::Stable(major));
        }
        if let Some(suf) = v.strip_prefix("alpha") {
            return if suf.is_empty() {
                Some(Version::Alpha(major, None))
            } else {
                Some(Version::Alpha(major, Some(suf.parse().ok()?)))
            };
        }
        if let Some(suf) = v.strip_prefix("beta") {
            return if suf.is_empty() {
                Some(Version::Beta(major, None))
            } else {
                Some(Version::Beta(major, Some(suf.parse().ok()?)))
            };
        }
        None
    }

    /// An infallble parse of a Kubernetes version string
    ///
    /// ```
    /// use kube_core::Version;
    /// assert_eq!(Version::parse("v10beta12"), Version::Beta(10, Some(12)));
    /// ```
    pub fn parse(v: &str) -> Version {
        match Self::try_parse(v) {
            Some(ver) => ver,
            None => Version::Nonconformant(v.to_string()),
        }
    }
}

/// An infallible FromStr implementation for more generic users
impl FromStr for Version {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Version::parse(s))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Stability {
    Nonconformant,
    Alpha,
    Beta,
    Stable,
}

/// See [`Version::priority`]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Priority {
    stability: Stability,
    major: u32,
    minor: Option<u32>,
    nonconformant: Option<Reverse<String>>,
}

/// See [`Version::generation`]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Generation {
    major: u32,
    stability: Stability,
    minor: Option<u32>,
    nonconformant: Option<Reverse<String>>,
}

impl Version {
    /// An [`Ord`] for `Version` that orders by [Kubernetes version priority](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definition-versioning/#version-priority)
    ///
    /// This order will favour stable versions over newer pre-releases and is used by `kubectl`.
    ///
    /// For example:
    ///
    /// ```
    /// # use kube_core::Version;
    /// assert!(Version::Stable(2).priority() > Version::Stable(1).priority());
    /// assert!(Version::Stable(1).priority() > Version::Beta(1, None).priority());
    /// assert!(Version::Stable(1).priority() > Version::Beta(2, None).priority());
    /// assert!(Version::Stable(2).priority() > Version::Alpha(1, Some(2)).priority());
    /// assert!(Version::Stable(1).priority() > Version::Alpha(2, Some(2)).priority());
    /// assert!(Version::Beta(1, None).priority() > Version::Nonconformant("ver3".into()).priority());
    /// ```
    ///
    /// Note that the type of release matters more than the version numbers:
    /// `Stable(x)` > `Beta(y)` > `Alpha(z)` > `Nonconformant(w)` for all `x`,`y`,`z`,`w`
    ///
    /// `Nonconformant` versions are ordered alphabetically.
    pub fn priority(&self) -> impl Ord {
        match self {
            &Self::Stable(major) => Priority {
                stability: Stability::Stable,
                major,
                minor: None,
                nonconformant: None,
            },
            &Self::Beta(major, minor) => Priority {
                stability: Stability::Beta,
                major,
                minor,
                nonconformant: None,
            },
            &Self::Alpha(major, minor) => Priority {
                stability: Stability::Alpha,
                major,
                minor,
                nonconformant: None,
            },
            Self::Nonconformant(nonconformant) => Priority {
                stability: Stability::Nonconformant,
                major: 0,
                minor: None,
                nonconformant: Some(Reverse(nonconformant.clone())),
            },
        }
    }

    /// An [`Ord`] for `Version` that orders by version generation
    ///
    /// This order will favour higher version numbers even if it's a pre-release.
    ///
    /// For example:
    ///
    /// ```
    /// # use kube_core::Version;
    /// assert!(Version::Stable(2).generation() > Version::Stable(1).generation());
    /// assert!(Version::Stable(1).generation() > Version::Beta(1, None).generation());
    /// assert!(Version::Beta(2, None).generation() > Version::Stable(1).generation());
    /// assert!(Version::Stable(2).generation() > Version::Alpha(1, Some(2)).generation());
    /// assert!(Version::Alpha(2, Some(2)).generation() > Version::Stable(1).generation());
    /// assert!(Version::Beta(1, None).generation() > Version::Nonconformant("ver3".into()).generation());
    /// ```
    pub fn generation(&self) -> impl Ord {
        match self {
            &Self::Stable(major) => Generation {
                stability: Stability::Stable,
                major,
                minor: None,
                nonconformant: None,
            },
            &Self::Beta(major, minor) => Generation {
                stability: Stability::Beta,
                major,
                minor,
                nonconformant: None,
            },
            &Self::Alpha(major, minor) => Generation {
                stability: Stability::Alpha,
                major,
                minor,
                nonconformant: None,
            },
            Self::Nonconformant(nonconformant) => Generation {
                stability: Stability::Nonconformant,
                major: 0,
                minor: None,
                nonconformant: Some(Reverse(nonconformant.clone())),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Version;
    use std::{cmp::Reverse, str::FromStr};

    #[test]
    fn test_stable() {
        assert_eq!(Version::parse("v1"), Version::Stable(1));
        assert_eq!(Version::parse("v3"), Version::Stable(3));
        assert_eq!(Version::parse("v10"), Version::Stable(10));
    }

    #[test]
    fn test_prerelease() {
        assert_eq!(Version::parse("v1beta"), Version::Beta(1, None));
        assert_eq!(Version::parse("v2alpha1"), Version::Alpha(2, Some(1)));
        assert_eq!(Version::parse("v10beta12"), Version::Beta(10, Some(12)));
    }

    fn check_not_parses(s: &str) {
        assert_eq!(Version::parse(s), Version::Nonconformant(s.to_string()))
    }

    #[test]
    fn test_nonconformant() {
        check_not_parses("");
        check_not_parses("foo");
        check_not_parses("v");
        check_not_parses("v-1");
        check_not_parses("valpha");
        check_not_parses("vbeta3");
        check_not_parses("vv1");
        check_not_parses("v1alpha1hi");
        check_not_parses("v1zeta3");
    }

    #[test]
    fn test_version_fromstr() {
        assert_eq!(
            Version::from_str("infallible").unwrap(),
            Version::Nonconformant("infallible".to_string())
        );
    }

    #[test]
    fn test_version_priority_ord() {
        // sorting makes sense from a "greater than" generation perspective:
        assert!(Version::Stable(2).priority() > Version::Stable(1).priority());
        assert!(Version::Stable(1).priority() > Version::Beta(1, None).priority());
        assert!(Version::Stable(1).priority() > Version::Beta(2, None).priority());
        assert!(Version::Stable(2).priority() > Version::Alpha(1, Some(2)).priority());
        assert!(Version::Stable(1).priority() > Version::Alpha(2, Some(2)).priority());
        assert!(Version::Beta(1, None).priority() > Version::Nonconformant("ver3".into()).priority());

        assert!(Version::Stable(2).priority() > Version::Stable(1).priority());
        assert!(Version::Stable(1).priority() > Version::Beta(2, None).priority());
        assert!(Version::Stable(1).priority() > Version::Beta(2, Some(2)).priority());
        assert!(Version::Stable(1).priority() > Version::Alpha(2, None).priority());
        assert!(Version::Stable(1).priority() > Version::Alpha(2, Some(3)).priority());
        assert!(Version::Stable(1).priority() > Version::Nonconformant("foo".to_string()).priority());
        assert!(Version::Beta(1, Some(1)).priority() > Version::Beta(1, None).priority());
        assert!(Version::Beta(1, Some(2)).priority() > Version::Beta(1, Some(1)).priority());
        assert!(Version::Beta(1, None).priority() > Version::Alpha(1, None).priority());
        assert!(Version::Beta(1, None).priority() > Version::Alpha(1, Some(3)).priority());
        assert!(Version::Beta(1, None).priority() > Version::Nonconformant("foo".to_string()).priority());
        assert!(Version::Beta(1, Some(2)).priority() > Version::Nonconformant("foo".to_string()).priority());
        assert!(Version::Alpha(1, Some(1)).priority() > Version::Alpha(1, None).priority());
        assert!(Version::Alpha(1, Some(2)).priority() > Version::Alpha(1, Some(1)).priority());
        assert!(Version::Alpha(1, None).priority() > Version::Nonconformant("foo".to_string()).priority());
        assert!(Version::Alpha(1, Some(2)).priority() > Version::Nonconformant("foo".to_string()).priority());
        assert!(
            Version::Nonconformant("bar".to_string()).priority()
                > Version::Nonconformant("foo".to_string()).priority()
        );
        assert!(
            Version::Nonconformant("foo1".to_string()).priority()
                > Version::Nonconformant("foo10".to_string()).priority()
        );

        // sort orders by default are ascending
        // sorting with std::cmp::Reverse on priority gives you the highest priority first
        let mut vers = vec![
            Version::Beta(2, Some(2)),
            Version::Stable(1),
            Version::Nonconformant("hi".into()),
            Version::Alpha(3, Some(2)),
            Version::Stable(2),
            Version::Beta(2, Some(3)),
        ];
        vers.sort_by_cached_key(|x| Reverse(x.priority()));
        assert_eq!(vers, vec![
            Version::Stable(2),
            Version::Stable(1),
            Version::Beta(2, Some(3)),
            Version::Beta(2, Some(2)),
            Version::Alpha(3, Some(2)),
            Version::Nonconformant("hi".into()),
        ]);
    }

    #[test]
    fn test_version_generation_ord() {
        assert!(Version::Stable(2).generation() > Version::Stable(1).generation());
        assert!(Version::Stable(1).generation() > Version::Beta(1, None).generation());
        assert!(Version::Stable(1).generation() < Version::Beta(2, None).generation());
        assert!(Version::Stable(2).generation() > Version::Alpha(1, Some(2)).generation());
        assert!(Version::Stable(1).generation() < Version::Alpha(2, Some(2)).generation());
        assert!(Version::Beta(1, None).generation() > Version::Nonconformant("ver3".into()).generation());

        assert!(Version::Stable(2).generation() > Version::Stable(1).generation());
        assert!(Version::Stable(1).generation() < Version::Beta(2, None).generation());
        assert!(Version::Stable(1).generation() < Version::Beta(2, Some(2)).generation());
        assert!(Version::Stable(1).generation() < Version::Alpha(2, None).generation());
        assert!(Version::Stable(1).generation() < Version::Alpha(2, Some(3)).generation());
        assert!(Version::Stable(1).generation() > Version::Nonconformant("foo".to_string()).generation());
        assert!(Version::Beta(1, Some(1)).generation() > Version::Beta(1, None).generation());
        assert!(Version::Beta(1, Some(2)).generation() > Version::Beta(1, Some(1)).generation());
        assert!(Version::Beta(1, None).generation() > Version::Alpha(1, None).generation());
        assert!(Version::Beta(1, None).generation() > Version::Alpha(1, Some(3)).generation());
        assert!(Version::Beta(1, None).generation() > Version::Nonconformant("foo".to_string()).generation());
        assert!(
            Version::Beta(1, Some(2)).generation() > Version::Nonconformant("foo".to_string()).generation()
        );
        assert!(Version::Alpha(1, Some(1)).generation() > Version::Alpha(1, None).generation());
        assert!(Version::Alpha(1, Some(2)).generation() > Version::Alpha(1, Some(1)).generation());
        assert!(
            Version::Alpha(1, None).generation() > Version::Nonconformant("foo".to_string()).generation()
        );
        assert!(
            Version::Alpha(1, Some(2)).generation() > Version::Nonconformant("foo".to_string()).generation()
        );
        assert!(
            Version::Nonconformant("bar".to_string()).generation()
                > Version::Nonconformant("foo".to_string()).generation()
        );
        assert!(
            Version::Nonconformant("foo1".to_string()).generation()
                > Version::Nonconformant("foo10".to_string()).generation()
        );

        // sort orders by default is ascending
        // sorting with std::cmp::Reverse on generation gives you the latest generation versions first
        let mut vers = vec![
            Version::Beta(2, Some(2)),
            Version::Stable(1),
            Version::Nonconformant("hi".into()),
            Version::Alpha(3, Some(2)),
            Version::Stable(2),
            Version::Beta(2, Some(3)),
        ];
        vers.sort_by_cached_key(|x| Reverse(x.generation()));
        assert_eq!(vers, vec![
            Version::Alpha(3, Some(2)),
            Version::Stable(2),
            Version::Beta(2, Some(3)),
            Version::Beta(2, Some(2)),
            Version::Stable(1),
            Version::Nonconformant("hi".into()),
        ]);
    }
}
