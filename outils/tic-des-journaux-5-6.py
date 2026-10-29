"""Pourquoi les journaux de cout 5 et 6 s'arretent avant le bloc apparie.

Les journaux `captures/technicien-tour2/cout-5.txt` et `cout-6.txt` finissent sur
« PAS DE MESURE » : le temoin 6.1 y tombe, et `instrument_6` rend faux avant
d'atteindre la mesure appariee. Leurs deux moities de balayage sont pourtant
identiques a chaque taille : ce n'est pas le bruit qui le fait tomber.

L'hypothese : l'estimateur du tic de `mesure.rs` prend le plus petit ecart non
nul entre deux valeurs mesurees. Si toutes les valeurs tombent sur 2, 5, 9 et
14 tics, aucune paire n'est a un tic d'ecart, et l'estimateur rend 3 tics.

Ce script recopie, ligne pour ligne, les calculs d'`instrument_6` qui menent au
verdict de 6.1, et les nourrit de ces seules valeurs. Le temoin est le journal
lui-meme : si l'hypothese est juste, le script retrouve au chiffre pres ce que
les deux journaux impriment. L'epreuve ajoute UNE valeur a un tic de sa voisine :
le tic doit redevenir juste, et 6.1 tenir.
"""

import math

# Le pas de l'horodateur, en millisecondes. Les journaux sains impriment
# 0.00102 ms, et les journaux 5 et 6 un ecart au multiple de 0.001024 ms.
TIC = 0.001024

# Les quatre tailles du temoin 6.1, et leurs nombres de pixels.
PIXELS = [160 * 74, 320 * 148, 480 * 222, 640 * 296]

BALAYAGES = 10


