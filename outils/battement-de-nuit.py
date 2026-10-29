"""Le battement de nuit, lu sur le rendu de l'APPLICATION.

Le banc l'a mesure sur son propre rendu (`DECISION.md`, essai 8.H) : sur la
rangee des chiffres, le bord de la direction ARBITREE bat de 79,2 niveaux d'une
seconde a la suivante, contre 14,2 pour le verre seul, et 0 a la place par
defaut. L'application ne rend pas exactement comme le banc : elle est a
l'echelle 1,25, son ruban est la couverture exacte, et c'est le compositeur qui
melange le bord sur les octets. Cet instrument relit donc le battement la ou
l'auteur le verra.

La methode, deterministe et sans fenetre a l'ecran :

1. l'application rend l'horloge plein ecran a deux instants voisins
   (`--capture --heure`) ;
2. elle rend l'ilot deploye, sans contenu, pose sur chacune de ces images a une
   place donnee (`--capture-ilot --verre-fond --verre-decalage --echelle=1.25`) ;
3. l'outil compose chaque rendu sur son fond, sur les octets, comme Windows :
   sortie = verre + fond x (1 - alpha) ;
4. il compare les deux compositions, pixel par pixel, en niveau de luminance.

    python outils/battement-de-nuit.py <dossier des rendus> <dossier de l'horloge> [--epreuve-decale]

Le dossier des rendus porte `<place>-<hhmmss>.png`, `<place>-031415-bis.png`
et `places.txt` (`nom:x,y` separes par des espaces). Le dossier de l'horloge
porte `apres-<hhmmss>.png`.

LES BANDES. Elles sont celles de `battement_de_nuit` au banc, portees a
l'echelle 1,25 de l'application : le bord est la bande de distance signee
[-7,5 ; -0,45], soit [-6 ; -0,45] au banc ; le coeur est au-dela de
-(15 + 2,5), soit -(12 + 2) au banc, le conge relatif valant 15 px physiques a
cette hauteur. La limite -0,45 n'est pas mise a l'echelle : elle vient de la
couverture du ruban, qui est une grandeur de pixel. La bande non mise a
l'echelle est lue aussi, pour que le chiffre reste comparable au banc a la
lettre.

LES TEMOINS, chacun avec sa reponse connue d'avance :

- *determinisme* : le meme instant rendu deux fois donne un battement nul,
  partout. L'epreuve `--epreuve-decale` decale d'un pixel la seconde
  composition avant la comparaison : ce temoin doit alors TOMBER ;
- *le fond change sous l'ilot* : a la place des secondes, les chiffres nus
  changent dans le rectangle de l'ilot entre les deux instants. Sans cela, un
  battement nul ne prouverait rien ;
- *la place par defaut ne voit aucun chiffre* : en haut de l'ecran, les
  chiffres nus ne changent pas dans le rectangle de l'ilot, et le verre non
  plus. C'est le zero du banc, a reponse connue.

Chaque verdict est calcule sur le nombre mesure. Aucun n'est ecrit a la main.
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

# La geometrie du galet deploye dans l'application, a l'echelle 1,25, telle que
# l'oeil la relit sur la fenetre reelle (`oeil.txt` : galet=0,0,400,185
# rayon=60 n=4).
RAYON = 60.0
EXPOSANT = 4.0
ECHELLE = 1.25
CONGE = 12.0 * ECHELLE
BORD = (-6.0 * ECHELLE, -0.45)
BORD_BANC = (-6.0, -0.45)
COEUR = -(CONGE + 2.0 * ECHELLE)

INSTANTS = [("031415", "031416", "la seconde change"), ("031459", "031500", "la minute change")]

# Les chiffres publies par le banc, pour la comparaison (`DECISION.md`, essai 8.H).
BANC_ARBITREE = 79.2
BANC_VERRE_SEUL = 14.2


def image(chemin):
    l, h, canaux, p = cc.lire_png(chemin)
    return l, h, canaux, p


def composer(verre_png, fond_png, x0, y0, decale=0):
    """Le verre compose sur son fond, sur les octets, en luminance par pixel."""
    l, h, cv, v = image(verre_png)
    lf, _, cf, f = image(fond_png)
    niveaux = [0.0] * (l * h)
    for y in range(h):
        for x in range(l):
            xs = min(l - 1, x + decale)
            i = (y * l + xs) * cv
            a = v[i + 3] if cv == 4 else 255
            j = ((y0 + y) * lf + (x0 + x)) * cf
            # L'arrondi est celui de verre-en-direct.py, qui retombe sur l'ecran a
            # 0 pixel pres : un autre arrondi ferait mentir le dernier niveau.
            rgb = [min(255, v[i + k] + (f[j + k] * (255 - a) + 127) // 255) for k in range(3)]
            niveaux[y * l + x] = bb.niveau(*rgb)
    return l, h, niveaux


def fond_seul(fond_png, x0, y0, l, h):
    lf, _, cf, f = image(fond_png)
    return [bb.niveau(*f[((y0 + y) * lf + (x0 + x)) * cf : ((y0 + y) * lf + (x0 + x)) * cf + 3])
            for y in range(h) for x in range(l)]


def distances(l, h):
    return [bb.distance(x, y, l, h, RAYON, EXPOSANT) for y in range(h) for x in range(l)]


def lire_ecart(a, b, dist):
    """Le plus grand ecart au bord, au bord du banc, au coeur, et l'etendue."""
    bord = bord_banc = coeur = 0.0
    changes = 0
    ecarts = []
    for i, d in enumerate(dist):
        if d > -0.45:
            continue
        e = abs(a[i] - b[i])
        ecarts.append(e)
        if e >= 1.0:
            changes += 1
        if BORD[0] <= d <= BORD[1]:
            bord = max(bord, e)
        if BORD_BANC[0] <= d <= BORD_BANC[1]:
            bord_banc = max(bord_banc, e)
        if d < COEUR:
            coeur = max(coeur, e)
    ecarts.sort()
    p99 = ecarts[int(0.99 * (len(ecarts) - 1))] if ecarts else 0.0
    return {"bord": bord, "bord_banc": bord_banc, "coeur": coeur, "changes": changes,
            "sur": len(ecarts), "p99": p99}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    decale = 1 if "--epreuve-decale" in sys.argv else 0
    if len(args) != 2:
        print(__doc__)
        sys.exit(2)
    rendus, horloge = args
    places = {}
    for morceau in open(os.path.join(rendus, "places.txt"), encoding="utf-8").read().split():
        nom, xy = morceau.split(":")
        x, y = xy.split(",")
        places[nom] = (int(x), int(y))

    lignes = []
    def dire(t=""):
        print(t)
        lignes.append(t)

    verdicts = []
    def verdict(nom, vrai, detail):
        verdicts.append(vrai)
        dire(f"  VERDICT {nom} : {'VRAI' if vrai else 'FAUX'}  ({detail})")

    dire("=== le battement de nuit, sur le rendu de l'application ===")
    dire(f"echelle {ECHELLE} ; bord d dans [{BORD[0]} ; {BORD[1]}] (banc [{BORD_BANC[0]} ; {BORD_BANC[1]}]) ; coeur d < {COEUR}")
    if decale:
        dire("EPREUVE : la seconde composition est decalee d'un pixel avant la comparaison")

    dist = None
    resultats = {}
    for nom, (x0, y0) in places.items():
        base = os.path.join(rendus, f"{nom}-031415.png")
        l, h, a = composer(base, os.path.join(horloge, "apres-031415.png"), x0, y0)
        if dist is None:
            dist = distances(l, h)
        _, _, a_bis = composer(os.path.join(rendus, f"{nom}-031415-bis.png"),
                               os.path.join(horloge, "apres-031415.png"), x0, y0, decale)
        tout = max(abs(p - q) for p, q in zip(a, a_bis))
        dire(f"--- place {nom} ({x0}, {y0})")
        verdict(f"{nom} : le meme instant rendu deux fois ne bat pas", tout == 0.0,
                f"plus grand ecart sur tout l'ilot {tout:.1f}")
        for t1, t2, quoi in INSTANTS:
            _, _, c1 = composer(os.path.join(rendus, f"{nom}-{t1}.png"), os.path.join(horloge, f"apres-{t1}.png"), x0, y0)
            _, _, c2 = composer(os.path.join(rendus, f"{nom}-{t2}.png"), os.path.join(horloge, f"apres-{t2}.png"), x0, y0)
            f1 = fond_seul(os.path.join(horloge, f"apres-{t1}.png"), x0, y0, l, h)
            f2 = fond_seul(os.path.join(horloge, f"apres-{t2}.png"), x0, y0, l, h)
            v = lire_ecart(c1, c2, dist)
            nu = lire_ecart(f1, f2, dist)
            resultats[(nom, t1)] = (v, nu)
            dire(f"  {quoi} ({t1} -> {t2})")
            dire(f"    chiffres nus sous l'ilot : bord {nu['bord']:.1f}, coeur {nu['coeur']:.1f}, pixels changes {nu['changes']} sur {nu['sur']}")
            dire(f"    verre compose            : bord {v['bord']:.1f} (bande du banc {v['bord_banc']:.1f}), coeur {v['coeur']:.1f}, "
                 f"pixels changes {v['changes']} sur {v['sur']}, 99e centile {v['p99']:.1f}")

    # Le volet en bascule. Le battement a l'arret compare deux secondes ; pendant
    # les 360 ms de la bascule, le verre suit le volet image apres image. Les
    # phases lues ici sont espacees de 90 ms : ce sont des instants, pas des
    # trames, et le pas entre deux trames reelles est plus petit.
    phases = sorted(
        (float(f[len("secondes-volet-"):-4]), f[len("secondes-volet-"):-4], f)
        for f in os.listdir(rendus)
        if f.startswith("secondes-volet-") and f.endswith(".png")
    )
    if phases and "secondes" in places:
        x0, y0 = places["secondes"]
        # La bascule rendue par --heure=03:14:16 --phase est celle qui QUITTE 16
        # pour 17 : la phase 1 egale 17 a l'arret au pixel pres. Le premier
        # temoin de cet instrument supposait 15 -> 16, et il est tombe.
        dire("--- le volet en bascule sous les secondes, 16 -> 17 (--heure=03:14:16 --phase)")
        suite = []
        for ph, texte, f in phases:
            fond = os.path.join(horloge, f"volet-031416-{texte}.png")
            _, _, c = composer(os.path.join(rendus, f), fond, x0, y0)
            suite.append((ph, c, fond_seul(fond, x0, y0, l, h)))
        f16 = fond_seul(os.path.join(horloge, "apres-031416.png"), x0, y0, l, h)
        f17 = fond_seul(os.path.join(horloge, "apres-031417.png"), x0, y0, l, h)

        def differents(p, q):
            return sum(1 for u, w in zip(p, q) if u != w)

        premier, dernier = suite[0], suite[-1]
        d0_16, d0_17 = differents(premier[2], f16), differents(premier[2], f17)
        d1_16, d1_17 = differents(dernier[2], f16), differents(dernier[2], f17)
        dire(f"  phase {premier[0]} contre 16 : {d0_16} px ; contre 17 : {d0_17} px")
        dire(f"  phase {dernier[0]} contre 16 : {d1_16} px ; contre 17 : {d1_17} px")
        # La phase 0 est le premier instant de la bascule, pas l'arret : elle
        # peut differer de 16 de quelques pixels. On demande qu'elle soit plus
        # proche de 16 que de 17, et que la phase 1 egale 17 exactement.
        verdict("la bascule rendue va de 16 a 17",
                d1_17 == 0 and d0_16 < d0_17,
                f"phase 1 contre 17 : {d1_17} px ; phase 0 plus proche de 16 ({d0_16}) que de 17 ({d0_17})")
        pire_bord = pire_coeur = 0.0
        for (p1, c1, _), (p2, c2, _) in zip(suite, suite[1:]):
            e = lire_ecart(c1, c2, dist)
            pire_bord, pire_coeur = max(pire_bord, e["bord"]), max(pire_coeur, e["coeur"])
            dire(f"  phase {p1} -> {p2} : bord {e['bord']:.1f}, coeur {e['coeur']:.1f}, pixels changes {e['changes']}")
        dire(f"  plus grand pas entre deux instants a 90 ms : bord {pire_bord:.1f}, coeur {pire_coeur:.1f}")

    dire("--- les temoins de place")
    if "secondes" in places:
        v, nu = resultats[("secondes", "031415")]
        verdict("sous les secondes, les chiffres nus changent", nu["changes"] > 0,
                f"{nu['changes']} pixels changes")
        verdict("sous les secondes, le verre bat", v["changes"] > 0, f"{v['changes']} pixels changes")
    if "defaut" in places:
        v, nu = resultats[("defaut", "031415")]
        verdict("a la place par defaut, les chiffres nus ne changent pas", nu["changes"] == 0,
                f"{nu['changes']} pixels changes")
        verdict("a la place par defaut, le verre ne bat pas", v["changes"] == 0,
                f"{v['changes']} pixels changes, bord {v['bord']:.1f}")

    dire("--- contre le banc")
    dire(f"  banc, direction ARBITREE sur la rangee des chiffres : bord {BANC_ARBITREE} ; verre seul {BANC_VERRE_SEUL} ; place par defaut 0")
    for (nom, t1), (v, _) in resultats.items():
        dire(f"  application, {nom}, a partir de {t1} : bord {v['bord']:.1f}, bande du banc {v['bord_banc']:.1f}")

    dire(f"verdicts : {sum(verdicts)} sur {len(verdicts)}")
    sortie = os.path.join(rendus, "battement-epreuve.txt" if decale else "battement.txt")
    open(sortie, "w", encoding="utf-8").write("\n".join(lignes) + "\n")
    print(f"-> {sortie}")


if __name__ == "__main__":
    main()
