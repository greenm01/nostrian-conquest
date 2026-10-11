# Combat odds sweep

Seed 20261010; 20000 trials per sampled case; exact enumeration up to 100000 (state, round) pairs. Percentages without an interval are exact; intervals are 95%.

## Rule sets

| rule set | effect | mixed_shift | base_condition | execution_classes | crippled_ds | remainder | critical | screen | roe | withdrawal |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| engine-t060 | flat | 1 | present | DD SB | full | carry | free-kill | absolute | lock3 | pressed |

## Left win rate

| case | engine-t060 |
| --- | --- |
| 10 CA vs 10 CA | 49.4% [48.7, 50.1] |
| 11 CA vs 10 CA | 65.2% [64.6, 65.9] |
| 15 CA vs 10 CA | 96.8% [96.6, 97.1] |
| 20 CA vs 10 CA | 100.0% |
| 30 DD vs 10 CA | 9.8% |
| 90 DD vs 10 BB | 5.4% [5.1, 5.7] |
| 30 CA vs 10 BB | 40.3% [39.6, 41.0] |
| 29 CA + 3 DD vs 30 CA | 55.0% [54.3, 55.7] |
| 10 CA ROE 6 vs 15 CA | 2.5% |

## Mean exchanges

| case | engine-t060 |
| --- | --- |
| 10 CA vs 10 CA | 4.58 |
| 11 CA vs 10 CA | 4.48 |
| 15 CA vs 10 CA | 3.32 |
| 20 CA vs 10 CA | 2.12 |
| 30 DD vs 10 CA | 2.31 |
| 90 DD vs 10 BB | 2.27 |
| 30 CA vs 10 BB | 4.97 |
| 29 CA + 3 DD vs 30 CA | 4.70 |
| 10 CA ROE 6 vs 15 CA | 3.06 |

## Left losses, destroyed + crippled build points

| case | engine-t060 |
| --- | --- |
| 10 CA vs 10 CA | 94.4 + 46.9 |
| 11 CA vs 10 CA | 79.8 + 68.4 |
| 15 CA vs 10 CA | 25.3 + 130.0 |
| 20 CA vs 10 CA | 4.2 + 133.2 |
| 30 DD vs 10 CA | 141.6 + 8.1 |
| 90 DD vs 10 BB | 436.9 + 12.9 |
| 30 CA vs 10 BB | 318.2 + 117.7 |
| 29 CA + 3 DD vs 30 CA | 258.5 + 161.8 |
| 10 CA ROE 6 vs 15 CA | 139.6 + 10.4 |

## Right losses, destroyed + crippled build points

| case | engine-t060 |
| --- | --- |
| 10 CA vs 10 CA | 92.7 + 47.8 |
| 11 CA vs 10 CA | 112.2 + 33.3 |
| 15 CA vs 10 CA | 147.3 + 2.6 |
| 20 CA vs 10 CA | 150.0 + 0.0 |
| 30 DD vs 10 CA | 71.3 + 0.0 |
| 90 DD vs 10 BB | 187.6 + 0.0 |
| 30 CA vs 10 BB | 242.2 + 170.1 |
| 29 CA + 3 DD vs 30 CA | 305.5 + 125.6 |
| 10 CA ROE 6 vs 15 CA | 21.6 + 132.7 |

## Detail: engine-t060

