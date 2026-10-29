# Le verre taillé — quand on cesse de le demander à Windows

`VERRE.md` conclut que le verre ne convient pas à cet îlot, et cinq mesures le
soutiennent. La conclusion est juste **pour la question posée**, qui était : que
peut-on obtenir du fond translucide de Windows ? Ce document pose l'autre
question, et la réponse change quatre des cinq raisons.

**L'idée.** On n'obtient pas le verre du système : on le fabrique. On capture ce
qui est derrière la fenêtre, on le floute soi-même sur le GPU, on le teinte, et
on le masque par **notre** forme. DWM n'intervient nulle part.

Tout ce qui suit est mesuré sur cette machine, dans `recherche/verre-taille`.

---

## 1. Le verrou, et pourquoi il saute

L'objection immédiate est fatale si elle tient : une fenêtre qui capture ce qui
est derrière elle se capture elle-même, et le verre boucle sur son propre
reflet jusqu'à saturer.

Windows offre la sortie depuis la version 2004 : une fenêtre peut se déclarer
**invisible à la capture**, tout en restant visible à l'œil. Reste à savoir si
cela vaut pour la duplication de bureau, qui est le chemin rapide, ou seulement
pour les captures de fenêtre.

**L'essai, avec son témoin.** Une fenêtre magenta de 480 × 240 posée en
(400, 300), capturée deux fois, sans le drapeau puis avec.

| | pixels magenta vus | premier pixel |
|---|---|---|
| sans exclusion | **115 200** | (400, 300) |
| avec exclusion | **0** | — |

115 200 est exactement 480 × 240, et le premier pixel tombe exactement où la
fenêtre est posée. Le témoin prouve que le test voit ce qu'il doit voir ; le
second essai prouve que l'exclusion est totale. **Aucun bouclage n'est
possible.**

Sans le témoin, un zéro n'aurait rien voulu dire : il se confond avec un test
qui ne mesure rien.

---

## 2. Trois pièges de la duplication, tous payés

Ils ne se voient pas à la lecture du code, et chacun coûte une séance entière.

**Le DPI.** Sans conscience du DPI déclarée **avant** toute fenêtre et tout
DXGI, les coordonnées du bureau reviennent mises à l'échelle — 1536 × 864 —
alors que la texture est en pixels physiques — 1920 × 1080. Les deux nombres
ont l'air corrects séparément, et tout tombe à côté.

**`LastPresentTime`.** `AcquireNextFrame` réussit aussi quand seul le curseur a
bougé, et la texture rendue ne contient alors **aucune image neuve**. Sans ce
contrôle, on lit du hasard.

**`ReleaseFrame`.** Il rend le contenu de la texture indéfini. La copie doit
avoir lieu **avant**. En la faisant après, on lit un noir parfait, moyenne
(0,0 ; 0,0 ; 0,0), et l'on conclut que la duplication ne marche pas sur cette
machine. C'est la faute qui a coûté le plus ici.

Enfin, la duplication ne rend que les **changements**. Un bureau immobile ne
produit rien et l'appel expire, ce qui ressemble à une panne. Tout banc de
mesure a donc besoin d'un agitateur : quelque chose qui bouge pour forcer une
présentation.

---

## 3. Ce qu'on gagne à posséder la chaîne

Une fois la capture acquise, tout le reste est à nous.

**Le flou se fait en espace linéaire.** Les textures sont vues en sRGB, donc
l'échantillonnage décode et l'écriture réencode : la somme gaussienne a lieu où
la lumière s'additionne vraiment. DWM, lui, mélange sur les octets stockés
(`ARCHITECTURE.md` §1). Le nôtre est physiquement juste.

**La teinte, elle, ne doit PAS suivre le flou.** C'est la découverte de ce
chantier, et elle était contre-intuitive. Le flou est une moyenne physique ; la
teinte est un geste de peintre, qui se juge là où l'œil juge, c'est-à-dire sur
les octets encodés. À teinte égale, mélanger en linéaire donne un verre bien
plus clair — donc bien moins lisible.

| teinte noire à 0,80 | résultat sur fond 226 |
|---|---|
| mélangée en linéaire | 109 |
| mélangée sur les octets | 45 |

