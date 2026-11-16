# Calling subplots_adjust on a constrained-layout figure warns and is ignored; the layout still runs.
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 2, layout="constrained", figsize=(5, 4))
for i, ax in enumerate(axs.flat):
    ax.plot(range(5), [v * i for v in range(5)])
    ax.set_title(f"#{i}")
fig.subplots_adjust(wspace=0.6)
fig.savefig("constrained_adjust.png")
print(fig.get_constrained_layout())
fig.set_constrained_layout(False) if hasattr(fig, "set_constrained_layout") else None
fig.set_layout_engine(None)
fig.subplots_adjust(wspace=0.6, hspace=0.6)
fig.savefig("no_layout.png")
print(fig.get_layout_engine())
