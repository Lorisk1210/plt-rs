# A lab-notebook-like script building a figure step by step, printing state after each step.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 4))
print("step 1", len(ax.lines), ax.get_xlim())
t = np.linspace(0, 4, 100)
ax.plot(t, np.exp(-t) * np.cos(4 * t), label="measured")
print("step 2", len(ax.lines), [round(v, 3) for v in ax.get_ylim()])
ax.axhline(0, color="k", lw=0.5)
ax.set_xlabel("time [s]")
ax.set_ylabel("signal [V]")
print("step 3", ax.get_xlabel(), len(ax.lines))
env = np.exp(-t)
ax.fill_between(t, -env, env, color="gray", alpha=0.2, label="envelope")
ax.legend()
print("step 4", [t.get_text() for t in ax.get_legend().get_texts()])
ax.set_xlim(0, 3)
ax.set_title("damped oscillation")
fig.tight_layout()
print("step 5", ax.get_title(), [round(v, 3) for v in ax.get_position().bounds])
fig.savefig("notebook.png", dpi=50)
