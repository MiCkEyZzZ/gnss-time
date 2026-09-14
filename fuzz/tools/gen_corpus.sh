#!/usr/bin/env bash
# Regenerate the seed corpus for fuzz_gps_utc.
#
# The input format is versioned: whenever fuzz_gps_utc.rs changes its decode
# scheme, stale corpus entries silently do nothing. This script wipes the
# corpus and writes fresh RAW + BOUNDARY seeds on every run, so the fuzzer
# always starts from a domain that provably reaches every leap-second window.
set -euo pipefail

cd "$(dirname "$0")/.."  # fuzz/

python3 tools/gen_corpus.py
echo "corpus size: $(find corpus/fuzz_gps_utc -type f | wc -l) files"
echo
echo "next:  cargo fuzz run fuzz_gps_utc -- -max_total_time=300 -max_len=12 -dict=fuzz_gps_utc.dict"