//! **What a caller is entitled to see** — the policy half of `fm_core::Scoped`.
//!
//! Vaults are audiences (`decisions.md#vault`), and until a client could arrive over a network
//! every caller was the person at the keyboard: entitled to all of them, by construction. A
//! paired tablet is not, so something has to say which audiences it got — and that something
//! belongs here rather than in `fm-core`, which enforces a list without opinions about how the
//! list was decided, or in the transport, which is the wrong layer to reason about audiences.
//!
//! It is deliberately tiny and deliberately *not* a permission system. There are no roles, no
//! verbs, no inheritance: a caller either is a member of an audience or is not, which is the same
//! shape the vault boundary already has on disk. Anything richer would be a second, weaker model
//! of access sitting beside the real one — and a frontmatter `access:` label is exactly the
//! forgeable non-enforcement `MultiStore`'s own doc comment rejects.

/// Which vaults a caller may reach.
///
/// [`Scope::All`] is the default everywhere and the only thing that existed before: `fm-cli`, the
/// phone, the study agent, and any caller on loopback. It is a distinct variant rather than
/// "a list containing everything" on purpose — a list would have to be recomputed whenever a
/// vault is created, and a stale one would silently start denying the machine's own user.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Scope {
    /// Every vault, including ones created after this scope was made.
    #[default]
    All,
    /// These vaults and no others. **Empty grants nothing**, and must never be read as a
    /// wildcard — that inversion is how a device whose vaults were all revoked would silently
    /// gain everything.
    Only(Vec<String>),
}

impl Scope {
    /// The form `fm_core::Scoped` wants: `None` for unrestricted.
    pub fn names(&self) -> Option<&[String]> {
        match self {
            Scope::All => None,
            Scope::Only(v) => Some(v),
        }
    }

    /// Whether this scope reaches `vault`. Used by the routes that are not commands and so
    /// cannot reach the store's own seams — `GET /api/blob` in particular.
    pub fn allows(&self, vault: &str) -> bool {
        match self {
            Scope::All => true,
            Scope::Only(v) => v.iter().any(|n| n == vault),
        }
    }

    /// Whether this is the machine's own caller. A few commands are refused to anyone else —
    /// not because the *vault* is out of reach but because the command acts on the host, and
    /// "which audience" is not the question it asks.
    pub fn is_all(&self) -> bool {
        matches!(self, Scope::All)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_is_the_default_and_reaches_everything() {
        assert_eq!(Scope::default(), Scope::All);
        assert!(Scope::All.allows("anything"));
        assert!(Scope::All.names().is_none(), "unrestricted is None, not a list of every vault");
    }

    #[test]
    fn only_reaches_exactly_what_it_names() {
        let s = Scope::Only(vec!["lab".into()]);
        assert!(s.allows("lab"));
        assert!(!s.allows("personal"));
        assert!(!s.allows(""), "an unnamed vault is not a wildcard");
        assert!(!s.is_all());
    }

    /// The inversion that matters: an empty allowlist is empty. A device whose last vault was
    /// revoked must lose access, not gain all of it.
    #[test]
    fn an_empty_scope_grants_nothing() {
        let s = Scope::Only(Vec::new());
        assert!(!s.allows("personal"));
        assert!(!s.is_all());
        assert_eq!(s.names(), Some(&[] as &[String]));
    }
}
