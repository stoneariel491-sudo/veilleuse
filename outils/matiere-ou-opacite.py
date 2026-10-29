"""Une matiere, ou un aplat : le critere de l'orfevre, lu sur l'APPLICATION.

« Un motif qui disparait n'est pas une preuve de verre, c'est une preuve
d'opacite. » Du verre fait deux choses ENSEMBLE : il efface le motif fin, et il
porte la luminance d'ensemble du fond — plus clair sur un fond clair, plus
sombre sur un fond sombre. Un aplat fonce ne fait que la premiere.

Cet outil lit les deux sur les rendus de l'application que
`verre-contre-banc.py` a ecrits (`<fond>-<etat>-app.png`, echelle 1, sans
contenu), composes sur leur fond du banc (`<fond>-fond.png`) sur les octets,
avec l'arrondi qui retombe sur l'ecran a 0 pixel pres.

    python outils/matiere-ou-opacite.py <rendus de l'application> <opaque.png> [--epreuve-opaque]

LES DEUX GRANDEURS, lues au COEUR du galet, hors du conge et de son ruban :

- *la luminance portee* : la pente, par moindres carres, de la luminance
  moyenne du coeur composee contre celle du fond nu au meme endroit, sur tous
  les fonds. Une pente nulle est un aplat ; une pente de 1 est une vitre claire ;
- *le motif efface* : l'energie de haute frequence, moyenne des sauts de
  luminance entre voisins horizontaux du coeur, sur le fond raye, composee
  contre nue.

LES TEMOINS, a reponse connue d'avance :

- *l'opaque ne porte rien* : le galet opaque d'avant le verre (`opaque.png`),
  compose sur chaque fond, a une pente NULLE par construction, puisque ses
  pixels couvrent le fond en entier. `--epreuve-opaque` met cet opaque a la
  place du verre : le verdict de la luminance portee doit alors TOMBER ;
- *le fond raye porte un motif* : son energie de haute frequence nue est grande,
  sans quoi un motif efface ne prouverait rien.

LES SEUILS, et d'ou ils viennent :

- luminance portee si la pente depasse 10 fois l'erreur d'arrondi rapportee a
  l'etendue des fonds : un niveau sur l'etendue de luminance des fonds du coeur ;
- motif efface si l'energie tombe d'un facteur 10 au moins. Le banc mesure 46
  (`VERRE-TAILLE.md` §4) : le seuil en garde plus de quatre fois la marge.
"""

import importlib.util
import os
import sys

ICI = os.path.dirname(os.path.abspath(__file__))


