# Veilleuse

Horloge à volets plein écran et îlot de concentration, pour lire la nuit. Rust
natif de bout en bout — `winit`, `wgpu`, `cosmic-text`, `glyphon` — sans couche
web : le rendu est écrit. `DESIGN.md` fait loi, `ARCHITECTURE.md` mesure.

## Lancer en développement

    cargo run                  # horloge plein écran + îlot
    cargo run -- --fenetre     # horloge en fenêtre, pour regarder de près
    cargo run -- --trace       # précision du réveil, trames par bascule

## Produire l'exécutable

    cargo build --release
    copy target\release\veilleuse.exe dist\Veilleuse.exe

Il est dans `dist\Veilleuse.exe`. Les polices sont dans le binaire ; il ne lui
faut que son fichier de blocs, qu'il écrit lui-même au premier lancement à côté
de lui, et il part depuis n'importe quel dossier de travail. Si la copie est
refusée, c'est que Veilleuse est ouverte : la fermer d'abord.

## Raccourcis

`F11` plein écran de l'horloge, et le retour · `Échap` replie l'îlot quand il a
le focus, sinon quitte · `Q` quitte · ~~`Ctrl+Alt+Maj+G`, pour tout le système
tant que l'îlot est là : le galet devient verre, ou le verre galet~~ **retiré : un
triple clic sur le galet replié fait la même chose, et n'ôte rien au reste du
système** (`GESTE-TRIPLE-CLIC.md`) · l'îlot ayant le focus, `Tab` et `Maj+Tab`
parcourent PAUSE, SUIVANT et VERRE, `Entrée` ou `Espace` agit. L'horloge laisse
passer les clics : elle perd le focus au premier clic ailleurs et les raccourcis
se taisent jusqu'à ce qu'une des deux fenêtres le reprenne.

## L'îlot

L'îlot n'est là que quand une autre fenêtre que l'horloge est devant ; il se
retire aussi devant une application en plein écran, sauf pour dire la fin d'une
lecture. Clic sur le galet : l'horloge revient devant. **Triple clic sur le galet
replié** : la matière change — c'est le geste qui a remplacé `Ctrl+Alt+Maj+G`.
Les deux se partagent le même clic, et c'est le triple clic qui décide : tant
que Windows peut encore compter un clic de plus (`GetDoubleClickTime`, 500 ms
ici) et que le pointeur reste dans son rectangle de 4 px, l'horloge attend. Elle
revient dès que la main bouge, et au plus tard une demi-seconde après le clic.
Sans verre possible — transparence coupée, économiseur de batterie, thème de
contraste — il n'y a rien à basculer, et le clic rend l'horloge à l'instant.
Appui maintenu sur le galet (le délai de survol de Windows, 400 ms) : il se
déploie. Clic sur *PAUSE* ou *SUIVANT* : l'action agit et l'îlot reste ouvert ;
sur *VERRE* ou *GALET* : la matière change, et le choix est retenu dans
`%LOCALAPPDATA%\Veilleuse\matiere.txt` (le galet est opaque par défaut). Clic
ailleurs, ou `Échap`, ou le pointeur parti : il se replie. Glisser le galet le
déplace ; sa position est retenue, écran par écran, dans
`%LOCALAPPDATA%\Veilleuse\ilot.txt` — effacer ce fichier le remet au centre.
La taille du texte de Windows, le thème de contraste, la transparence coupée
et l'économiseur de batterie sont suivis.

## L'icône

Un seul fichier, `assets\veilleuse.ico`, embarqué en ressource par `build.rs`
sous l'identifiant `1`. Il porte **quatorze tailles dessinées ou rendues une à
une** — 16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256 —, l'union
exacte de ce que la table *Icon scaling* de Microsoft demande. La même ressource
sert l'icône du fichier, l'icône de fenêtre et l'icône grande, et `LoadImageW`
sert la taille demandée aux quatorze sans en redimensionner aucune.

