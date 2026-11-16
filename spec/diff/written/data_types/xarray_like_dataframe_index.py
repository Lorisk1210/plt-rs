# Plot a pandas Series with a string index, a RangeIndex frame, and a MultiIndex flattened for labels.
import pandas as pd
import matplotlib.pyplot as plt

s = pd.Series([4, 6, 3], index=["x", "y", "z"])
mi = pd.MultiIndex.from_product([["2023", "2024"], ["H1", "H2"]])
m = pd.Series([10, 12, 11, 15], index=mi)
fig, axs = plt.subplots(1, 3, figsize=(10, 3))
axs[0].plot(s)
axs[0].set_title("string index")
axs[1].bar(range(len(m)), m.values, tick_label=[" ".join(t) for t in m.index])
axs[1].set_title("MultiIndex labels")
frame = pd.DataFrame({"v": [1, 3, 2, 4]})
axs[2].plot(frame.index, frame["v"])
axs[2].set_title("RangeIndex")
print([t.get_text() for t in axs[1].get_xticklabels()])
fig.savefig("indexes.png")
