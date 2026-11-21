#!/usr/bin/env python3
"""Capture the milestone screenshots, verifying each one and retrying blanks.

Why this exists
---------------
``game --capture <bookmark>`` is intermittently blank: it occasionally writes a PNG containing
only the HUD over a flat clear-colour background, with no 3D geometry, even though the HUD in
that same image reports terrain as loaded. See
``docs/issues/2026-11-21-flaky-screenshot-capture.md`` for the investigation and for the proper
fix (render to an offscreen image target instead of the window swapchain).

Until that fix lands, a batch of captures must be **verified, not trusted**. This script does
that: it runs each bookmark, checks the resulting PNG actually contains a rendered scene, and
re-runs the ones that do not.

Usage
-----
    python tools/scripts/capture_screenshots.py                 # all bookmarks, debug build
    python tools/scripts/capture_screenshots.py --release
    python tools/scripts/capture_screenshots.py noon altitude   # only these

Exits non-zero if any bookmark could not be captured successfully, so CI and the
``screenshot-review`` skill can rely on the return code.
"""

from __future__ import annotations

import argparse
import collections
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SCREENSHOT_DIR = REPO_ROOT / "screenshots"

# A real render of this world contains sky, water, sand, grass and rock. A blank capture is the
# clear colour plus a few hundred antialiased HUD glyph pixels, which lands around 50 distinct
# colours; a real one lands in the thousands. 400 separates them with a wide margin.
MIN_DISTINCT_COLOURS = 400

# A real render fills most of the frame. If one colour covers nearly everything, the scene did
# not draw even if dithering produced some colour variety.
MAX_DOMINANT_FRACTION = 0.90

DEFAULT_ATTEMPTS = 4


def load_rgb(path: Path) -> tuple[int, int, list[tuple[int, int, int]]]:
    """Returns (width, height, pixels) for a PNG, using Pillow."""
    from PIL import Image  # imported lazily so --help works without Pillow

    with Image.open(path) as image:
        rgb = image.convert("RGB")
        return rgb.width, rgb.height, list(rgb.getdata())


def inspect(path: Path) -> tuple[bool, str]:
    """Returns (looks_rendered, human readable reason)."""
    if not path.exists():
        return False, "no file written"
    if path.stat().st_size == 0:
        return False, "empty file"

    width, height, pixels = load_rgb(path)

    # Ignore the top of the frame: that is where the HUD sits, and it is present either way.
    start = (height // 3) * width
    body = pixels[start:]
    if not body:
        return False, "image too small to inspect"

    counts = collections.Counter(body)
    distinct = len(counts)
    _, dominant = counts.most_common(1)[0]
    dominant_fraction = dominant / len(body)

    if distinct < MIN_DISTINCT_COLOURS:
        return False, f"only {distinct} distinct colours below the HUD (blank frame)"
    if dominant_fraction > MAX_DOMINANT_FRACTION:
        return False, f"one colour covers {dominant_fraction:.0%} of the frame"

    return True, f"{distinct} distinct colours, {width}x{height}"


def binary_path(release: bool) -> Path:
    profile = "release" if release else "debug"
    name = "game.exe" if sys.platform == "win32" else "game"
    return REPO_ROOT / "target" / profile / name


def list_bookmarks(binary: Path) -> list[str]:
    result = subprocess.run(
        [str(binary), "--list-bookmarks"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=True,
    )
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def capture(binary: Path, bookmark: str, timeout: int) -> None:
    subprocess.run(
        [str(binary), "--capture", bookmark],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bookmarks", nargs="*", help="bookmarks to capture (default: all)")
    parser.add_argument("--release", action="store_true", help="use the release build")
    parser.add_argument(
        "--attempts",
        type=int,
        default=DEFAULT_ATTEMPTS,
        help=f"attempts per bookmark before giving up (default {DEFAULT_ATTEMPTS})",
    )
    parser.add_argument("--timeout", type=int, default=300, help="seconds per attempt")
    args = parser.parse_args()

    binary = binary_path(args.release)
    if not binary.exists():
        profile = "--release " if args.release else ""
        print(f"error: {binary} not found. Run: cargo build {profile}-p game", file=sys.stderr)
        return 1

    wanted = args.bookmarks or list_bookmarks(binary)
    SCREENSHOT_DIR.mkdir(exist_ok=True)

    failures: list[str] = []
    for bookmark in wanted:
        path = SCREENSHOT_DIR / f"{bookmark}.png"
        path.unlink(missing_ok=True)

        for attempt in range(1, args.attempts + 1):
            try:
                capture(binary, bookmark, args.timeout)
            except subprocess.TimeoutExpired:
                print(f"{bookmark:<10} attempt {attempt}: timed out")
                continue

            ok, reason = inspect(path)
            if ok:
                print(f"{bookmark:<10} OK       {reason}")
                break
            print(f"{bookmark:<10} attempt {attempt}: {reason}")
            path.unlink(missing_ok=True)
        else:
            print(f"{bookmark:<10} FAILED   after {args.attempts} attempts")
            failures.append(bookmark)

    print()
    if failures:
        print(f"{len(failures)} of {len(wanted)} bookmarks failed: {', '.join(failures)}")
        return 1
    print(f"all {len(wanted)} bookmarks captured and verified")
    return 0


if __name__ == "__main__":
    sys.exit(main())
