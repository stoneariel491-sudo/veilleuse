# L'intégration du verre taillé

Le verre décidé dans `DECISION.md` entre dans l'application en trois pas. Ce
document dit où en est chacun, ce qui est prouvé et comment le reprouver.

## Pas 1 — la fenêtre et la capture : fait

**Ce qui est entré.** L'apparence n'a pas bougé : le galet reste noir opaque.

- `src/bureau.rs` porte la capture du laboratoire. On y trouve la déclaration de
  la conscience du DPI, l'exclusion de la capture et le `Capteur` : acquisition
  non bloquante, `LastPresentTime`, copie avant `ReleaseFrame`, rectangles
  sales et de déplacement. S'y ajoute le `Suivi` : copie forcée quand la région
  change, réouverture quand l'accès est perdu, compteurs.
- `main.rs` déclare la conscience du DPI **avant** la boucle d'événements,
  donc avant toute fenêtre et avant wgpu.
- `ilot.rs` se déclare exclu de la capture dès sa construction, avant sa
  première trame, et relit l'affinité (17). À chaque trame, il suit la région du
  **galet courant** plus `MARGE` = 64 px de chaque côté. Cette marge est celle
  du banc, supérieure au noyau `ceil(3 σ)` = 54 px pour σ = 18.
- Rien de ce qui est capturé n'est encore peint. La région vit en octets RGBA
  dans `ilot.suivi.region`, avec son origine réelle (rognée au bord de
  l'écran).

**Comment le reprouver.**

```
cp target/release/veilleuse.exe <ailleurs>/veilleuse-copie.exe
timeout 120 <ailleurs>/veilleuse-copie.exe --releve-bureau=captures/integration/pas1-sain
```

Le relevé ouvre les vraies fenêtres, se déroule en 15 à 30 s, quitte tout
seul (fil de garde à 60 s) et écrit `releve.txt` ainsi que les six images du
témoin. **Lancer depuis une copie** : voir le piège du DPI ci-dessous.

**Ce qui est prouvé, avec le témoin de chaque mesure** (`pas1-sain`) :

| verdict | témoin, réponse connue | lu | épreuve qui le fait tomber |
|---|---|---|---|
| exclusion posée | affinité relue 17 | 17 | construction sans exclusion : 0, FAUX |
| DPI accordé | coordonnées de la sortie = texture dupliquée | 1920 × 1080 = 1920 × 1080 | `--epreuve-dpi` : 1536 × 864 contre 1920 × 1080, FAUX (`pas1-epreuve-dpi`) |
| îlot absent de la capture | caché contre exclu, fond stable : 0 | 0 px | image « exclu » prise sans exclusion : 4102 px, FAUX |
| la capture voit l'îlot non exclu | caché contre non exclu : > 0 | 73 075 px sur 74 000 | c'est lui le témoin positif du zéro |
| îlot absent après rétablissement | 0 | 0 px | reste VRAI quand seul « exclu » est saboté |
| région suit le déploiement / le repli | au repos, région = boîte de la découpe GDI relue à Windows + 64 | 528 × 313 et 238 × 173, position comprise | région d'un pixel trop large : 529 contre 528, 239 contre 238, FAUX |
| réouverture | duplication fermée à la main, rouverte, image neuve | 0,131 s | réouverture empêchée : FAUX après 4 s |

Les quatre sabotages ont tourné dans le même lancement : **3 verdicts sur 8**,
exactement les prévus (`pas1-epreuves`). Après restauration, `git diff -- src`
vide, relance saine : **8 sur 8**. Les cinq comparaisons d'images du témoin ont
été refaites par `outils/comparer-captures.py`, qui retrouve les mêmes nombres.

**L'horloge et le galet n'ont pas bougé d'un pixel** (`comparer-captures.py`,
binaire de `2547504` contre binaire final) :

| capture | même réglage | témoin, réglage différent |
|---|---|---|
| horloge `--heure=03:14:15` | 0 | 10 018 (03:14:16) |
| îlot déployé `--restant=41:27` | 0 | 370 (41:28) |
| îlot replié `--p=0` | 0 | 2 942 (contre déployé) |

**Le régime, lu dans l'application** (un lancement, tics à 60 Hz, machine
partagée) :

| phase | tours | copies | dont forcées | copie médiane | tour à vide médian |
|---|---|---|---|---|---|
| repos replié | 210 | 6 | 0 | 0,355 ms | 0,049 ms |
| déploiement | 112 | 73 | 32 | 0,991 ms | 0,073 ms |
| repos déployé | 125 | 5 | 0 | 1,280 ms | 0,055 ms |
| l'îlot se repeint à chaque tic | 248 | 114 | 0 | 1,266 ms | 0,049 ms |
| déplacement de 2 px par tic | 248 | 121 | 118 | 0,708 ms | 0,050 ms |
| repli | 110 | 67 | 30 | 0,626 ms | 0,052 ms |

