#!/bin/bash

set -euo pipefail

spacetime generate --out-dir crates/autogen/src/autogen --lang rust --delete-files
