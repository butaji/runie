#!/usr/bin/env python3
"""Compare Herdr visible ANSI frames as fixed terminal cells."""

import json
import argparse
import pathlib
import re
import sys

SGR = re.compile(r"\x1b\[([0-9;]*)m")
# (bold, dim, italic, underline, inverse, foreground, background)
DEFAULT = (False, False, False, False, False, "default", "default")


def parse(path):
    rows = []
    for raw in pathlib.Path(path).read_text().splitlines():
        style = DEFAULT
        cells = []
        cursor = 0
        for match in SGR.finditer(raw):
            cells.extend((char, style) for char in raw[cursor : match.start()])
            nums = [int(value or 0) for value in match.group(1).split(";")]
            i = 0
            while i < len(nums):
                code = nums[i]
                if code == 0:
                    style = DEFAULT
                elif code == 1:
                    style = (True, *style[1:])
                elif code == 2:
                    style = (style[0], True, *style[2:])
                elif code == 3:
                    style = (*style[:2], True, *style[3:])
                elif code == 4:
                    style = (*style[:3], True, *style[4:])
                elif code == 7:
                    style = (*style[:4], True, *style[5:])
                elif code == 22:
                    style = (False, False, *style[2:])
                elif code == 23:
                    style = (*style[:2], False, *style[3:])
                elif code == 24:
                    style = (*style[:3], False, *style[4:])
                elif code == 27:
                    style = (*style[:4], False, *style[5:])
                elif code == 39:
                    style = (*style[:5], "default", style[6])
                elif code == 49:
                    style = (*style[:6], "default")
                elif 30 <= code <= 37 or 90 <= code <= 97:
                    style = (*style[:5], code, style[6])
                elif 40 <= code <= 47 or 100 <= code <= 107:
                    style = (*style[:6], code)
                elif code in (38, 48) and i + 2 < len(nums) and nums[i + 1] == 5:
                    color = ("idx", nums[i + 2])
                    style = (*style[:5], color, style[6]) if code == 38 else (*style[:6], color)
                    i += 2
                elif code in (38, 48) and i + 4 < len(nums) and nums[i + 1] == 2:
                    color = ("rgb", *nums[i + 2 : i + 5])
                    style = (*style[:5], color, style[6]) if code == 38 else (*style[:6], color)
                    i += 4
                i += 1
            cursor = match.end()
        cells.extend((char, style) for char in raw[cursor:])
        rows.append(cells)
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("left")
    parser.add_argument("right")
    parser.add_argument("--cols", type=int)
    parser.add_argument("--rows", type=int)
    parser.add_argument(
        "--attributes-only",
        action="store_true",
        help="compare terminal styles while ignoring glyph text",
    )
    args = parser.parse_args()
    left, right = parse(args.left), parse(args.right)
    height = args.rows or max(len(left), len(right))
    width = args.cols or max((len(row) for row in left + right), default=0)
    glyphs = styles = 0
    hotspots = []
    for y in range(height):
        lrow = left[y] if y < len(left) else []
        rrow = right[y] if y < len(right) else []
        row_diff = 0
        for x in range(width):
            lc = lrow[x] if x < len(lrow) else (" ", DEFAULT)
            rc = rrow[x] if x < len(rrow) else (" ", DEFAULT)
            glyph_diff = lc[0] != rc[0]
            style_diff = lc[1] != rc[1]
            if style_diff:
                styles += 1
                row_diff += 1
            elif glyph_diff and not args.attributes_only:
                glyphs += 1
                row_diff += 1
        if row_diff:
            hotspots.append((row_diff, y + 1))
    differences = glyphs + styles
    result = {
        "geometry": {"width": width, "height": height},
        "different_cells": differences,
        "different_glyphs": glyphs,
        "different_styles_only": styles,
        "top_rows": [{"row": row, "different_cells": count} for count, row in sorted(hotspots, reverse=True)[:10]],
        "exact": differences == 0,
    }
    print(json.dumps(result, indent=2))
    return 0 if result["exact"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