Séparer les deux espaces n'est pas un détail de réglage : c'est ce qui décide
si le verre est lisible.

**Et la forme est libre**, puisque le masque est notre propre distance signée :
superellipse d'exposant 4, rayon 48, exactement le galet de `DESIGN.md` §6.

---

## 4. La table de décision

Fonds unis aux valeurs mêmes de l'orfèvre — clair 226, sombre 37, écart de
luminance 189 — pour que la comparaison ne mente pas. « Porte » est ce que le
verre laisse passer du fond, en luminance ; « part » le rapporte à l'écart.

| teinte | espace | verre / clair | verre / sombre | porte | part | `#F2EFE9` | `#8E8B86` | `#5E5B57` |
|---|---|---|---|---|---|---|---|---|
| 0,70 | linéaire | 132 | 17 | 115,0 | 60,8 % | 3,26 | 1,10 | 1,81 |
| 0,80 | linéaire | 109 | 12 | 97,0 | 51,3 % | 4,51 | 1,52 | 1,30 |
| 0,90 | linéaire | 78 | 6 | 72,0 | 38,1 % | 7,25 | 2,45 | 1,23 |
| 0,96 | linéaire | 49 | 3 | 46,0 | 24,3 % | 11,34 | 3,83 | 1,93 |
| 0,70 | octets | 68 | 11 | 57,0 | 30,2 % | 8,49 | 2,87 | 1,44 |
| **0,80** | **octets** | **45** | **7** | **38,0** | **20,1 %** | **12,00** | **4,06** | **2,04** |
| 0,86 | octets | 32 | 5 | 27,0 | 14,3 % | 14,20 | 4,80 | 2,41 |
| 0,90 | octets | 23 | 4 | 19,0 | 10,1 % | 15,62 | 5,28 | 2,66 |
| 0,93 | octets | 16 | 3 | 13,0 | 6,9 % | 16,58 | 5,61 | 2,82 |
| 1,00 | — | 0 | 0 | 0,0 | 0 % | 18,30 | 6,19 | 3,11 |

**La ligne qui compte est la sixième.** L'acrylique de DWM, mesuré par
l'orfèvre sur ces mêmes fonds, donne 12,2 de contraste sur le blanc pour une
porte de 10,8. Notre verre teinté à 0,80 sur les octets donne **12,00 de
contraste pour une porte de 38,0**.

> À lisibilité égale, il laisse passer **trois fois et demie plus** du fond.

Et le flou est réel : l'énergie de haute fréquence tombe de 47,32 derrière à
1,02 dans le verre, soit un facteur 46.

---

## 5. Ce que cela change aux cinq raisons de `VERRE.md`

| raison | état |
|---|---|
| 1. il est rectangulaire | **tombe** — le masque est notre superellipse |
| 2. il détruit la palette | **reste, mais devient un cadran** — voir §4 |
| 3. il ne descend jamais à `#000000` | **tombe** — sur fond sombre il atteint 3 |
| 4. il s'éteint à 20 % de batterie | **tombe** — il ne dépend plus de DWM |
| 5. il vient avec du grain | **tombe** — il n'y en a que si on en met |

La raison 3 mérite d'être dite en clair, parce que c'est celle qui touche
l'usage réel. `DESIGN.md` §2 refuse le plancher gris de DWM parce qu'il
**redessine le rectangle du moniteur** dans une chambre sombre. Notre verre
descend à (3, 3, 3) sur fond sombre. La nuit, l'îlot ne dessine plus de cadre.

La raison 2 ne tombe pas et ne tombera pas : laisser passer de la lumière élève
la luminance du fond, et le contraste d'une encre sombre baisse. C'est de la
physique, pas une limite d'interface. Mais elle cesse d'être une pénalité
subie : c'est désormais un réglage, et la table dit exactement ce qu'il coûte.

---

## 6. Ce que ça coûte

| | mesuré |
|---|---|
| le rendu seul, deux flous et la composition | **0,215 ms** |
| l'aller-retour complet du banc | **0,465 ms** |
| budget d'une trame à 144 Hz | 6,94 ms |

