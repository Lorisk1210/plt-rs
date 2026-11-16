# Wrong keyword names (typos and aliases from other libraries) raise AttributeError with a hint.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
for kwargs in [{"colour": "red"}, {"linewidht": 2}, {"markersize": 5, "mfc": "r"}, {"lw": 2, "linewidth": 3}]:
    try:
        ax.plot([1, 2, 3], **kwargs)
        print(kwargs, "ok")
    except (AttributeError, TypeError) as err:
        print(kwargs, type(err).__name__, str(err)[:90])
try:
    ax.set_title("t", fontsize="hugeish")
except ValueError as err:
    print("ValueError:", str(err)[:90])
try:
    ax.set(xlabl="x")
except AttributeError as err:
    print("AttributeError:", str(err)[:90])
fig.savefig("kw.png")
