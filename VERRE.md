# Veilleuse — le verre, éprouvé

Tout ce qui suit a été mesuré sur cette machine (Windows 11 Pro 10.0.26200, rustc 1.98.1,
MSVC, winit 0.30.13 / wgpu 30.0.1), dans un dossier d'essai séparé. L'application n'a pas
été touchée. Chaque affirmation est suivie de la sortie du programme qui l'a produite ou de
la capture qui la montre.

Matériel : écran unique 1920 × 1080 à 144 Hz, facteur d'échelle 1,25. Intel UHD Graphics
(intégrée) et NVIDIA GeForce RTX 4060 Laptop (dédiée) — la dualité qui décide déjà de
l'alpha par pixel au §1 d'ARCHITECTURE.md décide aussi du verre.

**La réponse courte est dans le §7, et elle est négative.** Le verre du système existe et
fonctionne, mais il est rectangulaire, il ne descend jamais sous `#212224`, il détruit la
lisibilité de la palette, et il s'éteint tout seul dès que Windows économise la batterie.
Les §8 et §9 — la vibrance et les coins continus — restent entièrement réalisables et sont
mesurés eux aussi.

---

## 0. Comment c'est mesuré

Une fenêtre ne peut pas se relire elle-même : ce qu'elle a dessiné n'est pas ce que l'on
voit. Comme au §1.4 d'ARCHITECTURE.md, on photographie le bureau **après composition**
(`BitBlt` + `GetDIBits`), jamais la texture de la chaîne d'échange.

Deux fenêtres par lancement :

- un **fond** opaque, 760 × 560 à (300, 200), rayé de bandes verticales de 6 px de période,
  découpé en trois zones horizontales calées exactement sur l'îlot : *clair*
  (moyenne mesurée `(227,4 ; 224,4 ; 218,9)`), *saturé* (`(99,8 ; 111,4 ; 120,3)`), *sombre*
  (`(23,0 ; 23,0 ; 28,0)`) ;
- l'**îlot** à éprouver, 320 × 148 à (520, 400), sans décoration, toujours au-dessus, avec un
  témoin blanc opaque de 14 × 14 px en haut à gauche qui prouve à chaque fois que la fenêtre
  est bien là, au bon endroit, et au-dessus du fond.

De chaque rectangle on tire deux nombres : la **moyenne** RVB et **hf**, la différence
moyenne entre un pixel et celui situé trois colonnes plus loin. Les raies ayant 6 px de
période, un décalage de 3 tombe sur la raie opposée : `hf` est maximal sur le motif net et
tombe à zéro dès que quelque chose le floute ou le couvre. Trois issues, qui ne se
confondent jamais :

| moyenne | hf | lecture |
|---|---|---|
| = celle du fond | ≈ 35 à 162 | la fenêtre est **transparente**, sans flou |
| = celle du fond, atténuée | ≈ 0 | la fenêtre est du **verre** |
| = (0, 0, 0) | 0 | la fenêtre est **opaque** |

Neuf programmes, un par combinaison, chacun ouvrant, dessinant 120 trames, photographiant et
sortant. Le dossier d'essai : `%TEMP%\claude\…\scratchpad\verre`. Les captures
citées ici sont dans `captures/verre/` (ce dossier est dans `.gitignore`, comme tout
`captures/`).

---

## 1. `DWMWA_SYSTEMBACKDROP_TYPE` — laquelle des trois valeurs floute vraiment

Attribut 38, posé par `DwmSetWindowAttribute` juste après la création de la fenêtre. Les
trois appels renvoient `hr = 0x00000000` et l'attribut se relit à la valeur demandée : le
système accepte l'attribut sur une fenêtre sans décoration, sans condition préalable.

Fond de référence : `(227,4 ; 224,4 ; 218,9)` hf 35,67 — `(99,8 ; 111,4 ; 120,3)` hf 162,00 —
`(23,0 ; 23,0 ; 28,0)` hf 37,33.

| valeur | clair | saturé | sombre | hf îlot | capture |
|---|---|---|---|---|---|
| 2 — `MAINWINDOW` (mica) | (239,0 ; 244,0 ; 249,0) | (239,1 ; 244,0 ; 249,0) | (239,8 ; 243,6 ; 249,0) | 0,00 | `01-mica.png` |
| **3 — `TRANSIENTWINDOW` (acrylique)** | **(213,0 ; 212,2 ; 207,2)** | **(169,2 ; 176,3 ; 185,0)** | **(145,1 ; 146,1 ; 151,7)** | **0,4 – 0,9** | `02-acrylique.png` |
| 4 — `TABBEDWINDOW` (mica alt) | (205,9 ; 220,9 ; 236,0) | (206,7 ; 220,2 ; 236,0) | (207,7 ; 219,8 ; 236,0) | 0,02 | `03-mica-alt.png` |
| 1 — `NONE` | identique au fond | identique | identique | 35,7 / 162,0 / 37,3 | `04-backdrop-none.png` |

**Seule la valeur 3 est du verre.** Mica (2) et mica alt (4) rendent un aplat *uniforme* : la
moyenne ne bouge pas d'un point selon qu'il y a du blanc, du magenta ou du noir derrière.
Quelle que soit l'origine de cette teinte — Microsoft dit qu'elle est tirée du papier peint,
et rien ici ne le contredit — ce n'est en tout cas pas la fenêtre située derrière.
`01-mica.png` le montre sans ambiguïté : une dalle bleu très pâle posée sur les rayures, qui
n'en laisse rien passer.

L'acrylique (3), lui, suit ce qu'il y a derrière. Modèle linéaire ajusté sur les trois
mesures, en clair de Windows :

```
ilot ≈ 0,332 × fond + 137,5      (ecart maximal sur les trois zones : 1,4 / 255)
```

soit **un tiers du fond, plus une teinte blanche fixe de 137**. C'est beaucoup de blanc.
`DWMWA_USE_IMMERSIVE_DARK_MODE` (attribut 20, valeur 1) change cette teinte, et rien
d'autre — `08-acryl-sombre.png` :

