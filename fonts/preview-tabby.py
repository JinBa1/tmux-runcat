#!/usr/bin/env python3
"""Compatibility entry point; preview-cats.py now supports all six coats."""

from pathlib import Path
import runpy

if __name__ == "__main__":
    runpy.run_path(str(Path(__file__).with_name("preview-cats.py")), run_name="__main__")
