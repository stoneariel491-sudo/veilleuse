"""Le verre de l'application contre celui du banc, sur les memes fonds.

Le banc (`recherche/verre-taille`, `mesure integration`) ecrit pour chaque fond
le fond lui-meme et le rendu de la direction ARBITREE, deploye et replie, dans
`captures/integration/pas2-banc`. Cet outil fait rendre les memes fonds a
l'APPLICATION, par son propre chemin (`--capture-ilot --verre-fond
--sans-contenu`, echelle 1), puis compare.

L'application ne peint pas hors du galet : son PNG porte un alpha premultiplie,
et c'est le compositeur qui montre le bureau autour. On compose donc comme lui,
sur les octets : sortie = verre + fond x (1 - alpha). Le banc, lui, melange en
lineaire sur un `smoothstep`. Les deux ne peuvent donc se rejoindre qu'a
l'interieur du galet ; le ruban du bord est compte a part, et hors du galet les
deux doivent rendre le fond intact.

    python outils/verre-contre-banc.py <veilleuse.exe> [dossier de sortie]

Le temoin, qui doit TOMBER : chaque rendu de l'application est aussi compare au
rendu du banc sur un AUTRE fond. Un outil qui trouverait zero partout, meme
la, ne mesurerait rien.
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

BANC = os.path.join(ICI, "..", "captures", "integration", "pas2-banc")
FONDS = ["page-texte", "bureau-reel", "bureau-raye", "bureau-sombre", "horloge", "coupe", "blanc"]
ETATS = {"deploye": "1", "replie": "0"}
MARGE = 64
# Le fond du temoin : le plus eloigne de chacun, pour que l'ecart soit franc.
TEMOIN = {"page-texte": "bureau-sombre", "bureau-sombre": "blanc", "blanc": "horloge",
          "horloge": "blanc", "bureau-reel": "horloge", "bureau-raye": "horloge", "coupe": "horloge"}


def rendre(exe, fond, p, sortie):
    args = [exe, f"--capture-ilot={sortie}", "--restant=41:27", "--echelle=1", f"--p={p}",
            "--sans-contenu", f"--verre-fond={fond}"]
    # L'adaptateur du rendu. Le banc tourne sur celui que wgpu prefere ; la
    # fenetre de l'ilot, sur celui qui sait heriter de la composition. Nommer
    # le second (VEILLEUSE_ADAPTATEUR=intel) mesure l'application telle qu'elle
    # s'affiche, contre le banc tel qu'il a ete mesure.
    if os.environ.get("VEILLEUSE_ADAPTATEUR"):
        args.append(f"--adaptateur={os.environ['VEILLEUSE_ADAPTATEUR']}")
    r = subprocess.run(args, capture_output=True, text=True, timeout=120)
    texte = r.stdout + r.stderr
    if r.returncode != 0 or "erreur" in texte.lower():
        raise SystemExit(f"rendu en echec :\n{texte}")
    g = re.search(r"galet (\d+)x(\d+) a \((\d+), (\d+)\)", texte)
    return tuple(int(x) for x in g.groups())


def comparer(app, fond, banc, galet):
    la, ha, _, pa = app
    lf, _, _, pf = fond
    lb, hb, _, pb = banc
    gl, gh, gx, gy = galet
    assert (gl, gh) == (lb, hb), f"galet {gl}x{gh} contre banc {lb}x{hb}"
    classes = {"interieur": [0, 0, 0, 0], "ruban": [0, 0, 0, 0], "hors": [0, 0, 0, 0]}
    for y in range(gh):
        for x in range(gl):
            ia = ((gy + y) * la + gx + x) * 4
            ifo = ((MARGE + y) * lf + MARGE + x) * 4
            ib = (y * lb + x) * 4
            a = pa[ia + 3]
            cle = "interieur" if a == 255 else ("hors" if a == 0 else "ruban")
            pire = 0
            for k in range(3):
                compose = pa[ia + k] + (pf[ifo + k] * (255 - a) + 127) // 255
                pire = max(pire, abs(min(compose, 255) - pb[ib + k]))
            c = classes[cle]
            c[0] += 1
            if pire:
                c[1] += 1
            c[2] = max(c[2], pire)
            c[3] += pire
    return classes


def srgb_vers_lin(o):
    c = o / 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def lin_vers_srgb(c):
    c = min(max(c, 0.0), 1.0)
    s = c * 12.92 if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055
    return s * 255.0


def ruban_a_la_maniere_du_banc(app, fond, banc, galet):
    """La cause annoncee de l'ecart du ruban, verifiee par un autre chemin.

    Le verre de l'application sort premultiplie sur les octets avec une
    couverture lineaire c = 0,5 - d ; le banc melange en lineaire avec
    1 - smoothstep(-0,5 ; 0,5 ; d). Si c'est bien toute la difference, alors
    on retrouve le banc en DEPREMULTIPLIANT la sortie de l'application
    (octets / c), puis en la melangeant au fond a la maniere du banc. Le reste
    ne peut plus venir que de l'arrondi de la depremultiplication, grand quand
    c est petit : on le rapporte par tranche de couverture.
    """
    la, _, _, pa = app
    lf, _, _, pf = fond
    lb, _, _, pb = banc
    gl, gh, gx, gy = galet
    tranches = {}
    for y in range(gh):
        for x in range(gl):
            ia = ((gy + y) * la + gx + x) * 4
            a = pa[ia + 3]
            if a in (0, 255):
                continue
            c = a / 255.0
            d = 0.5 - c
            t = d + 0.5
            masque = 1.0 - t * t * (3.0 - 2.0 * t)
            ifo = ((MARGE + y) * lf + MARGE + x) * 4
            ib = (y * lb + x) * 4
            pire = 0
            for k in range(3):
                verre = srgb_vers_lin(min(pa[ia + k] / c, 255.0))
                intact = srgb_vers_lin(pf[ifo + k])
                predit = lin_vers_srgb(intact + (verre - intact) * masque)
                pire = max(pire, abs(round(predit) - pb[ib + k]))
            cle = "c < 0,25" if c < 0.25 else ("0,25 <= c < 0,5" if c < 0.5 else "c >= 0,5")
            n, p = tranches.get(cle, (0, 0))
            tranches[cle] = (n + 1, max(p, pire))
    return tranches


def ligne(nom, c):
    total, diff, pire, somme = c
    if total == 0:
        return f"{nom} : aucun pixel"
    return f"{nom} : {diff} sur {total} differents, pire {pire}, moyen {somme / total:.3f}"


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    exe = sys.argv[1]
    sortie = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ICI, "..", "captures", "integration", "pas2-app")
    os.makedirs(sortie, exist_ok=True)
    pires_interieur, pires_temoin = [], []
    for nom in FONDS:
        chemin_fond = os.path.join(BANC, f"{nom}-fond.png")
        fond = cc.lire_png(chemin_fond)
        for etat, p in ETATS.items():
            png = os.path.join(sortie, f"{nom}-{etat}-app.png")
            galet = rendre(exe, chemin_fond, p, png)
            app = cc.lire_png(png)
            banc = cc.lire_png(os.path.join(BANC, f"{nom}-{etat}-arbitree.png"))
            r = comparer(app, fond, banc, galet)
            autre = TEMOIN[nom]
            t = comparer(app, fond, cc.lire_png(os.path.join(BANC, f"{autre}-{etat}-arbitree.png")), galet)
            print(f"{nom:14s} {etat:8s} galet {galet[0]}x{galet[1]}")
            for cle in ("interieur", "ruban", "hors"):
                print(f"    {ligne(cle, r[cle])}")
            print(f"    temoin, banc sur « {autre} » : {ligne('interieur', t['interieur'])}")
            for cle, (n, p) in sorted(ruban_a_la_maniere_du_banc(app, fond, banc, galet).items()):
                print(f"    ruban recompose a la maniere du banc, {cle} : {n} pixels, pire {p}")
            pires_interieur.append(r["interieur"][2])
            pires_temoin.append(t["interieur"][1])
    print()
    pire = max(pires_interieur)
    print(f"pire ecart a l'interieur, tous fonds et etats : {pire} niveaux")
    print(f"temoin : le moins de pixels differents contre un autre fond : {min(pires_temoin)}")
    # La tolerance d'un arrondi du banc (`TOLERANCE_ARRONDI`, DECISION.md) : deux
    # arrondis d'une meme valeur different d'un octet au plus.
    print(f"VERDICT l'interieur retombe sur le banc a un arrondi pres : {'VRAI' if pire <= 1 else 'FAUX'}")
    print(f"VERDICT le temoin voit un autre fond : {'VRAI' if min(pires_temoin) > 0 else 'FAUX'}")


if __name__ == "__main__":
    main()