```
ilot ≈ 0,351 × fond + 56,7
```

Un tiers du fond encore, sur un socle gris `#393939`. **La teinte de `DWMSBT` n'est pas
réglable** : il n'y a que ces deux jeux, clair et sombre, et aucun attribut DWM ne les
paramètre.

### `DwmExtendFrameIntoClientArea(−1)` : inutile ici, et nuisible

L'appel réussit (`hr = 0x00000000`) mais **ne change rien au verre** : `05-acryl-cadre.png`
donne (213,2 ; 212,1 ; 207,2) / (169,0 ; 176,4 ; 184,9) / (144,6 ; 145,8 ; 151,1), soit
l'essai 02 au dixième près. Le fond du système atteint la zone cliente d'une fenêtre sans
décoration **sans** qu'on ait à étendre le cadre.

Et il coûte cher. Regarder `05-acryl-cadre.png` :

- Windows dessine **les trois boutons de légende** (réduire, agrandir, fermer) dans notre
  zone cliente ;
- il **arrondit** les coins de la fenêtre ;
- il pose une **ombre portée**, mesurable : le fond juste au-dessus de l'îlot passe de
  (227,4 ; 224,4 ; 218,9) à (221,9 ; 218,9 ; 213,6) — 2,4 % plus sombre sur 30 px.

Trois choses que §8 de DESIGN.md interdit. **Ne pas étendre le cadre.**

### Ce dont l'acrylique a réellement besoin

| on change | résultat | capture |
|---|---|---|
| `with_transparent(false)` au lieu de `true` | **identique** — le verre est là | `09-acryl-fenopaque.png` |
| effacement `Color::BLACK` (alpha 1) au lieu de `TRANSPARENT` | **(0, 0, 0)** partout, verre perdu | `10-acryl-effopaque.png` |
| adaptateur NVIDIA (`alpha_modes = [Opaque]`) | **(0, 0, 0)** partout, verre perdu | `11-acryl-nvidia.png` |

Donc : l'appel `DwmEnableBlurBehindWindow` que winit fait pour `with_transparent(true)` n'est
**pas** nécessaire quand `SYSTEMBACKDROP_TYPE` est posé — l'attribut rend seul la zone
cliente sensible à l'alpha. En revanche les deux conditions du §1.4 d'ARCHITECTURE.md
restent absolues : **la surface doit venir d'un adaptateur dont `caps.alpha_modes` contient
`Inherit`** (ici, uniquement Vulkan + Intel UHD), et **l'effacement doit laisser l'alpha à
zéro**. Sans l'un ou l'autre, il n'y a pas de verre, seulement du noir.

---

## 2. `SetWindowCompositionAttribute` — la voie non documentée

L'export existe dans `user32.dll` sur cette machine (récupéré par `GetProcAddress`, jamais
par `#[link]`) et tous les appels renvoient 1.

| état | teinte | clair | saturé | sombre | hf | capture |
|---|---|---|---|---|---|---|
| 2 `TRANSPARENTGRADIENT` | `0x99000000` | (91,2 ; 89,7 ; 87,2) | (40,1 ; 44,4 ; 48,2) | (9,0 ; 9,0 ; 11,0) | **14,3 / 64,7 / 14,7** | `29-accent2-gradient.png` |
| 3 `BLURBEHIND` | `0x01000000` | (186,2 ; 185,7 ; 181,1) | (94,3 ; 101,3 ; 109,8) | (32,6 ; 33,6 ; 39,0) | 1,5 – 2,3 | `20-accent3-blur.png` |
| **4 `ACRYLICBLURBEHIND`** | `0x01000000` | (185,6 ; 185,1 ; 180,3) | (94,1 ; 101,1 ; 109,5) | (32,6 ; 33,6 ; 39,0) | 1,5 – 2,3 | — |
| **4** | `0x99000000` | (91,5 ; 91,4 ; 89,0) | (56,1 ; 59,5 ; 63,6) | (33,3 ; 33,8 ; 36,3) | 1,2 – 1,4 | `22-accent4-a99.png` |
| **4** | `0xE6000000` | (43,8 ; 43,9 ; 42,4) | (36,8 ; 38,4 ; 40,4) | (33,6 ; 33,9 ; 35,1) | 1,1 – 1,2 | `30-accent4-e6.png` |
| **4** | `0xF2000000` | (36,9 ; 36,5 ; 35,7) | (33,8 ; 35,3 ; 37,0) | (33,5 ; 33,7 ; 34,8) | 1,1 – 1,2 | `31-accent4-f2.png` |
| 4 | `0x40FFFFFF` | (224,5 ; 224,0 ; 219,7) | (156,0 ; 161,9 ; 169,0) | (110,7 ; 111,6 ; 116,5) | 1,4 – 1,8 | `24-accent4-blanc.png` |
| 5 `HOSTBACKDROP` | `0x99000000` | identique au fond | identique | identique | 35,7 / 162,0 | — |

**Elle marche mieux que la voie officielle, sur un point et un seul : la teinte est
réglable.** Le mot de 32 bits est un `0xAABBGGRR` — on choisit la couleur *et* son opacité.
`24-accent4-blanc.png` (teinte blanche) et `22-accent4-a99.png` (teinte noire) sont deux
matières entièrement différentes tirées du même appel.

Le reste se lit aussi :

- l'état 2 n'est **pas** un flou : hf reste à 14,3 et 64,7, c'est-à-dire que les rayures
  traversent. C'est un simple aplat translucide.
- les états 3 et 4 donnent le même résultat au dixième près à teinte nulle. La différence
  entre « flou » et « acrylique » ne se voit que par la teinte : l'état 3 ignore la mienne
  (`0x99000000` donne exactement les nombres de `0x01000000`), l'état 4 l'applique.
- l'état 5 ne fait rien du tout.
- l'acrylique ajoute son **grain** : visible au grossissement dans
  `41-verre-region-coin.png`, un bruit de ±2 niveaux. DESIGN.md §8 interdit « pas de
  matière » ; celle-ci vient avec, on ne la règle pas.

