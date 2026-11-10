"""
Driving Wayland windows through a weston nested in the X server
(`xdisplay.private_wayland`): the input faked on the X server reaches
weston's window, whose seat hands it to the Wayland clients. A client
cannot tell where its window is on the output, so the driver finds the
canvas by its pixels (`canvas.wait_shown`), which the program writes
when it is ready.
"""

#: The title of the X window of weston's output.
OUTPUT_TITLE = "Weston Compositor - X1"
#: The colour of the shell's background on weston's output, as
#: `xdisplay.private_wayland` configures it (RGB).
BACKGROUND = bytes([0x2e, 0x34, 0x40])
#: Where the close button of a window decoration is from the canvas'
#: top-right corner, for Qt's and pltrs' decorations alike: in the title
#: bar right above the canvas, at its right end.
CLOSE_BUTTON = (-14, -15)


def close(driver):
    """Click the close button of the window's decoration."""
    x, y = CLOSE_BUTTON
    driver.move(driver.width + x, driver.height - y)
    driver.click(1)
