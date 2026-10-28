/* plt-rs' modern chrome for matplotlib's client (see `plt::look`), with
   plt-modern.css; loaded after mpl.js, it uses the client's own hooks:
   - each figure is marked `plt-figure`, and the figures drawn on a dark
     background (the page lists them in `data-plt-dark`) take the dark
     theme, as plt-rs' windows do;
   - the canvas has a resize handle on a 24 px target, which a mouse, a
     pen or a finger drags;
   - the page is marked `plt-keyboard` while the keyboard moves the focus
     (Tab), so that the canvas shows a focus ring then, but not when the
     client focuses it as the page loads or on a click;
   - clicking a toolbar button leaves the focus, and with it the keys,
     where it was, as in the windows (Tab still reaches the buttons);
   - the zoom rectangle is drawn as the windows draw it, in the theme's
     colours, in device pixels;
   - the page of one figure is named after the figure. */
(function () {
    'use strict';

    var dark = (document.documentElement.getAttribute('data-plt-dark') || '').split(' ');

    mpl.figure.prototype._root_extra_style = function (root) {
        root.classList.add('plt-figure');
        if (dark.indexOf(String(this.id)) !== -1) {
            root.classList.add('plt-dark');
        }
    };

    // Dragging the handle sizes the canvas as the browser's own handle
    // does (the client then asks the figure for that size); the figure
    // does not see the drag.
    mpl.figure.prototype._canvas_extra_style = function (canvas_div) {
        var handle = document.createElement('div');
        handle.className = 'plt-resize-handle';
        handle.addEventListener('dblclick', function (event) {
            event.stopPropagation();
        });
        handle.addEventListener('pointerdown', function (event) {
            if (event.button !== 0) {
                return;
            }
            // This also keeps the mouse events of the drag from the figure.
            event.preventDefault();
            handle.setPointerCapture(event.pointerId);
            var x = event.clientX - canvas_div.clientWidth;
            var y = event.clientY - canvas_div.clientHeight;
            var drag = function (move) {
                canvas_div.style.width = Math.max(1, move.clientX - x) + 'px';
                canvas_div.style.height = Math.max(1, move.clientY - y) + 'px';
            };
            var drop = function () {
                handle.removeEventListener('pointermove', drag);
                handle.removeEventListener('pointerup', drop);
                handle.removeEventListener('pointercancel', drop);
            };
            handle.addEventListener('pointermove', drag);
            handle.addEventListener('pointerup', drop);
            handle.addEventListener('pointercancel', drop);
        });
        canvas_div.appendChild(handle);
    };

    document.addEventListener('keydown', function (event) {
        if (event.key === 'Tab') {
            document.documentElement.classList.add('plt-keyboard');
        }
    }, true);

    var toolbar_click = false;
    document.addEventListener('mousedown', function (event) {
        document.documentElement.classList.remove('plt-keyboard');
        toolbar_click = event.target.closest('.mpl-toolbar button') !== null;
    }, true);
    document.addEventListener('focusin', function (event) {
        if (toolbar_click && event.relatedTarget && event.target.closest('.mpl-toolbar')) {
            event.relatedTarget.focus();
        }
        toolbar_click = false;
    });

    mpl.figure.prototype.handle_rubberband = function (fig, msg) {
        var band = fig.rubberband_canvas;
        var width = fig.canvas.width;
        var height = fig.canvas.height;
        // Setting the size clears the canvas; it is shown at the figure's
        // size in CSS pixels.
        band.width = width;
        band.height = height;
        band.style.width = width / fig.ratio + 'px';
        band.style.height = height / fig.ratio + 'px';
        // The figure measures from the bottom.
        var left = Math.round(Math.min(msg.x0, msg.x1));
        var right = Math.round(Math.max(msg.x0, msg.x1));
        var top = Math.round(height - Math.max(msg.y0, msg.y1));
        var bottom = Math.round(height - Math.min(msg.y0, msg.y1));
        if (right <= left || bottom <= top) {
            return;
        }
        var style = getComputedStyle(fig.root);
        var edge = Math.max(1, Math.round(fig.ratio));
        var ctx = fig.rubberband_context;
        ctx.fillStyle = style.getPropertyValue('--plt-rubber-band');
        ctx.fillRect(left, top, right - left, bottom - top);
        ctx.strokeStyle = style.getPropertyValue('--plt-rubber-band-edge');
        ctx.lineWidth = edge;
        ctx.strokeRect(left + edge / 2, top + edge / 2, right - left - edge, bottom - top - edge);
    };

    var show_label = mpl.figure.prototype.handle_figure_label;
    mpl.figure.prototype.handle_figure_label = function (fig, msg) {
        show_label(fig, msg);
        if (document.getElementById('figure')) {
            document.title = msg.label;
        }
    };
})();
