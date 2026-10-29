"""Le verre compose par Windows contre le rendu hors-ecran de l'application.

Le relevé (`--releve-bureau`) laisse dans son dossier :

- `verre-fond.png`, le bureau que l'ilot a televerse dans son verre ;
- `verre-avant.png`, l'ecran capture avec l'ilot NON exclu, contenu cache :
  ce que DWM a reellement compose, verre et bureau autour ;
- dans `releve.txt`, la ligne `VERRE-DIRECT` qui dit ou tout cela se trouve.

Cet outil refait le verre HORS-ECRAN sur `verre-fond.png` (`--capture-ilot
--verre-fond --verre-decalage --sans-contenu`), le compose sur les octets comme
DWM, et compare a `verre-avant.png`. `outils/verre-contre-banc.py` a deja montre
que le rendu hors-ecran retombe sur le banc ; si celui-ci retombe sur l'ecran,
l'ecran retombe sur le banc.

    python outils/verre-en-direct.py <veilleuse.exe> <dossier du relevé>

Deux temoins :

- **le fond est-il reste le meme ?** Hors du galet, `verre-avant` doit egaler
  `verre-fond` pixel pour pixel. Sinon le bureau a bouge entre les deux images,
  et la comparaison du verre est NON CONCLUANTE ;
- **l'outil sait-il voir un ecart ?** Le meme verre rendu sur un AUTRE fond (la
  page de texte du banc) doit s'ecarter de l'ecran presque partout.
"""

import importlib.util
import os
import re
import subprocess
import sys

ICI = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("cc", os.path.join(ICI, "comparer-captures.py"))
cc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cc)


def rendre(exe, fond, echelle, decalage, sortie):
    args = [exe, f"--capture-ilot={sortie}", "--restant=41:27", f"--echelle={echelle}",
            "--p=1", "--sans-contenu", f"--verre-fond={fond}"]
    if decalage is not None:
        args.append(f"--verre-decalage={decalage[0]},{decalage[1]}")
    # L'adaptateur du rendu hors-ecran. La fenetre de l'ilot peint sur celui
    # qui sait heriter de la composition ; pour separer un ecart d'adaptateur
    # d'un ecart de rendu, on nomme le meme ici (VEILLEUSE_ADAPTATEUR=intel).
    if os.environ.get("VEILLEUSE_ADAPTATEUR"):
        args.append(f"--adaptateur={os.environ['VEILLEUSE_ADAPTATEUR']}")
    r = subprocess.run(args, capture_output=True, text=True, timeout=120)
    texte = r.stdout + r.stderr
    if r.returncode != 0 or "erreur" in texte.lower():
        raise SystemExit(f"rendu en echec :\n{texte}")
    print("   ", [l for l in texte.splitlines() if l.startswith("adaptateur")][0])
    return cc.lire_png(sortie)


def pixel(img, ox, oy, x, y):
    l, h, c, p = img
    i, j = x - ox, y - oy
    if i < 0 or j < 0 or i >= l or j >= h:
        return None
    k = (j * l + i) * c
    return p[k : k + c]


def comparer(hors, fond, fx, fy, ecran, ax, ay, wx, wy, galet):
    mx, my, gl, gh = galet
    classes = {"interieur": [0, 0, 0], "ruban": [0, 0, 0], "hors": [0, 0, 0]}
    for j in range(gh):
        for i in range(gl):
            x, y = mx + i, my + j
            X, Y = wx + x, wy + y
            v = pixel(hors, 0, 0, x, y)
            f = pixel(fond, fx, fy, X, Y)
            e = pixel(ecran, ax, ay, X, Y)
            if v is None or f is None or e is None:
                continue
            a = v[3]
            cle = "interieur" if a == 255 else ("hors" if a == 0 else "ruban")
            pire = max(abs(min(v[k] + (f[k] * (255 - a) + 127) // 255, 255) - e[k]) for k in range(3))
            c = classes[cle]
            c[0] += 1
            c[1] += 1 if pire else 0
            c[2] = max(c[2], pire)
    return classes


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        sys.exit(2)
    exe, dossier = sys.argv[1], sys.argv[2]
    texte = open(os.path.join(dossier, "releve.txt"), encoding="utf-8").read()
    m = re.search(
        r"VERRE-DIRECT echelle=([\d.]+) fenetre=(-?\d+),(-?\d+) galet=([\d.]+),([\d.]+),([\d.]+),([\d.]+) "
        r"fond=(-?\d+),(-?\d+),(\d+),(\d+) avant=(-?\d+),(-?\d+),(\d+),(\d+)",
        texte,
    )
    if not m:
        raise SystemExit("pas de ligne VERRE-DIRECT dans releve.txt")
    g = m.groups()
    echelle = g[0]
    wx, wy = int(g[1]), int(g[2])
    galet = tuple(int(float(v)) for v in g[3:7])
    fx, fy = int(g[7]), int(g[8])
    ax, ay = int(g[11]), int(g[12])
    fond = cc.lire_png(os.path.join(dossier, "verre-fond.png"))
    ecran = cc.lire_png(os.path.join(dossier, "verre-avant.png"))

    # Temoin 1 : hors du galet (et hors de sa boite), l'ecran egale le fond.
    mx, my, gl, gh = galet
    hors_boite, bouge = 0, 0
    for Y in range(ay, ay + ecran[1]):
        for X in range(ax, ax + ecran[0]):
            if wx + mx <= X < wx + mx + gl and wy + my <= Y < wy + my + gh:
                continue
            f = pixel(fond, fx, fy, X, Y)
            e = pixel(ecran, ax, ay, X, Y)
            if f is None:
                continue
            hors_boite += 1
            if any(f[k] != e[k] for k in range(3)):
                bouge += 1
    stable = hors_boite > 0 and bouge == 0
    print(f"temoin du fond : {bouge} pixels changes sur {hors_boite} hors de la boite du galet")

    sortie = os.path.join(dossier, "verre-hors-ecran.png")
    hors = rendre(exe, os.path.join(dossier, "verre-fond.png"), echelle, (wx - fx, wy - fy), sortie)
    r = comparer(hors, fond, fx, fy, ecran, ax, ay, wx, wy, galet)
    for cle in ("interieur", "ruban", "hors"):
        n, d, p = r[cle]
        print(f"hors-ecran compose contre ecran, {cle} : {d} sur {n} differents, pire {p}")

    banc = os.path.join(ICI, "..", "captures", "integration", "pas2-banc", "page-texte-fond.png")
    autre = rendre(exe, banc, echelle, None, os.path.join(dossier, "verre-temoin-autre-fond.png"))
    t = comparer(autre, fond, fx, fy, ecran, ax, ay, wx, wy, galet)
    n, d, p = t["interieur"]
    print(f"temoin, verre rendu sur la page de texte du banc, interieur : {d} sur {n} differents, pire {p}")

    if not stable:
        print("VERDICT l'ecran retombe sur le rendu hors-ecran : NON CONCLUANT (le fond a bouge)")
    else:
        n, d, p = r["interieur"]
        print(f"VERDICT l'ecran retombe sur le rendu hors-ecran a un arrondi pres : {'VRAI' if n > 0 and p <= 1 else 'FAUX'}")
    print(f"VERDICT le temoin voit un autre fond : {'VRAI' if t['interieur'][1] > 0 else 'FAUX'}")


if __name__ == "__main__":
    main()
