"""CQT + Harmonic Product Spectrum detector – faithful Python port of audio.rs.

Algorithm summary (mirrors the Rust implementation exactly):
1. Compute a Constant-Q Transform with one bin per semitone, spanning
   MIDI 0 (C-1, 8.176 Hz) to MIDI 108 (C8, 4186 Hz).
2. Average bin magnitudes across all CQT frames in the chunk.
3. Apply Harmonic Product Spectrum (HPS) in the log-frequency / semitone
   domain using the same pre-computed harmonic offsets as the Rust code.
4. Normalise to the peak value.
5. Keep all bins above RELATIVE_THRESHOLD, then truncate to the top-N
   strongest bins.
6. Map bin indices to MIDI note numbers (bin 0 → MIDI 0).
"""

from __future__ import annotations

import numpy as np

try:
    import librosa
except ImportError as exc:
    raise ImportError("librosa is required: pip install librosa") from exc

from .base import DetectorAlgorithm

# ── Constants mirrored from audio.rs ─────────────────────────────────────────
_CQT_MIN_FREQ: float = 8.176      # C-1  = MIDI 0
_CQT_MAX_FREQ: float = 4186.0     # C8   ≈ MIDI 108
_BINS_PER_OCTAVE: int = 12        # one bin per semitone
_HOP_LENGTH: int = 512
_N_BINS: int = int(round(_BINS_PER_OCTAVE * np.log2(_CQT_MAX_FREQ / _CQT_MIN_FREQ)))
_MIDI_OFFSET: int = 0             # bin 0 → MIDI 0

_RELATIVE_THRESHOLD: float = 0.15
_NOTES_TO_KEEP: int = 5

# Semitone offsets for harmonics k=2..8 (log₂(k) * 12, rounded):
#   k=2 → 12  (octave)
#   k=3 → 19  (octave + perfect fifth)
#   k=4 → 24  (two octaves)
#   k=5 → 28  (two octaves + major third)
#   k=6 → 31  (two octaves + perfect fifth)
#   k=7 → 34  (two octaves + minor seventh)
#   k=8 → 36  (three octaves)
_HARMONIC_OFFSETS: tuple[int, ...] = (12, 19, 24, 28, 31, 34, 36)


def _apply_hps_cqt(magnitudes: np.ndarray, harmonics: int) -> np.ndarray:
    """Harmonic Product Spectrum in semitone-spaced CQT domain.

    Mirrors ``apply_hps_cqt`` in ``audio.rs`` exactly:
    - Each bin is multiplied by the original magnitude at each harmonic offset.
    - Bins whose harmonic falls outside the array are zeroed.
    - A square-root is applied at the end to partially undo the multiplicative
      compounding (same as the Rust ``hps.iter().map(|&v| v.sqrt())``).
    """
    n = len(magnitudes)
    hps = magnitudes.copy()
    for offset in _HARMONIC_OFFSETS[:harmonics]:
        end = n - offset
        if end <= 0:
            hps[:] = 0.0
            break
        # Vectorised version of the inner loop in Rust
        hps[:end] *= magnitudes[offset : offset + end]
        hps[end:] = 0.0
    return np.sqrt(np.maximum(hps, 0.0))


class CqtHpsDetector(DetectorAlgorithm):
    """Direct Python port of the Rust CQT + HPS algorithm in ``audio.rs``.

    Parameters
    ----------
    harmonics:
        Number of harmonic offsets to use in HPS (0 disables HPS entirely).
        The Rust default is 2 (uses offsets 12 and 19).
    relative_threshold:
        Bins below this fraction of the peak are discarded (Rust default 0.15).
    notes_to_keep:
        Maximum number of notes returned (Rust default 5).
    name:
        Override the display name (useful when instantiating multiple
        variants with different hyper-parameters).
    """

    def __init__(
        self,
        harmonics: int = 2,
        relative_threshold: float = _RELATIVE_THRESHOLD,
        notes_to_keep: int = _NOTES_TO_KEEP,
        name: str | None = None,
    ) -> None:
        self.harmonics = harmonics
        self.relative_threshold = relative_threshold
        self.notes_to_keep = notes_to_keep
        self.name = name or (
            f"CQT+HPS (h={harmonics}, th={relative_threshold:.2f}, top={notes_to_keep})"
        )
        self.description = (
            "Rust port: CQT at 12 bins/octave, mean across frames, "
            f"HPS with {harmonics} harmonic(s), relative threshold "
            f"{relative_threshold}, top-{notes_to_keep} notes."
        )

    def detect(self, audio: np.ndarray, sample_rate: int) -> list[int]:
        # ── CQT ──────────────────────────────────────────────────────────────
        C = librosa.cqt(
            audio,
            sr=sample_rate,
            hop_length=_HOP_LENGTH,
            fmin=_CQT_MIN_FREQ,
            n_bins=_N_BINS,
            bins_per_octave=_BINS_PER_OCTAVE,
        )  # shape: (n_bins, n_frames), complex

        bin_magnitudes: np.ndarray = np.abs(C).mean(axis=1)  # (n_bins,)

        # ── HPS ──────────────────────────────────────────────────────────────
        if self.harmonics > 0:
            bin_magnitudes = _apply_hps_cqt(bin_magnitudes, self.harmonics)

        # ── Normalise to peak ─────────────────────────────────────────────────
        peak = float(bin_magnitudes.max())
        if peak == 0.0:
            return []
        norm = bin_magnitudes / peak

        # ── Threshold + top-N ────────────────────────────────────────────────
        above_mask = norm >= self.relative_threshold
        above_indices = np.where(above_mask)[0]
        if len(above_indices) == 0:
            return []

        # Sort descending by normalised magnitude, keep top-N
        order = np.argsort(norm[above_indices])[::-1]
        top_bins = above_indices[order][: self.notes_to_keep]

        return [int(_MIDI_OFFSET + b) for b in top_bins]
