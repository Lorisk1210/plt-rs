# Long category names on the x axis rotated 45 degrees and right-aligned with rotation_mode='anchor'.
import matplotlib.pyplot as plt

names = ["Customer service", "Billing department", "Technical support", "Field operations",
         "Research and development", "Human resources"]
tickets = [120, 85, 240, 60, 30, 45]
fig, ax = plt.subplots(figsize=(6, 4))
ax.bar(names, tickets, color="slategray")
plt.setp(ax.get_xticklabels(), rotation=45, ha="right", rotation_mode="anchor")
ax.set_ylabel("open tickets")
fig.tight_layout()
fig.savefig("tickets.png")
ax.tick_params(axis="x", labelrotation=90)
fig.tight_layout()
fig.savefig("tickets_vertical.png")
print(ax.get_xticklabels()[0].get_rotation())
