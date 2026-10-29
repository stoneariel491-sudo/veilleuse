# L'intégration de l'icône : les douze pas, et ce que chacun a prouvé

`DECISION-ICONE.md` §I donne douze pas dans l'ordre, chacun avec l'épreuve qui
le prouve, son témoin et son compte prévu. Ce document en tient le journal : un
chapitre par pas, dans l'ordre, disant ce qui a été fait, l'épreuve, le témoin,
le compte prévu, le compte obtenu, et ce qui a résisté.

La décision ne s'y rediscute pas. Ce qui s'y écrit, c'est **ce que la machine a
répondu** — y compris quand elle a démenti un chiffre du document. Là où c'est
arrivé, le chiffre faux est barré, ce qui l'a remplacé est écrit, et sa cause
avec. Barrer, jamais effacer.

Les instruments et les images de ce chantier sont sous
`captures/integration-icone/`. Tout ce qui se lance se lance depuis une copie
hors du dépôt.

---

## Pas 1 — La barre des tâches sombre, mesurée au lieu d'être supposée

**Ce que le pas demande.** C'est le seul chiffre supposé dont tout le reste
dépend. `ICONE-FONDATIONS.md` §3.1 l'écrit lui-même : *« le fond sombre :
`#202020`, supposé, non mesuré — il aurait fallu changer le réglage de la
machine de l'auteur »*, et `DECISION-ICONE.md` §G le classe fragilité numéro
un des quatre documents du chantier.

**Ce qui a été fait.** `captures/integration-icone/pas1-la-barre-sombre.py`
lit la fenêtre `Shell_TrayWnd` par `PrintWindow` avec `PW_RENDERFULLCONTENT`,
bascule `HKCU\…\Themes\Personalize\SystemUsesLightTheme` de 1 à 0, attend que
la barre se soit repeinte, lit, **restaure la valeur d'origine**, attend de
nouveau, et relit. Un seul réglage a été touché ; `AppsUseLightTheme` et
`EnableTransparency` n'ont pas bougé. Rien n'a été fermé ni déplacé.

