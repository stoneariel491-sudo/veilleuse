# La décision — le verre de l'îlot

L'auteur a délégué le choix en ces mots : « le rendu optimal, réaliste et beau ».
Les trois mots comptent ensemble. Un bord réaliste qui passe au-dessus de la
hiérarchie des encres est laid au sens de `DESIGN.md`. Un bord calme qui a perdu
sa réfraction est beau peut-être, mais ce n'est plus du verre taillé. Ce document
tranche les trois arbitrages que `TOUR3.md` a laissés ouverts, dit ce que chacun
coûte, et donne à l'intégrateur la liste exacte des paramètres.

**La direction décidée s'appelle ARBITREE** dans `recherche/verre-taille/src/mesure.rs`.
C'est la RETENUE avec une seule différence : la force de la réflexion passe de
0,30 à ~~**0,25**~~ ~~**0,19**~~ **une force par échelle d'affichage**. *Corrigé
à l'intégration après l'épreuve (`INTEGRATION-APRES-EPREUVE.md`, pas 1 et pas
1 bis) : 0,25 était dérivée sur le ruban du banc, à l'échelle 1 ; sous la loi de
`DECISION-APRES-EPREUVE.md` A1 (la lumière que le verre émet, lue
dépremultipliée sur la couverture exacte de l'application, coins compris), le
pire pixel monte avec l'échelle, parce que le congé la suit et que le premier
pixel physique lit une tranche plus raide de Fresnel ; une force unique pour
quatre échelles tombait à 0,19. L'auteur, à qui la question a été posée (« Une
force unique pour toutes les échelles, ou une force par échelle ? »), a
répondu : « Une par échelle ». La force est donc, à chaque échelle que Windows
propose, la plus grande du balayage au centième dont le pire pixel tient
≤ 90,37 sur huit fonds et deux états, lue par l'application sur l'Intel UHD
(`captures/integration-apres-epreuve/pas1/lisere-aux-echelles*.txt`,
`lisere-e*.txt`) :*

| échelle | 1 | 1,25 | 1,5 | 1,75 | 2 | 2,25 | 2,5 | 3 | 3,5 |
|---|---|---|---|---|---|---|---|---|---|
| force | **0,25** | **0,24** | **0,21** | **0,20** | **0,19** | **0,19** | **0,19** | **0,18** | **0,17** |
| pire pixel | 88,47 | 89,82 | 88,51 | 88,51 | 88,51 | 89,20 | 89,88 | 90,25 | 89,88 |

*Entre deux échelles, la force de la plus proche échelle mesurée au-dessus
(`galet.rs`, `arbitree::FORCES`). À l'échelle 1, celle de ce banc, la force
reste 0,25.* La couche nette reste la source brute. La couleur du bord n'est
pas touchée. Les directions RETENUE, PASSE-BAS et FLOU-FORT (la proposition du
partisan de la nuit) restent nommées à côté, pour que chaque tableau garde sa
colonne d'AVANT.

Toutes les mesures citées viennent de `captures/mesure/banc.txt`, essai 8.H,
sauf mention contraire. Le banc a tourné sainement deux fois avec la force 0,25.
L'essai 8.H est identique d'un lancement à l'autre, et les instruments sont
valides 9 sur 9. Aucun chiffre de ce document ne vient de l'instrument 6.

---

## Ce qui a été mesuré pour trancher

Les chiffres des partisans ne suffisaient pas, sur quatre points :

1. **La force n'avait jamais été dérivée sur tous les fonds.** L'essai 8.D ne
   lisait que quatre scènes, sans les coins. L'essai 8.F lisait le blanc seul,
   au pas de 0,02. L'essai 8.H lit **dix-neuf fonds** au centième, bord droit et
   coin : les quatre scènes, les dix aplats de 8.F, les deux peignes, le bureau
   gris, le bureau sombre, et un fond qui manquait, la **page de texte**. C'est
   le blanc tel qu'un bureau le montre vraiment.
2. **Le battement de nuit** n'avait été mesuré que sur des directions dérivées
   de V3, et sur le seul canal vert. Il ne l'avait jamais été sur la RETENUE, sur
   PASSE-BAS ni sur une force intermédiaire. L'essai 8.H le lit en luminance sur
   les quatre directions.
