#!/usr/bin/env python3
"""Benchmark entry point.

Usage examples
--------------
Run all algorithms against all test cases (requires FluidSynth + a soundfont):

    python run_benchmark.py --soundfont /path/to/soundfont.sf2

Keep the generated MIDI / WAV files after the run:

    python run_benchmark.py --soundfont sounds/GeneralUser.sf2 --keep

Run with a wider semitone tolerance (default 1):

    python run_benchmark.py --soundfont ... --tolerance 2

Filter to a subset of test cases by name substring:

    python run_benchmark.py --soundfont ... --filter triad

List registered algorithms without running anything:

    python run_benchmark.py --list-algorithms
"""

from __future__ import annotations

import argparse
import os
import sys

from generate_midi import generate_midi
from synthesize import synthesize
from evaluate import evaluate, DEFAULT_TOLERANCE
from report import print_report
from test_cases.definitions import TEST_CASES

# ── Import all algorithm implementations ──────────────────────────────────────
from algorithms.cqt_hps import CqtHpsDetector
from algorithms.librosa_cqt import LibrosaCqtDetector
from algorithms.librosa_piptrack import LibrosaPiptrackDetector

# ── Register algorithms ───────────────────────────────────────────────────────
#
# Add new DetectorAlgorithm subclasses here to include them in every benchmark
# run.  Each entry appears as a separate row in the ranking table.
#
ALGORITHMS = [
    # Rust port – exact default settings
    CqtHpsDetector(
        harmonics=2,
        name="CQT+HPS (Rust port, h=2)",
    ),
    # Rust-style with 3 harmonics
    CqtHpsDetector(
        harmonics=3,
        name="CQT+HPS (h=3)",
    ),
    # Rust-style with 5 harmonics – more selective but risks over-suppression
    CqtHpsDetector(
        harmonics=5,
        name="CQT+HPS (h=5)",
    ),
    # No HPS – shows raw CQT performance (same range as Rust port)
    CqtHpsDetector(
        harmonics=0,
        name="CQT only (no HPS, Rust range)",
    ),
    # Plain CQT from C1 (avoids very-low-frequency noise without HPS)
    LibrosaCqtDetector(
        relative_threshold=0.15,
        aggregation="mean",
        name="CQT simple (th=0.15, mean)",
    ),
    LibrosaCqtDetector(
        relative_threshold=0.10,
        aggregation="max",
        name="CQT simple (th=0.10, max)",
    ),
    # STFT pitch tracking
    LibrosaPiptrackDetector(
        frame_threshold=0.0,
        name="Piptrack",
    ),
]


# ── CLI ───────────────────────────────────────────────────────────────────────

def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="run_benchmark",
        description="Generate MIDI test cases, synthesize WAV, and rank note-detection algorithms.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    p.add_argument(
        "--soundfont", "-sf",
        metavar="PATH",
        default="soundfonts/MS_Basic.sf3",
        help="Path to a soundfont for FluidSynth synthesis "
             "(default: soundfonts/MS_Basic.sf3).",
    )
    p.add_argument(
        "--output-dir", "-o",
        default="benchmark_output",
        metavar="DIR",
        help="Directory for generated MIDI and WAV files (default: benchmark_output).",
    )

    p.add_argument(
        "--tolerance",
        type=int,
        default=DEFAULT_TOLERANCE,
        metavar="N",
        help=f"Semitone tolerance for note matching (default: {DEFAULT_TOLERANCE}).",
    )
    p.add_argument(
        "--filter",
        metavar="SUBSTRING",
        default=None,
        help="Only run test cases whose name contains SUBSTRING.",
    )
    p.add_argument(
        "--list-algorithms",
        action="store_true",
        help="Print registered algorithms and exit.",
    )
    return p


def main() -> None:
    args = build_parser().parse_args()

    # ── --list-algorithms ─────────────────────────────────────────────────────
    if args.list_algorithms:
        print(f"\n{len(ALGORITHMS)} registered algorithms:\n")
        for i, algo in enumerate(ALGORITHMS, 1):
            print(f"  {i:2d}. {algo.name}")
            if algo.description:
                print(f"       {algo.description}")
        print()
        return


    # ── Apply test-case filter ────────────────────────────────────────────────
    test_cases = TEST_CASES
    if args.filter:
        test_cases = [tc for tc in TEST_CASES if args.filter in tc.name]
        if not test_cases:
            print(
                f"error: no test cases match filter '{args.filter}'.\n"
                f"Available: {', '.join(tc.name for tc in TEST_CASES)}",
                file=sys.stderr,
            )
            sys.exit(1)
        print(f"Filter '{args.filter}' matched {len(test_cases)} test case(s).")

    midi_dir = os.path.join(args.output_dir, "midi")
    wav_dir  = os.path.join(args.output_dir, "wav")
    os.makedirs(midi_dir, exist_ok=True)
    os.makedirs(wav_dir,  exist_ok=True)

    print(
        f"\nBenchmark configuration:\n"
        f"  Test cases:  {len(test_cases)}\n"
        f"  Algorithms:  {len(ALGORITHMS)}\n"
        f"  Soundfont:   {args.soundfont}\n"
        f"  Tolerance:   ±{args.tolerance} semitone(s)\n"
        f"  Output dir:  {args.output_dir}\n"
    )

    results = []

    # ── Generate → Synthesize → Evaluate ─────────────────────────────────────
    for i, test_case in enumerate(test_cases, 1):
        prefix = f"[{i:>3}/{len(test_cases)}] {test_case.name}"
        print(f"{prefix:<45}", end="", flush=True)

        midi_path = os.path.join(midi_dir, f"{test_case.name}.mid")
        if not os.path.isfile(midi_path):
            midi_path = generate_midi(test_case, midi_dir)

        wav_path = os.path.join(wav_dir, f"{test_case.name}.wav")
        if not os.path.isfile(wav_path):
            try:
                synthesize(midi_path, wav_path, args.soundfont)
            except Exception as exc:
                print(f"\n  ERROR during synthesis: {exc}", file=sys.stderr)
                sys.exit(1)

        for algo in ALGORITHMS:
            result = evaluate(algo, test_case, wav_path, tolerance=args.tolerance)
            results.append(result)

        # Quick per-row summary: fraction of algorithms that were perfect
        perfect = sum(1 for r in results if r.test_case.name == test_case.name and r.perfect)
        print(f"  {perfect}/{len(ALGORITHMS)} algorithms perfect")

    # ── Report ────────────────────────────────────────────────────────────────
    print_report(results)

    print(f"Files kept in {args.output_dir!r}.")


if __name__ == "__main__":
    main()
