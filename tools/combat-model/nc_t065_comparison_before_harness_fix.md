# Combat odds sweep

Seed 20261010; 20000 trials per sampled case; exact enumeration up to 0 (state, round) pairs. Percentages without an interval are exact; intervals are 95%.

## Rule sets

| rule set | effect | mixed_shift | base_condition | execution_classes | crippled_ds | remainder | critical | screen | roe | withdrawal |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| engine-t065 | flat | 1 | present | DD SB | full | carry | free-kill | absolute | after1 | speed-pursuit |

## Left win rate

| case | engine-t065 |
| --- | --- |
| 10 CA vs 10 CA (dd-10-8-5) | 50.1% [49.4, 50.8] |
| 11 CA vs 10 CA (dd-10-8-5) | 65.6% [64.9, 66.3] |
| 15 CA vs 10 CA (dd-10-8-5) | 96.6% [96.3, 96.8] |
| 20 CA vs 10 CA (dd-10-8-5) | 100.0% [100.0, 100.0] |
| 30 DD vs 10 CA (dd-10-8-5) | 31.5% [30.9, 32.2] |
| 90 DD vs 10 BB (dd-10-8-5) | 23.4% [22.8, 24.0] |
| 30 CA vs 10 BB (dd-10-8-5) | 39.6% [39.0, 40.3] |
| 29 CA + 3 DD vs 30 CA (dd-10-8-5) | 58.7% [58.0, 59.4] |
| 10 CA ROE 6 vs 15 CA (dd-10-8-5) | 0.0% [0.0, 0.0] |

## Mean exchanges

| case | engine-t065 |
| --- | --- |
| 10 CA vs 10 CA (dd-10-8-5) | 4.60 |
| 11 CA vs 10 CA (dd-10-8-5) | 4.48 |
| 15 CA vs 10 CA (dd-10-8-5) | 3.32 |
| 20 CA vs 10 CA (dd-10-8-5) | 2.12 |
| 30 DD vs 10 CA (dd-10-8-5) | 3.88 |
| 90 DD vs 10 BB (dd-10-8-5) | 3.97 |
| 30 CA vs 10 BB (dd-10-8-5) | 4.97 |
| 29 CA + 3 DD vs 30 CA (dd-10-8-5) | 4.71 |
| 10 CA ROE 6 vs 15 CA (dd-10-8-5) | 1.64 |

## Left losses, destroyed + crippled build points

| case | engine-t065 |
| --- | --- |
| 10 CA vs 10 CA (dd-10-8-5) | 93.3 + 47.7 |
| 11 CA vs 10 CA (dd-10-8-5) | 79.9 + 68.8 |
| 15 CA vs 10 CA (dd-10-8-5) | 25.3 + 129.3 |
| 20 CA vs 10 CA (dd-10-8-5) | 4.3 + 133.5 |
| 30 DD vs 10 CA (dd-10-8-5) | 116.0 + 30.4 |
| 90 DD vs 10 BB (dd-10-8-5) | 379.4 + 65.3 |
| 30 CA vs 10 BB (dd-10-8-5) | 321.8 + 114.0 |
| 29 CA + 3 DD vs 30 CA (dd-10-8-5) | 245.3 + 170.9 |
| 10 CA ROE 6 vs 15 CA (dd-10-8-5) | 66.1 + 63.2 |

## Right losses, destroyed + crippled build points

| case | engine-t065 |
| --- | --- |
| 10 CA vs 10 CA (dd-10-8-5) | 94.0 + 47.2 |
| 11 CA vs 10 CA (dd-10-8-5) | 112.4 + 33.1 |
| 15 CA vs 10 CA (dd-10-8-5) | 147.3 + 2.7 |
| 20 CA vs 10 CA (dd-10-8-5) | 150.0 + 0.0 |
| 30 DD vs 10 CA (dd-10-8-5) | 99.3 + 0.0 |
| 90 DD vs 10 BB (dd-10-8-5) | 269.7 + 0.0 |
| 30 CA vs 10 BB (dd-10-8-5) | 240.1 + 171.3 |
| 29 CA + 3 DD vs 30 CA (dd-10-8-5) | 319.1 + 115.9 |
| 10 CA ROE 6 vs 15 CA (dd-10-8-5) | 4.3 + 115.7 |

## Detail: engine-t065

