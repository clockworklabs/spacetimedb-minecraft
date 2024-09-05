#!/bin/bash

set -euo pipefail

spacetime generate --out-dir crates/mc173-server/src/autogen --lang rust --delete-files -S