| case | method | left win | right win | mutual | stalemate | exchanges | left lost | right lost | left crippled | right crippled | left ships lost | right ships lost | withdrew L / R |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 10 CA vs 10 CA | sampled 20000 | 49.4% [48.7, 50.1] | 50.1% [49.4, 50.8] | 0.5% [0.4, 0.7] | 0.0% [0.0, 0.0] | 4.58 | 94.4 | 92.7 | 46.9 | 47.8 | CA 6.3 | CA 6.2 | 0.0% / 0.0% |
| 11 CA vs 10 CA | sampled 20000 | 65.2% [64.6, 65.9] | 34.3% [33.7, 35.0] | 0.4% [0.3, 0.5] | 0.0% [0.0, 0.0] | 4.48 | 79.8 | 112.2 | 68.4 | 33.3 | CA 5.3 | CA 7.5 | 0.0% / 0.0% |
| 15 CA vs 10 CA | sampled 20000 | 96.8% [96.6, 97.1] | 3.1% [2.9, 3.3] | 0.1% [0.0, 0.1] | 0.0% [0.0, 0.0] | 3.32 | 25.3 | 147.3 | 130.0 | 2.6 | CA 1.7 | CA 9.8 | 0.0% / 0.0% |
| 20 CA vs 10 CA | exact | 100.0% | 0.0% | 0.0% | 0.0% | 2.12 | 4.2 | 150.0 | 133.2 | 0.0 | CA 0.3 | CA 10.0 | 0.0% / 0.0% |
| 30 DD vs 10 CA | exact | 9.8% | 89.6% | 0.6% | 0.0% | 2.31 | 141.6 | 71.3 | 8.1 | 0.0 | DD 28.3 | CA 4.8 | 0.0% / 0.0% |
| 90 DD vs 10 BB | sampled 20000 | 5.4% [5.1, 5.7] | 94.2% [93.9, 94.6] | 0.4% [0.3, 0.5] | 0.0% [0.0, 0.0] | 2.27 | 436.9 | 187.6 | 12.9 | 0.0 | DD 87.4 | BB 4.2 | 0.0% / 0.0% |
| 30 CA vs 10 BB | sampled 20000 | 40.3% [39.6, 41.0] | 59.4% [58.8, 60.1] | 0.3% [0.2, 0.4] | 0.0% [0.0, 0.0] | 4.97 | 318.2 | 242.2 | 117.7 | 170.1 | CA 21.2 | BB 5.4 | 0.0% / 0.0% |
| 29 CA + 3 DD vs 30 CA | sampled 20000 | 55.0% [54.3, 55.7] | 44.9% [44.2, 45.6] | 0.1% [0.1, 0.2] | 0.0% [0.0, 0.0] | 4.70 | 258.5 | 305.5 | 161.8 | 125.6 | DD 2.3 CA 16.5 | CA 20.4 | 0.0% / 0.0% |
| 10 CA ROE 6 vs 15 CA | exact | 2.5% | 97.5% | 0.0% | 0.0% | 3.06 | 139.6 | 21.6 | 10.4 | 132.7 | CA 9.3 | CA 1.4 | 31.9% / 0.0% |

## Model (engine-t060) against the engine

| case | model left win | engine left win [95%] | agree | model mutual | engine mutual | model lost L / R | engine lost L / R | years per battle |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 10 CA vs 10 CA | 49.4% [48.7, 50.1] | 49.7% [49.0, 50.4] n=19998 | yes | 0.5% | 0.5% | 94.4 / 92.7 | 93.3 / 93.8 | 1.21 |
| 11 CA vs 10 CA | 65.2% [64.6, 65.9] | 65.8% [65.1, 66.4] n=20000 | yes | 0.4% | 0.4% | 79.8 / 112.2 | 79.0 / 113.0 | 1.20 |
| 15 CA vs 10 CA | 96.8% [96.6, 97.1] | 96.8% [96.6, 97.1] n=20000 | yes | 0.1% | 0.1% | 25.3 / 147.3 | 24.5 / 147.3 | 1.17 |
| 20 CA vs 10 CA | 100.0% | 100.0% [100.0, 100.0] n=20000 | yes | 0.0% | 0.0% | 4.2 / 150.0 | 4.0 / 150.0 | 1.14 |
| 30 DD vs 10 CA | 9.8% | 9.7% [9.3, 10.1] n=20000 | yes | 0.6% | 0.7% | 141.6 / 71.3 | 141.6 / 71.3 | 1.17 |
| 90 DD vs 10 BB | 5.4% [5.1, 5.7] | 5.5% [5.1, 5.8] n=20000 | yes | 0.4% | 0.3% | 436.9 / 187.6 | 436.9 / 188.4 | 1.06 |
| 30 CA vs 10 BB | 40.3% [39.6, 41.0] | 40.3% [39.7, 41.0] n=20000 | yes | 0.3% | 0.3% | 318.2 / 242.2 | 317.9 / 242.5 | 1.08 |
| 29 CA + 3 DD vs 30 CA | 55.0% [54.3, 55.7] | 55.5% [54.8, 56.2] n=20000 | yes | 0.1% | 0.1% | 258.5 / 305.5 | 255.5 / 306.5 | 1.05 |
| 10 CA ROE 6 vs 15 CA | 2.5% | 2.4% [2.2, 2.7] n=20000 | yes | 0.0% | 0.0% | 139.6 / 21.6 | 139.3 / 21.6 | 1.17 |

Agree checks left win, right win, mutual and stalemate: an exact model value when the engine count passes a two-sided exact binomial test against it at 5%, a sampled one when the two 95% intervals overlap; and destroyed build points per side within 2% of that side's starting build points. A case missing on either side, or without a battle, is listed with the reason.

