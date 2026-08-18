//! The trust vocabulary this facade owns, and the conversion it does inside.
//!
//! Specification: `docs/adr/ADR-0035-discovery-and-pairing.md` §§3–5, and
//! ADR-0031 for the decision itself, which is **not** reimplemented here.
//!
//! # Why this exists instead of a re-export
//!
//! ADR-0032 §2 bounds what `qyro_ffi` can name to this crate's public API, and
//! the guard `qyro_session_re_exports_nothing_it_does_not_own` keeps the bound
//! real: a `pub use qyro_identity_store::TrustVerdict` here would put that
//! crate's whole vocabulary on the C boundary, and every variant it ever adds
//! with it.
//!
//! So this crate owns three words and converts. The cost is a `match` with
//! three arms. What it buys is that the surface Dart sees changes when somebody
//! decides it should, and not as a side effect of an internal edit.
//!
//! # What is *not* here, and it matters
//!
//! **Nothing persists.** `seal_known_peers` needs a [`SecretWrapper`], which is
//! DPAPI on Windows and does not exist yet on Android — that is phase 06. So
//! this book lives in memory and dies with the process, exactly like the
//! Android identity does today.
//!
//! That is a real limitation and it is stated rather than hidden: **a peer
//! marked as known is known until the app closes.** What works today is the
//! decision — a changed key is refused by name — not its memory.
//!
//! [`SecretWrapper`]: qyro_identity_store::SecretWrapper

use std::collections::BTreeMap;

use qyro_crypto::PublicIdentity;
use qyro_identity_store::{HumanFingerprint, KnownPeer, KnownPeers, PeerCandidate, TrustVerdict};

use crate::error::SessionError;

/// What the store says about a peer, in this crate's words.
///
/// Three outcomes and **no boolean**. ADR-0031 refused a boolean on purpose:
/// «new» and «known» are not two values of one question, because the answer to
/// «should I trust this» is different for each and a `bool` erases which one
/// you were told.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeerTrust {
    /// A record exists under this name and the whole public identity matches.
    Known,
    /// A record exists under this name and the identity **changed**.
    ///
    /// In SSH this is a shouted warning, and it is one here. Nothing may treat
    /// it as a softer `New`.
    Changed,
    /// No record under this name. Not an error, and not permission either.
    New,
}

impl PeerTrust {
    /// The stable integer the C boundary carries.
    ///
    /// Written out rather than derived from the enum's ordering, because a
    /// discriminant that moves when somebody reorders the variants is a wire
    /// format that changes by accident.
    #[must_use]
    pub const fn code(self) -> i32 {
        match self {
            Self::Known => 0,
            Self::Changed => 1,
            Self::New => 2,
        }
    }

    const fn from_verdict(verdict: TrustVerdict) -> Self {
        match verdict {
            TrustVerdict::KnownAndMatches => Self::Known,
            TrustVerdict::KnownAndChanged => Self::Changed,
            TrustVerdict::New => Self::New,
        }
    }
}

/// The peers this device has been told to remember, and their identities.
///
/// In memory only — see the module comment.
#[derive(Debug, Default)]
pub struct TrustBook {
    peers: BTreeMap<String, PublicIdentity>,
}

impl TrustBook {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What the book says about `identity` under `name`.
    ///
    /// The decision is `qyro_identity_store::decide_trust` and nothing here
    /// reimplements it: this assembles the candidate and the store it already
    /// takes, and translates the answer.
    ///
    /// # Errors
    ///
    /// [`SessionError::BadArgument`] when the name is one the store refuses —
    /// empty, over 255 bytes, or carrying a control character.
    pub fn verdict(
        &self,
        name: &str,
        identity: &PublicIdentity,
    ) -> Result<PeerTrust, SessionError> {
        let candidate =
            PeerCandidate::new(name, identity.clone()).map_err(|_| SessionError::BadArgument)?;
        let store = self.as_known_peers()?;
        Ok(PeerTrust::from_verdict(qyro_identity_store::decide_trust(
            &candidate, &store,
        )))
    }

    /// Records `identity` under `name`, replacing whatever was there.
    ///
    /// **Only a person may cause this.** ADR-0035 §4: a peer never enters the
    /// book because a transfer succeeded, and a `Changed` verdict never
    /// overwrites silently — the caller has to [`Self::forget`] first, which is
    /// a different act with a different name.
    ///
    /// # Errors
    ///
    /// [`SessionError::BadArgument`] for a name the store refuses.
    pub fn remember(&mut self, name: &str, identity: &PublicIdentity) -> Result<(), SessionError> {
        // Validated through the store's own constructor rather than by a second
        // copy of its rules here: two validators are two validators that can
        // disagree, and the one that matters is the one the store applies.
        KnownPeer::new(name, identity.clone(), 0, 0).map_err(|_| SessionError::BadArgument)?;
        self.peers.insert(name.to_owned(), identity.clone());
        Ok(())
    }

    /// Removes `name`. Returns whether there was anything to remove.
    ///
    /// The only way back from [`PeerTrust::Changed`], and deliberately so.
    pub fn forget(&mut self, name: &str) -> bool {
        self.peers.remove(name).is_some()
    }

    /// Every remembered name, sorted, so a list does not reorder itself.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.peers.keys().cloned().collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// The fingerprint recorded under `name`, **formatted by the core**.
    ///
    /// ADR-0035 §4: the interface never invents a format. Two devices showing
    /// the same fingerprint differently makes comparing it out loud worthless,
    /// which is the only thing a fingerprint is for.
    #[must_use]
    pub fn fingerprint_of(&self, name: &str) -> Option<String> {
        self.peers.get(name).map(fingerprint_text)
    }

    fn as_known_peers(&self) -> Result<KnownPeers, SessionError> {
        let records: Vec<KnownPeer> = self
            .peers
            .iter()
            .map(|(name, identity)| KnownPeer::new(name, identity.clone(), 0, 0))
            .collect::<Result<_, _>>()
            .map_err(|_| SessionError::BadArgument)?;
        // `TryFrom` rather than a builder: it is the conversion the store
        // already publishes, and it is where the duplicate-name refusal lives.
        // A second way in would be a second set of rules.
        KnownPeers::try_from(records).map_err(|_| SessionError::BadArgument)
    }
}

/// The grouped-hex text for an identity, as `HumanFingerprint` writes it.
#[must_use]
pub fn fingerprint_text(identity: &PublicIdentity) -> String {
    HumanFingerprint::from_fingerprint(identity.fingerprint()).to_grouped_hex()
}
