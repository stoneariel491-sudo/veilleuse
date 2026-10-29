# Le triple clic : deux propositions, et la décision

Ce document est écrit **avant** le code, comme le veut la méthode du projet : deux
propositions indépendantes, sous deux angles réellement différents, chacune avec
ses conséquences chiffrées ; puis une décision qui tranche, et qui n'est pas la
moyenne des deux.

---

## 1. Ce que l'auteur a dit, et ce qui ne se rediscute pas

> « ce Ctrl+... est trop long, c'est nul ; on fait simple, un triple clic. »

Le geste qui bascule la matière de l'îlot — galet opaque ↔ verre — devient un
**triple clic**. C'est tranché. Ce qui se décide ici est **comment**, parce que le
triple clic commence par un clic simple et que le clic simple a déjà une action
dans chacun des deux états de l'îlot.

L'action « Verre » / « Galet » de l'îlot déployé reste, quoi qu'il arrive.

---

## 2. L'état des lieux, mesuré

### 2.1 Les gestes d'aujourd'hui (`DESIGN.md` §6, `DESIGN-ILOT.md` §1.3)

| geste | galet replié | îlot déployé |
|---|---|---|
| clic (relâché avant 400 ms, moins de 4 px) | l'horloge revient devant, l'îlot se retire | hors d'une action : se replie ; sur une action : elle agit |
| appui maintenu 400 ms sans bouger | se déploie, le relâchement ne vaut pas clic | — |
| départ du pointeur | — | repli après 400 ms |
| glisser au-delà de 4 px logiques | déplace | déplace |
| `Ctrl+Alt+Maj+G` | la matière bascule | la matière bascule |

### 2.2 Les chiffres du système, lus sur cette machine le 18 septembre 2026

| grandeur | appel | valeur lue | remarque |
|---|---|---|---|
| délai entre deux clics d'un double clic | `GetDoubleClickTime()` | **500 ms** | c'est la seule durée « clics successifs » que Windows publie |
| rectangle du double clic | `GetSystemMetrics(SM_CXDOUBLECLK)` / `SM_CYDOUBLECLK` | **4 × 4 px** | **mesuré : ces deux métriques ne suivent pas le DPI.** `GetSystemMetricsForDpi(SM_CXDOUBLECLK, dpi)` rend 4 à 96, 120, 144 et 192 ppp, et `GetSystemMetrics` rend 4 dans un processus conscient par moniteur dont le DPI système est 120. Le rectangle est donc en **pixels physiques**, jamais logiques |
| délai de l'appui maintenu | `SystemParametersInfoW(SPI_GETMOUSEHOVERTIME)` | **400 ms** | déjà utilisé (`src/systeme.rs`, `delai_de_survol`) |
| seuil de glissement | — | 4 px **logiques** | c'est un chiffre de la charte (`DESIGN.md` §6), pas du système ; il se multiplie par l'échelle, le rectangle du double clic non |

Conséquence directe : `400 < 500`. Un appui tenu au-delà du délai de survol
devient toujours un appui maintenu **avant** que le délai du double clic n'expire.
Les deux gestes ne peuvent donc pas se disputer un même appui.

### 2.3 Les trois murs

**Mur 1 — l'action du premier clic détruit sa propre cible.** Replié, le clic
appelle l'horloge devant (`focus_window`) ; la règle de présence lit le changement
de premier plan et **retire l'îlot**. Les clics 2 et 3 n'ont plus de cible.

**Mur 2 — pendant tout mouvement et tout fondu, la fenêtre est traversante.** La
découpe GDI devance de deux trames ce qui est peint, et le voile du compositeur se
voit dans l'écart ; la charte ne pose donc la découpe qu'au repos, et la fenêtre
laisse passer tous les clics tant que l'avance dépasse un demi-pixel
(`DESIGN.md` §6 ; `DESIGN-ILOT.md` §9.7 et §13 G2). Mesuré et calculé :

| échelle | 60 Hz | 120 Hz | 144 Hz |
|---|---|---|---|
| 1 | 350 ms | 308 ms | 292 ms |
| 1,25 | 350 ms | 317 ms | **306 ms** |
| 1,5 | 367 ms | 325 ms | 319 ms |
| 2 | 383 ms | 342 ms | 333 ms |

