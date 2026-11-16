# One card per athlete: radar of skills, bar of seasons, and a name banner; cards collected in a grid too.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(15)
skills = ["speed", "power", "stamina", "skill", "tactics"]
athletes = {name: rng.integers(4, 10, 5) for name in ["Ana", "Ben", "Cleo", "Dev"]}
angles = np.linspace(0, 2 * np.pi, 5, endpoint=False)
for name, vals in athletes.items():
    fig = plt.figure(figsize=(6, 3))
    r = fig.add_subplot(1, 2, 1, projection="polar")
    r.fill(np.r_[angles, angles[:1]], np.r_[vals, vals[:1]], alpha=0.4)
    r.set_xticks(angles, skills, fontsize=7)
    r.set_ylim(0, 10)
    b = fig.add_subplot(1, 2, 2)
    b.bar(["21", "22", "23", "24"], rng.integers(5, 30, 4))
    fig.suptitle(name, fontweight="bold")
    fig.savefig(f"card_{name}.png", dpi=40)
    plt.close(fig)
fig, axs = plt.subplots(2, 2, figsize=(7, 7), subplot_kw=dict(projection="polar"))
for ax, (name, vals) in zip(axs.flat, athletes.items()):
    ax.plot(np.r_[angles, angles[:1]], np.r_[vals, vals[:1]])
    ax.set_title(name)
    ax.set_xticks(angles, skills, fontsize=6)
print({k: v.tolist() for k, v in athletes.items()})
fig.savefig("all_cards.png", dpi=40)
