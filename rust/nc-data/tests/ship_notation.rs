//! The compact ship notation (`docs/dev/ship-notation.md`): `CODEtotal!crippled`
//! tokens, the crippled ships included in the total, nothing for a class with
//! no hulls, `!0` never written, and `TT*` kept for loaded transports.

use nc_data::{
    Composition, CrippledHulls, Fleet, ShipCategory, crippled_by_category, fit_notation,
    fleet_notation, join_tokens, set_armies_aboard, set_troop_transports,
    ship_composition_table_summary, ship_token,
};

fn hulls(count: u32) -> CrippledHulls {
    CrippledHulls {
        turns: [0, count],
        fresh: 0,
    }
}

fn fleet(sc: u16, bb: u16, ca: u16, dd: u16, loaded: u16, empty: u16, etac: u16) -> Fleet {
    let mut fleet = Fleet::default();
    fleet.ships.scouts = sc;
    fleet.ships.battleships = bb;
    fleet.ships.cruisers = ca;
    fleet.ships.destroyers = dd;
    set_troop_transports(&mut fleet, loaded + empty);
    set_armies_aboard(&mut fleet, loaded);
    fleet.ships.etacs = etac;
    fleet
}

#[test]
fn a_token_is_code_total_and_an_optional_crippled_count() {
    assert_eq!(ship_token("CA", 12, 0).as_deref(), Some("CA12"));
    assert_eq!(ship_token("CA", 12, 3).as_deref(), Some("CA12!3"));
    assert_eq!(ship_token("BB", 1, 1).as_deref(), Some("BB1!1"));
    assert_eq!(ship_token("SC", 1, 0).as_deref(), Some("SC1"));
    assert_eq!(ship_token("CA", 0, 0), None);
    // The crippled ships are part of the total, never more than it.
    assert_eq!(ship_token("CA", 2, 5).as_deref(), Some("CA2!2"));
}

#[test]
fn a_healthy_fleet_has_no_crippled_marks_at_all() {
    let healthy = fleet(1, 2, 12, 8, 4, 5, 2);
    let notation = fleet_notation(&healthy);
    assert_eq!(notation, "SC1 BB2 CA12 DD8 TT*4 TT5 ETAC2");
    assert!(!notation.contains('!'));
    assert_eq!(ship_composition_table_summary(&healthy), notation);
}

#[test]
fn damaged_classes_show_their_crippled_count_inside_the_total() {
    let mut mixed = fleet(0, 2, 12, 8, 0, 0, 0);
    mixed.crippled.battleships = hulls(1);
    mixed.crippled.cruisers = hulls(3);
    assert_eq!(fleet_notation(&mixed), "BB2!1 CA12!3 DD8");
    // A wholly crippled class reads the same way.
    mixed.crippled.destroyers = hulls(8);
    assert_eq!(fleet_notation(&mixed), "BB2!1 CA12!3 DD8!8");
}

#[test]
fn damage_stays_with_its_own_class() {
    let mut fleet = fleet(1, 0, 0, 4, 0, 0, 1);
    fleet.crippled.destroyers = hulls(2);
    assert_eq!(fleet_notation(&fleet), "SC1 DD4!2 ETAC1");
}

#[test]
fn loaded_transports_keep_their_star_and_damage_is_a_separate_mark() {
    let mut transports = fleet(0, 0, 0, 0, 4, 5, 0);
    assert_eq!(fleet_notation(&transports), "TT*4 TT5");
    // Loaded and empty transports each have their own crippled counter, so
    // the notation shows the record's state, not a split of a shared total.
    transports.crippled.loaded_transports = hulls(1);
    assert_eq!(fleet_notation(&transports), "TT*4!1 TT5");
    transports.crippled.empty_transports = hulls(2);
    assert_eq!(fleet_notation(&transports), "TT*4!1 TT5!2");
    let by_category = crippled_by_category(&transports);
    assert_eq!(by_category[ShipCategory::FullTransports.index()], 1);
    assert_eq!(by_category[ShipCategory::EmptyTransports.index()], 2);
}

