# Veilleuse — fondations techniques

Tout ce qui suit a été mesuré sur cette machine (Windows 11 Pro 10.0.26200, rustc 1.98.1, MSVC),
dans un dossier d'essai séparé. Rien n'est tiré de la mémoire : chaque affirmation est suivie
de la sortie du programme qui l'a produite.

Matériel des essais : écran unique 1920 × 1080 à 144 Hz, facteur d'échelle 1,25.
Deux cartes graphiques : Intel UHD Graphics (intégrée) et NVIDIA GeForce RTX 4060 Laptop
(dédiée). Cette dualité s'est révélée être la question centrale du projet — voir §1.

---

## 0. Les versions, et pourquoi celles-là

```toml
winit  = "0.30"     # résolu en 0.30.13
wgpu   = "30.0"     # résolu en 30.0.1
glyphon = "0.12"    # résolu en 0.12.0

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",                 # capture d'écran, découpe arrondie
    "Win32_UI_WindowsAndMessaging",       # WindowFromPoint, styles étendus
    "Win32_UI_Input_KeyboardAndMouse",    # seulement pour les essais
] }
```

L'accord entre les trois est imposé par glyphon, pas négociable, et c'est bien là que se
trouvent les ennuis habituels. Le `Cargo.toml` publié de glyphon 0.12.0 épingle :

```toml
[dependencies.cosmic-text]  version = "0.19"
[dependencies.wgpu]         version = "30.0.0"   features = ["wgsl"], default-features = false
[dev-dependencies.winit]    version = "0.30.12"
```

Donc : **glyphon 0.12 ⇒ wgpu 30 ⇒ cosmic-text 0.19**, et winit 0.30 est la version contre
laquelle glyphon est effectivement testé. glyphon 0.11 tire wgpu 29 et cosmic-text 0.18 :
mélanger les deux lignées ne compile pas, les types `wgpu::Device` n'étant pas les mêmes.

**Ne pas prendre winit 0.31.** La dernière publiée est `0.31.0-beta.3`, une préversion qui
éclate le crate en `winit-core` / `winit-win32` / etc. et change la création de fenêtre.
0.30.13 est la dernière stable et c'est celle que glyphon vise.

Ne pas déclarer `cosmic-text` en dépendance directe : `glyphon::cosmic_text` est réexporté et
c'est la même instance de crate. `FontFeatures` et `FeatureTag` ne font pas partie de la liste
réexportée au premier niveau, on les prend par le chemin long :

```rust
use glyphon::cosmic_text::{FeatureTag, FontFeatures};
```

Arbre complet résolu, pour mémoire : `cosmic-text 0.19.0`, `fontdb 0.23.0`, `harfrust 0.5.2`,
`swash 0.2.10`, `etagere 0.3.0`, `naga 30.0.1`, `wgpu-hal 30.0.1`, `raw-window-handle 0.6.2`.
Première compilation complète de l'arbre : 1 min 12 s.

### Pièges d'API de wgpu 30 rencontrés à la compilation

Ces signatures ont changé et le compilateur ne les devine pas :