## Ce que le pas 2 doit savoir

1. **L'îlot exclu produit des rectangles sales sur sa propre fenêtre.** Quand
   il se repeint, le plus grand rectangle sale qui croise la région est
   exactement la fenêtre de l'îlot. Un verre qui se repeint après chaque
   capture relancerait donc une copie à chaque trame. L'image ne boucle pas,
   puisque l'exclusion tient ; le travail, lui, boucle. Pistes non mesurées :
   ignorer un rectangle sale égal à notre fenêtre, ou borner la cadence.
2. **Des rectangles sales de l'écran entier** arrivent parfois : 63 copies sur
   186 tours au repos à un lancement, 6 sur 210 au suivant. La cause n'est pas
   isolée.
3. **La capture n'avance qu'avec les trames de l'îlot** : une fois par seconde
   au repos, à chaque trame pendant un mouvement. Un bureau qui change sous
   l'îlot immobile (vidéo, défilement) ne réveille rien aujourd'hui. Le verre
   devra décider de sa cadence.
4. **Deux adaptateurs** : wgpu sur l'Intel UHD, la duplication sur la NVIDIA
   RTX 4060. La relecture par le processeur puis le téléversement est la voie.
   Le téléversement n'est pas écrit.
5. **σ et la marge sont en pixels physiques.** Si σ suit l'échelle de l'écran
   (1,25 ici), `MARGE` doit la suivre.
6. **`DESIGN.md` §8** dit encore « pas de flou sous l'îlot ». La dérogation
   bornée de `DECISION.md` est à écrire au pas où l'apparence change.

## Ce qui n'est pas mesuré

- une perte d'accès **réelle** (changement de mode, bureau sécurisé, plein
  écran exclusif) : seule la réouverture après fermeture à la main l'est ;
- plusieurs moniteurs, une sortie tournée, un écran HDR ;
- ce que coûte, sur batterie, un périphérique D3D11 ouvert toute la nuit sur la
  carte NVIDIA ;
- la cause des rectangles sales plein écran ;
- la raison pour laquelle, à un lancement, l'îlot s'est trouvé déplacé de
  (71, 23) px avant le témoin : ce déplacement vient de l'environnement, pas du
  script.

## Un effet de bord à connaître

L'épreuve DPI lancée depuis `target/release/veilleuse.exe` a fait écrire par
Windows une couche de compatibilité `HIGHDPIAWARE` pour ce chemin :
`HKCU\Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers`,
valeur nommée d'après le chemin complet de l'exécutable. Depuis, la déclaration
y est refusée (`0x80070005`) alors que la conscience lue reste bonne, et le
relevé l'imprime. Elle n'a pas été retirée : le retrait se fait à la main,
dans le registre ou dans l'onglet Compatibilité des propriétés de
l'exécutable.

---

## Pas 2 — le galet de verre : fait

**Ce qui est entré.** L'îlot sur le bureau est désormais de verre. Hors du galet,
rien ne change : c'est toujours le vrai bureau que le compositeur montre.

- **`src/galet.wgsl`** reçoit deux entrées de plus :
  - `flou`, une passe gaussienne séparable en linéaire ;
  - `verre`, le nuanceur de `verrier.wgsl` réduit aux branches que l'ARBITREE
    allume : Snell sur la couche nette, Fresnel exact moyenné sur huit prises,
    rampe (1 − t)², couche nette en luminance seule, teinte sur les octets.

  L'entrée `fs` du galet noir n'a pas changé d'un octet.
- **`src/galet.rs`** porte le reste :
  - le module `arbitree`, qui est la liste de `DECISION.md` paramètre par
    paramètre ;
  - `marge(echelle)` ;
  - trois couches `Rgba8UnormSrgb` (nette, intermédiaire, floutée), allouées
    une fois à la taille de la fenêtre plus deux marges ;
  - le flou, refait **seulement** quand un fond neuf est téléversé ;
  - la composition, à chaque trame.

  Sans fond, le tuyau noir d'avant peint.
- **Les longueurs sont en pixels logiques**, comme toutes celles de `DESIGN.md`.
  À 1,25 on a donc σ = 22,5 px physiques, un noyau de 68 et une marge de
  capture de 80. Le conge et l'épaisseur suivent en plus la hauteur du galet.
  ~~`MARGE` = 64 px physiques~~ (pas 1) : à 64, le noyau de 68 aurait lu ses
  quatre derniers pixels dans le bord recopié.
