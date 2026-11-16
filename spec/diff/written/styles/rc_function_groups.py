# The plt.rc('group', **kw) shortcut and rcdefaults() reset, as in old tutorials.
import matplotlib.pyplot as plt
import matplotlib as mpl

plt.rc("lines", linewidth=3, color="r")
plt.rc("axes", titlesize=18, labelcolor="green")
plt.rc("font", family="serif")
fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot([1, 2, 3], [3, 1, 2])
ax.set_title("rc groups")
ax.set_xlabel("green label")
fig.savefig("rc_groups.png")
print(mpl.rcParams["lines.linewidth"], mpl.rcParams["axes.titlesize"], mpl.rcParams["font.family"])
mpl.rcdefaults()
print(mpl.rcParams["lines.linewidth"], mpl.rcParams["font.family"])
fig2, ax2 = plt.subplots()
ax2.plot([1, 2, 3], [3, 1, 2])
fig2.savefig("after_reset.png")
