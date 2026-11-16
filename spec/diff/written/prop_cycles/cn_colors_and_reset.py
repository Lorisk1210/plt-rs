# 'C0'...'C9' references after changing the cycle, and the cycle restarting per axes.
import matplotlib.pyplot as plt
from cycler import cycler

fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3))
for k in range(3):
    a.plot([0, 1], [k, k + 1])
b.set_prop_cycle(color=["olive", "navy", "teal"])
for k in range(4):
    b.plot([0, 1], [k, k + 1])
b.plot([0, 1], [5, 6], color="C1")
print([l.get_color() for l in a.lines], [l.get_color() for l in b.lines])
plt.rcParams["axes.prop_cycle"] = cycler(color=["gold", "silver"])
fig2, c = plt.subplots()
c.bar([1, 2], [3, 4], color=["C0", "C1"])
print([p.get_facecolor() for p in c.patches])
fig.savefig("cn.png")
fig2.savefig("cn_after.png")
