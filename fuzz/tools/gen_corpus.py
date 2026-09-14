#!/usr/bin/env python3
"""Generate a seed corpus for the fuzz_gps_utc target.

Output format (little-endian), matching fuzz_targets/fuzz_gps_utc.rs:

RAW mode (data[0] & 0x80 == 0), len >= 9:
    byte 0          : 0x00
    bytes 1..9      : u64 GPS nanoseconds over the full domain

BOUNDARY mode (data[0] & 0x80 != 0), len >= 12:
    byte 0          : 0x80 | 0   (bit 128 set; bits 0..6 reserved)
    byte 1          : transition index offset (idx - 1, decoded as 1 + b % 18)
    bytes 2..10     : i64 jitter in [-20 s, +20 s)
    bytes 10..12    : 0x00, 0x00  (reserved)

Run from the fuzz/ directory:
    python3 tools/gen_corpus.py
"""

import os
import random
import struct

CPU_NS = 1_000_000_000
GPS_TAI_OFFSET_NS = 19 * CPU_NS

# (tai_nanos_threshold, tai_minus_utc) for entries[1..=18] — the real
# leap-second transitions of the GPS era. Mirrors src/tables/leap_seconds.rs.
TRANSITIONS = [
    (46_828_820_000_000_000, 20),   # 1981-07-01
    (78_364_821_000_000_000, 21),   # 1982-07-01
    (109_900_822_000_000_000, 22),  # 1983-07-01
    (173_059_223_000_000_000, 23),  # 1985-07-01
    (252_028_824_000_000_000, 24),  # 1988-01-01
    (315_187_225_000_000_000, 25),  # 1990-01-01
    (346_723_226_000_000_000, 26),  # 1991-01-01
    (393_984_027_000_000_000, 27),  # 1992-07-01
    (425_520_028_000_000_000, 28),  # 1993-07-01
    (457_056_029_000_000_000, 29),  # 1994-07-01
    (504_489_630_000_000_000, 30),  # 1996-01-01
    (551_750_431_000_000_000, 31),  # 1997-07-01
    (599_184_032_000_000_000, 32),  # 1999-01-01
    (820_108_833_000_000_000, 33),  # 2006-01-01
    (914_803_234_000_000_000, 34),  # 2009-01-01
    (1_025_136_035_000_000_000, 35),  # 2012-07-01
    (1_119_744_036_000_000_000, 36),  # 2015-07-01
    (1_167_264_037_000_000_000, 37),  # 2017-01-01
]

# Interesting jitter values (ns) around a transition's GPS instant.
JITTERS = [
    -20 * CPU_NS,             # far before the window
    -18 * CPU_NS,             # two-pass approximate-TAI hazard zone
    -10 * CPU_NS,
    -2 * CPU_NS,
    -CPU_NS - 1,             # just before the ambiguity window
    -CPU_NS,                 # window edge
    -1,                      # 1 ns before the exact flip
    0,                       # the exact flip instant
    1,                       # 1 ns after
    CPU_NS - 1,              # inside the window, just before it closes
    CPU_NS,                  # window closed
    CPU_NS + 1,
    2 * CPU_NS,
    10 * CPU_NS,
    18 * CPU_NS,
    20 * CPU_NS,
    -123_456_789,
    500_000_000,
]

# Raw high/low/extreme anchors over the full u64 domain.
RAW_ANCHORS = [
    0,
    1,
    CPU_NS,
    252_892_800_000_000_000,  # GPS epoch as UTC-from-1972 (conversion baseline)
    600_000_000_000_000_000,  # ~2000
    900_000_000_000_000_000,  # ~2008
    1_167_264_018_000_000_000,  # 2017-01-01 GPS
    1_467_264_000_000_000_000,  # ~2026 (now)
    2 ** 63,
    2 ** 63 + CPU_NS * 100,
    2 ** 64 - 1,              # u64::MAX → overflow paths
    2 ** 64 - 1 - CPU_NS,
    (2 ** 64 - 1) // 2,
]


def raw_seed(nanos: int) -> bytes:
    return b"\x00" + struct.pack("<Q", nanos)


def boundary_seed(idx: int, jitter_ns: int) -> bytes:
    # decoded idx = 1 + (byte1 % 18), so store byte1 = idx - 1
    byte1 = idx - 1
    return bytes([0x80, byte1]) + struct.pack("<q", jitter_ns) + b"\x00\x00"


def main() -> None:
    here = os.path.dirname(os.path.abspath(__file__))
    out_dir = os.path.join(here, "..", "corpus", "fuzz_gps_utc")
    os.makedirs(out_dir, exist_ok=True)

    # fresh start: the input format changed, stale seeds are useless
    for entry in os.listdir(out_dir):
        os.remove(os.path.join(out_dir, entry))

    count = 0

    for nanos in RAW_ANCHORS:
        with open(os.path.join(out_dir, f"raw_{count:04d}"), "wb") as fh:
            fh.write(raw_seed(nanos))
        count += 1

    # small perturbations of the anchors so mutations have neighbours to grow
    for nanos in RAW_ANCHORS:
        for delta in (-1, 1, -2, 2, -17, 17):
            perturbed = nanos + delta
            if perturbed < 0 or perturbed >= 2**64:
                continue
            with open(os.path.join(out_dir, f"raw_{count:04d}"), "wb") as fh:
                fh.write(raw_seed(perturbed))
            count += 1

    for idx, (tai_ns, _) in enumerate(TRANSITIONS, start=1):
        gps_flip = tai_ns - GPS_TAI_OFFSET_NS
        for jitter_ns in JITTERS:
            nanos = gps_flip + jitter_ns
            if nanos < 0:
                continue
            with open(os.path.join(out_dir, f"bd_{count:04d}"), "wb") as fh:
                fh.write(boundary_seed(idx, jitter_ns))
            count += 1

    print(f"generated {count} seeds in {out_dir}")


if __name__ == "__main__":
    main()