| Ce qu'on écrit d'instinct | Ce qu'il faut en wgpu 30 |
|---|---|
| `Instance::new(&desc)` | `Instance::new(desc)` — par valeur ; et pas de `Default` : `InstanceDescriptor::new_without_display_handle()` ou `new_with_display_handle(Box::new(el.owned_display_handle()))` |
| `RequestAdapterOptions { .. }` | champ supplémentaire obligatoire `apply_limit_buckets: bool` |
| `vertex.buffers: &[VertexBufferLayout]` | `&[Option<VertexBufferLayout>]` |
| `PipelineLayoutDescriptor { bind_group_layouts: &[&bgl], push_constant_ranges: &[] }` | `bind_group_layouts: &[Some(&bgl)]` et `immediate_size: 0` (le champ `push_constant_ranges` n'existe plus) |
| `device.on_uncaptured_error(Box::new(f))` | `Arc::new(f)` |
| `device.poll(PollType::Wait)` | `PollType::wait_indefinitely()` (`Wait` est une variante à champs) |
| `buffer.get_mapped_range()` | renvoie un `Result`, à déballer |
| `surface.get_current_texture() -> Result<..>` | renvoie l'énumération `CurrentSurfaceTexture` : `Success`, `Timeout`, `Occluded`, `Outdated`, `Suboptimal(_)`, `Lost`, `Validation` |
| `queue.submit(..)` puis `frame.present()` | `queue.present(frame)` |
| `RenderPassColorAttachment` | champ `depth_slice: None` obligatoire |
| `RenderPassDescriptor` | champ `multiview_mask: None` obligatoire |
| `SurfaceConfiguration` | champ `color_space: SurfaceColorSpace::Auto` obligatoire |

---

## 1. La fenêtre transparente, sans décoration, toujours au-dessus — et le vrai piège

### Les attributs

```rust
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::{Window, WindowLevel};

Window::default_attributes()
    .with_inner_size(PhysicalSize::new(320, 148))
    .with_position(PhysicalPosition::new(x, y))
    .with_transparent(true)                       // per-pixel alpha
    .with_decorations(false)                      // pas de cadre ni de barre de titre
    .with_window_level(WindowLevel::AlwaysOnTop)  // WS_EX_TOPMOST
    .with_resizable(false)
    .with_skip_taskbar(true)                      // hors barre des tâches
    .with_undecorated_shadow(false)               // sinon Windows ajoute 1 px clair en haut
```

`with_skip_taskbar` et `with_undecorated_shadow` viennent du trait
`WindowAttributesExtWindows`, à importer explicitement. `with_undecorated_shadow(false)` est
le défaut mais mérite d'être écrit : la documentation de winit signale qu'activer l'ombre
« fait apparaître une fine ligne de 1 px en haut de la fenêtre », ce que §8 de DESIGN.md
interdit.

Mesuré : `is_decorated = false`, taille physique exacte demandée, position exacte demandée.

### Ce que `with_transparent(true)` fait réellement

Ce n'est pas un drapeau passé au pilote graphique. winit appelle, à la création :

```rust
let region = CreateRectRgn(0, 0, -1, -1);          // région vide
DwmEnableBlurBehindWindow(hwnd, &DWM_BLURBEHIND {
    dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
    fEnable: TRUE, hRgnBlur: region, ..
});
```

C'est **cet appel**, et lui seul, qui demande à DWM de tenir compte du canal alpha de la
fenêtre. Essai témoin, deux fenêtres empilées, celle du dessus peignant une bande à alpha 0
au-dessus d'un fond magenta `(204, 0, 153)` :

```
with_transparent = true     bande alpha 0 = (204, 0, 153)   ← le fond traverse
with_transparent = false    bande alpha 0 = (0, 0, 0)       ← noir opaque
```

### Le format et le mode alpha : ce que cette machine annonce vraiment

Tous les adaptateurs de la machine, interrogés pour la même surface
(`instance.enumerate_adapters` puis `surface.get_capabilities`) :

| Dorsal | Adaptateur | `alpha_modes` | `present_modes` |
|---|---|---|---|
| Vulkan | NVIDIA RTX 4060 | `[Opaque]` | Fifo, FifoRelaxed, Mailbox, Immediate |
| **Vulkan** | **Intel UHD** | **`[Opaque, Inherit]`** | Immediate, Fifo |
| Dx12 | NVIDIA RTX 4060 | `[Opaque]` | Mailbox, Fifo, Immediate |
| Dx12 | Intel UHD | `[Opaque]` | Mailbox, Fifo, Immediate |
| Dx12 | Microsoft Basic Render Driver | `[Opaque]` | Mailbox, Fifo, Immediate |
| Gl | NVIDIA RTX 4060 | `[Opaque]` | Fifo, Immediate |

Formats, dans l'ordre annoncé et identiques partout en tête :
`[Bgra8UnormSrgb, Rgba8UnormSrgb, Bgra8Unorm, Rgba8Unorm, …]`. **`Bgra8UnormSrgb` est le
premier de la liste sur les six combinaisons** : c'est celui qu'il faut prendre, et il est
sRGB, ce qui a des conséquences au §1.4.

`caps.usages` contient `COPY_SRC` : la texture de la chaîne d'échange est copiable, on peut
donc relire à l'écran ce qui a réellement été présenté.

**Ni `PreMultiplied` ni `PostMultiplied` ne sont offerts par aucun adaptateur de cette
machine.** Les demander produit une erreur de validation non capturée, et la configuration
précédente reste en place :

```
configure(alpha=PreMultiplied) annonce=false -> tentative
  !! erreur wgpu non capturee : Validation Error
  Caused by:
    In Surface::configure
      Requested alpha mode PreMultiplied is not in the list of supported alpha modes: [Opaque, Inherit]
```

`Auto` n'est pas non plus dans la liste annoncée mais **ne déclenche aucune erreur** : wgpu le
traduit en interne vers le premier mode réellement disponible. C'est donc la valeur à écrire.

### Le piège numéro un, et ce n'est pas celui qu'on croit

Le piège n'est pas de choisir le mauvais mode alpha. **C'est de choisir le mauvais
adaptateur.** Le même essai, rejoué sur chaque combinaison dorsal/carte, en demandant partout
`CompositeAlphaMode::Auto` :

```
Vulkan / Intel UHD         alpha_modes = [Opaque, Inherit]   bande alpha 0 = (204,0,153)  transparent
Vulkan / NVIDIA RTX 4060   alpha_modes = [Opaque]            bande alpha 0 = (0,0,0)      OPAQUE
Dx12   / NVIDIA RTX 4060   alpha_modes = [Opaque]            bande alpha 0 = (0,0,0)      OPAQUE
Dx12   / Intel UHD         alpha_modes = [Opaque]            bande alpha 0 = (0,0,0)      OPAQUE
Gl     / NVIDIA RTX 4060   alpha_modes = [Opaque]            bande alpha 0 = (0,0,0)      OPAQUE
```

Une seule combinaison sur six laisse passer l'alpha, alors que `with_transparent(true)` et
l'appel DWM sont identiques dans les six cas. Le marqueur fiable n'est pas le mode demandé :
c'est la **présence de `Inherit` dans `caps.alpha_modes`**. Sur l'adaptateur Intel/Vulkan, la
transparence marche même en demandant explicitement `Opaque` — c'est bien la capacité de la
chaîne d'échange, pas la demande, qui décide.

Règle à écrire dans le code :

```rust
// Une surface ne laisse passer l'alpha que si sa chaine d'echange sait heriter de la
// composition du systeme. Ce n'est vrai que sur certains couples dorsal/adaptateur :
// sur cette machine, uniquement Vulkan + Intel UHD.
let alpha_utilisable = caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Inherit);
// et on configure toujours avec Auto, jamais avec PreMultiplied/PostMultiplied.
let alpha_mode = wgpu::CompositeAlphaMode::Auto;
```

Sélectionner l'adaptateur par `PowerPreference` ne suffit pas et n'est pas portable : il faut
parcourir `enumerate_adapters`, garder ceux dont la surface annonce `Inherit`, et prévoir le
cas où il n'y en a aucun (§1.5).

### Ce que DWM attend de l'alpha : du prémultiplié, dans l'espace des octets stockés

Quatre bandes peintes en écriture directe (`blend: None`, donc l'alpha du nuanceur arrive tel
quel dans la surface), au-dessus d'un fond magenta `(204, 0, 153)` :

| Bande | Écrit | Mesuré à l'écran |
|---|---|---|
| alpha 0 | `(0, 0, 0, 0)` | `(204, 0, 153)` — le fond |
| blanc opaque | `(1, 1, 1, 1)` | `(255, 255, 255)` |
| **octet 128, alpha 0,5** | linéaire `0,21586` → octet stocké 128 | **`(230, 128, 204)`** |
| blanc, alpha droit 0,5 | `(1, 1, 1, 0,5)` → octet 255 | `(255, 255, 255)` — saturé |

Le vrai mélange à moitié entre blanc `(255,255,255)` et magenta `(204,0,153)` vaut
`(229,5 ; 127,5 ; 204)`. La troisième bande donne `(230, 128, 204)` : exact. Donc

> DWM compose en **alpha prémultiplié**, et le mélange se fait sur les **octets stockés**,
> c'est-à-dire *après* l'encodage sRGB — pas dans l'espace linéaire.

La surface étant `Bgra8UnormSrgb`, un nuanceur qui sort `0,5` en linéaire fait stocker l'octet
188, pas 128. Pour un blanc à 50 % correct il faut sortir la valeur linéaire dont l'encodage
sRGB vaut `0,5 × 255`, soit `0,21586`. En pratique, pour tout ce qui est translucide :

```rust
// La couleur sortie du fragment doit deja etre multipliee par son alpha, et cette
// multiplication doit avoir lieu dans l'espace ou DWM melange, c'est-a-dire apres
// l'encodage sRGB. On prepare donc les couleurs cote CPU en octets, et on les
// reconvertit en lineaire juste avant de les donner au nuanceur.
```

Conséquence pratique pour l'îlot : `#000000` à alpha 1 partout où le galet est peint, alpha 0
ailleurs. Aucune valeur intermédiaire sauf sur le liseré antialiasé du coin arrondi — et là,
l'encre étant noire, le prémultiplié donne exactement `(0, 0, 0, a)`, ce qui est correct sans
calcul.

### Le repli, si aucun adaptateur n'annonce `Inherit`

L'îlot de DESIGN.md §6 est un galet **noir opaque** à coins arrondis. Il ne demande donc aucun
alpha par pixel : il lui faut seulement une découpe. Windows la fournit depuis toujours :

```rust
use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};

// CreateRoundRectRgn prend les DIAMETRES des ellipses de coin, pas les rayons.
let rgn = CreateRoundRectRgn(0, 0, w + 1, h + 1, rayon * 2, rayon * 2);
SetWindowRgn(hwnd, rgn, 1);   // renvoie 0 en cas d'echec
```

Essai sur le pire cas — Dx12 + NVIDIA, `alpha_modes = [Opaque]`, `with_transparent(false)` :

```
SetWindowRgn -> 1
centre du galet          = (255, 255, 255)   ← la fenetre
coin haut-gauche         = (237, 244, 248)   ← ce qu'il y avait derriere
milieu du bord gauche    = (255, 255, 255)   ← la fenetre
```

La découpe fonctionne, sur n'importe quel adaptateur. Son coût : une région GDI est un masque
à un bit, donc le coin arrondi est **crénelé**, sans dégradé. À rayon 18 px (galet replié) et
28 px (déployé), sur fond noir dans une chambre sombre, c'est visible de près.

D'où l'architecture retenue pour l'îlot, qui ne tombe jamais en panne :

1. au démarrage, chercher un adaptateur dont la surface annonce `Inherit` ;
2. s'il y en a un : `with_transparent(true)`, coins arrondis dessinés dans le nuanceur de
   fragments (distance signée au rectangle arrondi), bords adoucis ;
3. sinon : `with_transparent(false)` + `SetWindowRgn`, coins crénelés, et la région est
   recalculée à chaque pas du ressort de déploiement puisque largeur, hauteur et rayon
   changent ensemble.

L'horloge plein écran, elle, est entièrement noire opaque : elle n'a aucun besoin de tout
ceci et tourne sur n'importe quel adaptateur.

---

## 2. Les clics qui traversent

### L'API et ce qu'elle fait vraiment

`window.set_cursor_hittest(false)` ajoute à la fenêtre les styles étendus
`WS_EX_TRANSPARENT | WS_EX_LAYERED` (vérifié dans le code de winit, puis mesuré). Preuve
automatique par `WindowFromPoint`, qui ignore par construction les fenêtres
`WS_EX_TRANSPARENT` :

```
HWND de la fenetre = 0x290a64
== hittest = vrai (defaut) ==
  styles ex (TRANSPARENT, LAYERED) = (false, false)
  WindowFromPoint(centre) == nous  = true
== hittest = faux ==
  styles ex (TRANSPARENT, LAYERED) = (true, true)
  WindowFromPoint(centre) == nous  = false
  WindowFromPoint(centre)          = 0x20574          ← la fenetre du dessous
== retour hittest = vrai ==
  styles ex (TRANSPARENT, LAYERED) = (false, false)
  WindowFromPoint(centre) == nous  = true
```

Le basculement est réversible et immédiat, dans les deux sens.

**`WS_EX_LAYERED` ne casse pas le rendu wgpu.** C'est le piège attendu — une fenêtre layered
sans `SetLayeredWindowAttributes` ni `UpdateLayeredWindow` peut rester invisible — et il ne se
produit pas ici. Essai : même scène que §1.4, avec `set_cursor_hittest(false)` appelé en
cours de rendu, puis photographie de l'écran :

```
set_cursor_hittest(false) pendant le rendu = true
  bande 0 (alpha 0)          = (204, 0, 153)
  bande 1 (blanc opaque)     = (255, 255, 255)
  bande 2 (octet 128, a=0.5) = (230, 128, 204)
```

Résultats identiques au point près à ceux sans `WS_EX_LAYERED`.

### Le vrai coût, et la seule façon de revenir

`set_cursor_hittest` est **global à la fenêtre** : il n'existe aucune API winit pour rendre
une région atteignable et le reste traversant. Et le prix est net :

```
  evenements CursorMoved recus depuis, hittest faux = 0
  GetCursorPos = (500, 400), dans le rectangle = true
```

Le curseur a été placé par programme au centre exact de la fenêtre ; **zéro** événement de
souris est arrivé. Une fenêtre non atteignable est sourde : elle ne peut pas apprendre d'un
événement que le curseur est revenu sur elle. Le retour à l'état atteignable ne peut donc
venir que d'une interrogation active — `GetCursorPos` — ou d'un crochet global de souris.

### La conclusion, qui simplifie tout

**Il ne faut pas basculer le drapeau sur une seule fenêtre.** Deux fenêtres séparées suppriment
le problème :

- **horloge** : plein écran, `set_cursor_hittest(false)` posé une fois pour toutes au
  démarrage et jamais retouché. Elle ne reçoit rien, ne bloque rien, laisse tout passer ;
- **îlot** : petit, exactement à la taille du galet, `hittest` toujours vrai. Il reçoit ses
  clics normalement, sans aucune interrogation périodique.

Si l'îlot devait un jour avoir une zone morte (par exemple un halo transparent autour du
galet), la seule voie est un réveil périodique — `ControlFlow::WaitUntil` à quelques dizaines
de hertz — qui lit `GetCursorPos`, compare au rectangle du galet, et appelle
`set_cursor_hittest` seulement quand l'état change. Mesuré au §6 : ce réveil est précis à
0,4 ms, donc suffisant. Mais tant que l'îlot est à la taille du galet, ce code n'a pas lieu
d'exister.