Mêmes deux conditions qu'au §1 : adaptateur annonçant `Inherit`, effacement à alpha zéro.
Avec NVIDIA ou un effacement noir opaque, le résultat est (0, 0, 0).

### Le plancher, qui décide de tout

En poussant la teinte noire : `0x99` (60 %) → sombre 33,3 ; `0xE6` (90 %) → 33,6 ;
`0xF2` (95 %) → 33,5. **La moyenne ne descend jamais sous ≈ (33, 34, 36), soit `#212224`.**
C'est la couche de luminosité de l'acrylique, et aucun réglage ne l'enlève.

DESIGN.md §2 fixe le fond de l'îlot à `#000000` et donne sa raison — « sur OLED l'écran
s'éteint, et tout gris de fond dessinerait le rectangle du moniteur dans la nuit ».
**Sur du verre, `#000000` est inatteignable**, et `#212224` dessine précisément ce rectangle.

---

## 3. La combinaison exacte qui fonctionne

Telle quelle, vérifiée par les essais 22, 30 et 31.

```rust
// --- 1. La fenetre --------------------------------------------------------
// with_transparent n'est PAS requis quand on pose un fond de systeme, mais il
// ne nuit pas et reste necessaire au repli sans verre : on le garde.
let ilot = el.create_window(
    Window::default_attributes()
        .with_inner_size(PhysicalSize::new(320, 148))
        .with_position(PhysicalPosition::new(x, y))
        .with_decorations(false)
        .with_resizable(false)
        .with_transparent(true)
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_skip_taskbar(true)
        .with_undecorated_shadow(false),
)?;
let hwnd = match ilot.window_handle()?.as_raw() {
    RawWindowHandle::Win32(h) => h.hwnd.get(),
    _ => unreachable!(),
};

// --- 2. Le verre ----------------------------------------------------------
// La voie non documentee : la seule qui laisse choisir la teinte.
#[repr(C)]
struct PolitiqueAccent { etat: u32, drapeaux: u32, couleur: u32, animation: u32 }
#[repr(C)]
struct DonneesComposition { attribut: u32, donnees: *mut c_void, taille: usize }

const WCA_ACCENT_POLICY: u32 = 19;
const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;

// L'export n'est declare dans aucun .lib : #[link] ne le trouverait pas, et son
// absence sur une autre machine doit se lire comme « pas de verre », pas comme
// un plantage. D'ou la recherche a la main, et l'Option qu'elle renvoie.
type FnComposition = unsafe extern "system" fn(isize, *mut DonneesComposition) -> i32;
let f: Option<FnComposition> = unsafe {
    let nom: Vec<u16> = "user32.dll\0".encode_utf16().collect();
    let module = GetModuleHandleW(nom.as_ptr());
    GetProcAddress(module, c"SetWindowCompositionAttribute".as_ptr().cast())
        .map(|p| std::mem::transmute(p))
};
// Teinte 0xAABBGGRR : ici du noir a 90 %. En dessous de 0xE6 le texte ne tient
// plus (§8) ; au-dessus il n'y a plus de fond du tout (§2).
let mut p = PolitiqueAccent {
    etat: ACCENT_ENABLE_ACRYLICBLURBEHIND,
    drapeaux: 0,
    couleur: 0xE600_0000,
    animation: 0,
};
let mut d = DonneesComposition {
    attribut: WCA_ACCENT_POLICY,
    donnees: (&mut p as *mut PolitiqueAccent).cast(),
    taille: std::mem::size_of::<PolitiqueAccent>(),
};
if let Some(f) = f { unsafe { f(hwnd, &mut d) }; }   // renvoie 1

// Ceinture et bretelles : Windows n'arrondit pas les fenetres sans decoration
// (§7), mais le dire coute un appel et ferme la question.
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
const DWMWCP_DONOTROUND: u32 = 1;
unsafe { DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE,
        (&DWMWCP_DONOTROUND as *const u32).cast(), 4) };

// --- 3. La surface --------------------------------------------------------
// Rien de nouveau : ce sont exactement les regles du §1.4 d'ARCHITECTURE.md,
// et sans elles le verre ne se voit pas.
let caps = surface.get_capabilities(&adaptateur);
assert!(caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Inherit));  // sinon : noir
let config = wgpu::SurfaceConfiguration {
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    format: wgpu::TextureFormat::Bgra8UnormSrgb,   // premier annonce sur les six couples
    width, height,
    present_mode: wgpu::PresentMode::Fifo,
    alpha_mode: wgpu::CompositeAlphaMode::Auto,    // jamais Pre/PostMultiplied : erreur de validation
    view_formats: vec![],
    desired_maximum_frame_latency: 2,
    color_space: wgpu::SurfaceColorSpace::Auto,
};

// --- 4. L'effacement ------------------------------------------------------
// LA ligne qui decide. Un effacement noir OPAQUE donne (0,0,0) partout et le
// verre disparait (essai 10).
load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
```

Et si l'on préfère la voie officielle — au prix d'une teinte imposée :

```rust
const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
const DWMSBT_TRANSIENTWINDOW: u32 = 3;   // 2 et 4 tirent du papier peint, pas du fond
unsafe { DwmSetWindowAttribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE,
        (&DWMSBT_TRANSIENTWINDOW as *const u32).cast(), 4) };
// Ne PAS appeler DwmExtendFrameIntoClientArea : inutile ici, et il fait
// apparaitre les boutons de legende (essai 05).
```

---

## 4. Ce que ça coûte

Cadence de présentation, boucle identique, 500 trames, les 167 premières rejetées, trois
lancements de chaque :

```
sans verre        median 6,98 ms (143,2 Hz)   6,98 (143,3)   6,98 (143,3)
accent acrylique  median 6,96 ms (143,8 Hz)   6,98 (143,3)   6,90 (144,8)
DWMSBT acrylique  median 6,97 ms (143,4 Hz)   6,90 (144,9)   6,92 (144,6)
```

