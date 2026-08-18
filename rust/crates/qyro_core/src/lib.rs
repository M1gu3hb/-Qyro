//! Shared product-state primitives for Qyro.
//!
//! This crate deliberately contains no UI or platform-specific code.

// Added in sprint 4D.1, and the reason is worth a line because the attribute
// looks like boilerplate and is not. `qyro_win_dpapi` is now allowed `unsafe`,
// so "which crates may relax this" became a list rather than a habit, and
// `only_the_listed_crates_may_relax_forbid_unsafe` in `qyro_identity_store`
// enforces it. Writing that guard is what found this crate: it could carry the
// attribute and did not, while STATUS.md and ADR-0024 both said every crate
// did (QYR-0054). Removing this line turns the guard red.
#![forbid(unsafe_code)]
use std::collections::BTreeMap;

#[cfg(test)]
mod guards;

/// Wire protocol implemented by this milestone.
pub const fn protocol_version() -> &'static str {
    "QYRO/1"
}

/// Components whose real initialization state is shown during boot.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Component {
    /// Protocol, identity, and cryptographic services.
    SecureCore,
    /// Local persistence.
    LocalDatabase,
    /// At least one usable transport.
    Transports,
}

/// Observable initialization state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentState {
    /// Initialization completed successfully.
    Ready,
    /// The component is not available and must not be reported as ready.
    Unavailable,
}

/// Immutable snapshot consumed by presentation layers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadinessReport {
    states: BTreeMap<Component, ComponentState>,
}

impl ReadinessReport {
    /// Returns the state of a required component.
    pub fn state(&self, component: Component) -> ComponentState {
        self.states
            .get(&component)
            .copied()
            .unwrap_or(ComponentState::Unavailable)
    }

    /// Returns true only when every required component is ready.
    pub fn is_ready(&self) -> bool {
        REQUIRED_COMPONENTS
            .iter()
            .all(|component| self.state(*component) == ComponentState::Ready)
    }
}

const REQUIRED_COMPONENTS: [Component; 3] = [
    Component::SecureCore,
    Component::LocalDatabase,
    Component::Transports,
];

/// Builds a readiness snapshot from actual initialization results.
pub fn readiness(results: impl IntoIterator<Item = (Component, bool)>) -> ReadinessReport {
    let mut states = REQUIRED_COMPONENTS
        .into_iter()
        .map(|component| (component, ComponentState::Unavailable))
        .collect::<BTreeMap<_, _>>();

    for (component, ready) in results {
        states.insert(
            component,
            if ready {
                ComponentState::Ready
            } else {
                ComponentState::Unavailable
            },
        );
    }

    ReadinessReport { states }
}
