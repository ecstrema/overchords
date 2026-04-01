"""Evaluation logic: run an algorithm on a WAV file and score its output.

The scoring uses a greedy nearest-neighbour match between detected notes
and expected notes with a configurable semitone tolerance.
"""

from __future__ import annotations

from collections import defaultdict
from dataclasses import dataclass

import numpy as np

#: Audio is sliced into non-overlapping 4096-sample blocks before being passed
#: to :meth:`~algorithms.base.DetectorAlgorithm.detect`, mirroring how the Rust
#: backend feeds the ring-buffer one HOP_SIZE-aligned chunk at a time.
CHUNK_SIZE: int = 4096

try:
    import librosa
    import soundfile as sf
except ImportError as exc:
    raise ImportError(
        "librosa and soundfile are required: pip install librosa soundfile"
    ) from exc

from algorithms.base import DetectorAlgorithm
from test_cases.definitions import TestCase

#: Default tolerance in semitones for matching detected vs expected notes.
DEFAULT_TOLERANCE: int = 0


@dataclass
class EvaluationResult:
    """Scores for one (algorithm, test_case) pair."""

    test_case: TestCase
    algorithm_name: str
    note_scores: dict[int, float]  # MIDI note → fraction of chunks it was detected in (>0)
    detected: list[int]            # sorted(note_scores.keys())
    expected: list[int]            # ground-truth MIDI notes, sorted

    # Soft metrics: TP/FP/FN are weighted by detection fractions, so they are
    # real-valued in [0, n_notes].  A note detected in 80% of chunks contributes
    # 0.8 to TP (or FP) rather than the binary 0/1 of a hard threshold.
    tp: float   # sum of detection fractions for correctly matched notes
    fp: float   # sum of detection fractions for spurious (unmatched) notes
    fn: float   # n_expected − tp  (captures partial / missing detections)

    precision: float
    recall: float
    f1: float

    @property
    def perfect(self) -> bool:
        """True when every expected note was detected in every chunk and nothing spurious."""
        return self.fn < 1e-9 and self.fp < 1e-9


def _soft_greedy_match(
    note_scores: dict[int, float],
    expected: list[int],
    tolerance: int,
) -> tuple[float, float, float]:
    """Soft TP / FP / FN matching weighted by per-note detection fractions.

    Each detected note carries a score in (0, 1] equal to the fraction of
    chunks in which the algorithm reported it.  Greedy nearest-neighbour
    matching (same as before) assigns each detected note to an expected note
    within ±*tolerance* semitones; the matched score accumulates into soft_tp.
    Unmatched detected scores sum into soft_fp, and the shortfall against the
    expected count becomes soft_fn.

    Returns:
        (soft_tp, soft_fp, soft_fn)
    """
    remaining = list(expected)
    matched: set[int] = set()
    soft_tp = 0.0

    for det, score in sorted(note_scores.items()):
        best_idx: int | None = None
        best_dist = tolerance + 1
        for idx, exp in enumerate(remaining):
            dist = abs(det - exp)
            if dist <= tolerance and dist < best_dist:
                best_dist = dist
                best_idx = idx
        if best_idx is not None:
            soft_tp += score
            remaining.pop(best_idx)
            matched.add(det)

    soft_fp = sum(score for note, score in note_scores.items() if note not in matched)
    soft_fn = len(expected) - soft_tp
    return soft_tp, soft_fp, soft_fn


def load_audio(wav_path: str) -> tuple[np.ndarray, int]:
    """Load a WAV file as a mono float32 array."""
    audio, sr = sf.read(wav_path, dtype="float32", always_2d=False)
    if audio.ndim == 2:
        audio = audio.mean(axis=1)
    return audio, sr


def _split_chunks(audio: np.ndarray) -> list[np.ndarray]:
    """Return non-overlapping CHUNK_SIZE-sample slices (trailing samples discarded)."""
    n_chunks = len(audio) // CHUNK_SIZE
    return [audio[i * CHUNK_SIZE : (i + 1) * CHUNK_SIZE] for i in range(n_chunks)]


def evaluate(
    algorithm: DetectorAlgorithm,
    test_case: TestCase,
    wav_path: str,
    tolerance: int = DEFAULT_TOLERANCE,
) -> EvaluationResult:
    """Run *algorithm* on *wav_path* and return a scored :class:`EvaluationResult`.

    Parameters
    ----------
    algorithm:
        The :class:`~algorithms.base.DetectorAlgorithm` to evaluate.
    test_case:
        The :class:`~test_cases.definitions.TestCase` that describes the
        expected notes and metadata.
    wav_path:
        Path to the WAV file synthesized from *test_case*.
    tolerance:
        Semitone tolerance for matching (default 1).
    """
    audio, sr = load_audio(wav_path)

    chunks = _split_chunks(audio)
    note_counts: dict[int, int] = defaultdict(int)
    if chunks:
        # Simulate real-time: call detect() once per 4096-sample chunk and
        # accumulate how many chunks each note was detected in.
        for chunk in chunks:
            for note in algorithm.detect(chunk, sr):
                note_counts[note] += 1
    else:
        # Clip shorter than one chunk – process in full, score as 1.0 or 0.
        for note in algorithm.detect(audio, sr):
            note_counts[note] = 1

    n_chunks = max(len(chunks), 1)
    # Detection fraction for every note that appeared in at least one chunk.
    note_scores: dict[int, float] = {note: count / n_chunks for note, count in note_counts.items()}

    expected: list[int] = test_case.notes
    detected: list[int] = sorted(note_scores.keys())

    tp, fp, fn = _soft_greedy_match(note_scores, expected, tolerance)

    precision = tp / (tp + fp) if (tp + fp) > 0 else 0.0
    recall = tp / (tp + fn) if (tp + fn) > 0 else 0.0
    f1 = (
        2.0 * precision * recall / (precision + recall)
        if (precision + recall) > 0.0
        else 0.0
    )

    return EvaluationResult(
        test_case=test_case,
        algorithm_name=algorithm.name,
        note_scores=note_scores,
        detected=detected,
        expected=sorted(expected),
        tp=tp,
        fp=fp,
        fn=fn,
        precision=precision,
        recall=recall,
        f1=f1,
    )
