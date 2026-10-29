"""La borne de `DESIGN.md` §8, lue sur le rendu de l'APPLICATION.

La borne : *le premier pixel du bord, coins compris, ne depasse pas `#5E5B57`
(91 niveaux de luminance) sur aucun fond, blanc pur compris, avec la marge d'un
octet*. Le banc l'a derivee sur SON rendu (essai 8.H). L'application ne rend pas
le bord comme le banc : le compositeur melange le ruban sur les octets, et la
fenetre peint sur un autre adaptateur. Cet outil relit donc la borne la ou
l'auteur la verra.

Le « premier pixel du bord » est lu exactement comme au banc
(`pic_du_bord_droit`, `pic_du_coin`) : les pixels dont la distance signee, prise
au centre, tombe dans [-1,5 ; -0,45], separes en bord droit et coins, lus en
niveau de luminance (255 x sRGB(luminance relative)).

    python outils/borne-du-bord.py <dossier de rendus de l'application> [...]

Les rendus sont ceux que `outils/verre-contre-banc.py` ecrit
(`<fond>-<etat>-app.png`), composes ici sur le fond, sur les octets, comme
Windows le fait.

TROIS LECTURES, parce que la bande du banc ne veut pas dire la meme chose dans
les deux rendus.

1. *A la lettre* : la bande en distance, sur l'image composee.
2. *A couverture egale* : la bande du banc s'arrete a d = -0,45, ou son ruban
   `1 - smoothstep(-0,5 ; 0,5 ; d)` couvre encore 0,99275 du pixel, soit
   253,15 octets. Le ruban de l'application est la couverture exacte
   0,5 - d : a d = -0,46 il ne couvre que 0,96 du pixel, et les 4 % restants
   montrent le fond. On garde donc, dans la bande, les seuls pixels que
   l'application couvre au moins autant que le banc a la limite de sa bande :
   alpha >= 254. Le seuil derive de cette grandeur, pas du resultat.
3. *Le verre seul* : la sortie de l'application depremultipliee (octets / alpha)
   dans la bande a la lettre. C'est ce que le verre peint, avant que le
   compositeur n'y ajoute le fond.

Et la cause, par un autre chemin : l'image de l'application est PREDITE depuis
l'image du BANC et la seule couverture de l'application (banc x c + fond x
(1 - c), sur les octets), sans lire la couleur de l'application. L'ecart de la
prediction est imprime.

Les temoins. Le banc relu par cet outil doit redonner les chiffres de
`DECISION.md` a la force 0,25 sur le blanc : 86 au bord droit, 90 au coin. Une
epreuve rendue a la force 0,30 doit casser la borne dans les lectures 2 et 3
(92 et 97 au banc).
"""

import importlib.util
import math
import os
import sys

ICI = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("cc", os.path.join(ICI, "comparer-captures.py"))
cc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cc)

BANC = os.path.join(ICI, "..", "captures", "integration", "pas2-banc")
FONDS = ["page-texte", "bureau-reel", "bureau-raye", "bureau-sombre", "horloge", "coupe", "blanc"]
# largeur, hauteur, rayon, exposant, et position du galet dans le rendu de
# l'application (fenetre deployee de 320 x 148 a l'echelle 1).
ETATS = {"deploye": (320, 148, 48.0, 4.0, 0, 0), "replie": (88, 36, 18.0, 2.5, 116, 0)}
MARGE = 64
TOLERANCE_ARRONDI = 1.0
LIMITE_BANDE = -0.45


def couverture_du_banc(d):
    t = min(max(d + 0.5, 0.0), 1.0)
    return 1.0 - t * t * (3.0 - 2.0 * t)


# La couverture du banc a la limite de sa bande, en octets d'alpha, arrondie
# au-dessus : un pixel couvert moins que cela n'est pas dans la bande du banc.
ALPHA_MINIMAL = math.ceil(couverture_du_banc(LIMITE_BANDE) * 255.0)


def lin(o):
    c = o / 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def enc(c):
    return 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055


def niveau(r, g, b):
    return 255.0 * enc(0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b))


def distance(x, y, l, h, r, n):
    px, py = x + 0.5 - l / 2.0, y + 0.5 - h / 2.0
    qx, qy = abs(px) - l / 2.0 + r, abs(py) - h / 2.0 + r
    if qx > 0 and qy > 0:
        f = (qx ** n + qy ** n) ** (1 / n)
        g = ((qx ** (n - 1)) ** 2 + (qy ** (n - 1)) ** 2) ** 0.5 / f ** (n - 1)
        return (f - r) / g
    return (max(qx, 0) ** 2 + max(qy, 0) ** 2) ** 0.5 + min(max(qx, qy), 0) - r


def bande(l, h, r, n):
    for y in range(h):
        for x in range(l):
            d = distance(x, y, l, h, r, n)
            if d > LIMITE_BANDE or d < -1.5:
                continue
            dans_coin = (x < r or x > l - r) and (y < r or y > h - r)
            yield x, y, dans_coin


def lire_banc(nom, etat):
    l, _, _, p = cc.lire_png(os.path.join(BANC, f"{nom}-{etat}-arbitree.png"))
    return lambda x, y: list(p[(y * l + x) * 4 : (y * l + x) * 4 + 3])


