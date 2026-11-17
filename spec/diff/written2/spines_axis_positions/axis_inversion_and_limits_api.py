# invert_xaxis, set_xlim with reversed values, set_xbound, get_xbound, autoscale on inverted axes, and limits callbacks.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
ax.plot([1, 5], [10, 50])
log = []
ax.callbacks.connect("xlim_changed", lambda a: log.append(("x", tuple(round(v, 3) for v in a.get_xlim()))))
ax.callbacks.connect("ylim_changed", lambda a: log.append(("y", tuple(round(v, 3) for v in a.get_ylim()))))
ax.invert_xaxis()
print(ax.xaxis_inverted(), [round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_xbound()])
ax.set_xbound(0, 10)
print([round(v, 3) for v in ax.get_xlim()], ax.xaxis_inverted())
ax.set_ylim(60, 0)
print(ax.yaxis_inverted(), ax.get_ybound())
ax.set_xlim(right=-5)
print(ax.get_xlim())
ax.autoscale()
print([round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_ylim()], ax.xaxis_inverted())
ax.set_xlim(auto=None)
ax.set_ylim(None, 100)
print(ax.get_ylim(), ax.get_autoscalex_on(), ax.get_autoscaley_on())
print(log[:6])
try:
    ax.set_xlim(left=1, xmin=2)
except TypeError as e:
    print("TypeError", str(e)[:60])
ax.set_xlim(xmin=-1, xmax=11)
print(ax.get_xlim())
ax.set_xlim((3, 4))
print(ax.get_xlim(), len(log))
ax.xaxis.set_inverted(False)
print(ax.get_xlim())
fig.savefig("inverted.png")
