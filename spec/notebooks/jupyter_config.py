"""
The Jupyter configuration of every process ``cargo xtask notebooks`` runs
(it copies this file into their configuration directory,
``JUPYTER_CONFIG_DIR``), which Jupyter's applications read (``jupyter
nbconvert --execute``, as matplotlib's notebook tests run it) and
``execute.py`` reads for its client.

A kernel's manager picks the kernel's TCP ports by binding them, frees
them and starts the kernel, which binds them again: any process of the
machine can take one in between (a connection's own port, another run's
kernel), and the kernel then dies (``Address already in use``). On Linux,
the kernels talk to their clients over Unix sockets of the abstract
namespace instead, named after a random id per process (the sockets of
its kernels end in their numbers), which no other process can take and
which leave no files behind. Elsewhere they keep TCP.
"""

import sys
import uuid

if sys.platform == "linux":
    c.KernelManager.transport = "ipc"  # noqa: F821 (the configuration's)
    c.KernelManager.ip = f"@pltrs-kernel-{uuid.uuid4().hex}"  # noqa: F821
