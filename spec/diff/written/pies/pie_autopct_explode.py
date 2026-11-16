# Market share pie with percentages, the largest slice exploded and a shadow.
import matplotlib.pyplot as plt

labels = ["Chrome", "Safari", "Edge", "Firefox", "Other"]
share = [64.7, 18.6, 5.2, 3.0, 8.5]
explode = [0.08, 0, 0, 0, 0]
fig, ax = plt.subplots(figsize=(5, 5))
wedges, texts, autotexts = ax.pie(share, labels=labels, explode=explode, autopct="%1.1f%%",
                                  shadow=True, startangle=90, counterclock=False)
for t in autotexts:
    t.set_color("white")
    t.set_fontsize(9)
ax.set_title("Browser share")
ax.axis("equal")
print([round(w.theta2 - w.theta1, 3) for w in wedges])
fig.savefig("browsers.png")
