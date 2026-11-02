"""Where the audit reads and writes: the repository, its generated data
(`target/api-audit/`) and the rustdoc JSON of the pltrs crates."""
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
DATA = os.path.join(ROOT, "target", "api-audit")
PORTING = os.path.join(ROOT, "crates", "pltrs", "src", "porting")
DOC = os.environ.get("API_AUDIT_DOC") or os.path.join(
    os.environ.get("CARGO_TARGET_DIR") or os.path.join(ROOT, "target"), "doc")
COVERAGE = os.path.join(HERE, "coverage.md")

os.makedirs(DATA, exist_ok=True)
