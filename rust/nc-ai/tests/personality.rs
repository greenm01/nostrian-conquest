//! Personalities (t167): every built-in profile parses, the default is the
//! tuned executor, and a profile must set every knob once, within bounds.

use nc_ai::{PERSONALITIES, Weights, parse_personality, personality};

#[test]
fn every_built_in_personality_parses_and_the_default_is_the_tuned_executor() {
    for name in PERSONALITIES {
        let style = personality(name).unwrap_or_else(|| panic!("{name} is built in"));
        assert_eq!(style.name, name);
    }
    assert_eq!(personality("land-grab").unwrap().weights, Weights::TUNED);
    assert!(personality("nobody").is_none());
}

#[test]
fn tuning_stays_within_a_styles_bounds() {
    let rusher = personality("rusher").unwrap();
    let mut values = rusher.weights.to_array();
    values[13] = 25.0; // war year, far past the rusher's bound
    values[11] = 4.0; // attack ratio
    let tuned = rusher.weights_from_array(values);
    assert_eq!(tuned.war_year, 3.0);
    assert_eq!(tuned.attack_ratio, 1.0);
    assert_eq!(tuned.doctrine, rusher.weights.doctrine);
}

/// `source` with the first line setting `name` replaced by `with` lines.
fn replace_setting(source: &str, name: &str, with: &[&str]) -> String {
    let prefix = format!("    {name} ");
    let mut done = false;
    source
        .lines()
        .flat_map(|line| {
            if !done && line.starts_with(&prefix) {
                done = true;
                with.iter().map(|line| line.to_string()).collect::<Vec<_>>()
            } else {
                vec![line.to_string()]
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_profile_must_set_every_knob_once_within_bounds() {
    let source = include_str!("../personalities/rusher.kdl");
    assert!(parse_personality(source).is_ok());
    let error = |text: String| parse_personality(&text).unwrap_err();
    assert!(error(replace_setting(source, "tax", &[])).contains("tax is not set"));
    assert!(
        error(replace_setting(
            source,
            "tax",
            &["    tax 60.0", "    tax 61.0"]
        ))
        .contains("set twice")
    );
    assert!(
        error(replace_setting(
            source,
            "tax",
            &["    tax 60.0", "    luck 1.0"]
        ))
        .contains("unknown setting luck")
    );
    assert!(error(replace_setting(source, "war-year", &["    war-year 9.0"])).contains("outside"));
    assert!(error(replace_setting(source, "allocate", &[])).contains("allocate is not set"));
    assert!(
        error(replace_setting(
            source,
            "allocation",
            &["    allocation 2 1"]
        ))
        .contains("bad setting allocation")
    );
}

#[test]
fn a_style_bounds_a_learned_seats_allocation() {
    let mut goals = nc_ai::Goals {
        tax: 0,
        diplomacy: Vec::new(),
        posture: 0,
        allocation: 0,
        strike: 0,
        colonize: 0,
    };
    personality("turtle").unwrap().bound_goals(&mut goals);
    assert_eq!(goals.allocation, 1);
    goals.allocation = 2;
    personality("rusher").unwrap().bound_goals(&mut goals);
    assert_eq!(goals.allocation, 0);
    goals.allocation = 2;
    personality("land-grab").unwrap().bound_goals(&mut goals);
    assert_eq!(goals.allocation, 2);
}