- **`src/ilot.rs`** gère le fond.
  - *Une copie neuve* n'est gardée que si ses octets diffèrent du fond déjà
    téléversé. C'est ce qui coupe la boucle annoncée au point 1 ci-dessus.
  - *Une capture perdue ou fermée* rend le galet noir à la trame suivante.
  - `dessiner` téléverse le fond neuf et donne à la vitrine sa place dans le
    bureau.
  - `sonder` fait un tour de capture entre deux trames.
- **`src/main.rs`** lance un fil de sonde à la période de rafraîchissement du
  moniteur de l'îlot : 6,94 ms ici, écran à 144 Hz. C'est le point 3 ci-dessus.
- **Instruments** :
  - de l'application : `--verre-fond`, `--verre-decalage`, `--sans-contenu`,
    `--adaptateur`, et les épreuves `--epreuve-boucle`, `--epreuve-repli` et
    `--epreuve-sans-sonde` ;
  - du banc : `mesure integration` ;
  - les outils `outils/verre-contre-banc.py` et `outils/verre-en-direct.py`.

### L'application contre le banc, sur les mêmes fonds

Sept fonds du banc (page de texte, bureau vraisemblable, rayé coloré, bureau
sombre, horloge, coupé, blanc), chacun déployé (320 × 148) et replié (88 × 36),
à l'échelle 1, contenu caché. Le rendu de l'application est composé sur les
octets comme le fait Windows.

| comparaison | intérieur du galet | ruban d'un pixel | hors du galet |
|---|---|---|---|
| application sur la NVIDIA, adaptateur du banc | **≤ 1 niveau**, au plus 6 px sur 46 504 | jusqu'à 21 niveaux | 0 |
| application sur l'Intel UHD, adaptateur de la fenêtre | **≤ 3 niveaux** (rayé coloré déployé, moyenne 0,35), ≤ 1 sur les autres fonds | jusqu'à 20 niveaux | 0 |

**Pourquoi le ruban s'écarte.** Le banc mélange le verre et le fond en linéaire,
sur `1 − smoothstep(−0,5 ; 0,5 ; d)`. L'application laisse le compositeur
mélanger en alpha prémultiplié sur les octets, avec la couverture linéaire du
galet noir, celle que la découpe GDI suit. La cause est prouvée par un autre
chemin. On dé-prémultiplie la sortie de l'application, on la remélange à la
manière du banc, et l'on retombe sur le banc à au plus 1 niveau sur tout le
ruban.

**Pourquoi l'intérieur s'écarte sur l'Intel.** C'est l'adaptateur. Le même
nuanceur rend à 1 niveau près sur la NVIDIA, et l'Intel retombe exactement sur
l'écran (tableau suivant). Le mécanisme, c'est-à-dire l'opération flottante qui
diverge, n'est pas isolé.

| témoin ou épreuve | réponse connue | lu |
|---|---|---|
| banc sur un autre fond | l'intérieur s'écarte partout | 2 900 px sur 2 900 et jusqu'à 46 504 sur 46 504 |
| épreuve : force 0,30 au lieu de 0,25 | l'intérieur doit dépasser 1 | 7 niveaux, 870 à 3 264 px : FAUX |

### Ce que Windows compose, contre le rendu hors-écran

Le relevé pose l'îlot sur la rangée des chiffres, contenu caché, et écrit :

- `verre-fond.png`, le fond que le verre a téléversé ;
- `verre-avant.png`, l'écran capturé îlot non exclu.

`outils/verre-en-direct.py` refait le verre hors-écran sur ce fond, le compose
sur les octets et compare.

| rendu hors-écran sur | intérieur | ruban | hors |
|---|---|---|---|
| **Intel UHD**, l'adaptateur de la fenêtre | **0 px sur 72 712** | 0 sur 364 | 0 sur 924 |
| NVIDIA | ≤ 2 niveaux, 21 436 px | ≤ 1, 94 px | 0 |

Les témoins :

- le fond n'a pas bougé entre les deux images : 0 pixel changé sur 119 200 hors
  de la boîte du galet ;
- le verre rendu sur la page de texte du banc s'écarte sur 72 712 px sur 72 712.

**De combien l'écran s'écarte du banc, et pourquoi.** L'écran est exactement le
rendu hors-écran de l'Intel. Il s'écarte donc du banc d'au plus **3 niveaux à
l'intérieur du galet**, à cause de l'adaptateur, et d'au plus **20 niveaux sur le
ruban d'un pixel**, à cause du modèle de composition du bord. Hors du galet, il
ne s'écarte pas.

