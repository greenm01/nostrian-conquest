//! The yearly economy against the exact models in `tools/economy-model/`
//! (`accounting_review.py` smooth growth and head start, `starbase_review.py`
//! fade and tax cycles, `scorch_review.py` decay). The models use exact
//! fractions; the engine stores Real48, about twelve significant digits, so
//! every comparison here allows a relative error of `TOLERANCE`.

mod common;

use std::collections::BTreeSet;

use common::production::{
    commissioned_starbase, configured_conquest, owned_planet, player_with_empire_name, zeroed_setup,
};
use nc_data::{
    Coords, CoreGameData, PlayerMode, ProductionItemKind, Real48, Starbase, set_planet_name,
    stored_production_points,
};
use nc_engine::{
    build_results_report_blocks, run_maintenance_turn, run_maintenance_turn_with_seed,
};

/// Relative error allowed against the models. The engine agrees to about 1e-10;
/// the model rows below are printed to nine decimals.
const TOLERANCE: f64 = 1e-8;
const WORLD: [u8; 2] = [8, 8];

fn assert_close(actual: f64, expected: f64, what: &str) {
    let allowed = TOLERANCE * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= allowed,
        "{what}: engine {actual} against model {expected}"
    );
}

/// One owned world at `WORLD` with potential `potential`.
fn single_world(tax: u8, potential: u8, production: f64, goods: f64) -> CoreGameData {
    let mut player = player_with_empire_name("Alpha", tax, 0);
    player.mode = PlayerMode::Human;
    let mut world = owned_planet(1, potential, production, 0, 3, 1);
    world.sector = Coords::from_array(WORLD);
    world.stored_goods = Real48::from_f64(goods);
    CoreGameData {
        players: vec![player],
        planets: vec![world],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    }
}

fn pass(game: &mut CoreGameData) {
    run_maintenance_turn(game).expect("maintenance should succeed");
}

fn production(game: &CoreGameData) -> f64 {
    game.planets[0].production.to_f64()
}

fn goods(game: &CoreGameData) -> f64 {
    game.planets[0].stored_goods.to_f64()
}

#[test]
fn the_70_and_71_percent_equilibria_match_the_model_and_the_hidden_optimum_is_gone() {
    // accounting_review.equilibrium(100, tax): production, then income.
    for (tax, production_model, income_model) in [
        (70u8, 88.23529411764706, 61.76470588235294),
        (71, 85.79881656804734, 60.917159763313606),
    ] {
        let mut game = single_world(tax, 100, 100.0, 0.0);
        for _ in 0..400 {
            // The economy ignores the year; keep it inside the campaign range.
            game.conquest.game_year = 3000;
            pass(&mut game);
        }
        assert_close(production(&game), production_model, "settled production");
        let income = production(&game) * f64::from(tax) / 100.0;
        assert_close(income, income_model, "settled income");

        // The settled value is a fixed point of the pass.
        let before = goods(&game);
        pass(&mut game);
        assert_close(production(&game), production_model, "fixed point");
        assert_close(goods(&game) - before, income_model, "income per pass");
    }
}

#[test]
fn growth_has_no_yearly_minimum_and_nothing_is_rounded_between_passes() {
    // Potential 100 at 50% tax from 99: the gap closes by 1/8 a pass,
    // 99 + 0.125, not a whole point.
    let mut game = single_world(50, 100, 99.0, 0.0);
    pass(&mut game);
    assert_close(production(&game), 99.125, "smooth growth");
    assert_close(goods(&game), 99.125 * 0.5, "fractional income");
}

