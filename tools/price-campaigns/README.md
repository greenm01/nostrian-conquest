# Hostile-map price controls (t090)

The Rust example plays scripted policies through real orders and maintenance
on generated maps. It is an analysis tool, not the game's AI. The
[investigation](../../docs/notes/investigations/n7fzd2vk-unit-prices-on-generated-maps-with-hostile-openings.md)
explains the policy, findings and limits. No gameplay price changes ship here.

From `rust/`, run one cell (use a fresh output directory; never clear existing evidence):

```sh
cargo-slot nostrian-conquest cargo run --release -p nc-harness --example price_campaigns -- --label base --players 4 --seeds 0..60 --years 80 --opening escort --doctrine value --goods 50 --threads 30 > /tmp/base_4p_escort_value.tsv
```

The recorded matrix has six labels: base, goods35, etac15, etac25, bb30,
bb60. Each runs 60 seeds for every combination of 2/4/6 players,
bare/escort openings and value/fixed doctrines; 150 seeds of four-player
mixed openings under each doctrine; and 150 seeds of four-player escort
openings with mixed value/cruiser doctrines. This is 1,170 games per label,
7,020 total. The engine/data base is `7348955f`; the identical example
source in all variants has the SHA-256 recorded in provenance.json.

For a price variant, use an isolated worktree at that engine revision and
change only the relevant build-cost constants in
`rust/nc-data/src/model/planet.rs` and `rust/nc-engine/src/orders/build.rs`.
Copy the example source into it and build through cargo-slot from its own
`rust/` directory. ETAC alternatives are 15/25, BB alternatives 30/60.
Starting goods use the base binary with `--goods 35`. Remove each worktree
and its cargo pool after collecting evidence. Do not merge experimental
prices. Do not copy built binaries into evidence.

The matrix predates t099. Running it on newer main tests a different engine.
Map seeds vary, but ScenarioPlay's maintenance campaign seed is zero;
these are map controls, not repeated independent combat seeds per map.
Both seats' orders can inspect the whole map. Most campaigns do not finish
within 80 years, so final worlds and elapsed time must not be treated as
human win rates or complete game lengths.

From the repository root, regenerate summaries from the TSVs:

```sh
python3 tools/price-campaigns/summarize.py --json tools/price-campaigns/results.json /tmp/price-runs/*.tsv > tools/price-campaigns/results.md
python3 tools/price-campaigns/doctrine_controls.py /tmp/price-runs/*.tsv > tools/price-campaigns/doctrine_controls.json
python3 -m unittest discover -s tools/price-campaigns -p 'test_*.py'
```

The JSON summary checks row widths, unique campaign seeds and complete
seat sets. The bootstrap samples whole campaigns, preserving correlations
between the four seats. Results carry conditional means and the share of
campaigns/seats that reached each dated event; `null` means none did.
An empire's `built_*` counts are units ordered, not completed hulls.
The provenance file records raw-output and binary hashes. Raw TSVs belong
in an external evidence archive; the repository keeps aggregate results
and the paired doctrine differences needed to reproduce their intervals.
