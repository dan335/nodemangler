//! Comparison of NodeMangler version strings (`X.Y.Z`), used to detect when a
//! saved graph was written by a version of NodeMangler newer than the one
//! currently running (see [`crate::GraphSaveData::version`] /
//! [`crate::APP_VERSION`]).
//!
//! Hand-rolled rather than pulling in the `semver` crate: NodeMangler's
//! version scheme is always a plain three-part `major.minor.patch` with no
//! pre-release/build-metadata suffixes, so a tiny parser is enough.

/// Parse a strict `X.Y.Z` version string into its numeric components.
///
/// Returns `None` for anything that isn't exactly three dot-separated
/// unsigned integers — garbage text, the empty string, a two-part `"1.0"`,
/// or a four-part `"1.0.0.1"`. The empty-string case in particular covers
/// graphs saved before the `version` field existed (see
/// `GraphSaveData::version`'s `#[serde(default)]`).
pub fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    // Reject any trailing segment ("1.0.0.1") — must be exactly three parts.
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Returns `true` if `file_version` parses as a version strictly newer than
/// this build's [`crate::APP_VERSION`].
///
/// An unparseable `file_version` — including the empty string used by
/// pre-versioning saves — is treated as old/unknown, never newer, so old or
/// malformed version stamps never spuriously trigger the "newer version"
/// warning. Comparison is numeric per component (`"1.0.10" > "1.0.9"`), not
/// lexicographic (tuple `Ord` compares major, then minor, then patch).
pub fn is_newer_than_app(file_version: &str) -> bool {
    let Some(file) = parse_version(file_version) else {
        return false;
    };
    let Some(app) = parse_version(crate::APP_VERSION) else {
        // APP_VERSION is derived from Cargo.toml at compile time and should
        // always parse; if it somehow doesn't, there's nothing sane to
        // compare against, so don't warn.
        return false;
    };
    file > app
}

#[cfg(test)]
#[path = "version_tests.rs"]
mod tests;
