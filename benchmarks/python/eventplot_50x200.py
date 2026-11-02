"""50 rows of 200 events each, one row per dataset."""

import matplotlib.pyplot as plt

from _common import FastRng, save

rng = FastRng(43)
spikes = [rng.uniform(200, 0.0, 10.0) for _ in range(50)]


def draw():
    fig, ax = plt.subplots()
    ax.eventplot(spikes, linelengths=0.8)
    ax.set_xlabel("time (s)")
    ax.set_ylabel("neuron")
    return fig


if __name__ == "__main__":
    save(draw())