### La borne de `DESIGN.md` §8 dans l'application

`outils/borne-du-bord.py` lit la borne sur le rendu de l'application, composé
sur les octets comme Windows le fait. Il couvre sept fonds, déployé et replié.
Le seuil est le plafond moins la
marge d'un octet : 91,37 − 1 = 90,37. Il y a trois lectures, parce que la bande
du banc ne veut pas dire la même chose dans les deux rendus.

| lecture | NVIDIA | **Intel UHD, l'écran** | épreuve : force 0,30, Intel |
|---|---|---|---|
| banc relu, témoin : 86 et 90 dans `DECISION.md` | 86 au bord droit, 90 au coin : VRAI (+0,37) | | |
| 1. **à la lettre** : bande d ∈ [−1,5 ; −0,45], image composée | 95 : **FAUX** (−4,63) | 95 : **FAUX** (−4,63) | 101 : FAUX |
| 2. **à couverture égale au banc** : même bande, alpha ≥ 254 | 87 : VRAI (+3,37) | **86 : VRAI (+4,37)** | 92 : FAUX (−1,63) |
| 3. **le verre seul**, sortie dépremultipliée, bande à la lettre | 88,47 : VRAI (+1,90) | 88,47 : VRAI (+1,90) | 94,71 : FAUX (−4,35) |

Sources des chiffres :

- les rendus commités de `pas2-app`, qui sont ceux de la NVIDIA, redonnent
  exactement la colonne NVIDIA ;
- les deux colonnes ont été refaites depuis une copie du binaire final.

**Où la lettre casse.** Sur le blanc déployé, 16 pixels dépassent le seuil, et
ce sont tous des pixels de coin à d = −0,46 et d = −0,472.

- **L'application** les couvre à 245 et 248 sur 255. C'est la couverture exacte
  0,5 − d du ruban de `DESIGN.md` §6.
- **Le banc** les couvre à 0,9928 et 0,995, par son `smoothstep`.
- **Le verre qu'ils portent** lit 88,5 et 87,4. Les 4 % de blanc que le
  compositeur y ajoute sur les octets les portent à 95.
- **Sur les pixels couverts en entier**, l'application et le banc lisent tous
  deux 86.
- **Le 90 du coin publié par `DECISION.md`** est lu sur ce même pixel à
  d = −0,46.

**La cause, par un autre chemin.** L'outil prédit l'image de l'application
depuis l'image du **banc** et la seule couverture de l'application, sans lire la
couleur de l'application. Sur les 336 pixels de bande que la lecture 2 écarte,
la prédiction tombe à au plus 3 niveaux. Ce reste vient du peu de fond que le
banc laisse lui-même passer à ces pixels, 0,7 % en linéaire.

**Le seuil de la lecture 2 n'est pas posé au jugé.**

- Il dérive de la couverture du banc à la limite de sa bande :
  1 − smoothstep(−0,5 ; 0,5 ; −0,45) = 0,99275, soit 253,15 octets, arrondi au
  dessus à 254.
- Le filtre n'est pas aveugle : il garde 12 684 pixels de bande et en écarte
  336, et sur les bords droits il redonne la lecture à la lettre.
- Les trois lectures tombent à la force 0,30, et le bord droit y lit 92, le
  chiffre du banc (`DECISION.md`, RETENUE).

**Le jugement.** À la lettre, la borne **ne tient pas** dans l'application : 16
pixels de coin lisent 95 sur le blanc.

Ce ne sont pas des pixels du liseré. Ce sont des pixels du ruban
d'anticrénelage, que le banc comptait comme pleins parce que son ruban est plus
étroit que la couverture exacte. Le verre qu'ils portent tient (88,47), et le
bord couvert en entier tient avec 4,37 de marge. Ce que la borne protège,
c'est-à-dire un liseré qui ne brille pas plus que la légende, tient donc. Ce qui
dépasse est le blanc du fond mêlé au bord, comme au bord de toute lettre
anticrénelée posée sur ce blanc.

C'est pourtant un **choix de lecture**, et il est écrit comme tel dans
`DESIGN.md` §8, **à ratifier par l'auteur**. Si la lettre doit primer, il faut
baisser la force ou rétrécir le ruban du verre, et ni l'un ni l'autre n'est dans
la décision.

**Ce que cela change à la dérivation de la force, non mesuré.** L'essai 8.H a
fixé 0,25 parce que le coin passait de 90 à 0,25 à 92 à 0,26. Ce coin est un
pixel que l'application ne couvre qu'à 96 %. Refaire la dérivation sur la
couverture exacte pourrait donc desserrer la force. Ce balayage n'a pas été
rendu.

