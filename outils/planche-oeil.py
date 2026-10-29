"""Assemble les neuf images composees de l'oeil en une planche a juger.

    python outils/planche-oeil.py captures/integration/pas3-oeil

L'oeil (`--oeil=dossier`) ecrit, pour chaque scene, ce que Windows compose a
l'ecran : `<fond>-<forme>-compose.png`. Cet outil les range en une grille de
trois lignes et trois colonnes, et ecrit `planche.png` dans le meme dossier :

- lignes : la nuit de l'horloge, le blanc pur, l'aplat `#0078D4` ;
- colonnes : replie, mi-course (p = 0,5, figee), deploye.

Chaque image garde ses pixels : aucune mise a l'echelle, aucun filtrage. Les
cases sont separees par une gouttiere grise de 8 px, #808080, qui n'est aucune
des valeurs lues par l'oeil. Une case sans image reste grise, et l'outil le dit.

Le decodeur est celui de `comparer-captures.py` : aucune bibliotheque d'images
n'est requise.
"""

import importlib.util
import os
import struct
import sys
import zlib

FONDS = ["nuit", "blanc", "couleur"]
FORMES = ["replie", "mi-course", "deploye"]
GOUTTIERE = 8
GRIS = (128, 128, 128)


def charger_decodeur():
    ici = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("comparer", os.path.join(ici, "comparer-captures.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.lire_png


def ecrire_png(chemin, largeur, hauteur, rgb):
    def morceau(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    pas = largeur * 3
    brut = bytearray()
    for j in range(hauteur):
        brut.append(0)
        brut += rgb[j * pas : (j + 1) * pas]
    with open(chemin, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(morceau(b"IHDR", struct.pack(">IIBBBBB", largeur, hauteur, 8, 2, 0, 0, 0)))
        f.write(morceau(b"IDAT", zlib.compress(bytes(brut), 9)))
        f.write(morceau(b"IEND", b""))


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        sys.exit(2)
    dossier = sys.argv[1]
    lire_png = charger_decodeur()
    images, manquent = {}, []
    for fond in FONDS:
        for forme in FORMES:
            chemin = os.path.join(dossier, f"{fond}-{forme}-compose.png")
            if os.path.exists(chemin):
                images[(fond, forme)] = lire_png(chemin)
            else:
                manquent.append(chemin)
    if not images:
        print("aucune image composee dans", dossier)
        sys.exit(1)
    l = max(i[0] for i in images.values())
    h = max(i[1] for i in images.values())
    L = 3 * l + 4 * GOUTTIERE
    H = 3 * h + 4 * GOUTTIERE
    toile = bytearray(bytes(GRIS) * (L * H))
    for r, fond in enumerate(FONDS):
        for c, forme in enumerate(FORMES):
            if (fond, forme) not in images:
                continue
            li, hi, canaux, px = images[(fond, forme)]
            ox = GOUTTIERE + c * (l + GOUTTIERE)
            oy = GOUTTIERE + r * (h + GOUTTIERE)
            for j in range(hi):
                for i in range(li):
                    k = (j * li + i) * canaux
                    d = ((oy + j) * L + ox + i) * 3
                    toile[d : d + 3] = px[k : k + 3]
    sortie = os.path.join(dossier, "planche.png")
    ecrire_png(sortie, L, H, toile)
    print(f"-> {sortie} ({L} x {H}) ; lignes {FONDS}, colonnes {FORMES}")
    for m in manquent:
        print("!! image absente, case grise :", m)


if __name__ == "__main__":
    main()