| case | method | left win | right win | mutual | stalemate | exchanges | left lost | right lost | left crippled | right crippled | left ships lost | right ships lost | withdrew L / R |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 10 CA vs 10 CA (dd-10-8-5) | sampled 20000 | 50.1% [49.4, 50.8] | 49.3% [48.6, 50.0] | 0.5% [0.4, 0.6] | 0.0% [0.0, 0.0] | 4.60 | 93.3 | 94.0 | 47.7 | 47.2 | CA 6.2 | CA 6.3 | 0.0% / 0.0% |
| 11 CA vs 10 CA (dd-10-8-5) | sampled 20000 | 65.6% [64.9, 66.3] | 33.9% [33.2, 34.6] | 0.5% [0.4, 0.6] | 0.0% [0.0, 0.0] | 4.48 | 79.9 | 112.4 | 68.8 | 33.1 | CA 5.3 | CA 7.5 | 0.0% / 0.0% |
| 15 CA vs 10 CA (dd-10-8-5) | sampled 20000 | 96.6% [96.3, 96.8] | 3.3% [3.1, 3.6] | 0.1% [0.1, 0.1] | 0.0% [0.0, 0.0] | 3.32 | 25.3 | 147.3 | 129.3 | 2.7 | CA 1.7 | CA 9.8 | 0.0% / 0.0% |
| 20 CA vs 10 CA (dd-10-8-5) | sampled 20000 | 100.0% [100.0, 100.0] | 0.0% [0.0, 0.0] | 0.0% [0.0, 0.0] | 0.0% [0.0, 0.0] | 2.12 | 4.3 | 150.0 | 133.5 | 0.0 | CA 0.3 | CA 10.0 | 0.0% / 0.0% |
| 30 DD vs 10 CA (dd-10-8-5) | sampled 20000 | 31.5% [30.9, 32.2] | 67.8% [67.1, 68.4] | 0.7% [0.6, 0.9] | 0.0% [0.0, 0.0] | 3.88 | 116.0 | 99.3 | 30.4 | 0.0 | DD 23.2 | CA 6.6 | 0.0% / 0.0% |
| 90 DD vs 10 BB (dd-10-8-5) | sampled 20000 | 23.4% [22.8, 24.0] | 76.0% [75.4, 76.6] | 0.6% [0.5, 0.7] | 0.0% [0.0, 0.0] | 3.97 | 379.4 | 269.7 | 65.3 | 0.0 | DD 75.9 | BB 6.0 | 0.0% / 0.0% |
| 30 CA vs 10 BB (dd-10-8-5) | sampled 20000 | 39.6% [39.0, 40.3] | 60.2% [59.5, 60.8] | 0.2% [0.1, 0.3] | 0.0% [0.0, 0.0] | 4.97 | 321.8 | 240.1 | 114.0 | 171.3 | CA 21.5 | BB 5.3 | 0.0% / 0.0% |
| 29 CA + 3 DD vs 30 CA (dd-10-8-5) | sampled 20000 | 58.7% [58.0, 59.4] | 41.2% [40.5, 41.8] | 0.1% [0.1, 0.2] | 0.0% [0.0, 0.0] | 4.71 | 245.3 | 319.1 | 170.9 | 115.9 | DD 2.3 CA 15.6 | CA 21.3 | 0.0% / 0.0% |
| 10 CA ROE 6 vs 15 CA (dd-10-8-5) | sampled 20000 | 0.0% [0.0, 0.0] | 100.0% [100.0, 100.0] | 0.0% [0.0, 0.0] | 0.0% [0.0, 0.0] | 1.64 | 66.1 | 4.3 | 63.2 | 115.7 | CA 4.4 | CA 0.3 | 100.0% / 0.0% |

## Model (engine-t065) against the engine

| case | model left win | engine left win [95%] | agree | model mutual | engine mutual | model lost L / R | engine lost L / R | years per battle |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 10 CA vs 10 CA | 50.1% [49.4, 50.8] | 50.6% [49.1, 52.2] n=4000 | yes | 0.5% | 0.4% | 93.3 / 94.0 | 91.9 / 95.9 | 1.22 |
| 11 CA vs 10 CA | 65.6% [64.9, 66.3] | 67.5% [66.0, 68.9] n=4000 | yes | 0.5% | 0.3% | 79.9 / 112.4 | 77.2 / 115.2 | 1.21 |
| 15 CA vs 10 CA | 96.6% [96.3, 96.8] | 96.9% [96.3, 97.4] n=4000 | yes | 0.1% | 0.0% | 25.3 / 147.3 | 23.2 / 147.4 | 1.18 |
| 20 CA vs 10 CA | 100.0% [100.0, 100.0] | 100.0% [99.9, 100.0] n=4000 | yes | 0.0% | 0.0% | 4.3 / 150.0 | 4.1 / 150.0 | 1.14 |
| 30 DD vs 10 CA | 31.5% [30.9, 32.2] | 31.8% [30.4, 33.3] n=4000 | yes | 0.7% | 0.5% | 116.0 / 99.3 | 115.6 / 100.3 | 1.18 |
| 90 DD vs 10 BB | 23.4% [22.8, 24.0] | 24.1% [22.8, 25.5] n=4000 | yes | 0.6% | 0.4% | 379.4 / 269.7 | 377.1 / 273.0 | 1.06 |
| 30 CA vs 10 BB | 39.6% [39.0, 40.3] | 41.0% [39.5, 42.5] n=4000 | yes | 0.2% | 0.3% | 321.8 / 240.1 | 316.1 / 246.4 | 1.09 |
| 29 CA + 3 DD vs 30 CA | 58.7% [58.0, 59.4] | 58.5% [56.9, 60.0] n=4000 | yes | 0.1% | 0.0% | 245.3 / 319.1 | 243.6 / 320.3 | 1.05 |
| 10 CA ROE 6 vs 15 CA | 0.0% [0.0, 0.0] | 0.0% [0.0, 0.1] n=4000 | NO: right win, stalemate | 0.0% | 0.0% | 66.1 / 4.3 | 63.9 / 4.2 | 1.17 |

Agree checks left win, right win, mutual and stalemate: an exact model value when the engine count passes a two-sided exact binomial test against it at 5%, a sampled one when the two 95% intervals overlap; and destroyed build points per side within 2% of that side's starting build points. A case missing on either side, or without a battle, is listed with the reason.
