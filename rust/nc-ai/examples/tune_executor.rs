//! Tune the executor's weights by evolution (t154).
//!
//! Each generation mutates the elite into a population. Every candidate
//! plays one seat against three seats of the current champion, rotating
//! seats over a fresh block of maps, and scores its mean win share plus
//! production share. As one of four equal seats it expects 0.5. A candidate
//! that clears the champion by `--margin` in its generation becomes the
//! champion, so later candidates face a stronger field.
//!
//! ```text
//! cargo run --release -p nc-ai --example tune_executor -- \
//!     [--generations 30] [--population 24] [--games 16] [--years 60] [--margin 0.1] \
//!     [--personality rusher] [--against-default] [--with pickets] [--out tuned.kdl]
//! ```
//!
//! `--personality` tunes a style (t167), built in or a `.kdl` file, from
//! its profile, within its bounds. `--against-default` scores candidates against three seats of the
//! default style instead of the champion, and the generation's best becomes
//! the champion: a style is tuned to hold its own in a mixed game, not to
//! beat its own kind.
//!
//! `--with` turns doctrine switches on for the style being tuned only.
//!
//! It prints each generation's best and the champion as a `Weights` literal.

use std::thread;

use nc_ai::{
    Personality, SeatPolicy, Weights, personality, personality_kdl, personality_or_file, play_match,
};

const PLAYERS: usize = 4;

/// Seeded splitmix64.
struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A standard normal draw (Box-Muller).
    fn normal(&mut self) -> f32 {
        let u = self.unit().max(1e-7);
        let v = self.unit();
        (-2.0 * u.ln()).sqrt() * (std::f32::consts::TAU * v).cos()
    }
}

fn mutate(parent: Weights, style: &Personality, sigma: f32, rng: &mut Rng) -> Weights {
    let mut values = parent.to_array();
    for (idx, value) in values.iter_mut().enumerate() {
        let (lo, hi) = style.ranges[idx];
        *value += rng.normal() * sigma * (hi - lo);
    }
    style.weights_from_array(values)
}

/// Mean win share plus production share of `candidate` against three
/// `champion` seats on `maps`, the candidate's seat rotating.
fn fitness(candidate: Weights, champion: Weights, maps: &[u64], years: u16) -> f32 {
    let total: f32 = maps
        .iter()
        .map(|&map| {
            let seat = (map as usize) % PLAYERS;
            let mut policies = [SeatPolicy::Executor(champion); PLAYERS];
            policies[seat] = SeatPolicy::Executor(candidate);
            let result = play_match(map, &policies, years);
            let empire = seat as u8 + 1;
            result.win_share(empire) + result.production_share(empire)
        })
        .sum();
    total / maps.len() as f32
}

fn evaluate(population: &[Weights], champion: Weights, maps: &[u64], years: u16) -> Vec<f32> {
    let threads = thread::available_parallelism().map_or(1, |n| n.get());
    let mut scores = vec![0.0; population.len()];
    thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|worker| {
                scope.spawn(move || {
                    (worker..population.len())
                        .step_by(threads)
                        .map(|idx| (idx, fitness(population[idx], champion, maps, years)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for worker in workers {
            for (idx, score) in worker.join().expect("worker should finish") {
                scores[idx] = score;
            }
        }
    });
    scores
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |name: &str, default: f32| {
        args.iter()
            .position(|arg| arg == name)
            .map_or(default, |idx| args[idx + 1].parse().expect(name))
    };
    let generations = arg("--generations", 30.0) as u32;
    let population_size = arg("--population", 24.0) as usize;
    let games = arg("--games", 16.0) as u64;
    let years = arg("--years", 60.0) as u16;
    let margin = arg("--margin", 0.1);
    let elite_size = (population_size / 4).max(1);
    let named = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .map(|idx| args[idx + 1].clone())
    };
    let style = named("--personality")
        .map(|name| personality_or_file(&name).unwrap_or_else(|err| panic!("{err}")));
    let against_default = args.iter().any(|arg| arg == "--against-default");
    let default = personality("land-grab").expect("the default style is built in");

    let mut rng = Rng(0x7E5_EC0);
    let mut champion = style
        .as_ref()
        .map_or(Weights::BASELINE, |style| style.weights);
    let mut style = style.unwrap_or(Personality {
        ranges: Weights::RANGES,
        ..default.clone()
    });
    // Doctrine switches the tuned style runs with, as in executor_matches.
    for switch in named("--with").iter().flat_map(|names| names.split(',')) {
        style
            .weights
            .doctrine
            .enable(switch)
            .expect("a known doctrine switch");
    }
    champion.doctrine = style.weights.doctrine;
    let mut elite = vec![champion];
    for generation in 0..generations {
        let sigma = 0.15 * (1.0 - generation as f32 / generations as f32).max(0.2);
        let mut population = elite.clone();
        while population.len() < population_size {
            let parent = elite[population.len() % elite.len()];
            population.push(mutate(parent, &style, sigma, &mut rng));
        }
        let first = 1_000 + u64::from(generation) * games;
        let maps: Vec<u64> = (first..first + games).collect();
        let opponent = if against_default {
            default.weights
        } else {
            champion
        };
        let scores = evaluate(&population, opponent, &maps, years);
        let mut ranked: Vec<usize> = (0..population.len()).collect();
        ranked.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
        elite = ranked[..elite_size]
            .iter()
            .map(|&idx| population[idx])
            .collect();
        let best = ranked[0];
        let promoted = against_default || scores[best] >= 0.5 + margin;
        if promoted {
            champion = population[best];
        }
        println!(
            "generation {generation}: best {:.3}, median {:.3}{}",
            scores[best],
            scores[ranked[ranked.len() / 2]],
            if promoted { ", new champion" } else { "" }
        );
    }
    println!("champion: {champion:?}");
    if let Some(path) = named("--out") {
        std::fs::write(&path, personality_kdl("tuned", &champion)).expect("--out is writable");
    }
}