---

## 3. Déplacer la fenêtre à la souris, et retenir sa position

### `drag_window`, et le piège de l'asynchronie

Sous Windows, `drag_window()` ne déplace rien lui-même : il poste `WM_NCLBUTTONDOWN` avec
`HTCAPTION` après avoir relâché la capture (code de winit, `handle_os_dragging`). Deux
conséquences :

1. **il ne peut être appelé que pendant un appui déjà en cours.** Appelé à froid il renvoie
   `Ok(())` et ne fait rien — mesuré ;
2. **son effet est différé.** Le message est posté, pas envoyé ; la boucle de déplacement
   modale de Windows démarre au tour de pompe suivant et **bloque la boucle d'événements**
   jusqu'au relâchement du bouton.

Premier essai, en lisant la position à la trame suivante : `déplacement effectif = (0, 0)`.
Le même essai en attendant que la boucle modale se termine :

```
position a la creation = Ok(PhysicalPosition { x: 400, y: 400 })
drag_window() pendant l'appui = Ok(())
position apres glisser = (520, 470) — attendu ~(520, 470)
  deplacement effectif = (120, 70)
```

Exact au pixel. L'appui, les dix déplacements de curseur et le relâchement ont été
synthétisés par `SendInput` depuis un fil secondaire — obligatoire, puisque la boucle modale
bloque le fil principal.

