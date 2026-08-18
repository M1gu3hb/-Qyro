//! Public contracts for ADR-0031.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "contract tests must fail loudly and alter exact wire bytes"
)]

use qyro_crypto::{DeviceIdentity, IdentitySecret, SEED_LEN};
use qyro_identity_store::{
    HumanFingerprint, KnownPeer, KnownPeerStoreError, KnownPeers, MAX_KNOWN_PEERS,
    MAX_PEER_NAME_LEN, MAX_WRAPPED_KNOWN_PEERS_LEN, PeerCandidate, SecretWrapper, StoreError,
    TrustVerdict, decide_trust, open_known_peers, seal_known_peers,
};
use std::cell::RefCell;
use zeroize::Zeroizing;

struct FakeWrapper;

impl FakeWrapper {
    fn tag(entropy: &[u8], payload: &[u8]) -> [u8; 8] {
        let mut acc: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in entropy.iter().chain(payload) {
            acc ^= u64::from(*byte);
            acc = acc.wrapping_mul(0x0100_0000_01b3);
        }
        acc.to_be_bytes()
    }
}

impl SecretWrapper for FakeWrapper {
    fn wrap(&self, secret: &[u8], entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
        let mut out = Vec::with_capacity(8 + secret.len());
        out.extend_from_slice(&Self::tag(entropy, secret));
        out.extend_from_slice(secret);
        Ok(out)
    }

    fn unwrap(&self, wrapped: &[u8], entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        if wrapped.len() < 8 {
            return Err(StoreError::Unwrap { code: 101 });
        }
        let (tag, payload) = wrapped.split_at(8);
        if tag != Self::tag(entropy, payload) {
            return Err(StoreError::Unwrap { code: 102 });
        }
        Ok(Zeroizing::new(payload.to_vec()))
    }

    fn wrap_id(&self) -> u8 {
        1
    }
}

fn identity(byte: u8) -> DeviceIdentity {
    DeviceIdentity::from_secret(&IdentitySecret::from_bytes(&[byte; SEED_LEN]))
}

fn indexed_identity(index: u32) -> DeviceIdentity {
    let mut seed = [0u8; SEED_LEN];
    seed[..4].copy_from_slice(&index.to_be_bytes());
    DeviceIdentity::from_secret(&IdentitySecret::from_bytes(&seed))
}

struct TransparentWrapper;

impl SecretWrapper for TransparentWrapper {
    fn wrap(&self, secret: &[u8], _entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
        Ok(secret.to_vec())
    }

    fn unwrap(&self, wrapped: &[u8], _entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        Ok(Zeroizing::new(wrapped.to_vec()))
    }

    fn wrap_id(&self) -> u8 {
        1
    }
}

struct EntropyRecorder {
    observed: RefCell<Vec<u8>>,
}

impl SecretWrapper for EntropyRecorder {
    fn wrap(&self, _secret: &[u8], entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
        self.observed.replace(entropy.to_vec());
        Ok(vec![0])
    }

    fn unwrap(&self, _wrapped: &[u8], _entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        Err(StoreError::Unwrap { code: 201 })
    }

    fn wrap_id(&self) -> u8 {
        1
    }
}

struct SizedWrapper(usize);

impl SecretWrapper for SizedWrapper {
    fn wrap(&self, _secret: &[u8], _entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
        Ok(vec![0; self.0])
    }

    fn unwrap(&self, _wrapped: &[u8], _entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        Err(StoreError::Unwrap { code: 202 })
    }

    fn wrap_id(&self) -> u8 {
        1
    }
}

fn transparent_store(clear: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + clear.len());
    out.extend_from_slice(b"QYRO-KPS");
    out.extend_from_slice(&[1, 1, 0, 0]);
    out.extend_from_slice(&u32::try_from(clear.len()).unwrap().to_be_bytes());
    out.extend_from_slice(clear);
    out
}

fn one_known_peer() -> KnownPeers {
    KnownPeers::try_from(vec![
        KnownPeer::new(
            "ordenador de casa",
            identity(0x11).public_identity().clone(),
            1_723_000_000,
            1_723_000_600,
        )
        .unwrap(),
    ])
    .unwrap()
}

#[test]
fn a_known_peer_whose_key_changed_is_refused_by_name() {
    let known = one_known_peer();
    let candidate = PeerCandidate::new(
        "ordenador de casa",
        identity(0x22).public_identity().clone(),
    )
    .unwrap();

    assert_eq!(
        decide_trust(&candidate, &known),
        TrustVerdict::KnownAndChanged
    );
}

#[test]
fn a_new_peer_is_reported_as_new_and_not_as_trusted() {
    let known = one_known_peer();
    let candidate =
        PeerCandidate::new("teléfono nuevo", identity(0x22).public_identity().clone()).unwrap();

    assert_eq!(decide_trust(&candidate, &known), TrustVerdict::New);
    assert_ne!(
        decide_trust(&candidate, &known),
        TrustVerdict::KnownAndMatches
    );
}

