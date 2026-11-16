# Dot plot with categories on the y axis (survey answers), sorted by mean score.
import numpy as np
import matplotlib.pyplot as plt

answers = {"strongly agree": 4.6, "agree": 3.9, "neutral": 3.0, "disagree": 2.2, "strongly disagree": 1.4}
order = sorted(answers, key=answers.get)
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.scatter([answers[k] for k in order], order, s=80, color="purple", zorder=3)
ax.hlines(order, 0, [answers[k] for k in order], color="0.7")
ax.set_xlim(0, 5)
ax.set_xlabel("mean satisfaction")
ax.invert_yaxis()
fig.tight_layout()
print(ax.get_ylim())
fig.savefig("survey.png")
