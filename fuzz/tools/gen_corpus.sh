#!/usr/bin/env bash
# Regenerate the seed corpus for all fuzz targets.
#
# The input format is versioned: whenever a target changes its decode scheme,
# stale corpus entries silently do nothing. This script wipes the corpus and
# writes fresh RAW + BOUNDARY seeds on every run, so the fuzzer always starts
# from a domain that provably reaches every target's edge set.
set -euo pipefail

cd "$(dirname "$0")/.."  # fuzz/

python3 tools/gen_corpus.py
echo "corpus size: $(find corpus/fuzz_gps_utc -type f | wc -l) files (fuzz_gps_utc), $(find corpus/fuzz_utc_to_gps -type f | wc -l) files (fuzz_utc_to_gps), $(find corpus/fuzz_week_tow -type f | wc -l) files (fuzz_week_tow), $(find corpus/fuzz_day_tod -type f | wc -l) files (fuzz_day_tod), $(find corpus/fuzz_try_extend -type f | wc -l) files (fuzz_try_extend)"
echo
echo "next:  ./tools/run-fuzz.sh fuzz_gps_utc   # or: fuzz_utc_to_gps, fuzz_week_tow, fuzz_day_tod, fuzz_try_extend"