#[test]
fn transport_damage_follows_the_cargo_state() {
    let mut transports = fleet(0, 0, 0, 0, 0, 3, 0);
    transports.crippled.empty_transports = hulls(2);
    assert_eq!(fleet_notation(&transports), "TT3!2");
    // Loading takes the healthy transport first and the crippled ones only
    // when no healthy one is left; unloading puts the crippled ones ashore
    // first.
    set_armies_aboard(&mut transports, 1);
    assert_eq!(fleet_notation(&transports), "TT*1 TT2!2");
    set_armies_aboard(&mut transports, 3);
    assert_eq!(fleet_notation(&transports), "TT*3!2");
    set_armies_aboard(&mut transports, 1);
    assert_eq!(fleet_notation(&transports), "TT*1 TT2!2");
}

#[test]
fn a_starbase_uses_the_same_syntax() {
    let base = |total, crippled| Composition {
        starbases: (total, crippled),
        ..Composition::default()
    };
    assert_eq!(base(1, 0).notation(), "SB1");
    assert_eq!(base(1, 1).notation(), "SB1!1");
    assert_eq!(base(0, 0).notation(), "");
}

#[test]
fn a_composition_writes_every_class_in_the_one_order() {
    let all = Composition::undamaged([
        // Given out of order on purpose: the order is the formatter's.
        (ShipCategory::Etacs, 7),
        (ShipCategory::Destroyers, 4),
        (ShipCategory::EmptyTransports, 6),
        (ShipCategory::Scouts, 1),
        (ShipCategory::FullTransports, 5),
        (ShipCategory::Cruisers, 3),
        (ShipCategory::Battleships, 2),
    ])
    .with_starbases(8);
    assert_eq!(all.notation(), "SC1 BB2 CA3 DD4 TT*5 TT6 ETAC7 SB8");
    assert_eq!(Composition::default().notation(), "");
}

#[test]
fn tokens_join_with_single_spaces_and_skip_empty_classes() {
    let joined = join_tokens([
        ship_token("DD", 8, 0),
        ship_token("SC", 0, 0),
        ship_token("CA", 12, 3),
    ]);
    assert_eq!(joined, "DD8 CA12!3");
    assert_eq!(join_tokens([None, None]), "");
}

#[test]
fn class_codes_parse_and_etac_keeps_its_short_form() {
    for category in ShipCategory::ALL {
        assert_eq!(ShipCategory::from_code(category.code()), Some(category));
    }
    assert_eq!(ShipCategory::from_code("ET"), Some(ShipCategory::Etacs));
    assert_eq!(
        ShipCategory::from_code("TT*"),
        Some(ShipCategory::FullTransports)
    );
    assert_eq!(ShipCategory::from_code("XX"), None);
}

#[test]
fn a_composition_that_fits_is_returned_whole() {
    assert_eq!(fit_notation("DD8 CA12!3", 18, true), "DD8 CA12!3");
    assert_eq!(fit_notation("DD8 CA12", 8, false), "DD8 CA12");
}

#[test]
fn a_healthy_composition_is_cut_between_tokens_with_a_plus() {
    let ships = "SC1 BB2 CA12 DD8 TT*4 TT5 ETAC2";
    assert_eq!(fit_notation(ships, 18, false), "SC1 BB2 CA12 DD8 +");
    assert_eq!(fit_notation(ships, 17, false), "SC1 BB2 CA12 +");
    assert_eq!(fit_notation(ships, 4, false), "+");
    assert_eq!(fit_notation(ships, 0, false), "");
}

#[test]
fn a_damaged_composition_keeps_a_lone_bang_however_far_it_is_cut() {
    let ships = "SC1 BB2 CA12 DD8 TT*4 TT5 ETAC2!1";
    assert_eq!(fit_notation(ships, 18, true), "SC1 BB2 CA12 +!");
    assert_eq!(fit_notation(ships, 5, true), "+!");
    assert_eq!(fit_notation(ships, 2, true), "+!");
    assert_eq!(fit_notation(ships, 1, true), "!");
    // The same fleet with its damage showing in the tokens that fit.
    assert_eq!(
        fit_notation("SC1 BB2!1 CA12!3 DD8 TT*4!1 TT5 ETAC2", 18, true),
        "SC1 BB2!1 +!"
    );
}
