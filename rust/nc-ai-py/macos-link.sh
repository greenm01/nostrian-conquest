#!/bin/sh
# cc, then realign the output's string pool; see .cargo/config.toml.
cc "$@" || exit
out=a.out
prev=
for arg in "$@"; do
    [ "$prev" = "-o" ] && out=$arg
    prev=$arg
done
exec python3 "$(dirname "$0")/macos_strpool.py" "$out"
