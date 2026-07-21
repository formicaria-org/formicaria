//! Hard safety guardrails on a **proposal** — the bound on the blast radius of any proposed
//! change, whether a human made it out of band or (later) an agent did.
//!
//! This module is **pure**: it decides "does this change fit the limits?", given the change's
//! weight and the vault's current open-proposal load. It touches no git and no store. The
//! command that *creates* a proposal enforces it at the write gate, so the by-hand path and the
//! future agent inherit the same bound from one place — the agent needs no guardrail of its own.
//!
//! The limits live in the vault's own `vault.json` (see [`crate::Descriptor`]) for exactly the
//! reason `git_assets_max` does: **they decide what may enter shared, permanent history, so the
//! vault — not the loosest device — sets them.** An absent config means the conservative
//! built-in [defaults](ProposalLimits::default); a limit is raised **explicitly**, never
//! silently, and a change that exceeds a live limit is **refused, not truncated**.

/// Conservative built-in defaults, in force when `vault.json` says nothing. Deliberately small:
/// a proposal is a focused change a human reviews, so the safe failure is "refused — raise the
/// limit if you meant it", never "a 500-file rewrite slipped through unreviewed".
pub const DEFAULT_MAX_FILES: usize = 10;
/// 256 kB across the whole change — comfortably more than a long note, far less than a dump.
pub const DEFAULT_MAX_CHANGE_BYTES: u64 = 256_000;
/// How many proposals a single vault may hold open at once (anti-flood).
pub const DEFAULT_MAX_OPEN: usize = 25;
/// The aggregate weight of a vault's open proposals — 2 MB.
pub const DEFAULT_MAX_OPEN_BYTES: u64 = 2_000_000;

/// The resolved per-vault limits, with defaults already applied. Held on the [`crate::Descriptor`]
/// and read from `vault.json`; [`Default`] is the conservative built-in policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalLimits {
    /// Most files one proposal may add or modify.
    pub max_files: usize,
    /// Most bytes of new content one proposal may carry, summed across its files.
    pub max_change_bytes: u64,
    /// Most proposals a vault may hold **open** at once.
    pub max_open: usize,
    /// Most aggregate change-bytes a vault's **open** proposals may hold at once.
    pub max_open_bytes: u64,
}

impl Default for ProposalLimits {
    fn default() -> Self {
        Self {
            max_files: DEFAULT_MAX_FILES,
            max_change_bytes: DEFAULT_MAX_CHANGE_BYTES,
            max_open: DEFAULT_MAX_OPEN,
            max_open_bytes: DEFAULT_MAX_OPEN_BYTES,
        }
    }
}

/// What a proposed change weighs: how many files it adds or modifies, and the total bytes of
/// the new content across them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProposalSize {
    pub files: usize,
    pub bytes: u64,
}

/// The vault's current open-proposal load — how many are already open and their aggregate
/// weight. Derived by the caller from the existing proposal notes; this module only compares.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VaultLoad {
    pub open: usize,
    pub open_bytes: u64,
}

/// Why a proposal was refused. Each variant carries the offending number **and** the limit it
/// hit, so the caller can tell the user exactly which guardrail bit and what to raise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardrailBreach {
    /// The proposal touches more files than `max`.
    TooManyFiles { files: usize, max: usize },
    /// The proposal's change is larger than `max` bytes.
    ChangeTooLarge { bytes: u64, max: u64 },
    /// Adding this proposal would leave the vault with `would_be` open proposals, past `max`.
    TooManyOpen { would_be: usize, max: usize },
    /// Adding this proposal would leave the vault's open proposals weighing `would_be`, past `max`.
    VaultTooLarge { would_be: u64, max: u64 },
}

impl std::fmt::Display for GuardrailBreach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::descriptor::format_size;
        match self {
            Self::TooManyFiles { files, max } => {
                write!(f, "this proposal changes {files} files, but the vault allows at most {max} per proposal")
            }
            Self::ChangeTooLarge { bytes, max } => write!(
                f,
                "this proposal's change is {}, but the vault allows at most {} per proposal",
                format_size(*bytes),
                format_size(*max)
            ),
            Self::TooManyOpen { would_be, max } => write!(
                f,
                "the vault would have {would_be} open proposals, but it allows at most {max}; merge or decline some first"
            ),
            Self::VaultTooLarge { would_be, max } => write!(
                f,
                "the vault's open proposals would weigh {}, but it allows at most {}; merge or decline some first",
                format_size(*would_be),
                format_size(*max)
            ),
        }
    }
}

