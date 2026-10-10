"""Binomial confidence intervals and tests for sampled results.

`binomial_interval` gives SciPy's exact Clopper-Pearson interval when
SciPy is installed and Wilson's score interval otherwise; `binomial_test`
is the exact two-sided test of a count against a known probability. The
command line adds intervals to a fleet results file.
"""
import argparse
import json
import math
from pathlib import Path
from statistics import NormalDist


def wilson_interval(wins, trials, confidence=.95):
    if trials == 0:
        return 0.0, 1.0
    z = NormalDist().inv_cdf((1 + confidence) / 2)
    p = wins / trials
    centre = p + z * z / (2 * trials)
    spread = z * ((p * (1 - p) + z * z / (4 * trials)) / trials) ** .5
    denominator = 1 + z * z / trials
    return max(0.0, (centre - spread) / denominator), min(1.0, (centre + spread) / denominator)


def binomial_interval(wins, trials, confidence=.95):
    """(low, high, method)."""
    try:
        from scipy.stats import binomtest
    except ImportError:
        return wilson_interval(wins, trials, confidence) + ('Wilson',)
    interval = binomtest(wins, trials).proportion_ci(confidence_level=confidence, method='exact')
    return float(interval.low), float(interval.high), 'Clopper-Pearson'


def binomial_test(successes, trials, probability):
    """Two-sided exact binomial p-value: the mass of every count whose
    probability is at most that of the observed count."""
    if probability <= 0.0:
        return 1.0 if successes == 0 else 0.0
    if probability >= 1.0:
        return 1.0 if successes == trials else 0.0
    log_p, log_q = math.log(probability), math.log1p(-probability)

    def log_pmf(k):
        return (math.lgamma(trials + 1) - math.lgamma(k + 1) - math.lgamma(trials - k + 1)
                + k * log_p + (trials - k) * log_q)

    observed = log_pmf(successes)
    total = 0.0
    for k in range(trials + 1):
        value = log_pmf(k)
        if value <= observed + 1e-9:
            total += math.exp(value)
    return min(1.0, total)


def _version(module_name):
    try:
        return __import__(module_name).__version__
    except ImportError:
        return None


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input',type=Path)
    parser.add_argument('--output',type=Path)
    args=parser.parse_args()
    data=json.loads(args.input.read_text())
    intervals=[]
    methods=set()
    for record in data['battles']:
        trials=record['N']
        counts=record.get('outcome_counts')
        if counts is None:
            if trials>=100000:
                raise ValueError('Exact outcome counts required for this trial count')
            # Original review snapshots retained rounded frequencies.
            wins=round(record['outcomes'].get('A',0)*trials)
        else:
            wins=counts.get('A',0)
        low,high,method=binomial_interval(wins,trials)
        methods.add(method)
        intervals.append(dict(case=record['case'],mode=record['mode'],
                              variant=record.get('variant'),wins=wins,trials=trials,
                              low=low,high=high))
    data=dict(numpy=_version('numpy'),scipy=_version('scipy'),
              confidence=.95,method=', '.join(sorted(methods)),intervals=intervals)
    payload=json.dumps(data,indent=2)+'\n'
    if args.output is None:
        print(payload,end='')
    else:
        args.output.write_text(payload)
        print(f'Wrote {args.output}')


if __name__=='__main__':
    main()
