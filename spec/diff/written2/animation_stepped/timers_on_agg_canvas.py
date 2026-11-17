# TimerBase on a non-GUI canvas: new_timer, add/remove callbacks, interval and single_shot properties, and _on_timer driven by hand.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig = plt.figure()
timer = fig.canvas.new_timer(interval=250)
print(type(timer).__name__, timer.interval, timer.single_shot)
log = []


def tick(tag):
    log.append(tag)
    if tag == "stop" and len(log) > 4:
        return False


timer.add_callback(tick, "a")
timer.add_callback(tick, "stop")
timer._on_timer()
timer._on_timer()
timer._on_timer()
print(log, len(timer.callbacks))
timer.remove_callback(tick, "a")
timer._on_timer()
print(log, len(timer.callbacks))
timer.interval = 33.7
print(timer.interval)
timer.single_shot = True
print(timer.single_shot)
timer.start()
timer.stop()
t2 = fig.canvas.new_timer(interval=10, callbacks=[(log.append, ("x",), {})])
t2._on_timer()
print(log[-1], t2.interval)
try:
    timer.interval = "fast"
except (TypeError, ValueError) as e:
    print(type(e).__name__)
