"""Evaluation logic: run an algorithm on a WAV file and score its output.

The scoring uses a greedy nearest-neighbour match between detected notes
and expected notes with a configurable semitone tolerance.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np

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
    detected: list[int]   # detected MIDI notes, sorted
    expected: list[int]   # ground-truth MIDI notes, sorted

    tp: int               # true positives
    fp: int               # false positives (spurious detections)
    fn: int               # false negatives (missed notes)

    precision: float
    recall: float
    f1: float

    @property
    def perfect(self) -> bool:
        """True when every expected note is detected and nothing spurious."""
        return self.tp == len(self.expected) and self.fp == 0


def _greedy_match(
    detected: list[int],
    expected: list[int],
    tolerance: int,
) -> tuple[int, int, int]:
    """Greedy TP / FP / FN matching with semitone tolerance.

    Each expected note can only be matched once.  For each detected note we
    find the closest unmatched expected note within ±*tolerance* semitones
    and claim it as a true positive.

    Returns:
        (tp, fp, fn)
    """
    print(f"Matching detected {detected} to expected {expected}")
    remaining = list(expected)
    tp = 0
    for det in detected:
        best_idx: int | None = None
        best_dist = tolerance + 1
        for idx, exp in enumerate(remaining):
            dist = abs(det - exp)
            if dist <= tolerance and dist < best_dist:
                best_dist = dist
                best_idx = idx
        if best_idx is not None:
            tp += 1
            remaining.pop(best_idx)

    fp = len(detected) - tp
    fn = len(expected) - tp
    return tp, fp, fn


def load_audio(wav_path: str) -> tuple[np.ndarray, int]:
    """Load a WAV file as a mono float32 array."""
    audio, sr = sf.read(wav_path, dtype="float32", always_2d=False)
    if audio.ndim == 2:
        audio = audio.mean(axis=1)
    return audio, sr


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
    detected: list[int] = algorithm.detect(audio, sr)
    expected: list[int] = test_case.notes

    tp, fp, fn = _greedy_match(detected, expected, tolerance)

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
        detected=sorted(detected),
        expected=sorted(expected),
        tp=tp,
        fp=fp,
        fn=fn,
        precision=precision,
        recall=recall,
        f1=f1,
    )