Le squelette correct :

```rust
WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
    // Le seuil de 4 px de DESIGN.md §6 ne se decide pas ici : une fois drag_window()
    // appele, Windows prend la main et on ne verra plus passer les mouvements. Il faut
    // donc memoriser le point d'appui, attendre le premier CursorMoved qui depasse
    // 4 px, et n'appeler drag_window() qu'a ce moment-la. En deca, c'est un clic.
}
```

Après le retour de la boucle modale, winit livre `WindowEvent::Moved` : c'est le bon endroit
pour enregistrer la position, plutôt qu'une lecture à l'aveugle.

### Mémoriser la position

`outer_position()` et `set_outer_position()` travaillent en pixels physiques et font un
aller-retour exact (`(333, 222)` demandé, `(333, 222)` relu). La position se sauve telle
quelle.

Une position mémorisée ne vaut rien sans validation : un écran débranché laisserait l'îlot
hors de vue pour toujours. Vérification obligatoire au démarrage contre
`el.available_monitors()` :

```rust
/// Une position memorisee ne vaut que si elle retombe sur un moniteur branche
/// aujourd'hui : un ecran debranche laisserait l'ilot hors de vue pour toujours.
fn position_visible(el: &ActiveEventLoop, p: (i32, i32)) -> bool {
    el.available_monitors().any(|m| {
        let o = m.position();
        let s = m.size();
        // on exige que le coin haut-gauche plus une marge reste dans l'ecran
        p.0 + 40 >= o.x && p.1 + 20 >= o.y
            && p.0 + 40 < o.x + s.width as i32
            && p.1 + 20 < o.y + s.height as i32
    })
}
```

