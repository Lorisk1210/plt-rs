# Invalid colour specifications: typos, out-of-range tuples, wrong-length arrays; print each message.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
for spec in ["bleu", (1.2, 0, 0), "#12345", (0.1, 0.2), "C", 5]:
    try:
        ax.plot([0, 1], [0, 1], color=spec)
        print(repr(spec), "accepted")
    except ValueError as err:
        print(repr(spec), "ValueError:", str(err)[:80])
try:
    ax.scatter([1, 2, 3], [1, 2, 3], c=["red", "green"])
except ValueError as err:
    print("scatter:", str(err)[:100])
fig.savefig("colors_ok.png")
