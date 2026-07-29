#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Prévisualise un .vtx (mosaïque G1) en ASCII, sans toucher au matériel.

Décode US (positionnement), SO, ESC 4x/5x (fg/bg) et REP, puis reconstitue la
grille de sous-pixels 80x72. Indispensable pour itérer sur une bannière sans
redéployer sur le Pi à chaque essai.

    ./vtx-preview.py logo.vtx [--rows 1-12]
"""
import sys

# code couleur Minitel -> niveau de luminosité 0-7 (inverse de MINITEL_COLORS)
LEVEL = {0: 0, 4: 1, 1: 2, 5: 3, 2: 4, 6: 5, 3: 6, 7: 7}


def render(data):
    px = [[0] * 80 for _ in range(72)]
    col = row = 1
    fg, bg, prev = 7, 0, None

    def put(col, row, b, fg, bg):
        if b is None:
            return
        bits = [b & 1, (b >> 1) & 1, (b >> 2) & 1,
                (b >> 3) & 1, (b >> 4) & 1, (b >> 6) & 1]  # p0..p5
        for k, (x, y) in enumerate([(0, 0), (1, 0), (0, 1), (1, 1), (0, 2), (1, 2)]):
            X, Y = (col - 1) * 2 + x, (row - 1) * 3 + y
            if 0 <= X < 80 and 0 <= Y < 72:
                px[Y][X] = 1 if (fg if bits[k] else bg) >= 4 else 0

    i = 0
    while i < len(data):
        b = data[i]
        if b == 0x1F:      # US : positionnement
            row, col = data[i + 1] - 0x40, data[i + 2] - 0x40
            i += 3
        elif b == 0x0E:    # SO : bascule G1
            i += 1
        elif b == 0x1B:    # ESC : attribut
            c = data[i + 1]
            if 0x40 <= c <= 0x47:
                fg = LEVEL[c - 0x40]
            elif 0x50 <= c <= 0x57:
                bg = LEVEL[c - 0x50]
            i += 2
        elif b == 0x12:    # REP
            for _ in range(data[i + 1] - 0x40):
                put(col, row, prev, fg, bg)
                col += 1
            i += 2
        else:
            put(col, row, b, fg, bg)
            prev = b
            col += 1
            i += 1
    return px


def main():
    a = sys.argv[1:]
    if not a:
        print("usage: vtx-preview.py <fichier.vtx> [--rows N-M]", file=sys.stderr)
        sys.exit(2)
    r0, r1 = 1, 24
    if "--rows" in a:
        r0, r1 = (int(v) for v in a[a.index("--rows") + 1].split("-"))
    px = render(open(a[0], "rb").read())
    for r in range(r0, r1 + 1):
        for y in range((r - 1) * 3, r * 3):
            print(f"{r:2}|" + "".join("█" if v else " " for v in px[y]) + "|")


if __name__ == "__main__":
    main()
