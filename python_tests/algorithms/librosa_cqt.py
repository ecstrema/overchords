"""Simple CQT detector without HPS – useful as a no-HPS baseline.

Uses a higher-range CQT (C1–C8, MIDI 24–108) to avoid the very low
frequency artefacts that appear around MIDI 0 when HPS is disabled.
The algorithm is intentionally minimal so its results highlight what
HPS actually contributes.
"""

from __future__ import annotations

import numpy as np

try:
    import librosa
except ImportError as exc:
    raise ImportError("librosa is required: pip install librosa") from exc

from .base import DetectorAlgorithm

_FMIN: float = librosa.note_to_hz("C1")   # MIDI 24
_N_BINS: int = 84                           # C1 to C8 (7 octaves × 12)
_BINS_PER_OCTAVE: int = 12
_HOP_LENGTH: int = 512
_MIDI_BASE: int = 24


class LibrosaCqtDetector(DetectorAlgorithm):
    """CQT magnitude detector (no HPS) operating from C1 to C8.

    Parameters
    ----------
    relative_threshold:
        Discard bins below this fraction of the peak magnitude.
    notes_to_keep:
        Maximum number of notes returned.
    aggregation:
        How to collapse the time axis: ``"mean"`` (default) or ``"max"``.
    name:
        Optional display name override.
    """

    def __init__(
        self,
        relative_threshold: float = 0.15,
        notes_to_keep: int = 5,
        aggregation: str = "mean",
        name: str | None = None,
    ) -> None:
        self.relative_threshold = relative_threshold
        self.notes_to_keep = notes_to_keep
        self.aggregation = aggregation
        self.name = name or (
            f"CQT only (th={relative_threshold:.2f}, top={notes_to_keep}, {aggregation})"
        )
        self.description = (
            f"librosa CQT, {aggregation} over time, "
            f"threshold={relative_threshold}, top-{notes_to_keep}, no HPS."
        )

    def detect(self, audio: np.ndarray, sample_rate: int) -> list[int]:
        C = librosa.cqt(
            audio,
            sr=sample_rate,
            hop_length=_HOP_LENGTH,
            fmin=_FMIN,
            n_bins=_N_BINS,
            bins_per_octave=_BINS_PER_OCTAVE,
        )  # (n_bins, n_frames), complex

        mag = np.abs(C)

        if self.aggregation == "max":
            bin_magnitudes = mag.max(axis=1)
        else:
            bin_magnitudes = mag.mean(axis=1)

        peak = float(bin_magnitudes.max())
        if peak == 0.0:
            return []
        norm = bin_magnitudes / peak

        above = np.where(norm >= self.relative_threshold)[0]
        if len(above) == 0:
            return []

        order = np.argsort(norm[above])[::-1]
        top_bins = above[order][: self.notes_to_keep]

        return [int(_MIDI_BASE + b) for b in top_bins]
