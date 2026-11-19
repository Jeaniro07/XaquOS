#!/bin/bash
# helper: run a command in WSL archlinux against /root/ferrix (read-only browsing)
cd /root/ferrix || exit 1
export MSYS_NO_PATHCONV=1
bash -lc "$*"