Un triple clic humain tient dans 300 à 450 ms. **Tout geste dont le premier clic
met l'îlot en mouvement perd donc ses deuxième et troisième clics** : ils tombent
dans la fenêtre de dessous. Cela condamne l'option « jouer l'action du premier
clic, puis la défaire au troisième » — aussi bien pour le repli du déployé que
pour le retrait du replié. Ce n'est pas une préférence, c'est une mesure.

**Mur 3 — la sémantique.** Replié, le clic dit « j'ai fini, ramène l'horloge ».
Le triple clic dirait « reste, et change de matière ». Faire jouer l'action puis
l'annuler mettrait l'horloge plein écran noire devant pendant ~300 ms avant de la
renvoyer : un éclair plein écran, que la barre du projet interdit.

### 2.4 Ce que coûte le retrait pur et simple du clic replié

Si le clic du galet replié perdait toute action, l'horloge perdrait son chemin de
retour. Mesuré sur le processus vivant (`GetWindowLongPtrW(GWL_EXSTYLE)` de la
fenêtre de l'horloge, 0x40118) : **pas de `WS_EX_TOOLWINDOW`**, et
`WS_EX_APPWINDOW` posé. winit retire le bouton de la barre des tâches par
`ITaskbarList::DeleteTab`, ce qui n'ôte pas la fenêtre d'Alt+Tab. L'horloge reste
donc joignable **au clavier**, et perd son seul chemin **à la souris**. Les mots de
l'auteur — « on peut cliquer dessus juste pour voir le tout réaparaitre
proprement » — portent sur la souris, et ils priment.

---

## 3. Proposition A — le plus petit pas : le triple clic ne vaut que déployé

**L'angle.** Ne toucher ni au galet replié, ni au chemin de retour de l'horloge,
ni au raccourci des états. Le triple clic vit là où l'action du clic est la moins
chère à retarder.

**La règle.**
- Le triple clic ne vaut que sur l'**îlot déployé**, **hors des trois mots**
  d'action (PAUSE / REPRENDRE, SUIVANT, VERRE / GALET).
- Dans cette zone, l'action du clic — le repli — n'est **jamais jouée tout de
  suite** : elle attend la fermeture du compte des clics. Un ou deux clics :
  l'îlot se replie. Trois : la matière bascule et l'îlot reste déployé.
- Le compte se ferme tôt : pointeur hors du rectangle du double clic, appui tenu
  au-delà de 400 ms, glissement, ou 500 ms après le dernier appui.
- Le galet replié, les mots d'action et le retour de l'horloge ne changent pas
  d'une ligne.

**Ce que ça coûte, chiffré.**
- Latence ajoutée : jusqu'à **500 ms** sur « clic hors action → replie », et rien
  ailleurs. Ce repli-là est déjà le moins pressé des quatre chemins de repli : le
  départ du pointeur en attend 400 par construction, et Échap, la perte de focus
  et le départ du pointeur restent instantanés.
- Code touché : le seul `souris_relachee` dans la branche `None` de
  `action_sous`, plus le compte.
- Rien à retirer, rien à déplacer.

**Ce que ça casse.**
1. **Le geste ne sert pas l'auteur.** L'îlot passe la nuit **replié** ; il ne se
   déploie qu'au bout de 400 ms de survol. Pour basculer la matière il faudrait
   donc : poser le pointeur, attendre 400 ms, attendre les 306 ms de traversée,
   puis triple-cliquer — quand le mot « VERRE » est alors sous la main, à un seul
   clic. Un geste qui ne vaut que là où un mot existe déjà ne justifie pas sa
   présence en une phrase, et la charte le retire (`DESIGN.md` §8, dernière
   ligne).
2. **Il prend un geste courant sur une surface qui n'est pas la sienne.** L'îlot
   déployé couvre 320 × 148 px logiques d'onglets et de barre d'adresse (c'est
   l'objection qui a fait changer ce clic, `DESIGN-ILOT.md` §13 G2). Or le triple
   clic est, dans un navigateur ou un traitement de texte, **le geste qui
   sélectionne une ligne ou une adresse**. Poser la matière dessus, c'est
   promettre une surprise.
3. Il laisse `Ctrl+Alt+Maj+G` indispensable pour le cas replié — donc il ne fait
   pas ce que l'auteur demande.

---

## 4. Proposition B — le geste partout, un seul mécanisme

**L'angle.** Un seul compte de clics, une seule règle, valable dans les deux
états, à toutes les échelles et sur tous les écrans. L'action du clic, quelle
qu'elle soit, attend la fermeture du compte.

**La règle.**
- Un compte de clics s'ouvre au premier appui sur l'îlot, replié **ou** déployé.
- Tant qu'il est ouvert, **aucune** action de clic n'est jouée : ni le retour de
  l'horloge, ni le repli, ni les actions des mots.
- Le compte se ferme :
  - **tôt**, dès que le geste ne peut plus être un triple clic : pointeur hors du
    rectangle de 4 × 4 px physiques autour du premier appui, pointeur hors de la
    silhouette, appui tenu au-delà des 400 ms du survol (c'est un appui maintenu),
    glissement au-delà de 4 px logiques, ou l'îlot qui change d'état pour une
    autre raison ;
  - **sinon**, 500 ms après le dernier appui.
- À la fermeture : 1 ou 2 clics → l'action du clic est jouée, telle qu'elle est
  aujourd'hui, dans l'état où l'îlot se trouve. 3 clics → la matière bascule,
  depuis le centre du galet, et l'action du clic n'est jamais jouée.
- Toutes les cotes suivent l'échelle sauf le rectangle du double clic, qui est en
  pixels physiques parce que Windows le publie ainsi (§2.2).

**Ce que ça coûte, chiffré.**
- Latence ajoutée, pire cas, sur **chaque** action de clic de l'îlot : **500 ms**.
  En pratique le compte se ferme dès que la main bouge de plus de 2 px physiques,
  ce qui arrive presque toujours — mais ce « presque toujours » n'est pas mesuré,
  et une main posée sur une souris immobile paie les 500 ms entiers.
- Les mots d'action paient le plus cher : cliquer PAUSE ne mettrait plus le chrono
  en pause avant un demi-quart de seconde. C'est une action dont l'effet doit être
  immédiat, et elle n'a rien à voir avec la matière.
- Code touché : les trois branches de `souris_relachee`, `souris_bougee`,
  `souris_sortie`, `verifier_les_gestes`, `echeance`, et le chemin du geste dans
  `main.rs`.

**Ce que ça casse.**
1. **Un demi-quart de seconde sur PAUSE et SUIVANT**, pour un geste qui n'a pas
   besoin d'eux. Les mots ont déjà leur propre chemin vers la matière ; les
   charger d'un compte est un prix payé pour rien.
2. Le triple clic sur l'îlot déployé garde la surprise du §3, point 2.
3. Le mécanisme est uniforme, mais l'uniformité ne se voit pas : l'auteur ne
   perçoit qu'une latence là où il n'a rien demandé.

---

## 5. La décision

**Le triple clic ne vaut que sur le galet replié. Il ne vaut pas sur l'îlot
déployé. Sur le galet replié, l'action du clic — le retour de l'horloge — n'est
jamais jouée tant que le compte est ouvert ; elle est jouée à sa fermeture s'il
compte un ou deux clics, et jamais s'il en compte trois.**

Ce n'est ni A ni B.

**Contre A** : le geste doit servir là où il n'y a pas de mot à cliquer. Replié,
le galet ne montre que le temps restant ; c'est exactement l'état où le raccourci
servait, et c'est le seul état où un geste nouveau se justifie en une phrase.
Déployé, le mot « VERRE » est à un clic, à 53,6 × 25,6 px : **un deuxième chemin
au même endroit ne se justifie pas**, et il prendrait le triple clic de
sélection de ligne sur 320 × 148 px d'onglets. Le triple clic déployé est donc
retiré, pas retardé.

**Contre B** : l'uniformité coûte 500 ms sur PAUSE et SUIVANT, dont la matière n'a
rien à faire. Un compte qui ne sert qu'à un geste ne s'ouvre que là où ce geste
existe. Et il ne s'ouvre pas non plus quand le verre est impossible
(transparence coupée, économiseur de batterie, thème de contraste, fenêtre sans
alpha par pixel) : dans ces cas le clic replié garde son action **immédiate**,
exactement comme aujourd'hui, puisqu'il n'y a pas de matière à basculer.

**Le prix assumé, dit en clair** : quand le verre est possible, le retour de
l'horloge par un clic sur le galet replié attend la fermeture du compte. Pire cas
**500 ms** ; en pratique, le temps que la main bouge de plus de 2 px physiques.
C'est le seul prix, il porte sur un seul geste, et il est borné par un chiffre du
système et non par un chiffre inventé.

**Pourquoi ce prix plutôt qu'un autre.** Les trois autres sorties ont été pesées
et sont plus chères : jouer puis défaire perd les clics 2 et 3 dans la traversée
(mur 2) et fait passer l'horloge plein écran devant pour rien (mur 3) ; retirer
l'action du clic replié ôte à l'horloge son seul chemin à la souris (§2.4) ;
déplacer l'action sur l'îlot déployé rouvre l'objection G2 que le projet a déjà
payée.

---

## 6. Les questions réglées, une par une

**Le triple clic vaut-il replié, déployé, ou les deux ?**
**Replié seulement.** Déployé, les trois mots gardent leur clic simple, et un clic
hors d'une action replie l'îlot à l'instant, comme aujourd'hui. Rien du déployé ne
change.

**Que devient l'action du premier clic ?**
**Jamais jouée dans cet état tant que le compte est ouvert** — ni annulée, ni
jouée puis défaite. Elle est **en attente**, et la fermeture du compte décide :
un ou deux clics, elle est jouée ; trois, elle ne l'est pas. Quand le verre est
impossible, aucun compte ne s'ouvre et l'action reste immédiate.

**Quel délai entre clics, et d'où vient ce chiffre ?**
`GetDoubleClickTime()` — **500 ms** sur cette machine, lu au lancement et relu à
chaque `WM_SETTINGCHANGE`, comme `SPI_GETMOUSEHOVERTIME` l'est déjà. C'est la
seule durée « clics successifs » que Windows publie, et c'est celle que le système
lui-même applique pour décider d'un double clic. Le délai se mesure **d'appui à
appui**, comme Windows le fait : les deux intervalles du triple clic doivent
tenir chacun sous 500 ms.

**Quelle tolérance de déplacement du pointeur entre les clics ?**
Le **rectangle du double clic** du système, `SM_CXDOUBLECLK` × `SM_CYDOUBLECLK` =
**4 × 4 px**, soit **± 2 px autour du premier appui**, et en **pixels physiques** :
mesuré, ces deux métriques ne suivent pas le DPI (§2.2). Elles ne se multiplient
donc pas par l'échelle, à la différence des 4 px logiques du glissement, qui sont
un chiffre de la charte.

**Que devient `Ctrl+Alt+Maj+G` ?**
**Retiré**, avec tout son code : `RegisterHotKey`, `MOD_NOREPEAT`, la branche
`WM_HOTKEY` de la procédure de fenêtre chaînée, le drapeau atomique, l'option
d'épreuve `--sans-raccourci`, et l'annonce de la sous-ligne. L'auteur l'a jugé :
« c'est nul ». Le garder en second chemin coûterait une combinaison prise à tout
le système pour une matière, ce que `DESIGN-ILOT.md` §12 relevait déjà, et
laisserait deux chemins pour un même geste là où la charte en veut un.

La sous-ligne sous le pointeur, qui était la découverte du raccourci, devient
**« Verre · triple clic sur le galet »** / **« Galet · triple clic sur le galet »**.
Elle reste le seul endroit où le geste s'apprend, avec la ligne « Raccourcis » de
`LISEZ-MOI.md`.

---

## 7. Les états et les cas limites, tranchés

| cas | ce qui se passe |
|---|---|
| verre impossible (transparence coupée, économiseur, contraste, sans alpha par pixel) | aucun compte ne s'ouvre ; le clic replié rend l'horloge **à l'instant**, comme aujourd'hui |
| « réduire les animations » | le compte ne change pas ; la matière change d'un coup, comme aujourd'hui (`DESIGN.md` §6) |
| deuxième appui au-delà de 500 ms | ce n'est plus le même compte : le premier compte s'était déjà fermé et avait rendu l'horloge ; le nouvel appui ouvre un compte neuf |
| deuxième appui à plus de 2 px physiques du premier | le compte se ferme **à l'appui**, l'horloge est rendue, et l'appui ouvre un compte neuf à sa place |
| troisième appui tenu au-delà de 400 ms | c'est un appui maintenu : l'îlot se déploie, le compte est abandonné, la matière ne bascule pas, l'horloge n'est pas rendue |
| glissement pendant le compte | le compte est abandonné, sans action : un glissement n'a jamais valu clic |
| le pointeur quitte la silhouette pendant le compte | le compte se ferme et l'horloge est rendue |
| quatrième, cinquième, sixième clic | trois clics ferment le compte ; le quatrième en ouvre un neuf. Six clics font donc deux bascules |
| l'îlot se retire pendant le compte | le compte est abandonné, sans action |
| échelles 1 / 1,25 / 1,5 / 2 | le délai ne dépend pas de l'échelle ; la tolérance non plus, elle est en pixels physiques |
| plusieurs écrans | idem : les deux grandeurs sont du système, pas de l'écran |

---

## 8. L'épreuve : les témoins, déclarés avant le lancement

Le banc est celui de `RECETTES.md` — il bouge vraiment le curseur avant chaque
clic, sans quoi `SetCursorPos` sur la position courante n'émet aucun mouvement et
l'application rejoue un point périmé. Il est étendu à ces témoins ; le compte
attendu de chacun est écrit ici, avant la mesure.

1. **Un triple clic vrai bascule la matière** : trois appuis à ~150 ms
   d'intervalle, au même point → une ligne « triple clic … la matière bascule » et
   une transformation partie ; **aucune** ligne « l'horloge revient devant ».
   Attendu : 3 essais sur 3.
2. **Un clic simple garde son action d'avant** : un appui court seul → « l'horloge
   revient devant » à la fermeture du compte, et l'îlot se retire. Attendu :
   3 sur 3, et le retard lu entre le relâchement et la ligne doit tenir dans
   [0, 560] ms (500 ms plus un tour de boucle).
3. **Un double clic ne bascule rien** : deux appuis à ~150 ms → « l'horloge
   revient devant », pas de bascule. Attendu : 3 sur 3.
4. **Trois clics trop lents ne basculent rien** : trois appuis à 700 ms
   d'intervalle → trois comptes séparés, aucune bascule. Attendu : 0 bascule.
5. **Trois clics avec le pointeur qui saute au-delà de la tolérance ne basculent
   rien** : trois appuis à ~150 ms, le deuxième et le troisième décalés de 20 px
   physiques → aucune bascule. Attendu : 0 bascule.
6. **Le verre impossible rend son immédiateté au clic** : la même application
   lancée avec `--transparence=coupee` → le clic simple rend l'horloge **sans**
   ouvrir de compte (aucune ligne « compte de »), et le retard entre le
   relâchement et le retour de l'horloge tient sous 120 ms. Attendu : 3 sur 3.
7. **L'épreuve du témoin** : chaque témoin doit pouvoir tomber. La condition de
   chacun est forcée à faux et l'instrument doit tomber ; puis elle est restaurée.

**Ce qui se restaure.** `%LOCALAPPDATA%\Veilleuse\ilot.txt` vaut « 520 400 » suivi
de la ligne de l'écran, et `matiere.txt` est **absent** (galet opaque). Les deux
sont sauvegardés avant, restaurés à l'identique après, et la restauration est
vérifiée.

**D'où se lance l'exécutable.** D'une copie hors du dépôt,
`%USERPROFILE%\dev\veilleuse-bin\<empreinte>\`, avec le `veilleuse.json` de
`dist/`. Jamais de `target/release`, qui porte une couche de compatibilité
d'échelle.

---

## 9. Ce que le banc a mesuré

Banc : `captures/triple-clic/triple-clic.py`, lancé depuis
`%USERPROFILE%\dev\veilleuse-bin\676bd7fb\veilleuse.exe` (copie hors du dépôt,
avec le `veilleuse.json` de `dist/`), le 18 septembre 2026. Écran 1920 × 1080 à
l'échelle 1,25 et 144 Hz, îlot replié à (760, 8), point d'appui (960, 30) px
physiques. `ilot.txt` et `matiere.txt` sauvegardés avant et restaurés **à
l'octet** après, la restauration vérifiée par empreinte.

**Le compte de référence : six témoins sur six.**

| témoin | mesure |
|---|---|
| 1. un triple clic vrai bascule la matière | 3 sur 3 : **330 à 341 ms** du premier appui, écart 0 px, transformation partie (0 ms vers le galet opaque, 103 à 125 ms vers le verre, le temps de la première image capturée), et **aucune** horloge rendue |
| 2. un clic simple garde son action d'avant | 3 sur 3 : l'horloge revient **475 à 483 ms** après le relâchement — le délai du système moins la tenue du clic, plus un tour de boucle — et le journal dit « compte de 1 clic(s) fermé après 500 ms (le délai du double clic est passé) » |
| 3. un double clic ne bascule rien | 3 sur 3 : « compte de 2 clic(s) fermé après 652 à 660 ms » (150 ms entre les appuis, plus les 500), l'horloge rendue, aucune bascule |
| 4. trois clics à 700 ms ne basculent rien | aucune bascule : chaque compte a échu avant le suivant |
| 5. trois clics décalés de 20 px ne basculent rien | aucune bascule : le compte se ferme « le pointeur sort du rectangle du double clic », 153 ms après le premier appui |
| 6. le verre impossible rend son immédiateté au clic | 3 sur 3 avec `--transparence=coupee` : l'horloge revient **4 à 10 ms** après le relâchement, **zéro** ligne de compte |

**Chaque témoin est tombé au moins une fois**, sa condition forcée à faux :

| épreuve | ce qui est forcé | ce qui tombe |
|---|---|---|
| `--epreuve=retard` | la borne du témoin 2 passe de 560 à 50 ms | témoin 2 FAUX (477 à 482 ms lus), les cinq autres VRAI |
| `--epreuve=immediat` | le témoin 6 est joué **sans** `--transparence=coupee` | témoin 6 FAUX (477 à 485 ms, une ligne de compte), les cinq autres VRAI |
| `--epreuve=lent` | les trois clics du témoin 4 passent de 700 à 150 ms | témoin 4 FAUX (bascule vraie), les cinq autres VRAI |
| `--epreuve=saut` | le saut du témoin 5 passe de 20 à 0 px | témoin 5 FAUX (bascule vraie), les cinq autres VRAI |
| le binaire d'avant, `49d06073` | le triple clic n'existe pas | témoins 1, 2 et 3 FAUX ; 4, 5 et 6 VRAI, faute de quoi que ce soit à basculer |

**Le banc refuse de conclure** quand il n'a pas eu lieu : il vérifie qu'il tient
le premier plan, que le curseur a vraiment atterri où il l'a posé, que **l'îlot a
vu au moins un appui**, et que le pointeur réel n'est pas sorti du rectangle en
cours de geste. Sans ces quatre conditions, un témoin « aucune bascule » passe
alors que rien n'a atteint l'îlot — mesuré : le témoin 5 a tenu une fois pour
cette raison, et le témoin 1 est tombé deux fois parce que l'auteur travaillait
sur sa machine (Edge au premier plan, la vraie souris sur le galet). Les essais
non concluants sont rejoués trois fois, puis le témoin est dit NON CONCLUANT,
jamais FAUX.

## 10. Ce qui pourrait être faux

- **« La main bouge presque toujours » n'est pas mesuré.** C'est l'argument qui
  rend la latence de 500 ms supportable en usage, et il ne repose sur aucune
  mesure : le banc, lui, tient le curseur immobile et paie donc les 500 ms
  entiers. Si l'auteur trouve le retour de l'horloge lent, la sortie honnête n'est
  pas de raccourcir 500 ms — ce serait un chiffre inventé — mais de rouvrir le
  choix de l'état où vit le triple clic.
- **`GetDoubleClickTime` est réglable par l'utilisateur** (panneau de la souris,
  de 200 à 900 ms). Un réglage long rallonge d'autant l'attente du retour de
  l'horloge. C'est le prix d'un chiffre du système plutôt que d'un chiffre à nous,
  et il est assumé.
- **Le rectangle de 4 px physiques est serré à haute densité.** À l'échelle 2, il
  vaut 2 px logiques : une main tremblante peut sortir du rectangle entre deux
  clics et casser le triple clic. C'est la règle que Windows applique à ses
  propres doubles clics, donc la main de l'auteur y est déjà accoutumée — mais ce
  n'est pas mesuré sur un écran à l'échelle 2, faute d'en avoir un.
- **Le compte se ferme au départ du pointeur**, or une fenêtre traversante reçoit
  un `WM_MOUSELEAVE` qui n'est pas un départ (`RECETTES.md`). Le galet replié au
  repos n'est pas traversant, donc le cas ne devrait pas se présenter ; s'il se
  présentait, le compte se fermerait trop tôt et le triple clic deviendrait
  intermittent. Le témoin 1, répété, est ce qui le verrait.
- **Le triple clic n'a pas d'équivalent au doigt.** Sur un écran tactile, trois
  touchers brefs au même endroit passent par le même chemin que trois clics ;
  ce n'est pas vérifié, faute d'écran tactile sur cette machine.