**Correction, et elle porte sur le nom plus que sur le nombre.** Ce document a
d'abord annoncé « 0,82 ms de GPU pour deux flous et la composition ». Ce chiffre
comptait aussi l'allocation de quatre textures, le téléversement de la région et
la relecture vers le processeur — dont l'application ne fait **rien** par trame.
Remesuré en séparant les deux, le rendu seul vaut 0,215 ms et l'aller-retour
complet 0,465 ms.

Ni l'un ni l'autre ne reproduit 0,82 ms, et **l'écart n'est pas expliqué**. Il
est dit tel quel plutôt que de retenir celui qui arrange. Ce qui est acquis,
c'est que le coût réel de l'effet est **inférieur** à ce qui était publié.

**Et il est expliqué depuis, par la réalisation du verrier.** Il n'y a rien à
réconcilier entre 0,82, 0,77, 0,465, 0,264 et 0,215 ms : le même binaire
mesurant le même travail rend, d'un lancement à l'autre sur cette machine, une
composition allant de **0,0041 à 0,0369 ms**, soit un facteur **neuf**. L'état
d'horloge du GPU ouvre à lui seul toute la plage des chiffres publiés. Un temps
GPU absolu ne se compare donc pas à un temps GPU absolu publié ailleurs ; seule
une différence **appariée**, mesurée en alternance dans le même lancement, est
reproductible. Voir `RECETTES.md`.

### En régime, sur une boucle à 60 Hz

Le premier banc mesurait surtout **l'attente** d'une présentation, pas du
travail. Celui-ci fait tourner la vraie boucle, en acquisition **non
bloquante** — car attendre une trame du bureau dans la boucle de rendu
reviendrait à caler notre affichage sur l'activité des autres fenêtres.

| | bureau calme | bureau animé à chaque trame | îlot qui se déplace |
|---|---|---|---|
| tours avec image neuve | 1 à 16 % | 100 % | 99 % |
| copie quand il y a du neuf | 0,83 ms | 1,17 ms (p95 3,36) | 1,36 ms (p95 2,45) |
| tour à vide | **0,060 ms** | — | 0,096 ms |
| accès perdus sur 9 s | 0 | 0 | 0 |

**Le total par trame**, rendu compris, va de **0,9 ms** quand rien ne bouge à
**2,2 ms** dans le pire cas. Soit 13 % et 32 % d'une trame à 144 Hz.

Le premier chiffre varie — 1 % à un lancement, 16 % au suivant — et c'est
normal : il dépend de ce qui vit sur le bureau à cet instant. Un bureau
vraiment immobile n'existe pas sur cette machine, l'îlot de l'application y
compte lui-même les secondes.

**Le déplacement demande sa propre règle.** Quand l'îlot bouge, la région
suivie change, et aucun rectangle sale ne peut dire ce qui se trouve sous la
**nouvelle** position. Il faut alors forcer une copie, sans attendre de trame
neuve. S'il n'y en a pas, on garde la précédente : mieux vaut un retard d'une
trame qu'un à-coup dans le déplacement.

### Le filtre sans lequel tout cela serait gaspillé

Sans lui, le banc rapportait **99 % de trames neuves même bureau immobile**.
La raison est instructive : `AcquireNextFrame` se réveille pour **tout
l'écran**. Une horloge qui change de minute dans un coin provoquait la recopie
de nos 483 Ko.

Windows dit pourtant exactement ce qui a bougé, par les rectangles **sales** et
les rectangles de **déplacement**. En ne recopiant que si l'un d'eux croise
notre région :

| bureau immobile | avant | après |
|---|---|---|
| trames recopiées | 99 % | **1 %** |
| coût d'un tour à vide | 0,380 ms | **0,103 ms** |

Et le cas animé détecte toujours 99 %, donc le filtre ne laisse pas passer de
verre périmé.

**La règle de prudence**, qui doit être écrite dans le code : en cas de doute,
la réponse est **oui, ça nous touche**. Un faux positif coûte une copie ; un
faux négatif laisse à l'écran un verre qui ment. Les deux erreurs n'ont pas le
même prix.

---

## 7. Ce que je n'ai pas mesuré, et qu'il faut savoir

Rien de ce qui suit n'est un obstacle connu. Ce sont des trous, et les nommer
vaut mieux que de les laisser se découvrir en marche.

