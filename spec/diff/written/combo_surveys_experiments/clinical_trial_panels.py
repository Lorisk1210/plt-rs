# Clinical trial: Kaplan-Meier-like step curves with censor marks, number-at-risk table, and adverse-event bars.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(73)
def km(times, events):
    order = np.argsort(times); t, e = times[order], events[order]
    surv, s = [1.0], 1.0
    at_risk = len(t)
    for ti, ei in zip(t, e):
        if ei:
            s *= 1 - 1 / at_risk
        surv.append(s); at_risk -= 1
    return np.r_[0, t], np.array(surv)
fig, (a, b) = plt.subplots(2, 1, figsize=(8, 7), gridspec_kw={"height_ratios": [3, 1]})
risk_rows = []
for arm, scale, color in [("placebo", 12, "C3"), ("drug", 20, "C0")]:
    t = rng.exponential(scale, 60); e = rng.random(60) < 0.8
    x, s = km(t, e)
    a.step(x, s, where="post", color=color, label=arm)
    a.plot(np.sort(t[~e]), np.interp(np.sort(t[~e]), x, s), "|", color=color, ms=8)
    risk_rows.append([arm] + [str((t >= m).sum()) for m in (0, 10, 20, 30)])
a.legend()
a.set_ylabel("survival")
a.set_xlim(0, 40)
b.axis("off")
b.table(cellText=risk_rows, colLabels=["at risk", "0", "10", "20", "30"], loc="center")
print(risk_rows)
fig.savefig("trial.png", dpi=50)
