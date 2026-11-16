# A grid of explanatory text boxes (wrapped) beside small charts, using subfigures for layout.
import numpy as np
import matplotlib.pyplot as plt

notes = ["Revenue grew steadily through the quarter with a dip in week six caused by a supply delay.",
         "Customer satisfaction improved after the support team doubled its weekend coverage.",
         "Costs stayed flat; the largest share remains cloud infrastructure and licenses."]
fig = plt.figure(figsize=(10, 7), layout="constrained")
rows = fig.subfigures(3, 1)
rng = np.random.default_rng(123)
for row, note in zip(rows, notes):
    left, right = row.subplots(1, 2, width_ratios=[1, 2])
    left.axis("off")
    left.text(0, 0.5, note, wrap=True, va="center", fontsize=10, bbox=dict(boxstyle="round", fc="0.95"))
    right.plot(rng.normal(0, 1, 30).cumsum())
print(len(fig.subfigs), len(fig.axes))
fig.savefig("notes.png", dpi=50)
