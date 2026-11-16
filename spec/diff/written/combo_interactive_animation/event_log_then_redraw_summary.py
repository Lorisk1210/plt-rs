# A key-driven viewer stepping through datasets (n/p keys), logging each step, with a summary table at the end.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backend_bases import KeyEvent

rng = np.random.default_rng(103)
datasets = [rng.normal(k, 1 + k / 3, 100) for k in range(5)]
state = {"i": 0, "log": []}
fig, (ax, tab) = plt.subplots(1, 2, figsize=(10, 4), gridspec_kw={"width_ratios": [2, 1]})
def show():
    ax.cla()
    d = datasets[state["i"]]
    ax.hist(d, bins=20, color=f"C{state['i']}")
    ax.set_title(f"dataset {state['i']}: mean {d.mean():.2f}")
def on_key(e):
    step = {"n": 1, "p": -1}.get(e.key, 0)
    state["i"] = (state["i"] + step) % len(datasets)
    state["log"].append((e.key, state["i"]))
    show()
fig.canvas.mpl_connect("key_press_event", on_key)
show()
for key in ["n", "n", "p", "n", "n", "x"]:
    KeyEvent("key_press_event", fig.canvas, key)._process()
tab.axis("off")
tab.table(cellText=[[k, str(i)] for k, i in state["log"]], colLabels=["key", "dataset"], loc="center")
print(state["log"], ax.get_title())
fig.savefig("viewer.png", dpi=50)