~~Aucune n'est laissée au redimensionnement de Windows.~~ Une surface fait
exception, et elle est mesurée : **le bouton de la barre des tâches de
Windows 11** ne dessine pas les icônes de fenêtre et met l'icône du fichier à
l'échelle de son bouton (`INTEGRATION-ICONE.md`, « En plus des douze pas »).

Ce qu'elle montre est le galet de verre de l'application, rendu par le nuanceur
de l'application, sur un sol `#7C7974` — la seule valeur du projet qui ne soit
pas à l'écran (`DESIGN.md` §2 et §8). L'horloge a son bouton dans la barre des
tâches ; l'îlot reste hors barre.

Pour refaire le fichier après un changement du nuanceur du galet :

    python captures\integration-icone\pas2-le-sol-et-la-largeur.py
    python captures\integration-icone\pas6-le-fichier.py
    cargo build --release

`INTEGRATION-ICONE.md` dit ce qui est prouvé, comment, et avec quels témoins.
Un avertissement qui a coûté un essai : **GDI+ (`System.Drawing.Icon`) ne sait
pas lire une entrée PNG d'un `.ico`** — il sert la 96 à qui demande la 256.
Vérifier par `LoadImageW`, jamais par GDI+.

## Éditer les blocs

Le fichier `veilleuse.json`, **à côté de l'exécutable**. Durées en minutes,
`pages` facultatif, relu au lancement et pas en cours de nuit. Un fichier fautif
ne bloque pas le démarrage : le programme par défaut sert, et la raison — avec
sa ligne — est écrite dans `veilleuse.log`, juste à côté. Ce journal disparaît
au premier lancement qui relit le fichier sans faute.

    { "nuit": 1,
      "blocs": [ { "lecture": 50, "livre_ferme": 10, "pages": "214 à 236" } ] }

## Juger le rendu

    cargo run -- --capture=captures\a.png --heure=23:25:44 --phase=0.4
    cargo run -- --capture-ilot=captures\b.png --p=0.85 --etat=livre --restant=7:12

`--exposant=` et `--rayon=` imposent la superellipse du coin, pour juger une
valeur côte à côte au lieu de la choisir de mémoire.

`--mouvement-reduit` force le repli que l'îlot prend quand Windows coupe les
animations. Sans lui, la seule façon de voir cette branche serait de changer les
réglages de la machine, qui ne sont pas les nôtres.

## Le verre de l'îlot

L'intérieur du galet est un verre taillé : le bureau capturé sous l'îlot,
flouté, teinté, avec un chant qui réfracte et réfléchit (`DECISION.md`, direction
ARBITREE ; `DESIGN.md` §8). Hors du galet, c'est le vrai bureau. Quand la
capture du bureau manque, l'îlot redevient le galet noir opaque.
`INTEGRATION.md` dit ce qui est prouvé et comment le reprouver.

Pour juger l'îlot **tel qu'on le voit**, et non un rendu hors-écran :

    copy target\release\veilleuse.exe %TEMP%\v.exe
    %TEMP%\v.exe --oeil=captures\integration\pas3-oeil
    python outils\planche-oeil.py captures\integration\pas3-oeil

L'œil ouvre les vraies fenêtres et pose l'îlot sur la rangée des chiffres. Il le
capture tel que Windows le compose, sur la nuit de l'horloge, un blanc pur et un
aplat bleu, replié, à mi-course et déployé. Il écrit ses verdicts dans
`oeil.txt`, puis quitte tout seul en moins de trente secondes. La planche range
les neuf images en trois lignes (les fonds) et trois colonnes (les formes).

Le fond translucide du système (`--verre`) est retiré : le verre taillé l'a
remplacé (`VERRE-TAILLE.md` §11). `VERRE.md` garde ses mesures.

**Lancer l'œil et les relevés depuis une copie de l'exécutable**, jamais
depuis `target\release` : une épreuve DPI lancée de là a fait écrire à Windows
une couche de compatibilité pour ce chemin (`INTEGRATION.md`, pas 1).
