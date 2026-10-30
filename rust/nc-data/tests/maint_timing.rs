use nc_data::Mission;
use nc_engine::maint::timing::{
    TimingCode, apply_timing_offset, event_base_week, format_report_first_line, format_stardate,
    timing_code_offset,
};

#[test]
fn timing_code_offsets_match_recovered_table() {
    assert_eq!(timing_code_offset(TimingCode::FleetCombat), 2);
    assert_eq!(timing_code_offset(TimingCode::Reserved2), 7);
    assert_eq!(timing_code_offset(TimingCode::ScoutView), 21);
    assert_eq!(timing_code_offset(TimingCode::Standing4), 0);
    assert_eq!(timing_code_offset(TimingCode::Standing5), 0);
    assert_eq!(timing_code_offset(TimingCode::Standing6), 0);
    assert_eq!(timing_code_offset(TimingCode::IpbmSummary7), 0);
    assert_eq!(timing_code_offset(TimingCode::HighOffset8), 30);
}

#[test]
fn apply_timing_offset_clamps_to_valid_range() {
    assert_eq!(apply_timing_offset(1, TimingCode::Standing4), 1);
    assert_eq!(apply_timing_offset(1, TimingCode::FleetCombat), 3);
    assert_eq!(apply_timing_offset(1, TimingCode::ScoutView), 22);
    assert_eq!(apply_timing_offset(40, TimingCode::ScoutView), 52); // 40+21=61 → clamped to 52
    assert_eq!(apply_timing_offset(52, TimingCode::FleetCombat), 52); // 52+2=54 → clamped
}

#[test]
fn base_week_standing_mission_is_1() {
    assert_eq!(event_base_week(Mission::PatrolSector, 1, 3), 1);
    assert_eq!(event_base_week(Mission::GuardStarbase, 1, 3), 1);
    assert_eq!(event_base_week(Mission::GuardBlockadeWorld, 1, 3), 1);
    assert_eq!(event_base_week(Mission::RendezvousSector, 1, 3), 1);
}

#[test]
fn base_week_no_travel_is_1() {
    assert_eq!(event_base_week(Mission::ColonizeWorld, 0, 3), 1);
    assert_eq!(event_base_week(Mission::MoveOnly, 0, 0), 1);
}

#[test]
fn base_week_one_year_speed3_is_around_30() {
    let w = event_base_week(Mission::ColonizeWorld, 1, 3);
    assert!(w >= 25 && w <= 35, "expected ~30, got {w}");
}

#[test]
fn format_stardate_zero_pads_week() {
    assert_eq!(format_stardate(32, 3001), "Stardate: 32/3001");
    assert_eq!(format_stardate(1, 3010), "Stardate: 01/3010");
}

#[test]
fn format_report_first_line_right_justifies_stardate() {
    let source = "From your 1st Fleet, located in System(13,15)";
    let line = format_report_first_line(source, 32, 3001);
    assert!(line.ends_with("Stardate: 32/3001"));
    assert!(
        line.len() == 72 || line.contains("  "),
        "should pad to 72 chars"
    );
}
