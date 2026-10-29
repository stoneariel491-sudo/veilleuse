//! L'ilot : la petite fenetre qui porte le chrono, toujours au-dessus.
//!
//! Replie, c'est un galet qui ne montre que le temps restant. Deploye, il porte
//! les TROIS lignes de DESIGN.md §6, et rien de plus : le temps en grand avec la
//! phase a droite, le bloc en cours et ses pages, deux actions. DEUX ressorts
//! pilotent la transformation, un par axe et tous deux en pixels (DESIGN.md §6).
//!
//! Le fichier est coupe en deux, et la coupure compte :
//!
//! - `Vitrine` compose et peint, et ne connait AUCUNE fenetre. C'est ce qui
//!   permet de la rendre hors-ecran et de la juger en PNG, exactement comme
//!   l'horloge — sans quoi l'ilot serait la seule partie du projet qu'on ne
//!   pourrait regarder qu'en la faisant tourner.
//! - `Ilot` ajoute la fenetre, le ressort, la souris et la region.
//!
//! Deux decisions structurantes, prises d'apres ARCHITECTURE.md :
//!
//! 1. **La fenetre garde toujours sa taille deployee** (320 x 148) et c'est le
//!    GALET qui grandit dedans. Redimensionner la fenetre a chaque pas du
//!    ressort obligerait a reconfigurer la chaine d'echange une trentaine de
//!    fois par clic.
//! 2. **Une region GDI, recalculee a chaque pas, restreint la fenetre au galet.**
//!    C'est elle qui fait que les clics traversent partout ailleurs : sans elle,
//!    l'ilot replie volerait 320 x 148 pixels de souris a l'ecran alors qu'il
//!    n'en peint que 88 x 36. ARCHITECTURE.md §2 avait identifie ce trou — la
//!    « zone morte » — et n'en voyait la sortie que par une interrogation
//!    periodique de `GetCursorPos` ; la region l'evite entierement, parce
//!    qu'elle agit avant le test de survol de Windows.
//!
//! **La region est tiree de la meme superellipse que le nuanceur**, rangee par
//! rangee (`galet::silhouette`). Ce n'est pas un rafinement : `SetWindowRgn`
//! ROGNE ce qui est peint, donc une region qui ne suit pas la courbe recoupe le
//! coin que le nuanceur vient de dessiner. Une region de
//! `CreateRoundRectRgn` — un arc de CERCLE — le faisait : a rayon egal la
//! superellipse passe partout a l'exterieur du cercle, et les mesures a l'ecran
//! donnaient un exposant ajuste de 1,9 la ou le nuanceur calculait 4, avec pres
//! de vingt pixels mordus sur la premiere rangee. La silhouette est celle du
//! PREMIER PIXEL ENCRE, ruban de transition compris : le bord adouci tombe donc
//! entierement a l'interieur, et l'anticrenelage du nuanceur survit a la
//! decoupe au lieu d'etre remplace par une marche d'escalier d'un bit.

use std::path::PathBuf;
use std::sync::Arc;

use winit::window::Window;

use crate::chrono::{Action, Chrono, ACTIONS};
use crate::galet::{RenduGalet, Uniformes};
use crate::gpu::Gpu;
use crate::palette;
use crate::panneau::{Ancre, Fonte, Ligne, Panneau};
use crate::ressort::Ressort;
use crate::texte::Typographie;

// --- La geometrie de DESIGN.md §6, en pixels logiques ------------------------

/// Replie : la moitie de la hauteur en rayon, donc un galet — la seule forme
/// qui ne dessine pas de cadre.
pub const REPLIE_H: f32 = 36.0;
const REPLIE_RAYON: f32 = REPLIE_H / 2.0;
/// Marges interieures « 0 18 px » : rien en haut ni en bas, la largeur suit le
/// contenu.
const REPLIE_MARGE_X: f32 = 18.0;
const REPLIE_CORPS: f32 = 22.0;

/// Exposants de la superellipse, replie puis deploye. Ils sont choisis par
/// l'essai, et la planche qui tranche est `captures/orfevre/2*`.
///
/// **Replie**, le galet fait 88 x 36 et son rayon de 18 EST la demi-hauteur :
/// c'est une pastille, il n'y a aucun bord vertical droit, donc aucune jonction
/// ou une cassure de courbure pourrait se voir — et le rayon ne peut pas monter
/// pour compenser, 18 est le maximum geometrique. Au-dessus de 2,5 la pastille
/// s'aplatit en barre a coins mous et se met a dessiner un cadre, ce que
/// DESIGN.md §6 lui interdit d'etre. 2,5 est le dernier exposant qui se lit
/// encore comme un galet.
const REPLIE_N: f32 = 2.5;
/// **Deploye**, il reste 92 px de bord vertical droit de part et d'autre du
/// coin : la jonction existe vraiment, et c'est la que la cassure du cercle se
/// voit. Sur `22-planche-deploye-x8.png` — les six exposants, coin agrandi huit
/// fois, rayon identique — l'entree dans le virage s'adoucit jusqu'a 4, puis a
/// 5 le coin est presque carre et le galet redevient une boite. 4 est le
/// dernier exposant qui courbe encore.
pub const DEPLOYE_N: f32 = 4.0;

pub const DEPLOYE_L: f32 = 320.0;
pub const DEPLOYE_H: f32 = 148.0;
/// 48 et non les 28 de DESIGN.md §6, et c'est le rayon qui GARDE la silhouette
/// ecrite la, pas qui s'en ecarte.
///
/// A rayon egal, une superellipse mange beaucoup moins de coin qu'un cercle :
/// 0,0730 r² retires a n = 4 contre 0,2146 r² a n = 2, soit 34 %. Garder 28
/// avec n = 4 aurait donc rendu le galet nettement plus carre qu'aujourd'hui —
/// c'est la deuxieme case de `26-planche-rayon-x7.png`, et le coin y a presque
/// disparu. L'egalite des aires retirees demande 1,715 x 28 = 48,0, et la
/// planche le confirme a l'oeil : a r = 48 la rondeur apparente est celle du
/// cercle d'aujourd'hui. `27-entier-cercle-vs-n4r48-x2.png` pose les deux
/// galets entiers l'un sous l'autre — meme silhouette, courbure continue.
///
/// 48 tient dans la geometrie : la demi-hauteur deployee vaut 74, et
/// l'interpolation 18 -> 48 reste sous la demi-hauteur 18 -> 74 tout du long.
pub const DEPLOYE_RAYON: f32 = 48.0;
/// Elargissement maximal du ruban du bord, en pixels logiques, atteint a la
/// vitesse de pointe du deploiement. Valeur tranchee sur planche : voir
/// `captures/orfevre/40-planche-adoucissement.png`.
const ADOUCI_MAX: f32 = 3.0;

const DEPLOYE_MARGE_X: f32 = 24.0;
const DEPLOYE_MARGE_Y: f32 = 20.0;

const CORPS_TEMPS: f32 = 44.0;
const CORPS_PHASE: f32 = 11.0;
const INTERLETTRAGE_PHASE: f32 = 0.24;
const CORPS_SOUS: f32 = 14.0;
const CORPS_ACTION: f32 = 11.0;
const INTERLETTRAGE_ACTION: f32 = 0.18;
const ECART_ACTIONS: f32 = 28.0;

/// Ecart minimal entre le temps et la phase, qui partagent leur ligne de base.
/// A la geometrie finale il en reste 105 px ; 16 px est le plancher sous lequel
/// les deux mots cesseraient d'etre deux, et il ne sert qu'a mi-ouverture.
const ECART_TEMPS_PHASE: f32 = 16.0;

/// Ecart entre la ligne de base des grands chiffres et celle de la sous-ligne.
///
/// 28 px sous une ligne de base de 44 px placent le haut de capitale de la
/// sous-ligne a 18 px des chiffres : assez pres pour qu'on lise les deux comme
/// un seul bloc. Les actions, elles, restent collees a la marge du bas et
/// laissent 41 px de noir : c'est cet ecart, et rien d'autre, qui les separe du
/// contenu. Pas de trait, pas de fond (DESIGN.md §8).
const ECART_SOUS: f32 = 28.0;

/// Hauteur de l'encre des chiffres de Barlow Condensed : le plus haut des dix
/// (le 8, a 714/1000 em), comme dans `geometrie.rs`.
const HAUT_CHIFFRES: f32 = 0.714;
/// Hauteur de capitale de Barlow (sCapHeight = 700/1000 em).
const CAPITALE: f32 = 0.70;

/// Fenetres de fondu du contenu, en p (DESIGN.md §6). Entre les deux, le galet
/// est vide, et c'est voulu.
const FONDU_REPLIE_FIN: f32 = 0.35;

/// Debut du fondu du contenu deploye — la valeur de DESIGN.md §6.
///
/// Ce seuil a ete pousse a 0,85 pour qu'aucune lettre ne soit tranchee par le
/// bord du galet. La mesure etait juste, la conclusion non. Sur le ressort a
/// amortissement critique de §6 (w = 20 rad/s), p = 0,35 tombe a 62 ms et
/// p = 0,85 a 168 ms : plus de cent millisecondes pendant lesquelles le galet
/// grandit VIDE. Ce n'est pas un fondu, c'est un panneau qui charge.
///
/// A 0,55, le contenu replie s'eteint a 62 ms et le contenu deploye s'allume a
/// 92 ms : trente millisecondes de galet vide, ce que §6 appelle voulu.
///
/// Le seuil ne tient que parce que le contenu s'ouvre AVEC le galet en largeur
/// (voir `composer`) : a 0,55 les deux premieres lignes sont deja entieres dans
/// le noir, et seules les actions attendent que la hauteur descende jusqu'a
/// elles, a p = 0,82 (157 ms). Pose a sa geometrie finale, le contenu se serait
/// fait trancher le 4 de « 41:27 » pendant soixante-cinq millisecondes.
const FONDU_DEPLOYE_DEBUT: f32 = 0.55;

/// Au-dela de ce deploiement, les actions sont assez lisibles pour qu'on puisse
/// les viser. En deca, un clic ne fait que deployer ou replier.
const SEUIL_ACTIONS: f32 = 0.5;

/// Un deplacement de plus de 4 px n'est pas un clic (DESIGN.md §6).
const SEUIL_GLISSER: f64 = 4.0;

/// La bordure du theme de contraste, en pixels logiques comme toutes les cotes
/// de l'ilot (DESIGN-ILOT.md §8.1 et m13 : 2, 2,5, 3 et 4 px physiques).
const BORDURE_CONTRASTE: f32 = 2.0;

/// Un appui du bouton gauche en cours (DECISION-APRES-EPREUVE.md §B) : on ne
/// sait pas encore s'il sera un clic (relache avant le delai), un appui
/// maintenu (le delai atteint, bouton enfonce : le galet replie se deploie) ou
/// un glissement (plus de 4 px logiques : l'ilot se deplace).
struct Appui {
    origine: (f64, f64),
    depuis: std::time::Instant,
    /// Le delai est atteint et le deploiement est parti : le relachement qui
    /// suivra ne vaut pas clic.
    tenu: bool,
}

/// Le compte des clics du triple clic (`GESTE-TRIPLE-CLIC.md` §5).
///
/// Il ne s'ouvre que sur le **galet replie**, et seulement quand il y a une
/// matiere a basculer : deploye, le mot VERRE est a un clic, et un second
/// chemin au meme endroit ne se justifie pas ; verre impossible, il n'y a rien
/// a basculer et le clic garde son action immediate.
///
/// Tant qu'il est ouvert, l'action du clic — le retour de l'horloge — n'est
/// **jamais jouee**. Elle l'est a la fermeture du compte s'il tient un ou deux
/// clics, et jamais s'il en tient trois. C'est la seule sortie que la mesure
/// laisse : l'action du premier clic retire l'ilot, et la fenetre traversante
/// du mouvement mangerait de toute facon les deux clics suivants
/// (`GESTE-TRIPLE-CLIC.md` §2.3).
struct Compte {
    /// Le point du PREMIER appui, en pixels physiques de la fenetre. C'est a
    /// lui que les suivants se comparent, comme Windows compare au premier clic
    /// d'un double clic — pas au precedent.
    origine: (f64, f64),
    /// Le dernier appui. Le delai se mesure d'appui a appui, comme Windows.
    dernier_appui: std::time::Instant,
    /// Le premier appui, pour dire la duree du geste au journal.
    premier_appui: std::time::Instant,
    /// Combien d'appuis comptes : 1, 2, puis 3.
    clics: u32,
    /// Le dernier appui est relache. Tant que le bouton est enfonce, c'est
    /// l'appui maintenu qui decide, et le compte n'echoit pas.
    relache: bool,
}

/// Ce qu'un relachement ou une touche demande a l'application.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Geste {
    /// Une action du chrono (ou la matiere), l'ilot reste deploye.
    Action(Action),
    /// Le clic sur le galet replie : l'horloge revient devant, et la regle de
    /// presence retire l'ilot (§B, « on peut cliquer dessus juste pour voir le
    /// tout reaparaitre proprement »).
    RendreLHorloge,
}

/// L'avance de la decoupe sur l'ecran, en pixels physiques, sous laquelle la
/// fenetre reprend les clics et pose sa region (DESIGN-ILOT.md §9.7, la regle
/// du demi-pixel) : la chaine d'echange garde deux trames d'avance, et sous un
/// demi-pixel la decoupe tombe dans le pixel d'anticrenelage du bord.
const DEMI_PIXEL: f32 = 0.5;
const TRAMES_D_AVANCE: f32 = 2.0;

/// Marge invisible ajoutee autour du mot d'une action pour en faire une cible
/// atteignable. Un mot en capitales de 11 px n'a que 8 px de haut ; viser 8 px
/// a la souris, de nuit, est une punition. Rien n'est dessine de plus :
/// DESIGN.md §8 interdit un fond ou un contour, pas une zone sensible plus
/// large que son mot.
const MARGE_CIBLE_X: f32 = 6.0;
const MARGE_CIBLE_Y: f32 = 9.0;

/// Position par defaut : centre horizontalement, a 24 px du bord haut de l'ecran
/// principal — la zone que l'horloge laisse vide au-dessus de la date.
/// ~~24~~ 6 px logiques du haut de la zone de travail (DESIGN.md §6, « Place » ;
/// DESIGN-ILOT.md §2.1, m7 et m13) : le galet replie de 36 px centre dans la
/// barre de titre haute de Windows, 48 px. Ils ne suivent pas la taille du
/// texte du systeme (N7).
const MARGE_HAUT_DEFAUT: f64 = 6.0;

/// La course du ressort du fondu d'apparition (DESIGN-ILOT.md §1.5) : cent,
/// comme la matiere, pour que les seuils de repos de `ressort.rs` tombent au
/// millieme de la course.
const COURSE_FONDU: f32 = 100.0;

// --- Les lignes, a rang fixe -------------------------------------------------
// Le rang est fixe pour que chaque ligne garde SON tampon d'une trame a
// l'autre : un tampon qui change de rang serait refaconne pour rien.
const L_TEMPS_REPLIE: usize = 0;
const L_TEMPS: usize = 1;
const L_PHASE: usize = 2;
const L_SOUS: usize = 3;
const L_ACTIONS: usize = 4;

/// Le galet a un instant donne du deploiement, en pixels de fenetre.
#[derive(Clone, Copy, Debug)]
pub struct Mesures {
    /// Le deploiement, dans [0, 1], relu sur la hauteur.
    pub p: f32,
    pub x: f32,
    pub y: f32,
    pub l: f32,
    pub h: f32,
    pub rayon: f32,
    /// Exposant de la superellipse a cet instant du deploiement.
    pub n: f32,
    /// Vitesse du point le plus vif du bord (un coin du bas, en diagonale),
    /// normalisee sur la vitesse de pointe d'une course complete de la
    /// hauteur et bornee a un. Zero aux deux repos.
    pub vitesse: f32,
    /// M3 : la vitesse d'un bord lateral (la moitie de celle de la largeur) et
    /// celle du bord bas (celle de la hauteur), en pixels par seconde.
    pub v_cote: f32,
    pub v_bas: f32,
}

impl Mesures {
    /// A partir des DEUX ressorts, en pixels.
    ///
    /// p — donc le rayon, l'exposant et les fondus du contenu — se lit sur la
    /// HAUTEUR, et c'est un choix : les deux bornes de la hauteur sont fixes
    /// (36 et 148 px logiques), alors que celles de la largeur bougent avec le
    /// contenu. Un p tire de la largeur changerait de sens quand le temps
    /// restant passe de « 1:05:00 » a « 59:59 ».
    fn depuis(l: f32, h: f32, vitesse_l: f32, vitesse_h: f32, echelle: f32, fenetre_l: f32) -> Mesures {
        let (v_cote, v_bas) = (vitesse_l.abs() / 2.0, vitesse_h.abs());
        // Largeur et hauteur sont arrondies au pixel : la region GDI ne connait
        // que des entiers, et le nuanceur doit decouper exactement la ou elle
        // coupe, sinon le bord adouci se ferait trancher.
        let (l, h) = (l.round(), h.round());
        // Un ressort a amortissement critique ne depasse pas, mais une cible qui
        // bouge sous lui peut le laisser un cheveu hors des bornes ; le galet ne
        // doit pas sortir de sa fenetre pour autant.
        let p = ((h / echelle - REPLIE_H) / (DEPLOYE_H - REPLIE_H)).clamp(0.0, 1.0);
        let entre = |a: f32, b: f32| a + (b - a) * p;
        Mesures {
            p,
            // La largeur croit symetriquement, le bord haut reste fixe.
            x: ((fenetre_l - l) / 2.0).round(),
            y: 0.0,
            l,
            h,
            rayon: entre(REPLIE_RAYON * echelle, DEPLOYE_RAYON * echelle),
            // L'exposant se pilote comme le rayon : une interpolation lineaire
            // de plus sur le meme p, un f32 de plus dans l'uniforme, et rien
            // d'autre. La pastille repliee et le galet deploye n'ont pas le
            // meme probleme de coin, ils n'ont donc pas le meme exposant.
            n: entre(REPLIE_N, DEPLOYE_N),
            // Normalisee sur la vitesse de pointe d'une course complete : sans
            // cet etalon, la valeur changerait de sens avec le facteur
            // d'echelle de l'ecran, puisqu'elle est en pixels par seconde.
            // Le bord le plus vif est un COIN du bas : son point va en
            // diagonale, a la somme des deux vitesses, et son ruban (M3,
            // `|v.n|` avec n le long de v) depasse ceux des bords droits ;
            // c'est lui que la decoupe et l'oeil doivent compter (mesure
            // `pas6/oeil-382b0c2b` : 10 a 16 pixels hors du seuil des bords).
            vitesse: (v_cote.hypot(v_bas)
                / crate::ressort::vitesse_de_pointe((DEPLOYE_H - REPLIE_H) * echelle))
            .clamp(0.0, 1.0),
            v_cote,
            v_bas,
        }
    }