/// (production, stored goods, budget) after each of ten passes at 50% tax from
/// a colony's head start `min(3, potential)`: accounting_review.trajectory
/// ('smooth', k, [50] * 10, start=min(3, k)).
const HEAD_START_ROWS: [(u8, [(f64, f64, u32); 10]); 3] = [
    (
        10,
        [
            (3.875000000, 1.937500000, 1),
            (4.640625000, 4.257812500, 4),
            (5.310546875, 6.913085938, 5),
            (5.896728516, 9.861450195, 6),
            (6.409637451, 13.066268921, 6),
            (6.858432770, 16.495485306, 7),
            (7.251128674, 20.121049643, 7),
            (7.594737589, 23.918418437, 8),
            (7.895395391, 27.866116133, 8),
            (8.158470967, 31.945351616, 8),
        ],
    ),
    (
        20,
        [
            (5.125000000, 2.562500000, 2),
            (6.984375000, 6.054687500, 6),
            (8.611328125, 10.360351562, 9),
            (10.034912109, 15.377807617, 10),
            (11.280548096, 21.018081665, 11),
            (12.370479584, 27.203321457, 12),
            (13.324169636, 33.865406275, 13),
            (14.158648431, 40.944730490, 14),
            (14.888817377, 48.389139179, 15),
            (15.527715205, 56.152996782, 16),
        ],
    ),
    (
        100,
        [
            (15.125000000, 7.562500000, 7),
            (25.734375000, 20.429687500, 20),
            (35.017578125, 37.938476562, 35),
            (43.140380859, 59.508666992, 43),
            (50.247833252, 84.632583618, 50),
            (56.466854095, 112.866010666, 56),
            (61.908497334, 143.820259333, 62),
            (66.669935167, 177.155226916, 67),
            (70.836193271, 212.573323552, 71),
            (74.481669112, 249.814158108, 74),
        ],
    ),
];

#[test]
fn colonies_with_the_head_start_follow_the_models_ten_pass_rows() {
    for (potential, rows) in HEAD_START_ROWS {
        let mut game = single_world(50, potential, f64::from(potential.min(3)), 0.0);
        for (pass_number, (production_model, goods_model, budget_model)) in
            rows.into_iter().enumerate()
        {
            pass(&mut game);
            let what = format!("potential {potential} pass {}", pass_number + 1);
            assert_close(production(&game), production_model, &what);
            assert_close(goods(&game), goods_model, &what);
            assert_eq!(
                game.planet_build_budget(1).expect("budget"),
                budget_model,
                "{what}: budget"
            );
        }
    }
}

#[test]
fn spending_takes_whole_points_and_leaves_the_fraction_in_the_treasury() {
    // accounting_review: goods 4.9, budget 4, 0.9 left. No tax, so no income.
    let mut game = single_world(0, 100, 100.0, 4.9);
    game.append_planet_build_units(1, ProductionItemKind::Destroyer, 1)
        .expect("order");
    assert_eq!(game.planet_build_budget(1).expect("budget"), 4);
    pass(&mut game);
    assert_eq!(game.planets[0].build_queue[0].points_owed, 1);
    assert_close(goods(&game), 0.9, "fraction carried");
    assert_eq!(stored_production_points(&game.planets[0]), 0);
}

#[test]
fn a_debt_stays_negative_buys_nothing_and_income_pays_it_down() {
    // Production 100 at 50% tax earns 50 a pass.
    let mut game = single_world(50, 100, 100.0, -73.0);
    assert_eq!(stored_production_points(&game.planets[0]), 0);
    assert_eq!(game.planet_build_budget(1).expect("budget"), 0);

    pass(&mut game);
    assert_close(goods(&game), -23.0, "debt after one pass");
    assert_eq!(stored_production_points(&game.planets[0]), 0);
    pass(&mut game);
    assert_close(goods(&game), 27.0, "debt cleared by income");
    assert_eq!(stored_production_points(&game.planets[0]), 27);
}

#[test]
fn a_queued_build_in_a_debtor_world_spends_nothing_and_forgives_nothing() {
    let mut game = single_world(0, 100, 100.0, -5.5);
    game.planets[0].build_queue[0].points_owed = 10;
    game.planets[0].build_queue[0].kind = ProductionItemKind::Destroyer;
    pass(&mut game);
    assert_eq!(game.planets[0].build_queue[0].points_owed, 10);
    assert_close(goods(&game), -5.5, "debt after a pass with no income");
}

