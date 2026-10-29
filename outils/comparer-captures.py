"""Compte les pixels qui different entre deux captures PNG.

Il existe pour une raison precise, et elle vaut d'etre ecrite : une comparaison
avant/apres d'une horloge a HEURE FIGEE rend forcement zero, et un zero ne
prouve rien tant qu'on n'a pas montre que le comparateur sait voir un ecart.
On l'appelle donc toujours DEUX fois — une fois sur le cas ou la reponse doit
etre zero, une fois sur un cas ou elle ne doit pas l'etre.

    python outils/comparer-captures.py avant.png apres.png

Le decodeur est ecrit ici plutot qu'importe : le poste n'a pas de bibliotheque
d'images installee, et un outil de preuve qui depend d'une installation n'est
pas disponible le jour ou il faut s'en servir.
"""

import struct
import sys
import zlib


def lire_png(chemin):
    d = open(chemin, "rb").read()
    assert d[:8] == b"\x89PNG\r\n\x1a\n", f"{chemin} n'est pas un PNG"
    i, idat, l, h, bits, coul = 8, b"", 0, 0, 0, 0
    while i < len(d):
        n = struct.unpack(">I", d[i : i + 4])[0]
        t, c = d[i + 4 : i + 8], d[i + 8 : i + 8 + n]
        if t == b"IHDR":
            l, h, bits, coul = struct.unpack(">IIBB", c[:10])
        elif t == b"IDAT":
            idat += c
        elif t == b"IEND":
            break
        i += 12 + n
    canaux = {0: 1, 2: 3, 4: 2, 6: 4}[coul]
    assert bits == 8, "seuls les PNG a huit bits sont lus"
    brut = zlib.decompress(idat)
    pas = l * canaux
    sortie, prec, p = bytearray(), bytearray(pas), 0
    for _ in range(h):
        f = brut[p]
        p += 1
        ligne = bytearray(brut[p : p + pas])
        p += pas
        for x in range(pas):
            a = ligne[x - canaux] if x >= canaux else 0
            b = prec[x]
            cc = prec[x - canaux] if x >= canaux else 0
            if f == 1:
                ligne[x] = (ligne[x] + a) & 255
            elif f == 2:
                ligne[x] = (ligne[x] + b) & 255
            elif f == 3:
                ligne[x] = (ligne[x] + (a + b) // 2) & 255
            elif f == 4:
                pa, pb, pc = abs(b - cc), abs(a - cc), abs(a + b - 2 * cc)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else cc)
                ligne[x] = (ligne[x] + pr) & 255
        sortie += ligne
        prec = ligne
    return l, h, canaux, bytes(sortie)


def comparer(x, y):
    la, ha, ca, pa = lire_png(x)
    lb, hb, cb, pb = lire_png(y)
    if (la, ha, ca) != (lb, hb, cb):
        return None
    n, pire = 0, 0
    for i in range(0, len(pa), ca):
        e = max(abs(pa[i + k] - pb[i + k]) for k in range(min(3, ca)))
        if e:
            n += 1
            pire = max(pire, e)
    return n, pire, la * ha


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(__doc__)
        sys.exit(2)
    r = comparer(sys.argv[1], sys.argv[2])
    if r is None:
        print("TAILLES DIFFERENTES — les deux captures ne se comparent pas.")
        sys.exit(1)
    n, pire, tot = r
    print(f"{n} pixels differents sur {tot}  (pire ecart {pire} niveaux)")