    /// La geometrie que les deux ressorts auraient a l'arret sur `p`. Elle ne
    /// sert qu'a la capture, qui fige un instant sans faire tourner de ressort.
    fn figee(p: f32, largeur_replie: f32, echelle: f32, fenetre_l: f32) -> Mesures {
        let p = p.clamp(0.0, 1.0);
        let entre = |a: f32, b: f32| a + (b - a) * p;
        Mesures::depuis(
            entre(largeur_replie, DEPLOYE_L * echelle),
            entre(REPLIE_H * echelle, DEPLOYE_H * echelle),
            // La capture fige un instant et n'a donc aucun ressort a lire : on
            // lui rend la vitesse qu'un deploiement complet AURAIT en passant
            // par ce p, sur chacun des deux axes (les deux ressorts ont la meme
            // pulsation et passent par p ensemble). Sans cela, le ruban du bord
            // serait toujours au repos et l'on ne pourrait pas regarder ce
            // qu'on vient d'ecrire.
            crate::ressort::vitesse_normalisee_a(p)
                * crate::ressort::vitesse_de_pointe(DEPLOYE_L * echelle - largeur_replie),
            crate::ressort::vitesse_normalisee_a(p)
                * crate::ressort::vitesse_de_pointe((DEPLOYE_H - REPLIE_H) * echelle),
            echelle,
            fenetre_l,
        )
    }
}

/// Pose une ligne deja faconnee : sa position, son ancrage, son opacite. Rien de
/// tout cela n'oblige a refaconner quoi que ce soit.
fn poser(ligne: &mut Ligne, x: f32, base: f32, ancre: Ancre, opacite: f32) {
    ligne.x = x;
    ligne.base = base;
    ligne.ancre = ancre;
    let c = ligne.couleur;
    ligne.couleur = glyphon::Color::rgba(
        c.r(),
        c.g(),
        c.b(),
        (opacite.clamp(0.0, 1.0) * 255.0) as u8,
    );
}

// --- Ce qui se peint, sans fenetre -------------------------------------------

pub struct Vitrine {
    galet: RenduGalet,
    panneau: Panneau,
    lignes: Vec<Ligne>,
    echelle: f32,
    taille: (u32, u32),
    alpha_par_pixel: bool,
    /// Exposant impose a la main, pour la planche qui a tranche celui des coins.
    /// C'est un instrument de mesure, comme `--phase` : sans lui, le choix se
    /// ferait de memoire au lieu de se faire sur piece.
    pub exposant: Option<f32>,
    /// Rayon impose a la main, meme raison et meme planche que `exposant`.
    pub rayon: Option<f32>,
    /// Zones sensibles des trois actions, en pixels de fenetre.
    zones: Vec<(Action, [f32; 4])>,
    /// Quelles actions le chrono autorise, relevees au faconnage : c'est la que
    /// le chrono est encore sous la main, et le placement en a besoin ensuite.
    disponibles: Vec<bool>,
    survol: Option<Action>,
    /// L'action au focus du clavier (Tab, Maj+Tab), peinte comme au survol :
    /// `#F2EFE9` sans contour (DECISION-APRES-EPREUVE.md, §2 du juge, m11).
    focus: Option<Action>,
    /// Le theme de contraste du systeme, s'il est actif (DESIGN-ILOT.md §8.1) :
    /// le galet dans ses couleurs, les textes en `WindowText`, l'action
    /// survolee en `HighlightText` sur `Highlight`, ni verre ni gris de la
    /// charte. L'accessibilite passe avant la palette.
    pub contraste: Option<crate::systeme::Contraste>,
    /// `Some(w)` : l'etat se peint en entier dans une image, qui se pose
    /// PAR-DESSUS ce que la vue contient deja, a l'opacite `w` — la seconde
    /// passe du fondu enchaine de « moins de mouvement » (m5). L'opacite de la
    /// vitrine vaut alors 1 : c'est l'image entiere qui prend `w`.
    pub superposer: Option<f32>,
    mesures: Mesures,
    /// Ou tombe le pixel (0, 0) de cette surface dans le bureau virtuel. La
    /// vitrine ne connait aucune fenetre : l'ilot le lui dit a chaque trame,
    /// la capture hors-ecran le pose pour un fond donne. Sans lui, ou sans
    /// fond televerse, le galet reste noir opaque.
    pub origine_fenetre: Option<(i32, i32)>,
    /// Instrument de mesure : le galet sans son contenu, pour comparer sa
    /// seule matiere a celle du banc, qui ne peint aucun texte.
    pub sans_contenu: bool,
    /// La transformation du galet opaque en verre (`GIVRE.md`) : croissance
    /// et fonte dans [0, 1]. `None` est le chemin d'avant : le verre des qu'un
    /// fond est la. Aujourd'hui, seule la capture hors-ecran la pose ; la
    /// fenetre la recevra de `matiere::Matiere` quand l'integration la cablera.
    pub matiere: Option<(f32, f32)>,
    /// D'ou part la fonte, en pixels LOGIQUES du galet deploye : le centre de la
    /// cible du mot clique, ou le centre du galet pour le triple clic (m-g8).
    /// Sans valeur, le centre du galet.
    pub origine_fonte: Option<(f32, f32)>,
    /// La matiere visee : vrai vers le verre. L'encre et le mot la suivent des
    /// le clic qui la change (DESIGN.md §2, « L'encre suit la matiere visee »),
    /// avant que la transformation ait bouge d'un pixel.
    pub matiere_visee: bool,
    /// L'action de la matiere est-elle peinte : le verre est possible (l'alpha
    /// par pixel ; les reglages du systeme s'y ajoutent au pas 7). Sans elle,
    /// ni mot ni cible : il n'y a rien a choisir (DESIGN.md §6).
    pub matiere_possible: bool,
    /// Rien n'est peint : la fenetre attend sa premiere image capturee, ou
    /// l'ilot est retire.
    pub vide: bool,
    /// L'opacite du fondu d'apparition ou de retrait (DESIGN-ILOT.md §1.5),
    /// 1 au repos ; elle multiplie le galet et le contenu.
    pub opacite: f32,
}

impl Vitrine {
    /// Impose la force du lisere (`--lisere`), instrument de la derivation sur
    /// la couverture exacte. `None` rend la constante de `galet.rs`.
    pub fn imposer_lisere(&mut self, force: Option<f32>) {
        self.galet.lisere = force;
    }

    /// Impose l'action sous le pointeur (`--survol`), pour la capture.
    pub fn imposer_survol(&mut self, survol: Option<Action>) {
        self.survol = survol;
    }

    /// L'epreuve du temoin de la table des forces (`--epreuve-echelle`) : la
    /// table est lue a la mauvaise echelle. Jamais posee par la fenetre.
    pub fn imposer_epreuve_echelle(&mut self, epreuve: bool) {
        self.galet.epreuve_echelle = epreuve;
    }

    /// Impose l'intensite de la lampe d'ecran (`--lampe`), instrument des
    /// temoins de la lampe. `None` rend la constante de `galet::lampe`.
    pub fn imposer_lampe(&mut self, intensite: Option<f32>) {
        self.galet.lampe = intensite;
    }

    /// L'epreuve du temoin de la pousse des cristaux (`--epreuve-pousse`) : la
    /// pousse se fait dans la trame, et non dans le fil du lancement.
    pub fn imposer_epreuve_pousse(&mut self, epreuve: bool) {
        self.galet.epreuve_pousse = epreuve;
    }

    /// L'epreuve du temoin des tiges tardives (`--epreuve-tiges`) : la texture
    /// d'avant m-g3.
    pub fn imposer_epreuve_tiges(&mut self, epreuve: bool) {
        self.galet.epreuve_tiges = epreuve;
    }

    /// Le releve de la pousse des cristaux a la premiere transformation (m-g1).
    pub fn pousse_relevee(&self) -> Option<crate::galet::PousseRelevee> {
        self.galet.pousse_relevee()
    }

    /// L'age du rendu du galet, depuis le depart du fil de la pousse.
    pub fn age(&self) -> std::time::Duration {
        self.galet.age()
    }
}

/// La ligne de l'action de la matiere, apres les actions du chrono : « VERRE »
/// sur le galet opaque, « GALET » sur le verre, calee sur la marge droite de la
/// ligne des actions (DESIGN.md §6 ; DECISION-APRES-EPREUVE.md D1). Le point
/// du verre du designer (§6.3) est remplace par ce mot (m6), et l'origine de
/// la fonte par le centre de sa cible (m-g8).
const L_MATIERE: usize = L_ACTIONS + ACTIONS.len();

/// Le geste de la matiere hors du mot (GESTE-TRIPLE-CLIC.md §5), tel que la
/// sous-ligne l'annonce sous le pointeur. ~~`Ctrl+Alt+Maj+G`~~ : l'auteur l'a
/// juge « trop long, c'est nul », et le triple clic sur le galet replie l'a
/// remplace.
pub const GESTE_MATIERE: &str = "triple clic sur le galet";

impl Vitrine {
    pub fn nouvelle(
        gpu: &Gpu,
        typo: &mut Typographie,
        format: wgpu::TextureFormat,
        taille: (u32, u32),
        echelle: f32,
        alpha_par_pixel: bool,
    ) -> Vitrine {
        Vitrine {
            galet: RenduGalet::nouveau(gpu, format),
            panneau: Panneau::nouveau(gpu, typo, format, taille.0, taille.1),
            lignes: Vec::new(),
            echelle,
            taille,
            alpha_par_pixel,
            exposant: None,
            rayon: None,
            zones: Vec::new(),
            disponibles: Vec::new(),
            survol: None,
            focus: None,
            contraste: None,
            superposer: None,
            origine_fenetre: None,
            sans_contenu: false,
            matiere: None,
            origine_fonte: None,
            matiere_visee: false,
            matiere_possible: false,
            vide: false,
            opacite: 1.0,
            mesures: Mesures {
                p: 0.0,
                vitesse: 0.0,
                v_cote: 0.0,
                v_bas: 0.0,
                x: 0.0,
                y: 0.0,
                l: 0.0,
                h: 0.0,
                rayon: 0.0,
                n: REPLIE_N,
            },
        }
    }

    pub fn mesures(&self) -> Mesures {
        self.mesures
    }

    /// Televerse le bureau capture sous le galet et en tire la couche
    /// floutee. Les couches sont allouees a la plus grande region possible :
    /// la fenetre, qui garde toujours sa taille deployee, plus deux marges.
    pub fn televerser_fond(&mut self, gpu: &Gpu, pixels: &[u8], source: crate::galet::Source) {
        let marge = crate::galet::marge(self.echelle).max(0) as u32;
        let plancher = (self.taille.0 + 2 * marge, self.taille.1 + 2 * marge);
        self.galet
            .televerser_fond(gpu, pixels, source, self.echelle, plancher);
    }

    /// Le fond est perdu : le galet redevient noir opaque.
    pub fn oublier_fond(&mut self) {
        self.galet.oublier_fond();
    }

    /// La region du bureau actuellement dans le verre.
    pub fn fond(&self) -> Option<crate::galet::Source> {
        self.galet.source()
    }

    /// Les mesures telles que le NUANCEUR les recoit, rayon et exposant imposes
    /// a la main compris (`--rayon`, `--exposant`).
    ///
    /// La region doit lire celles-ci et non les nominales. Sinon les deux
    /// drapeaux deplacent la peinture sans deplacer la decoupe, et la planche
    /// qui sert justement a trancher la forme juge une silhouette que la
    /// decoupe recoupe — c'est-a-dire le defaut meme qu'elle devait mesurer.
    pub fn forme(&self) -> Mesures {
        let mut m = self.mesures;
        m.rayon = self.rayon.unwrap_or(m.rayon);
        m.n = self.exposant.unwrap_or(m.n);
        m
    }

    /// Largeur du ruban de transition du bord LE PLUS VIF, en pixels, a cet
    /// instant du deploiement : un au repos — l'anticrenelage exact — et
    /// `ADOUCI_MAX` px logiques au plus a la vitesse de pointe (M3 : 3, 3,75,
    /// 4,5 et 6 px physiques aux quatre echelles).
    ///
    /// La region la lit ici, et le nuanceur calcule celle de chaque bord avec
    /// les memes vitesses (`Uniformes::vitesse`) : la silhouette de la decoupe
    /// est celle du premier pixel encre du bord le plus large, et la decoupe
    /// n'est posee que sous le demi-pixel, ou tous les rubans valent un.
    pub fn adoucissement(&self, m: Mesures) -> f32 {
        1.0 + (ADOUCI_MAX * self.echelle - 1.0) * m.vitesse
    }

    /// Largeur de chaque ligne, en pixels. Sert a juger la mise en page sur
    /// pieces plutot qu'a l'oeil.
    pub fn largeurs(&self) -> Vec<f32> {
        (0..self.lignes.len())
            .map(|i| self.panneau.largeur(i).round())
            .collect()
    }

    /// Construit les lignes, les faconne, et rend la largeur que le galet
    /// replie doit avoir.
    ///
    /// C'est la premiere moitie de `composer`, isolee pour une raison precise :
    /// cette largeur est « au contenu » (DESIGN.md §6), donc elle change quand
    /// le temps restant change de nombre de chiffres — « 1:05:00 » puis
    /// « 59:59 ». Le ressort de la largeur travaille en PIXELS : il lui faut
    /// donc cette valeur AVANT de savoir ou il va. Un p normalise l'ignorait, et
    /// le galet sautait de plusieurs pixels a l'instant ou le chiffre tombait.
    pub fn faconner(&mut self, typo: &mut Typographie, chrono: &Chrono) -> f32 {
        let e = self.echelle;
        let basse = chrono.lumiere_basse();

        // DESIGN.md §7 : pendant la lecture le temps est blanc et la phase
        // grise ; livre ferme, les deux echangent. Le nombre recule parce qu'on
        // ne lui demande plus rien ; l'instruction est la seule chose qui brille.
        // Sur le verre, la phase, la sous-ligne et l'action au repos prennent le
        // gris du verre (DESIGN.md §2), et l'encre suit la matiere visee des le
        // clic. Le temps, lui, garde ses encres.
        let (secondaire, repos) = if self.matiere_visee {
            (palette::GRIS_DU_VERRE, palette::GRIS_DU_VERRE)
        } else {
            (palette::SECONDAIRE, palette::LEGENDE)
        };
        let (couleur_temps, couleur_phase) = if basse {
            (palette::SECONDAIRE, palette::ENCRE)
        } else {
            (palette::ENCRE, secondaire)
        };
        // Sous le theme de contraste, une seule encre pour tout (`WindowText`),
        // et `HighlightText` pour l'action survolee ou au focus, dont le galet
        // peint la cible en `Highlight` (DESIGN-ILOT.md §8.1).
        let systeme = |k: [u8; 3]| glyphon::Color::rgb(k[0], k[1], k[2]);
        let texte_systeme = self.contraste.map(|c| systeme(c.texte));
        let texte_surbrillance = self.contraste.map(|c| systeme(c.texte_surbrillance));
        let (couleur_temps, couleur_phase, secondaire, repos) = match texte_systeme {
            Some(t) => (t, t, t, t),
            None => (couleur_temps, couleur_phase, secondaire, repos),
        };
        let en_cible = |a: Action, survol: Option<Action>, focus: Option<Action>| survol == Some(a) || focus == Some(a);

        let temps = chrono.temps_restant();
        let mut lignes = Vec::with_capacity(L_MATIERE + 1);
        // Replie, le temps suit la meme regle : il recule quand le livre est
        // ferme. C'est le seul element du galet, mais 8E8B86 sur noir en gras de
        // 22 px reste parfaitement lisible, et le galet refroidit avec la phase
        // au lieu de mentir sur l'etat.
        lignes.push(Ligne::simple(
            &temps,
            Fonte::Chiffres,
            REPLIE_CORPS * e,
            couleur_temps,
        ));
        lignes.push(Ligne::simple(
            &temps,
            Fonte::Chiffres,
            CORPS_TEMPS * e,
            couleur_temps,
        ));
        lignes.push(
            Ligne::simple(
                chrono.phase_affichee(),
                Fonte::Moyenne,
                CORPS_PHASE * e,
                couleur_phase,
            )
            .espacee(INTERLETTRAGE_PHASE),
        );
        // Sous le pointeur, l'action de la matiere fait dire a la sous-ligne
        // ce que ferait son clic, et l'autre chemin vers la meme bascule
        // (DESIGN.md §6 ; GESTE-TRIPLE-CLIC.md §6). C'est le seul endroit ou le
        // triple clic s'apprend, avec la ligne « Raccourcis » de LISEZ-MOI.md.
        let sous = if self.matiere_possible && self.survol == Some(Action::Matiere) {
            let mot = if self.matiere_visee { "Galet" } else { "Verre" };
            format!("{mot} · {GESTE_MATIERE}")
        } else {
            chrono.sous_ligne()
        };
        lignes.push(Ligne::simple(sous, Fonte::Courante, CORPS_SOUS * e, secondaire));
        self.disponibles.clear();
        for action in ACTIONS {
            // L'action principale est toujours la premiere : « Pause » pendant
            // qu'on travaille, « Reprendre » quand on s'est arrete. C'est celle
            // vers laquelle la main va. Les deux autres se devinent, et
            // s'allument au survol.
            let disponible = chrono.action_disponible(action);
            self.disponibles.push(disponible);
            let vive = disponible
                && (action == Action::Pause || self.survol == Some(action) || self.focus == Some(action));
            let encre = match (texte_systeme, texte_surbrillance) {
                (Some(t), Some(s)) => if disponible && en_cible(action, self.survol, self.focus) { s } else { t },
                _ => if vive { palette::ENCRE } else { repos },
            };
            lignes.push(
                Ligne::simple(action.mot(chrono), Fonte::Moyenne, CORPS_ACTION * e, encre)
                    .espacee(INTERLETTRAGE_ACTION),
            );
        }
        // L'action de la matiere : le mot dit ce que ferait le clic suivant,
        // « VERRE » vers le verre et « GALET » vers le galet opaque, dans les
        // memes encres que les deux autres. Vide quand le verre n'est pas
        // possible, pour que les rangs des lignes ne bougent pas.
        let mot_matiere = if !self.matiere_possible {
            ""
        } else if self.matiere_visee {
            "GALET"
        } else {
            "VERRE"
        };
        let vive = self.survol == Some(Action::Matiere) || self.focus == Some(Action::Matiere);
        let encre = match (texte_systeme, texte_surbrillance) {
            (Some(t), Some(s)) => if vive { s } else { t },
            _ => if vive { palette::ENCRE } else { repos },
        };
        lignes.push(
            Ligne::simple(mot_matiere, Fonte::Moyenne, CORPS_ACTION * e, encre)
                .espacee(INTERLETTRAGE_ACTION),
        );

        // Le faconnage : les largeurs decident de tout ce qui suit.
        self.panneau.preparer(typo, &lignes);
        self.lignes = lignes;
        self.panneau.largeur(L_TEMPS_REPLIE) + 2.0 * REPLIE_MARGE_X * e
    }

