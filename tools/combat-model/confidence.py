"""Add exact SciPy binomial intervals to sampled fleet/ship sweep results."""
import argparse
import json
from pathlib import Path

import numpy
import scipy
from scipy.stats import binomtest



def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input',type=Path)
    parser.add_argument('--output',type=Path)
    args=parser.parse_args()
    data=json.loads(args.input.read_text())
    intervals=[]
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
        interval=binomtest(wins,trials).proportion_ci(confidence_level=.95,method='exact')
        intervals.append(dict(case=record['case'],mode=record['mode'],
                              variant=record.get('variant'),wins=wins,trials=trials,
                              low=float(interval.low),high=float(interval.high)))
    data=dict(numpy=numpy.__version__,scipy=scipy.__version__,
              confidence=.95,method='Clopper-Pearson',intervals=intervals)
    payload=json.dumps(data,indent=2)+'\n'
    if args.output is None:
        print(payload,end='')
    else:
        args.output.write_text(payload)
        print(f'Wrote {args.output}')


if __name__=='__main__':
    main()
