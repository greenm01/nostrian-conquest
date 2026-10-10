"""Candidate automatic repairs: whole hulls, one local rounded bill.
Historical rejected alternative: the owner selected free RT recovery.
"""


def repair_batch(costs, budget):
    spent_quarters = 0
    remaining = []
    for cost in sorted(costs, reverse=True):
        if (spent_quarters + cost + 3) // 4 <= budget:
            spent_quarters += cost
        else:
            remaining.append(cost)
    return remaining, (spent_quarters + 3) // 4


def repair_installments(hulls, budget):
    """(build cost, quarter-points paid) per hull; no AS until completion."""
    remaining = []
    available = 4 * budget
    for cost, paid in sorted(hulls, key=lambda hull: -hull[0]):
        payment = min(cost - paid, available)
        available -= payment
        paid += payment
        if paid < cost:
            remaining.append((cost, paid))
    return remaining, (4 * budget - available + 3) // 4