#[test]
fn build_capacity_rounds_production_to_the_nearest_point() {
    let mut game = single_world(50, 100, 99.6, 1000.0);
    assert_eq!(game.planet_build_budget(1).expect("budget"), 100);
    game.planets[0].production = Real48::from_f64(99.4);
    assert_eq!(game.planet_build_budget(1).expect("budget"), 99);
    game.bases.push(commissioned_starbase(1, WORLD));
    assert_eq!(game.planet_build_budget(1).expect("budget"), 495);
}

// --- Starbase economics -----------------------------------------------------

/// Production after one pass from 3 of a potential-100 world with a base, by
/// tax: starbase_review.grow(3, 100, tax, 1), and without one.
const FADE_ROWS: [(u8, f64, f64); 9] = [
    (40, 24.825, 17.55),
    (50, 21.1875, 15.125),
    (51, 20.427666667, 14.8825),
    (55, 17.55, 13.9125),
    (57, 16.208166667, 13.4275),
    (60, 14.316666667, 12.7),
    (64, 12.021, 11.73),
    (65, 11.4875, 11.4875),
    (66, 11.239, 11.239),
];

fn one_pass_from_three(tax: u8, bases: Vec<Starbase>) -> f64 {
    let mut game = single_world(tax, 100, 3.0, 0.0);
    game.bases = bases;
    pass(&mut game);
    production(&game)
}

#[test]
fn the_starbase_bonus_fades_continuously_to_nothing_at_65() {
    for (tax, with_base, without_base) in FADE_ROWS {
        assert_close(
            one_pass_from_three(tax, vec![commissioned_starbase(1, WORLD)]),
            with_base,
            &format!("tax {tax} with a base"),
        );
        assert_close(
            one_pass_from_three(tax, vec![]),
            without_base,
            &format!("tax {tax} without a base"),
        );
    }
}

#[test]
fn more_bases_at_one_world_add_neither_growth_nor_capacity() {
    let one = vec![commissioned_starbase(1, WORLD)];
    let three = vec![
        commissioned_starbase(1, WORLD),
        commissioned_starbase(1, WORLD),
        commissioned_starbase(1, WORLD),
    ];
    assert_eq!(
        one_pass_from_three(50, one.clone()),
        one_pass_from_three(50, three.clone())
    );
    for bases in [one, three] {
        let mut game = single_world(50, 100, 100.0, 1000.0);
        game.bases = bases;
        assert_eq!(game.planet_build_budget(1).expect("budget"), 500);
    }
}

