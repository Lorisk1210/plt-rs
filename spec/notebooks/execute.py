"""
Executes notebooks in a real Jupyter kernel and records what the kernel
sends, for ``cargo xtask notebooks``, which runs it once in the
notebooks' reference venv (real matplotlib) and once in their pltrs venv
(``python/alias`` on ``PYTHONPATH``, so that ``import matplotlib`` gives
pltrs) and compares the two records.

``execute.py OUT NOTEBOOK...`` executes each notebook with nbclient in a
kernel of this interpreter (``python3``, ipykernel's), every cell even
after an error, in a working directory of its own under ``OUT/work``, and
writes ``OUT/<name>.json``:

* ``cells``: per cell, its ``id``, its ``outputs`` as nbformat keeps them
  (``display_data``, ``execute_result``, ``stream``, ``error``) and its
  ``messages``: the comm messages the kernel sent while the cell ran and
  every message (but the kernel's status) it sent while handling what the
  frontend sent after the cell, each ``{"type", "content", "buffers"}``
  with the buffers in base64;
* ``failure``: why the notebook could not be executed to its end (the
  kernel died, a cell timed out), or null.

The driver plays the part of the browser for the interactive backends,
as their scripts do: when the kernel opens a figure's comm (nbAgg's
``matplotlib`` target, ipympl's ``MPLCanvasModel`` widget) it sends the
messages the client sends on opening, asks for a new image whenever the
kernel says ``draw`` and none is awaited, acknowledges images (nbAgg) and
refreshes after a resize. A browser asks for images as long as the
kernel says ``draw`` (a figure whose draw leaves it stale, as an Axes of
fixed aspect with blitting widgets does, is drawn without end); the
driver asks for at most `DRAWS` images of a figure each time it settles,
so that every run sees the same bounded exchange. A cell's metadata can add what a user does
after the cell has run: ``{"pltrs": {"frontend": [ACTION...]}}``, each
action ``{"send": CONTENT}`` sent to a figure's comm as its client sends
messages (``"to": "canvas"``, the default, or ``"toolbar"`` for ipympl's
toolbar; ``"figure"``: the index of the figure among those opened, by
default the last). Everything the kernel sends back is waited for before
the next cell runs.

Before the notebooks run, ``matplotlib.pyplot`` is imported once in a
process of its own, so that no kernel builds the font cache (and logs
that it does).
"""

import asyncio
import base64
import collections
import json
import pathlib
import subprocess
import sys

import nbformat
from nbclient import NotebookClient
from nbclient.exceptions import CellTimeoutError, DeadKernelError

#: The seconds a cell may run, and those the kernel may take to answer
#: what the frontend sent.
TIMEOUT = 120

#: The images the frontend asks for of one figure each time it settles.
DRAWS = 4


def _buffers(message):
    """The binary buffers of *message*, in base64."""
    return [base64.b64encode(bytes(buffer)).decode("ascii")
            for buffer in message.get("buffers") or ()]


def _recorded(message):
    """*message* as the record keeps it."""
    return {
        "type": message["msg_type"],
        "content": message["content"],
        "buffers": _buffers(message),
    }


class Figure:
    """A figure's comm as its browser client sees it."""

    def __init__(self, kind, comm_id, figure_id=None):
        #: ``"nbagg"`` or ``"ipympl"``.
        self.kind = kind
        self.comm_id = comm_id
        #: nbAgg's ``figure_id`` (its output area's id).
        self.figure_id = figure_id
        #: ipympl's toolbar widget, once its model is known.
        self.toolbar = None
        #: Whether an image was asked for and has not come yet.
        self.waiting = False
        #: The images asked for since the frontend last settled.
        self.draws = 0

    def wrap(self, content):
        """The comm data carrying the client message *content*: nbAgg's
        client sends the JSON text itself, ipywidgets a custom message."""
        if self.kind == "nbagg":
            return json.dumps({**content, "figure_id": self.figure_id})
        return {"method": "custom", "content": content}

    def opening(self):
        """What the client sends once the comm is open (at a pixel ratio
        of 1)."""
        if self.kind == "nbagg":
            return [{"type": "supports_binary", "value": True},
                    {"type": "send_image_mode"},
                    {"type": "refresh"}]
        return [{"type": "refresh"}, {"type": "send_image_mode"},
                {"type": "initialized"}]

    def answer(self, data, buffers):
        """What the client sends back for the kernel's comm data *data*
        with *buffers* (and the toolbar an ipympl canvas names)."""
        if self.kind == "nbagg":
            if "blob" in data or str(data.get("data", "")).startswith("data:"):
                self.waiting = False
                return [{"type": "ack"}]
            message = json.loads(data["data"])
        else:
            toolbar = (data.get("state") or {}).get("toolbar")
            if toolbar:
                self.toolbar = toolbar.removeprefix("IPY_MODEL_")
            if data.get("method") != "custom":
                return []
            message = json.loads(data["content"]["data"])
            if message["type"] == "binary" and buffers:
                self.waiting = False
                return []
        if message["type"] == "draw" and not self.waiting and self.draws < DRAWS:
            self.waiting = True
            self.draws += 1
            return [{"type": "draw"}]
        if message["type"] == "resize":
            return [{"type": "refresh"}]
        return []


