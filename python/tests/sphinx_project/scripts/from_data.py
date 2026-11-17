"""A plot of the data file in the working directory: ``plot_working_directory``
or, where that is not set, this script's directory."""

from pathlib import Path

from matplotlib import pyplot as plt

values = [float(word) for word in Path('data.txt').read_text().split()]
plt.step(range(len(values)), values, where='mid')
plt.title(f'{len(values)} values')
