# Raster plot of neuron spike times with eventplot and per-row colours.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(12)
spikes = [np.sort(rng.uniform(0, 1, rng.integers(5, 30))) for _ in range(8)]
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 3.5))
a.eventplot(spikes, colors=[f"C{i}" for i in range(8)], linelengths=0.8)
a.set_xlabel("time [s]")
a.set_ylabel("neuron")
b.eventplot(spikes[:3], orientation="vertical", lineoffsets=[1, 3, 5], linewidths=2)
b.set_title("vertical")
print([len(s) for s in spikes])
fig.savefig("raster.png")
