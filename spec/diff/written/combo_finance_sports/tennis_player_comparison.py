# Player comparison: mirrored horizontal bars of stats, a radar of normalised skills, and match-history win streak.
import numpy as np
import matplotlib.pyplot as plt

stats = ["aces", "1st serve %", "break pts won", "winners", "unforced err"]
p1 = np.array([12, 64, 45, 38, 22]); p2 = np.array([8, 70, 52, 30, 18])
fig = plt.figure(figsize=(12, 4.5))
a = fig.add_subplot(1, 3, 1)
a.barh(stats, -p1, color="C0", label="P1"); a.barh(stats, p2, color="C1", label="P2")
a.set_xticks(a.get_xticks(), [str(abs(int(t))) for t in a.get_xticks()])
a.legend(loc="lower right")
r = fig.add_subplot(1, 3, 2, projection="polar")
ang = np.linspace(0, 2 * np.pi, 5, endpoint=False)
norm = np.maximum(p1, p2)
for p, c in [(p1, "C0"), (p2, "C1")]:
    v = p / norm
    r.plot(np.r_[ang, ang[:1]], np.r_[v, v[:1]], color=c)
    r.fill(np.r_[ang, ang[:1]], np.r_[v, v[:1]], color=c, alpha=0.2)
r.set_xticks(ang, stats, fontsize=7)
h = fig.add_subplot(1, 3, 3)
results = np.array([1, 1, 0, 1, 1, 1, 0, 0, 1, 1])
h.bar(range(10), np.where(results, 1, -1), color=np.where(results, "C0", "C1"))
h.set_yticks([-1, 1], ["P2 won", "P1 won"])
fig.tight_layout()
print([t.get_text() for t in a.get_xticklabels()][:3], int(results.sum()))
fig.savefig("tennis.png", dpi=50)