    /// Place les lignes deja faconnees dans le galet `m`, et calcule les zones
    /// sensibles. `faconner` doit avoir ete appele pour la meme trame.
    pub fn placer(&mut self, m: Mesures) {
        let e = self.echelle;
        let p = m.p;
        let mut lignes = std::mem::take(&mut self.lignes);

        let a_replie = (1.0 - p / FONDU_REPLIE_FIN).clamp(0.0, 1.0) * self.opacite;
        let a_deploye = ((p - FONDU_DEPLOYE_DEBUT) / (1.0 - FONDU_DEPLOYE_DEBUT)).clamp(0.0, 1.0) * self.opacite;

        // EN LARGEUR, le contenu garde ses marges avec le galet COURANT : « marges
        // interieures 20 px 24 px » (DESIGN.md §6) se mesurent depuis le bord du
        // galet, et le bord du galet bouge. Le contenu s'ouvre donc avec lui, au
        // lieu d'attendre a sa place finale que le noir vienne le decouvrir en
        // lui tranchant les jambages.
        //
        // EN HAUTEUR, non : les trois lignes gardent leurs bases finales. Les
        // tasser dans la hauteur courante — 98 px a p = 0,55 pour un bloc qui en
        // demande 128, de la ligne de base des chiffres a celle des actions —
        // ferait glisser les lignes les unes par rapport aux autres pendant toute
        // l'ouverture. Le bord bas du galet passe donc sur les actions comme un
        // store qu'on descend, et c'est le seul endroit ou la decoupe se voit.
        let f = Mesures::figee(1.0, 0.0, e, self.taille.0 as f32);
        // Plancher : la largeur que le contenu exige pour ne jamais se croiser.
        // Sans lui, un bloc de plus d'une heure poserait « 1:05:00 » (141 px) et
        // « LIVRE FERMÉ » (92 px) l'un sur l'autre a mi-course. Le contenu cesse
        // alors de se serrer et se laisse trancher, ce qui est le moindre mal.
        // L'action de la matiere compte dans le plancher, avec son ecart.
        let largeur_matiere = self.panneau.largeur(L_MATIERE);
        let somme_actions: f32 = (0..ACTIONS.len())
            .map(|i| self.panneau.largeur(L_ACTIONS + i))
            .sum::<f32>()
            + ECART_ACTIONS * e * (ACTIONS.len() - 1) as f32
            + if largeur_matiere > 0.0 { largeur_matiere + ECART_ACTIONS * e } else { 0.0 };
        let besoin = (self.panneau.largeur(L_TEMPS) + ECART_TEMPS_PHASE * e
            + self.panneau.largeur(L_PHASE))
            .max(self.panneau.largeur(L_SOUS))
            .max(somme_actions);
        let utile = (m.l - 2.0 * DEPLOYE_MARGE_X * e).max(besoin);
        let centre = self.taille.0 as f32 / 2.0;
        let gauche = centre - utile / 2.0;
        let droite = centre + utile / 2.0;
        let base_temps = f.y + DEPLOYE_MARGE_Y * e + HAUT_CHIFFRES * CORPS_TEMPS * e;
        let base_sous = base_temps + ECART_SOUS * e;
        let base_actions = f.y + f.h - DEPLOYE_MARGE_Y * e;
        // Le galet courant sert alors de fenetre de decoupe : rien ne peut se
        // peindre hors du noir, ni pendant l'ouverture ni pendant la fermeture.
        let bornes = Some([m.x, m.y, m.l, m.h]);

        // Le contenu replie ne suit PAS la croissance du galet : il reste ou il
        // est et ne fait que s'eteindre. Le centrer dans un galet qui grandit le
        // ferait glisser vers le bas pendant son fondu, mouvement que personne
        // n'a demande.
        let base_replie = m.y + (REPLIE_H * e + HAUT_CHIFFRES * REPLIE_CORPS * e) / 2.0;
        poser(
            &mut lignes[L_TEMPS_REPLIE],
            self.taille.0 as f32 / 2.0,
            base_replie,
            Ancre::Centre,
            a_replie,
        );
        poser(&mut lignes[L_TEMPS], gauche, base_temps, Ancre::Gauche, a_deploye);
        poser(&mut lignes[L_PHASE], droite, base_temps, Ancre::Droite, a_deploye);
        poser(&mut lignes[L_SOUS], gauche, base_sous, Ancre::Gauche, a_deploye);
        for ligne in lignes.iter_mut().skip(L_TEMPS) {
            ligne.bornes = bornes;
        }

        self.zones.clear();
        let haut_mot = CAPITALE * CORPS_ACTION * e;
        let mut x = gauche;
        for (i, action) in ACTIONS.iter().enumerate() {
            let rang = L_ACTIONS + i;
            let largeur = self.panneau.largeur(rang);
            let disponible = self.disponibles.get(i).copied().unwrap_or(false);
            // Une action indisponible n'est pas peinte (DECISION-APRES-EPREUVE.md
            // G, N4) : la palette n'a pas d'encre eteinte, et un mot qu'on ne
            // peut pas cliquer ne justifie pas sa presence.
            poser(&mut lignes[rang], x, base_actions, Ancre::Gauche, if disponible { a_deploye } else { 0.0 });
            if disponible {
                self.zones.push((
                    *action,
                    [
                        x - MARGE_CIBLE_X * e,
                        base_actions - haut_mot - MARGE_CIBLE_Y * e,
                        largeur + 2.0 * MARGE_CIBLE_X * e,
                        haut_mot + 2.0 * MARGE_CIBLE_Y * e,
                    ],
                ));
            }
            x += largeur + ECART_ACTIONS * e;
        }
        // L'action de la matiere, calee sur la marge droite de la meme ligne,
        // avec la meme cible que les deux autres (DECISION-APRES-EPREUVE.md D1 :
        // 53,6 x 25,6 px logiques pour un mot de la largeur de PAUSE).
        poser(&mut lignes[L_MATIERE], droite, base_actions, Ancre::Droite, a_deploye);
        if self.matiere_possible && largeur_matiere > 0.0 {
            self.zones.push((
                Action::Matiere,
                [
                    droite - largeur_matiere - MARGE_CIBLE_X * e,
                    base_actions - haut_mot - MARGE_CIBLE_Y * e,
                    largeur_matiere + 2.0 * MARGE_CIBLE_X * e,
                    haut_mot + 2.0 * MARGE_CIBLE_Y * e,
                ],
            ));
        }

        self.lignes = lignes;
        self.mesures = m;
    }

    /// La cible de l'action de la matiere, en pixels de fenetre, si elle est
    /// peinte : c'est de son centre que part la fonte au clic (m-g8).
    /// La cible de chaque action, en pixels de la fenetre.
    pub fn zones(&self) -> &[(Action, [f32; 4])] {
        &self.zones
    }

    pub fn zone_matiere(&self) -> Option<[f32; 4]> {
        self.zones
            .iter()
            .find(|(a, _)| *a == Action::Matiere)
            .map(|(_, r)| *r)
    }

    /// Les boites des mots du deploye, en pixels de fenetre `[x, y, l, h]`, un
    /// em de haut sous la ligne de base : la phase, la sous-ligne, chaque action
    /// et la matiere. Pour le journal de `--capture-ilot`, et les instruments
    /// qui lisent le fond sous un mot.
    pub fn boites(&self) -> Vec<(String, [f32; 4])> {
        let e = self.echelle;
        let mut boites = Vec::new();
        for (nom, rang, corps) in [
            ("phase".to_string(), L_PHASE, CORPS_PHASE),
            ("sous-ligne".to_string(), L_SOUS, CORPS_SOUS),
            ("action-1".to_string(), L_ACTIONS, CORPS_ACTION),
            ("action-2".to_string(), L_ACTIONS + 1, CORPS_ACTION),
            ("matiere".to_string(), L_MATIERE, CORPS_ACTION),
        ] {
            let Some(l) = self.lignes.get(rang) else { continue };
            let largeur = self.panneau.largeur(rang);
            if largeur <= 0.0 {
                continue;
            }
            let x = match l.ancre {
                Ancre::Gauche => l.x,
                Ancre::Centre => l.x - largeur / 2.0,
                Ancre::Droite => l.x - largeur,
            };
            let h = corps * e;
            boites.push((nom, [x, l.base - h, largeur, h]));
        }
        boites
    }

    /// Les deux moities d'affilee, sur un deploiement FIGE. C'est le chemin de
    /// la capture, qui n'a pas de ressort a faire tourner.
    pub fn composer(&mut self, typo: &mut Typographie, chrono: &Chrono, p: f32) {
        let largeur_replie = self.faconner(typo, chrono);
        let m = Mesures::figee(p, largeur_replie, self.echelle, self.taille.0 as f32);
        self.placer(m);
    }

    /// Peint le galet puis son contenu dans `vue`. `composer` doit avoir ete
    /// appele pour la meme trame.
    pub fn dessiner(&mut self, gpu: &Gpu, typo: &mut Typographie, vue: &wgpu::TextureView) {
        self.dessiner_fois(gpu, typo, vue, 1, None);
    }

    /// La transformation de cette trame, si la matiere n'est pas au repos sur
    /// le chemin d'avant.
    fn transformation(&self, m: Mesures) -> Option<crate::galet::Transformation> {
        let (g, f) = self.matiere?;
        let e = self.echelle;
        // L'origine de la fonte : le centre de la cible du mot clique, ou le
        // centre du galet pour le triple clic (m-g8) ; jamais un point fixe.
        //
        // Sans mot, le centre se lit sur le galet TEL QU'IL EST, et non sur les
        // 320 x 148 du deploye : le triple clic ne vit que sur le galet replie
        // (GESTE-TRIPLE-CLIC.md §5), ou (160 ; 74) logiques tomberaient a
        // 56 px sous une pastille haute de 36, et la fonte partirait de hors du
        // galet. Deploye, les deux expressions donnent le meme point au pixel,
        // et rien de ce qui a ete mesure ne bouge.
        let origine = match self.origine_fonte {
            Some(o) => (m.x + o.0 * e, m.y + o.1 * e),
            None => (m.x + m.l / 2.0, m.y + m.h / 2.0),
        };
        Some(crate::galet::Transformation {
            givre: g.clamp(0.0, 1.0),
            fonte: f.clamp(0.0, 1.0),
            origine,
            galet: (DEPLOYE_L, DEPLOYE_H, DEPLOYE_RAYON, DEPLOYE_N),
        })
    }

    /// La meme trame, le galet trace `fois` fois et la passe horodatee : c'est
    /// l'instrument du cout de la transformation, et le chemin normal passe
    /// ici avec `fois = 1` et sans horodateur, ce qui est exactement la trame
    /// d'avant.
    pub fn dessiner_fois(
        &mut self,
        gpu: &Gpu,
        typo: &mut Typographie,
        vue: &wgpu::TextureView,
        fois: u32,
        horodateur: Option<&wgpu::QuerySet>,
    ) {
        let m = self.mesures;
        // Le fondu enchaine : l'etat se peint dans l'image entiere, posee
        // ensuite sur la vue a l'opacite `w`.
        let image = self.superposer.map(|_| self.galet.image(gpu, self.taille.0, self.taille.1));
        let cible = image.as_ref().unwrap_or(vue);
        // Le theme de contraste, en lineaire pour une surface sRGB : les octets
        // du systeme ressortent tels quels a l'ecran (palette::lineaire).
        let contraste = match self.contraste {
            Some(c) => {
                let lin = |k: [u8; 3]| [palette::lineaire(k[0]), palette::lineaire(k[1]), palette::lineaire(k[2])];
                let (f, t, s) = (lin(c.fond), lin(c.texte), lin(c.surbrillance));
                let cible = self
                    .survol
                    .or(self.focus)
                    .and_then(|a| self.zones.iter().find(|(z, _)| *z == a).map(|(_, r)| *r));
                let (surbrillance, zone) = match cible {
                    Some(r) => ([s[0], s[1], s[2], 1.0], r),
                    None => ([0.0; 4], [0.0; 4]),
                };
                ([f[0], f[1], f[2], 1.0], [t[0], t[1], t[2], BORDURE_CONTRASTE * self.echelle], surbrillance, zone)
            }
            None => ([0.0; 4], [0.0; 4], [0.0; 4], [0.0; 4]),
        };
        let mut uniformes = Uniformes {
                taille: [self.taille.0 as f32, self.taille.1 as f32],
                centre: [m.x + m.l / 2.0, m.y + m.h / 2.0],
                demi: [m.l / 2.0, m.h / 2.0],
                rayon: self.forme().rayon,
                n: self.forme().n,
                alpha_par_pixel: if self.alpha_par_pixel { 1.0 } else { 0.0 },
                adoucissement: self.adoucissement(m),
                opacite: self.opacite,
                bourrage: 0.0,
                vitesse: [m.v_cote, m.v_bas],
                v_pointe: crate::ressort::vitesse_de_pointe((DEPLOYE_H - REPLIE_H) * self.echelle),
                ruban_max: ADOUCI_MAX * self.echelle - 1.0,
                fond: contraste.0,
                bord: contraste.1,
                surbrillance: contraste.2,
                zone: contraste.3,
        };
        self.galet.televerser(gpu, &uniformes);
        // Le verre ne se pose que s'il a un fond ET une place dans le bureau.
        // Sinon, c'est le galet noir d'avant, par le meme tuyau qu'avant.
        let verre = match self.origine_fenetre {
            Some(o) => self.galet.preparer_verre(gpu, o, m.h, self.echelle),
            None => false,
        };
        // La transformation ne se peint que sur du verre possible, et seulement
        // hors de ses deux extremites, qui restent les tuyaux d'avant.
        let mut transformation = if verre { self.transformation(m) } else { None };
        if let Some(t) = transformation {
            if t.au_repos().is_none() {
                let o = self.origine_fenetre.expect("verre pose, donc origine connue");
                let ok = self.galet.preparer_givre(
                    gpu,
                    o,
                    m.h,
                    self.echelle,
                    (m.x + m.l / 2.0, m.y + m.h / 2.0),
                    (m.l / 2.0, m.h / 2.0),
                    &t,
                );
                if !ok {
                    transformation = None;
                }
            }
        }

        let mut enc = gpu
            .peripherique
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("trame de l'ilot"),
            });
        {
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ilot"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: cible,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        // Alpha zero : tout ce que le galet ne couvre pas est
                        // troue, et ce qu'il y a derriere traverse.
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: horodateur.map(|h| wgpu::RenderPassTimestampWrites {
                    query_set: h,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // Vide : la passe efface seulement, la fenetre ne montre rien.
            if !self.vide {
                self.galet.dessiner_fois(&mut passe, verre, transformation, fois);
                if !self.sans_contenu {
                    let Vitrine { panneau, lignes, .. } = self;
                    panneau.dessiner(gpu, typo, lignes, &mut passe);
                }
            }
        }
        gpu.file.submit(Some(enc.finish()));
        if let Some(w) = self.superposer {
            // Deux soumissions : l'opacite de l'image doit changer ENTRE la
            // peinture de l'etat et sa pose, et `write_buffer` s'execute avant
            // les commandes soumises apres lui.
            uniformes.opacite = w;
            self.galet.televerser(gpu, &uniformes);
            let mut enc = gpu
                .peripherique
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("image posee") });
            {
                let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("image posee"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: vue,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                self.galet.poser_l_image(&mut passe);
            }
            gpu.file.submit(Some(enc.finish()));
        }
    }

    /// Refait les passes du flou du verre ou du givre, horodatees : instrument
    /// du cout.
    pub fn refaire_flou(&mut self, gpu: &Gpu, givre: bool, horodateur: &wgpu::QuerySet) {
        if givre {
            self.galet.refaire_flou_du_givre(gpu, horodateur);
        } else {
            self.galet.refaire_flou_du_verre(gpu, horodateur);
        }
    }
}

// --- La fenetre, le ressort, la souris ---------------------------------------

/// De quoi savoir si la region posee vaut encore. Tout ce qui change la
/// silhouette y figure, et rien d'autre.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Empreinte {
    x: i32,
    y: i32,
    l: i32,
    h: i32,
    rayon: i32,
    n: i32,
    ruban: i32,
}

/// La region de decoupe, tiree rangee par rangee de la superellipse du
/// nuanceur.
///
/// GDI ne sait pas decouper une courbe : ses regions sont des reunions de
/// rectangles. On lui en donne un par bande de rangees consecutives ayant la
/// meme silhouette — une centaine pour le galet deploye, deux fois le rayon
/// plus une, le long bord droit ne comptant que pour un.
///
/// `ExtCreateRegion` les prend TOUS D'UN COUP, la ou `CombineRgn(RGN_OR)` en
/// demanderait une centaine d'appels et autant de regions intermediaires. Le
/// tampon est un `Vec<u32>` et non un `Vec<u8>` : `RGNDATA` s'aligne sur quatre
/// octets, et seule l'allocation de mots le garantit.
#[cfg(windows)]
fn region_superellipse(
    m: &Mesures,
    ruban: f32,
) -> Option<windows_sys::Win32::Graphics::Gdi::HRGN> {
    use windows_sys::Win32::Graphics::Gdi::{ExtCreateRegion, RDH_RECTANGLES};

    let colonnes = crate::galet::silhouette(m.l, m.h, m.rayon, m.n, ruban);
    let (x0, y0) = (m.x as i32, m.y as i32);
    let l = m.l.round() as i32;
    if colonnes.is_empty() || l <= 0 {
        return None;
    }

    // Un mot par champ : l'en-tete en fait huit, chaque rectangle quatre.
    let mut mots: Vec<u32> = Vec::with_capacity(8 + 4 * colonnes.len());
    mots.extend_from_slice(&[
        32,                                  // dwSize de l'en-tete
        RDH_RECTANGLES,                      // iType
        0,                                   // nCount : rempli a la fin
        0,                                   // nRgnSize : de meme
        x0 as u32,                           // rcBound
        y0 as u32,
        (x0 + l) as u32,
        (y0 + colonnes.len() as i32) as u32,
    ]);

    let mut rectangles = 0u32;
    let mut debut = 0usize;
    for i in 1..=colonnes.len() {
        // Fin d'une bande : la rangee suivante n'a plus la meme silhouette.
        if i < colonnes.len() && colonnes[i] == colonnes[debut] {
            continue;
        }
        let c = colonnes[debut];
        // Une rangee entierement hors de la forme ne donne pas de rectangle.
        if 2 * c < l {
            mots.extend_from_slice(&[
                (x0 + c) as u32,
                (y0 + debut as i32) as u32,
                (x0 + l - c) as u32,
                (y0 + i as i32) as u32,
            ]);
            rectangles += 1;
        }
        debut = i;
    }
    if rectangles == 0 {
        return None;
    }
    mots[2] = rectangles;
    mots[3] = rectangles * 16;

    let rgn = unsafe {
        ExtCreateRegion(
            std::ptr::null(),
            (mots.len() * 4) as u32,
            mots.as_ptr().cast(),
        )
    };
    if rgn.is_null() {
        None
    } else {
        Some(rgn)
    }
}