class App:
    def __init__(self, dossier, nom, etat):
        self.la, _, _, self.pa = cc.lire_png(os.path.join(dossier, f"{nom}-{etat}-app.png"))
        self.lf, _, _, self.pf = cc.lire_png(os.path.join(BANC, f"{nom}-fond.png"))
        _, _, _, _, self.gx, self.gy = ETATS[etat]

    def _i(self, x, y):
        return ((self.gy + y) * self.la + self.gx + x) * 4, ((MARGE + y) * self.lf + MARGE + x) * 4

    def alpha(self, x, y):
        return self.pa[self._i(x, y)[0] + 3]

    def fond(self, x, y):
        return [self.pf[self._i(x, y)[1] + k] for k in range(3)]

    def compose(self, x, y):
        ia, ifo = self._i(x, y)
        a = self.pa[ia + 3]
        return [min(self.pa[ia + k] + (self.pf[ifo + k] * (255 - a) + 127) // 255, 255) for k in range(3)]

    def verre(self, x, y):
        ia, _ = self._i(x, y)
        a = self.pa[ia + 3]
        if a == 0:
            return [0, 0, 0]
        return [min(self.pa[ia + k] * 255.0 / a, 255.0) for k in range(3)]


def pics(valeurs):
    droit, coin = 0.0, 0.0
    for v, dans_coin in valeurs:
        if dans_coin:
            coin = max(coin, v)
        else:
            droit = max(droit, v)
    return droit, coin


def seuil():
    return niveau(0x5E, 0x5B, 0x57) - TOLERANCE_ARRONDI


def verdict(titre, pire):
    s = seuil()
    print(f"  VERDICT {titre} : {'VRAI' if pire <= s else 'FAUX'}  (pire {pire:.2f}, marge {s - pire:+.2f})")
    return pire <= s


def juger_banc():
    print(f"--- banc, direction ARBITREE (temoin : blanc deploye 86 et 90 dans DECISION.md)")
    pire = 0.0
    for nom in FONDS:
        ligne = f"  {nom:14s}"
        for etat, (l, h, r, n, _, _) in ETATS.items():
            lire = lire_banc(nom, etat)
            droit, coin = pics((niveau(*lire(x, y)), c) for x, y, c in bande(l, h, r, n))
            pire = max(pire, droit, coin)
            ligne += f"  {etat} droit {droit:6.2f} coin {coin:6.2f}"
        print(ligne)
    verdict("la borne tient au banc", pire)


def juger_app(dossier):
    print(f"--- application, {dossier}")
    print(f"    (plafond {seuil() + TOLERANCE_ARRONDI:.2f}, seuil avec la marge d'un octet {seuil():.2f} ;"
          f" couverture du banc a d = {LIMITE_BANDE} : alpha >= {ALPHA_MINIMAL})")
    pires = {"lettre": 0.0, "couverture": 0.0, "verre": 0.0}
    ecartes, gardes, erreur_prediction = 0, 0, 0.0
    for nom in FONDS:
        for etat, (l, h, r, n, _, _) in ETATS.items():
            app = App(dossier, nom, etat)
            banc = lire_banc(nom, etat)
            lettre, couv, verre = [], [], []
            for x, y, c in bande(l, h, r, n):
                vc = niveau(*app.compose(x, y))
                lettre.append((vc, c))
                verre.append((niveau(*app.verre(x, y)), c))
                a = app.alpha(x, y)
                if a >= ALPHA_MINIMAL:
                    couv.append((vc, c))
                    gardes += 1
                else:
                    ecartes += 1
                    # La prediction : le banc, couvert comme l'application.
                    fo = app.fond(x, y)
                    predit = [min(b + (f * (255 - a) + 127) // 255 - (b * (255 - a) + 127) // 255, 255)
                              for b, f in zip(banc(x, y), fo)]
                    erreur_prediction = max(erreur_prediction, abs(niveau(*predit) - vc))
            dl, cl = pics(lettre)
            dc, ccv = pics(couv)
            dv, cv = pics(verre)
            pires["lettre"] = max(pires["lettre"], dl, cl)
            pires["couverture"] = max(pires["couverture"], dc, ccv)
            pires["verre"] = max(pires["verre"], dv, cv)
            print(f"  {nom:14s} {etat:8s} lettre droit {dl:6.2f} coin {cl:6.2f} | couverture droit {dc:6.2f}"
                  f" coin {ccv:6.2f} | verre seul droit {dv:6.2f} coin {cv:6.2f}")
    print(f"  pixels de la bande gardes a couverture egale : {gardes}, ecartes : {ecartes}")
    print(f"  cause : l'application predite depuis le banc et sa seule couverture, sur les pixels ecartes :"
          f" ecart pire {erreur_prediction:.2f} niveau(x)")
    ok = [verdict("1. la borne tient a la lettre (bande en distance, image composee)", pires["lettre"]),
          verdict("2. la borne tient a couverture egale au banc", pires["couverture"]),
          verdict("3. la borne tient sur le verre seul, depremultiplie", pires["verre"])]
    return ok


def main():
    juger_banc()
    for dossier in sys.argv[1:]:
        juger_app(dossier)


if __name__ == "__main__":
    main()