- **Les pannes de la duplication.** Elle perd son accès au changement de mode,
  sur le bureau sécurisé, et devant une application en plein écran exclusif. Il
  faut la rouvrir, et savoir quoi afficher pendant ce temps.
- **Le contenu protégé.** Une vidéo sous protection rend du noir. Acceptable,
  mais à dire.
- **La consommation.** Sur batterie, garder une duplication ouverte a un coût
  qui n'a pas été chiffré.
- **Le réveil parasite.** Depuis Windows 11 24H2, `AcquireNextFrame` se réveille
  aussi quand la fenêtre exclue se met à jour, même si son contenu n'est pas
  capturé. Du travail pour rien, à éviter.
- **Le repli sans alpha.** Sur un adaptateur sans `Inherit`, on peut peindre
  hors du galet les pixels capturés eux-mêmes, ce qui donne une transparence
  parfaite tant que le fond ne bouge pas. Non essayé.

---

## 8. Ce que j'en conclus

Le verre en forme de galet est **possible**, il est **plus lisible à porte
égale** que celui du système, il **atteint le noir** la nuit, et il ne s'éteint
pas tout seul. Ce qu'il coûte est connu : **0,264 ms de GPU** pour les trois
passes, et un contraste qui se règle au lieu de se subir.

**Ce chiffre a été corrigé deux fois, et il faut le dire.** Ce paragraphe a
d'abord annoncé 0,82 ms — chiffre que le §6 avait déjà retiré sans que le §8
suive, ce qui est la pire forme d'erreur : une correction publiée à un endroit
et démentie à un autre. Le §6 l'a remplacé par 0,215 ms, mesuré à l'horloge du
**processeur** autour de `submit` et `poll`. Le banc du métrologue le mesure
aujourd'hui à l'horodateur du **GPU**, passe par passe, et trouve 0,264 ms —
dont 0,008 ms seulement pour la composition, le reste étant les deux flous.
Les deux bancs n'ont pas la même géométrie et je ne les ai pas alignés :
l'écart entre 0,215 et 0,264 est un trou nommé, pas une réconciliation.
Voir `MESURER-LE-VERRE.md` §6.

Le régime continu, qui était la vraie inconnue, est mesuré : **0,9 ms par
trame la nuit**, accès jamais perdu sur la durée de l'essai.

Ce qui reste à faire n'est pas une question ouverte, c'est du travail : traiter
les pannes de la duplication, et porter la chaîne dans l'application.

---

## 9. Dans l'application — le pas 1 de l'intégration

La capture est portée dans `src/bureau.rs`, sans l'apparence : le galet reste
noir opaque. Tout ce qui suit est lu dans l'application réelle par
`--releve-bureau`, relevé commité dans `captures/integration/pas1-sain`, et
détaillé dans `INTEGRATION.md`.

| ce que ce document affirmait | dans l'application |
|---|---|
| §1 exclusion : 115 200 px sans, 0 avec (fenêtre GDI) | fenêtre wgpu à alpha par pixel : **73 075 px sans, 0 avec, 0 rétablie** |
| §2 le DPI se déclare avant toute fenêtre | déclarée dans `main` avant la boucle ; coordonnées 1920 × 1080 = texture 1920 × 1080 ; épreuve inconsciente : 1536 × 864, le témoin tombe |
| §6 tour à vide 0,060 ms, bureau calme | 0,049 ms médian, galet replié, un lancement |
| §6 déplacement : on garde la précédente | au plus 1 tic d'attente, 2 px par tic |
| §7 **le réveil parasite**, non mesuré | **mesuré** : l'îlot exclu qui se repeint produit des rectangles sales égaux à sa propre fenêtre, 114 copies sur 248 tours |
| §7 **les pannes**, non mesurées | réouverture exercée en fermant la duplication à la main : 0,131 s. Une perte d'accès **réelle** n'a pas été provoquée |

Deux choses que ce document ne savait pas. La duplication tourne sur la NVIDIA
RTX 4060 et wgpu sur l'Intel UHD : la relecture par le processeur est
obligatoire. Et des rectangles sales de l'écran entier arrivent parfois, ce qui
annule le filtre ; leur cause n'est pas isolée.

