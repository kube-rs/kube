use std::{cmp::Reverse, convert::Infallible, str::FromStr};

/// Version parser for Kubernetes version patterns
///
/// Implements the [Kubernetes version priority order](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definition-versioning/#version-priority)
/// to allow getting the served api versions sorted by `kubectl` priority:
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
/// versions.sort_by_cached_key(|v| Reverse(Version::parse(v).latest_stable()));
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
    /// An non-conformant api string (sorted alphabetically)
    ///
    /// CRDs and APIServices can use arbitrary strings as versions.
    Nonconformant(String),
}
// NB:

impl Version {
    fn try_parse(v: &str) -> Option<Version> {
        let v = v.strip_prefix('v')?;
        let major_chars = v.chars().take_while(|ch| ch.is_ascii_digit()).count();
        let major = &v[..major_chars];
        let major: u32 = major.parse().ok()?;
        let v = &v[major_chars..];
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

/// See [`Version::latest_stable`]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct LatestStable {
    stability: Stability,
    major: u32,
    minor: Option<u32>,
    nonconformant: Option<Reverse<String>>,
}

impl Version {
    /// An [`Ord`] for `Version` that prefers stable versions over later prereleases
    ///
    /// Implements the [Kubernetes version priority order](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definition-versioning/#version-priority)
    ///
    /// For example:
    ///
    /// ```
    /// # use kube_core::Version;
    /// assert!(Version::Stable(2).latest_stable() > Version::Stable(1).latest_stable());
    /// assert!(Version::Stable(1).latest_stable() > Version::Beta(1, None).latest_stable());
    /// assert!(Version::Stable(1).latest_stable() > Version::Beta(2, None).latest_stable());
    /// assert!(Version::Stable(2).latest_stable() > Version::Alpha(1, Some(2)).latest_stable());
    /// assert!(Version::Stable(1).latest_stable() > Version::Alpha(2, Some(2)).latest_stable());
    /// assert!(Version::Beta(1, None).latest_stable() > Version::Nonconformant("ver3".into()).latest_stable());
    /// ```
    ///
    /// Note that the type of release matters more than the version numbers:
    /// `Stable(x)` > `Beta(y)` > `Alpha(z)` > `Nonconformant(w)` for all `x`,`y`,`z`,`w`
    pub fn latest_stable(&self) -> impl Ord {
        match self {
            &Version::Stable(major) => LatestStable {
                stability: Stability::Stable,
                major,
                minor: None,
                nonconformant: None,
            },
            &Version::Beta(major, minor) => LatestStable {
                stability: Stability::Beta,
                major,
                minor,
                nonconformant: None,
            },
            &Self::Alpha(major, minor) => LatestStable {
                stability: Stability::Alpha,
                major,
                minor,
                nonconformant: None,
            },
            Self::Nonconformant(nonconformant) => LatestStable {
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
    fn test_version_latest_stable_ord() {
        // sorting makes sense from a "greater than" semantic perspective:
        assert!(Version::Stable(2).latest_stable() > Version::Stable(1).latest_stable());
        assert!(Version::Stable(1).latest_stable() > Version::Beta(1, None).latest_stable());
        assert!(Version::Stable(1).latest_stable() > Version::Beta(2, None).latest_stable());
        assert!(Version::Stable(2).latest_stable() > Version::Alpha(1, Some(2)).latest_stable());
        assert!(Version::Stable(1).latest_stable() > Version::Alpha(2, Some(2)).latest_stable());
        assert!(
            Version::Beta(1, None).latest_stable() > Version::Nonconformant("ver3".into()).latest_stable()
        );

        assert!(Version::Stable(2).latest_stable() > Version::Stable(1).latest_stable());
        assert!(Version::Stable(1).latest_stable() > Version::Beta(2, None).latest_stable());
        assert!(Version::Stable(1).latest_stable() > Version::Beta(2, Some(2)).latest_stable());
        assert!(Version::Stable(1).latest_stable() > Version::Alpha(2, None).latest_stable());
        assert!(Version::Stable(1).latest_stable() > Version::Alpha(2, Some(3)).latest_stable());
        assert!(
            Version::Stable(1).latest_stable() > Version::Nonconformant("foo".to_string()).latest_stable()
        );
        assert!(Version::Beta(1, Some(1)).latest_stable() > Version::Beta(1, None).latest_stable());
        assert!(Version::Beta(1, Some(2)).latest_stable() > Version::Beta(1, Some(1)).latest_stable());
        assert!(Version::Beta(1, None).latest_stable() > Version::Alpha(1, None).latest_stable());
        assert!(Version::Beta(1, None).latest_stable() > Version::Alpha(1, Some(3)).latest_stable());
        assert!(
            Version::Beta(1, None).latest_stable()
                > Version::Nonconformant("foo".to_string()).latest_stable()
        );
        assert!(
            Version::Beta(1, Some(2)).latest_stable()
                > Version::Nonconformant("foo".to_string()).latest_stable()
        );
        assert!(Version::Alpha(1, Some(1)).latest_stable() > Version::Alpha(1, None).latest_stable());
        assert!(Version::Alpha(1, Some(2)).latest_stable() > Version::Alpha(1, Some(1)).latest_stable());
        assert!(
            Version::Alpha(1, None).latest_stable()
                > Version::Nonconformant("foo".to_string()).latest_stable()
        );
        assert!(
            Version::Alpha(1, Some(2)).latest_stable()
                > Version::Nonconformant("foo".to_string()).latest_stable()
        );
        assert!(
            Version::Nonconformant("bar".to_string()).latest_stable()
                > Version::Nonconformant("foo".to_string()).latest_stable()
        );
        assert!(
            Version::Nonconformant("foo1".to_string()).latest_stable()
                > Version::Nonconformant("foo10".to_string()).latest_stable()
        );

        // sort order by default is ascending
        // sorting with std::cmp::Reverse thus gives you the "most latest stable" first
        let mut vers = vec![
            Version::Beta(2, Some(2)),
            Version::Stable(1),
            Version::Nonconformant("hi".into()),
            Version::Alpha(3, Some(2)),
            Version::Stable(2),
            Version::Beta(2, Some(3)),
        ];
        vers.sort_by_cached_key(|x| Reverse(x.latest_stable()));
        assert_eq!(vers, vec![
            Version::Stable(2),
            Version::Stable(1),
            Version::Beta(2, Some(3)),
            Version::Beta(2, Some(2)),
            Version::Alpha(3, Some(2)),
            Version::Nonconformant("hi".into()),
        ]);
    }
}