Aucune différence. 6,90 à 6,98 ms partout, c'est-à-dire la médiane de 6,98 ms déjà mesurée au
§8 d'ARCHITECTURE.md pour `Fifo` sur un écran à 144 Hz. **Le fond acrylique ne coûte rien à
la cadence de l'application.**

Réserve honnête : cette mesure est celle de *notre* boucle. Le flou est calculé par DWM, dans
son propre processus ; son coût en GPU et en batterie n'a pas été mesuré ici. Le §5
ci-dessous suggère que Windows, lui, le juge coûteux.

---

## 5. Le repli — et l'expérience que la machine a faite toute seule

Je n'ai pas touché aux réglages de Windows. Je n'en ai pas eu besoin : **au milieu de la
campagne d'essais, la batterie est passée sous 20 %, l'économiseur d'énergie de Windows 11
s'est enclenché, et tout le verre est mort d'un coup.** L'état « transparence coupée » s'est
donc présenté de lui-même, et il a été mesuré.

```
DwmGetColorizationColor hr=0x00000000 couleur=0xe3006fc4 OPAQUE=true ;
secteur=0 batterie=18% economiseur=1
composition DWM = true ; transparence (registre) = Some(true) ; animations = true
```

Ce que l'îlot devient dans cet état, mesuré, chaque ligne rejouée deux fois et identique au
dixième près :

| voie | îlot = fenêtre active de son processus | une autre fenêtre du processus activée |
|---|---|---|
| `SetWindowCompositionAttribute` état 4, teinte `0x99000000` | **(0, 0, 0)** — dalle noire opaque | **(227,4 ; 224,4 ; 218,9) hf 35,67** — la fenêtre disparaît |
| `DWMWA_SYSTEMBACKDROP_TYPE` = 3 | **(211, 211, 211)** — dalle grise unie | **idem** — la fenêtre disparaît |

Les deux résultats sont inacceptables, et le second est le pire : une fenêtre invisible dont
seul le texte flotterait au-dessus du livre. C'est exactement la situation de l'îlot, qui
passe la nuit derrière la fenêtre du livre.

