# Loan amortization: stacked interest/principal bars per year, remaining balance line on a twin axis, and a summary table.
import numpy as np
import matplotlib.pyplot as plt

P, r, n = 250000, 0.04 / 12, 25 * 12
payment = P * r / (1 - (1 + r) ** -n)
balance, interest, principal = P, [], []
for _ in range(n):
    i = balance * r
    interest.append(i); principal.append(payment - i)
    balance -= payment - i
interest, principal = np.array(interest).reshape(25, 12).sum(1), np.array(principal).reshape(25, 12).sum(1)
years = np.arange(1, 26)
fig, (a, t) = plt.subplots(1, 2, figsize=(12, 4.5), gridspec_kw={"width_ratios": [2.5, 1]})
a.bar(years, principal, label="principal")
a.bar(years, interest, bottom=principal, label="interest")
tw = a.twinx()
tw.plot(years, P - np.cumsum(principal), "k-", label="balance")
a.legend(loc="center left")
t.axis("off")
t.table(cellText=[["payment", f"{payment:,.2f}"], ["total interest", f"{interest.sum():,.0f}"], ["total paid", f"{payment * n:,.0f}"]], loc="center")
print(round(payment, 2), round(float(interest.sum()), 2))
fig.savefig("loan.png", dpi=50)