impl ProposalLimits {
    /// **Fail-closed:** a change is allowed only if it clears *every* limit. Checked in a fixed
    /// order so the message is deterministic — the per-proposal limits first (they name the
    /// change), then the per-vault ceilings (they name the vault's standing load). Adding *this*
    /// proposal must not push the open count or the aggregate weight past its ceiling, hence the
    /// `+ 1` and `+ change.bytes`; both saturate so a colossal claimed size can never wrap.
    pub fn check(&self, change: ProposalSize, load: VaultLoad) -> Result<(), GuardrailBreach> {
        if change.files > self.max_files {
            return Err(GuardrailBreach::TooManyFiles { files: change.files, max: self.max_files });
        }
        if change.bytes > self.max_change_bytes {
            return Err(GuardrailBreach::ChangeTooLarge { bytes: change.bytes, max: self.max_change_bytes });
        }
        let would_be = load.open.saturating_add(1);
        if would_be > self.max_open {
            return Err(GuardrailBreach::TooManyOpen { would_be, max: self.max_open });
        }
        let would_be = load.open_bytes.saturating_add(change.bytes);
        if would_be > self.max_open_bytes {
            return Err(GuardrailBreach::VaultTooLarge { would_be, max: self.max_open_bytes });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ProposalLimits {
        ProposalLimits { max_files: 3, max_change_bytes: 1000, max_open: 2, max_open_bytes: 5000 }
    }

    #[test]
    fn a_small_change_into_an_empty_vault_passes() {
        let ok = limits().check(ProposalSize { files: 2, bytes: 500 }, VaultLoad::default());
        assert_eq!(ok, Ok(()));
    }

    #[test]
    fn at_the_limit_is_allowed_one_over_is_not() {
        let l = limits();
        // Exactly at each per-proposal limit: allowed.
        assert_eq!(l.check(ProposalSize { files: 3, bytes: 1000 }, VaultLoad::default()), Ok(()));
        // One file too many.
        assert_eq!(
            l.check(ProposalSize { files: 4, bytes: 0 }, VaultLoad::default()),
            Err(GuardrailBreach::TooManyFiles { files: 4, max: 3 })
        );
        // One byte too large.
        assert_eq!(
            l.check(ProposalSize { files: 1, bytes: 1001 }, VaultLoad::default()),
            Err(GuardrailBreach::ChangeTooLarge { bytes: 1001, max: 1000 })
        );
    }

    #[test]
    fn per_vault_count_counts_the_proposal_being_added() {
        let l = limits(); // max_open = 2
        // One already open + this one = 2: allowed.
        assert_eq!(l.check(ProposalSize { files: 1, bytes: 1 }, VaultLoad { open: 1, open_bytes: 0 }), Ok(()));
        // Two already open + this one = 3: refused.
        assert_eq!(
            l.check(ProposalSize { files: 1, bytes: 1 }, VaultLoad { open: 2, open_bytes: 0 }),
            Err(GuardrailBreach::TooManyOpen { would_be: 3, max: 2 })
        );
    }

    #[test]
    fn per_vault_aggregate_bytes_includes_this_change() {
        // max_open_bytes = 5000, and each change stays under the per-proposal cap (1000) so the
        // *aggregate* ceiling is what bites, not the per-proposal one.
        let l = limits();
        // 4000 already open + 1000 = 5000: exactly at the ceiling, allowed.
        assert_eq!(l.check(ProposalSize { files: 1, bytes: 1000 }, VaultLoad { open: 0, open_bytes: 4000 }), Ok(()));
        // 4500 already open + 600 = 5100: over the aggregate ceiling (each still ≤ the per-proposal cap).
        assert_eq!(
            l.check(ProposalSize { files: 1, bytes: 600 }, VaultLoad { open: 0, open_bytes: 4500 }),
            Err(GuardrailBreach::VaultTooLarge { would_be: 5100, max: 5000 })
        );
    }

    #[test]
    fn per_proposal_limits_are_checked_before_per_vault_ones() {
        // A change that violates both a per-proposal limit and a per-vault one reports the
        // per-proposal breach first — the deterministic, most-actionable message.
        let l = limits();
        let breach = l.check(
            ProposalSize { files: 99, bytes: 9999 },
            VaultLoad { open: 99, open_bytes: 99_999 },
        );
        assert_eq!(breach, Err(GuardrailBreach::TooManyFiles { files: 99, max: 3 }));
    }

    #[test]
    fn saturating_add_cannot_wrap_on_a_colossal_change() {
        let l = ProposalLimits { max_open_bytes: u64::MAX, ..limits() };
        // A change claiming near-u64::MAX bytes still refuses on the per-proposal cap, never wraps.
        let breach = l.check(ProposalSize { files: 1, bytes: u64::MAX }, VaultLoad { open: 0, open_bytes: 10 });
        assert_eq!(breach, Err(GuardrailBreach::ChangeTooLarge { bytes: u64::MAX, max: 1000 }));
    }

    #[test]
    fn the_default_policy_is_the_conservative_constants() {
        let d = ProposalLimits::default();
        assert_eq!(d.max_files, DEFAULT_MAX_FILES);
        assert_eq!(d.max_change_bytes, DEFAULT_MAX_CHANGE_BYTES);
        assert_eq!(d.max_open, DEFAULT_MAX_OPEN);
        assert_eq!(d.max_open_bytes, DEFAULT_MAX_OPEN_BYTES);
    }
}
