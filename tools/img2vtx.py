#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""image -> flux Vidéotex mosaïque G1 (.vtx) : quantification 2 couleurs
ADAPTATIVE par case 2x3 + couleur fg/bg par case + compression REP.

Bien meilleur qu'un seuil global : conserve nuances/dégradés (8 gris).

Usage: img2vtx.py <image> -o out.vtx [--cols 40] [--rows 24] [--disjoint]
"""
import sys
from math import sqrt
from operator import itemgetter
from PIL import Image

# Remap niveau de luminosité 0-7 -> code couleur Minitel (gris ordonnés).
MINITEL_COLORS = {0: 0, 1: 4, 2: 1, 3: 5, 4: 2, 5: 6, 6: 3, 7: 7}
ESCAPE, SHIFT_OUT, REP, US = 0x1B, 0x0E, 0x12, 0x1F


def eight_levels(level):
    try:
        return int(level * 8 / 256)
    except TypeError:  # tuple RGB
        return int(round(sqrt(0.299*level[0]**2 + 0.587*level[1]**2 + 0.114*level[2]**2)) * 8 / 256)


def two_colors(colors):
    levels = [0]*8
    for c in colors:
        levels[c] += 1
    pairs = sorted([(i, n) for i, n in enumerate(levels)], key=itemgetter(1), reverse=True)
    return pairs[0][0], pairs[1][0]


def pixel_to_color(p, bg, fg):
    return 0 if abs(bg-p) < abs(fg-p) else 1


def bg_seq(level):
    return [ESCAPE, 0x50 + MINITEL_COLORS.get(level, 0)]


def fg_seq(level):
    return [ESCAPE, 0x40 + MINITEL_COLORS.get(level, 7)]


def move_to(col, line):
    return [US, 0x40 + line, 0x40 + col]


def convert(img, cols, rows, disjoint, row_override=None):
    # cadre cible : cols*2 x rows*3 px, aspect préservé
    max_w, max_h = cols*2, rows*3
    w, h = img.size
    scale = min(max_w/w, max_h/h)
    nw = max(2, int(round(w*scale))) & ~1
    nh = max(3, int(round(h*scale)))
    nh -= nh % 3
    nw, nh = min(nw, max_w), min(nh, max_h)
    img = img.resize((nw, nh), Image.LANCZOS).convert("RGB")

    cw, ch = nw//2, nh//3
    col0 = 1 + (40 - cw)//2
    line0 = row_override if row_override else 1 + (24 - ch)//2

    out = bytearray()
    for row in range(ch):
        out += bytes(move_to(col0, line0 + row))
        out.append(SHIFT_OUT)
        if disjoint:
            out += bytes([ESCAPE, 0x5A])
        prev_bg = prev_fg = -1
        prev_char = 0
        repeat = 0
        for col in range(cw):
            px = [img.getpixel((col*2+x, row*3+y))
                  for x, y in [(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)]]
            px = [eight_levels(p) for p in px]
            bg, fg = two_colors(px)
            if disjoint and bg != 0:
                bg, fg = 0, bg
            bits = [pixel_to_color(p, bg, fg) for p in px]  # réduction binaire bg/fg
            # code = 0b0 p5 1 p4 p3 p2 p1 p0
            char = int("".join(["0", str(bits[5]), "1", str(bits[4]),
                                 str(bits[3]), str(bits[2]), str(bits[1]), str(bits[0])]), 2)
            if not disjoint and prev_bg == fg and prev_fg == bg:
                char ^= 0b01011111
                fg, bg = bg, fg
            if prev_bg == bg and prev_fg == fg and char == prev_char:
                repeat += 1
            else:
                if repeat > 0:
                    out += bytes([prev_char]) if repeat == 1 else bytes([REP, 0x40 + repeat])
                    repeat = 0
                if prev_bg != bg:
                    out += bytes(bg_seq(bg)); prev_bg = bg
                if prev_fg != fg:
                    out += bytes(fg_seq(fg)); prev_fg = fg
                out.append(char); prev_char = char
        if repeat > 0:
            out += bytes([prev_char]) if repeat == 1 else bytes([REP, 0x40 + repeat])
        if disjoint:
            out += bytes([ESCAPE, 0x59])
    return out, cw, ch


def convert_gray(img, cols, rows):
    """Mode NIVEAUX DE GRIS : chaque case = un gris uni (bloc plein 0x7F en
    couleur fg), 8 niveaux. Résolution cols×rows, vrai gris (pas de tramage)."""
    cols = min(cols, 40)
    rows = min(rows, 24)
    w, h = img.size
    scale = min(cols / w, rows / h)
    cw = max(1, min(cols, int(round(w * scale))))
    ch = max(1, min(rows, int(round(h * scale))))
    img = img.resize((cw, ch), Image.LANCZOS).convert("RGB")
    col0 = 1 + (40 - cw) // 2
    line0 = 1 + (24 - ch) // 2

    FULL = 0x7F  # bloc plein en G1
    out = bytearray()
    for row in range(ch):
        out += bytes(move_to(col0, line0 + row))
        out.append(SHIFT_OUT)
        prev_fg = -1
        repeat = 0
        for col in range(cw):
            lvl = eight_levels(img.getpixel((col, row)))
            if lvl == prev_fg:
                repeat += 1
            else:
                if repeat > 0:
                    out += bytes([FULL]) if repeat == 1 else bytes([REP, 0x40 + repeat])
                    repeat = 0
                out += bytes(fg_seq(lvl))
                out.append(FULL)
                prev_fg = lvl
        if repeat > 0:
            out += bytes([FULL]) if repeat == 1 else bytes([REP, 0x40 + repeat])
    return out, cw, ch


def main():
    a = sys.argv[1:]
    if not a:
        print("usage: img2vtx.py <image> -o out.vtx [--cols N] [--rows N] [--disjoint]", file=sys.stderr)
        sys.exit(2)
    path = a[0]
    out = "out.vtx"
    cols, rows, disjoint, gray = 40, 24, False, False
    row_override = None
    i = 1
    while i < len(a):
        if a[i] in ("-o", "--out"): out = a[i+1]; i += 2
        elif a[i] == "--cols": cols = int(a[i+1]); i += 2
        elif a[i] == "--rows": rows = int(a[i+1]); i += 2
        elif a[i] == "--disjoint": disjoint = True; i += 1
        elif a[i] == "--gray": gray = True; i += 1
        elif a[i] == "--row": row_override = int(a[i+1]); i += 2
        else: i += 1
    img = Image.open(path)
    if gray:
        data, cw, ch = convert_gray(img, cols, rows)
    else:
        data, cw, ch = convert(img, cols, rows, disjoint, row_override)
    open(out, "wb").write(data)
    print(f"{path} -> {cw}x{ch} cases, {len(data)} octets -> {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