### Le relevé du verre en direct

**14 verdicts sur 14**, en 22,9 s. Les
huit du pas 1 tiennent toujours, et six sont neufs.

| verdict | témoin, réponse connue | lu | épreuve qui le fait tomber |
|---|---|---|---|
| le verre ne boucle pas au repos | chaque trame a une cause : borne = secondes entamées + fonds changés + 1 | 2 trames, borne 4, 43 copies identiques écartées | `--epreuve-boucle` : 122 trames, borne 4, FAUX |
| verre posé en direct | sur les chiffres, l'intérieur n'est pas noir | 72 672 px non noirs sur 72 672 | c'est lui, le témoin du verdict suivant |
| capture perdue : galet noir, sans image figée | capture fermée et empêchée : 0 px non noir | 0 sur 72 672, fond non téléversé | `--epreuve-repli` : 72 664 non noirs, l'image figée, FAUX |
| verre revenu après réouverture | l'intérieur redevient du verre | 72 672 sur 72 672 | — |
| la sonde fait avancer la capture sous un îlot immobile | îlot sur les secondes, relevé muet 3 s : tours de sonde > 0 | 411 tours, 95 fonds changés, 94 téléversements | `--epreuve-sans-sonde` : 0 tour, 2 téléversements, FAUX |
| la sonde ne fait pas boucler le verre | même borne | 98 trames, borne 100 | `--epreuve-boucle` : 226 trames, borne 82, FAUX |

Chaque épreuve fait tomber **exactement** les verdicts prévus, et rien d'autre :
13 sur 14 sans la sonde, 12 sur 14 sans la comparaison des octets (les deux
bornes), 13 sur 14 sans le repli. Les épreuves sont des drapeaux : le binaire
éprouvé est le binaire sain.

**Le galet noir et l'horloge n'ont pas bougé d'un pixel.** Sans fond, le verre
n'est jamais posé. Captures du binaire final comparées à celles de `cc8abeb`
(`comparer-captures.py`) :

| capture | même réglage | témoin |
|---|---|---|
| horloge `--heure=03:14:15` | 0 | 10 018 (03:14:16) |
| îlot déployé `--restant=41:27` | 0 | 370 (41:28) |
| îlot replié `--p=0` | 0 | 2 942 (contre déployé) |
| îlot déployé à 1,25 | 0 | — |
| îlot à mi-course, p = 0,6, à 1,25 | 0 | — |

### Deux instruments qui se sont trompés, et ce qu'ils ont appris

- **Sur le noir, le verre est noir.** Le premier relevé jugeait le verre à la
  place par défaut de l'îlot, sur le bandeau noir de l'horloge. Il y lisait
  3 961 pixels non noirs sur 70 968 : trop peu pour distinguer un verre d'un
  galet noir. Le relevé pose désormais l'îlot sur la rangée des chiffres.
- **L'intérieur du galet se lit par sa distance.** Le relevé suivant a conclu
  FAUX, avec 6 pixels non noirs dans le galet « perdu ».
  Relus par la distance signée
  du nuanceur, ils étaient tous dans le ruban : 0 à couverture pleine, 156 dans
  le ruban. Le masque rétréci de deux pixels en x a été remplacé par la règle du
  nuanceur, d ≤ −0,5.

### Ce que le pas 3 doit savoir

1. **Le premier relevé n'a obtenu aucune image présentée en 60 s.** Le binaire
   du pas 1, lancé juste après, en a obtenu (8 sur 8), et le même binaire
   relancé aussi (12 sur 12). La cause n'est pas isolée ; l'écran était inactif
   depuis près d'une minute.
2. **La nuit, l'horloge présente à chaque bascule** : 63 copies identiques en
   3 s, îlot replié. Chacune coûte une copie (0,35 à 1,3 ms au pas 1) et une
   comparaison. À 144 Hz, la sonde réveille la boucle 144 fois par seconde. Ni
   l'un ni l'autre n'est mesuré en coût réel, sur secteur comme sur batterie.
3. **Le coût GPU du verre dans l'application n'est pas mesuré** : le flou à
   chaque fond neuf, la composition à chaque trame.
4. **Au lancement, la première trame est le galet noir**, jusqu'à la première
   copie. Ce n'est pas mesuré en durée.
