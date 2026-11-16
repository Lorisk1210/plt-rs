# plt.style.use for the whole script, combining two sheets, then back to defaults.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

plt.style.use(["seaborn-v0_8-darkgrid", "seaborn-v0_8-talk"])
fig, ax = plt.subplots(figsize=(6, 4))
ax.plot(np.arange(10), np.arange(10) ** 1.3, marker="o")
ax.set_title("talk + darkgrid")
fig.savefig("talk.png")
print(mpl.rcParams["font.size"], mpl.rcParams["axes.facecolor"])
plt.style.use("default")
fig2, ax2 = plt.subplots()
ax2.plot([1, 2, 3])
fig2.savefig("default.png")
print(mpl.rcParams["font.size"])
