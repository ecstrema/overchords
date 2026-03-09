"""MIDI file generator for benchmark test cases.

Writes a single MIDI file per TestCase: all notes sound simultaneously, are
held for `test_case.duration` seconds, then released together.
"""

import os

import mido

from test_cases.definitions import TestCase


def generate_midi(test_case: TestCase, output_dir: str = "midi_files") -> str:
    """Generate a MIDI file for *test_case* and save it under *output_dir*.

    Returns the path to the written file.
    """
    os.makedirs(output_dir, exist_ok=True)
    filepath = os.path.join(output_dir, f"{test_case.name}.mid")

    mid = mido.MidiFile()
    track = mido.MidiTrack()
    mid.tracks.append(track)

    ticks_per_beat: int = 480
    mid.ticks_per_beat = ticks_per_beat
    bpm: int = 120

    # Convert duration (seconds) → ticks.
    # At 120 BPM: 1 beat = 0.5 s   →   1 tick = 0.5 / 480 s
    beats = test_case.duration * (bpm / 60.0)
    note_ticks = int(round(beats * ticks_per_beat))

    # Small silence before the chord so FluidSynth has time to initialise its
    # reverb tail and the attack transient doesn't get clipped.
    silence_ticks = int(round(0.1 * (bpm / 60.0) * ticks_per_beat))

    # Tempo message
    track.append(mido.MetaMessage("set_tempo", tempo=mido.bpm2tempo(bpm), time=0))

    # All note-on events are simultaneous (delta-time = 0 for every note
    # after the first, which itself comes after the short silence).
    for i, note in enumerate(test_case.notes):
        delta = silence_ticks if i == 0 else 0
        track.append(
            mido.Message("note_on", note=note, velocity=test_case.velocity, time=delta)
        )

    # All note-off events are simultaneous, offset by note_ticks from the
    # last note-on (first note-off carries the full duration, the rest = 0).
    for i, note in enumerate(test_case.notes):
        delta = note_ticks if i == 0 else 0
        track.append(
            mido.Message("note_off", note=note, velocity=0, time=delta)
        )

    # Brief tail so FluidSynth renders the full release of the last note.
    tail_ticks = int(round(0.5 * (bpm / 60.0) * ticks_per_beat))
    track.append(mido.MetaMessage("end_of_track", time=tail_ticks))

    mid.save(filepath)
    return filepath