Trois lancements successifs :

```
aucune position memorisee -> defaut (400, 400)        ← premier lancement
position relue = (520, 470) (sur un moniteur : oui)   ← apres le glisser
position relue = (9000, 9000) hors ecran -> repli sur le defaut
```

---

## 4. Plein écran : une fenêtre ou deux ?

### Ce que fait `set_fullscreen`

```rust
w.set_fullscreen(Some(Fullscreen::Borderless(w.current_monitor())));
```

L'effet est **immédiat et synchrone** — pas besoin d'attendre un événement `Resized` :

```
avant plein ecran :  inner_size = 600x400   pos = (50, 50)
apres set_fullscreen (immediat) : inner_size = 1920x1080  pos = (0, 0)  fullscreen = true
```

Il faut néanmoins reconfigurer la surface sur `WindowEvent::Resized`, la chaîne d'échange ne
suivant pas toute seule.

### La question qui décide

Trois fenêtres : un fond magenta à un endroit connu, une horloge passée en plein écran sans
bordure, et un îlot toujours-au-dessus créé **après**. Puis photographie de l'écran entier.

Horloge transparente (alpha 0 partout) :

```
  point au-dessus du fond magenta (270, 600) = (204, 0, 153)   ← la transparence survit
  point sur l'ilot (800, 160)                = (0, 218, 89)    ← l'ilot est au-dessus
```

Horloge **opaque** — le cas réel, l'horloge étant noire :

```
  point au-dessus du fond magenta (270, 600) = (0, 0, 0)       ← l'horloge couvre
  point sur l'ilot (800, 160)                = (0, 218, 89)    ← l'ilot reste au-dessus
```

Deux choses sont donc vraies sur cette machine :

