# Box and violin of the same skewed samples side by side with shared y and medians annotated.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(10)
samples = [rng.lognormal(0, s, 200) for s in (0.3, 0.6, 0.9)]
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 4), sharey=True)
a.boxplot(samples, tick_labels=["0.3", "0.6", "0.9"])
a.set_title("box")
b.violinplot(samples, showmedians=True)
b.set_xticks([1, 2, 3], ["0.3", "0.6", "0.9"])
b.set_title("violin")
for i, s in enumerate(samples, start=1):
    b.text(i + 0.1, np.median(s), f"{np.median(s):.2f}", fontsize=8)
a.set_yscale("log")
print([round(float(np.median(s)), 4) for s in samples])
fig.savefig("box_vs_violin.png")
