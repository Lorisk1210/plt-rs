# tight_layout with pad/w_pad/h_pad/rect and TightLayoutEngine reuse; subplot params before and after, and with a suptitle.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 3, figsize=(6, 4), dpi=72)
for k, ax in enumerate(axs.flat):
    ax.set_title(f"title {k}")
    ax.set_ylabel("y label")


def params():
    sp = fig.subplotpars
    return [round(v, 3) for v in (sp.left, sp.right, sp.bottom, sp.top, sp.wspace, sp.hspace)]


print(params())
fig.tight_layout()
print(params())
fig.tight_layout(pad=2.0, w_pad=0.5, h_pad=3.0)
print(params())
fig.tight_layout(rect=(0, 0, 1, 0.9))
print(params())
fig.suptitle("suptitle")
fig.set_layout_engine("tight", pad=0.5)
fig.canvas.draw()
print(params(), type(fig.get_layout_engine()).__name__, fig.get_layout_engine().get())
fig.get_layout_engine().set(rect=(0.1, 0.1, 0.9, 0.9))
fig.canvas.draw()
print(params())
print(fig.get_tight_layout(), fig.get_constrained_layout())
fig.set_layout_engine(None)
fig.subplots_adjust(left=0.3)
print(params())
fig.savefig("tight.png")