1. la transparence par pixel **survit** au plein écran sans bordure (aucune bascule vers un
   mode exclusif qui l'aurait tuée) ;
2. une fenêtre `AlwaysOnTop` **reste visible au-dessus** d'une fenêtre plein écran opaque du
   même processus.

### Tranché : deux fenêtres

- **Horloge** — fenêtre propre, sans décoration, `Fullscreen::Borderless`, noire opaque,
  `set_cursor_hittest(false)` une fois pour toutes. Aucun besoin d'alpha, donc aucune
  contrainte d'adaptateur.
- **Îlot** — fenêtre propre, petite, déplaçable, toujours atteignable, `AlwaysOnTop`.

Ce qui l'emporte n'est pas une préférence : c'est que le §2 a montré qu'une fenêtre unique
obligerait à basculer `set_cursor_hittest` selon la position du curseur, donc à interroger
`GetCursorPos` en permanence, donc à réveiller le processus quarante fois par seconde toute
la nuit pour une horloge qui ne devrait dessiner qu'une fois par seconde. Deux fenêtres
suppriment ce code entier.

Une seule chose est partagée : `wgpu::Instance` et `Adapter`. Chaque fenêtre a sa `Surface`.
Rien n'empêche de partager aussi `Device`/`Queue` — c'est ce qu'il faut faire — à condition
que les deux surfaces viennent du même adaptateur. Si l'îlot exige un adaptateur annonçant
`Inherit` (Intel/Vulkan ici) et pas l'horloge, deux `Device` séparés sont possibles mais
coûtent une seconde chaîne de rendu ; le plus simple est de faire tourner les deux fenêtres
sur l'adaptateur compatible alpha, l'horloge n'ayant à dessiner qu'une poignée de quads.

---

## 5. Le texte : police embarquée, chiffres tabulaires, et ce que glyphon ne sait pas faire

### Charger la police depuis le binaire

`FontSystem::new()` scrute les polices du système. Ce n'est pas ce qu'on veut : DESIGN.md §3
exige que la chaîne de secours ne serve jamais. On part donc d'une base **vide** :

```rust
const BARLOW_COND_800: &[u8] = include_bytes!("../../assets/BarlowCondensed-ExtraBold.ttf");

let mut base = glyphon::fontdb::Database::new();   // vide, aucune police systeme
base.load_font_data(BARLOW_COND_800.to_vec());
let mut font_system = FontSystem::new_with_locale_and_db("fr-CA".to_string(), base);
```

Mesuré : `familles chargees : ["Barlow Condensed"]`. Le nom de famille vient du fichier, il
n'est pas à inventer ; c'est lui qu'on passe à `Family::Name`.

### Activer les chiffres tabulaires

```rust
use glyphon::cosmic_text::{FeatureTag, FontFeatures};

let mut tnum = FontFeatures::new();
tnum.enable(FeatureTag::new(b"tnum"));

let chiffres = Attrs::new()
    .family(Family::Name("Barlow Condensed"))
    .weight(Weight(800))
    .font_features(tnum);
```

`FeatureTag::new` est `const` et prend `&[u8; 4]`. La mise en forme doit être en
`Shaping::Advanced` : c'est le chemin HarfRust, le seul qui applique les tables OpenType.

Mesure de la largeur de mise en page à `F = 432 px` :

```
sans tnum : « 0000 » = 787,97 px, « 1111 » = 502,85 px, ecart = 285,12 px, avance/chiffre = 0,4560 em
avec tnum : « 0000 » = 898,56 px, « 1111 » = 898,56 px, ecart =   0,00 px, avance/chiffre = 0,5200 em
```

Écart exactement nul avec `tnum`, et avance de **0,5200 em** par chiffre — la valeur que
DESIGN.md §3 annonce, confirmée à quatre décimales. Sans `tnum`, « 1111 » est plus étroit de
285 px que « 0000 » : les cartes changeraient de largeur à chaque 1, ce qui est précisément le
défaut reproché aux dix-huit polices écartées.

`font-kerning: none` de DESIGN.md n'a pas d'équivalent direct ; l'équivalent OpenType est
`FeatureTag::new(b"kern")` désactivé par `tnum.disable(...)`. Entre chiffres tabulaires le
crénage est nul de toute façon, mais l'écrire coûte une ligne et ferme la question.

### Ce que glyphon ne sait pas faire, et la conséquence sur l'architecture

`TextArea` n'expose que `left`, `top`, `scale` et `bounds`. **Il n'y a aucun moyen de donner
une matrice à glyphon**, donc aucun moyen de lui faire tourner un glyphe en 3D. Le volet ne
peut pas être obtenu en faisant tourner du texte : il faut passer par une texture.

D'où la chaîne retenue, prouvée au §6 :

1. glyphon dessine les chiffres dans une **texture hors-écran** (`RENDER_ATTACHMENT |
   TEXTURE_BINDING`), et non dans la fenêtre ;
2. le volet est un quad texturé qui échantillonne cette texture, avec la rotation faite dans
   le nuanceur de sommets.

`TextAtlas::new(&device, &queue, &cache, format)` fige le format de la cible : il doit être
celui de la texture hors-écran, et `Viewport::update` reçoit **la résolution de cette
texture**, pas celle de la fenêtre.

Détail qui évite un long débogage : les deux valeurs — ancienne et nouvelle — sont rendues
dans **une seule** texture de hauteur double, la nouvelle en haut, l'ancienne en bas. Le
nuanceur choisit alors l'une ou l'autre par ses seules coordonnées de texture, sans changer
de groupe de liaison entre les quads, donc sans casser l'instanciation.

Enfin, un tampon `Buffer` **par chiffre**, pas un pour la rangée : c'est ce qui permettra de
ne refaçonner que la carte qui change, comme l'exige DESIGN.md §5.

---

## 6. La capture — `--capture=chemin.png`

Indispensable, donc prouvée. Elle fonctionne **sans aucune fenêtre** : instance créée par
`InstanceDescriptor::new_without_display_handle()`, rendu dans une texture
`RENDER_ATTACHMENT | COPY_SRC`, copie vers un tampon `COPY_DST | MAP_READ`, lecture, PNG,
sortie.

Le seul point délicat est l'alignement : `copy_texture_to_buffer` exige que chaque ligne du
tampon soit un multiple de 256 octets. À 1920 px en RGBA, `1920 × 4 = 7680` est déjà un
multiple de 256, mais il ne faut pas compter dessus :

```rust
let lig_octets = LARGEUR * 4;
let lig_alignee = lig_octets.div_ceil(256) * 256;   // exigence de copy_texture_to_buffer
…
let tranche = lecture.slice(..);
let (tx, rx) = std::sync::mpsc::channel();
tranche.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
rx.recv().unwrap().unwrap();
let donnees = tranche.get_mapped_range().unwrap();   // Result en wgpu 30
// puis on recopie ligne a ligne en sautant le remplissage
```

Sortie réelle :

```
adaptateur : NVIDIA GeForce RTX 4060 Laptop GPU (Vulkan)
familles chargees : ["Barlow Condensed"]
F = 432
carte 224.6 x 362.0, rangee 1628.6 px (85 % de la largeur)
phase=0 lumiere=1 quads=14 pixels clairs=234305 -> volet_0.png
```

Les 85 % de largeur et la géométrie en unités de F sortent du calcul de DESIGN.md §4 sans
retouche. Sept images ont été produites (phases 0 ; 0,15 ; 0,3 ; 0,44 ; 0,6 ; 0,8 ; 1) et
lues : la rangée affiche « 23 25 45 », les glyphes sont ceux de Barlow Condensed 800, la
carte des unités de seconde bascule.

Le drapeau accepte aussi `--phase=` et `--lumiere=`, ce qui permet de juger l'animation image
par image sans la regarder tourner. Ce dernier point vaut la peine d'être gardé dans
l'application.

---

## 7. Le volet — la rotation, en vrai

### Le nuanceur de sommets

Aucun raccourci 2D. Le point du quad passe dans un repère dont l'origine est **sur le pli**,
tourne autour de l'axe X, et est projeté par une perspective à un point propre à sa carte.

```wgsl
@vertex
fn vs(e: Entree) -> Sortie {
    let px = e.rect.x + e.coin.x * e.rect.z;
    let py = e.rect.y + e.coin.y * e.rect.w;

    let angle = e.par.x;
    let pli   = e.par.y;
    let d     = e.par.z;

    // Repere local : l'origine est SUR le pli, c'est l'axe de la charniere.
    let ly = py - pli;
    let c  = cos(angle);
    let s  = sin(angle);
    let y3 = ly * c;
    let z3 = ly * s;            // positif = vers l'observateur

    // Perspective a un point, propre a CETTE carte (DESIGN.md §5 : jamais partagee).
    // Le point de fuite est le centre horizontal de la carte.
    let cx = e.rect.x + e.rect.z * 0.5;
    let k  = d / max(d - z3, 1.0);
    let xp = (px - cx) * k + cx;
    let yp = y3 * k + pli;

    var o: Sortie;
    o.pos = vec4<f32>((xp / u.taille.x) * 2.0 - 1.0, 1.0 - (yp / u.taille.y) * 2.0, 0.0, 1.0);
    o.uv  = mix(e.uv.xy, e.uv.zw, e.coin);
    // La lumiere glisse sur la face : une carte de face prend tout, de profil presque rien.
    o.lum = mix(1.0, 0.30 + 0.70 * abs(c), e.par.w);
    return o;
}
```

`max(d - z3, 1.0)` est le garde-fou : sans lui, un volet qui passerait le plan de l'œil
diviserait par zéro. À `d = 4 × hauteur de carte = 3,36 F`, l'angle reste borné à ±90° et
`z3` ne peut pas approcher `d`, mais la borne coûte une instruction.

Le signe de `s` a été choisi par l'image, pas par la théorie : avec `z3 = ly·sin(angle)` et
`ly < 0` pour la moitié haute, un angle négatif donne `z3 > 0`, donc `k > 1`, donc un bord
supérieur qui **s'élargit** — le volet tombe vers l'observateur, comme une carte réelle. Le
signe opposé le ferait basculer derrière l'écran.

### La géométrie des quatre couches

Chaque carte qui change porte quatre quads, exactement comme DESIGN.md §5 les décrit :

| Couche | Contenu | Angle |
|---|---|---|
| haut fixe | **nouvelle** valeur, moitié haute | 0 |
| bas fixe | **ancienne** valeur, moitié basse | 0 |
| volet haut | **ancienne** valeur, moitié haute | 0 → −90° en 160 ms, `cubic-bezier(0.32, 0, 0.67, 0)` |
| volet bas | **nouvelle** valeur, moitié basse | +90° → 0 en 200 ms, `cubic-bezier(0.33, 1, 0.68, 1)` |

Les courbes de CSS sont reproduites par inversion de la Bézier en x, douze bissections
suffisant à 10⁻⁴. Les cartes qui ne changent pas ne portent que les deux couches fixes.

**Le piège à connaître :** une fois l'animation finie, il ne faut pas laisser les volets
dessinés par-dessus les moitiés fixes. Les deux couches portent alors le même glyphe, leurs
bords antialiasés se composent deux fois et l'encre s'épaissit visiblement. Rendu à
`phase = 1`, on voit le fantôme de l'ancien chiffre derrière le nouveau. La règle est donc :
à la fin de la bascule, la nouvelle valeur devient la valeur des deux moitiés fixes et les
deux volets cessent d'exister.

### La surface d'encre reste constante

DESIGN.md §5 point 4 affirme qu'aucun clignotement de luminance ne doit se produire pendant la
bascule, une moitié se cachant pendant que l'autre se révèle. Compté sur les sept images,
nombre de pixels dont le rouge dépasse 128 :

```
phase 0    0,15    0,3     0,44    0,6     0,8     1
232617  234293  234075  232620  238244  239466  239307
```

Variation maximale 2,9 %, et elle tient au fait que « 5 » porte un peu plus d'encre que « 4 ».
L'affirmation est vérifiée.

### Un désaccord à trancher, qui n'est pas technique

La commande demande que « la lumière glisse sur la face pendant la rotation — la luminosité
suit l'angle ». DESIGN.md §5 écrit l'inverse : « aucun assombrissement de la face qui
tourne. Seule l'encre bouge ; c'est toute l'idée. » §8 ajoute : « pas de face de volet
assombrie "pour le réalisme" ».

Les deux sont implémentés et pilotés par un seul paramètre, `lumiere ∈ {0, 1}` :
`volet_0.3.png` (lumière active, l'encre du volet descend à 79 % à 45°) et
`volet_sans_lumiere.png` (encre constante). Le code est le même à un `mix` près, la décision
reste ouverte. Une lecture qui réconcilie les deux textes : la carte étant noire sur noir,
seule l'**encre** peut prendre la lumière ; assombrir l'encre n'est pas assombrir la face.

**Arbitré depuis, et écrit dans la loi.** Le socle garde la lumière, DESIGN.md §5 et §8 la
décrivent désormais, et §2 la nomme comme sa seule dérogation aux cinq gris. Le plancher est
0,65 en linéaire — `#C8C5C0` — et non 0,30 : plus bas, le chiffre se lisait en deux encres
différentes à mi-bascule.

---

## 8. Cadence

Mesuré sur 100 présentations, après rejet des vingt premières.

```
moniteur : frequences annoncees = [60000, 75000, 100000, 120000, 144000] mHz
Fifo      : intervalle median 6,98 ms, moyen 7,47 ms (134 Hz), min 6,28, max 14,62
Immediate : intervalle median 1,20 ms, moyen 1,20 ms (834 Hz), min 1,06, max 1,39
```

`Fifo` cale sur l'écran : 6,98 ms de médiane, soit 143,3 Hz sur un écran à 144 Hz. Les
maxima à 14,6 ms sont des trames doublées d'une compilation de débogage qui efface tout
l'écran à chaque tour. `Immediate` ne synchronise rien et tourne à 834 Hz : à proscrire.
`Fifo` est disponible sur les six combinaisons dorsal/adaptateur ; `Mailbox` ne l'est pas
(absent sur Vulkan/Intel). **`PresentMode::Fifo`, donc, sans condition.**

Pour « pas de dessin quand rien ne change », `ControlFlow::WaitUntil` réglé sur la prochaine
frontière de seconde de l'horloge système, réarmé à chaque tour — jamais un intervalle fixe,
comme l'exige DESIGN.md §5. Précision du réveil, six secondes de suite, écart par rapport à
la frontière :

```
reveils : [0.099, 0.201, 0.19, 0.3, 0.41, 0.152] ms — ecart maximal 0.410 ms
```

Toujours **en retard**, jamais en avance, et de moins d'une demi-milliseconde. Le budget de
DESIGN.md est de 50 ms : il y a cent fois la marge. Le réveil se lit dans
`ApplicationHandler::new_events` sur
`StartCause::ResumeTimeReached`, et c'est de là que part la demande de redessin.

---

## 9. Ce qui reste ouvert

- **La sélection d'adaptateur est le point fragile du projet.** Sur cette machine, un seul
  couple sur six laisse passer l'alpha. Rien ne garantit qu'un autre poste se comporte de
  même ; le repli par `SetWindowRgn` (§1.5) doit être écrit dès le premier jour, pas ajouté
  après coup.
- Le crénelage des coins arrondis en mode repli n'a pas été mesuré à l'œil sur fond noir en
  chambre sombre. À juger sur pièce avant de décider s'il est acceptable.
- Le seuil des 4 px qui sépare le clic du glissement (DESIGN.md §6) doit être implémenté
  avant l'appel à `drag_window`, pas après : une fois la boucle modale lancée, on ne voit
  plus passer les mouvements.
- Le ressort à amortissement critique de l'îlot (raideur 400, amortissement 40, masse 1) n'a
  pas été implémenté ici : seules les courbes de Bézier du volet l'ont été. L'intégration du
  ressort, et surtout son interruptibilité — la reprise depuis position **et vitesse**
  courantes — reste à écrire et à vérifier.

---

## Les essais

Huit programmes, dans un dossier temporaire, chacun sortant seul après avoir imprimé sa
mesure : `probe` (attributs de fenêtre, capacités de toutes les surfaces), `transparence`
(alpha réel, convention de DWM, comparaison des six dorsaux, contrôle `WS_EX_LAYERED`),
`clics` (hit-test, `WindowFromPoint`, surdité), `pleinecran` (plein écran, ordre de
superposition), `deplacement` (glisser réel par `SendInput`, mémorisation), `galet`
(`SetWindowRgn`), `capture` (police, tabulaires, texture hors-écran, volet 3D, PNG),
`cadence` (Fifo, précision de `WaitUntil`).

Les trois premiers utilisent une photographie de l'écran par `BitBlt` + `GetDIBits` pour
juger le résultat composé par DWM plutôt que ce que l'application croit avoir dessiné : c'est
la seule façon honnête de vérifier une transparence.
