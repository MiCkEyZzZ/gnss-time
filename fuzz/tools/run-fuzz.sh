#!/usr/bin/env bash
# Run a fuzz target the way it is meant to be run.
#
# Usage:
#   ./tools/run-fuzz.sh fuzz_week_tow              # default 300 s
#   MAX_TOTAL_TIME=600 ./tools/run-fuzz.sh fuzz_gps_utc
#   ./tools/run-fuzz.sh fuzz_week_tow -runs=100000
set -euo pipefail

cd "$(dirname "$0")/.."  # fuzz/

target="${1:?usage: run-fuzz.sh <target> [extra-fuzzer-args...]}"
shift

timeout_seconds="${MAX_TOTAL_TIME:-300}"

# Per-target max input length matching the harness decode layout.
case "$target" in
    fuzz_gps_utc)     max_len=12; dict="fuzz_gps_utc.dict" ;;
    fuzz_utc_to_gps)  max_len=12; dict="fuzz_utc_to_gps.dict" ;;
    fuzz_week_tow)    max_len=17; dict="fuzz_week_tow.dict" ;;
    fuzz_day_tod)     max_len=17; dict="fuzz_day_tod.dict" ;;
    fuzz_try_extend)  max_len=793; dict="fuzz_try_extend.dict" ;;
    fuzz_leap_lookup)  max_len=922; dict="fuzz_leap_lookup.dict" ;;
    *) echo "unknown target: $target" >&2; exit 1 ;;
esac

cargo fuzz run "$target" -- \
    -max_total_time="$timeout_seconds" \
    -max_len="$max_len" \
    -dict="$dict" \
    -rss_limit_mb=4096 \
    "$@"
