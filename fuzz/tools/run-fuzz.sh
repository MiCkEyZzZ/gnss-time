#!/usr/bin/env bash
# Run the fuzz_gps_utc target the way it is meant to be run.
#
# -max_len: the harness consumes at most 12 bytes (RAW needs 9, BOUNDARY 12).
#   Without this cap libFuzzer lets inputs grow to KBs, wasting memory on
#   bytes the harness ignores.
# -dict:    byte-tokens that drop the fuzzer straight into boundary-heavy
#           numeric regions.
# -rss_limit_mb: headroom for ASAN.
#
# Usage:
#   ./tools/run-fuzz.sh                       # 300 s
#   MAX_TOTAL_TIME=600 ./tools/run-fuzz.sh
#   ./tools/run-fuzz.sh -runs=100000
set -euo pipefail

cd "$(dirname "$0")/.."  # fuzz/

timeout_seconds="${MAX_TOTAL_TIME:-300}"

cargo fuzz run fuzz_gps_utc -- \
    -max_total_time="$timeout_seconds" \
    -max_len=12 \
    -dict=fuzz_gps_utc.dict \
    "$@"