**Le témoin, déclaré avant le lancement.** Celui du §I : *le relevé du thème
clair, refait avant et après, doit redonner `#D9D9D9`* — dominante 217 sur cinq
rangées de 301 colonnes, avant comme après. Les cinq rangées sont celles des
fondations (y = 1035, 1045, 1055, 1065, 1075 en pixels physiques d'écran) ; les
fondations ne disent pas *quelles* 301 colonnes, et la règle est donc posée ici,
**mécanique et déclarée avant** : sur chaque rangée, parmi toutes les fenêtres
de 301 colonnes, celle qui porte le moins de couleurs distinctes, à égalité la
plus à gauche. Aucun jugement n'y entre. Second témoin, repris du partisan de
la forme : la largeur de la capture vaut `SM_CXSCREEN`.

**Les épreuves des témoins.** E1 : la même capture faite hors de l'écran doit
rendre une surface vide — **1 couleur distincte, elle tombe**. E2, et c'est le
cœur : la même lecture faite pendant que le thème est sombre doit faire tomber
le témoin T1, sinon la lecture ne distinguerait pas les deux thèmes —
**`#212121` contre le `#D9D9D9` attendu, T1 tombe**.

**Le compte prévu et le compte obtenu.**

| | prévu | obtenu |
|---|---|---|
| largeur de la capture | `SM_CXSCREEN` = 1920 | 1920, **tenu** |
| thème clair, avant | dominante `#D9D9D9`, 217 | `#D9D9D9`, **niveau 217,00**, dominante d'environ un quart de la surface |
| les cinq rangées, avant | 217 partout | 217 / 218 / 217 / 217 / 218 |
| thème clair, après restauration | identique à avant | **identique à l'octet, rangée par rangée, et sur la surface entière** |
| le registre relu après | `SystemUsesLightTheme` = 1 | 1, **identique** |

*Les parts de surface ne sont pas des constantes : la barre contient des
boutons qui vont et viennent, et la dominante en occupe 26 à 27 % selon le
moment. Ce que ce pas mesure, ce sont les VALEURS — 217 et 33 —, et elles ne
bougent pas d'un relevé à l'autre.*

Les cinq rangées lues ici tombent sur exactement les mêmes valeurs que le
relevé des fondations du 18 septembre 2026 (217, 218, 217, 217, 218), par un
chemin de capture différent — celui-ci passe par `PrintWindow`, celui-là par
`GetPixel` sur le contexte du bureau. C'est un second chemin, et il concorde.

**Le chiffre, et ce qu'il décide.**

> ~~Le fond sombre : `#202020`, niveau 32,00, **supposé**~~
> (`ICONE-FONDATIONS.md` §3.1, repris par `DECISION-ICONE.md` §D.4, §H et §G).
>
> **Mesuré : la barre des tâches sombre de cette machine est `#212121` =
> (33, 33, 33), niveau 33,00.** Dominante d'environ un quart de la surface ; les trois
> valeurs les plus fréquentes sont `#212121`, `#222222` et `#202020`, exactement
> la même dispersion de trois niveaux que le côté clair. **Ce qui a changé :**
> un niveau, dans le sens défavorable au sol. La cause n'est pas une erreur des
> fondations — `#202020` est la valeur que le champ publie pour la barre sombre
> de Windows 11, et elle figure d'ailleurs en troisième place de cette mesure —
> mais la barre réelle n'est pas un aplat : elle porte la matière du système.

**Le seuil, déclaré avant le lancement par le §I :** si la barre sombre dépasse
le **niveau 49**, plus aucun sol ne satisfait la règle de Microsoft et tout le
document est à reprendre. **Lu : 33,00, soit seize niveaux sous le seuil.**

Et la conséquence chiffrée, sur la seule valeur que ce pas pouvait renverser :

| | contre `#202020` supposé | contre `#212121` **mesuré** |
|---|---|---|
| le sol `#7C7974` | ~~3,758:1~~ | **3,714:1** |
| la barre demande | ≥ 3,000:1 | ≥ 3,000:1 |

**Le sol tient**, avec 0,714 de marge au lieu de 0,758. La perte est de 0,044,
et elle ne menace aucune des trois contraintes du §D.4 : le côté sombre était
la plus large des trois, et il le reste. La contrainte qui décide reste
SC 1.4.11, à 3,087:1 prédit.

**Ce qui a résisté.** Rien. Le pas a tenu du premier coup, et il a rendu au
dossier son chiffre le plus fragile.

**Ce qui n'est pas mesuré ici.** La barre sombre d'un *autre* écran, d'un autre
réglage de couleur d'accentuation, ou avec la transparence coupée. Ce relevé est
celui de cette machine, ce jour, `EnableTransparency = 1`.

**Sortie :** `captures/integration-icone/pas1-la-barre-sombre.txt`, et les trois
images `pas1-barre-clair-avant.png`, `pas1-barre-sombre.png`,
`pas1-barre-clair-apres.png`.

---

## Pas 2 — Le sol et la largeur, rendus pour de bon

**Ce que le pas demande.** `DECISION-ICONE.md` §G le dit sans détour : *« La
largeur 3/4 et le sol 121,30 sont des PRÉDICTIONS, calculées sur un modèle
linéaire calé sur un seul point mesuré. Rien n'a été rendu à cette largeur ni à
ce sol. »* Ce pas rend les quatorze tailles au sol `#7C7974` et à la part 3/4,
et lit sur le rendu neuf de 256 px les quatre grandeurs du §I.

L'instrument est `captures/integration-icone/pas2-le-sol-et-la-largeur.py`. Il
ne réécrit **aucune** mathématique du chantier : `instrument.py` importe celui
du partisan de la matière et ne redéfinit que ce qui dépend des deux nombres
changés ; `famille.py` appelle la règle du dessin en trois gestes telle
qu'elle est — le §I écrit « la règle du dessin ne change pas », elle n'est donc
pas recopiée, elle est appelée ; `lecture.py` reprend au mot les définitions de
lecture de l'arbitre, pour que les chiffres se comparent aux siens.

### T0 — Le nuanceur du galet n'a pas bougé

Le §G, point 7, porte cette réserve : *« Les images mesurées viennent du
binaire `49d06073`, et un autre agent travaillait dans `src/` pendant ce
chantier. Si le nuanceur du galet change, tous les chiffres du §A changent avec
lui. »* Ce pas la lève.

**Témoin.** Le galet nu rendu à l'échelle 0,6 sur le même sol par le binaire de
l'arbre commité est identique à l'octet à celui de `49d06073`. *L'instrument
fabrique lui-même cette copie hors du dépôt, sous l'empreinte du commit courant :
un premier essai pointait sur un dossier de travail que le ménage avait effacé,
et le pas tombait pour une raison qui n'avait rien à voir avec ce qu'il mesure.
Depuis, il compare le binaire du moment — donc, maintenant, celui qui porte
l'icône.*
**Épreuve.** Le même galet rendu avec `--lampe=0` doit en différer.

| | prévu | obtenu |
|---|---|---|
| l'arbre contre la référence | 0 pixel d'écart | **0 pixel, écart maximal 0** |
| l'épreuve, `--lampe=0` | doit différer | **546 pixels, écart maximal 51 — il tombe** |

**Le nuanceur est le même.** Les chiffres du §A, du §D et du §E tiennent sur
l'arbre d'aujourd'hui. La réserve du §G point 7 est levée, et elle est écrite
levée.

### T1 — Le sol doit déborder le carré, et le document ne l'avait pas vu

C'est le seul endroit où ce pas a résisté, et il valait la peine.

Le nuanceur prend une marge autour du galet pour lire le fond que le verre
montre : `galet::marge(e) = ceil(64 × e)` px physiques, 64 px **logiques** —
soit 0,2 de la largeur du galet, puisque celui-ci fait 320 px logiques. La
marge de sol qui reste dans le carré de l'icône vaut, elle, `(1 − p)/2` du
côté. Les deux se croisent à

    (1 − p)/2 = 0,2 p   →   p = 5/7 = 0,714…

La décision porte `p` à **0,750**. **Aux quatorze tailles, le carré de l'icône
ne porte donc plus la marge du nuanceur** — l'instrument du partisan de la
matière s'y arrêtait net (`le fond ne porte pas la marge du nuanceur à … px`),
et c'est exactement pourquoi il s'arrêtait : à 5/8 la marge tenait, à 3/4 elle
ne tient plus. Rien dans le §I ne le mentionne, et rien dans le §I ne pouvait
le prévoir : aucun rendu n'avait jamais été fait à cette largeur.

*Cette phrase parle de la marge que le nuanceur **déclare**. Le pas 2 bis
mesure celle où il **lit** vraiment, et les deux ne sont pas la même : il ne
sort réellement du carré qu'à 96 et à 256 px.*

**Ce n'est pas un obstacle à la décision : c'est un obstacle à la fenêtre qu'on
donnait au nuanceur.** Le sol est, par la charte que le §H écrit, *un aplat
d'une seule valeur*. Le prolonger au-delà du carré n'invente rien : c'est le
même plan. Le rendu se fait donc sur un sol de `taille + 2·débord` de côté, et
le carré de l'icône en est le découpage central.

| taille | marge du nuanceur | marge de sol dans le carré | débord |
|---|---|---|---|
| 16 | 3 | 2 | 1 |
| 20 | 3 | 2 | 1 |
| 24 | 4 | 3 | 1 |
| 30 | 5 | 4 | 1 |
| 32 | 5 | 4 | 1 |
| 36 | 6 | 4 | 2 |
| 40 | 6 | 5 | 1 |
| 48 | 8 | 6 | 2 |
| 60 | 9 | 7 | 2 |
| 64 | 10 | 8 | 2 |
| 72 | 11 | 9 | 2 |
| 80 | 12 | 10 | 2 |
| 96 | 15 | 12 | 3 |
| 256 | 39 | 32 | 7 |

Et au passage, une correction qui n'est pas cosmétique : l'instrument du
partisan calculait la marge du nuanceur par `floor(64·e + 0,5)`, quand le
nuanceur la calcule lui-même par `ceil(64·e)` — `src/galet.rs:326`, avec sa
raison écrite : *« Arrondie au-dessus pour ne jamais passer sous le noyau »* du
flou. À 256 px les deux diffèrent d'un pixel, ~~38~~ contre **39**, et c'est le
nuanceur qui a raison sur sa propre marge.

**Témoin.** Le sol prolongé de la marge du nuanceur donne le même rendu que le
sol prolongé du double : au-delà de sa marge, le nuanceur ne lit rien.
**Épreuve.** Le débord peint en **blanc** au lieu du sol doit changer le rendu —
sinon le témoin serait vide de sens, car cela voudrait dire que le nuanceur ne
lit pas du tout hors du carré.

| | prévu | obtenu |
|---|---|---|
| débord 7 px contre débord 14 px, à 256 | 0 pixel d'écart | **0 pixel, écart maximal 0** |
| l'épreuve, débord blanc | doit différer | **252 pixels, écart maximal 7 — il tombe** |

À 256 px, le débord de `ceil(64·e)` suffit et il est nécessaire : le nuanceur y
lit bien, et il n'y lit pas plus loin. *Aux treize autres tailles, ces deux
témoins n'ont pas tourné ici — le pas 2 bis les y porte, et il y trouve autre
chose.*

### Les quatorze tailles

Les cotes du galet tombent **exactement** sur le tableau du §I, aux quatorze
tailles, sans une exception :

| taille | famille | galet | débord | sol | corps | bord haut |
|---|---|---|---|---|---|---|
| 16 | dessinée | 12 × 6 | 1 | 121,30 | 44,1 | 85,0 |
| 20 | dessinée | 15 × 7 | 1 | 121,30 | 46,1 | 85,0 |
| 24 | dessinée | 18 × 8 | 1 | 121,30 | 45,1 | 85,0 |
| 30 | dessinée | 22 × 10 | 1 | 121,30 | 44,1 | 85,0 |
| 32 | dessinée | 24 × 11 | 1 | 121,30 | 43,1 | 85,0 |
| 36 | dessinée | 27 × 12 | 2 | 121,30 | 43,9 | 85,0 |
| 40 | dessinée | 30 × 14 | 1 | 121,30 | 44,1 | 85,0 |
| 48 | dessinée | 36 × 17 | 2 | 121,30 | 44,9 | 85,0 |
| 60 | rendue | 45 × 21 | 2 | 121,30 | 44,1 | 85,0 |
| 64 | rendue | 48 × 22 | 2 | 121,30 | 43,1 | 85,0 |
| 72 | rendue | 54 × 25 | 2 | 121,30 | 43,1 | 85,0 |
| 80 | rendue | 60 × 28 | 2 | 121,30 | 44,1 | 85,0 |
| 96 | rendue | 72 × 33 | 3 | 121,30 | 44,1 | 85,0 |
| 256 | rendue | 192 × 89 | 7 | 121,30 | 44,1 | 85,0 |

Le plancher de deux pixels de sol est tenu à la taille qui le borne : à 16 px,
le galet fait 12 px de large dans 16, et il reste **2 px de sol de chaque
côté**, ni plus ni moins. C'est ce plancher qui a fait retenir 3/4 plutôt que
4/5, et il est ici à l'unité près.

### T2 — Le raccord des deux familles à 60 px, refait à 3/4

Le témoin du partisan de la matière, que le §A.6 demande de refaire à chaque
changement : à 60 px les deux familles existent, et elles doivent se rejoindre
à 3 niveaux près — le plancher d'adaptateur.

| grandeur | rendu | dessin | écart | seuil |
|---|---|---|---|---|
| sol | 121,3 | 121,3 | **0,00** | 3,00 |
| corps | 44,1 | 44,9 | **0,78** | 3,00 |
| bord | 85,0 | 85,0 | **0,00** | 3,00 |

**Épreuve.** Le même dessin fait depuis un rendu de 256 px *sans* sa lumière
dessous doit faire tomber le témoin sur le corps : lu **23,1 contre 44,1 au
rendu** — il tombe. La règle de dérivation en trois gestes tient à 3/4 comme
elle tenait à 5/8, et le troisième geste — le chant remis à la valeur que la
loi de la lampe lui donne — y est pour tout : le bord haut vaut **85,0 aux
quatorze tailles**, dessinées comme rendues.

### Les quatre grandeurs du §I, lues sur le rendu neuf de 256 px

| grandeur | compte prévu | compte obtenu | |
|---|---|---|---|
| sol | 121,30 | **121,30** | tenu |
| aire du galet | 0,2565 ± 0,002 | **0,2581** | tenu (écart 0,0016) |
| premier pixel du bord | 84 à 85 | **85,00** | tenu |
| cœur | 47,0 ± 1,5 | **46,62** | tenu |
| SC 1.4.11, le cœur contre le sol | ≥ 3,000, prédit 3,087 | **3,105:1** | tenu |

L'aire calculée depuis la superellipse d'exposant 4 à 3/4 vaut 0,2565 ; l'aire
lue sur l'image vaut 0,2581. **L'écart de 0,0016 est le ruban d'anticrénelage du
bord**, que la lecture compte comme objet dès qu'il s'écarte du sol d'un niveau
— et ce n'est pas une explication, c'est une mesure : la couverture exacte de la
superellipse compte **124 pixels partiellement couverts** à 256 px, soit
**0,0019 du carré**. Le ruban est plus grand que l'écart, donc il suffit à en
rendre compte.

**La prédiction la plus fragile du dossier — le sol 121,30 et la largeur 3/4 —
tient, et elle tient avec un peu plus de marge que prédit** : SC 1.4.11 lit
3,105:1 là où le modèle linéaire prédisait 3,087:1. Le repli que le §I avait
préparé — monter le sol d'un niveau, puis redescendre la largeur d'un huitième
— n'a pas servi.

### Ce qui a résisté

Une chose, et une seule : **la marge du nuanceur**. Le document ne l'avait pas
vue parce qu'aucun rendu n'avait jamais été fait à 3/4 ; l'instrument du
partisan s'arrêtait net à la première taille. Elle a été résolue, pas
contournée : le sol est un plan, il est peint comme un plan, et l'icône en est
un découpage. Deux témoins et deux épreuves le prouvent.

**Sortie :** `captures/integration-icone/pas2-le-sol-et-la-largeur.txt`, les
quatorze images et les rendus bruts du nuanceur sous
`captures/integration-icone/icones/`, les images des témoins sous `temoins/`.

---

## Pas 2 bis — Le débord du sol, prouvé aux quatorze tailles

Le pas 2 n'avait prouvé le débord qu'à 256 px, et le journal l'écrivait comme
une faiblesse. Il est prouvé ici aux quatorze — et la mesure a rendu quelque
chose que je n'avais pas prévu.

**Les deux témoins du pas 2, refaits à chaque taille.** T1 : le débord
`ceil(64 × e)` contre son double, 0 pixel d'écart. T2, l'épreuve : le même
débord peint en **blanc** doit changer le rendu.

| taille | débord | T1, simple contre double | T2, l'épreuve du blanc |
|---|---|---|---|
| 16 | 1 | 0 pixel | **0 pixel — elle ne tombe pas** |
| 20 | 1 | 0 pixel | 0 pixel |
| 24 | 1 | 0 pixel | 0 pixel |
| 30 | 1 | 0 pixel | 0 pixel |
| 32 | 1 | 0 pixel | 0 pixel |
| 36 | 2 | 0 pixel | 0 pixel |
| 40 | 1 | 0 pixel | 0 pixel |
| 48 | 2 | 0 pixel | 0 pixel |
| 60 | 2 | 0 pixel | 0 pixel |
| 64 | 2 | 0 pixel | 0 pixel |
| 72 | 2 | 0 pixel | 0 pixel |
| 80 | 2 | 0 pixel | 0 pixel |
| 96 | 3 | 0 pixel | **4 pixels, max 2 — elle tombe** |
| 256 | 7 | 0 pixel | **252 pixels, max 7 — elle tombe** |

**T1 tient aux quatorze.** Le débord de `ceil(64 × e)` suffit partout : le
doubler ne change rien, à aucune taille.

**T2 ne tombe qu'à 96 et à 256 px, et c'est là que la mesure apprend quelque
chose.**

> **La marge que `galet::marge` DÉCLARE — 64 px logiques — n'est pas la distance
> à laquelle le nuanceur LIT.** En dessous de 96 px, peindre le débord en blanc
> ne change pas un pixel : à ces échelles, le noyau du flou, quantifié sur la
> grille de l'icône, ne sort pas du carré.
>
> Trois faits, tous mesurés : le débord est **nécessaire** à 96 et à 256 px ; il
> est **sans effet** de 16 à 80 px ; il n'est **nuisible nulle part**.
>
> **On le garde aux quatorze.** Il coûte zéro pixel, il suit la marge que le
> nuanceur déclare, et une marge qui suit la déclaration du nuanceur restera
> juste si le noyau du flou change un jour.
>
> Et la phrase du pas 2 se précise : le carré de l'icône ne porte plus la marge
> **déclarée** à aucune des quatorze tailles — c'est vrai, et c'est ce qui
> faisait abandonner l'instrument du partisan —, mais le nuanceur n'a réellement
> besoin de sortir du carré qu'aux **deux plus grandes**.

**Ce qui a résisté.** L'épreuve, à douze tailles sur quatorze. Et c'est elle qui
avait raison : un témoin qui ne peut pas tomber ne dit rien, et savoir *où* il
ne tombe pas est ce qui a transformé une précaution en un fait.

**Sortie :** `captures/integration-icone/pas2bis-le-debord-aux-quatorze.txt`.

---

## Pas 3 — C5 et C7, sur le dessin neuf, aux quatorze tailles

**Ce que le pas demande.** *« Mesurer pixel par pixel, aux quatorze tailles, la
part de l'aire qui tient 3:1 contre `#D9D9D9` mesuré et contre le fond sombre
du pas 1 »* (§I). L'instrument est `cibles.py` du partisan de la matière,
**seuils inchangés** : rien n'en est réécrit, ses fonctions sont appelées par
`captures/integration-icone/pas3-c5-et-c7.py`.

Le fond sombre n'est plus supposé : le pas 1 l'a mesuré à `#212121`. Les trois
colonnes sont imprimées côte à côte — le clair mesuré, le sombre mesuré, et le
~~`#202020`~~ supposé — pour que la comparaison avec les chiffres publiés reste
possible.

### L'épreuve, faite avant la mesure

Le §I l'écrit : *« le même calcul sur un carré uniformément `#D9D9D9` doit
rendre 0 % des deux côtés »*. Elle est faite en premier, et on lui ajoute son
miroir, qui n'était pas demandé et qui coûte une ligne — sans lui, l'épreuve ne
ferait tomber l'instrument que d'un seul côté.

| carré uniforme | clair | sombre | les deux à la fois | C5 |
|---|---|---|---|---|
| `#D9D9D9` | 0,0 % | 100,0 % | **0,0 %** | ratée |
| `#212121` | 100,0 % | 0,0 % | **0,0 %** | ratée |

L'instrument tombe des deux côtés.

### C5 — le contraste sur les deux thèmes

> « Make sure at least half of your icon passes a 3.0:1 contrast ratio on light
> and dark theme. » (`app-icon-design`, `ms.date` 2021-10-29, `updated_at`
> 2026-07-14)

| taille | contre `#D9D9D9` | contre `#212121` mesuré | contre ~~`#202020`~~ | C5 |
|---|---|---|---|---|
| 16 | 100,0 % | **71,9 %** | 71,9 % | oui |
| 20 | 100,0 % | 73,8 % | 73,8 % | oui |
| 24 | 100,0 % | 75,0 % | 75,0 % | oui |
| 30 | 100,0 % | **75,6 %** | 75,6 % | oui |
| 32 | 100,0 % | 74,2 % | 74,2 % | oui |
| 36 | 100,0 % | 75,2 % | 75,2 % | oui |
| 40 | 100,0 % | 73,9 % | 73,9 % | oui |
| 48 | 100,0 % | 73,6 % | 73,6 % | oui |
| 60 | 100,0 % | 73,9 % | 73,9 % | oui |
| 64 | 100,0 % | 74,3 % | 74,3 % | oui |
| 72 | 100,0 % | 74,2 % | 74,2 % | oui |
| 80 | 100,0 % | 73,9 % | 73,9 % | oui |
| 96 | 100,0 % | 74,4 % | 74,4 % | oui |
| 256 | 100,0 % | 74,2 % | 74,2 % | oui |

**C5 est atteinte aux quatorze tailles**, et la cible demande 50 % : la plus
faible des vingt-huit lectures est à **71,9 %**, soit vingt-deux points de
marge. La cible qui ne cède pas ne cède pas.

**Le compte prévu, lui, est démenti sur un point, et il faut le barrer.**

> ~~C5 sombre **≥ 74 %** (prédit 74,4)~~ (`DECISION-ICONE.md` §I, pas 3).
>
> **Lu : de 71,9 % à 75,6 % selon la taille.** Six tailles sur quatorze passent
> sous 74 % — 16, 20, 40, 48, 60, 80 —, la plus basse à 71,9 %.
>
> **Ce qui a changé, et sa cause, mesurée et non raisonnée.** Du côté sombre,
> ce qui passe 3:1 est **le sol, et lui seul** : le galet y lit 1,2:1. La part
> qui passe vaut donc exactement `1 − aire du galet`, et l'aire du galet varie
> avec l'arrondi au pixel de sa **hauteur**. L'instrument le vérifie taille par
> taille, par un second chemin — l'aire lue sur l'image par la lecture de
> l'arbitre :
>
> | taille | hauteur nominale | hauteur posée | aire lue | 1 − aire | part sombre |
> |---|---|---|---|---|---|
> | 16 | 5,55 | 6 | 0,2812 | 0,7188 | 0,7188 |
> | 20 | 6,94 | 7 | 0,2625 | 0,7375 | 0,7375 |
> | 24 | 8,33 | 8 | 0,2500 | 0,7500 | 0,7500 |
> | 30 | 10,41 | 10 | 0,2444 | 0,7556 | 0,7556 |
> | 32 | 11,10 | 11 | 0,2578 | 0,7422 | 0,7422 |
> | 36 | 12,49 | 12 | 0,2500 | 0,7500 | 0,7515 |
> | 40 | 13,88 | 14 | 0,2625 | 0,7375 | 0,7388 |
> | 48 | 16,65 | 17 | 0,2656 | 0,7344 | 0,7361 |
> | 60 | 20,81 | 21 | 0,2614 | 0,7386 | 0,7386 |
> | 64 | 22,20 | 22 | 0,2568 | 0,7432 | 0,7432 |
> | 72 | 24,98 | 25 | 0,2596 | 0,7404 | 0,7419 |
> | 80 | 27,75 | 28 | 0,2606 | 0,7394 | 0,7394 |
> | 96 | 33,30 | 33 | 0,2556 | 0,7444 | 0,7444 |
> | 256 | 88,80 | 89 | 0,2581 | 0,7419 | 0,7423 |
>
> Les deux dernières colonnes se suivent au millième, aux quatorze tailles. À
> 16 px la hauteur nominale de 5,55 px se pose sur 6, soit **+8 % d'aire**, et
> −2,2 points sur la part sombre. **Le 74,4 % prédit était celui de 256 px
> seul**, et il y est exact à deux dixièmes près (74,2 lu). Ce n'est pas la
> prédiction qui est fausse, c'est sa portée : elle valait pour une taille, et
> le §I la demandait aux quatorze.

### C7 — abandonnée (§B.3), et imprimée

*« L'arbitre doit dire quand sa coupe ne saigne pas »* (§B.3). Elle ne saigne
pas : C7, abandonnée, est **tenue aux quatorze tailles et des deux côtés**.

| taille | moyenne | contre `#D9D9D9` | contre `#212121` mesuré | contre ~~`#202020`~~ |
|---|---|---|---|---|
| 16 | 107,6 | 3,742:1 | 3,049:1 | 3,085:1 |
| 20 | 108,3 | 3,701:1 | 3,082:1 | 3,119:1 |
| 24 | 109,0 | 3,667:1 | 3,111:1 | 3,148:1 |
| 30 | 109,1 | **3,659:1** | 3,117:1 | 3,154:1 |
| 32 | 108,3 | 3,702:1 | 3,082:1 | 3,118:1 |
| 36 | 108,7 | 3,683:1 | 3,097:1 | 3,134:1 |
| 40 | 107,9 | 3,725:1 | 3,062:1 | 3,099:1 |
| 48 | 107,7 | 3,739:1 | 3,051:1 | 3,087:1 |
| 60 | 107,6 | 3,741:1 | 3,050:1 | 3,086:1 |
| 64 | 107,9 | 3,727:1 | 3,061:1 | 3,097:1 |
| 72 | 107,7 | 3,737:1 | 3,053:1 | 3,089:1 |
| 80 | 107,6 | 3,744:1 | 3,047:1 | 3,083:1 |
| 96 | 107,8 | 3,732:1 | 3,057:1 | 3,093:1 |
| 256 | 107,5 | 3,746:1 | **3,045:1** | 3,081:1 |

**Les comptes prévus, et ce qu'ils deviennent.**

| compte prévu par le §I | obtenu | |
|---|---|---|
| C5 clair, 100 % | 100,0 % aux quatorze | **tenu** |
| ~~C5 sombre ≥ 74 %~~ | **71,9 % à 75,6 %** | barré ci-dessus, avec sa cause |
| ~~C7 contre le clair, 3,74 ± 0,05~~ | **3,659 à 3,746** | même cause |
| C7 contre le sombre, 3,09 ± 0,05 | 3,045 à 3,117 | **tenu** |

Les deux comptes barrés le sont pour la **même** raison, et c'est la seule de ce
pas : ils ont été calculés sur le rendu de 256 px, où la quantification au
pixel est négligeable, puis demandés aux quatorze tailles, où elle ne l'est
plus. Les deux qui tiennent sont ceux que la quantification ne touche pas — le
côté clair, qui passe à 100 % quelle que soit l'aire, et le côté sombre de C7,
dont la bande prévue était deux fois plus large.

### Ce qui a résisté

Rien qui touche au dessin. Ce qui a résisté, ce sont deux **comptes prévus**,
et la mesure a dit pourquoi : une prédiction faite à une seule taille ne se
transporte pas aux treize autres quand la grandeur dépend d'un arrondi au
pixel. **Les deux cibles, elles, sont tenues partout** — C5 avec vingt-deux
points de marge, C7 abandonnée et tenue quand même.

**Sortie :** `captures/integration-icone/pas3-c5-et-c7.txt`.

---

## Pas 4 — C12, la cible neuve : la matière se lit

**Ce que le pas demande.** Lancer `captures/arbitre-icone/le-troisieme-chemin.py`
sur les images neuves, en ajoutant les quatre tailles ; les trois témoins et les
quatre épreuves du §A.3, **tels quels**.

L'instrument n'est pas réécrit : `captures/integration-icone/pas4-c12.py`
l'importe et ne lui change que les images qu'il regarde et la liste des tailles.
Ses deux grandeurs, son cadre du juge, son œil de 2 × 2 px, son plancher de
3 niveaux, sa composante de 4 blocs, ses mille permutations et sa graine
`20260918` sont les siens, intacts.

**Une chose est ajoutée.** Le témoin T1 de l'arbitre bâtit son cadre vrai aux
valeurs de *son* icône — sol 119, corps 45, chant 85, part 5/8. Il est repris
tel quel, puisque le §I le demande tel quel. On lui ajoute un second cadre vrai,
bâti aux valeurs de l'icône neuve — sol 121,30, corps 47, part 3/4 —, parce
qu'un contrôle qui ne partagerait plus aucune valeur avec l'objet contrôlé
serait moins fort.

### Les trois témoins

| témoin | compte prévu | compte obtenu | |
|---|---|---|---|
| T1, le cadre de l'arbitre | 0 bloc retenu aux quatorze tailles | **0 bloc, écart 0,000, aux quatorze** | vrai |
| T1 bis, le cadre aux valeurs neuves | idem | **0 bloc, écart 0,000, aux quatorze** | vrai |
| T2, à 256 px | plus grande composante > 1 000 et ρ > 0,7 | **1 772 blocs de composante, ρ = 0,7394 contre un seuil de hasard de 0,0749** | vrai |
| T3, l'ordre du tour à 256 px | haut > flancs > bas | **83,1 / 60,0 / 49,4** | vrai |

L'arbitre lisait 81,7 / 60,0 / 46,0 à la part 5/8 : l'ordre est le même, et
l'écart haut–bas s'est resserré de 35,7 à 33,7 niveaux — le galet plus grand
présente son tour sur plus d'échantillons, à valeurs de lampe inchangées.

### Les quatre épreuves

| épreuve | ce qui doit arriver | ce qui arrive |
|---|---|---|
| E1, le cadre du juge remplacé par l'image elle-même | la grandeur 1 tombe partout | **0 cas debout sur 14** |
| E2, le plancher porté de 3 à 256 niveaux | idem | **0 cas debout sur 14** |
| E3, l'œil moyenne l'objet entier au lieu de 2 × 2 px | idem | **0 cas debout sur 14** |
| E4, l'image retournée de haut en bas | ρ change de signe | **0,8668 → −0,8668 à 30 px ; 0,8393 → −0,8393 à 64 ; 0,7394 → −0,7394 à 256. Trois sur trois** |

### Le verdict, aux quatorze tailles

| taille | blocs retenus | plus grande composante | ρ | seuil du hasard | verdict |
|---|---|---|---|---|---|
| 16 | 12 | 6 | 0,954 | 0,297 | **matière** |
| 20 | 16 | 9 | 0,884 | 0,272 | **matière** |
| 24 | 18 | 10 | 0,891 | 0,219 | **matière** |
| 30 | 35 | 28 | 0,867 | 0,210 | **matière** |
| 32 | 48 | 48 | 0,870 | 0,187 | **matière** |
| 36 | 58 | 58 | 0,800 | 0,193 | **matière** |
| 40 | 90 | 70 | 0,795 | 0,189 | **matière** |
| 48 | 124 | 124 | 0,785 | 0,157 | **matière** |
| 60 | 198 | 198 | 0,865 | 0,147 | **matière** |
| 64 | 236 | 132 | 0,839 | 0,138 | **matière** |
| 72 | 254 | 136 | 0,763 | 0,131 | **matière** |
| 80 | 338 | 180 | 0,881 | 0,126 | **matière** |
| 96 | 482 | 242 | 0,876 | 0,116 | **matière** |
| 256 | 3 340 | 1 772 | 0,739 | 0,075 | **matière** |

**C12 est atteinte aux quatorze tailles, 16 px comprise.** Les deux grandeurs
passent partout : la plus grande composante n'est jamais sous 4 blocs — sa
valeur la plus basse est 6, à 16 px —, et ρ n'est jamais sous son seuil de
hasard, le rapport le plus serré étant 0,954 contre 0,297 à 16 px.

### Où vit ce qui distingue l'icône d'un cadre

| taille | part sur le tour | part dans le corps |
|---|---|---|
| 16 | **100,0 %** | 0,0 % |
| 20 | **100,0 %** | 0,0 % |
| 24 | **100,0 %** | 0,0 % |
| 30 | **80,0 %** | 20,0 % |
| 32 | 62,5 % | 37,5 % |
| 36 | 58,6 % | 41,4 % |
| 40 | 37,8 % | 62,2 % |
| 48 | 40,3 % | 59,7 % |
| 60 | 27,8 % | 72,2 % |
| 64 | 28,8 % | 71,2 % |
| 72 | 22,0 % | 78,0 % |
| 80 | 23,1 % | 76,9 % |
| 96 | 20,3 % | 79,7 % |
| 256 | 7,8 % | **92,2 %** |

**Le compte prévu :** *« la part de la différence portée par le tour aux petites
tailles doit rester ≥ 70 % à 30 px : c'est ce que la décision achète »*.
**Lu à 30 px : 80,0 %** — dix points au-dessus du compte, et six points de plus
que les 74,1 % que l'arbitre lisait à la part 5/8. **La décision a bien acheté
ce qu'elle disait acheter**, et à la taille que cette machine demande le plus
souvent.

### Ce qui a résisté

Rien. Les quatre témoins tiennent, les quatre épreuves tombent, le verdict est
matière aux quatorze tailles, et le seul compte chiffré du pas est dépassé.

**Sortie :** `captures/integration-icone/pas4-c12.txt`.

---

## Pas 5 — C1, C4, C6 et C8, sur le dessin neuf

**Ce que le pas demande.** *« Rejouer `cibles.py` ; ses propres épreuves,
inchangées »* (§I). L'instrument est importé par
`captures/integration-icone/pas5-c1-c4-c6-c8.py` ; seuls le sol, la part et les
quatorze tailles changent. Trois épreuves sont **ajoutées** — pour C4, C6 et C8,
qui n'en avaient pas chez le partisan — parce que le socle demande que tout
témoin puisse tomber, et qu'un chiffre qu'aucune épreuve ne peut faire tomber
n'est pas une mesure.

### C1 — la silhouette tient à 20 px

| | prévu | obtenu | |
|---|---|---|---|
| composantes connexes | 1 | **1** | tenu |
| plus petite composante | ≥ 4 px | 90 px | tenu |
| plus petit trait porteur | 7 px (≥ 2) | **7 px** | tenu |
| aire de la silhouette | ~~≥ 95 px (prédite 103)~~ | **90 px** | barré |

**C1 est atteinte** sur ses trois critères propres. Le compte d'aire du §I, lui,
est démenti, et **la cause est le troisième geste de la règle du dessin
lui-même**.

> ~~aire ≥ 95 px (prédite 103 : le galet y fait 15 × 7)~~
> (`DECISION-ICONE.md` §I, pas 5).
>
> **Lu : 90 px.** Trois grandeurs portent le nom d'« aire » et ne valent pas la
> même chose à 20 px :
>
> | | grandeur | à 20 px |
> |---|---|---|
> | a | l'aire **géométrique** de la superellipse, par la formule du coin d'exposant 4 | 103,5 px — c'est le 103 prédit |
> | b | les pixels **couverts à moitié ou plus** | 105 px |
> | c | la **silhouette de C1**, binarisée au milieu de sa propre dynamique (seuil 76,6 niveaux) | **90 px** |
>
> C1 lit (c) ; le §I prédisait (a). **Ce qui a changé, et sa cause :** le chant
> est remis à la valeur que la loi de la lampe lui donne — 85 au sommet, à toute
> échelle — et 85 est **au-dessus** du milieu de la dynamique de l'image, qui
> vaut 76,6. La rangée éclairée du galet tombe donc du côté *clair* de la
> binarisation, et la silhouette ne la compte pas. Le compte est exact au pixel :
> 105 pixels de galet, dont **15 au-dessus du seuil** — treize à 85,0 et deux à
> 87,1, exactement la rangée haute —, et 105 − 15 = **90**, qui est ce que
> l'instrument lit.
>
> Ce n'est donc pas un défaut du dessin. C'est ce que le dessin a de plus
> vivant — la lumière qui fait le tour de l'objet, celle que le §A mesure comme
> **la** matière aux petites tailles — qui sort de la silhouette de C1. Une
> silhouette binaire ne sait pas voir une lampe.

**Épreuve.** Le même comptage sur une version volontairement détaillée — le
galet plus trois points de 2 px — doit rendre plus de 2 composantes : **4
composantes, aires 88, 4, 4, 4.** L'instrument tombe.

### C4 — la borne du bord

> « la lumière que le **verre** émet au **premier pixel du bord**, coins
> compris […] ne dépasse pas `#5E5B57` » (`DESIGN.md` §8), soit 91 niveaux, plus
> un octet de marge : 92.

**Lu : 85,0 aux quatorze tailles, sans une exception.** Le compte du §I — 84 à
85, jamais 92 — est **tenu**, et il est tenu à la valeur exacte : la loi de la
lampe donne 85 au premier pixel à toute échelle, et les quatorze tailles le
rendent.

**Épreuve, ajoutée ici.** Une borne doit pouvoir être franchie : le même calcul
sur l'icône de 256 px dont le premier anneau du bord est forcé à 120 lit
**149,3 contre 92**. L'instrument tombe.

### C6 — la lumière garde sa direction et son ordre

`w(n) = (1 − n_y)/2` : 1 au bord haut, 0,5 aux flancs, 0 au bas.

| taille | haut | flancs | bas | ordre |
|---|---|---|---|---|
| 16 | 64,5 | 52,1 | 40,1 | oui |
| 20 | 63,6 | 51,2 | 39,1 | oui |
| 24 | 63,0 | 50,1 | 38,6 | oui |
| 30 | 62,0 | 49,3 | 37,5 | oui |
| 32 | 61,6 | 49,1 | 37,1 | oui |
| 36 | 61,1 | 48,3 | 36,6 | oui |
| 40 | 60,5 | 47,9 | 36,1 | oui |
| 48 | 59,6 | 47,2 | 35,1 | oui |
| 60 | 58,6 | 46,1 | 31,1 | oui |
| 64 | 58,4 | 45,9 | 31,0 | oui |
| 72 | 58,1 | 45,6 | 31,1 | oui |
| 80 | 58,1 | 45,5 | 31,1 | oui |
| 96 | 57,6 | 45,1 | 30,8 | oui |
| 256 | 58,0 | 45,5 | 33,6 | oui |

**C6 est atteinte aux quatorze tailles.** C'est la cible que le §A.6 déclare
porteuse, et elle ne faiblit nulle part : l'écart haut–bas, calculé sur les
deux colonnes du tableau, va de **24,4 niveaux** au plus serré (16, 24, 40 et
256 px) à **27,5** au plus large (60 px), et il ne descend jamais sous 24,4.

**Épreuve, ajoutée ici.** La même lecture sur le galet retourné de haut en bas
doit **inverser** l'ordre, aux six tailles essayées : les valeurs reviennent en
miroir exact — 256 : 33,6 / 45,5 / 58,0 ; 64 : 31,0 / 45,9 / 58,4 ; 40 : 36,1 /
47,9 / 60,5 ; 30 : 37,5 / 49,3 / 62,0 ; 20 : 39,1 / 51,2 / 63,6 ; 16 : 40,1 /
52,1 / 64,5. L'instrument tombe.

*Un détail que l'épreuve a coûté, et qui est une recette :* c'est la **boîte du
galet** qu'il faut retourner, pas le carré entier. Le galet n'est pas toujours
centré au pixel près — à 20 px il tombe en `y = 6` avec 7 rangées sous lui —, et
retourner le carré le décale d'une rangée, ce qui fait lire le tour à côté de
lui. Un premier essai a rendu, pour cette seule raison, un ordre non inversé à
20 px.

### C8 — ce qui récompense à cinq centimètres

Le trait nommé **avant** d'être cherché : la vignette du corps, l'écart en
niveaux entre le milieu du corps et ses bords, que le flou du verre creuse sur
la lumière posée dessous.

| taille | milieu du corps | bord du corps | vignette | > 3 niveaux ? |
|---|---|---|---|---|
| 16 | 46,9 | 42,1 | 4,8 | oui |
| 20 | 46,9 | 46,1 | 0,7 | non |
| 24 | 46,9 | 45,1 | 1,7 | non |
| 30 | 46,9 | 44,1 | 2,7 | non |
| 32 | 46,9 | 43,1 | 3,7 | oui |
| 36 | 46,1 | 42,1 | 4,0 | oui |
| 40 | 46,9 | 41,1 | 5,8 | oui |
| 48 | 46,9 | 38,9 | **8,0** | oui |
| 60 | 46,9 | 35,1 | 11,7 | oui |
| 64 | 46,9 | 35,1 | 11,7 | oui |
| 72 | 46,9 | 34,1 | 12,7 | oui |
| 80 | 46,9 | 34,1 | 12,8 | oui |
| 96 | 46,9 | 34,1 | 12,7 | oui |
| 256 | 46,9 | 31,1 | **15,7** | oui |

**C8 est ratée, et elle avait été écrite ratée.** Le compte du §I — *« ≥ 12
niveaux à 256 px et encore > 3 à 48 px »* — est **tenu** : 15,7 à 256 et 8,0 à
48. Le §I l'avait prévu mot pour mot : *« Un C8 qui passerait après
l'élargissement serait une surprise : la vignette grandit avec le galet. Publier
le chiffre, ne pas le maquiller. »* Elle a grandi — 14,8 à 256 et 5,8 à 48 chez
le partisan, 15,7 et 8,0 ici — et le chiffre est publié.

**Épreuve, ajoutée ici.** La même lecture sur une icône dont la lumière posée
dessous est retirée doit rendre une vignette **nulle** à 256 px, puisque la
vignette est précisément ce que le flou du verre creuse sur cette lumière : lu
**0,0 niveau contre 15,7 avec**. L'instrument tombe.

*Une observation non demandée, et qui n'est pas une cible :* la vignette n'est
pas monotone en dessous de 32 px — 4,8 à 16 px contre 0,7 à 20 px. À ces
tailles le cœur ne compte plus qu'une poignée de pixels, et la médiane du
cinquième le plus profond contre celle du cinquième le plus superficiel se lit
sur trop peu d'échantillons pour être une grandeur. Elle est publiée telle
quelle, et elle ne sert à rien.

### Le compte du pas

| | résultat |
|---|---|
| C1 | **atteinte** ; le compte d'aire du §I barré, avec sa cause mesurée au pixel |
| C4 | **atteinte**, 85,0 aux quatorze tailles ; compte tenu |
| C6 | **atteinte** aux quatorze tailles ; compte tenu |
| C8 | **ratée**, écrite ratée d'avance ; compte tenu (15,7 à 256 ; 8,0 à 48) |
| les quatre épreuves | **4 sur 4 font tomber leur instrument** |

### Ce qui a résisté

Deux choses, et aucune ne touche au dessin. L'aire de C1, dont la cause est le
troisième geste lui-même — une silhouette binaire ne sait pas voir une lampe.
Et l'épreuve de C6, qui a d'abord échoué parce qu'elle retournait le carré au
lieu du galet ; c'est une recette, et elle est portée au pas 11.

**Sortie :** `captures/integration-icone/pas5-c1-c4-c6-c8.txt`, et les images
des épreuves sous `temoins/`.

---

## Pas 6 — Le fichier `.ico`, quatorze entrées

**Ce que le pas demande.** Écrire `veilleuse.ico` octet par octet depuis la
structure de l'article de 1995 : la 256 en PNG 32 bpp ARGB, les treize autres en
DIB 32 bpp BGRA non prémultipliée avec leur masque AND.

L'écrivain n'est pas réécrit : `ecrire_ico` du partisan de la matière est appelé
tel quel par `captures/integration-icone/pas6-le-fichier.py`. Il pose `ICONDIR`
+ `ICONDIRENTRY[]` + les images, encode 256 sur l'octet 0, et donne à chaque
entrée DIB son masque AND — nul, puisque l'icône est entièrement opaque, mais
présent, comme l'article de 1995 l'exige.

**Le fichier va dans l'application, pas dans une capture : `assets/veilleuse.ico`,
158 377 octets.** C'est lui que le `build.rs` du pas 7 embarque.

### Le témoin, et son épreuve, déclarés avant le lancement

« 0 pixel d'écart » ne veut rien dire tant qu'on n'a pas montré que l'instrument
sait rendre autre chose que zéro. Avant toute mesure, chaque entrée est donc
comparée à l'entrée **voisine** du même fichier, ramenée à sa taille — l'écart
doit être non nul aux quatorze —, et à **elle-même** — il doit être exactement
zéro.

| | prévu | obtenu |
|---|---|---|
| contre la voisine | non nul aux quatorze | **de 69 pixels (24 px) à 12 578 (256 px)**, écart maximal 11 à 93 |
| contre elle-même | zéro aux quatorze | **0 pixel** |

L'instrument est borné par le haut et par le bas. Tout ce qui suit se lit avec
ça.

### 1. La relecture par son propre `ICONDIR`

`réservé = 0`, `type = 1`, `compte = 14`.

| entrée | taille | plans | bits | octets | PNG ? |
|---|---|---|---|---|---|
| 0 | 16 | 1 | 32 | 1 128 | non |
| 1 | 20 | 1 | 32 | 1 720 | non |
| 2 | 24 | 1 | 32 | 2 440 | non |
| 3 | 30 | 1 | 32 | 3 760 | non |
| 4 | 32 | 1 | 32 | 4 264 | non |
| 5 | 36 | 1 | 32 | 5 512 | non |
| 6 | 40 | 1 | 32 | 6 760 | non |
| 7 | 48 | 1 | 32 | 9 640 | non |
| 8 | 60 | 1 | 32 | 14 920 | non |
| 9 | 64 | 1 | 32 | 16 936 | non |
| 10 | 72 | 1 | 32 | 21 640 | non |
| 11 | 80 | 1 | 32 | 26 600 | non |
| 12 | 96 | 1 | 32 | 38 056 | non |
| 13 | **256** | 1 | 32 | 4 771 | **oui** |

Une seule entrée en PNG, et c'est la 256 : c'est exactement l'ancienne guidance
que Raymond Chen rapporte dans *The evolution of the ICO file format, part 4:
PNG images*
(https://devblogs.microsoft.com/oldnewthing/20101022-00/?p=12473, publié le
22 octobre 2010, cité par `ICONE-FONDATIONS.md` §1.3) — *« ne compresser que
l'image 256 × 256 »*. La formule est celle des fondations, qui ont lu la page ;
je ne l'ai pas rouverte.

### L'union de la table *Icon scaling*, vérifiée case par case

| échelle | barre de titre | barre des tâches | épingles Démarrer |
|---|---|---|---|
| 100 % | 16 | 24 | 32 |
| 125 % | **20** | **30** | **40** |
| 150 % | 24 | 36 | 48 |
| 200 % | 32 | 48 | 64 |
| 250 % | 40 | 60 | 80 |
| 300 % | 48 | 72 | 96 |
| 400 % | 64 | 96 | 256 |

L'union des vingt et une cases vaut
`16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256`. Les entrées du
fichier valent la même liste, dans le même ordre. **C'est exactement l'union,
sans une de trop ni une de moins** — et donc : aucune taille de la table n'est
laissée à la mise à l'échelle de Windows, à aucune échelle, sur aucune des trois
surfaces.

### 2. La relecture par Pillow

**0 pixel de différence, écart maximal 0, aux quatorze tailles**, 256 × 256
comprise. Un décodeur qui n'est pas le nôtre relit ce qu'on a voulu y mettre.

### 3. La relecture par `LoadImageW` — le chemin de l'application

C'est le chemin de `IconExtWindows::from_resource` (`winit 0.30.13`) et celui de
l'Explorateur. C'est donc lui qui décide de C9 : *« Windows ira-t-il chercher
l'entrée qu'on a dessinée, ou en réduira-t-il une autre ? »*

| demandé | servi | pixels différents | écart maximal | mise à l'échelle ? |
|---|---|---|---|---|
| 16 | 16 × 16 | 0 | 0 | non |
| 20 | 20 × 20 | 0 | 0 | non |
| 24 | 24 × 24 | 0 | 0 | non |
| 30 | 30 × 30 | 0 | 0 | non |
| 32 | 32 × 32 | 0 | 0 | non |
| 36 | 36 × 36 | 0 | 0 | non |
| 40 | 40 × 40 | 0 | 0 | non |
| 48 | 48 × 48 | 0 | 0 | non |
| 60 | 60 × 60 | 0 | 0 | non |
| 64 | 64 × 64 | 0 | 0 | non |
| 72 | 72 × 72 | 0 | 0 | non |
| 80 | 80 × 80 | 0 | 0 | non |
| 96 | 96 × 96 | 0 | 0 | non |
| **256** | **256 × 256** | **0** | **0** | **non** |

**Taille demandée contre taille servie : zéro pixel d'écart, aux quatorze, la
256 × 256 comprise.** C'est le cœur de ce chantier, et il tient.

### 4. GDI+, qui n'est le chemin de personne, et le piège à garder écrit

Le §I le prévoyait mot pour mot : *« GDI+ en rendra treize et servira la 96 à
qui demande la 256 »*. Lu : **treize justes sur quatorze, et le 96 servi à qui
demande la 256.** Le compte est tenu.

GDI+ (`System.Drawing.Icon`) ne sait pas lire une entrée compressée en PNG. Ce
n'est le chemin ni de `winit` ni de l'Explorateur — mais tout outil qui passe
par `System.Drawing.Icon` ne verra jamais notre plus grande taille, et **il ne
faut donc pas s'en servir comme instrument de vérification**. Le partisan de la
matière l'avait trouvé à ses dépens ; le passage de dix à quatorze entrées n'a
fait que déplacer la substitution de la 64 à la 96.

### 5. L'écart à une réduction mécanique de la 256 (C9)

Un écart **nul** voudrait dire que la taille n'a pas été dessinée mais réduite.

| taille | écart moyen | écart maximal | part de l'écart sur le bord |
|---|---|---|---|
| 16 | 3,64 | 29,9 | 77,8 % |
| 20 | 4,77 | 47,4 | 33,6 % |
| 24 | 3,52 | 54,9 | 60,8 % |
| 30 | 3,69 | 56,9 | 57,1 % |
| 32 | 2,74 | 56,3 | 34,6 % |
| 36 | 3,41 | 63,0 | 53,6 % |
| 40 | 1,31 | 33,9 | 74,3 % |
| 48 | 2,50 | 58,8 | 77,4 % |
| 60 | 2,39 | 59,2 | 68,6 % |
| 64 | 1,24 | 38,9 | 58,0 % |
| 72 | 1,62 | 57,8 | 69,1 % |
| 80 | 0,65 | 25,9 | 55,2 % |
| 96 | 1,08 | 59,0 | 64,1 % |

**Aucune des treize n'est une réduction de la 256**, et l'écart se concentre là
où la règle du dessin travaille : le bord en porte de 33,6 % à 77,8 %, pour une
bande qui ne fait que quelques pourcents des pixels.

### Le compte du pas

| | prévu | obtenu |
|---|---|---|
| entrées lues par les trois chemins | 14 | **14** |
| Pillow | 0 pixel d'écart | **0, aux quatorze** |
| `LoadImageW` | 0 pixel d'écart, 256 comprise | **0, aux quatorze** |
| GDI+ | 13 justes, la 96 servie pour la 256 | **13 et 96** |
| écart à la réduction mécanique | non nul pour chacune des treize | **non nul pour les treize** |

### Ce qui a résisté

Rien. Les cinq comptes sont tenus.

**Sortie :** `captures/integration-icone/pas6-le-fichier.txt`, le fichier
`assets/veilleuse.ico`, et les vingt-huit images des deux relectures sous
`verif-loadimage/` et `verif-gdiplus/`.

---

## Pas 7 — `build.rs` et la ressource

**Ce que le pas demande.** *« `embed-resource` 3.0.11 ou `winresource` 0.1.31 —
jamais `winres`, abandonnée depuis cinq ans. Une ligne de `.rc` :
`1 ICON "veilleuse.ico"`. Identifiant 1, le plus bas. »*

**Ce qui a été écrit.** Trois fichiers, et rien de plus.

- `build.rs` — appelle `embed_resource::compile("assets/veilleuse.rc",
  embed_resource::NONE).manifest_optional()`, sous `#[cfg(windows)]`, avec ses
  deux `cargo:rerun-if-changed`. `manifest_optional` parce que l'icône est
  cosmétique : sur une machine sans compilateur de ressources, la compilation
  doit aboutir sans icône plutôt que d'échouer.
- `assets/veilleuse.rc` — une ligne, `1 ICON "veilleuse.ico"`, et le commentaire
  qui dit pourquoi l'identifiant est 1 : Windows prend la `RT_GROUP_ICON` dont
  l'identifiant vient en premier, et sous les deux règles — celle de NT et celle
  d'avant — la conclusion est la même.
- `Cargo.toml` — `embed-resource = "3.0"` sous
  `[target.'cfg(windows)'.build-dependencies]`. `winres` n'a pas été envisagée.

Le compilateur de ressources employé est le `rc.exe` du SDK Windows
10.0.26100.0, et **c'est relevé, pas supposé** :
`captures/integration-icone/releve-du-compilateur.txt` énumère les huit dossiers
du SDK installés et montre qu'**un seul** porte un `rc.exe` x64 — celui-là. Le
journal de `cargo -vv` montre le `.lib` que `embed-resource` produit, mais **ne
nomme pas l'outil qu'il a appelé**, et le relevé le dit : ce qui est établi,
c'est qu'il n'y a qu'un candidat sur la machine — ni `llvm-rc` ni `windres` n'y
sont — et non que celui-là a bien été appelé.

### L'épreuve, faite avant la mesure

Le binaire d'**avant** ne doit porter **aucune** ressource d'icône. Il a d'abord
été une copie compilée au même arbre sans `build.rs` ; l'instrument prend
désormais **`dist/Veilleuse.exe`**, qui date du 12 septembre 2026 et vit **dans
le dépôt**, de sorte que l'épreuve se rejoue sans rien qui dépende d'un dossier
de travail. Deux binaires plus anciens lui servent de repli. Le jour où `dist/`
sera rebâti avec l'icône, l'épreuve tombera bruyamment — et ce sera la bonne
réaction.

| | prévu | obtenu |
|---|---|---|
| types de ressources du binaire d'avant | aucun | **aucun** |
| `RT_GROUP_ICON` | 0 | **0** |
| `RT_ICON` | 0 | **0** |

L'épreuve tombe : l'instrument sait dire « il n'y en a pas ».

### La mesure

La lecture passe par `LoadLibraryExW` avec `LOAD_LIBRARY_AS_DATAFILE` — le
module est chargé comme une donnée, rien n'en est exécuté — puis
`EnumResourceTypesW` et `EnumResourceNamesW`. C'est la table de ressources du
fichier PE elle-même, et non une image rendue : ce chemin ne peut pas confondre
« l'icône s'affiche » avec « l'icône est dans le binaire ».

| | compte prévu | compte obtenu |
|---|---|---|
| `RT_GROUP_ICON` | une seule, identifiant 1 | **une seule, identifiant 1** |
| `RT_ICON` | quatorze | **quatorze**, identifiants 1 à 14 |

### Le `GRPICONDIR`, relu octet par octet

C'est le pendant d'`ICONDIR` dans l'exécutable : même structure, sauf que
`dwImageOffset` devient un `WORD nID` pointant la `RT_ICON` correspondante.
`réservé = 0`, `type = 1`, `compte = 14`.

| entrée | taille | plans | bits | octets | nID |
|---|---|---|---|---|---|
| 0 | 16 | 1 | 32 | 1 128 | 1 |
| 1 | 20 | 1 | 32 | 1 720 | 2 |
| 2 | 24 | 1 | 32 | 2 440 | 3 |
| 3 | 30 | 1 | 32 | 3 760 | 4 |
| 4 | 32 | 1 | 32 | 4 264 | 5 |
| 5 | 36 | 1 | 32 | 5 512 | 6 |
| 6 | 40 | 1 | 32 | 6 760 | 7 |
| 7 | 48 | 1 | 32 | 9 640 | 8 |
| 8 | 60 | 1 | 32 | 14 920 | 9 |
| 9 | 64 | 1 | 32 | 16 936 | 10 |
| 10 | 72 | 1 | 32 | 21 640 | 11 |
| 11 | 80 | 1 | 32 | 26 600 | 12 |
| 12 | 96 | 1 | 32 | 38 056 | 13 |
| 13 | 256 | 1 | 32 | 4 771 | 14 |

Les quatorze tailles, dans l'ordre, avec les mêmes longueurs qu'au pas 6, à
l'octet.

### Le second chemin — chaque `RT_ICON` relue par son contenu

Un compte d'entrées ne dit pas qu'elles portent la bonne image. Les quatorze
ressources ont donc été **extraites du binaire** et comparées, au pixel, aux
quatorze images du pas 2 : chaque DIB décodé depuis son `BITMAPINFOHEADER`, le
PNG depuis sa signature.

**0 pixel de différence, écart maximal 0, aux quatorze.** L'image qui est dans
l'exécutable est celle qui a été mesurée aux pas 2 à 5, et non une cousine.

### Ce qui a résisté

Une chose, et c'est une recette d'instrument, pas un fait sur l'icône :
`ENUMRESTYPEPROC` prend **trois** arguments et `ENUMRESNAMEPROC` en prend
**quatre**. Les confondre ne fait pas échouer l'énumération : la callback est
abandonnée sans bruit visible et la table revient **vide**. Un premier essai a
donc rapporté « aucune ressource » sur un binaire qui en portait quatorze — et
c'est précisément le genre de faux négatif qu'une épreuve de témoin ne rattrape
pas, puisque l'épreuve attendait justement zéro. Portée au pas 11.

**Sortie :** `captures/integration-icone/pas7-la-ressource.txt`.

---

## Pas 8 — Les deux icônes de fenêtre, depuis cette même ressource

**Ce que le pas demande.** *« `IconExtWindows::from_resource(1, None)` pour
`with_window_icon` (`ICON_SMALL`) et pour `with_taskbar_icon` (`ICON_BIG`).
Jamais `Icon::from_rgba`, qui ne porte qu'une taille et laisse Windows
redimensionner le reste. »* **Compte prévu :** *« `ICON_SMALL` rend 20 × 20 et
`ICON_BIG` 40 × 40 sur cette machine à 125 %, 0 pixel d'écart avec les entrées
20 et 40 du fichier. »*

### La lettre du §I et son compte se contredisent, et c'est le compte qui a raison

> ~~`IconExtWindows::from_resource(1, **None**)` pour les deux~~
> (`DECISION-ICONE.md` §I, pas 8).
>
> **Porté : `from_resource(1, Some(PhysicalSize::new(c, c)))`, où `c` est la
> taille que le système emploie de ce côté-là** — `SM_CXSMICON` pour
> `ICON_SMALL`, `SM_CXICON` pour `ICON_BIG`, lues chez Windows au lancement.
>
> **Ce qui a changé, et sa cause, mesurée.** `Icon::from_resource(ordinal,
> None)` passe `0, 0` et `LR_DEFAULTSIZE` à `LoadImageW` — lu ce jour dans
> `winit-0.30.13/src/platform_impl/windows/icon.rs`, avec le commentaire de la
> caisse elle-même : *« width / height of 0 along with LR_DEFAULTSIZE tells
> windows to load the default icon size »*. Or `LR_DEFAULTSIZE` ne connaît
> qu'**une** taille par défaut, celle de `SM_CXICON`, et il ignore
> complètement à quel côté de `WM_SETICON` l'icône est destinée. Mesuré ici :
> `LoadImageW(…, 0, 0, LR_DEFAULTSIZE)` sur `assets/veilleuse.ico` rend
> **40 × 40**, la grande taille, et non 20 × 20.
>
> Avec `None` des deux côtés, `ICON_SMALL` aurait donc porté une image de 40 px
> que Windows aurait réduite à 20 pour la barre de titre — **exactement la mise
> à l'échelle que le fichier à quatorze entrées existe pour éviter**, et le
> compte prévu par le §I lui-même serait tombé. La lettre change d'un mot ; le
> compte, lui, est tenu, et c'est le compte qui dit ce que la décision voulait.

**Témoin T1, déclaré avant le lancement :** `LR_DEFAULTSIZE` rend la grande
taille du système et non la petite. Réponse connue d'avance par la
documentation de `LoadImage` ; si elle avait été fausse, la déviation aurait été
injustifiée et il aurait fallu revenir à `None`. **Lu : (40, 40) contre
`SM_CXICON` = 40 et `SM_CXSMICON` = 20. Vrai.**

### Ce qui a été écrit

`src/systeme.rs` gagne `tailles_d_icone()`, qui lit `SM_CXSMICON` et `SM_CXICON`
et porte la raison ci-dessus en commentaire. `src/main.rs` charge les deux
icônes **une seule fois**, avant la création des fenêtres, et les pose sur
**les deux fenêtres**.

*Une interprétation, et je la dis.* Le §I écrit « les deux icônes de fenêtre »
et son épreuve dit « lire les `HICON` attachés **aux deux fenêtres** ». J'ai
donc posé les deux icônes sur l'horloge **et** sur l'îlot. La raison est dans le
§I lui-même, au pas 9 : les deux fenêtres portent `WS_EX_APPWINDOW`, donc l'îlot
paraît dans Alt+Tab. **Ce qu'il y montrerait sans cette ligne n'est pas mesuré** —
je ne l'ai pas ouvert, et c'est une déduction, pas un relevé : une fenêtre sans
`ICON_SMALL` ni `ICON_BIG` n'a rien à donner à Alt+Tab, et Windows y met ce
qu'il met. Ce qui **est** mesuré, c'est qu'avec la ligne l'îlot porte les
entrées 20 et 40 à l'octet. Cela ne change aucune grandeur du dossier et ne
remet en cause aucune décision ; l'îlot reste hors de la barre des tâches.

### Le témoin T2

Le processus est lancé depuis une copie hors du dépôt, en fenêtre de 640 × 360
pour ne pas couvrir l'écran de l'auteur. *Le mode fenêtré n'est pas moins réel
que le plein écran : c'est celui de `cargo run -- --fenetre`, et les `HICON`
d'une fenêtre ne dépendent pas de sa taille — le pas 10 les relit d'ailleurs
identiques sous cinq configurations.*

**Les fenêtres visibles du processus, toutes, sans tri préalable :**

| titre | classe | EXSTYLE | outil ? |
|---|---|---|---|
| *(vide)* | `Winit Thread Event Target` | 0x080800a0 | **oui** |
| Veilleuse | `Window Class` | 0x000c0130 | non |
| Veilleuse — ilot | `Window Class` | 0x00040118 | non |

*Les poignées de fenêtre ne sont pas reproduites : elles changent à chaque
lancement, et un nombre qui change à chaque lancement n'a pas sa place dans un
journal de mesures. L'EXSTYLE de l'horloge, lui, dépend de l'instant de la
lecture — voir le pas 9.*

La troisième est la fenêtre interne de `winit` — 14 × 14 à l'origine, porteuse
de `WS_EX_TOOLWINDOW`. Elle ne reçoit aucune icône et n'en doit recevoir
aucune : une fenêtre-outil n'est pas une fenêtre d'application. **Le tri se fait
sur le drapeau, pas sur le titre** — un tri sur le titre aurait marché ici et
aurait été faux.

**Les `HICON` attachés, lus sans aucun `DrawIconEx`** : on lit le bitmap de
couleur du `HICON` directement (`GetIconInfo` puis `GetDIBits`), de sorte
qu'aucune mise à l'échelle ne peut se glisser entre le `HICON` et ce qu'on
mesure.

| fenêtre | côté | taille du `HICON` | pixels différents | écart max |
|---|---|---|---|---|
| Veilleuse | `ICON_SMALL` | **20 × 20** | **0** | 0 |
| Veilleuse | `ICON_BIG` | **40 × 40** | **0** | 0 |
| Veilleuse — ilot | `ICON_SMALL` | **20 × 20** | **0** | 0 |
| Veilleuse — ilot | `ICON_BIG` | **40 × 40** | **0** | 0 |

**Quatre lectures sur quatre.** Les pixels sont ceux des entrées 20 et 40 du
fichier, à l'octet : Windows est allé chercher les entrées qu'on a dessinées, et
n'a redimensionné rien du tout.

**Épreuve de T2.** La même lecture sur une fenêtre qui n'a reçu aucune de nos
icônes — celle du Bureau, `Progman` — ne doit pas rendre notre image : **aucun
`HICON` des deux côtés**. L'instrument ne voit pas notre icône là où elle n'est
pas.

### Ce qui a résisté

La lettre du §I, et elle avait tort contre son propre compte. La mesure de T1
tranche : `LR_DEFAULTSIZE` ne sait pas qu'il existe une petite icône.

**Sortie :** `captures/integration-icone/pas8-les-icones-de-fenetre.txt`, et les
quatre `HICON` extraits sous `temoins/`.

---

## Pas 9 — Le bouton de barre des tâches de l'horloge, et l'identifiant

**Ce que le pas demande.** *« `with_skip_taskbar(true)` devient `false` pour
l'horloge seulement ; l'îlot le garde. Et l'`AppUserModelID` explicite, posé par
`SetCurrentProcessExplicitAppUserModelID` avant que l'application présente la
moindre interface. »*

**Ce qui a été écrit.** Dans `src/main.rs`, le drapeau de l'horloge passe à
`false`, avec la raison en commentaire et l'ancien appel barré ; celui de l'îlot
ne bouge pas. (Le §I prévenait que les deux appels étaient aux lignes 883 et 917
et qu'il fallait chercher le drapeau, pas le numéro : ils avaient encore bougé.)
Dans `src/systeme.rs`, `declarer_identifiant_application()` pose
`Stone.Veilleuse.Horloge.1` et le **relit chez Windows**. Elle est appelée dans
`main`, juste après la déclaration de conscience du DPI et **avant la boucle
d'événements**, donc avant toute fenêtre.

### Le témoin : le même relevé, trois fois

La barre des tâches est énumérée par automatisation d'interface, en lecture
seule — rien n'est ouvert, fermé, déplacé ni cliqué.

| moment | boutons | ceux qui nomment Veilleuse |
|---|---|---|
| avant le lancement | 23 | **aucun** |
| l'instance de mesure ouverte | **24** | **un seul** |
| l'instance fermée | 23 | aucun |

**Un bouton là où il n'y en avait aucun, et il repart avec le programme.**
L'îlot n'a pas de bouton à lui : il reste hors de la barre.

Le relevé est fait avec le programme lancé **en plein écran**, c'est-à-dire dans
la configuration réelle — c'est là que `MarkFullscreenWindow` et la barre des
tâches se croisent.

### Ce que le bouton porte comme nom — à rapporter, pas à changer

> « veilleuse.exe - 1 fenêtre en cours d'exécution »

**Le bouton porte le nom du fichier, pas le nom du programme.** La cause est
connue et hors de ce chantier : le binaire ne porte aucune ressource
`VERSIONINFO`, donc aucun `FileDescription`, et Windows retombe sur le nom du
fichier. **L'icône, elle, est la bonne** — c'est la ressource 1 que le pas 7 a
mesurée au pixel.

Ajouter un `VERSIONINFO` serait décider du nom du programme, de son éditeur et
de sa version. Ce n'est pas une question d'icône, et la décision est à l'auteur.
C'est mesuré, publié, et rien n'est changé.

### Les styles étendus — à rapporter, pas à changer

| fenêtre | EXSTYLE | `WS_EX_APPWINDOW` | `WS_EX_TOOLWINDOW` |
|---|---|---|---|
| Veilleuse | **0x000c0030** | **oui** | non |
| Veilleuse — ilot | 0x00040118 | **oui** | non |

**Ce chiffre a été corrigé deux fois, et la seconde correction annule la
première.** Le journal a d'abord publié `0x00040110` ; un juge a montré qu'il
n'était dans aucune sortie, et je l'ai corrigé en `0x000c0030` en écrivant que
c'était **une faute de copie**. ~~Faute de copie.~~ **Ce n'en était pas une :**
relancé après la correction, le pas 8 a rendu `0x00040110` à son tour. Le nombre
existait ; c'est mon explication qui était fausse.

**La vraie cause, mesurée.** L'EXSTYLE de l'horloge **n'est pas stable** :
l'application appelle `set_cursor_hittest(false)` après la création de la
fenêtre, ce qui lui pose `WS_EX_LAYERED` (`0x80000`) et `WS_EX_TRANSPARENT`
(`0x20`). Selon l'instant où on lit, on voit `0x00040110` (avant) ou
`0x000c0030` / `0x000c0130` (après, le dernier bit d'écart étant
`WS_EX_WINDOWEDGE`, que Windows retire à une fenêtre qui couvre l'écran). Le
pas 9 lit désormais **deux fois, à trois secondes d'intervalle**, et publie les
deux.

**Et rien de tout cela ne touche ce que ce pas interroge :** `WS_EX_APPWINDOW`
(`0x40000`) et `WS_EX_TOOLWINDOW` (`0x80`) sont les mêmes dans toutes les
lectures, avant comme après.

Les deux fenêtres portent `WS_EX_APPWINDOW`. **Alt+Tab présentera donc
vraisemblablement deux entrées pour un seul programme.** Le §I demande de
mesurer, publier, et laisser la décision à l'auteur : ce n'est pas une question
d'icône. Rien n'est changé.

### L'identifiant de modèle d'application, par trois chemins

**1. Ce que Windows en rend au processus lui-même.**
`declarer_identifiant_application()` n'annonce pas la constante qu'elle a
posée : elle appelle `GetCurrentProcessExplicitAppUserModelID` **avant** de
poser, puis **après**, et n'annonce que ce que Windows lui rend. La ligne porte
donc sa propre épreuve.

> `identifiant d'application : Stone.Veilleuse.Horloge.1 (avant l'appel : aucun)`

**L'épreuve de ce chemin, sur un processus étranger et neuf** — un PowerShell
qui appelle `Get…` avant tout `Set…`, pose un identifiant à lui, et rappelle
`Get…` :

    AVANT : aucun
    POSE  : 0x00000000
    APRES : Epreuve.Identifiant.Neuf.1

La relecture tombe bien avant l'appel : elle mesure quelque chose.

**2. Ce que le magasin de propriétés de chaque fenêtre en dit.**
`SHGetPropertyStoreForWindow` + `PKEY_AppUserModel_ID` rend **(aucun)** sur les
deux fenêtres — **et c'est la bonne réponse, même si elle surprend.** Le magasin
d'une fenêtre ne porte que les *remplacements* posés sur cette fenêtre-là ;
l'identifiant posé est celui du **processus**, et le shell y retombe quand la
fenêtre n'a rien. Aucun remplacement n'est posé, donc le magasin est vide — et
il **doit** l'être : deux fenêtres du même programme qui porteraient des
identifiants différents feraient deux boutons de barre des tâches.

**3. L'effet observable.** Un seul bouton pour deux fenêtres d'application.
C'est l'identifiant qui groupe les fenêtres en un bouton, et le relevé en compte
un.

**La forme imposée par Microsoft**, vérifiée point par point :
`Stone.Veilleuse.Horloge.1`, 25 caractères sur 128 ; quatre sections ; aucun
espace ; chaque section en PascalCase.

### Le compte du pas

| | prévu | obtenu |
|---|---|---|
| bouton de l'horloge | un, là où il n'y en avait aucun | **un** (23 → 24 → 23) |
| bouton de l'îlot | aucun | **aucun** |
| identifiant | posé avant toute interface, forme respectée | **relu chez Windows, et nul avant l'appel** |
| `WS_EX_APPWINDOW` sur les deux | à rapporter | **rapporté, rien changé** |

### Ce qui a résisté

Le deuxième chemin de l'identifiant, d'abord — il rendait « aucun » et j'ai cru
à un échec. La lecture du contrat de `SHGetPropertyStoreForWindow` a montré que
c'était la réponse juste, et qu'il fallait un troisième chemin : la relecture
chez Windows depuis le processus, avec son épreuve. Une mesure qui ne peut rendre
qu'une seule valeur n'est pas une mesure ; celle-ci en rend deux, et on a vu les
deux.

Et un fait mesuré qui déborde ce chantier : **le bouton dit « veilleuse.exe »**.
Publié, non corrigé.

**Sortie :** `captures/integration-icone/pas9-la-barre-des-taches.txt`.

---

## Pas 10 — Les quatre réglages du système (C10), tranchés et vérifiés

**Ce que le pas demande.** Le §I tranche les quatre, et son épreuve est
*« forcer chaque réglage et relire l'icône affichée »*. **Compte prévu :**
*« l'icône identique au pixel sous les trois premiers réglages ; sous le thème
sombre, les chiffres du pas 3. »*

**Comment « l'icône affichée » est lue.** Sur les `HICON` réellement attachés
aux fenêtres, par `GetIconInfo` puis `GetDIBits` — le même chemin qu'au pas 8,
celui où aucun `DrawIconEx` ne s'interpose. C'est ce que Windows tient pour
l'icône de cette fenêtre à cet instant : s'il en substituait une autre, on le
verrait.

**L'épreuve de l'instrument, faite avant les mesures.** La même comparaison
entre deux entrées qu'on sait différentes — la 30 du fichier ramenée à 20 px,
contre l'entrée 20 — doit rendre un écart non nul : **159 pixels différents,
écart maximal 71**. Sans elle, « 0 pixel d'écart » ne voudrait rien dire.

### Les cinq forçages

Un lancement par réglage, quatre lectures chacun (deux fenêtres × deux côtés).

| réglage forcé | lectures | pixels différents |
|---|---|---|
| transparence coupée, `--transparence=coupee` | 4 | **0** |
| économiseur de batterie, `--batterie=economie` | 4 | **0** |
| thème de contraste, `--contraste=on` | 4 | **0** |
| mouvement réduit, `--mouvement-reduit` | 4 | **0** |
| galet opaque, `--matiere=opaque` | 4 | **0** |

**Vingt lectures justes sur vingt.**

> ~~L'icône est une ressource figée à la compilation : aucun de ces réglages ne
> peut l'atteindre, et c'est maintenant **mesuré et non déduit**.~~
>
> **C'est faux, et c'est l'inverse.** Un juge qui n'a pas fait le travail l'a
> trouvé, et il a raison : ces cinq drapeaux sont les **forçages internes de
> l'application**, pas les réglages du système. `src/options.rs` le dit
> lui-même — ils existent pour exercer les branches de repli *« sans changer les
> réglages de la machine, qui ne sont pas les nôtres »*. Or un `HICON` est
> construit par Windows depuis une ressource compilée : **aucun drapeau interne
> ne pouvait l'atteindre**, et les vingt lectures à zéro pixel étaient acquises
> avant le lancement.
>
> **Ce que ces vingt lectures établissent vraiment, et c'est petit :** les
> branches de repli de l'application ne touchent pas ses icônes de fenêtre. Elles
> n'établissent rien de ce que Windows fait.
>
> **Le pas 10 bis force un vrai réglage**, avec un témoin qui montre que la
> condition a vraiment varié. Ce qui reste non mesuré après lui est nommé là-bas,
> et porté à « Ce qui pourrait être faux ».

*Contraste élevé* : ~~les quatre lectures sous `--contraste=on` le confirment~~.
Elles ne confirment rien : **aucun thème de contraste réel n'a été activé**, et
c'est précisément le seul des quatre réglages où Windows pourrait substituer une
ressource. Que Windows 11 n'exige plus de ressource d'icône dédiée est la
position de `DECISION-ICONE.md` §I, prise sur la documentation ; **ce chantier ne
l'a pas vérifiée**.

*Le réglage de matière de l'îlot* : la charte dit *« quand la transparence est
coupée, le galet de l'application redevient opaque ; l'icône, elle, reste du
verre »*. C'était une affirmation ; elle est désormais mesurée — sous
`--matiere=opaque`, les quatre `HICON` sont identiques au pixel à ceux du verre.

### Le thème clair et le thème sombre

Le thème est basculé **pendant qu'une instance tourne**, et ses `HICON` sont
relus **sans la relancer** : c'est la seule façon de mesurer que le système ne
substitue rien en cours de route. Le réglage est sauvegardé avant, restauré
après, et la restauration est vérifiée — comme au pas 1.

| moment | la barre lit | l'icône, quatre lectures |
|---|---|---|
| thème clair, avant | `#D9D9D9` | **0 pixel différent** |
| thème **sombre** | `#212121` | **0 pixel différent** |
| thème clair, après | `#D9D9D9` | **0 pixel différent** |

Le registre relu après restauration : `SystemUsesLightTheme = 1`, identique.

**L'icône ne change pas d'un thème à l'autre.** Ce qui change, c'est le **fond**
sur lequel elle se pose, et c'est C5 : le pas 3 en donne les chiffres — 100 %
du côté clair, 71,9 à 75,6 % du côté sombre, contre 50 % demandés.

### Le facteur d'échelle

C'est C9, et le pas 6 l'a vérifié case par case : les quatorze entrées sont
exactement l'union de la table *Icon scaling*, aux sept échelles et sur les
trois surfaces, et `LoadImageW` sert la taille demandée aux quatorze avec zéro
pixel d'écart. Rappelé ici, pas refait.

### Le compte du pas

| | prévu | obtenu | ce que cela vaut |
|---|---|---|---|
| l'icône sous les forçages **de l'application** | identique au pixel | **20 lectures sur 20 à 0 pixel** | acquis d'avance : ces drapeaux ne peuvent pas atteindre un `HICON` |
| sous le thème **du système** | les chiffres du pas 3 | **icône inchangée ; le fond passe de `#D9D9D9` à `#212121`, et C5 tient à 71,9 % au pire** | **mesuré** : la condition a vraiment varié |
| l'épreuve | doit tomber | **159 pixels d'écart, elle tombe** | elle borne le comparateur, pas la condition |
| le registre | restauré | **identique** | vérifié |

### Ce qui a résisté

**Le pas lui-même, relu par un juge.** Il croyait mesurer et il déduisait, sur
quatre réglages sur cinq. Le seul qui était vraiment mesuré est le thème — parce
que c'est le seul dont j'ai varié la condition dans le système. Le pas 10 bis
répare la moitié de ce qui manquait ; l'autre moitié est écrite comme non
mesurée.

**Sortie :** `captures/integration-icone/pas10-les-reglages.txt`.

---

## Pas 10 bis — Un vrai réglage du système, forcé pour de bon

**Pourquoi ce pas existe.** Le juge a montré que le pas 10 forçait les drapeaux
internes de l'application et non les réglages de Windows, et que les vingt
lectures à zéro pixel étaient acquises avant le lancement. Ce pas force un vrai
réglage, et un seul : **la transparence des effets de Windows**,
`EnableTransparency`, dans la même clé que le thème — celle dont le pas 1 a déjà
prouvé la sauvegarde, la restauration et la vérification.

### Le témoin qui manquait au pas 10 : la condition a-t-elle varié ?

La barre des tâches est ce que ce réglage repeint. Elle est donc capturée avant
et après, et **elle doit changer** — sinon le pas ne mesure rien.

| | prévu | obtenu |
|---|---|---|
| la barre, transparence coupée contre transparence active | doit changer | **plus de 100 000 pixels sur 115 200** — 104 941 au premier relevé, 105 039 au dernier |

**La condition a vraiment varié** — plus de neuf dixièmes de la surface de la
barre. C'est exactement ce qui manquait au pas 10.

*Le compte de pixels est stable d'un lancement à l'autre ; l'écart maximal, lui,
ne l'est pas — il dépend de ce qui se trouve derrière la barre au moment de la
capture, et il a été lu de 23 à 215 selon les essais. C'est le compte qui sert de
témoin, pas l'amplitude.*

### L'icône, sous le réglage coupé

| | ICON_SMALL | ICON_BIG | pixels différents |
|---|---|---|---|
| transparence active, état de départ | 20 × 20 | 40 × 40 | **0**, sur les deux fenêtres |
| transparence coupée, **instance déjà lancée** | 20 × 20 | 40 × 40 | **0**, sur les deux fenêtres |
| transparence coupée, **instance née sous le réglage** | 20 × 20 | 40 × 40 | **0**, sur les deux fenêtres |

**Douze lectures sur douze.** La seconde ligne montre que Windows ne remplace pas
l'icône d'une fenêtre existante ; la troisième, qu'un processus qui *naît* sous
le réglage coupé charge la même entrée — c'est celle-là qui compte, puisque c'est
au chargement qu'une substitution se ferait.

### La restauration

| | prévu | obtenu |
|---|---|---|
| le registre | `EnableTransparency` = 1 | **1, identique** |
| la dominante de la barre | `#D9D9D9` | **`#D9D9D9`, avant comme après** |
| les cinq rangées | identiques | **identiques** |

*Un détail qui a coûté un essai, et c'est une recette :* la barre porte
**l'horloge de Windows**. Deux captures prises à une minute d'intervalle diffèrent
sur les chiffres de l'heure — 63 pixels, écart 195, tous dans la zone de
notification — et une égalité pixel à pixel rendait un faux négatif. Le témoin de
restauration est donc celui du pas 1 : la dominante et les cinq rangées.

### Ce qui reste non mesuré, et il faut le nommer

- **Le contraste élevé.** C'est le seul des quatre réglages où Windows pourrait
  substituer une ressource, et **il n'est pas mesuré**. Le forcer par
  `SPI_SETHIGHCONTRAST` repeint tout le bureau de l'auteur, et le projet a pour
  doctrine de ne pas toucher à ses réglages — `src/options.rs` : *« les réglages
  de la machine, qui ne sont pas les nôtres »*. `DECISION-ICONE.md` §I le tranche
  sur la documentation ; ce chantier ne l'a pas vérifié. Porté à « Ce qui
  pourrait être faux ».
- **L'économiseur de batterie.** Il dépend de l'état d'alimentation, qu'aucune
  interface publique ne pose. Non mesurable ici.

### Ce qui a résisté

Rien, une fois le témoin de la condition posé. Ce pas n'existe que parce qu'un
juge a lu ce que le pas 10 affirmait et l'a confronté à ce qu'il avait fait.

**Sortie :** `captures/integration-icone/pas10bis-un-vrai-reglage.txt`, et les
trois captures de la barre.

---

## Pas 11 — Les documents

**Ce que le pas demande.** Porter le §H dans `DESIGN.md` **mot pour mot** ;
barrer « 106 à 123 » dans `ICONE-FONDATIONS.md` §3.1 et §9.3 et y porter 106 à
122 avec sa cause ; verser à `RECETTES.md` chaque technique éprouvée en route, y
compris les deux du partisan de la forme qu'il n'avait pas pu porter ; et écrire
la ligne de l'icône dans `LISEZ-MOI.md`.

### `DESIGN.md`

Les **cinq** textes du §H sont portés à leur place, en **six** modifications —
la ligne de palette après « Pli », le remplacement de la phrase « Aucune autre
valeur… », la ligne des contours du §8, la phrase de plus après « Pas d'icône ;
les actions sont des mots. », la phrase de plus au §6 « Son rôle », et le
paragraphe neuf à la fin du §8.

*Une seule forme diffère, et le texte est intact :* le §H écrit la phrase de
typographie comme « une ligne de plus » après la puce « Pas d'icône… » ; elle est
portée **dans** cette puce, parce que le §8 est une liste et qu'une ligne de plus
y serait une puce de plus, sans sujet.

**Et la règle de ce chantier s'y applique quatre fois : là où un texte porte un
chiffre qu'une mesure postérieure a démenti, le texte est porté, le chiffre est
barré, et ce qui l'a remplacé est écrit avec sa cause.**

*Une précision que le tableau ci-dessous doit porter :* trois des quatre barrés
corrigent un chiffre **du §H**. Le quatrième, `~~from_resource(1, None)~~`, ne
vient pas du §H — **le §H ne contient pas cette phrase**. Elle vient du §I,
pas 8, et elle a été ajoutée au paragraphe neuf de la charte parce que c'est là
qu'elle vit désormais. Le barré est donc celui d'une consigne, non d'un texte de
charte, et il est signalé comme tel.

| texte du §H | chiffre barré | ce qui l'a remplacé | cause |
|---|---|---|---|
| §2, palette | ~~3,758:1 contre un fond sombre **supposé** `#202020`~~ | **3,714:1** contre la barre sombre **mesurée** `#212121` | pas 1 : la barre sombre réelle est d'un niveau plus claire |
| §2, palette | ~~3,087:1 du galet contre le sol~~ | **3,105:1** | pas 2 : lu sur le rendu à 3/4 au lieu d'être prédit par un modèle linéaire |
| §8, paragraphe neuf *(la phrase vient du §I pas 8, pas du §H — voir ci-dessous)* | ~~`from_resource(1, None)`~~ | **`from_resource(1, Some(taille))`**, la taille exacte du système | pas 8 : `LR_DEFAULTSIZE` ne connaît qu'une taille par défaut, la grande, mesurée à 40 × 40 |
| §8, paragraphe neuf | ~~84 ou 85 aux dix tailles~~ | **85,0 aux quatorze tailles** | pas 5 : le troisième geste pose le bord à la valeur de la loi, sans dispersion |

Deux chiffres mesurés sont ajoutés au paragraphe neuf, parce qu'ils disent ce
que la décision achetait et que personne ne les avait : **le tour porte 80,0 %
de ce qui distingue l'icône d'un cadre à 30 px**, et l'icône reste du verre sous
les cinq forçages, **vingt lectures à zéro pixel**.

### `ICONE-FONDATIONS.md`

Trois corrections, toutes barrées, rien d'effacé. Une seule porte une date
dans son texte — celle de la barre sombre, mesurée le 18 septembre 2026 ; les
deux autres renvoient au document qui les établit.

1. §3.1 et §9.3 : ~~106 à 123~~ → **106 à 122**, avec sa cause — le niveau 123
   lit 2,9986:1 contre la barre claire mesurée, et le chiffre publié avait
   arrondi la borne réelle (122,97) au lieu de garder le dernier niveau qui
   passe.
2. §3.1 : ~~le fond sombre `#202020`, **supposé, non mesuré**~~ → **`#212121`,
   niveau 33,00, mesuré**. La réserve *« il aurait fallu changer le réglage de
   la machine de l'auteur »* est levée : le réglage a été changé, lu, et remis à
   l'identique.
3. §6 : ~~« le 30 px de la barre des tâches […] ne sert à rien »~~ → **caduc**.
   L'horloge a repris son bouton, et le pas 9 l'a mesuré.

### `RECETTES.md`

Une section neuve, **L'icône de l'application : le fichier, la ressource, et ce
que Windows en fait**, avec **vingt-deux entrées**. Les deux premières sont celles
du partisan de la forme, qu'il n'avait pas pu verser lui-même :

- la barre des tâches de Windows 11 ne se capture pas depuis le contexte de
  l'écran — `BitBlt` y rend du noir, et un témoin d'échelle ne l'attrape pas ;
- `PrivateExtractIconsW` extrait à la taille exacte, et ses `argtypes` doivent
  être des `c_void_p`.

**Les vingt autres sont de ce chantier, et chacune a coûté un essai** : le sol
qui doit déborder le carré au-delà de `p = 5/7` ; la marge du nuanceur qui
s'arrondit au-dessus ; `ENUMRESTYPEPROC` à trois arguments contre
`ENUMRESNAMEPROC` à quatre, dont la confusion rend une table **vide** au lieu
d'une erreur ; `LR_DEFAULTSIZE` qui ne connaît qu'une taille ; un `HICON` qui se
lit sans `DrawIconEx` ; GDI+ qui ne lit pas le PNG d'un `.ico` ; le magasin de
propriétés d'une fenêtre qui ne porte pas l'identifiant du processus ; le bouton
de barre des tâches qui porte le nom du fichier sans `VERSIONINFO` ; le carré
qu'il ne faut pas retourner à la place du galet ; la silhouette binaire qui ne
sait pas voir une lampe ; la part d'aire prédite à une seule taille ; la bascule
du thème et sa restauration ; un raccourci qui ne se lit pas par son chemin mais
par son PIDL ; la barre des tâches de Windows 11 qui ne dessine pas les icônes
de fenêtre et met à l'échelle ; et la console en cp1252. **Cinq de plus sont
venues de la relecture du juge** : un drapeau interne d'application ne force pas
un réglage du système ; deux captures d'une barre des tâches ne sont jamais
égales au pixel, puisqu'elle porte une horloge ; un `WS_EX_*` de fenêtre n'est
pas stable et se lit deux fois ; un instrument ne doit pas pointer sur un dossier
de travail ; et un champ ajouté à un instrument partagé se recopie chez tous ses
lecteurs.

### `LISEZ-MOI.md`

Une section **L'icône**, entre les blocs et le verre : ce qu'elle est, ce qu'elle
montre, les deux commandes qui la refont après un changement du nuanceur du
galet, et l'avertissement sur GDI+.

### Ce qui a résisté

Rien. Mais il faut dire ce que ce pas **n'a pas** fait : `DESIGN.md` est un
document que d'autres chantiers touchent aussi, et les ajouts ont été faits aux
seuls endroits que le §H nomme, sans toucher à une ligne voisine.

---

## Pas 12 — L'épreuve du mot (C11), et elle est à l'auteur seul

**Ce que le pas demande.** *« Lui montrer l'icône de 30 px, seule, à 1:1, sur le
gris mesuré de sa barre, sans jamais l'agrandir, et noter le premier mot.
Épreuve : aucune. C'est la seule cible du dossier qui n'ait pas d'instrument, et
il ne faut pas lui en fabriquer un. »*

Je ne l'ai donc pas passée, et je n'ai fabriqué aucun instrument. J'ai préparé
ce qu'il faut montrer, et je me suis arrêté là.

**Deux images, et la seconde n'était pas prévue.**

- `captures/integration-icone/pas12-le-mot-30px.png` — **la planche prescrite** :
  l'icône de 30 px, seule, à 1:1, sur `#D9D9D9`, le gris mesuré de la barre de
  l'auteur, relu ce jour au pas 1. L'image fait 240 × 90 px ; l'icône y occupe
  30 × 30 px, ni plus ni moins.
- `captures/integration-icone/pas12-le-mot-dans-la-barre.png` — **la même chose,
  mais vraie** : l'icône telle qu'elle est réellement affichée dans la barre des
  tâches vivante, découpée au rectangle du bouton (55 × 60 px physiques), sans
  une mise à l'échelle. `pas12-barre-entiere.png` porte la barre entière, 1920 ×
  60 px, pour le contexte.

Le §G, point 9, écrivait : *« Je n'ai pas vu l'icône sur l'écran de l'auteur.
Toutes mes planches sont composées sur les octets à partir d'images de fichier.
Une icône de 30 px dans une vraie barre des tâches vivante n'a été vue par
personne. »* Elle l'est maintenant — et c'est cette découpe-là qu'il faut
montrer, pour une raison que le chapitre suivant mesure.

**L'ensemble déclaré**, et il n'est pas de moi : **verre, pierre, galet, nuit**.
~~horloge~~ est barré (§B.5). Si le mot tombe hors de l'ensemble, **le dessin est
faux**, quelles que soient les onze autres cibles — et il faudra rouvrir, pas
rattraper.

---

## En plus des douze pas — ce que la barre des tâches montre vraiment

Ce chapitre n'est pas un pas du §I. Il existe parce que la consigne demande
*« que l'icône paraît bien dans la barre des tâches, dans l'explorateur et sur un
raccourci »*, et parce que la planche du pas 12 a montré quelque chose que
personne ne pouvait voir avant elle.

### Ce que la découpe de la barre vivante a montré

L'icône découpée dans la barre **n'est pas identique à l'entrée 30 du fichier**.
Le sol y est juste à l'octet — (124, 121, 116) —, le cœur aussi — (47, 47, 45) —,
mais le **chant** du galet y lit **53** là où l'entrée 30 porte **85**. C'est la
signature d'une réduction : un aplat survit, un trait d'un pixel se moyenne.

### L'expérience, et elle est décisive

On ne conclut pas d'une ressemblance. On **change** une des icônes de la fenêtre,
de l'extérieur, par `WM_SETICON`, et on regarde si le bouton change.

| ce qu'on change | le changement a-t-il pris ? | le bouton change-t-il ? |
|---|---|---|
| `ICON_BIG` ← l'entrée 16 | **oui** — la fenêtre porte désormais un `HICON` de 16 × 16 | **non : 0 pixel** |
| `ICON_SMALL` ← l'entrée 16 | **oui** — idem | **non : 0 pixel** |

**Témoin** : la capture de contrôle, après restauration, redonne celle du départ
**à l'octet, 0 pixel** — sans quoi la barre bougerait toute seule et rien de ce
qui précède ne voudrait dire quoi que ce soit.

Le contrôle du « le changement a-t-il pris ? » est ce qui fait la différence
entre une expérience et une impression : sans lui, « 0 pixel » aurait pu
signifier « mon `SendMessage` n'a rien fait ».

### Ce dont le bouton est le plus proche

**Et d'abord ce que cette table suppose, parce qu'elle le suppose.** Elle glisse
des candidats de **30 × 30** sur une découpe de 55 × 60 et retient le meilleur
décalage. **La taille que la barre dessine réellement n'est pas mesurée** : 30 px
vient de la table *Icon scaling* de Microsoft — précisément la table dont ce
chapitre établit que la barre ne la suit pas. Si elle dessinait à 28 ou à 32, les
nombres ci-dessous changeraient. Ce qu'ils établissent malgré tout, et qui ne
dépend pas de la taille supposée : **aucun candidat ne tombe à zéro**, alors que
l'entrée exacte, servie par `LoadImageW`, y tombe (pas 6).

| candidat, ramené à 30 px | pixels différents sur 900 | écart max |
|---|---|---|
| l'entrée 30, telle quelle | 238 | 42 |
| l'entrée 32, moyenne d'aire | 207 | 29 |
| l'entrée 40, bilinéaire | **199** | **17** |
| l'icône que le **shell** donne pour le fichier, 40 × 40, bilinéaire | **199** | **17** |
| l'entrée 48, moyenne d'aire | 249 | 36 |

### Ce qui se conclut, et rien de plus

> **La barre des tâches de Windows 11 n'est pas un bouton Win32 : c'est une
> interface XAML qui prend l'icône du FICHIER par le cache du shell et la pose à
> la taille de son bouton. Elle met donc à l'échelle, et l'entrée 30 du fichier
> n'est pas celle qu'on voit dans la barre.**
>
> **Ce qui tombe est une phrase, pas un dessin.** *« Aucune taille n'est laissée
> à la mise à l'échelle de Windows »* (§H, paragraphe neuf) vaut pour le fichier
> et pour les fenêtres — et **non** pour le bouton de la barre des tâches de
> Windows 11. La phrase reste vraie de ce qu'elle nomme : les quatorze entrées
> sont l'union exacte de la table *Icon scaling*, et `LoadImageW` les sert
> toutes à zéro pixel d'écart. Ce que la mesure ajoute, c'est que **la table
> d'une page de Microsoft ne décrit pas le chemin que la barre de Windows 11
> emprunte réellement**.
>
> Ce n'est pas mesuré : si le 30 px du fichier sert quand même — à une autre
> surface, à une autre version, à un autre shell. Il est légal, il est servi par
> `LoadImageW`, et il ne coûte que 3 760 octets. Il reste.

### L'explorateur et le raccourci

`SHGetFileInfoW` est le chemin du shell — celui de l'explorateur, du bureau, de
la recherche et d'« Ouvrir avec ».

| source | taille | pixels différents | où ils tombent |
|---|---|---|---|
| l'exécutable | 40 × 40 | **0** | nulle part |
| l'exécutable | 20 × 20 | **0** | nulle part |
| le raccourci `.lnk` | 40 × 40 | 289 | (0, 23) sur 17 × 17 |
| le raccourci `.lnk` | 20 × 20 | 72 | (0, 11) sur 8 × 9 |

**L'exécutable rend l'entrée du fichier à l'octet, aux deux tailles.** Le
raccourci la rend aussi, plus la **superposition** que Windows ajoute à tout
raccourci : les pixels qui diffèrent sont tous dans le coin inférieur gauche, sur
au plus la moitié du côté. C'est la flèche du système, elle n'est pas de nous, et
elle ne se retire pas ; hors de ce coin, l'icône est la nôtre au pixel.

**Témoin.** `notepad.exe` doit rendre une icône différente : 1 600 pixels sur
1 600 à 40 px, 400 sur 400 à 20 px. L'instrument tombe.

**Et une porte à ne pas reprendre.** `SHGetFileInfoW` appelé sur un **chemin** a
rendu, pour un `.lnk` neuf sur un cache froid, l'icône générique du type de
fichier — une page blanche à flèche, 1 600 pixels de différence — puis, une fois
le cache chaud, l'icône résolue. Le chemin n'est pas faux : **il est instable**,
et une mesure qui dépend de l'état d'un cache n'est pas une mesure. Il faut
passer par le PIDL de l'élément (`SHParseDisplayName` puis `SHGFI_PIDL`), qui
rend la même chose froid ou chaud.

### Ce que cela change pour le pas 12

C'est la **découpe de la barre vivante** qu'il faut montrer à l'auteur, et pas
seulement l'entrée 30 sur son gris — puisque ce n'est pas l'entrée 30 qu'il voit.
Les deux images sont produites, et la vraie est nommée.

**Sorties :** `captures/integration-icone/pas-en-plus-quelle-entree-la-barre-montre.txt`
et `pas-en-plus-explorateur-et-raccourci.txt`.

---

## Le compte des douze pas

| pas | ce qu'il prouvait | verdict |
|---|---|---|
| 1 | la barre sombre, mesurée au lieu d'être supposée | **tenu** — `#212121`, niveau 33, seize niveaux sous le seuil de 49 |
| 2 | le sol `#7C7974` et la part 3/4, rendus | **tenu** — les cinq comptes ; un obstacle trouvé et résolu |
| 2 bis | le débord du sol, aux quatorze | **tenu** — et l'épreuve, en ne tombant qu'à deux tailles, a dit où le nuanceur lit vraiment |
| 3 | C5 et C7 aux quatorze tailles | **tenu** — C5 atteinte partout ; deux comptes prévus barrés avec leur cause |
| 4 | C12, la matière se lit | **tenu** — matière aux quatorze tailles ; le tour porte 80,0 % à 30 px |
| 5 | C1, C4, C6, C8 | **tenu** — trois atteintes, C8 ratée comme écrit d'avance ; un compte barré |
| 6 | le fichier, quatorze entrées | **tenu** — les cinq comptes |
| 7 | `build.rs` et la ressource | **tenu** — une `RT_GROUP_ICON` d'identifiant 1, quatorze `RT_ICON`, identiques aux images |
| 8 | les deux icônes de fenêtre | **tenu** — 20 × 20 et 40 × 40, zéro pixel ; la lettre du §I corrigée contre son propre compte |
| 9 | le bouton de barre des tâches, et l'identifiant | **tenu** — 23 → 24 → 23 ; deux faits rapportés et non changés |
| 10 | les quatre réglages du système | **partiel** — le thème est mesuré, les quatre autres « forçages » étaient ceux de l'application et ne prouvaient rien |
| 10 bis | un vrai réglage forcé, avec le témoin de la condition | **tenu** — la barre change sur plus de 100 000 pixels, l'icône ne bouge pas, douze lectures sur douze ; le contraste élevé reste non mesuré |
| 11 | les documents | **tenu** — quatre documents portés, douze chiffres barrés |
| 12 | l'épreuve du mot | **préparé, non passé** — elle est à l'auteur seul, et il ne fallait pas lui fabriquer d'instrument |

**Onze pas sur douze sont faits et prouvés — le dixième seulement après qu'un
juge l'eut renvoyé à sa copie. Le douzième ne m'appartient pas.**

### Les douze chiffres barrés, et ce qui les a remplacés

Aucun n'a été effacé. Chacun porte sa cause.

| barré | remplacé par | d'où vient la correction |
|---|---|---|
| ~~fond sombre `#202020`, supposé~~ | `#212121`, niveau 33,00, **mesuré** | pas 1 |
| ~~3,758:1 du sol contre le sombre~~ | **3,714:1** | pas 1 |
| ~~3,087:1, SC 1.4.11, prédit~~ | **3,105:1**, lu sur le rendu | pas 2 |
| ~~marge du nuanceur 38 px à 256~~ | **39 px** — le nuanceur arrondit au-dessus | pas 2 |
| ~~106 à 123~~ | **106 à 122** — le 123 lit 2,9986:1 | §E de la décision, porté au pas 11 |
| ~~C5 sombre ≥ 74 %~~ | **71,9 % à 75,6 %** selon la taille | pas 3 |
| ~~C7 clair 3,74 ± 0,05~~ | **3,659 à 3,746** | pas 3 |
| ~~C1, aire ≥ 95 px (prédite 103)~~ | **90 px** — la silhouette ne compte pas le chant éclairé | pas 5 |
| ~~84 ou 85 aux dix tailles~~ | **85,0 aux quatorze** | pas 5 |
| ~~`from_resource(1, None)`~~ | **la taille exacte du système** — `LR_DEFAULTSIZE` n'en connaît qu'une | pas 8 |
| ~~« le 30 px de la barre des tâches ne sert à rien »~~ | **caduc** — l'horloge a repris son bouton | pas 9 |
| ~~« aucune taille n'est laissée à la mise à l'échelle de Windows »~~ | **vrai du fichier et des fenêtres, faux du bouton de la barre de Windows 11** | la mesure en plus |

---

## La machine, et ce que j'y ai laissé

**Ce que j'ai touché, et comment je l'ai rendu.**

| | ce que j'ai fait | l'état final, vérifié |
|---|---|---|
| `HKCU\…\Themes\Personalize\SystemUsesLightTheme` | basculé à 0 puis restauré, deux fois (pas 1 et pas 10) | relu à **1**, identique ; la barre claire redonne `#D9D9D9` et les cinq rangées leurs 217/218 |
| `%LOCALAPPDATA%\Veilleuse\ilot.txt` | sauvegardé avant tout, restauré | **`520 400`**, identique à la sauvegarde, comparé octet par octet |
| `%LOCALAPPDATA%\Veilleuse\matiere.txt` | idem | **`verre`**, identique |
| l'horloge de l'auteur | trouvée en cours d'exécution depuis `veilleuse-bin\676bd7fb\`, matière verre, îlot à (520, 400) | **relancée depuis ce même binaire**, après la restauration des deux fichiers ; l'îlot retrouve (520, 400) et la matière est le verre |
| un raccourci `.lnk` de mesure | créé dans un dossier temporaire | **effacé**, vérifié |
| les copies de binaire hors du dépôt | une par lancement, créées en route | **effacées**, il n'en reste qu'une : `veilleuse-bin\icone-livree\`, la copie qui porte l'icône. Son nom ne porte pas d'empreinte : une copie nommée d'après un commit cesse d'être à jour au commit suivant, et le journal se mettait à courir après son propre nom |

**Deux choses que je dois dire sans les adoucir.**

1. **L'horloge de l'auteur ne tournait plus quand je suis arrivé au pas 9.** Elle
   tournait au début (relevé au lancement de ce chantier : processus 37092,
   `veilleuse-bin\676bd7fb\`, matière verre). Je ne peux pas établir qui l'a
   fermée : mes propres lancements ne terminent que leur propre processus, par
   son identifiant, et plusieurs autres chantiers lancent et ferment le même
   programme en ce moment. Je l'ai relancée à la fin, depuis le même binaire,
   dans la même matière et à la même place — ce que la consigne demande.
2. **`ilot.txt` a changé sous ma main entre ma sauvegarde et ma restauration**,
   et pas de mon fait : il est passé de `520 400` seul à `520 400` plus une
   ligne par écran, `\\.\DISPLAY1 1920x1080 689.60 24.80`, qui posait l'îlot à
   (862, 31) physiques — calculé, pas mesuré : `position_memorisee_pour`
   multiplie la position logique par l'échelle de l'écran, 689,60 × 1,25 et
   24,80 × 1,25 —, et non à (520, 400) où l'auteur l'avait laissé. J'ai restauré
   **ce que j'avais trouvé**, comme la consigne le demande, et je le dis ici
   pour que le chantier voisin qui manipule ce fichier sache que sa ligne a été
   retirée.

   **Et l'horloge de l'auteur, relancée ensuite, a réglé la question elle-même.**
   Le fichier porte maintenant `520 400` **et**
   `\\.\DISPLAY1 1920x1080 416.00 320.00` : 416 × 1,25 = 520 et 320 × 1,25 = 400,
   soit la même position dans les deux formats. Sous l'ancien lecteur comme sous
   le nouveau, l'îlot revient exactement où l'auteur l'avait laissé. Vérifié
   après coup, et c'est le programme qui l'a écrit, pas moi.

**Ce que je n'ai pas touché :** `dist\Veilleuse.exe`, qui date du 12 septembre.
Le §I ne le demandait pas, et le mettre à jour serait livrer une version que ce
chantier n'a pas mandat de livrer. Il a d'ailleurs gagné un second rôle : c'est
lui, désormais, qui sert de binaire d'**avant** à l'épreuve du pas 7, parce qu'il
ne porte aucune ressource d'icône et qu'il vit dans le dépôt. La copie qui porte
l'icône est `veilleuse-bin\icone-livree\veilleuse.exe` — une `RT_GROUP_ICON`
d'identifiant 1 et quatorze `RT_ICON`, vérifiées sur elle.

---

## Ce qui pourrait être faux

Dans l'ordre de ce que cela coûterait.

1. **Le mot n'est pas passé.** C11 est la seule cible que je ne pouvais pas
   mesurer, et elle domine les onze autres : *« si le mot tombe hors de
   l'ensemble, le dessin est faux »*. Onze cibles tenues ne valent rien contre
   un mot qui ne vient pas.

2. **Le contraste élevé n'est pas mesuré.** C'est le seul des quatre réglages
   du système où Windows pourrait substituer une ressource à une autre, et le
   pas 10 croyait l'avoir vérifié alors qu'il n'avait forcé qu'un drapeau
   interne de l'application. Le forcer pour de bon repeint tout le bureau de
   l'auteur, et le projet a pour doctrine de ne pas toucher à ses réglages.
   `DECISION-ICONE.md` §I tranche la question sur la documentation — *« Windows
   11 n'exige plus de ressource dédiée »* — et **ce chantier ne l'a pas
   vérifiée**. Ce qui est mesuré à sa place : la transparence, coupée pour de
   bon, ne change rien à l'icône, ni pour une instance en cours ni pour une
   instance qui naît sous le réglage (pas 10 bis).

3. **La taille que la barre des tâches dessine n'est pas mesurée.** Le chapitre
   « en plus » glisse des candidats de 30 × 30 sur la découpe du bouton parce
   que la table *Icon scaling* annonce 30 px à 125 % — précisément la table dont
   ce même chapitre établit que la barre ne la suit pas. Si elle dessinait à 28
   ou à 32, les nombres de cette table changeraient. Ce qui ne change pas :
   **aucun candidat ne tombe à zéro**, quand l'entrée servie par `LoadImageW` y
   tombe.

4. **Je n'établis pas de quelle entrée la barre des tâches part.** J'établis
   qu'elle ne dessine ni `ICON_BIG` ni `ICON_SMALL` — c'est décisif, l'épreuve
   le montre — et qu'elle met à l'échelle. Le meilleur candidat est l'icône que
   le shell donne pour le fichier, 40 × 40, réduite ; il laisse encore
   **199 pixels sur 900** d'écart. Le rééchantillonneur de la barre n'est ni la
   moyenne d'aire ni le bilinéaire, et je ne l'ai pas identifié. **Tant que ce
   n'est pas fait, on ne sait pas quelle entrée du fichier sert la surface où
   l'icône se voit le plus souvent.**

5. ~~**Le débord du sol n'est prouvé qu'à 256 px.**~~ **Levé par le pas 2 bis :**
   il est prouvé aux quatorze. Ce qui reste, plus petit et plus précis : la
   marge *déclarée* par `galet::marge` n'est pas la distance à laquelle le
   nuanceur *lit*, et il ne sort réellement du carré qu'à 96 et à 256 px. Le
   débord est gardé aux quatorze parce qu'il ne coûte rien et qu'il suivra le
   noyau du flou si celui-ci change ; **mais on ne sait pas où le nuanceur lit
   vraiment**, on sait seulement que c'est en deçà de sa marge déclarée.

6. **Tout est rendu sur l'Intel UHD.** Rien n'a été relu sur la NVIDIA. C'est
   pour cela que le plancher de trois niveaux est partout dans les seuils — mais
   la famille dessinée dérive du rendu de 256 px, et si ce rendu diffère d'un
   adaptateur à l'autre, les quatorze entrées en héritent.

7. **La barre sombre est celle de cette machine, ce jour.** `EnableTransparency
   = 1`, une couleur d'accentuation donnée, un seul écran. Un autre réglage
   donnerait un autre niveau, et le seuil de 49 n'a de sens que contre celui-là.

8. **Alt+Tab n'a pas été ouvert.** Les deux fenêtres portent `WS_EX_APPWINDOW`,
   c'est mesuré ; qu'Alt+Tab en présente effectivement deux entrées est une
   conséquence attendue, pas une observation. Le §I demandait de mesurer et de
   rapporter, pas de trancher.

9. **Le cache d'icônes de l'explorateur n'a pas été purgé.** Mes mesures du
   chemin du shell rendent nos entrées à l'octet, donc le cache ne masque rien
   ici. Mais si l'auteur voit une ancienne icône quelque part, c'est la première
   chose à écarter : `%LocalAppData%\IconCache.db` et
   `%LocalAppData%\Microsoft\Windows\Explorer\iconcache_*.db`, puis redémarrer
   l'explorateur.

10. **Le bouton dit « veilleuse.exe ».** Faute de `VERSIONINFO`. Mesuré, publié,
   non corrigé : ce serait décider du nom du programme, de son éditeur et de sa
   version, et ce n'est pas une question d'icône.

---

## Ce qui reste, et à qui

**À l'auteur, et à lui seul :** le pas 12. Deux images l'attendent —
`captures/integration-icone/pas12-le-mot-30px.png` (l'icône de 30 px seule sur
son gris, à 1:1) et `pas12-le-mot-dans-la-barre.png` (la même, telle que la
barre des tâches vivante la montre vraiment). Regarder, sans agrandir, et dire
le premier mot. L'ensemble déclaré est **verre, pierre, galet, nuit**.

**À l'auteur, et ce n'est pas une question d'icône :** le nom du bouton
(`VERSIONINFO`), et les deux entrées d'Alt+Tab.

**À un chantier suivant, si quelqu'un le veut :** identifier le chemin exact par
lequel la barre des tâches de Windows 11 compose son icône, puisque la table
*Icon scaling* de Microsoft ne le décrit pas.

---

## L'épreuve du juge, et ce qu'elle a coûté

Le §I, pas 11, demande une **relecture par un juge qui n'a pas fait le travail**,
et le socle en fait une règle de la barre. Un juge indépendant a relu le journal,
les sorties brutes, les six textes de charte au mot, les corrections des
fondations et le diff entier du code, avec une consigne qui lui disait d'être
dur. Il l'a été, et le dossier en est sorti différent.

**Ce qu'il a confirmé.** Sur plusieurs centaines de chiffres publiés, **un seul**
ne se retrouvait dans aucune sortie. Les textes du §H qui ne portaient pas de
chiffre sont dans `DESIGN.md` au caractère près ; les autres avec leur barré et
leur cause ; **rien n'a été effacé**, ni dans `DESIGN.md` ni dans
`ICONE-FONDATIONS.md`. Le code fait ce que le §I demande sur ses quatre points,
sans code mort, sans une ligne de prose en anglais, sans marque d'outil, et les
commits sont sous le nom de l'auteur sans trailer.

**Les onze erreurs réelles qu'il a trouvées, et ce qu'elles sont devenues.**

| # | ce qu'il a trouvé | corrigé par |
|---|---|---|
| 1 | **Le pas 10 ne forçait aucun réglage du système** : les cinq drapeaux sont ceux de l'application, et ils ne peuvent pas atteindre un `HICON`. Les vingt lectures étaient acquises d'avance, et le journal les donnait pour « mesuré et non déduit » | le **pas 10 bis** : un vrai réglage coupé, avec le témoin qui montre que la condition a varié (plus de 100 000 pixels sur la barre) ; et la phrase du pas 10 barrée |
| 2 | « Aucune taille n'est laissée à la mise à l'échelle de Windows » comptée parmi les douze barrés, **mais laissée debout dans la charte** et dans le mode d'emploi | barrée dans `DESIGN.md` §8 et dans `LISEZ-MOI.md`, avec sa cause mesurée |
| 3 | `EXSTYLE = 0x00040110` publié pour l'horloge : **ce nombre n'était dans aucune sortie** | corrigé — **puis la correction elle-même corrigée** : relancé, le pas 8 a rendu `0x00040110`. L'EXSTYLE n'est pas stable (`set_cursor_hittest` pose `WS_EX_LAYERED` et `WS_EX_TRANSPARENT` après la création) ; le pas 9 le lit maintenant deux fois et publie les deux |
| 4 | `ICONE-FONDATIONS.md` §7.2 affirmait encore le fond sombre supposé — en énonçant une fragilité que la mesure a réalisée —, et §9.1 que l'application n'a pas de bouton | les deux barrés, avec le calcul qui montre que la conclusion tient (`#5E5B57` lit 2,39:1 contre `#212121`) |
| 5 | `~~from_resource(1, None)~~` rangé parmi les « textes du §H » : le §H ne contient pas cette phrase | dit d'où elle vient — le §I pas 8 — dans le tableau et dans le texte |
| 6 | « trois corrections, toutes barrées **et datées** » : une seule l'était | la phrase dit ce qui est vrai |
| 7 | « les **quatre** textes du §H » suivis de six | cinq textes, six modifications, et la seule différence de forme est dite |
| 8 | Deux commentaires de code démentis par le chantier lui-même : `ICON_BIG` décrit comme servant la barre des tâches | barrés dans `src/main.rs` et `src/systeme.rs`, avec le renvoi à la mesure |
| 9 | « exact **à un dixième** près » là où l'écart est de deux | corrigé |
| 10 | Le `rc.exe` du SDK affirmé sans trace ; une citation de Microsoft sans adresse | `releve-du-compilateur.txt` ajouté — un seul `rc.exe` sur la machine, et ce que le journal de `cargo` montre et ne montre pas ; l'adresse et la date de la source ajoutées |
| 11 | `from_resource(…).ok()` avalait l'erreur en silence ; deux `clone()` inutiles | un `eprintln!` sur l'échec, les deux clones retirés |

**Les sept endroits où il a jugé que le journal affirmait plus qu'il n'avait
mesuré.** Le premier est l'erreur 1 ci-dessus. Les six autres sont corrigés à
leur place : le pas 6 a désormais son témoin et son épreuve déclarés — il était
le seul des douze à en manquer ; le ruban d'anticrénelage du pas 2 est mesuré
(124 pixels partiellement couverts, 0,0019 du carré) au lieu d'être invoqué ; la
phrase du pas 8 sur l'icône générique de l'îlot est écrite comme non mesurée ;
le « il est nécessaire » du pas 2 est borné à la taille où il a été prouvé ; la
« configuration réelle » du pas 9 cesse d'être une raison ; et **la taille de
30 px que la barre dessinerait** est portée à « Ce qui pourrait être faux »,
puisque c'est une hypothèse et non un relevé.

**Ce qu'il faut retenir, et ce n'est pas confortable.** Deux des corrections ne
sont pas des fautes de copie : l'erreur 1 retirait au pas 10 la valeur qu'il
s'attribuait, et l'erreur 2 laissait la charte du projet affirmer ce que ce même
chantier avait réfuté trois chapitres plus loin. **Aucune des deux n'aurait été
trouvée par celui qui a fait le travail** : la première demandait de lire ce
qu'un instrument prouve et non ce qu'il affiche, la seconde de relier un
chapitre à un autre document. C'est exactement ce à quoi sert un juge qui n'a pas
fait le travail, et c'est pourquoi le socle l'exige.

Le verdict complet, ses preuves et ses citations sont ce que ce chapitre
résume ; ce qu'il a confirmé comme ce qu'il a cassé est porté ci-dessus sans rien
retrancher.
