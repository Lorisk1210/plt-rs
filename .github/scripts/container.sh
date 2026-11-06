#!/bin/sh
# Builds and tests pltrs (Python) inside a manylinux or musllinux container
# (quay.io/pypa/...), with the source tree as the working directory:
#
#   .github/scripts/container.sh build          the wheel of the container's platform, into dist/
#   .github/scripts/container.sh test DIST...   DIST... installed and checked with the container's
#                                               oldest Python (python/tools/check_dist.py --install)
#
# The build links against the container's C library (glibc 2.17 in
# manylinux2014, musl 1.2 in musllinux_1_2), and the backend tags the wheel
# with what the library it built needs. The test runs the wheel on that
# same C library, with the numpy and Pillow pip picks for it.
set -eu

python=/opt/python/cp312-cp312/bin/python

case "${1:-}" in
    build)
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs |
            sh -s -- -y --profile minimal --default-toolchain none
        . "$HOME/.cargo/env"
        "$python" -m pip wheel --no-deps --wheel-dir dist .
        ;;
    test)
        shift
        "$python" python/tools/check_dist.py --install --python "$python" \
            --work /tmp/check-dist "$@"
        ;;
    *)
        echo "usage: $0 build | test DIST..." >&2
        exit 2
        ;;
esac
