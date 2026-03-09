"""Abstract base class for note-detection algorithms."""

from __future__ import annotations

from abc import ABC, abstractmethod

import numpy as np


class DetectorAlgorithm(ABC):
    """Interface that every detection algorithm must implement.

    Subclasses should set the class-level ``name`` and optionally
    ``description`` attributes, and implement :meth:`detect`.
    """

    #: Short human-readable name shown in reports and tables.
    name: str = "Unnamed algorithm"
    #: Optional longer description of the algorithm and its parameters.
    description: str = ""

    @abstractmethod
    def detect(self, audio: np.ndarray, sample_rate: int) -> list[int]:
        """Return the MIDI note numbers believed to be present in *audio*.

        Args:
            audio:       Mono float32 audio samples in [-1, 1].
            sample_rate: Sample rate of *audio* in Hz.

        Returns:
            A list of MIDI note numbers (0–127).  The list may be empty.
            Duplicates should not be returned.
        """

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(name={self.name!r})"
