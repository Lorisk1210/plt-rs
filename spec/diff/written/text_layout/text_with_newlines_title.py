# Two-line titles, labels with newlines, and a long suptitle wrapped manually with textwrap.
import textwrap
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 2, figsize=(8, 4))
axs[0].plot([1, 2, 3])
axs[0].set_title("Main title\nsubtitle in second line", fontsize=11)
axs[0].set_xlabel("time\n(seconds since start)")
axs[1].bar([1, 2], [3, 4])
axs[1].set_ylabel("count of\nevents")
long = "This figure summarises the results of a fairly long experiment with many conditions and repeats"
fig.suptitle("\n".join(textwrap.wrap(long, 50)), fontsize=10)
fig.tight_layout()
print(fig._suptitle.get_text().count("\n"))
fig.savefig("newlines.png")
