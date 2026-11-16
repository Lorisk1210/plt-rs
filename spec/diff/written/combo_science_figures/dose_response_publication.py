# Dose-response figure: log-x error bars per compound, fitted Hill curves, EC50 markers and a summary table.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(31)
dose = np.logspace(-3, 1, 9)
compounds = {"A": (0.05, 1.2), "B": (0.3, 1.0), "C": (2.0, 1.8)}
fig, (ax, tab) = plt.subplots(1, 2, figsize=(11, 4), gridspec_kw={"width_ratios": [2.2, 1]})
rows = []
for name, (ec50, hill) in compounds.items():
    resp = 100 / (1 + (ec50 / dose) ** hill)
    obs = resp + rng.normal(0, 4, dose.size)
    eb = ax.errorbar(dose, obs, yerr=4, fmt="o", capsize=3, label=name)
    fine = np.logspace(-3, 1, 200)
    ax.plot(fine, 100 / (1 + (ec50 / fine) ** hill), color=eb[0].get_color())
    ax.axvline(ec50, color=eb[0].get_color(), ls=":", lw=1)
    rows.append([name, f"{ec50:g}", f"{hill:.1f}", f"{obs.max():.0f}"])
ax.set_xscale("log")
ax.set_xlabel("dose [µM]")
ax.set_ylabel("response [%]")
ax.legend(title="compound")
tab.axis("off")
t = tab.table(cellText=rows, colLabels=["cmpd", "EC50", "Hill", "max"], loc="center")
t.scale(1, 1.6)
print(rows, [l.get_color() for l in ax.get_lines()][:3])
fig.savefig("dose_response.png", dpi=50)