pub struct Ilot {
    pub fenetre: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    vitrine: Vitrine,
    /// **Deux ressorts, un par axe, et tous deux en PIXELS.**
    ///
    /// Un seul ressort sur un p normalise marchait tant que les deux bornes de
    /// chaque axe etaient fixes. Elles ne le sont pas : la largeur repliee est
    /// « au contenu » (DESIGN.md §6) et change avec le nombre de chiffres du
    /// temps restant, et les deux bornes changent aussi quand l'ilot passe sur
    /// un moniteur d'un autre facteur d'echelle. Avec un p, deplacer une borne
    /// deplace instantanement le galet ; avec un ressort en pixels, la cible
    /// bouge et le ressort y GLISSE depuis sa position et sa vitesse du moment.
    ///
    /// Les deux gardent la pulsation de DESIGN.md §6 — raideur 400, masse 1,
    /// amortissement critique 40 — donc aucune deformation de travers : a
    /// bornes fixes ils restent exactement synchrones, et la boite d'aujourd'hui
    /// ne bouge pas d'un pixel. Ce qu'ils apportent est ce qui arrive quand une
    /// borne, elle, bouge.
    ressort_l: Ressort,
    ressort_h: Ressort,
    deploye: bool,
    /// Le systeme demande moins de mouvement : le galet prend sa forme d'un
    /// coup, sans ressort (DESIGN.md §5, point 6, applique a l'ilot).
    sans_ressort: bool,
    echelle: f64,
    /// L'appui du bouton gauche en cours, tant qu'on ne sait pas s'il est un
    /// clic, un appui maintenu ou un glissement.
    appui: Option<Appui>,
    /// Le delai de l'appui maintenu et du depart du pointeur
    /// (`SPI_GETMOUSEHOVERTIME`, 400 ms ici), relu avec les reglages.
    delai_appui: std::time::Duration,
    /// Le compte des clics du triple clic, ouvert sur le galet replie seul.
    compte: Option<Compte>,
    /// Le delai entre deux appuis successifs (`GetDoubleClickTime`, 500 ms
    /// ici), relu avec les reglages comme le delai de survol. 400 < 500 : un
    /// appui tenu devient toujours un appui maintenu AVANT que ce delai
    /// n'expire, et les deux gestes ne peuvent pas se disputer un appui.
    delai_double: std::time::Duration,
    /// Le rectangle du double clic (`SM_CXDOUBLECLK` x `SM_CYDOUBLECLK`, 4 x 4
    /// ici), en pixels PHYSIQUES : mesure, ces metriques ne suivent pas le DPI
    /// (`src/systeme.rs`, `rectangle_du_double_clic`). Il ne se multiplie donc
    /// pas par l'echelle, a la difference des 4 px logiques du glissement.
    rectangle_double: (f64, f64),
    /// Le geste qu'une fermeture de compte met en attente : la boucle le draine
    /// par `geste_differe`, et c'est elle qui le joue. L'ilot ne s'appelle pas
    /// lui-meme l'horloge.
    differe: Option<Geste>,
    /// Le pointeur a quitte la silhouette de l'ilot deploye a cet instant ; il
    /// se replie si le pointeur n'est pas revenu au bout du delai, pour qu'un
    /// depassement bref ne le referme pas (DESIGN-ILOT.md §1.2).
    depart_pointeur: Option<std::time::Instant>,
    /// La fenetre laisse passer les clics (`set_cursor_hittest(false)`) : tant
    /// que l'avance de deux trames depasse un demi-pixel sur un bord, pendant
    /// un fondu, et quand l'oeil fige une mi-course.
    traversant: bool,
    /// Le rafraichissement de l'ecran, en hertz, pour l'avance de deux trames.
    pub hz: f32,
    /// Le systeme permet-il le verre (DESIGN-ILOT.md §8.1 : ni transparence
    /// coupee, ni economiseur de batterie, ni theme de contraste) ? Relu avec
    /// les reglages ; sinon la raison, pour le journal.
    verre_permis: Result<(), &'static str>,
    /// La taille du texte du systeme (N7, `UISettings.TextScaleFactor`, 1 a
    /// 2,25) : toutes les cotes logiques de l'ilot la suivent, comme une
    /// echelle d'ecran locale — `echelle` la contient — sauf les 6 px du haut,
    /// qui derivent de la barre de titre de Windows et ne la suivent pas.
    facteur_texte: f32,
    /// Sous « moins de mouvement » (m5, DESIGN-ILOT.md §8.1), le deploiement
    /// est un FONDU ENCHAINE du galet replie vers l'ilot deploye, sur le
    /// ressort du §6 au lieu de la croissance : ce ressort va de 0 (replie) a
    /// `COURSE_FONDU` (deploye), et la trame peint le replie en entier puis le
    /// deploye par-dessus, a l'opacite de sa position. La forme, elle, se
    /// prend d'un coup (`sans_ressort`) : l'ilot est traversant tant que le
    /// fondu court, comme pendant tout fondu.
    enchaine: Ressort,
    enchaine_trame: std::time::Instant,
    /// L'instant du dernier geste qui a change la cible, pour dater au journal
    /// le passage traversant et son retour.
    geste_a: std::time::Instant,
    /// Derniere position connue du curseur. `WindowEvent::MouseInput` ne la
    /// porte pas : winit ne la livre que dans `CursorMoved`, et c'est ici qu'on
    /// la garde.
    curseur: (f64, f64),
    /// Date de la trame precedente de l'ilot. Elle lui appartient, et n'est pas
    /// celle de l'horloge : les deux fenetres ne dessinent pas aux memes
    /// instants, et l'ilot passe le plus clair de la nuit a ne rien dessiner.
    trame_precedente: std::time::Instant,
    /// La premiere trame n'a pas encore eu lieu.
    ///
    /// La largeur repliee est « au contenu » : elle n'est connue qu'apres le
    /// premier faconnage, et le ressort de la largeur ne peut donc pas etre pose
    /// dessus a la construction. Sans ce drapeau il partirait de sa valeur
    /// provisoire et l'ilot s'ELARGIRAIT de soixante-cinq pixels au lancement,
    /// mouvement que personne n'a demande et qui dirait « il se passe quelque
    /// chose » a l'instant precis ou il ne se passe rien.
    premiere_trame: bool,
    /// Empreinte de la derniere region posee, pour ne pas la reposer a
    /// l'identique : position, taille, rayon, exposant et largeur du ruban.
    region: Option<Empreinte>,
    /// Derniere position ecrite sur le disque.
    position_ecrite: Option<(i32, i32)>,
    /// Le bureau sous le galet, capture a chaque trame de l'ilot et a chaque tic
    /// de la sonde. Le verre le peint des qu'un fond est televerse ; sans fond,
    /// le galet reste noir opaque (`INTEGRATION.md`, pas 2).
    #[cfg(windows)]
    pub suivi: crate::bureau::Suivi,
    /// Le relevé peut suspendre le suivi de `dessiner` pendant qu'il capture
    /// lui-meme une region fixe : sans cela, les deux se disputeraient la
    /// region demandee et chaque tour deviendrait une copie forcee.
    pub suivi_suspendu: bool,
    /// L'affinite d'affichage RELUE apres l'avoir posee : 17 si l'ilot est
    /// exclu de la capture, 0 sinon, `None` si la lecture a echoue.
    pub affinite: Option<u32>,
    /// Le fond televerse dans le verre, garde pour savoir si une copie neuve
    /// apporte vraiment du neuf (voir `suivre_le_bureau`).
    #[cfg(windows)]
    fond: Option<crate::bureau::Region>,
    /// Une copie neuve attend d'etre televersee a la prochaine trame.
    fond_neuf: bool,
    /// Ce que le verre a fait des copies, pour le relevé.
    pub verre_compteurs: CompteursVerre,
    /// L'epreuve du temoin de la boucle : televerser meme un fond identique,
    /// comme si la comparaison des octets n'existait pas. Relevé seulement.
    pub epreuve_sans_comparaison: bool,
    /// L'epreuve du temoin du repli : garder le verre quand la capture est
    /// perdue, c'est-a-dire l'image figee que le repli doit empecher.
    pub epreuve_sans_repli: bool,
    /// Un deploiement fige, pour l'oeil seulement : la fenetre peint le galet a
    /// ce p au lieu de lire ses ressorts. `None` dans l'application.
    pub p_fige: Option<f32>,
    /// La matiere du galet (`matiere.rs`) : deux ressorts en cascade, du noir
    /// opaque au verre. Son etat `(g, f)` est passe a la vitrine a chaque trame ;
    /// posee sur le verre elle donne le verre, posee sur le noir le galet noir.
    matiere: crate::matiere::Matiere,
    /// La matiere est en marche : un ordre l'a mise en route, et elle avance a
    /// chaque trame jusqu'a se poser. Distinct de `Matiere::au_repos`, qui ne
    /// sait pas qu'un ordre vient d'arriver avant le premier pas.
    matiere_en_marche: bool,
    /// Le verre est-il possible sur cette fenetre : l'alpha par pixel (les
    /// reglages du systeme s'y ajoutent au pas 7). Sans lui, ni action de
    /// matiere ni triple clic ; le choix reste retenu, inerte (DESIGN.md §6).
    verre_possible: bool,
    /// La derniere trame de la matiere, pour son dt.
    matiere_trame: std::time::Instant,
    /// L'instant de l'ordre en cours, pour le journal de la premiere image.
    ordre: Option<std::time::Instant>,
    /// Un ordre vers le verre attend la premiere image capturee : la
    /// transformation ne part qu'avec un fond, au plus `ATTENTE_PREMIERE_IMAGE`
    /// apres l'ordre ; passe ce delai, elle ne part pas et le choix reste
    /// retenu.
    transformation_en_attente: Option<std::time::Instant>,
    /// La premiere apparition avec le verre choisi (DECISION-APRES-EPREUVE.md
    /// F, point 6) : la fenetre, region vide et rien de peint, attend la
    /// premiere image capturee, au plus `ATTENTE_PREMIERE_IMAGE`, avant de
    /// paraitre ; passe ce delai elle parait opaque.
    apparition_en_attente: Option<std::time::Instant>,
    /// La premiere apparition est a attendre, et son attente partira au premier
    /// tic de la sonde : entre `resumed` et le premier evenement, la boucle peut
    /// rester morte une seconde ou deux sur une machine chargee (mesure :
    /// `pas4/raccourci-d1f5b750.txt`), et ce temps-la n'est pas celui de la
    /// capture, qui ne peut rien avant.
    apparition_a_attendre: bool,
    /// L'instant de la construction, pour dater la premiere image capturee.
    naissance: std::time::Instant,
    /// La presence (DESIGN.md §6) : l'ilot est la si et seulement si la derniere
    /// fenetre passee au premier plan, l'ilot mis a part, n'est pas l'horloge,
    /// et que rien n'est en plein ecran. C'est la cible ; le fondu la suit.
    presente: bool,
    /// Forcee a « present » par un instrument scripte qui a besoin de l'ilot.
    presence_forcee: bool,
    /// Le fondu d'opacite, porte par le ressort du deploiement (raideur 400,
    /// amortissement 40, masse 1) : 0 retire, `COURSE_FONDU` present.
    fondu: Ressort,
    fondu_trame: std::time::Instant,
    /// Le depart du fondu en cours, pour le journal de sa duree.
    fondu_debut: Option<std::time::Instant>,
    /// L'ecran ou l'ilot est pose.
    ecran: Option<crate::systeme::Ecran>,
    /// Un changement d'ecran en attente : retrait, deplacement quand l'ilot est
    /// invisible, apparition (DESIGN-ILOT.md §1.5).
    destination: Option<(crate::systeme::Ecran, String)>,
    /// Les trames demandees pendant que l'ilot etait retire et pose, et non
    /// presentees (apres la seule trame vide qui pose la region), pour le
    /// relevé : un ilot retire ne presente rien.
    pub trames_retire: u64,
    /// Ce que la regle dit, a part de ce que l'ilot fait : une alerte le
    /// garde deploye le temps du message, meme la ou la regle le retire.
    regle_presente: bool,
    /// Une alerte en cours (la fin d'une lecture, la nuit terminee), jusqu'a
    /// cet instant ; puis l'ilot se replie, et se retire si la regle le dit.
    alerte_jusqu_a: Option<std::time::Instant>,
    /// Un mode scripte ne lit pas la position retenue par l'auteur : ses
    /// images ne doivent pas dependre d'une place laissee par lui (comme le
    /// relevé de la capture), et la place par defaut est ce qu'il mesure.
    pub sans_memoire: bool,
    /// Retire et pose, une trame vide a ete presentee : elle pose la region
    /// vide et efface la surface ; les suivantes n'ont rien a montrer et ne
    /// sont pas presentees (« aucune trame » tant que l'ilot est retire).
    vide_presentee: bool,
}

/// ~~Ce que l'ilot attend d'une premiere image capturee, au plus : deux fois
/// le delai mesure d'une reouverture de la capture (0,117 s, `EPREUVE.md` §5 ;
/// DECISION-APRES-EPREUVE.md F, point 6), 234 ms.~~ Ce chiffre est celui d'une
/// REOUVERTURE, et aucune des deux attentes de l'ilot n'en est une : en
/// matiere opaque la capture est fermee (A3), et un ordre vers le verre
/// l'ouvre pour la premiere fois du processus, comme la premiere apparition ;
/// mesure a 600 a 700 ms sur la machine chargee (`pas4/raccourci-21bca336.txt`,
/// cas 3, abandonne a 234 ms alors que l'image venait). Les deux attentes
/// prennent donc le plafond d'une premiere ouverture, `ATTENTE_PREMIERE_IMAGE`.
/// Pourquoi attendre plutot que partir : partir sans fond ferait peindre le
/// galet noir pendant que g avance (0,68 a 117 ms), et le givre paraitrait
/// d'un coup a mi-course, le « switch brutal » que l'auteur refuse.

/// Ce que la premiere apparition avec le verre choisi attend de la premiere
/// image capturee, au plus. ~~234 ms~~ (DECISION-APRES-EPREUVE.md F, point 6,
/// tire d'une reouverture) : la PREMIERE ouverture de la capture dans le
/// processus prend 357 a 445 ms sur dix lancements a machine calme
/// (`captures/integration-apres-epreuve/pas4/premiere-ouverture-7d87391a.txt`).
/// Par la regle meme de l'arbitre, deux fois le delai mesure : 2 x 445,
/// arrondi a la centaine au-dessus. Ce plafond suppose que la duplication a
/// quelque chose a rendre : une duplication neuve ne rend que les CHANGEMENTS
/// du bureau, et l'ilot cache n'en fait aucun ; sans rien d'autre, la premiere
/// image attendait la presentation suivante de l'horloge, une par seconde
/// (1 026 a 1 354 ms, `pas4/premiere-ouverture-b9c4c29b.txt`). Tant qu'il
/// attend une premiere image, l'ilot demande donc une trame a l'horloge
/// (`Ilot::attend_une_premiere_image`, `main.rs`), qui est a nous. Passe ce
/// delai, la fenetre parait opaque et le verre vient avec la premiere image.
const ATTENTE_PREMIERE_IMAGE: std::time::Duration = std::time::Duration::from_millis(900);

/// Ce que le verre a fait des images que la capture lui a rendues.
#[derive(Clone, Debug, Default)]
pub struct CompteursVerre {
    /// Copies neuves dont les octets egalent le fond deja televerse : rien a
    /// refaire, aucune trame demandee.
    pub identiques: u64,
    /// Copies neuves qui changent le fond : une trame est demandee.
    pub differentes: u64,
    /// Fonds televerses, flou compris.
    pub televersements: u64,
    /// Retours au galet noir parce que la capture est perdue ou fermee.
    pub oublis: u64,
    /// Trames dessinees par l'ilot.
    pub trames: u64,
    /// Tours de capture faits par la sonde du bureau.
    pub sondes: u64,
}

impl Ilot {
    pub fn nouveau(
        gpu: &Gpu,
        fenetre: Arc<Window>,
        surface: wgpu::Surface<'static>,
        typo: &mut Typographie,
        // Les deux drapeaux qui imposent la forme du coin a la main. Ils
        // n'atteignaient que la capture hors-ecran, ce qui les rendait muets la
        // ou ils servent le plus : une planche hors-ecran ne passe ni par la
        // fenetre ni par la decoupe, et c'est justement ce que la planche doit
        // pouvoir juger.
        exposant: Option<f32>,
        rayon: Option<f32>,
        // Invisible a la capture. Toujours vrai dans l'application ; faux
        // seulement pour le temoin qui doit montrer que la capture VOIT l'ilot
        // quand on ne l'exclut pas.
        exclure: bool,
    ) -> Ilot {
        let caps = surface.get_capabilities(&gpu.adaptateur);
        let format = if caps.formats.contains(&wgpu::TextureFormat::Bgra8UnormSrgb) {
            wgpu::TextureFormat::Bgra8UnormSrgb
        } else {
            caps.formats[0]
        };
        // Le marqueur fiable de la transparence n'est pas le mode demande, c'est
        // la presence d'`Inherit` dans les modes annonces (ARCHITECTURE.md §1.4).
        let alpha_par_pixel = caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Inherit);