#[test]
fn a_known_peer_with_the_same_key_is_trusted() {
    let known = one_known_peer();
    let candidate = PeerCandidate::new(
        "ordenador de casa",
        identity(0x11).public_identity().clone(),
    )
    .unwrap();

    assert_eq!(
        decide_trust(&candidate, &known),
        TrustVerdict::KnownAndMatches
    );
}

#[test]
fn the_human_fingerprint_is_exactly_the_first_one_hundred_twenty_eight_bits() {
    let identity = identity(0x33);
    let human = HumanFingerprint::from(identity.public_identity());
    let displayed = human.to_string();
    let compact = displayed.replace('-', "");

    assert_eq!(human.as_bytes().len(), 16);
    assert_eq!(compact.len(), 32);
    assert_eq!(
        displayed.split('-').map(str::len).collect::<Vec<_>>(),
        [8; 4]
    );
    assert!(identity.fingerprint().to_hex().starts_with(&compact));
    assert!(
        compact
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    assert_eq!(human.to_grouped_hex(), displayed);
    assert_eq!(format!("{human:?}"), format!("HumanFingerprint({human})"));
}

#[test]
fn known_peers_survive_a_seal_and_open_round_trip() {
    let original = one_known_peer();
    let sealed = seal_known_peers(&original, &FakeWrapper).unwrap();
    let opened = open_known_peers(&sealed, &FakeWrapper).unwrap();

    assert_eq!(opened, original);
    assert_eq!(opened.len(), 1);
    let peer = opened.iter().next().unwrap();
    assert_eq!(peer.name(), "ordenador de casa");
    assert_eq!(peer.first_seen(), 1_723_000_000);
    assert_eq!(peer.last_seen(), 1_723_000_600);
    assert_eq!(peer.identity(), identity(0x11).public_identity());
    assert!(!opened.is_empty());
}

#[test]
fn an_empty_store_reports_zero_and_is_empty() {
    let empty = KnownPeers::new();

    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
}

#[test]
fn two_records_report_the_measured_length() {
    let records = vec![
        KnownPeer::new("one", indexed_identity(1).public_identity().clone(), 0, 0).unwrap(),
        KnownPeer::new("two", indexed_identity(2).public_identity().clone(), 0, 1).unwrap(),
    ];
    let store = KnownPeers::try_from(records).unwrap();

    assert_eq!(store.len(), 2);
    assert!(!store.is_empty());
}

#[test]
fn duplicate_local_names_and_duplicate_keys_are_distinct_refusals() {
    let first =
        KnownPeer::new("same", indexed_identity(1).public_identity().clone(), 0, 0).unwrap();
    let duplicate_name =
        KnownPeer::new("same", indexed_identity(2).public_identity().clone(), 0, 0).unwrap();
    let duplicate_key = KnownPeer::new(
        "different",
        indexed_identity(1).public_identity().clone(),
        0,
        0,
    )
    .unwrap();

    assert_eq!(
        KnownPeers::try_from(vec![first.clone(), duplicate_name]),
        Err(KnownPeerStoreError::DuplicateName)
    );
    assert_eq!(
        KnownPeers::try_from(vec![first, duplicate_key]),
        Err(KnownPeerStoreError::DuplicateIdentity)
    );
}

#[test]
fn the_peer_limit_accepts_the_exact_boundary_and_refuses_one_more() {
    let mut records = Vec::with_capacity(MAX_KNOWN_PEERS + 1);
    for index in 0..=u32::try_from(MAX_KNOWN_PEERS).unwrap() {
        records.push(
            KnownPeer::new(
                &format!("peer-{index}"),
                indexed_identity(index).public_identity().clone(),
                0,
                0,
            )
            .unwrap(),
        );
    }
    let extra = records.pop().unwrap();
    let exact = KnownPeers::try_from(records).unwrap();

    assert_eq!(exact.len(), MAX_KNOWN_PEERS);
    let mut over: Vec<KnownPeer> = exact.iter().cloned().collect();
    over.push(extra);
    assert_eq!(
        KnownPeers::try_from(over),
        Err(KnownPeerStoreError::TooManyPeers {
            found: MAX_KNOWN_PEERS + 1
        })
    );
}

#[test]
fn names_and_timestamps_accept_only_the_frozen_boundaries() {
    let key = indexed_identity(17).public_identity().clone();
    let maximum_name = "a".repeat(MAX_PEER_NAME_LEN);

    assert!(KnownPeer::new(&maximum_name, key.clone(), 0, 0).is_ok());
    assert_eq!(
        KnownPeer::new(&format!("{maximum_name}a"), key.clone(), 0, 0),
        Err(KnownPeerStoreError::NameTooLong {
            found: MAX_PEER_NAME_LEN + 1
        })
    );
    assert_eq!(
        KnownPeer::new("", key.clone(), 0, 0),
        Err(KnownPeerStoreError::EmptyName)
    );
    assert_eq!(
        KnownPeer::new("line\nbreak", key.clone(), 0, 0),
        Err(KnownPeerStoreError::NameContainsControl)
    );
    assert!(KnownPeer::new("equal", key.clone(), 7, 7).is_ok());
    assert_eq!(
        KnownPeer::new("negative first", key.clone(), -1, 0),
        Err(KnownPeerStoreError::InvalidTimestamps {
            first_seen: -1,
            last_seen: 0
        })
    );
    assert_eq!(
        KnownPeer::new("last before first", key, 1, 0),
        Err(KnownPeerStoreError::InvalidTimestamps {
            first_seen: 1,
            last_seen: 0
        })
    );
}

#[test]
fn candidate_accessors_return_the_local_name_and_complete_key() {
    let identity = indexed_identity(88);
    let candidate =
        PeerCandidate::new("selected locally", identity.public_identity().clone()).unwrap();

    assert_eq!(candidate.expected_name(), "selected locally");
    assert_eq!(candidate.identity(), identity.public_identity());
}

#[test]
fn a_store_from_a_future_version_is_refused_by_version() {
    let mut sealed = seal_known_peers(&one_known_peer(), &FakeWrapper).unwrap();
    sealed[8] = 2;

    assert_eq!(
        open_known_peers(&sealed, &FakeWrapper),
        Err(KnownPeerStoreError::UnsupportedKnownPeerVersion { found: 2 })
    );
}

#[test]
fn the_known_peer_entropy_is_the_frozen_domain_and_header() {
    let wrapper = EntropyRecorder {
        observed: RefCell::new(Vec::new()),
    };

    seal_known_peers(&KnownPeers::new(), &wrapper).unwrap();

    let mut expected = b"qyro.known-peers.store.v1".to_vec();
    expected.extend_from_slice(b"QYRO-KPS");
    expected.extend_from_slice(&[1, 1, 0, 0]);
    assert_eq!(*wrapper.observed.borrow(), expected);
}

#[test]
fn the_wrapped_limit_accepts_exactly_two_mebibytes_and_refuses_one_more() {
    assert_eq!(MAX_WRAPPED_KNOWN_PEERS_LEN, 2_097_152);
    assert!(
        seal_known_peers(
            &KnownPeers::new(),
            &SizedWrapper(MAX_WRAPPED_KNOWN_PEERS_LEN)
        )
        .is_ok()
    );
    assert_eq!(
        seal_known_peers(
            &KnownPeers::new(),
            &SizedWrapper(MAX_WRAPPED_KNOWN_PEERS_LEN + 1)
        ),
        Err(KnownPeerStoreError::WrappedTooLarge {
            found: MAX_WRAPPED_KNOWN_PEERS_LEN + 1
        })
    );

    let exact = transparent_store(&vec![0; MAX_WRAPPED_KNOWN_PEERS_LEN]);
    assert!(!matches!(
        open_known_peers(&exact, &SizedWrapper(0)),
        Err(KnownPeerStoreError::WrappedTooLarge { .. })
    ));
    let over = transparent_store(&vec![0; MAX_WRAPPED_KNOWN_PEERS_LEN + 1]);
    assert_eq!(
        open_known_peers(&over, &SizedWrapper(0)),
        Err(KnownPeerStoreError::WrappedTooLarge {
            found: MAX_WRAPPED_KNOWN_PEERS_LEN + 1
        })
    );
}

#[test]
fn clear_body_and_record_count_limits_accept_the_exact_boundaries() {
    const MAX_CLEAR_STORE_LEN: usize = 1_269_764;
    let exact_clear = transparent_store(&vec![0; MAX_CLEAR_STORE_LEN]);
    assert!(!matches!(
        open_known_peers(&exact_clear, &TransparentWrapper),
        Err(KnownPeerStoreError::UnwrappedTooLarge { .. })
    ));
    let over_clear = transparent_store(&vec![0; MAX_CLEAR_STORE_LEN + 1]);
    assert_eq!(
        open_known_peers(&over_clear, &TransparentWrapper),
        Err(KnownPeerStoreError::UnwrappedTooLarge {
            found: MAX_CLEAR_STORE_LEN + 1
        })
    );

    let exact_count = transparent_store(&u32::try_from(MAX_KNOWN_PEERS).unwrap().to_be_bytes());
    assert!(!matches!(
        open_known_peers(&exact_count, &TransparentWrapper),
        Err(KnownPeerStoreError::TooManyPeers { .. })
    ));
    let over_count = transparent_store(&u32::try_from(MAX_KNOWN_PEERS + 1).unwrap().to_be_bytes());
    assert_eq!(
        open_known_peers(&over_count, &TransparentWrapper),
        Err(KnownPeerStoreError::TooManyPeers {
            found: MAX_KNOWN_PEERS + 1
        })
    );
}

#[test]
fn error_display_names_the_future_version() {
    assert_eq!(
        KnownPeerStoreError::UnsupportedKnownPeerVersion { found: 9 }.to_string(),
        "known-peer store declares unsupported version 9"
    );
}

#[test]
fn a_truncated_store_is_refused_and_does_not_partially_load() {
    let original = one_known_peer();
    let sealed = seal_known_peers(&original, &FakeWrapper).unwrap();

    for cut in 0..sealed.len() {
        assert!(
            open_known_peers(&sealed[..cut], &FakeWrapper).is_err(),
            "a {cut}-byte prefix loaded as a complete store"
        );
    }
}