def mediane(x):
    # `mesure.rs` : y[len / 2] sur les valeurs triees.
    y = sorted(x)
    return y[len(y) // 2]


def ajuster(pts):
    # `ajuster` de `mesure.rs` : moindres carres, puis le plus grand residu.
    n = len(pts)
    sx = sum(p[0] for p in pts)
    sy = sum(p[1] for p in pts)
    sxx = sum(p[0] * p[0] for p in pts)
    sxy = sum(p[0] * p[1] for p in pts)
    det = n * sxx - sx * sx
    b = (n * sxy - sx * sy) / det
    a = (sy - b * sx) / n
    r = max(abs(p[1] - (a + b * p[0])) for p in pts)
    return a, b, r


def temoin_61(balayages):
    rapports = [[] for _ in range(4)]
    absolus = [[] for _ in range(4)]
    for ligne in balayages:
        reference = max(ligne[3], 1e-9)
        for k in range(4):
            rapports[k].append(ligne[k] / reference)
            absolus[k].append(ligne[k])

    pts = []
    plancher = 0.0
    for k in range(4):
        r = mediane(rapports[k])
        r1 = mediane(rapports[k][: BALAYAGES // 2])
        r2 = mediane(rapports[k][BALAYAGES // 2 :])
        pts.append((PIXELS[k], r))
        plancher = max(plancher, abs(r1 - r2))

    toutes = sorted(v for colonne in absolus for v in colonne)
    tic_mesure = math.inf
    for a, b in zip(toutes, toutes[1:]):
        d = b - a
        if d > 1e-9:
            tic_mesure = min(tic_mesure, d)
    ecart_multiple = max(abs(x - round(x / tic_mesure) * tic_mesure) for x in toutes)
    tic_sain = math.isfinite(tic_mesure) and ecart_multiple < 0.1 * tic_mesure

    plancher_quant = tic_mesure / max(mediane(absolus[3]), 1e-9)
    plancher = max(plancher, plancher_quant)

    _, pente, residu = ajuster(pts)
    seuil = plancher
    sxy = sum(p[0] * p[1] for p in pts)
    sxx = sum(p[0] * p[0] for p in pts)
    c_prop = sxy / sxx
    residu_prop = max(abs(p[1] - c_prop * p[0]) for p in pts)
    discriminant = tic_sain and plancher > 0.0 and seuil < residu_prop
    t61 = discriminant and pente > 0.0 and residu < seuil
    return {
        "tic MESURE (ms)": f"{tic_mesure:.5f}",
        "ecart au multiple (ms)": f"{ecart_multiple:.6f}",
        "tic sain": tic_sain,
        "quantification du rapport": f"{plancher_quant:.5f}",
        "plus grand residu": f"{residu:.5f}",
        "residu du modele y = c x": f"{residu_prop:.5f}",
        "le seuil refuse quelque chose": discriminant,
        "temoin 6.1": t61,
    }


def dire(titre, resultat):
    print(titre)
    for cle, valeur in resultat.items():
        print(f"  {cle:<32} {valeur}")
    print()


def comparer(resultat, journal):
    """Le temoin : chaque nombre du journal, retrouve ou non."""
    ecarts = [c for c in journal if str(resultat[c]) != str(journal[c])]
    if ecarts:
        print("  TEMOIN : le script ne retrouve PAS le journal, sur : " + ", ".join(ecarts))
    else:
        print("  TEMOIN : le script retrouve CHAQUE nombre imprime par le journal.")
    print()
    return not ecarts


def cas(titre, tics, journal, tics_epreuve):
    """Un cas : les valeurs posees, le journal, et l'epreuve.

    L'epreuve change UNE valeur d'un seul balayage pour qu'une paire de
    valeurs a un tic d'ecart existe. Elle sert deux fois : le tic doit
    redevenir juste et 6.1 tenir, et le temoin du script doit tomber, puisque
    ce ne sont plus les valeurs du journal.
    """
    stable = [[t * TIC for t in tics] for _ in range(BALAYAGES)]
    resultat = temoin_61(stable)
    dire(titre, resultat)
    retrouve = comparer(resultat, journal)

    epreuve = [ligne[:] for ligne in stable]
    epreuve[0][3] = tics_epreuve * TIC
    resultat_epreuve = temoin_61(epreuve)
    dire(f"  l'epreuve : un seul balayage rend {tics_epreuve} tics au lieu de {tics[3]}", resultat_epreuve)
    tombe = not comparer(resultat_epreuve, journal)
    return retrouve and tombe and resultat_epreuve["temoin 6.1"]


# Ce que les journaux 5 et 6 du second tour impriment, recopie.
JOURNAUX_5_6 = {
    "tic MESURE (ms)": "0.00307",
    "ecart au multiple (ms)": "0.001024",
    "tic sain": False,
    "quantification du rapport": "0.21429",
    "plus grand residu": "0.02575",
    "residu du modele y = c x": "0.09322",
    "le seuil refuse quelque chose": False,
    "temoin 6.1": False,
}

# Ce que la tentative 4 du troisieme tour imprime, recopie
# (`captures/mesure/cout-tour3-banc-4.txt`). Le journal n'avertit pas que le
# tic serait faux : le temoin du tic l'a laisse passer.
TENTATIVE_4 = {
    "tic MESURE (ms)": "0.00205",
    "ecart au multiple (ms)": "0.000000",
    "tic sain": True,
    "quantification du rapport": "0.16667",
    "plus grand residu": "0.04070",
    "residu du modele y = c x": "0.10028",
    "le seuil refuse quelque chose": False,
    "temoin 6.1": False,
}

bilan = cas(
    "LES JOURNAUX 5 ET 6 DU SECOND TOUR : 2, 5, 9, 14 tics a chaque balayage",
    [2, 5, 9, 14],
    JOURNAUX_5_6,
    13,
)
bilan &= cas(
    "LA TENTATIVE 4 DU TROISIEME TOUR : 2, 4, 8, 12 tics a chaque balayage",
    [2, 4, 8, 12],
    TENTATIVE_4,
    11,
)
print(
    "BILAN : "
    + (
        "les deux journaux sont reproduits au chiffre pres, chaque epreuve fait\n"
        "tenir 6.1 et fait tomber le temoin du script."
        if bilan
        else "au moins un cas n'est pas reproduit, ou une epreuve ne fait pas ce\n"
        "qu'elle doit. La cause n'est pas etablie."
    )
)