def charger(nom):
    spec = importlib.util.spec_from_file_location(nom.replace("-", "_"), os.path.join(ICI, nom + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


cc = charger("comparer-captures")
bb = charger("borne-du-bord")

BANC = os.path.join(ICI, "..", "captures", "integration", "pas2-banc")
FONDS = ["page-texte", "bureau-reel", "bureau-raye", "bureau-sombre", "horloge", "coupe", "blanc"]
MARGE = 64
# Le galet deploye a l'echelle 1 : 320 x 148, rayon 48, exposant 4, pose en (0, 0)
# de la surface. Le coeur est au-dela du conge (12 px) et de deux pixels de plus,
# comme `battement_de_nuit` au banc.
L, H, RAYON, EXPOSANT = 320, 148, 48.0, 4.0
COEUR = -(12.0 + 2.0)


def coeur():
    return [[bb.distance(x, y, L, H, RAYON, EXPOSANT) < COEUR for x in range(L)] for y in range(H)]


def lire(chemin):
    l, h, c, p = cc.lire_png(chemin)
    return l, c, p


def composer(rendu, fond):
    """Luminance composee et luminance nue, pixel par pixel, sur le galet."""
    la, ca, pa = lire(rendu)
    lf, cf, pf = lire(fond)
    compose, nu = [[0.0] * L for _ in range(H)], [[0.0] * L for _ in range(H)]
    for y in range(H):
        for x in range(L):
            i = (y * la + x) * ca
            a = pa[i + 3] if ca == 4 else 255
            j = ((MARGE + y) * lf + MARGE + x) * cf
            rgb = [min(255, pa[i + k] + (pf[j + k] * (255 - a) + 127) // 255) for k in range(3)]
            compose[y][x] = bb.niveau(*rgb)
            nu[y][x] = bb.niveau(pf[j], pf[j + 1], pf[j + 2])
    return compose, nu


def moyenne(img, masque):
    v = [img[y][x] for y in range(H) for x in range(L) if masque[y][x]]
    return sum(v) / len(v)


def haute_frequence(img, masque):
    s, n = 0.0, 0
    for y in range(H):
        for x in range(L - 1):
            if masque[y][x] and masque[y][x + 1]:
                s += abs(img[y][x + 1] - img[y][x])
                n += 1
    return s / n


def pente(xs, ys):
    mx, my = sum(xs) / len(xs), sum(ys) / len(ys)
    sxx = sum((x - mx) ** 2 for x in xs)
    return sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sxx


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    epreuve = "--epreuve-opaque" in sys.argv
    if len(args) != 2:
        print(__doc__)
        sys.exit(2)
    rendus, opaque = args
    masque = coeur()

    lignes, verdicts = [], []

    def dire(t=""):
        print(t)
        lignes.append(t)

    def verdict(nom, vrai, detail):
        verdicts.append(vrai)
        dire(f"  VERDICT {nom} : {'VRAI' if vrai else 'FAUX'}  ({detail})")

    dire("=== une matiere, ou un aplat : le coeur du galet deploye, echelle 1 ===")
    if epreuve:
        dire("EPREUVE : l'opaque est mis a la place du verre")
    dire(f"{'fond':14} {'nu':>7} {'verre':>7} {'opaque':>7}   {'hf nue':>7} {'hf verre':>8}")
    nus, verres, opaques = [], [], []
    hf = {}
    for fond in FONDS:
        chemin_fond = os.path.join(BANC, f"{fond}-fond.png")
        rendu = opaque if epreuve else os.path.join(rendus, f"{fond}-deploye-app.png")
        c_verre, nu = composer(rendu, chemin_fond)
        c_opaque, _ = composer(opaque, chemin_fond)
        m_nu, m_verre, m_opaque = moyenne(nu, masque), moyenne(c_verre, masque), moyenne(c_opaque, masque)
        nus.append(m_nu); verres.append(m_verre); opaques.append(m_opaque)
        hf[fond] = (haute_frequence(nu, masque), haute_frequence(c_verre, masque))
        dire(f"{fond:14} {m_nu:7.2f} {m_verre:7.2f} {m_opaque:7.2f}   {hf[fond][0]:7.2f} {hf[fond][1]:8.2f}")

    etendue = max(nus) - min(nus)
    seuil_pente = 10.0 / etendue
    p_verre, p_opaque = pente(nus, verres), pente(nus, opaques)
    dire(f"etendue de luminance des fonds au coeur : {etendue:.1f} ; seuil de pente : 10 / {etendue:.1f} = {seuil_pente:.4f}")
    dire(f"pente du verre : {p_verre:.4f} ; pente de l'opaque : {p_opaque:.4f}")

    hf_nue, hf_verre = hf["bureau-raye"]
    facteur = hf_nue / hf_verre if hf_verre > 0 else float("inf")
    verdict("temoin : l'opaque ne porte rien", abs(p_opaque) <= 1.0 / etendue,
            f"pente {p_opaque:.4f}, tolerance d'un niveau sur l'etendue {1.0 / etendue:.4f}")
    verdict("temoin : le fond raye porte un motif", hf_nue >= 10.0, f"energie nue {hf_nue:.2f}")
    verdict("le verre porte la luminance du fond", p_verre > seuil_pente, f"pente {p_verre:.4f} contre seuil {seuil_pente:.4f}")
    verdict("le verre efface le motif fin", facteur >= 10.0, f"energie {hf_nue:.2f} -> {hf_verre:.2f}, facteur {facteur:.1f}")
    dire(f"verdicts : {sum(verdicts)} sur {len(verdicts)}")

    sortie = os.path.join(rendus, "matiere-epreuve.txt" if epreuve else "matiere.txt")
    open(sortie, "w", encoding="utf-8").write("\n".join(lignes) + "\n")
    print(f"-> {sortie}")


if __name__ == "__main__":
    main()
