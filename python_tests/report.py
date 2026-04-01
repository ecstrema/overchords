"""Reporting: print per-test details and an algorithm ranking table.

Uses ``tabulate`` if available; falls back to plain tab-separated text.
"""

from __future__ import annotations

from collections import defaultdict

from evaluate import EvaluationResult


# ── Helpers ───────────────────────────────────────────────────────────────────

def _midi_name(midi: int) -> str:
    """Return a human-readable note name for a MIDI note number, e.g. ``C4``."""
    names = ("C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B")
    octave = (midi // 12) - 1
    return f"{names[midi % 12]}{octave}"


def _note_list(midi_notes: list[int]) -> str:
    return ", ".join(_midi_name(n) for n in midi_notes) if midi_notes else "–"


def _note_score_list(note_scores: dict[int, float]) -> str:
    """Format detected notes with their detection fraction, e.g. ``C4(100%) E4(83%)``."""
    if not note_scores:
        return "–"
    return ", ".join(
        f"{_midi_name(n)}({s:.0%})" for n, s in sorted(note_scores.items())
    )


def _try_tabulate(rows: list, headers: list[str], tablefmt: str = "rounded_outline") -> str:
    try:
        from tabulate import tabulate  # type: ignore[import-untyped]
        return tabulate(rows, headers=headers, tablefmt=tablefmt, floatfmt=".3f")
    except ImportError:
        lines = ["\t".join(headers)]
        for row in rows:
            lines.append("\t".join(str(c) for c in row))
        return "\n".join(lines)


# ── Public API ────────────────────────────────────────────────────────────────

def print_report(results: list[EvaluationResult]) -> None:
    """Print a full benchmark report to *stdout*.

    Output is divided into two sections:
    1. Per-algorithm, per-test detail tables.
    2. Cross-algorithm ranking sorted by mean F1 (descending).
    """
    by_algo: dict[str, list[EvaluationResult]] = defaultdict(list)
    for r in results:
        by_algo[r.algorithm_name].append(r)

    # ── Section 1: per-test detail ────────────────────────────────────────────
    print("\n" + "=" * 90)
    print("DETAILED RESULTS BY ALGORITHM")
    print("=" * 90)

    for algo_name, algo_results in by_algo.items():
        print(f"\n── {algo_name} ──")
        rows = []
        for r in algo_results:
            status = "✓" if r.perfect else "✗"
            rows.append([
                status,
                r.test_case.name,
                _note_list(r.expected),
                _note_score_list(r.note_scores),
                f"{r.tp:.2f}",
                f"{r.fp:.2f}",
                f"{r.fn:.2f}",
                f"{r.precision:.2f}",
                f"{r.recall:.2f}",
                f"{r.f1:.2f}",
            ])
        headers = ["", "Test case", "Expected", "Detected", "TP", "FP", "FN",
                   "Prec", "Recall", "F1"]
        print(_try_tabulate(rows, headers))

    # ── Section 2: algorithm ranking ─────────────────────────────────────────
    print("\n" + "=" * 90)
    print("ALGORITHM RANKING  (sorted by mean F1 ↓)")
    print("=" * 90)

    summary: list[tuple[str, float, float, float, float, float, float, float]] = []
    for algo_name, algo_results in by_algo.items():
        n = len(algo_results)
        mean_f1        = sum(r.f1        for r in algo_results) / n
        mean_precision = sum(r.precision for r in algo_results) / n
        mean_recall    = sum(r.recall    for r in algo_results) / n
        total_tp = sum(r.tp for r in algo_results)
        total_fp = sum(r.fp for r in algo_results)
        total_fn = sum(r.fn for r in algo_results)
        perfect  = sum(1 for r in algo_results if r.perfect)
        summary.append((algo_name, mean_precision, mean_recall, mean_f1,
                        total_tp, total_fp, total_fn, perfect))

    summary.sort(key=lambda x: x[3], reverse=True)

    ranked_rows = []
    for rank, (name, prec, rec, f1, tp, fp, fn, perfect) in enumerate(summary, 1):
        medal = {1: "🥇", 2: "🥈", 3: "🥉"}.get(rank, f"#{rank}")
        ranked_rows.append([
            medal,
            name,
            f"{prec:.3f}",
            f"{rec:.3f}",
            f"{f1:.3f}",
            tp,
            fp,
            fn,
            f"{perfect}/{len(by_algo[name])}",
        ])

    headers = ["Rank", "Algorithm", "Precision", "Recall", "F1",
               "ΣTP", "ΣFP", "ΣFN", "Perfect"]
    print(_try_tabulate(ranked_rows, headers))
    print()