/// starbase_review.cycle_control(start, 100, taxes, bases=1) after 20 cycles:
/// (start production, taxes, production, income) for the cycle and, for the
/// steady 65% schedule of the same length, (production, income).
type CycleRow = (f64, &'static [u8], (f64, f64), (f64, f64));
const TAX_CYCLES: [CycleRow; 4] = [
    (
        50.0,
        &[75, 64, 64],
        (93.134624805, 3457.394254771),
        (99.794438209, 3562.464843852),
    ),
    (
        50.0,
        &[70, 64],
        (93.594781187, 2229.152211682),
        (98.716827837, 2269.769502733),
    ),
    (
        100.0,
        &[75, 64, 64],
        (93.314670584, 3803.541165805),
        (100.0, 3900.0),
    ),
    (
        100.0,
        &[70, 64],
        (94.795766308, 2565.286952585),
        (100.0, 2600.0),
    ),
];

#[test]
fn the_two_tested_tax_cycles_fall_behind_a_steady_65_with_a_base_present() {
    for (start, cycle, (cycle_production, cycle_income), (steady_production, steady_income)) in
        TAX_CYCLES
    {
        let run = |taxes: Vec<u8>| {
            let mut game = single_world(taxes[0], 100, start, 0.0);
            game.bases = vec![commissioned_starbase(1, WORLD)];
            for tax in taxes {
                game.players[0].tax_rate = tax;
                pass(&mut game);
            }
            (production(&game), goods(&game))
        };
        let cycles: Vec<u8> = cycle
            .iter()
            .copied()
            .cycle()
            .take(cycle.len() * 20)
            .collect();
        let (cycle_end, cycle_goods) = run(cycles.clone());
        let (steady_end, steady_goods) = run(vec![65; cycles.len()]);
        let what = format!("start {start} cycle {cycle:?}");
        assert_close(cycle_end, cycle_production, &what);
        assert_close(cycle_goods, cycle_income, &what);
        assert_close(steady_end, steady_production, &what);
        assert_close(steady_goods, steady_income, &what);
        assert!(cycle_goods < steady_goods, "{what}: cycling loses");
    }
}

// --- Scorch Earth -----------------------------------------------------------

/// The decay factors the order can draw: `0.30 + Random(20) / 100`.
fn scorch_factor_index(before: f64, after: f64) -> usize {
    let factor = after / before;
    let index = ((factor - 0.30) * 100.0).round();
    assert!(
        (0.0..20.0).contains(&index) && (factor - (0.30 + index / 100.0)).abs() < 1e-9,
        "{factor} is not one of the twenty factors"
    );
    index as usize
}

fn scorched_world(production: f64, goods: f64, potential: u8) -> CoreGameData {
    let mut game = single_world(50, potential, production, goods);
    game.planets[0].scorch_earth = true;
    game
}

#[test]
fn a_scorched_world_decays_instead_of_growing_or_earning() {
    // scorch_review.yearly: P -> P * (0.30 + roll / 100), 500 goods -> sqrt(500).
    let mut game = scorched_world(100.0, 500.0, 100);
    run_maintenance_turn_with_seed(&mut game, 7).expect("maintenance");
    scorch_factor_index(100.0, production(&game));
    assert_close(
        goods(&game),
        500f64.sqrt(),
        "positive goods above 100 take the root",
    );
    assert_eq!(
        game.planets[0].potential_production, 100,
        "potential is kept"
    );
    assert!(game.planets[0].scorch_earth, "the order stays set");
}

#[test]
fn scorch_goods_branches_match_the_model() {
    for (before, after) in [
        (50.0, 5.0),
        (100.0, 10.0),
        (35.0, 3.5),
        (500.0, 500f64.sqrt()),
        (0.0, 0.0),
    ] {
        let mut game = scorched_world(100.0, before, 100);
        pass(&mut game);
        assert_close(goods(&game), after, &format!("goods {before}"));
    }
}

#[test]
fn scorch_leaves_a_debt_alone() {
    for debt in [-100.0, -1.0] {
        let mut game = scorched_world(100.0, debt, 100);
        pass(&mut game);
        assert_close(goods(&game), debt, "debt untouched");
    }
}

#[test]
fn scorch_floors_production_at_one_and_never_invents_potential() {
    let mut game = scorched_world(1.0, 0.0, 10);
    pass(&mut game);
    assert_close(production(&game), 1.0, "floored at one");

    // 2 * 0.30..0.49 is under 1, so the floor lifts it to 1.
    let mut game = scorched_world(2.0, 0.0, 10);
    pass(&mut game);
    assert_close(production(&game), 1.0, "floored at one");

    let mut game = scorched_world(0.0, 0.0, 0);
    pass(&mut game);
    assert_eq!(
        production(&game),
        0.0,
        "a world with no potential stays at 0"
    );
}

#[test]
fn scorch_draws_all_twenty_factors_from_the_campaign_seed_and_repeats() {
    let mut seen = BTreeSet::new();
    for seed in 0..400u64 {
        let mut game = scorched_world(100.0, 0.0, 100);
        run_maintenance_turn_with_seed(&mut game, seed).expect("maintenance");
        seen.insert(scorch_factor_index(100.0, production(&game)));

        let mut again = scorched_world(100.0, 0.0, 100);
        run_maintenance_turn_with_seed(&mut again, seed).expect("maintenance");
        assert_eq!(
            production(&game),
            production(&again),
            "same seed, same roll"
        );
    }
    assert_eq!(seen.len(), 20, "every factor 0.30 to 0.49 comes up");
}

#[test]
fn rescinding_scorch_resumes_smooth_recovery_at_the_models_pace() {
    // scorch_review.recover at 50% tax: a world at P recovers
    // k - (k - P) * (7/8)^n, converting nothing.
    let mut game = scorched_world(79.0 / 4.0, 0.0, 100);
    game.planets[0].scorch_earth = false;
    for _ in 0..20 {
        pass(&mut game);
    }
    let expected = 100.0 - (100.0 - 79.0 / 4.0) * (7.0f64 / 8.0).powi(20);
    assert_close(production(&game), expected, "recovery after rescinding");
}

#[test]
fn the_owner_is_told_each_year_that_scorch_earth_is_in_force() {
    let mut game = scorched_world(100.0, 500.0, 100);
    set_planet_name(&mut game.planets[0], "Cinder");
    let events = run_maintenance_turn(&mut game).expect("maintenance");
    assert_eq!(events.planet_scorch_events.len(), 1);

    let rows = build_results_report_blocks(&game, &events);
    let text = rows
        .iter()
        .filter(|row| row.is_visible_to_viewer(1))
        .map(|row| row.decoded_text.replace('\n', " "))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(text.contains("planet \"Cinder\""), "{text}");
    assert!(text.contains("Scorch-Earth is in force."), "{text}");
    assert!(text.contains("Our production fell from 100 to"), "{text}");
    assert!(text.contains("stored points from 500 to 22"), "{text}");
    assert!(text.contains("until you rescind it"), "{text}");

    // Once rescinded the world reports no decay.
    game.planets[0].scorch_earth = false;
    let events = run_maintenance_turn(&mut game).expect("maintenance");
    assert!(events.planet_scorch_events.is_empty());
}

#[test]
fn revenue_fields_forecast_the_next_pass_in_whole_points() {
    // 50 grows to 56.25 at 50% tax and earns 28.125: shown as 28, growth 6.
    let mut game = single_world(50, 100, 50.0, 0.0);
    let row = game.empire_planet_economy_rows(1).remove(0);
    assert_eq!(row.yearly_tax_revenue, 28);
    assert_eq!(row.yearly_growth_delta, 6);
    assert_eq!(game.empire_available_production_points(1), 28);
    assert_eq!(game.planet_forecast_revenue(&game.planets[0]), 28);

    // The forecast is what the pass then credits (to the nearest point).
    pass(&mut game);
    assert_close(goods(&game), 28.125, "credited revenue");

    // A starbase that will stay at the world lifts the forecast too:
    // 50 grows to 59.375 and earns 29.69.
    let mut based = single_world(50, 100, 50.0, 0.0);
    based.bases = vec![commissioned_starbase(1, WORLD)];
    let row = based.empire_planet_economy_rows(1).remove(0);
    assert_eq!(row.yearly_tax_revenue, 30);
    assert_eq!(row.yearly_growth_delta, 9);

    // A fully developed world earns 50 and grows nothing.
    let full = single_world(50, 100, 100.0, 0.0);
    let row = full.empire_planet_economy_rows(1).remove(0);
    assert_eq!((row.yearly_tax_revenue, row.yearly_growth_delta), (50, 0));
}

#[test]
fn revenue_forecast_shows_zero_for_scorched_and_converting_worlds() {
    let scorched = scorched_world(100.0, 500.0, 100);
    assert_eq!(scorched.empire_available_production_points(1), 0);
    let row = scorched.empire_planet_economy_rows(1).remove(0);
    assert_eq!((row.yearly_tax_revenue, row.yearly_growth_delta), (0, 0));

    let mut converting = single_world(50, 100, 100.0, 0.0);
    converting.planets[0].years_under_owner = 1;
    assert_eq!(converting.empire_available_production_points(1), 0);
}