La variable n'est pas `GetForegroundWindow` — au premier essai l'îlot n'était pas au premier
plan (une autre application l'était) et la dalle noire était bien là. C'est
`SetForegroundWindow` appelé sur l'**autre fenêtre du même processus** qui fait disparaître
le fond. Je n'ai pas isolé la règle plus finement, et je n'ai pas pu vérifier si la même
dépendance existe quand la transparence est allumée : elle s'était déjà éteinte.

### Comment le détecter

**Le registre ment.** `HKCU\…\Themes\Personalize\EnableTransparency` valait toujours 1
pendant que rien ne fonctionnait. Il ne voit que le réglage manuel, pas la coupure décidée
par l'économiseur d'énergie.

> **Correction du 2026-09-12 — `DwmGetColorizationColor` n'est pas un signal.**
> Le trou du §10 a été comblé en un lancement, machine sur secteur
> (`verre.exe --nom=50-sonde-secteur --accent=4 --teinte=0x99000000`) :
>
> ```
> DwmGetColorizationColor hr=0x00000000 couleur=0xe3006fc4 OPAQUE=true ;
> secteur=1 batterie=22% economiseur=0
> ilot = (91,5 ; 91,4 ; 89,0) / (56,1 ; 59,5 ; 63,6) / (33,3 ; 33,8 ; 36,3) ; hf 1,2 – 1,4
> ```
>
> Les trois moyennes sont celles de la ligne `0x99` du §2 au dixième près : **le
> verre était parfaitement vivant**, et `OPAQUE` valait quand même `true`. Le
> drapeau dit l'opacité de la couleur d'accentuation, pas la disponibilité de la
> composition ; il valait `true` dans les deux états, et la lecture symétrique
> espérée n'existe pas. La fonction ci-dessous, qui exigeait `!opaque`, **aurait
> donc renvoyé faux en permanence** et éteint le verre pour toujours. Elle est
> corrigée : il ne reste que deux signaux, l'économiseur et le registre, et
> `verre.rs` n'en pose pas d'autre. *Depuis le pas 3 de l'intégration,
> `src/verre.rs` n'est plus dans l'application : le verre taillé l'a remplacé
> (`VERRE-TAILLE.md` §11). Il se relit au commit `852a751`.*
>
> C'est la leçon générale du §0 appliquée à elle-même : un drapeau observé dans
> un seul état ne prouve rien, même quand son nom raconte la bonne histoire.

Deux signaux honnêtes, tous deux documentés :

```rust
/// L'economiseur d'energie de Windows 11 coupe la transparence SANS toucher au
/// registre. `economiseur` (SystemStatusFlag) vaut 1 quand il est actif.
#[repr(C)] #[derive(Default)]
pub struct EtatAlimentation {
    pub secteur: u8, pub batterie: u8, pub charge_pourcent: u8,
    pub economiseur: u8, pub secondes_restantes: u32, pub secondes_pleine: u32,
}
#[link(name = "kernel32")]
extern "system" { fn GetSystemPowerStatus(etat: *mut EtatAlimentation) -> i32; }

/// Le verre n'est disponible que si les deux sont d'accord.
fn verre_disponible() -> bool {
    alimentation().economiseur == 0 && transparence_activee().unwrap_or(true)
}
```

`economiseur == 0` reste le seul des deux signaux dont le comportement ait été observé **dans
les deux sens** : allumé pendant la coupure, éteint pendant que le verre vivait. Le registre,
lui, n'a jamais été vu à zéro — c'est un réglage de la machine, et il n'est pas à moi.

Ces changements arrivent en cours de route : l'économiseur s'enclenche à 20 % de batterie,
c'est-à-dire au milieu d'une nuit de lecture. Il faut donc **réinterroger à chaque réveil**,
pas seulement au démarrage, et refaire la surface en conséquence.

### Le repli lui-même est déjà prouvé, et il ne bouge pas

L'îlot actuel — alpha par pixel, galet dessiné par sa distance signée dans `galet.wgsl` —
est **totalement insensible** à tout ceci. Essai `91-galet-alpha-econo`, joué en plein
économiseur d'énergie :

```
DwmGetColorizationColor ... OPAQUE=true ; economiseur=1
ilot = (0,0,0) hf 0,00 sur les trois fonds
```

et `91-galet-alpha-econo-coin.png` montre le coin arrondi, noir, au bord adouci, exactement
comme prévu. Le mécanisme du §1.4 d'ARCHITECTURE.md ne dépend pas des effets de
transparence : il ne dépend que de `Inherit` dans `caps.alpha_modes`. **Rien à réparer, rien
à écrire : le repli est l'application d'aujourd'hui.**

Pour le mouvement, `SPI_GETCLIENTAREAANIMATION` répond déjà (`systeme.rs`) et vaut vrai ici.

---

## 6. Les coins — Windows n'impose rien, mais il peut

`DWMWA_WINDOW_CORNER_PREFERENCE` (attribut 33). Mesure directe : l'îlot est peint en noir
opaque plein cadre, et pour chacune des 24 premières lignes on cherche la première colonne
noire. Un coin carré donne une colonne constante ; un arc donne son profil.

```
aucun appel        [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
DWMWCP_DEFAULT (0) [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
DWMWCP_DONOTROUND  [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
DWMWCP_ROUND   (2) [-1,9,6,4,3,3,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]
DWMWCP_ROUNDSMALL  [-1,5,2,2,2,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]
```

**Sur une fenêtre sans décoration, Windows n'arrondit pas.** Ni par défaut, ni avec
`DEFAULT` explicite : le coin est carré au pixel près. Il n'y a donc **aucune superposition
de courbures** à craindre ; notre superellipse sera seule.

Le système *sait* le faire, si on le lui demande : `ROUND` donne un arc de 9 à 10 px
physiques, soit les 8 px logiques standard de Windows 11 à l'échelle 1,25 ; `ROUNDSMALL`
descend à 5 px. `33-coins-round-coin.png` le montre, et montre aussi ce qui vient avec : un arc de
**cercle** — exactement ce que le §8 cherche à éviter — plus une ombre portée, visible par le
`-1` de la première ligne et par le fond qui s'assombrit de 2,4 % autour de la fenêtre.

Recommandation : poser quand même `DWMWCP_DONOTROUND` (valeur 1). Ça ne change rien sur cette
machine, ça coûte un appel, et ça protège d'une version future de Windows qui changerait
d'avis.

---

## 7. Le mur : le verre est un rectangle, et rien ne le découpe

C'est le résultat qui décide, et il n'était pas prévu.

**Essai décisif.** Verre acrylique posé sur la fenêtre, puis le galet peint par-dessus en
encre noire opaque (distance signée, rayon 28). Si le verre suivait notre forme, les quatre
coins du rectangle montreraient le fond rayé. `40-verre-rect-galetnoir-coin.png`, grossi six
fois : **le coin est rempli de verre**, gris lisse, sans rayures. Le fond du système couvre
tout le rectangle de la fenêtre, et l'alpha de notre nuanceur ne peut rien en retrancher —
l'alpha dit quelle part de *notre encre* couvre le verre, jamais l'inverse.

**Et la découpe de fenêtre ne le rattrape pas.** `SetWindowRgn` avec un
`CreateRoundRectRgn` de rayon 28, la seule découpe que Windows offre :

| essai | ce qu'on voit | capture |
|---|---|---|
| région seule, îlot opaque | galet arrondi, bord **crénelé** (masque à un bit) | `44-region-opaque-coin.png` |
| région + accent acrylique | notre témoin est découpé par l'arc, **le verre reste carré** | `41-verre-region-coin.png` |
| région + `DWMSBT` 3 | idem : le verre reste carré | `42-backdrop3-region-coin.png` |
| région posée **après** l'accent, puis accent redemandé | **le verre meurt** — dalle noire opaque | `45-region-tard-coin.png` |
| région posée après `DWMSBT` 3, puis attribut redemandé | **le verre dégénère** — (211, 211, 211) uni | — |

DWM compose le fond de système comme un visuel séparé, **sous** la surface de
l'application ; la région ne s'applique qu'à la seconde. Il n'existe donc aucun chemin, sur
cette machine, vers un verre en forme de galet :

- l'alpha par pixel ne soustrait pas le verre ;
- la région de fenêtre ne le découpe pas, et posée trop tard elle le détruit ;
- il n'existe pas de région par pixel sous Windows.

**Le verre du système est un rectangle à coins carrés, ou rien.** DESIGN.md §6 demande un
galet de rayon 18 replié et 28 déployé — « la seule forme qui ne dessine pas de cadre ». Les
deux sont incompatibles.

---

## 8. La vibrance — ce que la palette devient sur le verre

Contraste WCAG des trois encres de DESIGN.md contre le verre **mesuré** aux §1 et §2, sur les
trois fonds. (Les encres n'ont pas été photographiées sur du verre vivant — l'économiseur
d'énergie l'avait déjà coupé quand j'y suis arrivé — mais les valeurs du verre, elles, sont
des mesures, et le contraste s'en déduit exactement.)

| matière | fond | `#F2EFE9` | `#8E8B86` | `#5E5B57` |
|---|---|---|---|---|
| **îlot noir opaque (aujourd'hui)** | tous | **18,30** | **6,19** | **3,11** |
| accent, noir 95 % (`0xF2`) | clair / saturé / sombre | 13,44 / 13,68 / 13,91 | 4,54 / 4,62 / 4,70 | 2,28 / 2,32 / 2,36 |
| accent, noir 90 % (`0xE6`) | clair / saturé / sombre | 12,21 / 13,15 / 13,88 | 4,13 / 4,45 / 4,69 | 2,07 / 2,23 / 2,36 |
| accent, noir 60 % (`0x99`) | clair / saturé / sombre | 5,89 / 9,74 / 13,88 | **1,99** / 3,30 / 4,69 | **1,00** / 1,66 / 2,36 |
| `DWMSBT` acrylique, sombre | clair / saturé / sombre | 3,32 / 5,62 / 8,81 | **1,12** / 1,90 / 2,98 | 1,77 / **1,05** / 1,50 |
| `DWMSBT` acrylique, clair | clair / saturé / sombre | **1,29** / 1,90 / 2,70 | 2,29 / 1,56 / 1,09 | 4,56 / 3,10 / 2,18 |

### Ce que le texte atteint vraiment, mesuré sur l'application

Le tableau ci-dessus est calculé à partir de la couleur nominale des encres. Ce que l'œil
reçoit est autre chose : une capitale de 11 px a des fûts d'un pixel de large, et
l'anticrénelage ne les laisse jamais atteindre leur encre. Mesure du 2026-09-12 sur des
photographies de l'îlot déployé, après composition — « pic » est le pixel le plus éloigné de
la matière, « cœur » la moyenne du corps du trait (`captures/orfevre/52-vibrance-quatre-etats.png`) :

| élément | galet noir, clair | galet noir, sombre | verre, clair | verre, sombre |
|---|---|---|---|---|
| temps, 44 px Condensed 800 | 18,30 / **17,87** | 18,30 / 17,87 | 12,13 / 11,80 | 13,84 / 13,52 |
| « LECTURE », 11 px 500 | 6,19 / **5,38** | 6,19 / 5,38 | 4,10 / 3,71 | 4,68 / 4,16 |
| « Bloc 1 », 14 px 400 | 6,19 / **4,81** | 6,19 / 4,81 | 4,10 / 3,60 | 4,68 / 3,98 |
| « PAUSE » vive, 11 px 500 | 18,30 / **15,15** | 18,30 / 15,15 | 12,13 / 10,30 | 13,84 / 11,62 |
| « SUIVANT » au repos, 11 px 500 | 3,11 / **2,75** | 3,11 / 2,75 | 2,06 / 1,94 | 2,35 / 2,16 |

Trois choses s'y lisent, et la première est la plus importante :

- **sur le galet noir, les deux colonnes sont identiques.** La matière vaut `(0, 0, 0)` que le
  fond soit clair ou sombre, donc rien de ce que l'auteur a ouvert derrière ne peut faire
  varier la lisibilité d'un centième. C'est exactement ce qu'on demande à la vibrance — la
  couleur ne se pose jamais sur la couche translucide, toujours sur une couche opaque — et
  l'application l'obtient sans rien faire, parce que sa couche opaque est le galet lui-même.
  Sur le verre, les mêmes lignes bougent de 4,10 à 4,68 et de 1,94 à 2,16 selon le fond.
- **le cœur du trait est toujours en dessous du pic**, et l'écart grandit quand le corps
  diminue : −2 % à 44 px, −12 % à 11 px, −22 % pour « Bloc 1 ». Un tableau d'encres nominales
  surestime donc toujours la lisibilité des petits corps.
- **« SUIVANT » au repos délivre 2,75**, pas les 3,11 de sa couleur. C'est sous le plancher de
  3 des grands corps, et très loin des 4,5 que demandent les petits. DESIGN.md §2 veut cette
  encre « presque éteinte — une légende se devine, ne se lit pas », et §6 la donne aux actions
  au repos : le chiffre confirme l'intention plutôt qu'il ne la contredit. Il faut seulement
  savoir que l'intention coûte cela, et qu'elle le coûte à 2,75 et non à 3,11. **Sur le verre,
  la même action tombe à 1,94 sur un fond clair : là, ce n'est plus une intention, c'est
  illisible.**

Lecture du tableau nominal :

- **la voie officielle est éliminée par ce tableau seul.** En clair de Windows, `#F2EFE9` —
  le blanc du temps restant, l'élément le plus important de l'îlot — tombe à **1,29** sur un
  fond clair. Pire, la hiérarchie s'inverse : `#5E5B57`, « presque éteint » selon DESIGN.md
  §2, devient l'encre la plus lisible (4,56). Tout le §7 de DESIGN.md, qui dit « livre
  fermé » en déplaçant la lumière entre les trois gris, cesse de fonctionner.
- avec la teinte réglable, il faut monter à **90 % de noir** pour que `#F2EFE9` tienne
  au-dessus de 12 partout. À cette teinte, le fond ne contribue plus que pour
  `0,050 × fond` : entre le fond le plus clair et le plus sombre, l'îlot ne varie que de
  43,8 à 33,6, soit **dix niveaux sur 255**. Ce n'est plus du verre, c'est un noir qui
  respire imperceptiblement.
- `#5E5B57` ne dépasse jamais 2,36 sur du verre, contre 3,11 sur du noir. Les actions au
  repos de DESIGN.md §6 deviennent illisibles quel que soit le réglage.

Les trois corrections que demande la vibrance — plus de contraste, un peu plus de graisse, un
léger gain d'interlettrage — ne rachètent pas un facteur 14 (18,30 → 1,29). Et la couleur
posée sur une couche opaque plutôt que sur le verre, c'est exactement l'îlot d'aujourd'hui.

### La porte est fermée deux fois, et il faut le dire ainsi

La forme (§7) n'est pas le seul verrou. **La teinte en est un second, indépendant**, et c'est
le tableau ci-dessus qui l'impose : il faut monter à `0xE6` pour que `#F2EFE9` tienne au-dessus
de 12 de contraste partout. À cette teinte, le fond ne traverse presque plus. Les nombres sont
ceux du §2, et ce sont les miens :

- entre le fond le plus clair (moyenne 223,6) et le plus sombre (24,7), soit **198,9 niveaux**
  de fond, l'îlot ne varie que de 43,4 à 34,2 — **9,2 niveaux, 4,6 % de transmission** ;
- `hf` tombe de 35,7 sur le fond nu à **1,1 à travers l'îlot** : le motif fin ne traverse pas
  du tout.

Il faut nommer correctement ce que ce second nombre dit. Un motif qui disparaît n'est pas une
preuve de verre — **c'est une preuve d'opacité**. Du verre floute le motif fin *et garde la
luminance d'ensemble* de ce qu'il y a derrière ; ici c'est la luminance d'ensemble qui part
avec le motif. À `0xE6`, l'instrument ne montre pas du verre : il montre une dalle sombre qui
prend une dizaine de niveaux du fond.

Donc deux verrous, et non un : **la forme** — le fond du système est un rectangle que rien ne
découpe (§7) — et **la teinte** — la seule teinte que la palette de DESIGN.md §2 supporte ne
laisse passer que 4,6 % du fond. Lever le premier ne suffirait pas.

---

## 9. Les coins continus — celui-là, on peut

Rien de ceci ne dépend du compositeur : la forme est une distance signée, donc un calcul par
pixel, et `galet.wgsl` la porte déjà. **La différence entre les deux formes tient dans une
seule norme** — remplacer la norme euclidienne par la norme *n* :

```wgsl
// Ce qui est dans galet.wgsl aujourd'hui : arc de cercle (norme 2).
let q = abs(p) - u.demi + vec2<f32>(u.rayon);
let d = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - u.rayon;

// La superellipse : la meme distance, avec la norme n. n = 2 redonne le cercle.
let m = max(q, vec2<f32>(0.0));
let norme = pow(pow(m.x, u.n) + pow(m.y, u.n), 1.0 / u.n);
let d = norme + min(max(q.x, q.y), 0.0) - u.rayon;
```

Le garde-fou indispensable : `pow(0, n)` est défini mais `pow` sur une valeur négative ne
l'est pas — d'où le `max(q, 0)` **avant** la puissance, déjà présent dans la version au
cercle.

### Pourquoi un exposant plus grand que 2, en chiffres

Courbure le long du quart de coin, rayon 28 px. `t = 0` est la jonction avec le bord droit —
où la courbe doit prolonger une droite, donc où la courbure devrait être nulle. `t = 1` est la
pointe du coin, à 45°.

```
   n     t=0.00   t=0.05   t=0.20   t=0.50   t=1.00   (max x r)
  2.0    0.0357   0.0357   0.0357   0.0357   0.0357    1.000
  2.5    0.0000   0.0104   0.0210   0.0350   0.0500    1.400
  3.0    0.0000   0.0028   0.0114   0.0303   0.0636    1.782
  3.5    0.0000   0.0007   0.0060   0.0249   0.0770    2.155
  4.0    0.0000   0.0002   0.0030   0.0198   0.0901    2.523
  5.0    0.0000   0.0000   0.0008   0.0121   0.1160    3.249
  6.0    0.0000   0.0000   0.0002   0.0071   0.1417    3.969
```

Voilà la cassure, en un nombre. À `n = 2`, la courbure vaut `1/r = 0,0357` **dès le premier
point de l'arc** : elle saute de 0 à 0,0357 à l'endroit exact où le droit rencontre le
courbe. Pour tout `n > 2` elle part de **zéro** et monte progressivement — c'est la
continuité de courbure, et c'est toute la définition du coin continu. Le prix est au
sommet : la courbure maximale passe de `1/r` à `2,52/r` pour `n = 4`, c'est-à-dire que le
point le plus serré du coin est deux fois et demie plus serré, mais il ne l'est qu'en un
point et sans discontinuité.

`coin-cercle-vs-superellipse-x12.png` met les deux côte à côte, grossis douze fois, à rayon
identique : à gauche l'arc de cercle et son coude visible, à droite la superellipse `n = 4`
qui entre dans le virage. `coin-superposition-x12.png` superpose les deux silhouettes.

### Le prix, qu'il faut connaître avant de choisir

À rayon égal, la superellipse **mange moins** de coin :

```
  n = 2.0 : aire retiree = 0.2146 r^2   (100,0 % du cercle)   r equivalent = 1.000 r
  n = 2.5 : aire retiree = 0.1548 r^2   ( 72,1 % du cercle)   r equivalent = 1.178 r
  n = 3.0 : aire retiree = 0.1167 r^2   ( 54,4 % du cercle)   r equivalent = 1.356 r
  n = 4.0 : aire retiree = 0.0730 r^2   ( 34,0 % du cercle)   r equivalent = 1.715 r
  n = 5.0 : aire retiree = 0.0498 r^2   ( 23,2 % du cercle)   r equivalent = 2.075 r
```

Apple compense en augmentant le rayon : à `n = 4` il faut `1,715 × r` pour retrouver la même
rondeur apparente. **Et c'est là que le galet replié bloque.** Replié, DESIGN.md §6 pose
88 × 36 avec un rayon de 18, c'est-à-dire exactement la demi-hauteur : c'est une pastille, il
n'y a aucun bord vertical droit, et le rayon ne peut pas monter — 18 est le maximum
géométrique. Un exposant élevé ne peut donc pas être compensé.

`eventail-replie-n2-25-3-4-5-x4.png` montre les cinq, empilés : à `n = 4` et `n = 5` le galet
n'est plus un galet, c'est une barre à coins mous. `galet-replie-x4.png` le montre encore plus
crûment, cercle contre `n = 4`. DESIGN.md §6 appelle le galet « la seule forme qui ne dessine
pas de cadre » — à `n = 4` il en dessine un.

**Ce que je proposerais, et c'est un arbitrage de dessin, pas une mesure :**

- **déployé** (320 × 148, rayon 28) : il reste 92 px de bord vertical droit de part et
  d'autre du coin, la jonction existe vraiment, la cassure se voit. `n = 4` est là le bon
  choix — c'est l'exposant le plus proche de ce que fait Apple. Si l'on veut en plus
  retrouver la silhouette d'aujourd'hui, l'égalité des aires retirées demande
  `1,715 × 28 ≈ 48` de rayon, ce que la demi-hauteur (74) autorise largement ;
- **replié** (88 × 36, rayon 18 = demi-hauteur) : `n = 2,5`. La courbure part encore de zéro,
  l'aire retirée ne tombe qu'à 72 % de celle du cercle, et sur `eventail-replie` c'est le
  dernier exposant qui se lit encore comme un galet.

Un exposant qui change avec le déploiement se pilote comme le rayon : DESIGN.md §6 interpole
déjà largeur, hauteur et rayon linéairement en `p`. Interpoler `n` de 2,5 à 4 sur le même `p`
coûte un `f32` de plus dans l'uniforme et rien d'autre.

---

## 10. Ce que je n'ai pas mesuré

- ~~**`DwmGetColorizationColor` dans l'état sain** (§5).~~ Comblé le 2026-09-12, et le
  résultat contredit l'hypothèse : voir la correction au §5.
- **Le coût du flou pour DWM lui-même** : le §4 mesure notre cadence, pas la consommation du
  compositeur. Que Windows coupe la transparence quand il économise la batterie suggère qu'il
  n'est pas nul.
- **Le balayage complet des teintes** : j'ai `0x01`, `0x40`, `0x99`, `0xE6`, `0xF2` et la
  bascule d'énergie a coupé la campagne avant `0xBF` et `0xD9`. Le comportement est monotone
  et le plancher à `#212224` est atteint dès `0x99` ; je n'attends pas de surprise entre les
  deux, mais je ne l'ai pas vu.
- **Le réglage manuel « Effets de transparence »** dans Paramètres › Personnalisation ›
  Couleurs : je n'y ai pas touché, c'est un réglage de la machine et il n'est pas à moi.
  L'économiseur d'énergie a produit le même état, ce qui rend l'essai manuel probablement
  redondant — mais « probablement » n'est pas « mesuré ».
- **Les deux ressorts indépendants** et **l'interruptibilité avec report de vitesse** : hors
  périmètre de cette campagne. `ressort.rs` intègre déjà par la solution exacte du cas
  critique et `Ilot::viser` ne touche ni la position ni la vitesse — le report de vitesse est
  donc déjà là ; ce qui manque est le second ressort, un par axe, et un essai qui le montre.

---

## 11. Ce que j'en conclus

**Le verre est possible sur cette machine. Il ne convient pas à cet îlot.** Cinq raisons, dans
l'ordre où elles pèsent :

1. **Il est rectangulaire** (§7). Le galet de DESIGN.md §6 ne peut pas être en verre : ni
   l'alpha ni la région de fenêtre ne découpent le fond du système, et c'est la seule forme
   que l'îlot ait jamais eue.
2. **Il détruit la palette** (§8). `#F2EFE9` passe de 18,30 de contraste à 1,29 sur la voie
   officielle ; `#5E5B57` ne dépasse jamais 2,36. Le langage du §7 de DESIGN.md — dire
   « livre fermé » en déplaçant la lumière entre trois gris — cesse de fonctionner.
3. **Il ne descend jamais à `#000000`** (§2). Le plancher est `#212224`, et DESIGN.md §2
   donne la raison exacte pour laquelle ce gris est inacceptable la nuit : il redessine le
   rectangle du moniteur.
4. **Il s'éteint quand il faut le plus** (§5). L'économiseur d'énergie de Windows 11 le coupe
   à 20 % de batterie — c'est-à-dire vers la fin d'une nuit de lecture sur un portable — et
   l'îlot devient alors soit une dalle grise unie, soit invisible.
5. **Il vient avec du grain** (§2), que DESIGN.md §8 n'autorise pas et qu'aucun réglage
   n'enlève.

À l'inverse, **les trois autres ambitions tiennent entièrement, et sans le verre** :

- **les coins continus** (§9) sont un changement d'une ligne dans `galet.wgsl`, mesurable, et
  visible côte à côte ;
- **la matière qui arrive** — deux ressorts indépendants, interruptibles avec report de
  vitesse — ne dépend que de `ressort.rs`, qui intègre déjà par la solution exacte ;
- **la vibrance** est déjà au maximum sur fond noir opaque : 18,30 / 6,19 / 3,11. C'est la
  meilleure ligne du tableau du §8, et c'est celle de l'application d'aujourd'hui.

Si l'on veut malgré tout essayer le verre un jour, la seule combinaison défendable est celle
du §3 avec une teinte noire à 90 % (`0xE6000000`) et des coins carrés assumés. Elle garde
12,2 de contraste sur le blanc, ne varie que de dix niveaux selon le fond, et demande
d'abandonner le galet. Autrement dit : elle coûte la forme pour un effet qu'on ne voit
presque pas. Ce n'est pas « less is more », c'est moins pour moins.

---

## Les essais

Neuf combinaisons de drapeaux sur un seul programme, `verre.exe`, plus `coins.exe` pour la
géométrie. Chaque capture citée porte le nom de son lancement ; la ligne de commande se relit
directement dans le nom :

```
verre.exe --nom=02-acrylique        --backdrop=3
verre.exe --nom=05-acryl-cadre      --backdrop=3 --etendre=-1
verre.exe --nom=08-acryl-sombre     --backdrop=3 --etendre=-1 --sombre
verre.exe --nom=10-acryl-effopaque  --backdrop=3 --etendre=-1 --effacement-opaque
verre.exe --nom=11-acryl-nvidia     --backdrop=3 --etendre=-1 --sans-inherit
verre.exe --nom=22-accent4-a99      --accent=4 --teinte=0x99000000
verre.exe --nom=30-accent4-e6       --accent=4 --teinte=0xE6000000
verre.exe --nom=33-coins-round      --accent=4 --teinte=0x99000000 --coins=2
verre.exe --nom=40-verre-rect-galetnoir --accent=4 --teinte=0x99000000 --galet=28
verre.exe --nom=41-verre-region     --accent=4 --teinte=0x99000000 --region=28
verre.exe --nom=44-region-opaque    --region=28 --effacement-opaque
verre.exe --nom=91-galet-alpha-econo --galet=28
verre.exe --nom=60-cadence          --cadence --trames=500 [--accent=4 | --backdrop=3]
coins.exe
```

Chaque lancement produit deux images : la vue générale (400 × 228, l'îlot plus 40 px de marge)
et `<nom>-coin.png`, le coin haut-gauche grossi six fois — c'est dans celle-là que se lisent
l'arrondi, le crénelage et le grain.
