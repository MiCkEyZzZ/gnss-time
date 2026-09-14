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

    # ── fuzz_utc_to_gps ───────────────────────────────────────────────────────
    ug_dir = os.path.join(here, "..", "corpus", "fuzz_utc_to_gps")
    os.makedirs(ug_dir, exist_ok=True)
    for entry in os.listdir(ug_dir):
        os.remove(os.path.join(ug_dir, entry))

    count = 0

    # UTC epoch (1972-01-01), GPS epoch boundary (underflow edge), flip
    # instants and high-end anchors over the full u64 domain.
    UTC_TO_GPS_EPOCH_NS = 252_892_800_000_000_000  # GPS epoch as UTC
    RAW_UTC = [
        0,
        1,
        CPU_NS,
        UTC_TO_GPS_EPOCH_NS - 1,  # 1 ns below the GPS epoch → underflow
        UTC_TO_GPS_EPOCH_NS,  # exact GPS epoch boundary
        UTC_TO_GPS_EPOCH_NS + 1,
        UTC_TO_GPS_EPOCH_NS + CPU_NS,
        46_828_800_000_000_000,  # 1981-07-01 UTC flip instant
        1_167_264_000_000_000_000,  # 2017-01-01 UTC flip instant
        1_467_264_000_000_000_000,  # ~2026 (now)
        2**63,
        2**63 + CPU_NS * 100,
        2**64 - 1,  # u64::MAX — high-end / overflow paths
        2**64 - 1 - CPU_NS,
        (2**64 - 1) // 2,
    ]

    def raw_ug_seed(nanos: int) -> bytes:
        return b"\x00" + struct.pack("<Q", nanos)

    for nanos in RAW_UTC:
        with open(os.path.join(ug_dir, f"ug_raw_{count:04d}"), "wb") as fh:
            fh.write(raw_ug_seed(nanos))
        count += 1

    # small perturbations of the anchors so mutations have neighbours to grow
    for nanos in RAW_UTC:
        for delta in (-1, 1, -2, 2, -17, 17):
            perturbed = nanos + delta
            if perturbed < 0 or perturbed >= 2**64:
                continue
            with open(os.path.join(ug_dir, f"ug_raw_{count:04d}"), "wb") as fh:
                fh.write(raw_ug_seed(perturbed))
            count += 1

    # BOUNDARY seeds: UTC flip instant of every transition, jittered so the
    # 1 s ambiguity window is provably reached on every run.
    def boundary_ug_seed(idx: int, jitter_ns: int) -> bytes:
        # decoded idx = 1 + (byte1 % 18), so store byte1 = idx - 1
        byte1 = idx - 1
        return bytes([0x80, byte1]) + struct.pack("<q", jitter_ns) + b"\x00\x00"

    for idx, (tai_ns, off) in enumerate(TRANSITIONS, start=1):
        utc_flip = tai_ns - off * CPU_NS
        for jitter_ns in JITTERS:
            nanos = utc_flip + jitter_ns
            if nanos < 0:
                continue
            with open(os.path.join(ug_dir, f"ug_bd_{count:04d}"), "wb") as fh:
                fh.write(boundary_ug_seed(idx, jitter_ns))
            count += 1

    print(f"generated {count} seeds in {ug_dir}")

    # ── fuzz_try_extend ──────────────────────────────────────────────────────
    te_dir = os.path.join(here, "..", "corpus", "fuzz_try_extend")
    os.makedirs(te_dir, exist_ok=True)
    for entry in os.listdir(te_dir):
        os.remove(os.path.join(te_dir, entry))

    count = 0

    CPU = 1_000_000_000
    I32_MAX = 2**31 - 1
    I32_MIN = -(2**31)

    def write_te(name: str, data: bytes) -> None:
        nonlocal count
        with open(os.path.join(te_dir, f"te_{name}_{count:04d}"), "wb") as fh:
            fh.write(data)
        count += 1

    # ── RAW: full 12-byte entries ─────────────────────────────────────────────
    def te_raw_seed(flags: int, entries: list) -> bytes:
        out = bytes([flags])
        for tai, off in entries:
            out += struct.pack("<Qi", tai, off)
        return out

    # Start empty: 65 valid entries → the 65th is BufferFull (1..64 Ok).
    chain = [(t * CPU, 19 + t - 1) for t in range(1, 66)]  # off 19..83
    write_te("raw_empty_full", te_raw_seed(0x00, chain))

    # Start builtin (19 entries): 45 valid entries extend to 64, 46th is full.
    chain = [(1_167_300_000_000_000_000 + t * CPU, 37 + t) for t in range(1, 47)]
    write_te("raw_builtin_full", te_raw_seed(0x80, chain))

    # Known wraparound defect: empty + MAX offset then MIN offset is *accepted*
    # in release, while the i64 model says NonUnitIncrement.
    write_te("raw_wrap_max_min", te_raw_seed(0x00, [(100, I32_MAX), (200, I32_MIN)]))
    # Same, but with the exact boundary in between: still a defect (MAX+1).
    write_te("raw_wrap_max_max", te_raw_seed(0x00, [(100, I32_MAX), (200, I32_MAX)]))

    # Validation orders from an empty table (first entry arbitrary).
    write_te("raw_first_any", te_raw_seed(0x00, [(0, I32_MIN)]))
    write_te("raw_first_any2", te_raw_seed(0x00, [(0, I32_MAX)]))
    # Descending / equal threshold rejection on the *second* entry.
    write_te("raw_not_asc", te_raw_seed(0x00, [(100, 1), (100, 2)]))
    write_te("raw_not_asc2", te_raw_seed(0x00, [(100, 1), (99, 2)]))
    write_te("raw_non_unit", te_raw_seed(0x00, [(100, 1), (200, 3)]))
    write_te("raw_non_unit2", te_raw_seed(0x00, [(100, 1), (200, 1)]))
    # BufferFull has priority over order tests at len == RUNTIME_CAPACITY.
    chain = [(t * CPU, 19 + t - 1) for t in range(1, 65)]
    chain += [(64 * CPU, 0)]  # 65th entry at capacity, even invalid → BufferFull
    write_te("raw_full_priority", te_raw_seed(0x00, chain))

    # ── BOUNDARY: 2-byte walk entries ─────────────────────────────────────────
    def te_boundary_seed(flags: int, pairs: list) -> bytes:
        out = bytes([flags])
        for a, b in pairs:
            out += bytes([a, b])
        return out

    # a = tai delta byte (i8), b = offset selector (7 = 19 base default).
    # Empty: valid chain (63× a=1,b=0) then BufferFull on 64th/65th.
    write_te("bnd_empty_full", te_boundary_seed(0x40, [(1, 0)] * 66))
    # Builtin: 45 valid then 46th full.
    write_te("bnd_builtin_full", te_boundary_seed(0xC0, [(1, 0)] * 48))
    # Walk the NotStrictlyAscending edge: a=0 (tai == last).
    write_te("bnd_equal", te_boundary_seed(0x40, [(1, 0), (0, 0)]))
    # a negative: tai < last.
    write_te("bnd_lt", te_boundary_seed(0x40, [(1, 0), (0xFF, 0)]))
    # NonUnitIncrement walks: sel 1 (last), 2 (last+2), 5 (0).
    write_te("bnd_non_unit", te_boundary_seed(0x40, [(1, 1), (1, 2), (1, 5)]))
    # MAX/Min walk: sel 3 then sel 4 via boundary (exercises wrap path
    # deterministically per run).
    write_te("bnd_wrap", te_boundary_seed(0x40, [(1, 3), (1, 4)]))
    # Offsets from the builtin start: sel 6 = 37 → NonUnitIncrement.
    write_te("bnd_builtin_off", te_boundary_seed(0xC0, [(1, 6)]))
    # Tai far future then ramping chain.
    write_te("bnd_ramp", te_boundary_seed(0x40, [(1, 0)] * 5 + [(9, 0)] * 5))

    print(f"generated {count} seeds in {te_dir}")

    # ── fuzz_leap_lookup ────────────────────────────────────────────────────
    # Format: byte0 = flags (0x80 builtin start, 0x40 RAW build, 0x20 BOUNDARY
    # build, 0x10 also static provider, 0x08 BOUNDARY instants), byte1 = build
    # entry count, then RAW (12-byte) / BOUNDARY (2-byte) entries, then lookup
    # instants (RAW 8-byte u64 TAI / BOUNDARY 2-byte selector+jitter).
    ll_dir = os.path.join(here, "..", "corpus", "fuzz_leap_lookup")
    os.makedirs(ll_dir, exist_ok=True)
    for entry in os.listdir(ll_dir):
        os.remove(os.path.join(ll_dir, entry))

    count = 0

    def write_ll(name: str, flags: int, build: bytes, instants: list) -> None:
        nonlocal count
        data = (
            bytes([flags, len(build) // (12 if flags & 0x40 else 2)])
            if (flags & 0x40 or flags & 0x20)
            else bytes([flags, 0])
        )
        data += build
        for inst in instants:
            data += inst
        with open(os.path.join(ll_dir, f"ll_{name}_{count:04d}"), "wb") as fh:
            fh.write(data)
        count += 1

    def raw_inst(tai: int) -> bytes:
        return struct.pack("<Q", tai)

    # Builtin table thresholds (entries[0] is the 19 s base at 0).
    BUILTIN = [(0, 19)] + TRANSITIONS

    def ll_boundary_inst(selector: int, jitter: int) -> bytes:
        # selector 0x00..0x7F scaled to table length; 0xFF = raw anchor.
        return bytes([selector & 0xFF, jitter & 0xFF])

    # RAW instants straddling every builtin threshold (exact, +-1, +-CPU_NS).
    for tai_ns, off in BUILTIN:
        for d in (-CPU_NS, -1, 0, 1, CPU_NS):
            t = tai_ns + d
            if t < 0 or t >= 2**64:
                continue
            write_ll("raw_thr", 0x80, b"", [raw_inst(t)])
    # RAW extremes and the pre-first fallback.
    for t in (0, 1, CPU_NS - 1, 2**63, 2**63 + CPU_NS, 2**64 - 1 - CPU_NS, 2**64 - 1):
        write_ll("raw_x", 0x80, b"", [raw_inst(t)])
    # BOUNDARY instants: selector scaled to the 19-entry builtin, jitter +-1,0.
    for idx in range(0, 19):
        sel = idx * 256 // 19
        for jitter in (-1, 0, 1):
            write_ll("bnd_thr", 0x88, b"", [ll_boundary_inst(sel, jitter)])
    # BOUNDARY raw anchors.
    write_ll(
        "bnd_anchor", 0x88, b"", [ll_boundary_inst(0xFF, 1), ll_boundary_inst(0xFF, 0)]
    )
    # Static provider alongside runtime builtin (same instants both paths).
    for t in (46_828_820_000_000_000 + 1, 1_167_264_037_000_000_000 - 1):
        write_ll("static", 0x90, b"", [raw_inst(t)])
    # Empty table: instants must all report the 19 s fallback.
    for t in (0, 2**32, 2**64 - 1):
        write_ll("empty", 0x00, b"", [raw_inst(t)])
    # RAW build from empty: valid unit chain then lookup around last threshold.
    chain = b"".join(struct.pack("<Qi", t * CPU_NS, 19 + t - 1) for t in range(1, 9))
    write_ll("raw_build", 0x40, chain, [raw_inst(8 * CPU_NS + 1), raw_inst(9 * CPU_NS)])
    # RAW build from builtin: extend 38 s (valid) then adjacent lookups.
    ext = struct.pack("<Qi", 1_167_264_038_000_000_000, 38)
    write_ll(
        "raw_ext38",
        0xC0,
        ext,
        [raw_inst(1_167_264_037_000_000_000 - 1), raw_inst(1_167_264_038_000_000_000)],
    )
    # BOUNDARY build: ascending chain from empty then boundary instants.
    bnd_build = bytes([1, 0]) * 6
    write_ll("bnd_build", 0x20, bnd_build, [raw_inst(6 * CPU_NS), raw_inst(7 * CPU_NS)])

    print(f"generated {count} seeds in {ll_dir}")


if __name__ == "__main__":
    main()
