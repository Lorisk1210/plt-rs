# Batch-export one histogram per sensor in a loop, closing each figure so memory does not grow.
import numpy as np
import matplotlib.pyplot as plt

np.random.seed(3)
sensors = {f"S{i}": np.random.normal(20 + i, 1 + i / 3, 400) for i in range(1, 7)}

for name, values in sensors.items():
    fig = plt.figure(figsize=(4, 3))
    plt.hist(values, bins=25, color="teal", edgecolor="white")
    plt.axvline(values.mean(), color="k", ls="--", lw=1)
    plt.title(f"Sensor {name}: mean {values.mean():.2f}")
    plt.xlabel("temperature [°C]")
    plt.savefig(f"hist_{name}.png", dpi=72)
    plt.close(fig)

print("open figures:", plt.get_fignums())
print(len(sensors), "files")
