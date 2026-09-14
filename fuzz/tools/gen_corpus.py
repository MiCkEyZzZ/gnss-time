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
import struct

CPU_NS = 1_000_000_000
GPS_TAI_OFFSET_NS = 19 * CPU_NS

# (tai_nanos_threshold, tai_minus_utc) for entries[1..=18] — the real
# leap-second transitions of the GPS era. Mirrors src/tables/leap_seconds.rs.
TRANSITIONS = [
    (46_828_820_000_000_000, 20),  # 1981-07-01
    (78_364_821_000_000_000, 21),  # 1982-07-01
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
    -20 * CPU_NS,  # far before the window
    -18 * CPU_NS,  # two-pass approximate-TAI hazard zone
    -10 * CPU_NS,
    -2 * CPU_NS,
    -CPU_NS - 1,  # just before the ambiguity window
    -CPU_NS,  # window edge
    -1,  # 1 ns before the exact flip
    0,  # the exact flip instant
    1,  # 1 ns after
    CPU_NS - 1,  # inside the window, just before it closes
    CPU_NS,  # window closed
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
    2**63,
    2**63 + CPU_NS * 100,
    2**64 - 1,  # u64::MAX → overflow paths
    2**64 - 1 - CPU_NS,
    (2**64 - 1) // 2,
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

    # ── fuzz_week_tow ──────────────────────────────────────────────────
    wt_dir = os.path.join(here, "..", "corpus", "fuzz_week_tow")
    os.makedirs(wt_dir, exist_ok=True)
    for entry in os.listdir(wt_dir):
        os.remove(os.path.join(wt_dir, entry))

    count = 0

    # RAW anchors over the u32/u64 week × tow domain (17-byte layout).
    RAW_WEEK_TOW = [
        (0, 0, 0),  # GPS epoch
        (0, 604_799, 999_999_999),  # last valid tow with week 0
        (1023, 0, 0),
        (1024, 0, 0),
        (2047, 0, 0),
        (30_499, 604_799, 999_999_999),  # huge but valid
        (30_500, 0, 0),  # week rim: valid with small tow
        (30_500, 7, 0),  # still valid
        (30_501, 0, 0),  # overflow by one week
        (30_501, 1, 0),  # overflow
        (65_535, 0, 0),
        (4_000_000_000, 0, 0),
        (2**32 - 1, 0, 0),  # week == u32::MAX
        (0, 604_800, 0),  # invalid: tow seconds == 604_800
        (0, 604_801, 0),  # invalid
        (0, 2**64 - 1, 0),  # invalid: tow seconds == u64::MAX
        (0, 0, 1_000_000_000),  # invalid: nanos == 1e9
        (0, 0, 2**32 - 1),  # invalid: nanos == u32::MAX
        (1, 604_799, 999_999_999),  # max valid tow, week 1
    ]

    def raw_wt_seed(week: int, secs: int, nanos: int) -> bytes:
        return b"\x00" + struct.pack("<IQI", week, secs, nanos)

    for week, secs, nanos in RAW_WEEK_TOW:
        with open(os.path.join(wt_dir, f"wt_raw_{count:04d}"), "wb") as fh:
            fh.write(raw_wt_seed(week, secs, nanos))
        count += 1

    # small perturbations around the raw anchors
    for week, secs, nanos in RAW_WEEK_TOW:
        for dw in (-1, 1):
            w = week + dw
            if w < 0 or w >= 2**32:
                continue
            for ds in (-1, 1):
                s = secs + ds
                if s < 0 or s >= 2**64:
                    continue
                for dn in (-1, 1):
                    n = nanos + dn
                    if n < 0 or n >= 2**32:
                        continue
                    with open(os.path.join(wt_dir, f"wt_raw_{count:04d}"), "wb") as fh:
                        fh.write(raw_wt_seed(w, s, n))
                    count += 1

    # BOUNDARY seeds: single-axis edges are independent — week affects only
    # Overflow, secs/nanos only InvalidInput. No cross-product needed:
    # one seed per axis edge (others at zero) + a handful of combos.
    WEEK_EDGES = [
        0,
        1,
        1023,
        1024,
        2047,
        2048,
        10_000,
        30_499,
        30_500,
        30_501,
        30_502,
        65_535,
        262_143,
        1_000_000,
        4_000_000_000,
        2**32 - 1,
    ]
    SECS_EDGES = [0, 1, 604_799, 604_800, 604_801, 2**32 - 1, 2**64 - 2, 2**64 - 1]
    NANOS_EDGES = [
        0,
        1,
        999_999_998,
        999_999_999,
        1_000_000_000,
        1_000_000_001,
        2**32 - 2,
        2**32 - 1,
    ]

    def wt_boundary_seed(wi: int, si: int, ni: int, jw: int, js: int, jn: int) -> bytes:
        return bytes([0x80, wi, si, ni]) + struct.pack("<iii", jw, js, jn)

    def write_wt(
        wi: int, si: int, ni: int, jw: int = 0, js: int = 0, jn: int = 0
    ) -> None:
        nonlocal count
        with open(os.path.join(wt_dir, f"wt_bd_{count:04d}"), "wb") as fh:
            fh.write(wt_boundary_seed(wi, si, ni, jw, js, jn))
        count += 1

    # One seed per week edge, tows at zero.
    for wi in range(len(WEEK_EDGES)):
        write_wt(wi, 0, 0)

    # One seed per secs edge, week and nanos at zero.
    for si in range(len(SECS_EDGES)):
        write_wt(0, si, 0)

    # One seed per nanos edge, week and secs at zero.
    for ni in range(len(NANOS_EDGES)):
        write_wt(0, 0, ni)

    # Overflow rim walks: jitter walks week across 30_500 → 30_501.
    for jw in (-4, -3, -2, -1, 1, 2, 3, 4):
        write_wt(WEEK_EDGES.index(30_500), 0, 0, jw=jw)

    # TOW-boundary jitter walks: secs across 604_799 → 604_800.
    for js in (-4, -2, -1, 1, 2, 4):
        write_wt(0, SECS_EDGES.index(604_799), 0, js=js)
        write_wt(0, SECS_EDGES.index(604_800), 0, js=js)

    # Nanos-boundary jitter walks: nanos across 999_999_999 → 1_000_000_000.
    for jn in (-2, -1, 1, 2):
        write_wt(0, 0, NANOS_EDGES.index(999_999_999), jn=jn)
        write_wt(0, 0, NANOS_EDGES.index(1_000_000_000), jn=jn)

    # Combos: intersecting valid / invalid classes.
    write_wt(
        WEEK_EDGES.index(30_500),
        SECS_EDGES.index(604_799),
        NANOS_EDGES.index(999_999_999),
    )  # max in-range
    write_wt(
        WEEK_EDGES.index(30_499),
        SECS_EDGES.index(604_799),
        NANOS_EDGES.index(999_999_999),
    )  # valid
    write_wt(
        WEEK_EDGES.index(30_501), SECS_EDGES.index(0), NANOS_EDGES.index(0)
    )  # overflow
    write_wt(
        WEEK_EDGES.index(30_500), SECS_EDGES.index(1), NANOS_EDGES.index(0)
    )  # week rim + a few seconds
    write_wt(
        WEEK_EDGES.index(0), SECS_EDGES.index(604_800), NANOS_EDGES.index(1_000_000_000)
    )  # double invalid
    write_wt(
        WEEK_EDGES.index(1), SECS_EDGES.index(604_799), NANOS_EDGES.index(999_999_999)
    )  # max valid tow

    print(f"generated {count} seeds in {wt_dir}")

    # ── fuzz_day_tod ─────────────────────────────────────────────────────────
    dd_dir = os.path.join(here, "..", "corpus", "fuzz_day_tod")
    os.makedirs(dd_dir, exist_ok=True)
    for entry in os.listdir(dd_dir):
        os.remove(os.path.join(dd_dir, entry))

    count = 0

    # RAW anchors over the u32/u64 day × tod domain (17-byte layout).
    RAW_DAY_TOD = [
        (0, 0, 0),  # GLONASS epoch
        (0, 86_399, 999_999_999),  # last valid tod with day 0
        (1_460, 0, 0),  # last day of the first 4-year interval
        (1_461, 0, 0),  # first day of the second 4-year interval
        (213_502, 0, 0),  # just below the overflow rim
        (213_503, 0, 0),  # day rim: valid with small tod
        (213_503, 7, 0),  # still valid
        (213_504, 0, 0),  # overflow by one day
        (213_504, 1, 0),  # overflow
        (65_535, 0, 0),
        (262_143, 0, 0),
        (2**32 - 1, 0, 0),  # day == u32::MAX
        (0, 86_400, 0),  # invalid: tod seconds == 86_400
        (0, 86_401, 0),  # invalid
        (0, 2**64 - 1, 0),  # invalid: tod seconds == u64::MAX
        (0, 0, 1_000_000_000),  # invalid: nanos == 1e9
        (0, 0, 2**32 - 1),  # invalid: nanos == u32::MAX
        (1, 86_399, 999_999_999),  # max valid tod, day 1
    ]

    def raw_dd_seed(day: int, secs: int, nanos: int) -> bytes:
        return b"\x00" + struct.pack("<IQI", day, secs, nanos)

    for day, secs, nanos in RAW_DAY_TOD:
        with open(os.path.join(dd_dir, f"dd_raw_{count:04d}"), "wb") as fh:
            fh.write(raw_dd_seed(day, secs, nanos))
        count += 1

    # small perturbations around the raw anchors
    for day, secs, nanos in RAW_DAY_TOD:
        for dw in (-1, 1):
            d = day + dw
            if d < 0 or d >= 2**32:
                continue
            for ds in (-1, 1):
                s = secs + ds
                if s < 0 or s >= 2**64:
                    continue
                for dn in (-1, 1):
                    n = nanos + dn
                    if n < 0 or n >= 2**32:
                        continue
                    with open(os.path.join(dd_dir, f"dd_raw_{count:04d}"), "wb") as fh:
                        fh.write(raw_dd_seed(d, s, n))
                    count += 1

    # BOUNDARY seeds: single-axis edges are independent — day affects only
    # Overflow, secs/nanos only InvalidInput. No cross-product needed.
    DAY_EDGES = [
        0,
        1,
        1_460,
        1_461,
        1_462,
        10_000,
        100_000,
        209_715,
        213_502,
        213_503,
        213_504,
        213_505,
        65_535,
        262_143,
        1_000_000,
        2**32 - 1,
    ]
    TOD_EDGES = [0, 1, 1_439, 86_399, 86_400, 86_401, 2**32 - 1, 2**64 - 1]
    NANOS_EDGES_DD = [
        0,
        1,
        999_999_998,
        999_999_999,
        1_000_000_000,
        1_000_000_001,
        2**32 - 2,
        2**32 - 1,
    ]

    def dd_boundary_seed(di: int, si: int, ni: int, jd: int, js: int, jn: int) -> bytes:
        return bytes([0x80, di, si, ni]) + struct.pack("<iii", jd, js, jn)

    def write_dd(
        di: int, si: int, ni: int, jd: int = 0, js: int = 0, jn: int = 0
    ) -> None:
        nonlocal count
        with open(os.path.join(dd_dir, f"dd_bd_{count:04d}"), "wb") as fh:
            fh.write(dd_boundary_seed(di, si, ni, jd, js, jn))
        count += 1

    # One seed per day edge, tods at zero.
    for di in range(len(DAY_EDGES)):
        write_dd(di, 0, 0)

    # One seed per tod-seconds edge, day and nanos at zero.
    for si in range(len(TOD_EDGES)):
        write_dd(0, si, 0)

    # One seed per nanos edge, day and tod at zero.
    for ni in range(len(NANOS_EDGES_DD)):
        write_dd(0, 0, ni)

    # Overflow rim walks: jitter walks day across 213_503 → 213_504.
    for jd in (-4, -3, -2, -1, 1, 2, 3, 4):
        write_dd(DAY_EDGES.index(213_503), 0, 0, jd=jd)

    # TOD-boundary jitter walks: secs across 86_399 → 86_400.
    for js in (-4, -2, -1, 1, 2, 4):
        write_dd(0, TOD_EDGES.index(86_399), 0, js=js)
        write_dd(0, TOD_EDGES.index(86_400), 0, js=js)

    # Nanos-boundary jitter walks: nanos across 999_999_999 → 1_000_000_000.
    for jn in (-2, -1, 1, 2):
        write_dd(0, 0, NANOS_EDGES_DD.index(999_999_999), jn=jn)
        write_dd(0, 0, NANOS_EDGES_DD.index(1_000_000_000), jn=jn)

    # Four-year interval walk: day across 1_460 → 1_461.
    for jd in (0, 1, -1):
        write_dd(DAY_EDGES.index(1_460), 0, 0, jd=jd)
        write_dd(DAY_EDGES.index(1_461), 0, 0, jd=jd)

    # Combos: intersecting valid / invalid classes.
    write_dd(
        DAY_EDGES.index(213_503),
        TOD_EDGES.index(86_399),
        NANOS_EDGES_DD.index(999_999_999),
    )  # max in-range
    write_dd(
        DAY_EDGES.index(213_502),
        TOD_EDGES.index(86_399),
        NANOS_EDGES_DD.index(999_999_999),
    )  # valid
    write_dd(
        DAY_EDGES.index(213_504), TOD_EDGES.index(0), NANOS_EDGES_DD.index(0)
    )  # overflow
    write_dd(
        DAY_EDGES.index(213_503), TOD_EDGES.index(1), NANOS_EDGES_DD.index(0)
    )  # day rim + a few seconds
    write_dd(
        DAY_EDGES.index(0), TOD_EDGES.index(86_400), NANOS_EDGES_DD.index(1_000_000_000)
    )  # double invalid
    write_dd(
        DAY_EDGES.index(1), TOD_EDGES.index(86_399), NANOS_EDGES_DD.index(999_999_999)
    )  # max valid tod

    print(f"generated {count} seeds in {dd_dir}")


if __name__ == "__main__":
    main()
