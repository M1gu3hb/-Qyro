use qyro_core::{protocol_version, readiness, Component, ComponentState};

#[test]
fn protocol_version_is_qyro_one() {
    assert_eq!(protocol_version(), "QYRO/1");
}

#[test]
fn bootstrap_reports_real_component_states() {
    let report = readiness([
        (Component::SecureCore, true),
        (Component::LocalDatabase, false),
        (Component::Transports, true),
    ]);

    assert_eq!(report.state(Component::SecureCore), ComponentState::Ready);
    assert_eq!(
        report.state(Component::LocalDatabase),
        ComponentState::Unavailable
    );
    assert!(!report.is_ready());
}

#[test]
fn bootstrap_is_ready_only_when_every_component_is_ready() {
    let report = readiness([
        (Component::SecureCore, true),
        (Component::LocalDatabase, true),
        (Component::Transports, true),
    ]);

    assert!(report.is_ready());
}
