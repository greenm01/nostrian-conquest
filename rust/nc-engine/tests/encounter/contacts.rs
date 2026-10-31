//! What a sighting tells its empire: a fleet that is not closing only
//! reaches contact, and a later sighting by a closing fleet identifies the
//! alien and states its decision (`docs/dev/ec-encounter-spec.md`).

use nc_data::{ContactDecision, Order};

use super::support::{Fleet, board, ca, declared, put, years};

const HERE: [u8; 2] = [9, 10];

#[test]
fn a_fleet_that_is_not_closing_only_reaches_contact() {
    let mut scenario = board();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, HERE, false, Order::HoldPosition, ca(1)),
    );
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, HERE, false, Order::HoldPosition, ca(3)),
    );
    let mut contacts = 0;
    for (_, events) in years(&scenario, &declared(&[]), 0..300) {
        for event in &events.scout_contact_events {
            contacts += 1;
            assert!(!event.identified, "{event:?}");
            assert_eq!(event.decision, None);
            assert_eq!(event.target_speed, 0, "the alien is at rest");
        }
    }
    assert!(contacts > 0, "no sightings in 300 years");
}

#[test]
fn a_patrol_identifies_a_contact_in_a_later_week() {
    let mut scenario = board();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, HERE, false, Order::HoldPosition, ca(1)),
    );
    put(
        &mut scenario,
        1,
        Fleet::on_station(1, HERE, false, Order::PatrolSector, ca(1)).roe(6),
    );
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, HERE, false, Order::HoldPosition, ca(3)),
    );
    let mut later = 0;
    for (_, events) in years(&scenario, &declared(&[]), 0..300) {
        let ours: Vec<_> = events
            .scout_contact_events
            .iter()
            .filter(|event| event.viewer_empire == 1)
            .collect();
        let identified: Vec<_> = ours.iter().filter(|event| event.identified).collect();
        assert!(identified.len() <= 1, "{ours:?}");
        for event in &identified {
            assert_eq!(event.decision, Some(ContactDecision::Ignore), "not hostile");
        }
        if let (Some(contact), Some(identification)) = (
            ours.iter().find(|event| !event.identified),
            identified.first(),
        ) {
            assert!(contact.stardate_week <= identification.stardate_week);
            later += usize::from(contact.stardate_week < identification.stardate_week);
        }
    }
    assert!(later > 0, "no identification in a later week in 300 years");
}
