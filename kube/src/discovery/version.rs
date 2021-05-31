//! Version definitions to allow sorting (not exported)
//!
//! Follows [kubernetes version priority](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definition-versioning/#version-priority)
//! but only in sort-order at the moment.
//!
//! Changes to Ord is needed if this is ever exported outside the crate.

use std::cmp::Reverse;

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum Version {
    Stable(u32),
    Beta(u32, Option<u32>),
    Alpha(u32, Option<u32>),
    // CRDs and APIServices can use arbitrary strings as versions.
    Nonconformant(String),
}

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

    pub(crate) fn parse(v: &str) -> Version {
        match Self::try_parse(v) {
            Some(ver) => ver,
            None => Version::Nonconformant(v.to_string()),
        }
    }
}
// A key used to allow sorting Versions
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum VersionSortKey<'a> {
    Stable(Reverse<u32>),
    Beta(Reverse<u32>, Reverse<Option<u32>>),
    Alpha(Reverse<u32>, Reverse<Option<u32>>),
    Nonconformant(&'a str),
}
impl Version {
    fn to_sort_key(&self) -> VersionSortKey {
        match self {
            Version::Stable(v) => VersionSortKey::Stable(Reverse(*v)),
            Version::Beta(v, beta) => VersionSortKey::Beta(Reverse(*v), Reverse(*beta)),
            Version::Alpha(v, alpha) => VersionSortKey::Alpha(Reverse(*v), Reverse(*alpha)),
            Version::Nonconformant(nc) => VersionSortKey::Nonconformant(nc),
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_sort_key().cmp(&other.to_sort_key())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

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
    fn test_version_ord() {
        // NB: semantically this is exact opposite of what makes sense, but we never export Version.
        // We can reverse the Ord, but would have to remember to .reverse() again after sorts.
        assert!(Version::Stable(2) < Version::Stable(1));
        assert!(Version::Stable(2) < Version::Stable(1));
        assert!(Version::Stable(1) < Version::Beta(1, None));
        assert!(Version::Stable(1) < Version::Beta(2, None));
        assert!(Version::Stable(2) < Version::Alpha(1, Some(2)));
        assert!(Version::Stable(1) < Version::Alpha(2, Some(2)));
        assert!(Version::Beta(1, None) < Version::Nonconformant("hi".into()));

        // The sorting results is what we export, and this works by default because Ord is reversed:
        let mut vers = vec![
            Version::Beta(2, Some(2)),
            Version::Stable(1),
            Version::Nonconformant("hi".into()),
            Version::Alpha(1, Some(2)),
            Version::Stable(2),
            Version::Beta(2, Some(3)),
        ];
        vers.sort();
        assert_eq!(vers, vec![
            Version::Stable(2),
            Version::Stable(1),
            Version::Beta(2, Some(3)),
            Version::Beta(2, Some(2)),
            Version::Alpha(1, Some(2)),
            Version::Nonconformant("hi".into()),
        ]);
    }
}
