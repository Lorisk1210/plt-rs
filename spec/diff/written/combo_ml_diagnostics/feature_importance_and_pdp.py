# Feature importances as sorted horizontal bars with error, plus partial-dependence-like lines for the top 3 features.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(42)
features = [f"f{i}" for i in range(10)]
imp = np.sort(rng.dirichlet(np.ones(10) * 0.7))[::-1]
err = imp * rng.uniform(0.1, 0.3, 10)
fig = plt.figure(figsize=(11, 4.5))
gs = fig.add_gridspec(1, 4)
ax = fig.add_subplot(gs[0, :2])
order = np.argsort(imp)
ax.barh(np.array(features)[order], imp[order], xerr=err[order], color="teal", capsize=2)
ax.set_xlabel("importance")
for k in range(2):
    sub = fig.add_subplot(gs[0, 2 + k])
    grid = np.linspace(0, 1, 50)
    sub.plot(grid, np.sin(3 * grid + k) * imp[k] * 5, color="teal")
    sub.fill_between(grid, np.sin(3 * grid + k) * imp[k] * 5 - 0.05, np.sin(3 * grid + k) * imp[k] * 5 + 0.05, alpha=0.2, color="teal")
    sub.set_title(f"PD of {features[k]}")
fig.tight_layout()
print(np.round(imp[:3], 4).tolist(), [t.get_text() for t in ax.get_yticklabels()][-3:])
fig.savefig("importance.png", dpi=50)
