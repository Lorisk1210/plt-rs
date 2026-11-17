# plt.close on Agg figures: close_event delivery, manager destruction, Gcf bookkeeping and reuse of a closed figure object.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib._pylab_helpers import Gcf

events = []
fig = plt.figure()
fig.canvas.mpl_connect("close_event", lambda e: events.append(("close", e.canvas.figure is fig)))
mgr = fig.canvas.manager
print(type(mgr).__name__, mgr.num, Gcf.get_fig_manager(mgr.num) is mgr)
plt.close(fig)
print(events, plt.get_fignums(), fig.canvas.manager)
fig.add_subplot().plot([1, 2])
fig.savefig("closed_but_saved.png")
print(len(fig.axes))
fig2 = plt.figure()
fig2.canvas.mpl_connect("close_event", lambda e: events.append(("close2", True)))
Gcf.destroy(fig2.number)
print(events[-1:], plt.get_fignums())
fig3 = plt.figure()
Gcf.destroy_fig(fig3)
print(plt.get_fignums())
fig4 = plt.figure()
fig5 = plt.figure()
Gcf.destroy_all()
print(plt.get_fignums(), Gcf.get_active())
f = plt.figure()
Gcf.set_active(f.canvas.manager)
print(Gcf.get_num_fig_managers(), [m.num for m in Gcf.get_all_fig_managers()])
plt.close("all")
print(Gcf.get_num_fig_managers())
