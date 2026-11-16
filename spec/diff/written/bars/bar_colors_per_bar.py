# Highlight one bar: all gray except the maximum in orange, plus colour by a colormap.
import numpy as np
import matplotlib.pyplot as plt

scores = [61, 74, 58, 89, 70, 66]
teams = ["Ants", "Bees", "Cats", "Dogs", "Eels", "Fox"]
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
colors = ["orange" if s == max(scores) else "lightgray" for s in scores]
a.bar(teams, scores, color=colors)
a.set_title("winner highlighted")
norm = plt.Normalize(min(scores), max(scores))
b.bar(teams, scores, color=plt.cm.viridis(norm(scores)), edgecolor="black", linewidth=0.5)
b.set_title("colour by value")
for ax in (a, b):
    ax.set_ylim(50, 95)
fig.tight_layout()
print([p.get_facecolor()[:3] for p in a.patches][:2])
fig.savefig("teams.png")