        let taille = fenetre.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: taille.width.max(1),
            height: taille.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            // Jamais PreMultiplied ni PostMultiplied : aucun adaptateur ne les
            // annonce, et les demander est une erreur de validation qui laisse
            // la configuration precedente en place. `Auto` se traduit tout seul
            // vers le premier mode reellement disponible.
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&gpu.peripherique, &config);

        // L'echelle de l'ilot : celle de l'ecran, multipliee par la taille du
        // texte du systeme (N7). La fenetre est nee a cette taille (main.rs).
        let facteur_texte = crate::systeme::taille_du_texte();
        let echelle = fenetre.scale_factor() * facteur_texte as f64;
        let mut vitrine = Vitrine::nouvelle(
            gpu,
            typo,
            format,
            (config.width, config.height),
            echelle as f32,
            alpha_par_pixel,
        );

        vitrine.exposant = exposant;
        vitrine.rayon = rayon;
        let mut ilot = Ilot {
            fenetre,
            surface,
            config,
            vitrine,
            // Poses sur la geometrie repliee. La largeur exacte n'est connue
            // qu'apres le premier faconnage ; la premiere trame l'ajustera, et
            // elle arrive avant que quoi que ce soit ne s'affiche.
            ressort_l: Ressort::pose(REPLIE_H * echelle as f32),
            ressort_h: Ressort::pose(REPLIE_H * echelle as f32),
            deploye: false,
            sans_ressort: crate::systeme::mouvement_reduit(),
            echelle,
            appui: None,
            delai_appui: crate::systeme::delai_de_survol(),
            compte: None,
            delai_double: crate::systeme::delai_du_double_clic(),
            rectangle_double: crate::systeme::rectangle_du_double_clic(),
            differe: None,
            depart_pointeur: None,
            traversant: false,
            hz: 60.0,
            verre_permis: crate::systeme::verre_permis(),
            facteur_texte: facteur_texte,
            enchaine: Ressort::pose(0.0),
            enchaine_trame: std::time::Instant::now(),
            geste_a: std::time::Instant::now(),
            curseur: (0.0, 0.0),
            trame_precedente: std::time::Instant::now(),
            premiere_trame: true,
            region: None,
            position_ecrite: None,
            #[cfg(windows)]
            suivi: crate::bureau::Suivi::nouveau(),
            suivi_suspendu: false,
            affinite: None,
            #[cfg(windows)]
            fond: None,
            fond_neuf: false,
            verre_compteurs: CompteursVerre::default(),
            epreuve_sans_comparaison: false,
            epreuve_sans_repli: false,
            p_fige: None,
            matiere: crate::matiere::Matiere::posee(false),
            matiere_en_marche: false,
            verre_possible: false,
            matiere_trame: std::time::Instant::now(),
            ordre: None,
            transformation_en_attente: None,
            apparition_en_attente: None,
            apparition_a_attendre: false,
            naissance: std::time::Instant::now(),
            presente: false,
            presence_forcee: false,
            fondu: Ressort::pose(0.0),
            fondu_trame: std::time::Instant::now(),
            fondu_debut: None,
            ecran: None,
            destination: None,
            trames_retire: 0,
            regle_presente: false,
            alerte_jusqu_a: None,
            sans_memoire: false,
            vide_presentee: false,
        };
        ilot.ne_pas_arrondir();
        ilot.ecouter_le_systeme();
        // Avant la premiere trame, donc avant la premiere capture : l'ilot ne
        // doit jamais figurer dans une image du bureau, pas meme une fois.
        ilot.affinite = ilot.exclure_de_la_capture(exclure);
        ilot.preparer_la_matiere();
        ilot
    }

    // --- La matiere, choix de l'auteur -------------------------------------

    /// Le choix retenu (`matiere.txt`) devient l'etat de depart : la matiere
    /// posee sur le verre s'il est choisi et possible, sur le noir sinon ; la
    /// capture n'est ouverte que si le verre est voulu (A3 : le galet opaque
    /// n'ouvre ni sonde, ni duplication, ni televersement). Avec le verre
    /// choisi, la fenetre attend sa premiere image avant de paraitre (N9).
    fn preparer_la_matiere(&mut self) {
        let choisi = crate::matiere::choix_memorise();
        self.verre_permis = crate::systeme::verre_permis();
        self.verre_possible = self.alpha_par_pixel() && self.verre_permis.is_ok();
        self.vitrine.contraste = crate::systeme::theme_de_contraste();
        let verre = choisi && self.verre_possible;
        self.matiere = crate::matiere::Matiere::posee(verre);
        self.vitrine.matiere = Some(self.matiere.etat());
        self.vitrine.matiere_visee = verre;
        self.vitrine.matiere_possible = self.verre_possible;
        #[cfg(windows)]
        self.suivi.empecher_ouverture(!self.capture_voulue());
        println!(
            "matiere : {} ({}), verre {}",
            if choisi { "verre choisi" } else { "galet opaque" },
            match crate::matiere::fichier() {
                Some(f) if f.exists() => format!("{}", f.display()),
                _ => "matiere.txt absent, opaque par defaut".to_string(),
            },
            match (self.verre_possible, self.verre_permis) {
                (true, _) => "possible".to_string(),
                (false, Err(pourquoi)) => format!("impossible : {pourquoi}, choix retenu mais inerte (§8.1)"),
                (false, Ok(())) => "impossible : pas d'alpha par pixel, choix retenu mais inerte".to_string(),
            }
        );
        let _ = verre;
    }

    /// Les reglages du systeme viennent de changer (pas 7, DESIGN-ILOT.md
    /// §8.1) : si le verre cesse d'etre permis, le galet devient opaque d'un
    /// coup, la capture se ferme, l'action de matiere et le triple clic
    /// s'eteignent, le choix reste retenu ; s'il redevient permis et que le
    /// choix retenu est le verre, il revient avec la premiere image capturee.
    fn relire_la_permission_du_verre(&mut self) {
        let contraste = crate::systeme::theme_de_contraste();
        if contraste != self.vitrine.contraste {
            println!(
                "reglages relus : theme de contraste {}",
                match contraste {
                    Some(c) => format!("actif, fond {:?} texte {:?} surbrillance {:?} texte de surbrillance {:?}", c.fond, c.texte, c.surbrillance, c.texte_surbrillance),
                    None => "inactif, la palette de la charte".to_string(),
                }
            );
            self.vitrine.contraste = contraste;
            self.fenetre.request_redraw();
        }
        let permis = crate::systeme::verre_permis();
        if permis == self.verre_permis {
            return;
        }
        self.verre_permis = permis;
        let possible = self.alpha_par_pixel() && permis.is_ok();
        if possible == self.verre_possible {
            return;
        }
        self.verre_possible = possible;
        self.vitrine.matiere_possible = possible;
        if !possible {
            let pourquoi = permis.err().unwrap_or("pas d'alpha par pixel");
            println!("reglages relus : {pourquoi}, le verre n'est plus permis ; galet opaque, choix retenu");
            self.matiere_en_marche = false;
            self.transformation_en_attente = None;
            self.matiere = crate::matiere::Matiere::posee(false);
            self.vitrine.matiere = Some(self.matiere.etat());
            self.vitrine.matiere_visee = false;
            #[cfg(windows)]
            {
                self.suivi.empecher_ouverture(true);
                self.fond = None;
            }
            self.fond_neuf = false;
            self.vitrine.oublier_fond();
        } else {
            let choisi = crate::matiere::choix_memorise();
            println!(
                "reglages relus : le verre est de nouveau permis ; le choix retenu est {}",
                if choisi { "le verre, qui revient avec la premiere image capturee" } else { "le galet opaque" }
            );
            if choisi {
                self.matiere = crate::matiere::Matiere::posee(true);
                self.vitrine.matiere = Some(self.matiere.etat());
                self.vitrine.matiere_visee = true;
                #[cfg(windows)]
                self.suivi.empecher_ouverture(!self.capture_voulue());
            }
        }
        self.fenetre.request_redraw();
    }

    // --- La presence et la place ----------------------------------------------

    /// Un instrument scripte qui a besoin de l'ilot sous les yeux : present
    /// sans fondu ni regle, la ou il est.
    pub fn forcer_presence(&mut self) {
        self.presence_forcee = true;
        self.presente = true;
        self.fondu = Ressort::pose(COURSE_FONDU);
        if self.matiere.vers_le_verre() && self.vitrine.fond().is_none() {
            // La capture avait ete empechee a la construction, l'ilot n'etant
            // pas encore present (`preparer_la_matiere`) : comme `paraitre`,
            // on la laisse s'ouvrir, sans quoi la premiere image n'arrive
            // jamais et l'oeil attend en vain (mesure `pas6/oeil-1868c13f`).
            #[cfg(windows)]
            self.suivi.empecher_ouverture(false);
            self.apparition_a_attendre = true;
        }
        self.fenetre.request_redraw();
    }

    pub fn est_presente(&self) -> bool {
        self.presente
    }

    /// Ce que la regle decide a un changement de premier plan : present sur
    /// cet ecran, ou retire. Une alerte en cours differe le retrait jusqu'a sa
    /// fin (DESIGN.md §6, « Phases » : l'alerte parait meme en plein ecran).
    pub fn appliquer_la_regle(&mut self, presente: bool, ecran: Option<&crate::systeme::Ecran>, raison: &str) {
        self.regle_presente = presente;
        if presente {
            match ecran {
                Some(e) => self.paraitre_sur(e, raison),
                None => self.paraitre(raison),
            }
        } else if self.alerte_jusqu_a.is_some() {
            println!("presence : retrait differe, une alerte est en cours ({raison})");
        } else {
            self.se_retirer(raison);
        }
    }

    /// Une alerte (la fin d'une lecture, la nuit terminee) : l'ilot parait s'il
    /// est retire, sur l'ecran donne, se deploie, et le reste le temps d'un
    /// message du systeme ; puis `finir_l_alerte` le replie, et le retire si la
    /// regle le dit.
    pub fn alerter(&mut self, quoi: &str, duree: std::time::Duration, ecran: Option<&crate::systeme::Ecran>) {
        self.alerte_jusqu_a = Some(std::time::Instant::now() + duree);
        println!("alerte : {quoi}, deploye pour {} s", duree.as_secs());
        if !self.presente {
            match ecran {
                Some(e) => self.paraitre_sur(e, &format!("alerte : {quoi}")),
                None => self.paraitre(&format!("alerte : {quoi}")),
            }
        }
        if !self.deploye {
            self.viser(true);
        }
    }

    /// L'alerte est-elle echue : le repli, et le retrait si la regle le dit.
    fn verifier_l_alerte(&mut self) {
        let Some(fin) = self.alerte_jusqu_a else { return };
        if std::time::Instant::now() < fin {
            return;
        }
        self.alerte_jusqu_a = None;
        self.replier();
        if self.regle_presente {
            println!("alerte : finie, replie");
        } else {
            println!("alerte : finie, replie et retire (la regle le dit)");
            self.se_retirer("fin de l'alerte");
        }
    }

    /// L'ilot parait : le fondu d'opacite part de son opacite courante. Avec
    /// le verre choisi et aucun fond, il attend d'abord sa premiere image
    /// capturee (F, point 6) ; le fondu part avec elle.
    pub fn paraitre(&mut self, raison: &str) {
        if self.presente {
            return;
        }
        self.presente = true;
        println!("presence : parait ({raison})");
        if self.verre_possible && self.matiere.vers_le_verre() && self.vitrine.fond().is_none() {
            #[cfg(windows)]
            self.suivi.empecher_ouverture(false);
            self.apparition_a_attendre = true;
        } else {
            self.lancer_le_fondu();
        }
        self.fenetre.request_redraw();
    }

    /// L'ilot se retire : le fondu d'opacite vers zero, depuis l'opacite et la
    /// vitesse courantes (interruption, DESIGN-ILOT.md §1.5). Retire et pose,
    /// la capture se ferme et rien n'est peint.
    pub fn se_retirer(&mut self, raison: &str) {
        if !self.presente {
            return;
        }
        self.presente = false;
        // L'ilot s'en va : un compte ouvert n'a plus de cible, et son horloge
        // n'a plus a etre rendue — c'est deja elle qui le retire, le plus
        // souvent.
        self.abandonner_le_compte("l'ilot se retire");
        self.apparition_a_attendre = false;
        self.apparition_en_attente = None;
        self.fondu_debut = Some(std::time::Instant::now());
        self.fondu_trame = std::time::Instant::now();
        self.fondu.viser(0.0);
        println!("presence : se retire ({raison})");
        self.fenetre.request_redraw();
    }

    /// Le fondu d'apparition part de l'opacite courante ; « moins de
    /// mouvement » garde les fondus (DESIGN.md §6, « Reglages du systeme »).
    fn lancer_le_fondu(&mut self) {
        self.fondu_debut = Some(std::time::Instant::now());
        self.fondu_trame = std::time::Instant::now();
        self.fondu.viser(COURSE_FONDU);
        self.fenetre.request_redraw();
    }

    /// L'ilot parait sur l'ecran de la fenetre passee devant : a sa place
    /// retenue pour cet ecran, ou a la place par defaut. Deja present sur un
    /// autre ecran, il se retire, se deplace invisible, et parait la-bas.
    pub fn paraitre_sur(&mut self, ecran: &crate::systeme::Ecran, raison: &str) {
        let ailleurs = self.ecran.as_ref().is_some_and(|e| e.hmonitor != ecran.hmonitor);
        if self.presente && ailleurs {
            self.destination = Some((ecran.clone(), raison.to_string()));
            self.se_retirer(&format!("changement d'ecran, vers {}", ecran.nom));
            return;
        }
        if !self.presente || ailleurs {
            self.placer_pour(ecran);
        }
        self.paraitre(raison);
    }

    /// L'echelle d'un ecran, par la poignee que winit connait.
    fn echelle_de(&self, ecran: &crate::systeme::Ecran) -> f64 {
        #[cfg(windows)]
        {
            use winit::platform::windows::MonitorHandleExtWindows;
            for m in self.fenetre.available_monitors() {
                if m.hmonitor() as isize == ecran.hmonitor {
                    return m.scale_factor();
                }
            }
        }
        self.echelle
    }

    /// Pose la fenetre sur cet ecran : la position retenue pour lui, sinon
    /// centree, le bord haut a 6 px logiques du haut de la zone de travail.
    pub fn placer_pour(&mut self, ecran: &crate::systeme::Ecran) {
        let echelle = self.echelle_de(ecran);
        let retenue = if self.sans_memoire { None } else { Ilot::position_memorisee_pour(ecran, echelle) };
        let (x, y) = match retenue {
            Some(p) => p,
            None => {
                // La largeur reelle de la fenetre : la taille du texte du
                // systeme la multiplie (N7), les 6 px du haut non.
                let largeur = (DEPLOYE_L as f64 * echelle * self.facteur_texte as f64).round() as i32;
                let (tx, ty, tl, _) = ecran.travail;
                (tx + (tl - largeur) / 2, ty + (MARGE_HAUT_DEFAUT * echelle).round() as i32)
            }
        };
        println!(
            "place : ecran {} ({}x{} a ({}, {}), zone de travail des ({}, {}), echelle {echelle}) -> ({x}, {y})",
            ecran.nom, ecran.entier.2, ecran.entier.3, ecran.entier.0, ecran.entier.1, ecran.travail.0, ecran.travail.1
        );
        self.ecran = Some(ecran.clone());
        self.fenetre.set_outer_position(winit::dpi::PhysicalPosition::new(x, y));
    }

    /// Retire et pose : rien n'est peint, la capture est fermee.
    fn retiree_au_repos(&self) -> bool {
        !self.presente && self.fondu.au_repos()
    }

    /// Fait avancer le fondu d'une trame, et pose ce qui en decoule : l'opacite
    /// de la vitrine, le vide de l'ilot retire, la capture fermee, et le
    /// deplacement attendu quand l'ilot est invisible.
    fn avancer_le_fondu(&mut self) {
        if !self.fondu.au_repos() {
            let maintenant = std::time::Instant::now();
            let dt = maintenant.duration_since(self.fondu_trame).as_secs_f32();
            self.fondu_trame = maintenant;
            self.fondu.avancer(dt);
            if self.fondu.au_repos() {
                self.fondu = Ressort::pose(if self.presente { COURSE_FONDU } else { 0.0 });
                if let Some(depuis) = self.fondu_debut.take() {
                    println!(
                        "presence : fondu {} fini en {:.0} ms",
                        if self.presente { "d'apparition" } else { "de retrait" },
                        depuis.elapsed().as_secs_f64() * 1e3
                    );
                }
                if !self.presente {
                    // Retire : la capture se ferme, le fond s'oublie.
                    #[cfg(windows)]
                    {
                        self.suivi.empecher_ouverture(true);
                        self.fond = None;
                    }
                    self.fond_neuf = false;
                    self.vitrine.oublier_fond();
                    if let Some((ecran, raison)) = self.destination.take() {
                        self.placer_pour(&ecran);
                        self.paraitre(&raison);
                    }
                }
            }
        }
        self.vitrine.opacite = (self.fondu.position / COURSE_FONDU).clamp(0.0, 1.0);
    }

    /// La premiere apparition est-elle encore devant nous : a attendre, ou en
    /// attente. La fenetre est alors vide, region comprise.
    fn apparition_devant(&self) -> bool {
        self.apparition_a_attendre || self.apparition_en_attente.is_some()
    }

    /// La capture du bureau est-elle voulue : le verre est vise, ou une
    /// transformation est en cours ou attendue. Tant qu'elle ne l'est pas, ni
    /// tour, ni sonde, ni televersement (DECISION-APRES-EPREUVE.md A3).
    fn capture_voulue(&self) -> bool {
        self.verre_possible
            && (self.presente || !self.fondu.au_repos())
            && (self.matiere.vers_le_verre()
                || self.matiere_en_marche
                || self.transformation_en_attente.is_some())
    }

    /// Le verre est-il choisi (la cible de la matiere), independamment de ce
    /// que la transformation en est.
    pub fn verre_choisi(&self) -> bool {
        self.matiere.vers_le_verre()
    }

    /// Le mot VERRE / GALET est-il peint.
    pub fn matiere_possible(&self) -> bool {
        self.verre_possible
    }

    /// La matiere bouge-t-elle, ou attend-elle un fond : la fenetre redemande
    /// alors une trame.
    pub fn matiere_en_cours(&self) -> bool {
        self.matiere_en_marche
    }

    /// L'ordre de changer la matiere, par le mot (`depuis_le_mot`) ou par le
    /// triple clic : la cible s'inverse depuis l'etat courant, vitesse comprise
    /// (DESIGN.md §6, « un second ordre la renverse depuis l'etat courant »),
    /// le choix est retenu, l'encre et le mot suivent la cible des maintenant,
    /// et la fonte partira du centre de la cible du mot ou du centre du galet
    /// (m-g8). Vers le verre, la transformation attend la premiere image
    /// capturee ; vers le noir, elle part tout de suite et la capture se ferme
    /// quand elle est posee.
    pub fn basculer_matiere(&mut self, depuis_le_mot: bool) {
        if !self.verre_possible {
            return;
        }
        let vers_le_verre = !self.matiere.vers_le_verre();
        if !self.sans_memoire {
            crate::matiere::memoriser(vers_le_verre);
        }
        self.vitrine.matiere_visee = vers_le_verre;
        let e = self.echelle as f32;
        let m = self.vitrine.mesures();
        // Sans mot clique — le triple clic — l'origine reste vide, et la
        // trame la lit comme le centre du galet TEL QU'IL EST : replie, ce
        // n'est pas (160 ; 74) (voir `Vitrine::transformation`).
        self.vitrine.origine_fonte = match self.vitrine.zone_matiere() {
            Some(z) if depuis_le_mot => Some(((z[0] + z[2] / 2.0 - m.x) / e, (z[1] + z[3] / 2.0 - m.y) / e)),
            _ => None,
        };
        self.ordre = Some(std::time::Instant::now());
        self.matiere.viser(vers_le_verre);
        if vers_le_verre {
            #[cfg(windows)]
            {
                self.suivi.empecher_ouverture(false);
                self.suivi.exiger_copie();
            }
            if self.vitrine.fond().is_some() {
                self.mettre_en_marche();
            } else {
                self.transformation_en_attente = Some(std::time::Instant::now());
            }
        } else {
            self.transformation_en_attente = None;
            self.mettre_en_marche();
        }
        self.fenetre.request_redraw();
    }

    fn mettre_en_marche(&mut self) {
        self.matiere_en_marche = true;
        self.matiere_trame = std::time::Instant::now();
        if let Some(o) = self.ordre {
            let m = self.vitrine.mesures();
            let e = self.echelle as f32;
            let origine = self
                .vitrine
                .origine_fonte
                .unwrap_or((m.l / 2.0 / e, m.h / 2.0 / e));
            println!(
                "transformation : part {:.1} ms apres l'ordre, vers {}, fonte depuis ({:.2}, {:.2}) logiques du galet",
                o.elapsed().as_secs_f64() * 1e3,
                if self.matiere.vers_le_verre() { "le verre" } else { "le galet opaque" },
                origine.0,
                origine.1
            );
        }
    }

    /// Une image du bureau vient d'etre televersee : ce qu'elle debloque.
    fn premiere_image_capturee(&mut self) {
        if self.transformation_en_attente.take().is_some() {
            self.mettre_en_marche();
        }
        if let Some(depuis) = self.apparition_en_attente.take() {
            println!(
                "apparition : premiere image capturee {:.1} ms apres le premier tic, la fenetre parait en verre",
                depuis.elapsed().as_secs_f64() * 1e3
            );
            self.lancer_le_fondu();
        }
    }

    /// L'ilot attend-il sa premiere image capturee (pour paraitre, ou pour
    /// partir vers le verre) sans en avoir encore recu ? Une duplication neuve
    /// ne rend que les changements du bureau, et l'ilot, exclu de la capture,
    /// n'en fait aucun : la fenetre de l'horloge, qui est a nous, presente
    /// alors une trame pour lui en donner un.
    pub fn attend_une_premiere_image(&self) -> bool {
        #[cfg(windows)]
        let sans_fond = self.fond.is_none();
        #[cfg(not(windows))]
        let sans_fond = true;
        sans_fond && (self.apparition_devant() || self.transformation_en_attente.is_some())
    }

    /// Les attentes d'un fond qui n'arrive pas, bornees a `ATTENTE_PREMIERE_IMAGE`.
    fn verifier_attentes(&mut self) {
        if let Some(depuis) = self.transformation_en_attente {
            if depuis.elapsed() > ATTENTE_PREMIERE_IMAGE {
                // Pas de fond : le verre n'est pas possible maintenant. Le choix
                // reste retenu ; l'encre et le mot reviennent au galet, et la
                // capture se referme.
                self.transformation_en_attente = None;
                self.matiere.viser(false);
                self.vitrine.matiere_visee = false;
                #[cfg(windows)]
                self.suivi.empecher_ouverture(true);
                println!(
                    "transformation : aucune image capturee en {} ms, le galet reste opaque (choix retenu)",
                    ATTENTE_PREMIERE_IMAGE.as_millis()
                );
                self.fenetre.request_redraw();
            }
        }
        if let Some(depuis) = self.apparition_en_attente {
            if depuis.elapsed() > ATTENTE_PREMIERE_IMAGE {
                self.apparition_en_attente = None;
                println!(
                    "apparition : aucune image capturee en {} ms, la fenetre parait opaque ; le verre viendra avec la premiere image",
                    ATTENTE_PREMIERE_IMAGE.as_millis()
                );
                self.lancer_le_fondu();
            }
        }
    }

    /// Fait avancer la matiere d'une trame, et passe son etat a la vitrine.
    fn avancer_la_matiere(&mut self) {
        if self.matiere_en_marche {
            let maintenant = std::time::Instant::now();
            let dt = maintenant.duration_since(self.matiere_trame).as_secs_f32();
            self.matiere_trame = maintenant;
            self.matiere.avancer(dt);
            if self.sans_ressort {
                // Moins de mouvement : la matiere change d'un coup (DESIGN.md §6).
                self.matiere.poser();
            }
            if let Some(o) = self.ordre.take() {
                let (g, f) = self.matiere.etat();
                println!(
                    "transformation : premiere image g = {g:.4} f = {f:.4}, {:.1} ms apres l'ordre",
                    o.elapsed().as_secs_f64() * 1e3
                );
            }
            if self.matiere.au_repos() {
                self.matiere.poser();
                self.matiere_en_marche = false;
                self.transformation_posee();
            }
        }
        self.vitrine.matiere = Some(self.matiere.etat());
    }

    /// La matiere vient de se poser : sur le noir, la capture se ferme et le
    /// fond s'oublie, pour que le galet opaque ne coute rien de plus que le
    /// binaire d'avant (A3).
    fn transformation_posee(&mut self) {
        if !self.matiere.vers_le_verre() {
            #[cfg(windows)]
            {
                self.suivi.empecher_ouverture(true);
                self.fond = None;
            }
            self.fond_neuf = false;
            self.vitrine.oublier_fond();
        }
        let (g, f) = self.matiere.etat();
        println!("transformation : posee, g = {g:.0} f = {f:.0}, capture {}", if self.matiere.vers_le_verre() { "ouverte" } else { "fermee" });
    }

    /// La poignee Win32 de la fenetre, pour l'ecoute des reglages du systeme.
    #[cfg(windows)]
    pub fn hwnd_public(&self) -> Option<isize> {
        self.hwnd()
    }

    /// La poignee Win32 de la fenetre, s'il y en a une.
    #[cfg(windows)]
    fn hwnd(&self) -> Option<isize> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let poignee = self.fenetre.window_handle().ok()?;
        match poignee.as_raw() {
            RawWindowHandle::Win32(w) => Some(w.hwnd.get()),
            _ => None,
        }
    }

    /// Pose ou retire l'exclusion de la capture, et rend l'affinite relue.
    #[cfg(windows)]
    pub fn exclure_de_la_capture(&self, exclure: bool) -> Option<u32> {
        crate::bureau::exclure_de_la_capture(self.hwnd()?, exclure)
    }

    #[cfg(not(windows))]
    pub fn exclure_de_la_capture(&self, _exclure: bool) -> Option<u32> {
        None
    }

    /// La region du bureau a suivre : le galet COURANT, en pixels physiques du
    /// bureau virtuel, plus la marge du flou de chaque cote. Elle suit la
    /// taille du galet et non celle de la fenetre, qui garde toujours sa taille
    /// deployee.
    #[cfg(windows)]
    pub fn region_suivie(&self) -> Option<crate::bureau::Rect> {
        self.region_du_galet(self.vitrine.mesures())
    }

    /// La meme region, pour le galet deploye au repos.
    #[cfg(windows)]
    pub fn region_deployee(&self) -> Option<crate::bureau::Rect> {
        self.region_du_galet(Mesures::figee(
            1.0,
            0.0,
            self.echelle as f32,
            self.config.width as f32,
        ))
    }

    /// La marge capturee autour du galet, en pixels physiques de cet ecran.
    pub fn marge(&self) -> i32 {
        crate::galet::marge(self.echelle as f32)
    }

    /// Le galet de la derniere trame, en pixels de fenetre.
    pub fn mesures_galet(&self) -> Mesures {
        self.vitrine.mesures()
    }

    /// La largeur du ruban de bord du galet de la derniere trame, en pixels : la
    /// couverture tombe a zero a une demi-largeur hors du bord.
    pub fn ruban_galet(&self) -> f32 {
        self.vitrine.adoucissement(self.vitrine.mesures())
    }

    /// Le verre est-il pose : un fond est-il televerse ?
    pub fn verre_pose(&self) -> bool {
        self.vitrine.fond().is_some()
    }

    /// Le fond garde pour le verre, tel que la capture l'a rendu.
    #[cfg(windows)]
    pub fn fond_capture(&self) -> Option<&crate::bureau::Region> {
        self.fond.as_ref()
    }

    /// Montre ou cache le contenu du galet. Le relevé le cache pour juger la
    /// seule matiere, comme le banc.
    pub fn montrer_contenu(&mut self, montrer: bool) {
        self.vitrine.sans_contenu = !montrer;
        self.fenetre.request_redraw();
    }

    #[cfg(windows)]
    fn region_du_galet(&self, m: Mesures) -> Option<crate::bureau::Rect> {
        let marge = self.marge();
        if m.l <= 0.0 || m.h <= 0.0 {
            return None;
        }
        let origine = self.fenetre.inner_position().ok()?;
        Some(crate::bureau::Rect {
            x: origine.x + m.x as i32 - marge,
            y: origine.y + m.y as i32 - marge,
            l: m.l as i32 + 2 * marge,
            h: m.h as i32 + 2 * marge,
        })
    }

    /// La boite englobante de la region de decoupe REELLEMENT posee sur la
    /// fenetre, relue a Windows, dans le bureau virtuel. `None` si aucune
    /// region n'est posee (pendant le mouvement).
    ///
    /// Elle sert de chemin independant au relevé : elle vient de
    /// `galet::silhouette` et de GDI, pas des mesures dont la region suivie est
    /// tiree.
    #[cfg(windows)]
    pub fn boite_de_decoupe(&self) -> Option<crate::bureau::Rect> {
        use windows_sys::Win32::Foundation::{HWND, RECT};
        use windows_sys::Win32::Graphics::Gdi::GetWindowRgnBox;
        let hwnd = self.hwnd()? as HWND;
        let mut b = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // 0 erreur, 1 region vide : ni l'une ni l'autre n'est une decoupe.
        if unsafe { GetWindowRgnBox(hwnd, &mut b) } < 2 {
            return None;
        }
        // La region est relative au coin de la FENETRE, pas de la zone client.
        let o = self.fenetre.outer_position().ok()?;
        Some(crate::bureau::Rect {
            x: o.x + b.left,
            y: o.y + b.top,
            l: b.right - b.left,
            h: b.bottom - b.top,
        })
    }

    /// Un tour de capture sur la region suivie. Non bloquant : sur un bureau
    /// qui ne change pas, il ne coute qu'une acquisition qui expire.
    ///
    /// Ce qu'il rapporte decide du verre :
    ///
    /// - **une copie neuve** n'est gardee que si ses octets different du fond
    ///   deja televerse. Le pas 1 a mesure pourquoi : l'ilot exclu qui se
    ///   repeint produit des rectangles sales egaux a sa propre fenetre, donc
    ///   une copie. Si chaque copie demandait une trame, chaque trame
    ///   produirait une copie, et le travail bouclerait a la cadence de
    ///   l'ecran. Comparer les octets coupe la boucle sans jamais laisser un
    ///   verre perime : une image qui a change est toujours differente. Un
    ///   filtre sur le rectangle de notre fenetre, lui, aurait ignore un
    ///   changement du bureau fusionne dans ce meme rectangle ;
    /// - **une perte ou une duplication fermee** rend le galet noir opaque a
    ///   la trame suivante : ni panne ni image figee. Le fond revient avec la
    ///   premiere copie apres reouverture, qui est forcee.
    #[cfg(windows)]
    pub fn suivre_le_bureau(&mut self) -> Option<crate::bureau::Tour> {
        use crate::bureau::Tour;
        let r = self.region_suivie()?;
        let tour = self.suivi.tour(r);
        match tour {
            Tour::Neuve => {
                if let Some(neuve) = self.suivi.region.take() {
                    let meme = self.fond.as_ref().is_some_and(|f| {
                        (f.x, f.y, f.largeur, f.hauteur)
                            == (neuve.x, neuve.y, neuve.largeur, neuve.hauteur)
                            && f.pixels == neuve.pixels
                    });
                    if meme {
                        self.verre_compteurs.identiques += 1;
                    } else {
                        self.verre_compteurs.differentes += 1;
                    }
                    // L'epreuve du relevé televerse meme un fond identique :
                    // c'est la boucle que la comparaison coupe, et le verdict
                    // qui la surveille doit la voir.
                    let a_jour = meme && (self.fond_neuf || self.vitrine.fond().is_some());
                    if !a_jour || self.epreuve_sans_comparaison {
                        if self.fond.is_none() && self.vitrine.fond().is_none() {
                            println!(
                                "capture : premiere image capturee {:.1} ms apres la construction de l'ilot",
                                self.naissance.elapsed().as_secs_f64() * 1e3
                            );
                        }
                        self.fond = Some(neuve);
                        self.fond_neuf = true;
                        self.fenetre.request_redraw();
                    }
                }
            }
            Tour::Perdu | Tour::Ferme if !self.epreuve_sans_repli => {
                if self.fond.is_some() || self.vitrine.fond().is_some() {
                    self.fond = None;
                    self.fond_neuf = false;
                    self.vitrine.oublier_fond();
                    self.verre_compteurs.oublis += 1;
                    self.fenetre.request_redraw();
                }
            }
            _ => {}
        }
        Some(tour)
    }

    /// La sonde du bureau entre deux trames de l'ilot. Sans elle, la capture
    /// n'avancerait qu'avec les trames de l'ilot, une par seconde au repos, et
    /// un bureau qui change sous un ilot immobile laisserait un verre fige.
    /// Pendant un mouvement, `dessiner` fait deja un tour a chaque trame.
    #[cfg(windows)]
    pub fn sonder(&mut self) {
        self.verifier_l_alerte();
        // L'attente de la premiere apparition part au premier tic vivant.
        if self.apparition_a_attendre {
            self.apparition_a_attendre = false;
            self.apparition_en_attente = Some(std::time::Instant::now());
            println!(
                "apparition : l'attente de la premiere image part au premier tic de la sonde, {:.0} ms apres la construction de l'ilot",
                self.naissance.elapsed().as_secs_f64() * 1e3
            );
        }
        // Les attentes d'un fond se verifient a chaque tic, meme sans capture.
        self.verifier_attentes();
        if self.en_mouvement() || self.suivi_suspendu || !self.capture_voulue() {
            return;
        }
        self.verre_compteurs.sondes += 1;
        let _ = self.suivre_le_bureau();
    }

    /// Dit au systeme de ne pas arrondir la fenetre. Ceinture et bretelles :
    /// `VERRE.md` §6 a mesure que Windows n'arrondit PAS une fenetre sans
    /// decoration, le coin y est carre au pixel pres, et les coins continus du
    /// nuanceur sont donc seuls. L'appel ferme la question pour la version de
    /// Windows qui changerait d'avis, et il coute un appel au demarrage.
    #[cfg(windows)]
    fn ne_pas_arrondir(&self) {
        use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
        const DWMWCP_DONOTROUND: u32 = 1;
        let Ok(poignee) = self.fenetre.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(w) = poignee.as_raw() else {
            return;
        };
        unsafe {
            DwmSetWindowAttribute(
                w.hwnd.get() as _,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                (&DWMWCP_DONOTROUND as *const u32).cast(),
                4,
            );
        }
    }

    #[cfg(not(windows))]
    fn ne_pas_arrondir(&self) {}

    /// Met la fenetre de l'ilot a l'ecoute des annonces du systeme. C'est
    /// l'ilot qui ecoute, et non l'horloge, parce que c'est lui qui se rabat.
    #[cfg(windows)]
    fn ecouter_le_systeme(&self) {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        let Ok(poignee) = self.fenetre.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(w) = poignee.as_raw() else {
            return;
        };
        crate::systeme::ecouter_les_changements(w.hwnd.get());
    }

    #[cfg(not(windows))]
    fn ecouter_le_systeme(&self) {}

    /// Relit le reglage « moins de mouvement » et se rabat, ou se releve, en
    /// consequence. Appelee quand `systeme::reglages_changes` dit que Windows a
    /// annonce quelque chose — pas plus souvent. Il etait lu UNE SEULE FOIS, a
    /// la construction : un defaut discret, mais un defaut.
    /// L'echelle effective de l'ilot : l'ecran fois la taille du texte.
    pub fn echelle(&self) -> f64 {
        self.echelle
    }

    pub fn facteur_texte(&self) -> f32 {
        self.facteur_texte
    }

    pub fn relire_les_reglages(&mut self) {
        self.relire_la_permission_du_verre();
        let facteur = crate::systeme::taille_du_texte();
        if (facteur - self.facteur_texte).abs() > 1e-3 {
            println!(
                "reglages relus : taille du texte {:.0} % ; toutes les cotes logiques de l'ilot suivent, sauf les 6 px du haut (N7)",
                facteur * 100.0
            );
            self.facteur_texte = facteur;
            // La fenetre prend sa nouvelle taille logique ; `Resized` suit, et
            // `redimensionner` relit l'echelle avec le facteur.
            let _ = self
                .fenetre
                .request_inner_size(winit::dpi::LogicalSize::new(DEPLOYE_L * facteur, DEPLOYE_H * facteur));
        }
        let delai = crate::systeme::delai_de_survol();
        if delai != self.delai_appui {
            println!("reglages relus : delai de l'appui maintenu {} ms", delai.as_millis());
            self.delai_appui = delai;
        }
        // Le delai et le rectangle du double clic se relisent de meme : ils
        // vivent dans le panneau de la souris, que l'auteur peut changer en
        // cours de nuit (GESTE-TRIPLE-CLIC.md §9).
        let double = crate::systeme::delai_du_double_clic();
        let rectangle = crate::systeme::rectangle_du_double_clic();
        if double != self.delai_double || rectangle != self.rectangle_double {
            println!(
                "reglages relus : delai du double clic {} ms, rectangle {:.0} x {:.0} px physiques",
                double.as_millis(),
                rectangle.0,
                rectangle.1
            );
            self.delai_double = double;
            self.rectangle_double = rectangle;
        }
        let reduit = crate::systeme::mouvement_reduit();
        if reduit != self.sans_ressort {
            self.sans_ressort = reduit;
            if reduit && self.matiere_en_marche {
                // Moins de mouvement, en cours de transformation : elle se pose.
                self.matiere.poser();
            }
            self.fenetre.request_redraw();
        }
    }

    pub fn alpha_par_pixel(&self) -> bool {
        self.vitrine.alpha_par_pixel
    }

    /// Un changement de taille — donc, pour une fenetre a taille logique fixe, un
    /// changement de moniteur — refait la chaine d'echange ET la vitrine.
    ///
    /// Sans cela, glisse sur un ecran d'un autre facteur d'echelle, l'ilot
    /// garderait une chaine d'echange a l'ancienne taille et un galet mesure pour
    /// le mauvais DPI : `get_current_texture` rendrait `Outdated` et se
    /// reconfigurerait sur une configuration perimee, et la region GDI ne
    /// tomberait plus sur ce qui est peint. L'horloge traite deja son `Resized`
    /// de la meme facon ; l'ilot ne le faisait pas.
    pub fn redimensionner(&mut self, gpu: &Gpu, typo: &mut Typographie, l: u32, h: u32) {
        let echelle = self.fenetre.scale_factor() * self.facteur_texte as f64;
        let (l, h) = (l.max(1), h.max(1));
        if (l, h) == (self.config.width, self.config.height) && echelle == self.echelle {
            return;
        }
        self.config.width = l;
        self.config.height = h;
        self.surface.configure(&gpu.peripherique, &self.config);
        // L'echelle porte la geometrie du galet ET le seuil de glissement de
        // 4 px, qui est en pixels logiques.
        self.echelle = echelle;
        // Tout ce qui n'est pas recalcule par la construction se reporte a la
        // main : les deux formes imposees par drapeau.
        let (exposant, rayon) = (self.vitrine.exposant, self.vitrine.rayon);
        self.vitrine = Vitrine::nouvelle(
            gpu,
            typo,
            self.config.format,
            (l, h),
            echelle as f32,
            self.vitrine.alpha_par_pixel,
        );
        self.vitrine.exposant = exposant;
        self.vitrine.rayon = rayon;
        // La vitrine neuve n'a pas de fond : celui qu'on garde sera reteleverse
        // a la prochaine trame, en attendant la copie a la nouvelle echelle.
        #[cfg(windows)]
        {
            self.fond_neuf = self.fond.is_some();
        }
        // La region est en pixels : celle qui est posee ne vaut plus rien.
        self.region = None;
        self.fenetre.request_redraw();
    }

    /// La cible du deploiement : vrai si l'ilot va vers, ou est, deploye.
    pub fn deploye(&self) -> bool {
        self.deploye
    }

    pub fn en_mouvement(&self) -> bool {
        !self.ressort_l.au_repos()
            || !self.ressort_h.au_repos()
            || !self.fondu.au_repos()
            || !self.enchaine.au_repos()
    }

    /// Le fondu enchaine de « moins de mouvement » court-il ?
    fn enchaine_en_cours(&self) -> bool {
        !self.enchaine.au_repos()
    }

    /// Fait avancer le fondu enchaine du temps reel ecoule, s'il court.
    fn avancer_l_enchaine(&mut self) {
        if self.enchaine.au_repos() {
            return;
        }
        let maintenant = std::time::Instant::now();
        let dt = maintenant.duration_since(self.enchaine_trame).as_secs_f32();
        self.enchaine_trame = maintenant;
        self.enchaine.avancer(dt);
        if self.enchaine.au_repos() {
            self.enchaine = Ressort::pose(if self.deploye { COURSE_FONDU } else { 0.0 });
            println!("mouvement reduit : fondu enchaine fini, {}", if self.deploye { "deploye" } else { "replie" });
        }
    }

    /// Les deux passes du fondu enchaine : le galet replie en entier, puis
    /// l'ilot deploye par-dessus a l'opacite `w`. Le grand contient le petit :
    /// dans le petit, `w·D + (1 − w)·R` exactement ; hors du petit, `w·D`
    /// sur le bureau. La forme n'a pas bouge, seule l'opacite l'a fait (m5,
    /// Apple : des fondus plutot que du mouvement).
    fn dessiner_enchaine(&mut self, gpu: &Gpu, typo: &mut Typographie, chrono: &Chrono, vue: &wgpu::TextureView) {
        let w = (self.enchaine.position / COURSE_FONDU).clamp(0.0, 1.0);
        let fondu = self.vitrine.opacite;
        self.vitrine.superposer = None;
        self.vitrine.composer(typo, chrono, 0.0);
        self.vitrine.dessiner(gpu, typo, vue);
        // Le deploye en entier (opacite 1, AVANT de composer : `placer` pose
        // l'alpha des lignes avec), dans l'image, posee a `w` fois le fondu.
        self.vitrine.opacite = 1.0;
        self.vitrine.composer(typo, chrono, 1.0);
        self.vitrine.superposer = Some(w * fondu);
        self.vitrine.dessiner(gpu, typo, vue);
        self.vitrine.superposer = None;
        self.vitrine.opacite = fondu;
    }

    /// Le deploiement courant, relu sur la hauteur — la seule des deux mesures
    /// dont les bornes ne bougent pas.
    fn p(&self) -> f32 {
        let e = self.echelle as f32;
        ((self.ressort_h.position / e - REPLIE_H) / (DEPLOYE_H - REPLIE_H)).clamp(0.0, 1.0)
    }

    /// Pose les deux cibles et avance les deux ressorts du temps REEL ecoule
    /// depuis la trame precedente de l'ilot.
    fn avancer(&mut self, largeur_replie: f32) {
        let e = self.echelle as f32;
        let (cible_l, cible_h) = if self.deploye {
            (DEPLOYE_L * e, DEPLOYE_H * e)
        } else {
            (largeur_replie, REPLIE_H * e)
        };
        self.ressort_l.viser(cible_l);
        self.ressort_h.viser(cible_h);
        if self.premiere_trame {
            // Il apparait replie ; il n'arrive de nulle part.
            self.premiere_trame = false;
            self.ressort_l = Ressort::pose(cible_l);
            self.ressort_h = Ressort::pose(cible_h);
            return;
        }
        if self.sans_ressort {
            // Le systeme demande moins de mouvement : la forme se prend d'un
            // coup, sans ressort (DESIGN.md §5, point 6, applique a l'ilot).
            self.ressort_l = Ressort::pose(cible_l);
            self.ressort_h = Ressort::pose(cible_h);
            return;
        }
        let maintenant = std::time::Instant::now();
        let dt = maintenant.duration_since(self.trame_precedente).as_secs_f32();
        self.trame_precedente = maintenant;
        self.ressort_l.avancer(dt);
        self.ressort_h.avancer(dt);
    }

    /// Change la cible, et RIEN d'autre : la position et la vitesse du ressort
    /// restent ce qu'elles sont. C'est toute l'interruptibilite de DESIGN.md §6 —
    /// un clic pendant l'ouverture referme depuis la position ET la vitesse
    /// courantes, sans reprise a zero ni saut.
    pub fn viser(&mut self, deploye: bool) {
        // L'animation part d'ICI. Sans cette remise a l'heure, sa premiere trame
        // mesurerait le temps ecoule depuis le dernier dessin de l'ilot — qui
        // peut dater d'une seconde entiere, l'ilot ne dessinant rien tant que
        // rien ne bouge. Le ressort avalerait alors toute sa course en un pas
        // (e^(-20 x 0,5) = 4,5e-5) et le deploiement serait instantane pour tout
        // clic tombant plus de 400 ms apres une frontiere de seconde.
        self.trame_precedente = std::time::Instant::now();
        // On ne change QUE la direction. Les deux positions et les deux vitesses
        // restent ce qu'elles sont, et c'est toute l'interruptibilite : les
        // cibles elles-memes sont posees a la trame suivante, dans `avancer`,
        // quand le faconnage aura dit la largeur du galet replie.
        self.deploye = deploye;
        if deploye {
            // Le compte ne vit que sur le galet replie : l'ilot qui s'ouvre
            // l'abandonne, sans jouer son horloge.
            self.abandonner_le_compte("l'ilot se deploie");
        }
        self.geste_a = std::time::Instant::now();
        // Sous « moins de mouvement », le deploiement est un fondu enchaine
        // sur le meme ressort ; sinon ce ressort reste pose sur l'etat.
        let cible = if deploye { COURSE_FONDU } else { 0.0 };
        if self.sans_ressort {
            self.enchaine.viser(cible);
            self.enchaine_trame = std::time::Instant::now();
        } else {
            self.enchaine = Ressort::pose(cible);
        }
        if !deploye {
            self.vitrine.survol = None;
            self.vitrine.focus = None;
            self.depart_pointeur = None;
        }
        self.fenetre.request_redraw();
    }

    pub fn replier(&mut self) {
        if self.deploye {
            self.viser(false);
        }
    }

    // --- Le compte des clics (GESTE-TRIPLE-CLIC.md §5) ----------------------

    /// Le compte a-t-il lieu d'etre : le galet est replie, il ne bouge pas, et
    /// il y a une matiere a basculer. Sinon aucun compte ne s'ouvre, et le clic
    /// garde son action immediate — c'est ce qui rend son immediatete au clic
    /// sous transparence coupee, economiseur, theme de contraste ou fenetre
    /// sans alpha par pixel.
    fn compte_possible(&self) -> bool {
        !self.deploye && self.verre_possible
    }

    /// Le point est-il dans le rectangle du double clic autour du premier
    /// appui : `|dx| <= cx / 2` et `|dy| <= cy / 2`, en pixels physiques.
    fn dans_le_rectangle(&self, origine: (f64, f64), point: (f64, f64)) -> bool {
        (point.0 - origine.0).abs() <= self.rectangle_double.0 / 2.0
            && (point.1 - origine.1).abs() <= self.rectangle_double.1 / 2.0
    }

    /// Ferme le compte et met son action en attente : un ou deux clics rendent
    /// l'horloge, comme le ferait un clic simple d'aujourd'hui. Sans compte
    /// ouvert, ne fait rien.
    fn fermer_le_compte(&mut self, pourquoi: &str) {
        let Some(compte) = self.compte.take() else {
            return;
        };
        println!(
            "geste : compte de {} clic(s) ferme apres {} ms ({pourquoi}) -> l'horloge revient devant",
            compte.clics,
            compte.premier_appui.elapsed().as_millis()
        );
        self.differe = Some(Geste::RendreLHorloge);
    }

    /// Abandonne le compte SANS jouer son action : le geste n'etait ni un clic
    /// ni un triple clic (un appui maintenu, un glissement, un ilot qui s'en
    /// va). Un glissement n'a jamais valu clic, et un appui maintenu non plus.
    fn abandonner_le_compte(&mut self, pourquoi: &str) {
        let Some(compte) = self.compte.take() else {
            return;
        };
        println!(
            "geste : compte de {} clic(s) abandonne sans action ({pourquoi})",
            compte.clics
        );
    }

    /// Le geste qu'une fermeture de compte a mis en attente. La boucle le
    /// draine a chaque tour et apres chaque evenement de souris de l'ilot.
    pub fn geste_differe(&mut self) -> Option<Geste> {
        self.differe.take()
    }

    // --- La souris ----------------------------------------------------------

    /// Renvoie vrai s'il faut redessiner. Un mouvement n'arrive que dans la
    /// silhouette (la region au repos) ou nulle part (fenetre traversante) :
    /// il annule un depart du pointeur.
    pub fn souris_bougee(&mut self, x: f64, y: f64) -> bool {
        self.curseur = (x, y);
        self.depart_pointeur = None;
        if let Some(appui) = self.appui.as_ref() {
            let (ax, ay) = appui.origine;
            if (x - ax).hypot(y - ay) > SEUIL_GLISSER * self.echelle {
                // Plus de 4 px logiques, bouton enfonce : l'ilot se deplace,
                // replie ou deploye (§B). Une fois `drag_window` appele,
                // Windows prend la main : sa boucle modale bloque la notre et
                // on ne verra ni les deplacements ni le relachement. L'etat
                // d'appui se ferme donc ICI, pas au relachement — qui
                // n'arrivera pas.
                self.appui = None;
                self.abandonner_le_compte("glissement");
                println!("geste : glissement, l'ilot se deplace");
                let _ = self.fenetre.drag_window();
                return false;
            }
        }
        // Hors du rectangle du double clic, le geste ne peut plus etre un
        // triple clic : le compte se ferme et rend l'horloge tout de suite. En
        // pratique c'est ce qui l'abrege, la main bougeant avant les 500 ms.
        if let Some(origine) = self.compte.as_ref().map(|c| c.origine) {
            if !self.dans_le_rectangle(origine, (x, y)) {
                self.fermer_le_compte("le pointeur sort du rectangle du double clic");
            }
        }
        let survol = self.action_sous(x, y);
        if survol != self.vitrine.survol {
            self.vitrine.survol = survol;
            return true;
        }
        false
    }

    /// Le pointeur quitte la silhouette. Deploye, l'ilot se repliera si le
    /// pointeur n'est pas revenu au bout du delai. Traversante, la fenetre
    /// recoit un `WM_MOUSELEAVE` qui n'est pas un depart : Windows ne la voit
    /// plus sous le curseur (`WS_EX_TRANSPARENT`), et le relachement d'un appui
    /// maintenu pendant le deploiement le declenche (mesure
    /// `pas6/gestes-6e460de9.txt` : l'ilot se repliait 400 ms apres). Le vrai
    /// depart se relit sur la position du curseur quand les clics reviennent.
    pub fn souris_sortie(&mut self) -> bool {
        if !self.traversant {
            self.marquer_le_depart("le pointeur quitte la silhouette");
            // Le pointeur hors de la silhouette ne reviendra pas dans le
            // rectangle du double clic sans repasser par un mouvement : le
            // compte se ferme. Traversante, la fenetre recoit un
            // `WM_MOUSELEAVE` qui n'est pas un depart (RECETTES.md), et il ne
            // doit rien fermer du tout.
            self.fermer_le_compte("le pointeur quitte la silhouette");
        }
        if self.vitrine.survol.is_some() {
            self.vitrine.survol = None;
            return true;
        }
        false
    }

    /// Le pointeur est hors de la silhouette de l'ilot deploye : le delai du
    /// repli part, s'il n'est pas deja parti.
    fn marquer_le_depart(&mut self, pourquoi: &str) {
        if self.deploye && self.appui.is_none() && self.depart_pointeur.is_none() {
            self.depart_pointeur = Some(std::time::Instant::now());
            println!(
                "geste : {pourquoi}, repli dans {} ms s'il ne revient pas",
                self.delai_appui.as_millis()
            );
        }
    }

    /// Le curseur est-il dans le rectangle du galet, en pixels de la fenetre ?
    /// Lu quand la fenetre reprend les clics : elle n'a recu aucun mouvement
    /// pendant qu'elle etait traversante.
    #[cfg(windows)]
    fn curseur_dans_le_galet(&self) -> Option<bool> {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let mut p = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut p) } == 0 {
            return None;
        }
        let origine = self.fenetre.inner_position().ok()?;
        let (x, y) = ((p.x - origine.x) as f32, (p.y - origine.y) as f32);
        let m = self.vitrine.forme();
        Some(x >= m.x && x < m.x + m.l && y >= m.y && y < m.y + m.h)
    }

    #[cfg(not(windows))]
    fn curseur_dans_le_galet(&self) -> Option<bool> {
        None
    }

    /// L'appui, au dernier point connu du curseur. Sur le galet replie, le
    /// delai part d'ici : atteint sans deplacement, il deploie.
    pub fn souris_appuyee_ici(&mut self) {
        let (x, y) = self.curseur;
        let maintenant = std::time::Instant::now();
        // Le compte des clics (GESTE-TRIPLE-CLIC.md §5). Un appui continue le
        // compte s'il tombe sous le delai du systeme ET dans son rectangle,
        // tous deux mesures depuis le PREMIER appui pour le rectangle et depuis
        // le dernier pour le delai, comme Windows. Sinon le compte ouvert se
        // ferme d'abord — son horloge est rendue — et celui-ci en ouvre un neuf.
        if self.compte_possible() {
            let continue_ = self.compte.as_ref().is_some_and(|c| {
                c.relache
                    && maintenant.duration_since(c.dernier_appui) <= self.delai_double
                    && self.dans_le_rectangle(c.origine, (x, y))
            });
            if continue_ {
                if let Some(c) = self.compte.as_mut() {
                    c.clics += 1;
                    c.dernier_appui = maintenant;
                    c.relache = false;
                }
            } else {
                self.fermer_le_compte("un appui hors du delai ou du rectangle");
                self.compte = Some(Compte {
                    origine: (x, y),
                    dernier_appui: maintenant,
                    premier_appui: maintenant,
                    clics: 1,
                    relache: false,
                });
            }
        } else {
            self.abandonner_le_compte("l'ilot n'est plus replie, ou le verre n'est plus possible");
        }
        self.appui = Some(Appui { origine: self.curseur, depuis: maintenant, tenu: false });
        self.depart_pointeur = None;
        println!(
            "geste : appui a ({x:.0}, {y:.0}) sur l'ilot {}{}",
            if self.deploye { "deploye" } else { "replie" },
            match self.compte.as_ref() {
                Some(c) => format!(", appui {} du compte", c.clics),
                None => String::new(),
            }
        );
    }

    /// Le relachement, au dernier point connu du curseur.
    pub fn souris_relachee_ici(&mut self) -> Option<Geste> {
        let (x, y) = self.curseur;
        self.souris_relachee(x, y)
    }

    /// Ce que le relachement demande, selon §B. Un relachement sans appui
    /// memorise est la fin d'un glissement : il ne vaut pas clic.
    pub fn souris_relachee(&mut self, x: f64, y: f64) -> Option<Geste> {
        let appui = self.appui.take()?;
        let ms = appui.depuis.elapsed().as_millis();
        if appui.tenu {
            println!("geste : relachement apres l'appui maintenu ({ms} ms), ne vaut pas clic");
            return None;
        }
        if (x - appui.origine.0).hypot(y - appui.origine.1) > SEUIL_GLISSER * self.echelle {
            self.abandonner_le_compte("le relachement est a plus de 4 px logiques de l'appui");
            return None;
        }
        if !self.deploye {
            // Le galet replie. Avec un compte ouvert, l'action du clic attend
            // sa fermeture ; le troisieme clic bascule la matiere et elle n'est
            // jamais jouee (GESTE-TRIPLE-CLIC.md §5).
            if let Some(compte) = self.compte.as_mut() {
                compte.relache = true;
                if compte.clics >= 3 {
                    let geste = compte.premier_appui.elapsed().as_millis();
                    let (dx, dy) = (x - compte.origine.0, y - compte.origine.1);
                    self.compte = None;
                    println!(
                        "geste : triple clic sur le galet replie ({geste} ms du premier appui, \
                         ecart {dx:.0}, {dy:.0} px physiques) -> la matiere bascule"
                    );
                    self.basculer_matiere(false);
                    return None;
                }
                let clics = compte.clics;
                println!(
                    "geste : clic de {ms} ms sur le galet replie -> appui {clics} du compte, \
                     l'horloge attend {} ms",
                    self.delai_double.as_millis()
                );
                self.fenetre.request_redraw();
                return None;
            }
            if self.verre_possible {
                // Un compte etait ouvert et s'est ferme PENDANT l'appui : le
                // pointeur est sorti du rectangle du double clic sans aller
                // jusqu'au glissement. Son horloge attend deja d'etre jouee, et
                // ce relachement ne doit pas la jouer une seconde fois.
                println!("geste : clic de {ms} ms sur le galet replie, compte deja ferme");
                return None;
            }
            // Sans compte possible — le verre ne l'est pas, il n'y a rien a
            // basculer : l'horloge revient devant a l'instant, comme avant le
            // triple clic.
            println!("geste : clic de {ms} ms sur le galet replie -> l'horloge revient devant");
            return Some(Geste::RendreLHorloge);
        }
        match self.action_sous(x, y) {
            // Clic sur une action : elle agit, et l'ilot reste deploye.
            Some(action) => {
                println!("geste : clic de {ms} ms sur l'action {action:?}");
                self.fenetre.request_redraw();
                Some(Geste::Action(action))
            }
            // Clic sur l'ilot deploye ailleurs que sur une action : il se
            // replie, une erreur douce la ou l'horloge couperait la lecture
            // (§B). Un clic PENDANT l'ouverture ne peut pas arriver ici : la
            // fenetre est traversante tant qu'elle bouge.
            None => {
                println!("geste : clic de {ms} ms sur l'ilot deploye hors action -> replie");
                self.viser(false);
                None
            }
        }
    }

    /// L'echeance la plus proche des gestes en attente (l'appui maintenu, le
    /// depart du pointeur), pour que la boucle se reveille a temps : rien a
    /// interroger sinon.
    pub fn echeance(&self) -> Option<std::time::Instant> {
        let appui = self
            .appui
            .as_ref()
            .filter(|a| !a.tenu && !self.deploye)
            .map(|a| a.depuis + self.delai_appui);
        let depart = self.depart_pointeur.filter(|_| self.deploye).map(|d| d + self.delai_appui);
        // Le compte des clics : il n'echoit que le bouton releve, sinon c'est
        // l'appui maintenu qui decide, et il tombe plus tot (400 < 500).
        let compte = self
            .compte
            .as_ref()
            .filter(|c| c.relache)
            .map(|c| c.dernier_appui + self.delai_double);
        [appui, depart, compte].into_iter().flatten().min()
    }

    /// Les gestes a echeance : l'appui maintenu qui deploie, le depart du
    /// pointeur qui replie. Renvoie vrai si la cible a change.
    pub fn verifier_les_gestes(&mut self) -> bool {
        let maintenant = std::time::Instant::now();
        let mut change = false;
        if let Some(appui) = self.appui.as_mut() {
            if !appui.tenu && !self.deploye && maintenant.duration_since(appui.depuis) >= self.delai_appui {
                appui.tenu = true;
                let ms = appui.depuis.elapsed().as_millis();
                println!("geste : appui maintenu {ms} ms sur le galet replie -> deploie, bouton enfonce");
                // L'appui maintenu supersede : le compte est abandonne sans
                // action, et le galet n'est plus replie de toute facon.
                self.abandonner_le_compte("appui maintenu");
                self.viser(true);
                change = true;
            }
        }
        if let Some(depart) = self.depart_pointeur {
            if self.deploye && maintenant.duration_since(depart) >= self.delai_appui {
                let ms = depart.elapsed().as_millis();
                println!("geste : le pointeur est parti depuis {ms} ms -> replie");
                self.depart_pointeur = None;
                self.viser(false);
                change = true;
            } else if !self.deploye {
                self.depart_pointeur = None;
            }
        }
        // Le compte des clics a echu : le bouton est releve depuis le delai du
        // systeme sans qu'un appui de plus soit venu. Un ou deux clics rendent
        // l'horloge ; trois ne passent jamais par ici, le relachement du
        // troisieme ayant deja bascule la matiere et ferme le compte.
        let echu = self
            .compte
            .as_ref()
            .is_some_and(|c| c.relache && maintenant.duration_since(c.dernier_appui) >= self.delai_double);
        if echu {
            self.fermer_le_compte("le delai du double clic est passe");
        }
        change
    }

    /// Tab et Maj+Tab, une fois l'ilot deploye : le focus parcourt les actions
    /// disponibles (PAUSE ou REPRENDRE, SUIVANT, VERRE ou GALET). Renvoie vrai
    /// s'il faut redessiner.
    pub fn clavier_tab(&mut self, arriere: bool) -> bool {
        if !self.deploye {
            return false;
        }
        let mut ordre: Vec<Action> = ACTIONS
            .iter()
            .copied()
            .zip(self.vitrine.disponibles.iter().copied())
            .filter(|(_, d)| *d)
            .map(|(a, _)| a)
            .collect();
        if self.vitrine.matiere_possible {
            ordre.push(Action::Matiere);
        }
        if ordre.is_empty() {
            return false;
        }
        let courant = self.vitrine.focus.and_then(|f| ordre.iter().position(|a| *a == f));
        let suivant = match (courant, arriere) {
            (None, false) => 0,
            (None, true) => ordre.len() - 1,
            (Some(i), false) => (i + 1) % ordre.len(),
            (Some(i), true) => (i + ordre.len() - 1) % ordre.len(),
        };
        self.vitrine.focus = Some(ordre[suivant]);
        println!("clavier : focus {:?}", ordre[suivant]);
        true
    }

    /// Entree ou Espace : l'action au focus agit.
    pub fn clavier_agir(&mut self) -> Option<Geste> {
        let action = self.vitrine.focus.filter(|_| self.deploye)?;
        println!("clavier : {action:?} (Entree ou Espace)");
        Some(Geste::Action(action))
    }

    fn action_sous(&self, x: f64, y: f64) -> Option<Action> {
        if self.p() < SEUIL_ACTIONS {
            return None;
        }
        let (x, y) = (x as f32, y as f32);
        self.vitrine
            .zones
            .iter()
            .find(|(_, r)| x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3])
            .map(|(a, _)| *a)
    }

    // --- La position, retenue d'un lancement a l'autre ----------------------

    /// Le fichier de position vit dans les donnees locales de l'utilisateur, et
    /// non a cote de l'executable : le dossier de l'application ne doit
    /// contenir que le binaire et son JSON.
    pub fn fichier_position() -> Option<PathBuf> {
        let base = std::env::var_os("LOCALAPPDATA")?;
        Some(PathBuf::from(base).join("Veilleuse").join("ilot.txt"))
    }

    pub fn position_memorisee() -> Option<(i32, i32)> {
        let texte = std::fs::read_to_string(Ilot::fichier_position()?).ok()?;
        // L'ancien format, une ligne `<x> <y>` en px physiques absolus, vaut
        // pour l'ecran principal ; les lignes a quatre champs sont par ecran.
        for ligne in texte.lines() {
            let mots: Vec<&str> = ligne.split_whitespace().collect();
            if mots.len() == 2 {
                return Some((mots[0].parse().ok()?, mots[1].parse().ok()?));
            }
        }
        None
    }

    /// La position retenue pour cet ecran, en px physiques absolus : sa ligne
    /// `<nom> <largeur>x<hauteur> <x> <y>` (x et y logiques, relatifs a
    /// l'ecran), ou l'ancienne ligne `<x> <y>` si l'ecran est celui de
    /// l'origine du bureau. Un ecran dont la taille a change n'a plus de ligne.
    pub fn position_memorisee_pour(ecran: &crate::systeme::Ecran, echelle: f64) -> Option<(i32, i32)> {
        let texte = std::fs::read_to_string(Ilot::fichier_position()?).ok()?;
        let taille = format!("{}x{}", ecran.entier.2, ecran.entier.3);
        for ligne in texte.lines() {
            let mots: Vec<&str> = ligne.split_whitespace().collect();
            if mots.len() == 4 && mots[0] == ecran.nom && mots[1] == taille {
                let xl: f64 = mots[2].parse().ok()?;
                let yl: f64 = mots[3].parse().ok()?;
                return Some((
                    ecran.entier.0 + (xl * echelle).round() as i32,
                    ecran.entier.1 + (yl * echelle).round() as i32,
                ));
            }
        }
        if ecran.entier.0 == 0 && ecran.entier.1 == 0 {
            return Ilot::position_memorisee();
        }
        None
    }

    /// A appeler sur `WindowEvent::Moved`. L'ecriture est sautee si la position
    /// n'a pas change, ce qui suffit a ne pas marteler le disque si Windows
    /// livre l'evenement plusieurs fois pendant un glissement.
    ///
    /// Elle est sautee AUSSI tant que la regle de presence n'a pas pose
    /// l'ecran : voir le garde en tete.
    pub fn memoriser_position(&mut self, x: i32, y: i32) {
        // Sans ecran pose, on ne sait pas a quel ecran cette position
        // appartient, et l'ecrire a l'ancien format `<x> <y>` remplacerait le
        // fichier ENTIER par cette seule ligne : toutes les lignes par ecran
        // seraient perdues. Or le seul `Moved` qui arrive avant que l'ecran
        // soit pose est l'echo du placement de depart (`resumed`), qui vient
        // de lire sa position dans ce meme fichier — il n'a rien a y rendre.
        // Mesure du 18 septembre 2026 : `ilot.txt` passait de 43 a 8 octets
        // dans la premiere seconde et demie de marche, sans que personne
        // touche a rien (`captures/position-par-ecran/`).
        //
        // La position n'est pas mise en attente pour plus tard non plus :
        // `placer_pour` deplace la fenetre a l'instant ou il pose l'ecran, et
        // celle-ci n'aurait alors plus cours.
        let Some(ecran) = self.ecran.clone() else {
            println!("place : ({x}, {y}) n'est pas retenue, l'ecran n'est pas encore pose");
            return;
        };
        if self.position_ecrite == Some((x, y)) {
            return;
        }
        self.position_ecrite = Some((x, y));
        let Some(chemin) = Ilot::fichier_position() else {
            return;
        };
        if let Some(parent) = chemin.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Une position par ecran (DESIGN-ILOT.md §2.3) : la ligne de cet ecran
        // est remplacee, les autres gardees, une ligne a l'ancien format aussi.
        let echelle = self.echelle_de(&ecran);
        let taille = format!("{}x{}", ecran.entier.2, ecran.entier.3);
        let xl = (x - ecran.entier.0) as f64 / echelle;
        let yl = (y - ecran.entier.1) as f64 / echelle;
        let mut lignes: Vec<String> = std::fs::read_to_string(&chemin)
            .unwrap_or_default()
            .lines()
            .filter(|l| {
                let m: Vec<&str> = l.split_whitespace().collect();
                !(m.len() == 4 && m[0] == ecran.nom && m[1] == taille)
            })
            .map(str::to_string)
            .collect();
        lignes.push(format!("{} {taille} {xl:.2} {yl:.2}", ecran.nom));
        let contenu = lignes.join("\n") + "\n";
        // Une position perdue n'est pas une panne : l'ilot reprendra sa place
        // par defaut. On ne s'en plaint pas.
        let _ = std::fs::write(chemin, contenu);
    }

    /// Une position memorisee ne vaut que si elle retombe sur un moniteur
    /// branche aujourd'hui : un ecran debranche laisserait l'ilot hors de vue
    /// pour toujours (ARCHITECTURE.md §3).
    pub fn position_visible(el: &winit::event_loop::ActiveEventLoop, p: (i32, i32)) -> bool {
        el.available_monitors().any(|m| {
            let o = m.position();
            let s = m.size();
            // On exige que le coin haut-gauche, plus la marge d'un galet replie,
            // tombe dans l'ecran : c'est la partie qu'il faut pouvoir attraper.
            p.0 + 40 >= o.x
                && p.1 + 20 >= o.y
                && p.0 + 40 < o.x + s.width as i32
                && p.1 + 20 < o.y + s.height as i32
        })
    }

    /// La position par defaut : centree en largeur (la fenetre suit la taille
    /// du texte du systeme, N7), a 6 px logiques du haut (qui ne la suivent pas).
    pub fn position_defaut(ecran_l: u32, echelle: f64) -> (i32, i32) {
        let largeur = (DEPLOYE_L as f64 * echelle * crate::systeme::taille_du_texte() as f64).round() as i32;
        (
            (ecran_l as i32 - largeur) / 2,
            (MARGE_HAUT_DEFAUT * echelle).round() as i32,
        )
    }

    // --- La region, qui decide de ce qui traverse ---------------------------

    #[cfg(windows)]
    fn poser_region(&mut self) {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::Graphics::Gdi::{DeleteObject, SetWindowRgn};
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        // **Pendant le mouvement, aucune decoupe.**
        //
        // La region est posee au moment ou la trame est ENCODEE, mais la chaine
        // d'echange en garde deux en attente (`desired_maximum_frame_latency`) :
        // la decoupe est donc en avance de deux trames sur ce qui est a l'ecran.
        // Le galet grandissant de ~14 px par trame, il reste autour de lui un
        // ruban de cinq a sept pixels que la region laisse passer et que notre
        // surface n'a pas encore peint — transparent, donc livre au voile de
        // DWM. Sur un fond clair, cela dessine un LISERE BLANC MAT tout autour
        // du galet pendant tout le deploiement, et les rayures du fond y
        // disparaissent : c'est bien du flou de compositeur, pas notre encre.
        //
        // Mesure : `captures/orfevre/42-arrivee-120.png` le montre, et
        // `46-sans-region-140.png` — meme instant, region retiree — ne le montre
        // plus du tout. La cause est la region, pas le nuanceur.
        //
        // On ne decoupe donc qu'AU REPOS, et le prix est une contrepartie de
        // temps : pendant toute l'ouverture, l'ilot prend les 320 x 148 px de
        // souris de sa fenetre au lieu de son seul galet. Ce temps est celui du
        // ressort jusqu'a son repos complet, **521 ms** sur la largeur (la plus
        // longue des deux courses, `ressort.rs`), et non les 250 ms qu'on a
        // longtemps ecrites ici — l'ordre de grandeur avait ete pris sur
        // l'instant ou le mouvement finit pour l'oeil, pas sur celui ou le
        // calcul s'arrete. C'est la demi-seconde qui suit un clic de
        // l'utilisateur sur cet ilot meme ; il n'y a personne d'autre a servir a
        // cet instant.
        //
        // L'autre voie a ete pesee et ecartee : garder une file des dernieres
        // geometries et poser la region de celle qui vient d'arriver a l'ecran.
        // Elle supprimerait le lisere — mais elle demande de savoir QUAND une
        // trame est reellement presentee, ce que wgpu 30 ne dit pas ; et une
        // region d'une seule trame en retard pendant l'ouverture ne laisse plus
        // passer de voile, elle TRANCHE le galet peint sur une vingtaine de
        // pixels. Un defaut franc contre un defaut mesure : on garde le mesure.
        // Un instant fige par l'oeil est un instant du mouvement : pas de
        // decoupe non plus, sans quoi l'image ne serait pas celle que l'auteur
        // voit a mi-course.
        if self.mettre_a_jour_le_traversant() {
            if self.region.is_some() {
                self.retirer_region();
            }
            return;
        }
        // En attente de sa premiere image capturee (DECISION-APRES-EPREUVE.md
        // F, point 6), la fenetre existe, vit et capture, mais n'a ni surface
        // visible ni surface cliquable : une region vide. Rien n'est peint non
        // plus (`Vitrine::vide`). Aucune tache noire qui s'eclaircit.
        if self.apparition_devant() {
            self.poser_region_vide();
            return;
        }
        // `forme` et non `mesures` : la decoupe doit suivre ce que le nuanceur
        // peint, drapeaux `--rayon` et `--exposant` compris.
        let m = self.vitrine.forme();
        let ruban = self.vitrine.adoucissement(m);
        // L'empreinte de la forme, au centieme pres : elle suffit a savoir si la
        // region posee vaut encore, et elle evite de refaire cent vingt
        // rectangles pour rien a chaque seconde du chrono.
        let r = Empreinte {
            x: m.x as i32,
            y: m.y as i32,
            l: m.l as i32,
            h: m.h as i32,
            rayon: (m.rayon * 100.0) as i32,
            n: (m.n * 100.0) as i32,
            ruban: (ruban * 100.0) as i32,
        };
        if self.region == Some(r) {
            return;
        }

        let Ok(poignee) = self.fenetre.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(w) = poignee.as_raw() else {
            return;
        };
        let hwnd = w.hwnd.get() as HWND;

        unsafe {
            let Some(rgn) = region_superellipse(&m, ruban) else {
                return;
            };
            // bRedraw = 0 : c'est notre presentation qui repeint, pas Windows.
            // A 1, le galet clignote a chaque pas du ressort.
            if SetWindowRgn(hwnd, rgn, 0) == 0 {
                // Echec : la region nous appartient encore, il faut la rendre.
                // En cas de succes elle appartient au systeme, et la detruire
                // serait une faute.
                DeleteObject(rgn as _);
                return;
            }
        }
        self.region = Some(r);
    }

    /// Rend la fenetre entiere a Windows. `SetWindowRgn(null)` est le seul
    /// chemin : une region « pleine » resterait une region, et le ruban de
    /// decalage reapparaitrait au premier pixel d'arrondi.
    #[cfg(windows)]
    fn retirer_region(&mut self) {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::Graphics::Gdi::SetWindowRgn;
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let Ok(poignee) = self.fenetre.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(w) = poignee.as_raw() else {
            return;
        };
        unsafe { SetWindowRgn(w.hwnd.get() as HWND, std::ptr::null_mut(), 0) };
        self.region = None;
    }

    #[cfg(not(windows))]
    fn poser_region(&mut self) {
        self.mettre_a_jour_le_traversant();
    }

    /// L'avance de la decoupe sur l'ecran si elle etait posee maintenant, en
    /// pixels physiques : la vitesse du bord le plus vif (les cotes vont chacun
    /// a la moitie de la vitesse de la largeur, le bord bas a celle de la
    /// hauteur, le bord haut ne bouge pas) sur deux trames.
    fn avance_de_deux_trames(&self) -> f32 {
        let v = (self.ressort_l.vitesse.abs() / 2.0).max(self.ressort_h.vitesse.abs());
        v * TRAMES_D_AVANCE / self.hz.max(1.0)
    }

    /// La fenetre est traversante (`set_cursor_hittest(false)`) tant que
    /// l'avance de deux trames depasse un demi-pixel sur un bord, pendant un
    /// fondu, et quand l'oeil fige une mi-course ; elle reprend les clics et
    /// pose sa region sous le demi-pixel (DESIGN-ILOT.md §9.7 ; 306 ms a 1,25
    /// et 144 Hz d'apres le calcul du designer, refait par le juge). Renvoie
    /// l'etat courant.
    fn mettre_a_jour_le_traversant(&mut self) -> bool {
        let avance = self.avance_de_deux_trames();
        let traversant =
            self.p_fige.is_some() || !self.fondu.au_repos() || self.enchaine_en_cours() || avance > DEMI_PIXEL;
        if traversant != self.traversant {
            self.traversant = traversant;
            let _ = self.fenetre.set_cursor_hittest(!traversant);
            println!(
                "traversant : {} a p = {:.2}, avance {:.2} px, {} ms apres le geste",
                if traversant { "oui, la fenetre laisse passer les clics" } else { "non, la fenetre reprend les clics" },
                self.p(),
                avance,
                self.geste_a.elapsed().as_millis()
            );
            if !traversant && self.deploye && self.appui.is_none() {
                // Les clics reviennent : ou est le curseur ? Aucun mouvement
                // n'est arrive pendant la traversee, et un `WM_MOUSELEAVE` y a
                // pu mentir. Dehors, le delai du repli part maintenant.
                match self.curseur_dans_le_galet() {
                    Some(false) => self.marquer_le_depart("le pointeur est hors de la silhouette quand les clics reviennent"),
                    Some(true) => self.depart_pointeur = None,
                    None => {}
                }
            }
        }
        traversant
    }

    /// La region vide de l'attente : posee une fois, et remplacee par la vraie
    /// des que l'attente finit (son empreinte n'est celle d'aucune forme).
    #[cfg(windows)]
    fn poser_region_vide(&mut self) {
        use windows_sys::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, SetWindowRgn};
        let vide = Empreinte { x: 0, y: 0, l: 0, h: 0, rayon: 0, n: 0, ruban: 0 };
        if self.region == Some(vide) {
            return;
        }
        let Some(hwnd) = self.hwnd() else {
            return;
        };
        unsafe {
            let rgn = CreateRectRgn(0, 0, 0, 0);
            if rgn.is_null() {
                return;
            }
            if SetWindowRgn(hwnd as _, rgn, 0) == 0 {
                DeleteObject(rgn as _);
                return;
            }
        }
        self.region = Some(vide);
    }

    #[cfg(not(windows))]
    fn poser_region_vide(&mut self) {}

    #[cfg(not(windows))]
    fn retirer_region(&mut self) {}

    // --- Le rendu -----------------------------------------------------------

    pub fn dessiner(&mut self, gpu: &Gpu, typo: &mut Typographie, chrono: &Chrono) {
        self.verre_compteurs.trames += 1;
        self.preparer_trame(gpu, typo, chrono);
        if self.retiree_au_repos() {
            // Retire et pose : une seule trame vide, qui pose la region vide et
            // efface la surface. Ensuite, rien a montrer, rien de presente.
            if self.vide_presentee {
                self.trames_retire += 1;
                return;
            }
            self.vide_presentee = true;
        } else {
            self.vide_presentee = false;
        }

        let trame = self.surface.get_current_texture();
        use wgpu::CurrentSurfaceTexture as T;
        let trame = match trame {
            T::Success(t) | T::Suboptimal(t) => t,
            T::Outdated | T::Lost => {
                self.surface.configure(&gpu.peripherique, &self.config);
                return;
            }
            _ => return,
        };
        let vue = trame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        if self.enchaine_en_cours() {
            self.dessiner_enchaine(gpu, typo, chrono, &vue);
        } else {
            self.vitrine.dessiner(gpu, typo, &vue);
        }
        gpu.file.present(trame);
    }

    /// Ce qu'une trame fait avant la chaine d'echange : la mise en page, la
    /// region, le tour de capture, le televersement. Une fenetre cachee ne
    /// peut pas etre servie ainsi (sa chaine d'echange bloque la boucle, et
    /// sans trame sa region n'existe pas : `pas4/raccourci-c9d7b286.txt` et
    /// `-1c3da3e0.txt`) ; la fenetre est donc visible des sa naissance, region
    /// vide et rien de peint tant qu'elle attend sa premiere image.
    fn preparer_trame(&mut self, gpu: &Gpu, typo: &mut Typographie, chrono: &Chrono) {
        self.avancer_le_fondu();
        self.avancer_l_enchaine();
        self.vitrine.vide = self.apparition_devant() || self.retiree_au_repos();
        // L'ordre compte : faconner dit la largeur repliee, les ressorts s'en
        // servent comme cible, et seulement ensuite le contenu se place.
        let largeur_replie = self.vitrine.faconner(typo, chrono);
        self.avancer(largeur_replie);
        self.avancer_la_matiere();
        let m = match self.p_fige {
            // L'oeil fige un instant du deploiement dans la fenetre reelle : la
            // geometrie que les ressorts auraient en passant par p, vitesse et
            // ruban compris, celle-la meme de `--capture-ilot --p`.
            Some(p) => Mesures::figee(p, largeur_replie, self.echelle as f32, self.config.width as f32),
            None => Mesures::depuis(
                self.ressort_l.position,
                self.ressort_h.position,
                self.ressort_l.vitesse,
                self.ressort_h.vitesse,
                self.echelle as f32,
                self.config.width as f32,
            ),
        };
        self.vitrine.placer(m);
        // La region est posee AVANT la presentation : la zone qui vient de
        // s'ouvrir est ainsi peinte du meme coup, sans trame intermediaire ou
        // elle serait visible mais vide.
        self.poser_region();
        // La geometrie de cette trame est connue : la capture suit le galet tel
        // qu'il va etre peint. Rien de ce qu'elle rapporte n'est encore dessine.
        #[cfg(windows)]
        {
            if !self.suivi_suspendu && self.capture_voulue() {
                let _ = self.suivre_le_bureau();
            }
            // Le flou ne se refait que pour un fond neuf.
            if self.fond_neuf {
                if let Some(f) = self.fond.as_ref() {
                    let source = crate::galet::Source {
                        x: f.x,
                        y: f.y,
                        l: f.largeur,
                        h: f.hauteur,
                    };
                    self.vitrine.televerser_fond(gpu, &f.pixels, source);
                    self.verre_compteurs.televersements += 1;
                    self.premiere_image_capturee();
                }
                self.fond_neuf = false;
            }
            self.verifier_attentes();
            self.vitrine.origine_fenetre =
                self.fenetre.inner_position().ok().map(|p| (p.x, p.y));
        }
    }
}
