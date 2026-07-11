//! Deterministic entity IDs (mazzap convention): a physical thing at
//! the same position keeps its ID across rebuilds, so re-importing an
//! unchanged capture is a no-op against the store.
//!
//! IDs hash the *source* and the position rounded to 0.1 m, so jitter
//! below sensor precision does not mint new identities. Identity is
//! positional by design: a feature that genuinely moves becomes a new
//! entity and the old one is retired.

/// FNV-1a 64-bit — tiny, dependency-free, and stable across platforms
/// and Rust versions (unlike `DefaultHasher`).
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Deterministic ID for a positional entity:
/// `"<kind>:" + hex12(fnv1a("{source}|{x:.1}|{y:.1}"))`.
pub fn positional_entity_id(kind: &str, source: &str, x: f64, y: f64) -> String {
    let key = format!("{source}|{x:.1}|{y:.1}");
    format!("{kind}:{:012x}", fnv1a(key.as_bytes()) & 0xffff_ffff_ffff)
}

/// Natural-key ID for named entities (zones, devices):
/// `"<kind>:<slug>"` with the slug lowercased and non-alphanumerics
/// collapsed to single underscores.
pub fn named_entity_id(kind: &str, name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_sep = true;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            last_sep = false;
        } else if !last_sep {
            slug.push('_');
            last_sep = true;
        }
    }
    let slug = slug.trim_end_matches('_');
    format!("{kind}:{slug}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_ids_are_deterministic_and_jitter_stable() {
        let a = positional_entity_id("feature", "walkthrough-1", 12.34, -5.67);
        let b = positional_entity_id("feature", "walkthrough-1", 12.31, -5.72);
        let c = positional_entity_id("feature", "walkthrough-1", 13.00, -5.67);
        assert_eq!(a, b, "sub-0.05m jitter keeps identity");
        assert_ne!(a, c, "a real move is a new identity");
        assert!(a.starts_with("feature:"));
    }

    #[test]
    fn named_ids_slugify() {
        assert_eq!(named_entity_id("zone", "Back Porch"), "zone:back_porch");
        assert_eq!(named_entity_id("zone", "  Fire—Pit! "), "zone:fire_pit");
    }
}