5. **La lecture de la borne est à ratifier.** Deux lectures s'opposent :
   - à la lettre, 16 pixels de coin lisent 95 sur le blanc, et la borne casse ;
   - à couverture égale au banc, la borne tient à 86.

   Si l'auteur tient à la lettre, il reste deux voies : baisser la force, ou
   donner au verre un ruban plus étroit que celui du galet noir. Dans le second
   cas, la découpe GDI ne le suivrait plus. Et la dérivation de la force sur la
   couverture exacte, qui pourrait la desserrer, n'est pas refaite.
6. ~~**Aucun œil n'a jugé le verre** sur le bureau réel.~~ Le pas 3 capture
   l'îlot réel tel que Windows le compose, et en fait une planche. L'intégrateur
   l'a regardée ; l'auteur ne l'a pas encore jugée (pas 3 ci-dessous).
6. Toujours non mesurés depuis le pas 1 : plusieurs moniteurs, une sortie
   tournée, le HDR, une perte d'accès réelle.

### Comment le reprouver

```
cargo build --release
cd recherche/verre-taille && cargo build --release --bin mesure && ./target/release/mesure.exe integration && cd ../..
cp target/release/veilleuse.exe <ailleurs>/v.exe
python outils/verre-contre-banc.py <ailleurs>/v.exe
VEILLEUSE_ADAPTATEUR=intel python outils/verre-contre-banc.py <ailleurs>/v.exe <dossier temporaire>
timeout 120 <ailleurs>/v.exe --releve-bureau=captures/integration/pas2-verre-sain
python outils/verre-en-direct.py <ailleurs>/v.exe captures/integration/pas2-verre-sain
VEILLEUSE_ADAPTATEUR=intel python outils/verre-en-direct.py <ailleurs>/v.exe captures/integration/pas2-verre-sain
timeout 120 <ailleurs>/v.exe --releve-bureau=<dossier> --epreuve-boucle   (puis --epreuve-repli, --epreuve-sans-sonde)
python outils/borne-du-bord.py captures/integration/pas2-app <rendus Intel de verre-contre-banc>
```

L'épreuve de la borne n'est pas un drapeau. On porte `arbitree::FORCE` à 0,30
dans `src/galet.rs`, on construit, on copie le binaire, on rend les fonds sur
l'Intel, puis on restaure. Après restauration, `git diff -- src` était vide.

**Refait par un second chemin, depuis un binaire copié**, avant le commit final
du pas :

- `verre-contre-banc.py` sur les deux adaptateurs : intérieur ≤ 1 sur la NVIDIA,
  ≤ 3 sur l'Intel, témoin à 2 900 ; *sur l'Intel, le verdict de l'outil, à la
  tolérance d'un arrondi, tombe donc à FAUX, par le seul bureau rayé déployé.
  Cette ligne ne le disait pas* ;
- le relevé en direct : **14 sur 14**, 3 pertes de focus ignorées ;
- l'horloge et le galet, contre un binaire **reconstruit** depuis `cc8abeb` dans
  un arbre de travail séparé :
  - horloge : 0, contre un témoin à 10 018 ;
  - galet déployé : 0, contre un témoin à 370 ;
  - galet replié : 0, contre un témoin à 2 942 ;
  - mi-course à 1,25 : 0.

---

## Pas 3 — l'œil sur l'îlot réel, et la remise en ordre : fait

**Ce qui est entré.**

- **`--oeil=dossier`** (`src/oeil.rs`) : l'application ouvre ses vraies
  fenêtres et pose l'îlot, contenu montré, au centre de l'écran, sur la rangée
  des chiffres. Elle le capture **tel que Windows le compose** dans neuf scènes :
  - trois fonds : la nuit de l'horloge, un blanc pur, l'aplat `#0078D4` ;
  - trois formes : replié, mi-course (p = 0,5, figée) et déployé.

  Elle écrit `oeil.txt`, trois images par scène (`dessous`, `compose`,
  `dessous-2`) plus le fond du verre, puis quitte d'elle-même, avec un fil de
  garde à 150 s.
- **`src/toile.rs`** : le fond connu, une fenêtre GDI plein écran peinte par sa
  brosse, à l'octet exact, sous l'îlot et au-dessus de l'horloge.
- **`Ilot::p_fige`** : la fenêtre peint la géométrie des ressorts à p donné,
  sans découpe comme pendant le mouvement. Vaut `None` dans l'application.
- **`outils/planche-oeil.py`** : les neuf images `compose` en une planche de
  trois fonds sur trois formes, `planche.png`.
- **Retiré** : `src/verre.rs`, `--verre`, `--verre-indisponible` et l'écoute de
  `WM_POWERBROADCAST`. Voir plus bas.

