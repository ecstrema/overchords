"""Entry point alias – delegates to run_benchmark.main().

This file exists so that `uv run python main.py` also works, but the
primary way to run the benchmark is:

    python run_benchmark.py --soundfont /path/to/soundfont.sf2
"""
from run_benchmark import main

if __name__ == "__main__":
    main()
