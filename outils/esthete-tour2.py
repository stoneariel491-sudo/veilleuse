"""Les mesures du second esthete, relues sur les vignettes que le banc ecrit.

Le banc `mesure` rend ; ce script lit ses PNG et ne rend rien. Il recalcule la
distance signee a la superellipse avec la meme formule que `distance()` dans
`recherche/verre-taille/src/mesure.rs`, pour pouvoir lire les COINS, que
plusieurs mesures du banc ecartent.

Lancer depuis n'importe ou : python outils/esthete-tour2.py [section]
"""
import os
import sys

import numpy as np
from PIL import Image

RACINE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CAPTURES = os.path.join(RACINE, "recherche", "verre-taille", "captures", "mesure")

# Rec. 709 sur l'espace lineaire, comme `luminance_relative` du banc.
LUMA = np.array([0.2126, 0.7152, 0.0722])


def etat(p):
    """Meme loi que `etat_du_ressort` du banc."""
    l = round(88 + p * (320 - 88))
    h = round(36 + p * (148 - 36))
    return (l // 2) * 2, (h // 2) * 2, float(np.float32(18 + p * 30)), float(np.float32(2.5 + p * 1.5))


def distance(l, h, r, n):
    """Distance signee au bord et masque des coins, par pixel."""
    y, x = np.mgrid[0:h, 0:l].astype(np.float64)
    px = x + 0.5 - l / 2
    py = y + 0.5 - h / 2
    qx = np.abs(px) - l / 2 + r
    qy = np.abs(py) - h / 2 + r
    coin = (qx > 0) & (qy > 0)
    a = np.clip(qx, 1e-9, None)
    b = np.clip(qy, 1e-9, None)
    f = (a ** n + b ** n) ** (1 / n)
    g = np.sqrt((a ** (n - 1)) ** 2 + (b ** (n - 1)) ** 2) / f ** (n - 1)
    mx = np.maximum(qx, 0)
    my = np.maximum(qy, 0)
    droit = np.sqrt(mx * mx + my * my) + np.minimum(np.maximum(qx, qy), 0) - r
    d = np.where(coin, (f - r) / g, droit)
    # La meme zone de coin que `pic_du_coin` du banc.
    zc = ((x < r) | (x > l - r)) & ((y < r) | (y > h - r))
    return d, zc


def lire(nom):
    return np.asarray(Image.open(os.path.join(CAPTURES, nom + ".png")).convert("RGB")).astype(np.float64)


def vers_lineaire(c):
    c = c / 255.0
    return np.where(c > 0.04045, ((c + 0.055) / 1.055) ** 2.4, c / 12.92)


def vers_srgb(c):
    return np.where(c > 0.0031308, 1.055 * np.clip(c, 0, None) ** (1 / 2.4) - 0.055, c * 12.92)


def niveau(rgb):
    """Le gris encode de meme luminance, en niveaux 0-255."""
    return 255.0 * vers_srgb(vers_lineaire(rgb) @ LUMA)


def aplats():
    """Objection 1 : la portee de la matiere sur un aplat, coins COMPRIS."""
    print("APLATS -- ecart a la vignette du verre seul, bord droit puis coin")
    for nom_etat, p in [("replie", 0.0), ("mi-chemin", 0.5), ("deploye", 1.0)]:
        l, h, r, n = etat(p)
        d, zc = distance(l, h, r, n)
        b = 12 * h / 148
        for fond in ["clair", "sombre"]:
            v = lire(f"esthete-{nom_etat}-{fond}-verre")[..., 1]
            for dirn in ["ancienne", "retenue"]:
                t = lire(f"esthete-{nom_etat}-{fond}-{dirn}")[..., 1]
                ecart = np.abs(t - v) > 2
                dedans = d < -0.45
                morceaux = []
                for zone, m in [("droit", ~zc), ("coin", zc)]:
                    mm = dedans & m
                    portee = (-d[mm & ecart]).max() if (mm & ecart).any() else 0.0
                    pt = t[mm & (d >= -1.5)].max()
                    pv = v[mm & (d >= -1.5)].max()
                    morceaux.append(
                        f"{zone} portee {portee:5.2f} px ({100 * portee / (h / 2):4.1f} %) pic {pt:5.1f} / verre {pv:5.1f} ({pt - pv:+5.1f})"
                    )
                print(f"  {nom_etat:9} {fond:6} {dirn:8} b={b:5.2f} | " + " | ".join(morceaux))


if __name__ == "__main__":
    quoi = sys.argv[1] if len(sys.argv) > 1 else "tout"
    if quoi in ("aplats", "tout"):
        aplats()