**Pourquoi trois images par scène.** Pour capturer ce que l'auteur voit d'une
fenêtre exclue. Le verre est posé puis figé, l'exclusion
levée le temps d'une image, et quatre égalités à zéro disent que l'image est
jugeable. Sinon, la scène est recommencée, trois fois au plus.

### Les verdicts

Premier lancement et lancement final (binaire final) : **43 sur 43**
(`captures/integration/pas3-oeil`).

| verdict, par scène | témoin, réponse connue | lu | épreuve, dans son propre lancement |
|---|---|---|---|
| exclusion posée | affinité 17 | 17 | — |
| fond stable et porté par le verre (9) | 0 + 0 + 0 pixels changés | 0 partout | `fond` : la toile passe à 254 entre `compose` et `dessous-2` sur « blanc-déployé » : 193 200 px, trois tentatives, FAUX. **42 sur 43**, le seul prévu |
| l'image composée montre l'îlot (9) | à couverture pleine, `compose` ≠ `dessous` | 4 480 sur 4 550 (nuit replié) à 72 672 sur 72 672 | `exclu` : l'exclusion n'est pas levée, 0 partout, FAUX ×9 |
| hors du galet, le bureau (9) | dans la fenêtre, là où la couverture est nulle : 0 | 0 sur 924 à 69 302 | `hors` : zone étendue au ruban, 275 à 1 592 px, FAUX ×9. **34 sur 43** |
| verre posé (9) | un fond était téléversé au moment de `compose` | vrai | `noir` : capture fermée juste avant, FAUX ×9 |
| le cœur lit la valeur calculée (6 aplats) | `coeur_attendu` : 50 sur le blanc, (0, 23, 46) sur le bleu, à 1 niveau près | exact, en valeur la plus fréquente | `noir` : lit 0, FAUX ×6 (**28 sur 43**) ; `exclu` : lit le fond, FAUX ×6 (**28 sur 43**) |

Chaque compte a été écrit avant de lancer, et chaque épreuve fait tomber
exactement les verdicts prévus et rien d'autre.
Les épreuves sont des drapeaux :
`--epreuve-oeil=noir|exclu|hors|fond`. Le binaire éprouvé est donc le binaire
sain.

**Le cœur, refait par un autre chemin.** Il a été recalculé en Python depuis les
seuls paramètres de `DECISION.md` : vibrance 1,35 en linéaire, teinte 0,80 sur
les octets, 1 − r0 de N-BK7. Le calcul redonne 50 et (0, 23, 46). Les pixels
relus dans les PNG par un autre décodeur sont ces mêmes octets.

### Rien d'autre n'a bougé

Le binaire final est comparé au binaire **`852a751`**, reconstruit dans un arbre
de travail séparé (depuis retiré), avec `comparer-captures.py` :

| capture | même réglage | témoin |
|---|---|---|
| horloge `--heure=03:14:15` | 0 | 10 018 (03:14:16) |
| îlot déployé `--restant=41:27` | 0 | 370 (41:28) |
| îlot replié `--p=0` | 0 | 3 067 (contre déployé 41:27) |
| îlot déployé à 1,25 | 0 | — |
| mi-course p = 0,6 à 1,25 | 0 | — |
| mi-course p = 0,5 à 1,25 | 0 | 3 796 (contre 0,6) |

Le relevé du pas 2, relancé sur le binaire final, rend **14 sur 14**.
Les essais unitaires passent tous,
46 sur 46, dont les deux de l'œil.

### Ce que l'œil voit, dit par l'intégrateur

La planche est `captures/integration/pas3-oeil/planche.png`. Ce qui suit
décrit ce qu'on y voit. Ce n'est pas un jugement de goût, qui revient à
l'auteur.

- **Sur la nuit, déployé**, les chiffres blancs passent sous le verre en masses
  grises floues. Les contre-formes noires des chiffres deviennent des taches
  sombres sous le temps restant, qui reste net et lisible. Le bord haut
  s'allume par segments, là où un trait blanc passe dessous. C'est le battement
  de 79,2 niveaux que `DECISION.md` mesure sur la rangée des chiffres ; il se
  voit.
- **Sur la nuit, replié**, c'est une pastille gris foncé à cheval sur la
  jonction de deux chiffres.
- **Sur le blanc et le bleu**, c'est un aplat sombre uniforme (50 ; bleu nuit
  0, 23, 46), bordé d'un liseré plus clair d'un ou deux pixels. Un aplat n'a
  rien à réfracter : le verre ne s'y distingue pas d'une pastille teintée. Il ne
  se lit comme du verre que sur un fond structuré.
