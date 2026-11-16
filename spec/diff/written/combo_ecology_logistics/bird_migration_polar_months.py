# Migration counts: polar bars by month for two species, a cumulative arrival curve, and a ringing-station table.
import numpy as np
import matplotlib.pyplot as plt

months = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"]
swallow = np.array([0, 0, 5, 40, 80, 60, 50, 70, 90, 30, 2, 0])
crane = np.array([10, 30, 80, 40, 5, 0, 0, 0, 20, 90, 60, 15])
theta = np.linspace(0, 2 * np.pi, 12, endpoint=False)
fig = plt.figure(figsize=(12, 4.5))
p = fig.add_subplot(1, 3, 1, projection="polar")
p.bar(theta, swallow, width=0.25, label="swallow"); p.bar(theta + 0.25, crane, width=0.25, label="crane")
p.set_xticks(theta, months); p.set_theta_zero_location("N"); p.set_theta_direction(-1); p.legend(fontsize=7)
c = fig.add_subplot(1, 3, 2)
c.plot(months, np.cumsum(swallow) / swallow.sum(), "o-", label="swallow")
c.plot(months, np.cumsum(crane) / crane.sum(), "s-", label="crane")
c.legend()
t = fig.add_subplot(1, 3, 3); t.axis("off")
t.table(cellText=[["north", "120", "35"], ["coast", "260", "80"], ["lake", "70", "140"]], colLabels=["station", "swallow", "crane"], loc="center")
fig.tight_layout()
print(int(swallow.sum()), int(crane.sum()), [t.get_text() for t in c.get_xticklabels()][:4])
fig.savefig("migration.png", dpi=50)