3. **Les dents sur un fond vraisemblable** : le partisan du réalisme notait
   qu'elles n'étaient mesurées que sur des peignes construits. L'essai 8.H les
   lit aussi sur la page de texte et sur le bureau vraisemblable.
4. **La combinaison 0,24 + σ = 3** du partisan de la nuit n'avait jamais été
   rendue comme une direction.

Trois témoins neufs portent ces mesures. Chacun a une réponse connue d'avance, et
chacun a été éprouvé : on a saboté sa condition, relancé, puis restauré et
relancé le cas sain.

| témoin | réponse connue | épreuve | l'instrument tombe ? |
|---|---|---|---|
| **8.13** premier pixel du blanc, prédit de zéro à chaque force de 0,20 à 0,30 | écart ≤ 1 octet (lu : 0 partout, 1 à 0,29) | prédiction faite avec la force de la RETENUE au lieu de la force balayée : écart de 13 octets à 0,20 | oui, 8 sur 9 |
| **8.14** sur du noir pur, chaque direction rend un galet entièrement noir | 0 (R × 0, réfraction de noir, teinte d'octets nuls) | fond noir remplacé par un gris 8 : le galet lit 2 | oui |
| **8.15** le cœur du galet bat de la même valeur pour les cinq directions | écart ≤ 1 niveau (lu : 12,0 contre 11,9) | fenêtre du cœur étendue dans le congé : le cœur passe de 11,9 à 16,8 | oui |

Les trois épreuves ont tourné dans le même lancement, et chaque verdict a
imprimé son propre échec. L'instrument 8 est passé à FAUX, 8 instruments sur 9.
Après restauration : 9 sur 9.

Deux chiffres dont je dépends ont été refaits par un autre chemin :

- **Le plafond** : luminance de `#5E5B57`, calculée hors du banc en Python,
  vaut **91,37**, comme le banc.
- **Le premier pixel du blanc** : je l'ai recalculé depuis la réflectance
  moyennée publiée, 0,2782 (`OPTIQUE.md` §4), sans passer par le code du banc.
  J'obtiens 86,2 à la force 0,25 (banc : 86), 92,1 à 0,30 (banc : 92) et 84,9 à
  0,24 (banc : 84). Les deux chemins s'accordent à un octet près. Le reste
  d'écart vient du fait que le banc ne compte la réflexion que sur les prises
  qui voient hors du galet ; le calcul fait à la main ne le fait pas.

---

## Arbitrage 1 — le blanc pur : **force 0,25** à l'échelle 1, et une table par échelle à l'intégration après l'épreuve

*Ce qui suit est la dérivation au banc, à l'échelle 1, sur le ruban
`smoothstep`. Elle reste vraie de ce qu'elle mesure, et 0,25 reste la force de
l'échelle 1. Ce qui entre dans `galet.rs` est désormais une table par échelle,
dérivée par l'application elle-même sur sa couverture exacte
(`INTEGRATION-APRES-EPREUVE.md`, pas 1 bis, décision de l'auteur) ; ~~0,19,
force unique aux quatre échelles~~ (pas 1) n'a vécu qu'un pas.*

### La mesure

Voici le pire des dix-neuf fonds, coins compris. Le seuil vaut 91,37 − 1 = 90,37 :
le plafond, moins la tolérance d'un arrondi.

| force | pire bord droit | pire coin | marge au plafond | verdict avec la marge d'un arrondi |
|---|---|---|---|---|
| 0,24 | 84 | 89 | +2,4 | tient |
| **0,25** | **86** | **90** | **+1,4** | **tient** |
| 0,26 | 87 | 92 | −0,6 | casse |
| 0,30 (RETENUE) | 92 | 97 | −5,6 | casse |

Le pire fond est la page de texte, à égalité avec le blanc 255 jusqu'à 0,29.

*Relu dans l'application, pas 2 de l'intégration (`INTEGRATION.md`).* Le chiffre
tient : l'outil `borne-du-bord.py` redonne 86 et 90 sur ce banc. Mais il faut
savoir d'où vient le 90 du coin.

- Il vient d'un pixel à d = −0,46. Le ruban `smoothstep` du banc le couvre à
  99,3 %, et le ruban exact de l'application à 96 % seulement.
- Dans l'application, ce pixel mêle 4 % du fond et lit 95 sur le blanc, alors
  que le verre qu'il porte lit 88,5.
- Sur les pixels couverts au moins autant qu'au banc, l'application lit 86.

La borne n'y tient donc qu'à couverture égale, pas à la lettre. La dérivation de
la force sur la seule couverture exacte n'est pas refaite.

### Pourquoi ce chiffre, et pas un autre

- **Le pas du centième n'est pas un goût.** Sur le blanc, un centième de force
  déplace le coin d'un à deux octets. C'est le pas de l'image. Au pas de 0,02
  (essai 8.F), 0,25 n'avait simplement jamais été rendu. La phrase « seule 0,24
  y tiendrait » est donc barrée dans `ESTHETE-TOUR2.md`, `TOUR3.md` et
  `mesure.rs`.
- **La marge d'un octet n'est pas un goût non plus.** Deux arrondis d'une même
  valeur diffèrent d'un octet (témoins 8.6, 8.9, 8.13). Un pixel qui tient à
  moins d'un octet du plafond ne tient que par hasard d'arrondi. Au-delà d'un
  octet, aucune grandeur mesurée ne dérive une marge. C'est pour cela que je
  prends 0,25 et non 0,24, que proposait le partisan de la nuit.

### Contre le partisan du réalisme (0,30 et une dérogation)

Son argument est juste sur un point : ce qui fait lire du verre, c'est le rapport
entre le bord et la face. Mais le prix qu'il demande pour garder 0,30 tombe
**précisément sur le fond le plus courant d'un bureau de jour** : une page
blanche, un document. Le coin de l'îlot y brillerait plus que la légende
`#5E5B57`. Or c'est la seule règle qui empêche un ornement de dépasser le texte
(`MATIERE.md` §5.2, `DESIGN.md` §2). Une dérogation « au-dessus de 239 » est
bien argumentée, mais elle écrit dans la loi que le bord gagne la hiérarchie là
où on le voit le plus souvent. Pour « beau » au sens de ce projet, où tout se
retire jusqu'à ce qu'il ne reste que ce qui se justifie, c'est le mauvais sens.

Il objecte aussi que le coin est l'endroit où l'analyse plane est la moins sûre.
La garantie porte pourtant sur **l'image**, pas sur la physique. Le pixel du coin
est lu exactement (témoins 8.5 et 9.B), et c'est ce pixel-là qu'on voit.

### Ce que 0,25 coûte, mesuré

| grandeur | RETENUE 0,30 | **ARBITREE 0,25** | perte |
|---|---|---|---|
| éclat du liseré sur 226, déployé (ajout au verre seul) | +36 | **+31** | −14 % |
| éclat du liseré sur 226, replié | +17 | **+14** | −18 % |
| §5.4, pire des trois tailles | 14,6 | **14,1** | −3 % |
| réflectance effective du pixel du bord (0,2782 × force) | 8,35 % | **6,96 %** | la physique en donne 27,8 % |

Le réalisme concède donc 75 % de la force physique au lieu de 70 %. C'était déjà
une atténuation d'auteur à 0,30, et elle ne conserve pas l'énergie : le terme
transmis reste (1 − R). Le liseré reste nettement au-dessus du verre seul, de
+31 niveaux au déployé. Il ne s'éteint pas.

**Ce que la force ne règle pas, et qu'il faut savoir.** Sur la rangée des
chiffres, le bord bat de **79,2** niveaux à 0,25, contre 85,2 à 0,30 et 14,2 pour
le verre seul. Baisser la force ne retire que 6 des 71 niveaux au-dessus du
témoin, soit 8 %. Le battement n'est pas un argument pour la force. À la place
par défaut, sur le noir au-dessus de la date, il vaut **0** pour toutes les
directions (témoin 8.14).

---

## Arbitrage 2 — les dents ou la réfraction : **couche nette brute, σ = 0**

### La mesure

| direction | force | σ | dents peigne 3 px | dents rayé coloré | dents page de texte | dents bureau vraisemblable | §5.4 pire | battement nuit |
|---|---|---|---|---|---|---|---|---|
| verre seul | — | — | 0 | 23,1 | 0 | 19 | 1,9 | 14,2 |
| RETENUE | 0,30 | 0 | 71 | 82,7 | 25 | 45 | 14,6 | 85,2 |
| PASSE-BAS | 0,30 | 1 | 46 | 76,8 | 23 | **45** | 10,7 | **85,2** |
| **ARBITREE** | **0,25** | **0** | 66 | 77,4 | 24 | 43 | **14,1** | 79,2 |
| FLOU-FORT | 0,24 | 3 | 1 | 43,4 | 8 | 41 | **4,3 — casse** | 76,2 |

Pour lire FLOU-FORT sans confondre les deux effets, il faut la même force sans
passe-bas. Le premier lancement de l'essai 8.H a rendu l'ARBITREE à 0,24,
avant que la dérivation ne la corrige à 0,25. On y lisait : dents 64 / 76,4 /
24 / 41, §5.4 14,0, battement 78,2.

### Ce que ces lignes disent

- **Le passe-bas n'agit que sur le fond construit pour lui.** Sur le bureau
  vraisemblable, PASSE-BAS lit 45 et la RETENUE 45. À même force, FLOU-FORT lit
  41 et l'ARBITREE à 0,24 lit 41. La nuit non plus ne bouge pas : PASSE-BAS bat
  de 85,2, la RETENUE de 85,2. Tout ce que le flou gagne se gagne sur un peigne
  de 3 px, que `RECETTES.md` défend de laisser condamner seul un réglage.
- **Ce qu'il coûte se paie partout.** FLOU-FORT ramène §5.4 à **4,3**, soit
  moins de la moitié de la cible de dix, et à peine plus que le verre seul
  (1,9). La réfraction visible disparaît, et c'est elle qui fait d'un liseré
  gris un verre taillé. Pour ce prix, la nuit gagne **2 niveaux** de battement
  (78,2 → 76,2), soit 3 % de ce que le bord ajoute au témoin.
- **PASSE-BAS** garde §5.4, mais avec 7 % de marge seulement (10,7). Il invente
  une rugosité sur la seule face que le modèle déclare polie, puisque la
  réflexion lit la même liaison. Et il ne change rien hors du peigne.
- **Réaliste** : un chant poli montre ce qu'il réfracte, net. Les dents de la
  page de texte (24, contre 0 pour le verre seul) sont les lignes du document
  déplacées par le biseau. C'est exactement ce que fait un vrai verre. Le seul
  passe-bas physique légitime existe déjà : la moyenne sur l'empreinte du pixel,
  à huit prises.

L'argument du partisan de la nuit (« si on remarque l'îlot en lisant l'heure, il
a échoué ») est le bon critère. Mais il vaut **la nuit, là où l'îlot est posé** :
sur du noir, où les trois arbitrages rendent zéro. Là où l'auteur le pose sur
les chiffres, les chiffres font 311 px de capitale, sans aucun peigne, et le
flou n'y retire que 2 niveaux.

### Ce qui est sacrifié

Le calme du bord sur un fond rayé fin : 66 de dents sur le peigne gris, contre 1
sous FLOU-FORT, et 77,4 contre 43,4 sur le rayé coloré. L'objection 4 de
l'esthète reste **partiellement traitée**, et c'est un choix : je la tiens pour
le prix de la réfraction, pas pour un défaut.

---

## Arbitrage 3 — la couleur du bord : **rien n'est retiré**

### La mesure

Saturation de la bande de bord sur le bureau rayé coloré (0-5 px) : RETENUE 62,
**ARBITREE 60**, PASSE-BAS 62, FLOU-FORT 55. Pour comparer : cœur 35, verre seul
36, fond nu 176 (essai 8.B). Frange de dispersion : 0 px, 0 niveau pour
l'ARBITREE (instrument 3). Sur le bureau vraisemblable, la saturation vaut 0.

### Pourquoi

- La couleur **inventée**, celle que la couche nette ramenait en RVB (131), est
  déjà retirée par `nette_luminance`, que l'ARBITREE garde. Ce qui reste, c'est
  la réflexion de surface du pixel voisin. Elle ne traverse pas le corps teinté.
  Un bord plus coloré qu'un cœur flouté et teinté est la signature physique de
  cette réflexion (`OPTIQUE.md` §5, `MATIERE.md` §5.2 : « celle du pixel de
  bureau qui touche le galet »).
- Elle reste à 34 % de la saturation de ce qu'elle laisse voir. Le second
  témoin de 8.B dit qu'une matière n'invente pas de couleur tant qu'elle reste
  sous cette saturation-là. Le verre seul porte déjà 36 : la lettre de
  `MATIERE.md` §6, « aucune saturation hors de l'axe des gris », était cassée
  avant tout bord. Elle ne peut interdire que la couleur inventée.
- Ramener le bord au cœur demanderait une force d'environ zéro, donc éteindre
  le liseré. Désaturer la seule réflexion dessinerait un trait gris à côté d'un
  voisin coloré, c'est-à-dire un contour, que `DESIGN.md` §8 interdit. **Ce
  remède n'est pas mesuré.**
- La nuit de ce produit est achromatique. La question ne s'y pose pas.

La force 0,25 fait baisser la saturation de 62 à 60. C'est un effet de bord de
l'arbitrage 1, pas un remède.

---

## Pour l'intégrateur — les paramètres exacts

C'est `verrier.wgsl` avec l'uniforme ci-dessous, soit la constante `ARBITREE`
dépliée. Toute valeur qui n'est pas dans cette liste est hors de la décision.

| paramètre (`Direction` du banc) | valeur | sens |
|---|---|---|
| `modele` | `Verrier` | nuanceur `recherche/verre-taille/src/verrier.wgsl` |
| `sigma` | **18,0 px** | écart-type du flou de la couche floutée, en linéaire |
| `teinte` | **0,80** | teinte posée sur les octets **encodés** |
| `saturation` | **1,35** | vibrance de la couche floutée |
| `vibrance_relative` | **faux** | |
| `largeur` (congé `b`) | **12 px à h = 148**, soit `12 × h / 148` | |
| `epaisseur` (`e`) | **12 px à h = 148**, soit `12 × h / 148` | |
| `conge_relatif` | **vrai** | `b` et `e` suivent la hauteur, comme le rayon et l'exposant (`DESIGN.md` §6) |
| `indice` | **1,51680** | N-BK7 |
| `dispersion` | **0** | une seule passe de réfraction |
| `galbe` | **0** | |
| `refraction` | **1,0** | Snell, prise sur la couche **nette** |
| `clarte` | **1,0** | la couche nette ramenée au biseau |
| `profil` | **1** | rampe (1 − t)², pas de marche |
| `nette_luminance` | **vrai** | la couche nette n'apporte que la luminance |
| `nette_sigma` | **0** | **aucun passe-bas** : la couche nette est la source brute |
| `fresnel` | **1,0** | Fresnel exact, moyenné sur l'empreinte |
| `prises` | **8** | prises sur l'empreinte du pixel |
| `lisere` (force de la réflexion) | ~~**0,25**~~ ~~**0,19**~~ **table par échelle**, 0,25 à 1, 0,24 à 1,25, 0,21 à 1,5, 0,20 à 1,75, 0,19 de 2 à 2,5, 0,18 à 3, 0,17 à 3,5 | ~~dérivée, essai 8.H~~ dérivée sur la couverture exacte de l'application à chaque échelle de Windows, décision de l'auteur (`INTEGRATION-APRES-EPREUVE.md`, pas 1 bis) |
| `beer` | **0** | |
| `integre` | 0 | inutilisé par `Verrier` |

Ce que l'application doit donc lier : **deux couches** du bureau capturé sous
l'îlot, la floutée (σ = 18) et la nette (brute). `galet.wgsl` n'a aujourd'hui
qu'un uniforme et aucune texture. Les acquis de capture sont dans
`VERRE-TAILLE.md` : fenêtre exclue de la capture, flou en linéaire, teinte sur
les octets encodés, conscience du DPI avant toute fenêtre, horodatage de
présentation, copie avant la libération de la trame, acquisition non bloquante.

Trois choses que l'intégration doit écrire, et qui ne le sont pas encore :

1. **`DESIGN.md` §8** dit encore « Pas de flou d'arrière-plan sous l'îlot : il
   est noir opaque ». Le verre et le liseré doivent y entrer comme dérogation
   nommée et bornée (`MATIERE.md` §6, « pas en fraude »). La borne décidée est
   la suivante : *le premier pixel du bord, coins compris, ne dépasse pas
   `#5E5B57` (91 niveaux de luminance) sur aucun fond, blanc pur compris, avec
   la marge d'un octet*. *Fait au pas 2 de l'intégration : la phrase est barrée
   dans `DESIGN.md` §8 et la dérogation écrite avec cette borne. Ce que la borne
   devient dans l'application est dit dans `INTEGRATION.md`, pas 2.*
2. **L'en-tête de `src/verre.rs`** affirme qu'une fenêtre ne peut pas lire ce
   qu'il y a derrière elle. Le laboratoire l'a démenti. *Fait au pas 1 de
   l'intégration : la phrase est barrée et renvoie à `VERRE-TAILLE.md` §9.*
3. **L'horloge à volets ne doit pas bouger** : `outils/comparer-captures.py`,
   avec son témoin (zéro à la même heure, un grand nombre à une heure
   différente). Ce tour-ci n'a touché aucun fichier de `src/` : `git diff
   8289af9 -- src/` est vide.

---

## Ce qui reste non mesuré

- ~~**Aucun œil n'a regardé une planche de l'ARBITREE.**~~ La planche existe
  depuis le pas 3 de l'intégration : `captures/integration/pas3-oeil/planche.png`,
  l'îlot réel tel que Windows le compose, sur trois fonds et trois formes
  (`INTEGRATION.md`, pas 3). L'intégrateur l'a regardée et dit ce qu'il voit.
  **L'auteur ne l'a pas encore jugée**, et le « beau » reste à ratifier par lui.
- **Le plafond de 91 n'est pas vérifié à l'œil**, pas plus que le seuil de 45 ou
  le « dix fois » de §5.4. Ce dernier est ancré sur une esquisse dont le champ
  de déplacement était faux.
- **Les fonds hors du banc** : photographies, HDR, écrans à plus de 8 bits,
  fonds animés. Le blanc 255 en aplat est la borne haute des aplats. Pour un
  fond structuré, c'est raisonné, pas démontré.
- **Les coins en trois dimensions** : l'analyse reste plane.
- **Le battement quand un volet tourne sous le bord**, et le glissement des
  dents quand une fenêtre défile derrière l'îlot. Ce sont des mesures dans le
  temps, et aucune n'existe.
- **La durée pendant laquelle l'îlot reste déployé la nuit.**
- **Le liseré sur un bureau sombre à 0,25** : +3 niveaux à 0,30 (essai 8.E),
  non relu à 0,25.
- **Le coût GPU dans l'application.** Au banc, l'ARBITREE a le même nuanceur et
  les mêmes liaisons que la RETENUE, qui coûte « environ deux fois » la face nue
  (`TOUR3.md`, case 4). Seul un uniforme change. L'instrument 6 est instable et
  ne l'a pas relue.
- **La désaturation de la seule réflexion**, écartée par raisonnement (elle
  dessinerait un contour), n'a pas été rendue.

## Ce qui a changé dans le banc

- `mesure.rs` : `TOLERANCE_ARRONDI`, `FORCE_ARBITREE = 0,25`, directions
  `ARBITREE` et `FLOU_FORT`, fond `fond_page_texte`, lecteur
  `battement_de_nuit` (en luminance), essai 8.H `arbitrage` avec les témoins
  8.13, 8.14 et 8.15.
- L'ARBITREE passe aussi par les instruments 1 à 5. Distorsion, dispersion,
  matière et lisibilité y sont identiques à la RETENUE : 12,17 au cœur,
  0,000 px de frange. Elle ne passe pas par l'instrument 6, pour la raison
  donnée plus haut.
- Un piège a été évité : les planches de mire prennent la première lettre du
  nom de direction, et « ARBITREE » écrasait celles de « A. dalle ». L'ARBITREE
  en est filtrée, et les trois PNG ont été restaurés.