- **À mi-course**, le bord est flou sur 4,19 px physiques. C'est le ruban de
  mouvement du §6 de `DESIGN.md` (1 px plus au plus 3 px logiques à la vitesse
  de pointe). Aucun contenu n'est montré à p = 0,5 ; c'est la même géométrie que
  `--capture-ilot --p=0.5`, identique au pixel au binaire d'avant.

### L'ancien verre du système : retiré

Il ne reste pas utile, et voici pourquoi.

- **`--verre` ne posait plus un instrument à côté du produit, il éteignait le
  produit.** La Vitrine ne peignait plus le galet, donc plus de verre taillé,
  et la découpe n'était plus posée.
- **Sa question est close.** Quatre des cinq raisons de `VERRE.md` tombent
  devant le verre taillé (`VERRE-TAILLE.md` §5). Ses mesures restent dans
  `VERRE.md` et ses captures, et son code se relit au commit `852a751`.
- **Ce qui est gardé** : `ne_pas_arrondir`, désormais dans `ilot.rs`, et le
  repli « moins de mouvement ».
- **Ce qui part avec lui** : le repli « transparence indisponible », qui ne
  servait qu'à lui, et l'écoute de l'alimentation, qui ne servait qu'à ce
  repli.
- **L'en-tête qui mentait n'existe plus.** `--verre` et `--verre-indisponible`
  impriment la raison du retrait.

Le verre taillé ne demande rien aux effets de transparence de Windows. Qu'il
tienne quand l'économiseur d'énergie coupe ces effets est **raisonné, pas
mesuré**.

### L'exécutable

`dist/Veilleuse.exe` est reconstruit depuis le binaire final. L'application
n'était pas ouverte, et la copie a les mêmes octets que `target/release`
(SHA-256 ~~`f989e5e0…`~~ `30195e39…`). `dist/` n'est pas suivi par git.

**Pourquoi l'empreinte a changé.** Après les preuves, un commentaire seul a été
corrigé dans `ilot.rs`, puis le binaire relié. L'empreinte a changé. Le binaire
éprouvé (œil, relevé, horloge) et le binaire final diffèrent de **24 octets**
sur 10 738 176 :

- 2 aux octets 249-250, dans l'horodatage de l'en-tête PE ;
- 6 en trois paires vers 9,6 Mo ;
- une suite de 16 octets vers 9,6 Mo, de la taille d'un identifiant de PDB.

C'est la signature d'un lien refait, pas d'un code changé. Cette lecture des
emplacements n'a pas été vérifiée par un outil qui décode le PE. Conséquence
pratique : deux liens d'un même code n'ont jamais la même empreinte sur cette
machine, et l'identité d'un binaire se prouve par ses images, pas par son
SHA-256.

### Comment le reprouver

```
cargo build --release
cargo test --release
cp target/release/veilleuse.exe <ailleurs>/v.exe
timeout 200 <ailleurs>/v.exe --oeil=captures/integration/pas3-oeil
python outils/planche-oeil.py captures/integration/pas3-oeil
timeout 200 <ailleurs>/v.exe --oeil=<dossier> --epreuve-oeil=noir     (puis exclu, hors, fond)
timeout 150 <ailleurs>/v.exe --releve-bureau=<dossier>
```

Chaque lancement de l'œil prend 16 à 18 s. Deux lancements ne tournent jamais en
même temps, puisqu'ils partagent l'écran.

### Ce qui n'est pas mesuré, et ce que la suite doit savoir

1. **L'auteur n'a pas jugé la planche.** La lecture de la borne de `DESIGN.md`
   §8 (pas 2, point 5) reste elle aussi à ratifier.
2. **La mi-course de l'œil est figée.** Le retard d'une trame du fond pendant
   un vrai mouvement n'y est pas, et aucune image d'un vrai mouvement composé
   n'existe.
3. **Les fonds de l'œil sont trois.** Il manque une photographie, un texte, un
   fond qui bouge, et le bureau réel de l'auteur. Le banc en couvre sept, mais
   hors écran.
4. **L'œil est centré sur l'écran principal**, à l'échelle 1,25. Les autres
   échelles et plusieurs moniteurs ne sont pas vus.
5. **Toujours non mesurés** : le coût GPU du verre dans l'application, le coût
   sur batterie, une perte d'accès réelle, le HDR, et la tenue du verre quand
   l'économiseur d'énergie coupe les effets de transparence.
6. **La couche de compatibilité `HIGHDPIAWARE` écrite au pas 1** pour
   `target/release/veilleuse.exe` est toujours au registre. Elle n'a pas été
   retirée, puisque c'est un réglage du système : l'auteur peut le faire.
