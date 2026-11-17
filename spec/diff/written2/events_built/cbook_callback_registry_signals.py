# cbook.CallbackRegistry used on its own: declared signals, connect/disconnect, process with arguments, blocked() and pickling.
import pickle
from matplotlib import cbook

reg = cbook.CallbackRegistry(signals=["changed", "removed"])
out = []


def on_changed(value, *, source="?"):
    out.append(("changed", value, source))


class Watcher:
    def handle(self, value, **kw):
        out.append(("watcher", value, kw.get("source")))


w = Watcher()
c1 = reg.connect("changed", on_changed)
c2 = reg.connect("changed", w.handle)
c3 = reg.connect("removed", lambda v: out.append(("removed", v)))
c1b = reg.connect("changed", on_changed)
print(c1 == c1b, c1, c2, c3)
reg.process("changed", 1, source="first")
reg.process("removed", 2)
print(out)
out.clear()
with reg.blocked(signal="changed"):
    reg.process("changed", 3)
    reg.process("removed", 4)
with reg.blocked():
    reg.process("removed", 5)
print(out)
out.clear()
try:
    reg.connect("unknown", on_changed)
except ValueError as e:
    print("ValueError", str(e)[:60])
reg.disconnect(c2)
reg.process("changed", 6)
print(out)
data = pickle.dumps(reg)
copy = pickle.loads(data)
print(type(copy).__name__, copy.callbacks)
del w
reg2 = cbook.CallbackRegistry()
reg2.connect("x", lambda: out.append("x"))
reg2.process("x")
print(out[-1], len(reg2.callbacks["x"]))