class Frontend:
    """The browser's part: the figures' comms, what is to be sent to
    them, and the record of what the kernel sends."""

    def __init__(self):
        self.figures = []
        #: (comm id, data) pairs to send, in order.
        self.pending = collections.deque()
        #: Where the messages of the cell running are recorded.
        self.messages = []

    def observe(self, message):
        """Records the kernel's comm *message* and queues the answers."""
        self.messages.append(_recorded(message))
        content = message["content"]
        data = content.get("data") or {}
        kind = message["msg_type"]
        if kind == "comm_open":
            self._opened(content["comm_id"], content.get("target_name"), data)
        elif kind == "comm_msg":
            for figure in self.figures:
                if figure.comm_id == content["comm_id"]:
                    for answer in figure.answer(data, message.get("buffers")):
                        self.pending.append((figure.comm_id, figure.wrap(answer)))

    def _opened(self, comm_id, target, data):
        """Starts the client of a figure whose comm opened."""
        state = data.get("state") or {}
        if target == "matplotlib":
            figure = Figure("nbagg", comm_id, data.get("id"))
        elif state.get("_model_name") == "MPLCanvasModel":
            figure = Figure("ipympl", comm_id)
            figure.answer({"state": state}, None)
        else:
            return
        self.figures.append(figure)
        for content in figure.opening():
            self.pending.append((comm_id, figure.wrap(content)))

    def act(self, action):
        """Queues what a user does: *action* of a cell's metadata."""
        figure = self.figures[action.get("figure", -1)]
        comm_id = figure.comm_id
        if action.get("to", "canvas") == "toolbar":
            comm_id = figure.toolbar
        self.pending.append((comm_id, figure.wrap(action["send"])))


class RecordingClient(NotebookClient):
    """nbclient's client, recording the kernel's comm messages and
    playing the frontend."""

    def __init__(self, nb, frontend, **kwargs):
        super().__init__(nb, **kwargs)
        self.frontend = frontend

    def process_message(self, msg, cell, cell_index):
        if msg["msg_type"].startswith("comm"):
            self.frontend.observe(msg)
        return super().process_message(msg, cell, cell_index)

    async def settle(self):
        """Sends what the frontend has to send, one message after the
        other, each once the kernel is done with the one before, and
        records what the kernel sends meanwhile."""
        for figure in self.frontend.figures:
            figure.draws = 0
        while self.frontend.pending:
            comm_id, data = self.frontend.pending.popleft()
            message = self.kc.session.msg(
                "comm_msg", {"comm_id": comm_id, "data": data})
            self.kc.shell_channel.send(message)
            await self._await_idle(message["header"]["msg_id"])

    async def _await_idle(self, parent):
        """Records the kernel's messages until it is idle again after
        handling the message *parent*."""
        while True:
            reply = await self.kc.iopub_channel.get_msg(timeout=TIMEOUT)
            content = reply["content"]
            if reply["msg_type"] == "status":
                if (content["execution_state"] == "idle"
                        and reply["parent_header"].get("msg_id") == parent):
                    return
            elif reply["msg_type"].startswith("comm"):
                self.frontend.observe(reply)
            else:
                self.frontend.messages.append(_recorded(reply))


async def execute(path, work):
    """The record of the notebook at *path*, executed in *work*."""
    nb = nbformat.read(path, as_version=4)
    frontend = Frontend()
    client = RecordingClient(
        nb, frontend, timeout=TIMEOUT, allow_errors=True, kernel_name="python3",
        resources={"metadata": {"path": str(work)}})
    cells = []
    failure = None
    async with client.async_setup_kernel():
        for index, cell in enumerate(nb.cells):
            frontend.messages = []
            try:
                if cell.cell_type == "code":
                    await client.async_execute_cell(cell, index)
                    await client.settle()
                    for action in cell.metadata.get("pltrs", {}).get("frontend", []):
                        frontend.act(action)
                        await client.settle()
            except (CellTimeoutError, DeadKernelError, TimeoutError) as err:
                failure = f"cell {cell.get('id')}: {type(err).__name__}: {err}"
            cells.append({
                "id": cell.get("id"),
                "outputs": cell.get("outputs", []),
                "messages": frontend.messages,
            })
            if failure:
                break
    return {"cells": cells, "failure": failure}


def warm_up():
    """Imports pyplot in a process of its own, which builds the font
    cache if it is missing."""
    subprocess.run([sys.executable, "-c", "import matplotlib.pyplot"], check=True)


def main(out, notebooks):
    """Executes *notebooks*, writing their records into *out*."""
    out.mkdir(parents=True, exist_ok=True)
    warm_up()
    for path in notebooks:
        work = out / "work" / path.stem
        work.mkdir(parents=True, exist_ok=True)
        record = asyncio.run(execute(path, work))
        (out / f"{path.stem}.json").write_text(json.dumps(record, indent=1))
        state = record["failure"] or "done"
        print(f"{path.name}: {state}", flush=True)


if __name__ == "__main__":
    if len(sys.argv) < 3:
        raise SystemExit("usage: execute.py OUT NOTEBOOK...")
    main(pathlib.Path(sys.argv[1]), [pathlib.Path(arg) for arg in sys.argv[2:]])