---

## 10. Dans l'application — le pas 2 de l'intégration

Le verre de la direction ARBITREE est peint dans le galet. Tout ce qui suit est
détaillé, avec témoins et épreuves, dans `INTEGRATION.md`.

| ce que ce document affirmait | dans l'application |
|---|---|
| §3 flou en linéaire, teinte sur les octets | porté tel quel ; l'intérieur du galet retombe sur le banc à 1 niveau près sur la NVIDIA, sept fonds, déployé et replié |
| §3 la forme est notre distance signée | la couverture est celle du galet noir, et Windows compose le bord en alpha prémultiplié sur les octets : le ruban s'écarte du banc jusqu'à 21 niveaux, cause prouvée |
| §6 le flou coûte 0,215 à 0,264 ms | **non mesuré dans l'application**. Le flou n'y est refait que pour un fond dont les octets ont changé |
| §6 la marge de 64 px | ~~64 px physiques~~ : 64 px **logiques**, 80 physiques à 1,25, parce que sigma suit l'échelle (22,5 px, noyau 68) |
| §7 le réveil parasite | coupé par la comparaison des octets : 2 trames au repos contre 122 sans elle |
| §7 les pannes | capture fermée et empêchée : le galet redevient noir, 0 pixel non noir sur 72 672, contre 72 664 pour une image figée |
| `DECISION.md` : la borne du premier pixel, 86 et 90 au banc | à couverture égale au banc (alpha ≥ 254) : **86**, elle tient ; **à la lettre : 95**, elle casse sur 16 pixels de coin que le ruban exact ne couvre qu'à 96 %. Le verre de ces pixels lit 88,5. La lecture est à ratifier |
| §8 « porter la chaîne dans l'application » | fait, à une réserve près : le rendu de la fenêtre est celui de l'Intel UHD, qui s'écarte du banc d'au plus 3 niveaux à l'intérieur |

---

## 11. Dans l'application — le pas 3 de l'intégration

L'îlot réel est désormais jugeable tel que l'auteur le voit, et l'ancien verre du
système est retiré. Le détail, avec témoins et épreuves, est dans
`INTEGRATION.md`, pas 3.

| ce qui manquait | ce qui existe |
|---|---|
| `DECISION.md` : « aucun œil n'a regardé une planche de l'ARBITREE » | `--oeil=dossier` capture l'image que Windows **compose**, îlot réel posé sur la rangée des chiffres : nuit, blanc pur et `#0078D4`, croisés avec replié, mi-course (p = 0,5, figée) et déployé. `outils/planche-oeil.py` en fait une planche. **43 verdicts sur 43**, et chacune des quatre épreuves fait tomber exactement les verdicts prévus |
| le cœur du galet sur un aplat, jamais relu à l'écran | calculé sur le processeur depuis la formule, sans le GPU : **50** sur le blanc, **(0, 23, 46)** sur le bleu. Il est lu à l'identique au pixel près dans les six scènes sur aplat, et le même calcul refait à part depuis les paramètres de `DECISION.md` redonne les mêmes octets |

**L'ancien verre du système est retiré** : `src/verre.rs`, `--verre`,
`--verre-indisponible`, et l'écoute de l'alimentation qui ne servait qu'à lui.
Voici pourquoi.

- **Il n'était plus un instrument neutre.** Allumé, il ne posait pas le fond du
  système *à côté* du verre taillé, il le **remplaçait** : la Vitrine ne
  peignait plus le galet, et la découpe n'était plus posée.
- **Il n'a plus de question à servir.** Quatre des cinq raisons de `VERRE.md`
  tombent devant le verre taillé (§5). La cinquième, la palette, est un cadran
  que la table du §4 règle.
- **Ses mesures sont conservées** : dans `VERRE.md`, dans les captures, et dans
  le code, au commit `852a751`.
- **Le seul appel encore utile est gardé.** `ne_pas_arrondir` vit maintenant
  dans `ilot.rs`.
- **Rien n'a bougé** : l'horloge, le galet déployé et replié, et la mi-course à
  1,25 sont identiques au pixel au binaire `852a751`, et le relevé du pas 2 rend
  toujours 14 sur 14.
