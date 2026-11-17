# subplots_adjust edge cases (left>=right refused, partial updates, rcParams figure.subplot.*) and SubplotParams objects.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.figure import SubplotParams

fig, axs = plt.subplots(2, 2)
sp = fig.subplotpars
print(type(sp).__name__, [round(getattr(sp, k), 3) for k in ("left", "right", "bottom", "top", "wspace", "hspace")])
fig.subplots_adjust(left=0.2)
print(round(sp.left, 3), round(sp.right, 3), [round(v, 3) for v in axs[0, 0].get_position().bounds])
try:
    fig.subplots_adjust(left=0.9, right=0.1)
except ValueError as e:
    print("ValueError", str(e)[:60])
try:
    fig.subplots_adjust(bottom=0.8, top=0.2)
except ValueError as e:
    print("ValueError", str(e)[:60])
fig.subplots_adjust(wspace=0, hspace=0)
print([round(v, 3) for v in axs[0, 1].get_position().bounds])
with plt.rc_context({"figure.subplot.left": 0.05, "figure.subplot.wspace": 0.5}):
    f2, a2 = plt.subplots(1, 2)
    print(round(f2.subplotpars.left, 3), round(f2.subplotpars.wspace, 3), [round(v, 3) for v in a2[1].get_position().bounds])
custom = SubplotParams(left=0.3, right=0.7)
f3 = plt.figure(subplotpars=custom)
ax3 = f3.add_subplot()
print([round(v, 3) for v in ax3.get_position().bounds], f3.subplotpars is custom)
custom.update(bottom=0.4)
print(round(custom.bottom, 3))
gs = fig.add_gridspec(1, 1, left=0.5)
print(gs.left, gs.get_subplot_params(fig).left)
plt.close("all")
