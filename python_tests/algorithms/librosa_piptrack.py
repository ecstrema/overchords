"""STFT-based pitch detector using ``librosa.piptrack``.

``piptrack`` tracks sinusoidal peaks in consecutive STFT frames and returns
a (frequencies, magnitudes) pair.  This detector accumulates per-note
magnitude over all frames and returns the top-N most prominent pitches.
"""

from __future__ import annotations

from collections import defaultdict

import numpy as np

try:
    import librosa
except ImportError as exc:
    raise ImportError("librosa is required: pip install librosa") from exc

from .base import DetectorAlgorithm


def _hz_to_midi(hz: float) -> int:
    """Convert a frequency in Hz to the nearest MIDI note number."""
    return int(round(69.0 + 12.0 * np.log2(hz / 440.0)))


class LibrosaPiptrackDetector(DetectorAlgorithm):
    """STFT pitch tracker using ``librosa.piptrack``.

    Accumulates per-pitch magnitude across all STFT frames, then returns the
    top-N pitches by total accumulated energy.

    Parameters
    ----------
    frame_threshold:
        Per-frame minimum magnitude before a pitch is counted (librosa uses
        this as its internal threshold; lower = more sensitive but noisier).
    notes_to_keep:
        Maximum number of notes returned.
    name:
        Optional display name override.
    """

    def __init__(
        self,
        frame_threshold: float = 0.0,
        notes_to_keep: int = 5,
        name: str | None = None,
    ) -> None:
        self.frame_threshold = frame_threshold
        self.notes_to_keep = notes_to_keep
        self.name = name or (
            f"Piptrack (ft={frame_threshold:.2f}, top={notes_to_keep})"
        )
        self.description = (
            "librosa.piptrack: STFT sinusoidal peak picking, "
            f"frame_threshold={frame_threshold}, top-{notes_to_keep} by total energy."
        )

    def detect(self, audio: np.ndarray, sample_rate: int) -> list[int]:
        pitches, magnitudes = librosa.piptrack(
            y=audio,
            sr=sample_rate,
            threshold=self.frame_threshold,
            fmin=librosa.note_to_hz("C1"),
            fmax=librosa.note_to_hz("C8"),
        )
        # pitches:    (n_fft_bins, n_frames)  – Hz estimate or 0 when absent
        # magnitudes: (n_fft_bins, n_frames)  – corresponding magnitude

        note_energy: dict[int, float] = defaultdict(float)

        for t in range(pitches.shape[1]):
            for f in range(pitches.shape[0]):
                hz = pitches[f, t]
                mag = magnitudes[f, t]
                if hz > 0.0 and mag > 0.0:
                    midi = _hz_to_midi(hz)
                    if 0 <= midi <= 127:
                        note_energy[midi] += float(mag)

        if not note_energy:
            return []

        # Return the top-N notes by accumulated energy.
        sorted_notes = sorted(note_energy, key=note_energy.__getitem__, reverse=True)
        return sorted_notes[: self.notes_to_keep]
