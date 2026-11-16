# Inspect containers returned by bar, errorbar, stem and boxplot: their types, lengths and children types.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 4, figsize=(12, 3))
bc = axs[0].bar(range(3), [1, 3, 2], label="b")
eb = axs[1].errorbar(range(3), [1, 3, 2], yerr=[0.2, 0.3, 0.1], capsize=3)
sc = axs[2].stem(range(3), [1, 3, 2])
bp = axs[3].boxplot([np.arange(10), np.arange(5, 20)])
print(type(bc).__name__, len(bc), bc.get_label(), type(bc.patches[0]).__name__, bc.errorbar)
print(type(eb).__name__, len(eb), [type(x).__name__ for x in eb], eb.has_yerr)
print(type(sc).__name__, type(sc.markerline).__name__, type(sc.stemlines).__name__, type(sc.baseline).__name__)
print(type(bp).__name__, sorted((k, len(v)) for k, v in bp.items()))
print([type(c).__name__ for c in axs[0].containers], len(axs[1].containers))
fig.savefig("containers.png", dpi=40)
