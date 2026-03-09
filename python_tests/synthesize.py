"""FluidSynth-based MIDI → WAV synthesizer.

Requires FluidSynth to be installed and on PATH, plus a .sf2 soundfont file.

Windows install:
    scoop install fluidsynth        # via Scoop
    choco install fluidsynth        # via Chocolatey

Linux install:
    sudo apt install fluidsynth     # Debian / Ubuntu

macOS install:
    brew install fluid-synth

Free soundfonts:
    GeneralUser GS  – https://schristiancollins.com/generaluser.php
    MuseScore General – https://musescore.org/en/handbook/soundfonts-and-sfz-files
    FluidR3_GM.sf2  – ships with many Linux distros (fluid-soundfont-gm package)
"""

import os
import shutil
import subprocess


def synthesize(midi_path: str, wav_path: str, soundfont_path: str) -> str:
    """Render *midi_path* to *wav_path* using FluidSynth.

    Returns the path to the written WAV file.

    Raises:
        RuntimeError:    If FluidSynth is not on PATH or returns a non-zero exit code.
        FileNotFoundError: If *soundfont_path* does not exist.
    """
    if not shutil.which("fluidsynth"):
        raise RuntimeError(
            "fluidsynth not found on PATH.\n"
            "  Windows:  scoop install fluidsynth   (or choco install fluidsynth)\n"
            "  Linux:    sudo apt install fluidsynth\n"
            "  macOS:    brew install fluid-synth"
        )

    if not os.path.isfile(soundfont_path):
        raise FileNotFoundError(
            f"Soundfont not found: {soundfont_path}\n"
            "Download one from https://schristiancollins.com/generaluser.php"
        )

    os.makedirs(os.path.dirname(os.path.abspath(wav_path)), exist_ok=True)

    cmd = [
        "fluidsynth",
        "-ni",             # non-interactive, no MIDI I/O driver
        "-F", wav_path,    # output file (WAV)
        "-r", "44100",     # sample rate
        "-g", "1.0",       # gain
        soundfont_path,
        midi_path,
    ]

    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        raise RuntimeError(
            f"FluidSynth exited with code {result.returncode}.\n"
            f"stdout: {result.stdout}\n"
            f"stderr: {result.stderr}"
        )

    if not os.path.isfile(wav_path):
        raise RuntimeError(
            f"FluidSynth returned exit code 0 but {wav_path} was not created.\n"
            f"stdout: {result.stdout}\nstderr: {result.stderr}"
        )

    return wav_path
