# 100% stacked horizontal bars of survey responses (Likert scale) per question.
import numpy as np
import matplotlib.pyplot as plt

questions = ["Q1 easy to use", "Q2 fast", "Q3 reliable", "Q4 good value"]
counts = np.array([[5, 10, 20, 40, 25], [10, 15, 30, 30, 15], [2, 8, 15, 45, 30], [20, 25, 25, 20, 10]])
labels = ["strongly disagree", "disagree", "neutral", "agree", "strongly agree"]
share = counts / counts.sum(axis=1, keepdims=True) * 100
colors = plt.colormaps["RdYlGn"](np.linspace(0.15, 0.85, 5))
fig, ax = plt.subplots(figsize=(8, 3.5))
left = np.zeros(len(questions))
for i, (label, color) in enumerate(zip(labels, colors)):
    ax.barh(questions, share[:, i], left=left, color=color, label=label)
    left += share[:, i]
ax.invert_yaxis()
ax.set_xlim(0, 100)
ax.legend(ncols=5, bbox_to_anchor=(0, 1), loc="lower left", fontsize="small")
fig.tight_layout()
print(np.round(share[0], 2))
fig.savefig("likert.png")
