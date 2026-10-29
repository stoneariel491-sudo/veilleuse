//! Le banc du METROLOGUE : les instruments, avant les reglages.
//!
//! `matiere.rs` juge des DIRECTIONS — il repond a « laquelle ». Ce banc-ci
//! repond a une question qui vient avant : « avec quoi mesure-t-on ? ». Il ne
//! propose aucun reglage. Il construit six instruments, il valide chacun sur un
//! cas dont la reponse est connue d'avance, et il ne s'en sert pour juger l'etat
//! du labo qu'apres.
//!
//! DEUX nuanceurs passent au banc, et ce commentaire a menti tant qu'il n'en
//! nommait qu'un. `matiere.wgsl` porte l'esquisse — les directions A, B, C — et
//! reste INCHANGE, parce qu'un instrument qui demanderait qu'on modifie l'objet
//! mesure ne mesurerait plus le meme objet. `verrier.wgsl` porte la matiere
//! resolue depuis l'optique — V0 a V6 — et c'est lui qui porte la DIRECTION
//! RETENUE. Les six instruments sont les memes pour les deux, ce qui est la
//! seule facon de comparer une realisation a l'esquisse qu'elle remplace.
//!
//! LES SIX INSTRUMENTS, ET LE TEMOIN DE CHACUN
//!
//! 1. LA DISTORSION — le champ de deplacement a DEUX composantes, lu par deux
//!    sondes en dents de scie (une en x, une en y), puis la fleche des droites
//!    du fond qu'on en deduit. Temoin : biseau eteint, le champ doit etre nul.
//! 2. LA REFLECTANCE DU BORD — le profil de luminance perpendiculaire a l'arete,
//!    compare a Fresnel EXACT calcule ici, de zero. Temoin : bord eteint, le
//!    profil doit etre plat dans la matiere.
//! 3. LA DISPERSION — l'ecart entre canaux, en pixels et en niveaux. Temoin :
//!    dispersion demandee nulle, ecart nul.
//! 4. LA MATIERE, ET NON L'OPACITE — la haute frequence effacee ET la luminance
//!    portee, TOUJOURS ensemble, sur fond clair et sur fond sombre. Temoin : une
//!    matiere opaque doit echouer au second critere en reussissant le premier.
//! 5. LA LISIBILITE — les trois contrastes de la palette sur le pire fond, pris
//!    au PIRE pixel de la zone d'encre et non a la moyenne du centre. Temoin :
//!    la ligne de VERRE-TAILLE.md §4, 12,00 / 4,06 / 2,04.
//! 6. LE COUT — l'horodatage GPU par passe, la composition ISOLEE des flous.
//!    Temoin : une cible reduite doit couter proportionnellement moins.

use std::fmt::Write as _;

// ---------------------------------------------------------------------------
// La geometrie de l'ilot, celle de DESIGN.md §6 deploye.
// ---------------------------------------------------------------------------

/// LA GEOMETRIE DE L'ILOT N'EST PLUS UNE CONSTANTE, et c'est ce tour qui l'a
/// payee. Le banc ne connaissait que l'ilot DEPLOYE — 320 x 148, rayon 48,
/// exposant 4 — cables en constantes, et ses six instruments ne pouvaient donc
/// rien dire du galet REPLIE (88 x 36, rayon 18, exposant 2,5) ni des tailles
/// intermediaires que le ressort parcourt. Or c'est le galet replie qui est
/// l'etat par defaut de l'ilot : c'est ce que l'auteur voit toute la nuit.
///
/// La SOURCE, elle, garde sa taille : `SL x SH` est dimensionnee sur l'ilot
/// deploye plus deux marges, et une cible plus petite n'en lit qu'une fenetre
/// plus petite — le fond reste a la MEME echelle d'une taille a l'autre, ce qui
/// est la seule facon de comparer deux tailles sur le meme bureau.
mod geometrie {
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Les deux longueurs sont en pixels, les deux reels en CENTIEMES : un
    /// atomique porte un entier, et le centieme suffit — le rayon interpole va
    /// de 18,00 a 48,00 et l'exposant de 2,50 a 4,00.
    static L: AtomicU32 = AtomicU32::new(320);
    static H: AtomicU32 = AtomicU32::new(148);
    static R: AtomicU32 = AtomicU32::new(4800);
    static N: AtomicU32 = AtomicU32::new(400);

    pub fn poser(l: u32, h: u32, r: f32, n: f32) {
        L.store(l, Ordering::Relaxed);
        H.store(h, Ordering::Relaxed);
        R.store((r * 100.0).round() as u32, Ordering::Relaxed);
        N.store((n * 100.0).round() as u32, Ordering::Relaxed);
    }

    /// L'ilot deploye, qui est l'etat de tous les instruments sauf ceux qui
    /// disent explicitement qu'ils changent de taille.
    pub fn deploye() {
        poser(320, 148, 48.0, 4.0);
    }

    pub fn il() -> u32 {
        L.load(Ordering::Relaxed)
    }
    pub fn ih() -> u32 {
        H.load(Ordering::Relaxed)
    }
    pub fn rayon() -> f32 {
        R.load(Ordering::Relaxed) as f32 / 100.0
    }
    pub fn exposant() -> f32 {
        N.load(Ordering::Relaxed) as f32 / 100.0
    }
}

use geometrie::{exposant, ih, il, rayon};

/// La plus grande cible que le banc rende, et la taille de la source. Ce sont
/// les seules constantes de forme qui restent : elles bornent des tampons.
const PLUS_GRAND_L: u32 = 320;
const PLUS_GRAND_H: u32 = 148;
const MARGE: u32 = 64;
const SL: u32 = PLUS_GRAND_L + 2 * MARGE;
const SH: u32 = PLUS_GRAND_H + 2 * MARGE;

/// La periode de la sonde.
///
/// Elle borne la plage sans ambiguite : le deroulement contre la position
/// nominale n'est juste que sous la demi-periode, soit 48 px.
const PERIODE: f64 = 96.0;

/// Les DEUX phases, et c'est le premier temoin de ce banc qui les a imposees.
///
/// `matiere.rs` mesure le champ sur une seule phase et sur les quarante premiers
/// pixels du bord gauche, ou aucune dent ne tombe. Ce banc-ci mesure le champ
/// sur TOUT l'ilot, et le temoin 1.1 a immediatement rendu 40,85 px de
/// deplacement la ou il devait rendre zero : le pixel fautif tombait exactement
/// sur une dent, ou la rampe saute de 1 a 0 et ou le deroulement ne veut plus
/// rien dire.
///
/// Une seule phase ne peut pas resoudre cela — une dent existe toujours quelque
/// part. Deux sondes decalees d'une DEMI-PERIODE, si : leurs dents sont alors a
/// 48 px l'une de l'autre, donc chaque pixel est a 24 px au moins de la dent de
/// la MEILLEURE des deux. On garde, pixel par pixel, celle qui est la plus loin
/// de sa dent.
const PHASES: [f64; 2] = [48.0, 0.0];

/// La plus petite encre du galet deploye est a 19,66 px du bord (OPTIQUE.md §9).
/// C'est la borne de la zone ou la lisibilite se mesure.
const ENCRE_LA_PLUS_PROCHE: f32 = 19.66;

/// De combien la distance d'un pixel peut s'ecarter du centre de son bac, dans
/// le profil du bord. Voir la note de `profil_normal` : sur un bord droit
/// l'ecart est nul par construction, c'est le coin qui l'impose.
const TOLERANCE: f64 = 0.10;

// ---------------------------------------------------------------------------
// sRGB
// ---------------------------------------------------------------------------

fn vers_srgb(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn depuis_srgb(c: f64) -> f64 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn octet(lineaire: f64) -> u8 {
    (vers_srgb(lineaire.clamp(0.0, 1.0)) * 255.0).round() as u8
}

fn lineaire(o: u8) -> f64 {
    depuis_srgb(o as f64 / 255.0)
}

// ---------------------------------------------------------------------------
// Les fonds. Chacun a une reponse connue d'avance : c'est ce qui en fait des
// fonds de mesure et non des decors.
// ---------------------------------------------------------------------------

fn uni(v: u8) -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for i in 0..(SL * SH) as usize {
        p[i * 4] = v;
        p[i * 4 + 1] = v;
        p[i * 4 + 2] = v;
    }
    p
}

/// La sonde en dents de scie, LINEAIRE DANS L'ESPACE LINEAIRE, en x ou en y.
///
/// Une rampe lineaire est invariante par un flou symetrique : elle traverse
/// toute la chaine sans se deformer, donc la valeur lue dans l'image finale dit
/// exactement DE QUEL PIXEL DU FOND elle vient. Grise sur les trois canaux,
/// sans quoi l'ecart chromatique mesure serait celui de la saturation.
fn sonde(selon_y: bool, phase: f64) -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let t = if selon_y { y as f64 } else { x as f64 };
            let v = octet(((t + 0.5 + phase) / PERIODE).fract());
            let i = ((y * SL + x) * 4) as usize;
            p[i] = v;
            p[i + 1] = v;
            p[i + 2] = v;
        }
    }
    p
}

/// La distance d'une coordonnee de source a la dent la plus proche de la sonde
/// de phase donnee.
///
/// L'argument est le CENTRE du texel, `indice + 0,5` — la meme convention que
/// partout ailleurs dans ce banc. La sonde vaut `((indice + 0,5 + phase) / P)`
/// tronque a sa partie fractionnaire, donc la dent tombe la ou
/// `centre + phase` boucle sur la periode.
///
/// La version precedente prenait l'indice et rajoutait le demi-pixel elle-meme,
/// et on lui passait deja un centre : elle comptait donc le demi-pixel DEUX
/// fois. L'erreur ne valait qu'un demi-pixel sur une marge de vingt-quatre,
/// donc elle n'a jamais choisi la mauvaise sonde — mais un decalage qu'aucune
/// mesure ne peut plus atteindre reste un decalage, et il se serait reveille au
/// premier changement de periode.
fn distance_a_la_dent(centre_source: f64, phase: f64) -> f64 {
    let u = (centre_source + phase).rem_euclid(PERIODE);
    u.min(PERIODE - u)
}

/// Une rampe LINEAIRE sur toute la largeur de la source, sans aucune dent.
///
/// Elle existe pour un seul temoin, et c'est le plus dur : verifier que le champ
/// lu a sigma 1 est bien celui de sigma 18. Aucune sonde a dents ne peut le
/// faire — a sigma 18 le flou mord sur 54 px, et les dents ne sont qu'a 48 px
/// l'une de l'autre. Sans discontinuite, la question ne se pose plus.
///
/// Son prix est la precision : un octet vaut 1,29 px de position vers le bord
/// gauche, contre 0,03 px pour la sonde a dents. On le paie en moyennant sur
/// soixante rangees.
fn rampe(selon_y: bool) -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    let n = if selon_y { SH } else { SL } as f64;
    for y in 0..SH {
        for x in 0..SL {
            let t = if selon_y { y as f64 } else { x as f64 };
            let v = octet((t + 0.5) / n);
            let i = ((y * SL + x) * 4) as usize;
            p[i] = v;
            p[i + 1] = v;
            p[i + 2] = v;
        }
    }
    p
}

/// La mire de droites, a l'angle demande. Elle ne sert PAS a mesurer — elle sert
/// a regarder. Le nombre, lui, vient du champ de deplacement : une mire fine ne
/// survit pas a un flou de sigma 18, et une mire large ne dit pas le pixel.
fn mire(angle_deg: f64, periode: f64) -> Vec<u8> {
    let (s, c) = angle_deg.to_radians().sin_cos();
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let t = (x as f64 + 0.5) * c + (y as f64 + 0.5) * s;
            let dans = (t.rem_euclid(periode)) < 2.0;
            let v: u8 = if dans { 0x10 } else { 0xE8 };
            let i = ((y * SL + x) * 4) as usize;
            p[i] = v;
            p[i + 1] = v;
            p[i + 2] = v;
        }
    }
    p
}

/// La scene de nuit : l'horloge a volets telle qu'elle est reellement derriere
/// l'ilot quand celui-ci est pose sur la rangee des chiffres. Noir, et deux futs
/// de l'encre claire de `DESIGN.md`.
fn fond_horloge(decale: bool) -> Vec<u8> {
    // Un changement de seconde deplace un fut. C'est exactement ce que
    // `MATIERE.md` §7 reclame en premier essai : le seul qui puisse
    // disqualifier l'effet entier.
    let (a0, a1) = if decale { (110, 166) } else { (96, 152) };
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let i = ((y * SL + x) * 4) as usize;
            let encre = (x >= a0 && x < a1) || (x >= 268 && x < 324);
            let c = if encre { (0xF2u8, 0xEFu8, 0xE9u8) } else { (0, 0, 0) };
            p[i] = c.0;
            p[i + 1] = c.1;
            p[i + 2] = c.2;
        }
    }
    p
}

/// Un bureau vraisemblable, et il manquait.
///
/// Les trois fonds de bureau de ce banc portent tous le meme peigne de 3 px sur
/// 3, et c'est voulu : ils ont ete construits pour donner au FLOU quelque chose
/// a detruire. Mais un peigne de 3 px n'est pas un bureau, et juger une matiere
/// de bord dessus revient a juger un vernis sur du papier de verre. Celui-ci
/// porte ce qu'un ecran porte vraiment : le bord franc d'une fenetre, une
/// diagonale, et des lignes de texte. Ce sont les seules structures dont la
/// deformation dise quelque chose — une droite qui se courbe se voit, un peigne
/// qui reste un peigne ne dit rien.
fn fond_bureau_reel() -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let (fx, fy) = (x as f32, y as f32);
            // Un fond en degrade doux, comme une photo de bureau.
            let g = 0.42 + 0.26 * (fx / SL as f32) - 0.12 * (fy / SH as f32);
            let mut v = (g * 255.0) as i32;
            // Le bord franc d'une fenetre, vertical, et son ombre.
            if fx > SL as f32 * 0.52 {
                v = 238;
            }
            // Une diagonale — la seule direction ou une fleche se lit.
            if (fy - 0.62 * fx - 18.0).abs() < 2.0 {
                v = 24;
            }
            // Des lignes de texte : hauteur 4 px, pas de 11 px, dans la fenetre.
            if fx > SL as f32 * 0.56 && fx < SL as f32 * 0.94 {
                let ligne = (y % 11) < 4;
                let mot = ((x / 7) % 5) != 4;
                if ligne && mot {
                    v = 46;
                }
            }
            let i = ((y * SL + x) * 4) as usize;
            let c = v.clamp(0, 255) as u8;
            p[i] = c;
            p[i + 1] = c;
            p[i + 2] = c;
        }
    }
    p
}

/// Un fond coupe en deux : clair a gauche, sombre a droite. C'est la planche qui
/// tranche la question « de quelle couleur est un lisere » — s'il est peint, il
/// vaut la meme chose des deux cotes ; s'il est nourri par l'ecran, il change.
fn fond_coupe() -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let i = ((y * SL + x) * 4) as usize;
            let c = if x < SL / 2 { 226u8 } else { 37u8 };
            p[i] = c;
            p[i + 1] = c;
            p[i + 2] = c;
        }
    }
    p
}

/// Un fond de bureau ordinaire : des bandes larges pour la couleur, des rayures
/// fines pour que le flou ait quelque chose a detruire.
fn fond_bureau() -> Vec<u8> {
    let bandes: [(u8, u8, u8); 6] = [
        (0xE0, 0xE4, 0xE8),
        (0xE8, 0xEF, 0xF2),
        (0x40, 0x30, 0xC0),
        (0x30, 0xA0, 0xE0),
        (0xD8, 0xDC, 0xE0),
        (0xF0, 0xF0, 0xF0),
    ];
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let b = bandes[((x * 6) / SL).min(5) as usize];
            let raie = (x % 6) < 3;
            let c = if raie { (0x10, 0x10, 0x10) } else { b };
            let i = ((y * SL + x) * 4) as usize;
            p[i] = c.0;
            p[i + 1] = c.1;
            p[i + 2] = c.2;
        }
    }
    p
}

/// Le meme motif fin, mais ACHROMATIQUE. Sur un fond colore, l'ecart entre
/// canaux contient la couleur du fond ; sur celui-ci, il ne peut venir que de la
/// dispersion. C'est le seul fond ou la question « y a-t-il une frange » a une
/// reponse qui ne soit pas polluee.
fn fond_bureau_gris() -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let large = [0xE4u8, 0xEC, 0x60, 0x9C, 0xDA, 0xF0][((x * 6) / SL).min(5) as usize];
            let raie = (x % 6) < 3;
            let v = if raie { 0x10 } else { large };
            let i = ((y * SL + x) * 4) as usize;
            p[i] = v;
            p[i + 1] = v;
            p[i + 2] = v;
        }
    }
    p
}

/// Le meme motif fin, mais SOMBRE. Le critere de l'orfevre demande les deux
/// fonds : une matiere qui porterait la luminance d'un fond clair et pas celle
/// d'un fond sombre n'est pas du verre, c'est un voile blanc.
fn fond_bureau_sombre() -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let raie = (x % 6) < 3;
            let c: (u8, u8, u8) = if raie { (0x08, 0x08, 0x0A) } else { (0x3C, 0x3C, 0x42) };
            let i = ((y * SL + x) * 4) as usize;
            p[i] = c.0;
            p[i + 1] = c.1;
            p[i + 2] = c.2;
        }
    }
    p
}

// ---------------------------------------------------------------------------
// La geometrie du galet, recalculee ICI.
//
// Elle double celle du nuanceur, et c'est voulu : une mesure qui demanderait au
// nuanceur ou est son propre bord ne prouverait rien.
// ---------------------------------------------------------------------------

fn distance(px_x: f32, px_y: f32) -> f32 {
    let p = (px_x + 0.5 - il() as f32 / 2.0, px_y + 0.5 - ih() as f32 / 2.0);
    let q = (
        p.0.abs() - il() as f32 / 2.0 + rayon(),
        p.1.abs() - ih() as f32 / 2.0 + rayon(),
    );
    if q.0 > 0.0 && q.1 > 0.0 {
        let n = exposant();
        let f = (q.0.powf(n) + q.1.powf(n)).powf(1.0 / n);
        let g = ((q.0.powf(n - 1.0)).powi(2) + (q.1.powf(n - 1.0)).powi(2)).sqrt() / f.powf(n - 1.0);
        (f - rayon()) / g
    } else {
        let m = (q.0.max(0.0), q.1.max(0.0));
        (m.0 * m.0 + m.1 * m.1).sqrt() + q.0.max(q.1).min(0.0) - rayon()
    }
}

/// Le point du BORD (d = 0) le plus proche, obtenu par marche sur la normale
/// numerique. Sert a poser les droites de la mire exactement sur l'arete.
fn point_du_bord(depart: (f64, f64)) -> (f64, f64) {
    let mut p = depart;
    for _ in 0..40 {
        let d = distance(p.0 as f32, p.1 as f32) as f64;
        let e = 0.05;
        let gx = (distance((p.0 + e) as f32, p.1 as f32) - distance((p.0 - e) as f32, p.1 as f32))
            as f64
            / (2.0 * e);
        let gy = (distance(p.0 as f32, (p.1 + e) as f32) - distance(p.0 as f32, (p.1 - e) as f32))
            as f64
            / (2.0 * e);
        let n = (gx * gx + gy * gy).sqrt().max(1e-6);
        p = (p.0 - d * gx / n, p.1 - d * gy / n);
    }
    p
}

// ---------------------------------------------------------------------------
// Les lectures elementaires
// ---------------------------------------------------------------------------

fn luminance_relative(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * lineaire(r) + 0.7152 * lineaire(g) + 0.0722 * lineaire(b)
}

/// Le niveau du gris sRGB qui a la meme luminance que le pixel, en niveaux
/// flottants de 0 a 255.
///
/// C'est la grandeur que la garantie de `MATIERE.md` §5.2 compare : son plafond,
/// `encre_la_moins_lue`, est construit exactement ainsi. Sur un gris elle rend
/// l'octet lui-meme ; sur une couleur, le canal vert s'en ecarte, et c'est pour
/// cela que les lecteurs du bord ne lisent plus le vert.
fn niveau_luminance(r: u8, g: u8, b: u8) -> f64 {
    255.0 * vers_srgb(luminance_relative(r, g, b))
}

fn contraste_l(la: f64, lb: f64) -> f64 {
    let (h, bas) = if la > lb { (la, lb) } else { (lb, la) };
    (h + 0.05) / (bas + 0.05)
}

fn contraste(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    contraste_l(
        luminance_relative(a.0, a.1, a.2),
        luminance_relative(b.0, b.1, b.2),
    )
}

/// L'energie de haute frequence : la moyenne des sauts entre pixels voisins,
/// dans une bande verticale comptee depuis le bord gauche.
fn hf_bande(px: &[u8], l: u32, h: u32, x0: u32, x1: u32) -> f64 {
    let mut somme = 0.0;
    let mut n = 0.0;
    for y in h / 2 - 30..h / 2 + 30 {
        for x in x0.max(1)..x1 {
            let a = ((y * l + x) * 4) as usize;
            let b = ((y * l + x - 1) * 4) as usize;
            for c in 0..3 {
                somme += (px[a + c] as f64 - px[b + c] as f64).abs();
                n += 1.0;
            }
        }
    }
    somme / n
}

fn haute_frequence(px: &[u8], l: u32, h: u32) -> f64 {
    hf_bande(px, l, h, 1, l)
}

/// La luminance moyenne de la matiere, prise la ou l'encre se pose : au-dela de
/// la zone que le biseau touche. C'est le nombre qui porte la PORTE.
fn luminance_coeur(px: &[u8]) -> f64 {
    let mut s = 0.0;
    let mut n = 0.0;
    for y in 0..ih() {
        for x in 0..il() {
            if distance(x as f32, y as f32) > -ENCRE_LA_PLUS_PROCHE {
                continue;
            }
            let i = ((y * il() + x) * 4) as usize;
            s += luminance_relative(px[i], px[i + 1], px[i + 2]);
            n += 1.0;
        }
    }
    s / n
}

/// Le PIRE pixel de la zone d'encre — le plus clair. Le contraste d'une encre
/// claire se juge sur lui, pas sur la moyenne : une moyenne cache le pixel qui
/// fait disparaitre la lettre.
fn luminance_pire(px: &[u8]) -> f64 {
    let mut pire = 0.0f64;
    for y in 0..ih() {
        for x in 0..il() {
            if distance(x as f32, y as f32) > -ENCRE_LA_PLUS_PROCHE {
                continue;
            }
            let i = ((y * il() + x) * 4) as usize;
            pire = pire.max(luminance_relative(px[i], px[i + 1], px[i + 2]));
        }
    }
    pire
}

fn ecrire_png(chemin: &str, px: &[u8], l: u32, h: u32) {
    let f = std::fs::File::create(chemin).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), l, h);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(px).unwrap();
}

// ---------------------------------------------------------------------------
// INSTRUMENT 1 — le champ de deplacement, a deux composantes
// ---------------------------------------------------------------------------

/// Le champ lu par une sonde, canal par canal, sur TOUT l'ilot.
///
/// Positif = la source est a une coordonnee plus petite que la nominale,
/// c'est-a-dire que le pixel va chercher sa lumiere vers l'exterieur du galet
/// sur le bord gauche (et vers le haut sur le bord du haut).
fn champ(px: &[u8], selon_y: bool, phase: f64) -> Vec<[f64; 3]> {
    let mut sortie = vec![[0.0f64; 3]; (il() * ih()) as usize];
    for y in 0..ih() {
        for x in 0..il() {
            let i = ((y * il() + x) * 4) as usize;
            let t = if selon_y { y } else { x };
            // Le pixel lit le fond au CENTRE du texel : + 0,5. L'oublier laisse
            // un demi-pixel de biais dans chaque chiffre.
            let nominal = (t + MARGE) as f64 + 0.5;
            for c in 0..3 {
                let v = lineaire(px[i + c]);
                let fin = v * PERIODE - phase;
                let tours = ((nominal - fin) / PERIODE).round();
                sortie[(y * il() + x) as usize][c] = nominal - (fin + tours * PERIODE);
            }
        }
    }
    sortie
}

/// Les deux phases fondues en un seul champ : pixel par pixel, on garde la sonde
/// dont la dent est la plus loin. Voir la note de `PHASES`.
fn fondre(a: &[[f64; 3]], b: &[[f64; 3]], selon_y: bool) -> Vec<[f64; 3]> {
    let mut sortie = vec![[0.0f64; 3]; (il() * ih()) as usize];
    for y in 0..ih() {
        for x in 0..il() {
            let t = if selon_y { y } else { x };
            let nominal = (t + MARGE) as f64 + 0.5;
            let i = (y * il() + x) as usize;
            sortie[i] = if distance_a_la_dent(nominal, PHASES[0])
                >= distance_a_la_dent(nominal, PHASES[1])
            {
                a[i]
            } else {
                b[i]
            };
        }
    }
    sortie
}

/// Le champ vectoriel, canal vert, echantillonne en bilineaire.
struct Champ {
    ux: Vec<f64>,
    uy: Vec<f64>,
}

impl Champ {
    fn depuis(cx: &[[f64; 3]], cy: &[[f64; 3]]) -> Champ {
        Champ {
            ux: cx.iter().map(|t| t[1]).collect(),
            uy: cy.iter().map(|t| t[1]).collect(),
        }
    }

    fn a(&self, x: f64, y: f64) -> (f64, f64) {
        let bl = |v: &Vec<f64>| -> f64 {
            let xc = x.clamp(0.0, (il() - 1) as f64);
            let yc = y.clamp(0.0, (ih() - 1) as f64);
            let x0 = xc.floor() as u32;
            let y0 = yc.floor() as u32;
            let x1 = (x0 + 1).min(il() - 1);
            let y1 = (y0 + 1).min(ih() - 1);
            let fx = xc - x0 as f64;
            let fy = yc - y0 as f64;
            let p = |a: u32, b: u32| v[(b * il() + a) as usize];
            p(x0, y0) * (1.0 - fx) * (1.0 - fy)
                + p(x1, y0) * fx * (1.0 - fy)
                + p(x0, y1) * (1.0 - fx) * fy
                + p(x1, y1) * fx * fy
        };
        (bl(&self.ux), bl(&self.uy))
    }

    /// L'image du point de fond `q`, donne en coordonnees de la SOURCE.
    ///
    /// On resout `p + (MARGE + 0,5) - u(p) = q` par point fixe. Le champ est
    /// lisse et borne par quelques pixels : huit tours suffisent tres largement,
    /// et le banc verifie la convergence au lieu de la supposer.
    fn image_de(&self, q: (f64, f64)) -> ((f64, f64), f64) {
        let base = (q.0 - (MARGE as f64 + 0.5), q.1 - (MARGE as f64 + 0.5));
        let mut p = base;
        let mut bouge = 0.0;
        for _ in 0..16 {
            let u = self.a(p.0, p.1);
            let n = (base.0 + u.0, base.1 + u.1);
            bouge = ((n.0 - p.0).powi(2) + (n.1 - p.1).powi(2)).sqrt();
            p = n;
        }
        (p, bouge)
    }

    /// Le plus grand module du champ, et ou il tombe.
    fn module_max(&self) -> (f64, u32, u32) {
        let mut m = 0.0;
        let mut ou = (0u32, 0u32);
        for y in 0..ih() {
            for x in 0..il() {
                // Le pixel du bord ne se mesure pas : le masque y vaut une
                // demi-couverture, donc la moitie du pixel est du fond intact et
                // le deplacement lu y est divise par deux.
                if distance(x as f32, y as f32) > -1.0 {
                    continue;
                }
                let i = (y * il() + x) as usize;
                let v = (self.ux[i] * self.ux[i] + self.uy[i] * self.uy[i]).sqrt();
                if v > m {
                    m = v;
                    ou = (x, y);
                }
            }
        }
        (m, ou.0, ou.1)
    }

    /// La distance au bord au-dela de laquelle le deplacement retombe sous un
    /// demi-pixel — en dessous, la grille des pixels ne peut plus le porter.
    fn extinction(&self) -> f64 {
        let mut loin = 0.0f64;
        for y in 0..ih() {
            for x in 0..il() {
                let d = distance(x as f32, y as f32) as f64;
                if d > -1.0 {
                    continue;
                }
                let i = (y * il() + x) as usize;
                let v = (self.ux[i] * self.ux[i] + self.uy[i] * self.uy[i]).sqrt();
                if v >= 0.5 {
                    loin = loin.max(-d);
                }
            }
        }
        loin
    }
}

/// La FLECHE d'une droite du fond : de combien son image s'ecarte de la corde
/// qui joint ses deux bouts. Zero = la droite reste droite.
///
/// C'est la vraie definition de « les droites se courbent », et elle se
/// distingue d'un simple decalage : une droite parallele ou perpendiculaire a
/// l'arete d'un biseau est TRANSLATEE, pas courbee. Il faut de l'obliquite.
///
/// La direction est donnee en VECTEUR et non en angle absolu, parce que la
/// seule direction qui ait un sens ici est relative a l'arete locale : « 0
/// degre » doit vouloir dire « parallele au bord », partout, y compris dans un
/// coin ou la normale tourne. C'est ce qui rend le temoin 1.3 formulable.
///
/// Renvoie (fleche en px, ecart maximal a la position nue en px, plus grand
/// residu du point fixe en px).
fn fleche(champ: &Champ, centre: (f64, f64), dir: (f64, f64), demi_longueur: f64) -> (f64, f64, f64) {
    let l0 = (dir.0 * dir.0 + dir.1 * dir.1).sqrt().max(1e-12);
    let (c, s) = (dir.0 / l0, dir.1 / l0);
    let n = 201usize;
    let mut pts = Vec::with_capacity(n);
    let mut ecart_max = 0.0f64;
    let mut residu = 0.0f64;
    for k in 0..n {
        let t = -demi_longueur + 2.0 * demi_longueur * k as f64 / (n - 1) as f64;
        let q = (centre.0 + t * c, centre.1 + t * s);
        let (p, bouge) = champ.image_de(q);
        residu = residu.max(bouge);
        // La position sans aucun effet, dans le repere de l'image.
        let nu = (q.0 - (MARGE as f64 + 0.5), q.1 - (MARGE as f64 + 0.5));
        ecart_max = ecart_max.max(((p.0 - nu.0).powi(2) + (p.1 - nu.1).powi(2)).sqrt());
        pts.push(p);
    }
    let a = pts[0];
    let b = pts[n - 1];
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let l = (dx * dx + dy * dy).sqrt().max(1e-9);
    let mut f = 0.0f64;
    for p in &pts {
        // Distance signee du point a la corde.
        let e = ((p.0 - a.0) * dy - (p.1 - a.1) * dx) / l;
        f = f.max(e.abs());
    }
    (f, ecart_max, residu)
}

// ---------------------------------------------------------------------------
// INSTRUMENT 2 — le profil perpendiculaire, et Fresnel EXACT
// ---------------------------------------------------------------------------

/// Ou l'on prend le profil. Les deux zones doivent donner la meme chose : un
/// bord qui ne vaut que sur les droites est un accident de grille.
#[derive(Clone, Copy, PartialEq)]
enum Zone {
    Droit,
    Coin,
}

/// Le profil de luminance perpendiculaire a l'arete, en NIVEAUX.
///
/// On ne l'interpole pas : on range les pixels par leur distance signee, que ce
/// banc recalcule lui-meme, et on moyenne par bande d'un pixel. Sur un bord
/// droit cela redonne exactement la rangee de pixels ; dans un coin, cela evite
/// d'inventer des valeurs entre les pixels.
/// Les bacs sont centres sur les DEMI-ENTIERS, et c'est le temoin 2.2 qui l'a
/// impose. Avec des bacs entiers, le bac « 0 » contenait les pixels a
/// `d = −0,5` et le bac « −1 » ceux a `d = −1,5` : toute la colonne du bord
/// etait decalee d'un rang, et le banc a d'abord rapporte un lisere ABSENT du
/// bord droit alors qu'il y etait. Un bac doit etre pose sur la grille du
/// rendu, pas sur un nombre rond — c'est la meme regle que la fenetre de
/// `MATIERE.md` §0.
///
/// Sur un bord droit, les centres de pixels tombent exactement sur
/// `−0,5 / −1,5 / −2,5`, donc chaque bac contient une rangee et une seule.
///
/// LA TOLERANCE, et c'est le temoin 2.3 qui l'a imposee. Sur un bord droit les
/// centres de pixels tombent EXACTEMENT sur les demi-entiers, donc un bac d'un
/// pixel contient une rangee et une seule. Dans un coin ils tombent n'importe
/// ou : le bac « -0,5 » y contient alors des pixels dont la distance vraie
/// couvre tout l'intervalle [-1 ; 0[ — c'est-a-dire toute la rampe
/// d'anticrenelage du masque, qui passe de couvert a decouvert sur exactement
/// ce pixel-la. Le bac melange donc du verre et du fond intact, et il a
/// rapporte 67,7 niveaux la ou le verre vaut 45.
///
/// La reponse n'est pas d'elargir le bac mais de le RESSERRER : on ne garde que
/// les pixels dont la distance est a moins de `tolerance` du centre du bac. Le
/// coin en fournit assez — environ trois cents pixels par bande d'un pixel. Le
/// nombre retenu est rendu avec la valeur, pour qu'un bac vide se voie au lieu
/// de se deviner.
fn profil_normal(px: &[u8], zone: Zone, jusqu_a: i32, tolerance: f64) -> Vec<(f64, f64, usize)> {
    let mut bacs: Vec<(f64, usize)> = vec![(0.0, 0); (jusqu_a + 3) as usize];
    for y in 0..ih() {
        for x in 0..il() {
            let dc = distance(x as f32, y as f32) as f64;
            let dans_coin = ((x as f32) < rayon() || (x as f32) > il() as f32 - rayon())
                && ((y as f32) < rayon() || (y as f32) > ih() as f32 - rayon());
            if (zone == Zone::Coin) != dans_coin {
                continue;
            }
            // m = 0 pour d = -0,5 ; m = -1 pour d = -1,5 ; m = +1 pour d = +0,5.
            let m = (dc + 0.5).round() as i32;
            let k = 2 - m;
            if k < 0 || k as usize >= bacs.len() {
                continue;
            }
            if (dc - (m as f64 - 0.5)).abs() > tolerance {
                continue;
            }
            let i = ((y * il() + x) * 4) as usize;
            bacs[k as usize].0 += px[i + 1] as f64;
            bacs[k as usize].1 += 1;
        }
    }
    bacs.iter()
        .enumerate()
        .map(|(k, (s, n))| {
            (
                (2 - k as i32) as f64 - 0.5,
                if *n > 0 { s / *n as f64 } else { f64::NAN },
                *n,
            )
        })
        .collect()
}

/// La valeur du profil au bac dont le centre est exactement `d`.
fn au(pr: &[(f64, f64, usize)], d: f64) -> Option<f64> {
    pr.iter()
        .find(|(c, _, n)| (c - d).abs() < 0.01 && *n > 0)
        .map(|(_, v, _)| *v)
}

/// La pente du biseau du nuanceur, recalculee ici. Profil `h(t) = (1-(1-t)^4)^(1/4)`,
/// `t = 0` a l'arete et 1 au bout. Meme plancher sur `t` que le nuanceur.
fn pente_biseau(t: f64, galbe: f64) -> f64 {
    let v = 1.0 - t.clamp(0.004, 1.0);
    let v4 = v * v * v * v;
    (v * v * v / (1.0f64 - v4).max(1e-6).powf(0.75)) * galbe
}

/// Fresnel EXACT, non polarise, air vers verre. Pas Schlick : OPTIQUE.md §4
/// mesure 23 % d'erreur relative a 55,8 degres, c'est-a-dire en plein dans la
/// bande qui porte la reflexion.
fn fresnel_exact(cos_i: f64, n: f64) -> f64 {
    let ci = cos_i.clamp(0.0, 1.0);
    let si2 = 1.0 - ci * ci;
    let ct2 = 1.0 - si2 / (n * n);
    if ct2 <= 0.0 {
        return 1.0;
    }
    let ct = ct2.sqrt();
    let rs = (ci - n * ct) / (ci + n * ct);
    let rp = (n * ci - ct) / (n * ci + ct);
    (rs * rs + rp * rp) / 2.0
}

/// Schlick, pour chiffrer ce qu'il coute — c'est ce que le nuanceur utilise.
fn fresnel_schlick(cos_i: f64, n: f64) -> f64 {
    let r0 = ((n - 1.0) / (n + 1.0)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cos_i.clamp(0.0, 1.0)).powi(5)
}

/// L'exces de reflectance au-dessus de `r0`, MOYENNE SUR L'EMPREINTE DU PIXEL.
///
/// La montee de Fresnel est sous-pixellaire (OPTIQUE.md §3) : prise au centre du
/// pixel elle manque tout l'effet. La moyenne est donc la grandeur physique, et
/// le centre est l'erreur.
fn exces_fresnel(d_centre: f64, largeur: f64, indice: f64, galbe: f64, exact: bool) -> f64 {
    if largeur <= 0.0 {
        return 0.0;
    }
    let r0 = ((indice - 1.0) / (indice + 1.0)).powi(2);
    let n = 65;
    let mut s = 0.0;
    for k in 0..n {
        let d = d_centre - 0.5 + k as f64 / (n - 1) as f64;
        let t = (-d / largeur).clamp(0.0, 1.0);
        let p = pente_biseau(t, galbe);
        let cos_i = 1.0 / (1.0 + p * p).sqrt();
        let r = if exact {
            fresnel_exact(cos_i, indice)
        } else {
            fresnel_schlick(cos_i, indice)
        };
        s += ((r - r0) / (1.0 - r0)).max(0.0);
    }
    s / n as f64
}

// ---------------------------------------------------------------------------
// Le rendu — `matiere.wgsl`, inchange
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Reglages {
    taille: [f32; 4],
    flou: [f32; 4],
    forme: [f32; 4],
    teinte: [f32; 4],
    reglages: [f32; 4],
    biseau: [f32; 4],
    bord: [f32; 4],
    /// Lu par `verrier.wgsl` seul. `matiere.wgsl` declare une structure plus
    /// courte et ignore ce champ : un tampon uniforme plus grand que la
    /// structure declaree est licite, et c'est ce qui permet aux deux nuanceurs
    /// de partager une disposition.
    couches: [f32; 4],
    /// La lampe d'ecran (`DECISION-APRES-EPREUVE.md` E2), lue par
    /// `verrier.wgsl` seul : x = intensite L en luminance lineaire, y = 1 si
    /// son poids suit la normale sortante du bord.
    lampe: [f32; 4],
}

/// Quel nuanceur porte la direction. Les deux sont mesures par les MEMES six
/// instruments, ce qui est la seule facon de comparer une realisation a
/// l'esquisse qu'elle remplace.
#[derive(Clone, Copy, PartialEq)]
enum Modele {
    /// `matiere.wgsl` — l'esquisse, jugee fausse sur trois points par
    /// `OPTIQUE.md` §12. Gardee telle quelle : c'est la ligne de depart de
    /// `MESURER-LE-VERRE.md` §8, et elle doit rester reproductible.
    Esquisse,
    /// `verrier.wgsl` — la matiere resolue depuis l'optique.
    Verrier,
}

#[derive(Clone, Copy)]
struct Direction {
    nom: &'static str,
    modele: Modele,
    sigma: f32,
    teinte: f32,
    saturation: f32,
    largeur: f32,
    indice: f32,
    dispersion: f32,
    epaisseur: f32,
    galbe: f32,
    lisere: f32,
    clarte: f32,
    /// Largeur, en px, du profil d'auteur du lisere. 0 = Fresnel echantillonne
    /// au seul centre du pixel. `Verrier` ne s'en sert pas.
    integre: f32,
    // -- Les couches de `verrier.wgsl`, chacune extinguible --
    refraction: f32,
    /// Fresnel : le partage d'energie `(1-R)` transmis / `R` reflechi.
    fresnel: f32,
    /// Beer-Lambert sur le chemin reel.
    beer: f32,
    /// Nombre de prises sur l'empreinte du pixel.
    prises: f32,
    /// 0 = marche a la frontiere du conge, 1 = rampe en (1-t)^2.
    profil: f32,
    /// La vibrance SUIT-ELLE le flou qu'elle compense ?
    ///
    /// Le rehaussement de saturation de `verre.wgsl` compense ce que le flou a
    /// pris. La couche nette n'ayant rien perdu, la lui appliquer rehausse une
    /// saturation qui n'a pas ete abaissee. Vrai : la vibrance descend a 1,0 la
    /// ou la couche nette domine.
    vibrance_relative: bool,
    /// La couche nette n'apporte-t-elle que de la LUMINANCE ?
    ///
    /// Second remede a la chroma du bord, et le seul qui s'attaque a sa source.
    /// La dispersion etant mesuree a 0,000 px (instrument 3), la couche nette
    /// n'apporte rien de chromatique : ce qu'elle ramene est une structure
    /// spatiale. On garde donc la chroma de la couche floue et on lui donne la
    /// luminance de la nette. Prime sur `vibrance_relative` quand les deux sont
    /// poses.
    nette_luminance: bool,
    /// Le conge `b` et l'epaisseur `e` SUIVENT-ILS la hauteur du galet ?
    ///
    /// Faux : ce sont des longueurs fixes en pixels. C'est ce que la premiere
    /// realisation faisait, et le galet replie de 36 px de haut y disparaissait
    /// sous 12 px de conge de chaque cote.
    ///
    /// Vrai : `b = e = 12 x h / 148`, la meme loi que `DESIGN.md` §6 applique
    /// deja au rayon de coin et a l'exposant, qui se lisent tous deux sur la
    /// HAUTEUR. A la taille deployee le facteur vaut exactement 1, donc rien
    /// ne bouge la ou tout a ete mesure.
    conge_relatif: bool,
    /// La couche nette passe-t-elle d'abord par un PASSE-BAS, et de combien ?
    ///
    /// 0 : la couche nette est la source brute, telle que la RETENUE la lit.
    /// Au-dessus : l'ecart-type, en pixels, d'un flou gaussien applique a la
    /// source avant qu'elle soit liee comme couche nette — et donc aussi comme
    /// ce que la reflexion va chercher, puisque les deux lisent la meme liaison.
    /// C'est l'hypothese de la case 5 : les dents du peigne viendraient des
    /// hautes frequences que la couche nette ramene au bord.
    nette_sigma: f32,
    /// La lampe d'ecran (`DECISION-APRES-EPREUVE.md` E2) : son intensite, en
    /// luminance lineaire, 0 = pas de lampe. C'est la lumiere que le verre
    /// porte de lui-meme, a toute heure : sur du noir, le bord haut lit ce que
    /// le bord droit lit sur du blanc le jour.
    lampe: f32,
    /// 1 : la lampe est un ciel au-dessus de l'ecran, ponderee par
    /// `(1 - n_y) / 2` sur la normale sortante du bord (1 au bord haut, 1/2
    /// aux cotes, 0 au bord bas). 0 : uniforme, ce qui dessinerait un anneau.
    ciel: f32,
}

const TEMOIN: Direction = Direction {
    nom: "temoin verre.wgsl",
    modele: Modele::Esquisse,
    sigma: 18.0,
    teinte: 0.80,
    saturation: 1.35,
    largeur: 0.0,
    indice: 1.5,
    dispersion: 0.0,
    epaisseur: 0.0,
    galbe: 0.0,
    lisere: 0.0,
    clarte: 0.0,
    integre: 1.0,
    refraction: 0.0,
    fresnel: 0.0,
    beer: 0.0,
    prises: 8.0,
    profil: 0.0,
    vibrance_relative: false,
    nette_luminance: false,
    conge_relatif: false,
    nette_sigma: 0.0,
    lampe: 0.0,
    ciel: 0.0,
};

// ---------------------------------------------------------------------------
// Les couches du verrier — `verrier.wgsl`, une par une
// ---------------------------------------------------------------------------
//
// `MATIERE.md` §4 a tranche : direction A, conge 12 px, epaisseur 12 px. Ces
// deux nombres ne sont pas des reglages, ce sont les decisions d'auteur du
// chantier precedent, et ils ne se rediscutent pas ici. L'indice est celui du
// N-BK7, verre crown ordinaire (OPTIQUE.md §6).
const CONGE: f32 = 12.0;
const EPAISSEUR: f32 = 12.0;
const INDICE: f32 = 1.51680;

/// Le socle : `verrier.wgsl` avec TOUTES ses couches eteintes. Doit rendre le
/// temoin au pixel pres — c'est le seul temoin qui valide le nuanceur entier.
const V0: Direction = Direction {
    nom: "V0 rien",
    modele: Modele::Verrier,
    sigma: 18.0,
    teinte: 0.80,
    saturation: 1.35,
    largeur: 0.0,
    indice: INDICE,
    dispersion: 0.0,
    epaisseur: EPAISSEUR,
    galbe: 0.0,
    lisere: 1.0,
    clarte: 0.0,
    integre: 0.0,
    refraction: 0.0,
    fresnel: 0.0,
    beer: 0.0,
    prises: 8.0,
    profil: 0.0,
    vibrance_relative: false,
    nette_luminance: false,
    conge_relatif: false,
    nette_sigma: 0.0,
    lampe: 0.0,
    ciel: 0.0,
};

/// Couche 2 : la refraction de Snell seule, sur la couche FLOUTEE.
const V2_FLOUE: Direction = Direction {
    nom: "V2 Snell (floutee)",
    largeur: CONGE,
    refraction: 1.0,
    ..V0
};

/// Couche 2 bis : la meme refraction, sur la couche NETTE ramenee au biseau.
const V2: Direction = Direction {
    nom: "V2 Snell (nette)",
    largeur: CONGE,
    refraction: 1.0,
    clarte: 1.0,
    ..V0
};

/// Couche 3 et 4 : Fresnel exact moyenne sur l'empreinte, qui porte a la fois
/// le partage `(1-R)` et la reflexion nourrie par l'ecran. Les deux sont le
/// MEME terme et ne peuvent pas se separer sans mentir sur l'energie.
const V3: Direction = Direction {
    nom: "V3 Fresnel",
    largeur: CONGE,
    refraction: 1.0,
    clarte: 1.0,
    fresnel: 1.0,
    ..V0
};

/// Couche 6 : Beer-Lambert sur le chemin reel.
const V6: Direction = Direction {
    nom: "V6 +Beer",
    largeur: CONGE,
    refraction: 1.0,
    clarte: 1.0,
    fresnel: 1.0,
    beer: 1.0,
    ..V0
};

/// La force de la reflexion que la GARANTIE de `MATIERE.md` §5.2 laisse.
///
/// Ce nombre n'est pas un gout et n'est pas la physique. La physique dit 1,00,
/// et une dalle de verre posee sur un bureau a 226 y porte son premier pixel de
/// bord a 139 niveaux — mesure, essai 8.C. La garantie du projet, elle, plafonne
/// toute decoration de bord a l'encre la moins lue, `#5E5B57`, dont le gris de
/// meme luminance vaut 91 niveaux. 0,30 est la PLUS GRANDE force qui tienne ce
/// plafond sur les quatre scenes (86 pour 91), et l'essai 8.D la balaie.
///
/// Les quatre scenes ne depassent pas 242. Sur un aplat BLANC de 255 la meme
/// force casse la garantie — 92 au bord droit, 97 dans le coin — et ~~seule 0,24~~
/// y tient, coins compris (essai 8.F, au pas de 0,02). Au centieme, 0,25 y tient
/// aussi (essai 8.H) : c'est la force de l'ARBITREE. La force n'est pas changee ici : c'est
/// une decision d'auteur, et le journal la pose devant lui.
///
/// C'est donc une ATTENUATION D'AUTEUR, et elle est nommee comme telle : le
/// terme reflechi est multiplie par 0,30 alors que le terme transmis reste a
/// (1 - R), ce qui ne conserve pas l'energie. Le prix est ecrit dans le journal
/// et dans `MATIERE.md` §5.2 ; la force physique de 1,00 demanderait une
/// derogation ecrite dans `DESIGN.md`, que ce chantier n'a pas le droit d'ecrire
/// a la place de l'auteur.
const FORCE_RETENUE: f32 = 0.30;

/// LA DIRECTION RETENUE, apres le premier tour de correction.
///
/// Elle differe de `V3` par quatre choses, et chacune repond a une objection
/// mesuree plutot qu'a un avis. Chacune a sa colonne dans les instruments 7 ou
/// 8, et aucune n'entre sans le chiffre qui la paie :
///
///   `conge_relatif`   `b` et `e` suivent la hauteur du galet. Fixes, ils
///                     occupaient 74,3 % de l'aire du galet REPLIE contre
///                     21,8 % du deploye — la matiere avalait l'objet, et
///                     changeait de nature pendant les 330 ms du ressort
///                     (instrument 7).
///   `profil = 1`      rampe (1-t)^2 au lieu d'une marche. La marche laissait
///                     une couture de 31 niveaux tout le tour de l'ilot, a
///                     12 px du bord, la ou `DESIGN.md` §8 interdit le trait
///                     de separation (essai 8.A).
///   `nette_luminance` la couche nette n'apporte que de la LUMINANCE. Prise en
///                     RVB elle ramenait au bord la chroma que le flou venait
///                     de dissoudre : saturation 131 dans la bande contre 35
///                     au coeur de la meme image (essai 8.B).
///   `lisere`          la force que la garantie de `MATIERE.md` §5.2 laisse,
///                     et non celle de la physique (voir `FORCE_RETENUE`).
const RETENUE: Direction = Direction {
    nom: "RETENUE",
    lisere: FORCE_RETENUE,
    profil: 1.0,
    nette_luminance: true,
    conge_relatif: true,
    ..V3
};

/// Ce que le premier tour retenait, garde nomme pour que chaque planche et
/// chaque tableau aient leur colonne d'AVANT. Sans elle, une correction se
/// jugerait contre le souvenir qu'on en a.
const ANCIENNE: Direction = Direction {
    nom: "ANCIENNE",
    ..V3
};

/// L'ecart-type, en pixels, du passe-bas de la direction PASSE-BAS.
///
/// Il n'est pas choisi a l'oeil : l'essai 8.G le derive comme le plus grand
/// sigma balaye qui garde la cible de `MATIERE.md` §5.4 — bande de bord au moins
/// dix fois plus structuree que le centre, et jamais plus que le fond nu — aux
/// trois tailles. L'essai dit a chaque lancement si cette constante vaut encore
/// le sigma qu'il derive.
const SIGMA_PASSE_BAS: f32 = 1.0;

/// UNE DIRECTION NOMMEE, A COTE DE LA RETENUE, ET NON A SA PLACE.
///
/// La RETENUE, dont la couche nette passe par un passe-bas. C'est l'hypothese de
/// la case 5 du troisieme tour, mesuree par l'essai 8.G. Choisir entre un bord
/// qui brille avec ses dents, un bord eteint sans dents, ou celui-ci, est une
/// decision d'auteur : cette constante ne remplace rien.
const PASSE_BAS: Direction = Direction {
    nom: "PASSE-BAS",
    nette_sigma: SIGMA_PASSE_BAS,
    ..RETENUE
};

/// La tolerance d'un arrondi : deux arrondis d'une meme valeur different d'un
/// octet au plus. C'est la tolerance des temoins 8.6 et 8.9, et c'est la marge
/// qu'une garantie doit garder pour ne pas tenir par hasard d'arrondi.
const TOLERANCE_ARRONDI: f64 = 1.0;

/// La force de la reflexion que l'ARBITRE retient.
///
/// Elle n'est pas choisie a l'oeil : l'essai 8.H la derive comme la plus grande
/// force balayee au centieme dont le premier pixel du bord, coins compris, reste
/// sous l'encre la moins lue AVEC la marge d'un arrondi, sur les dix-neuf fonds
/// du banc — blanc pur et page de texte compris. Le pas du centieme n'est pas un
/// gout : sur le blanc, un centieme de force deplace le coin d'un a deux octets
/// (essai 8.H), c'est-a-dire du pas de l'image. L'essai dit a chaque lancement si
/// cette constante vaut encore la force qu'il derive.
///
/// ~~0,24~~ : c'etait la force que l'essai 8.F laissait sur le blanc SANS marge,
/// au pas de 0,02. Au centieme et avec la marge d'un arrondi, sur tous les fonds,
/// la mesure donne 0,25 (coin du blanc a 90, marge +1,4).
///
/// Depuis l'integration apres l'epreuve, la loi est la lecture du verre seul
/// depremultiplie sur la couverture exacte de l'APPLICATION, echelle par
/// echelle (`DECISION-APRES-EPREUVE.md` A1 ; decision de l'auteur, « une par
/// echelle », `INTEGRATION-APRES-EPREUVE.md` pas 1 bis) : la force est une
/// table dans `galet.rs`, 0,25 a l'echelle 1, 0,24 a 1,25, 0,21 a 1,5, 0,19 a
/// 2, et ainsi de suite. Ce banc rend a l'echelle 1 : sa force est celle de
/// l'echelle 1, 0,25, que l'application y derive aussi
/// (`captures/integration-apres-epreuve/pas1/lisere-aux-echelles.txt`).
const FORCE_ARBITREE: f32 = 0.25;

/// Le niveau que le bord DROIT du verre lit sur du blanc pur a la force retenue
/// de chaque echelle, dans l'application, au plus bas des neuf echelles :
/// 86 / 87 / 85 / 86 / 86 / 87 / 88 / 88 / 88 de l'echelle 1 a 3,5
/// (`captures/integration-apres-epreuve/pas1/lisere-aux-echelles.txt` et
/// `lisere-e*.txt`, blanc, deploye, « verre seul droit »), donc **85**, a 1,5.
/// La lampe d'ecran en tire son intensite : sur du noir, le bord haut lit ce
/// que le bord droit lit sur du blanc le jour ; et comme 85 est le plus bas des
/// neuf, sur du blanc elle ne gagne a aucune echelle. ~~88,47, le pire pixel~~
/// (`DECISION-APRES-EPREUVE.md` E2) : c'est le COIN ; une lampe au niveau du
/// coin gagnerait sur le bord droit du blanc, qui lit moins, et le temoin
/// « blanc pur identique a l'octet » tomberait. Le banc rend a l'echelle 1, ou
/// le bord droit lit 86 : l'essai `lampe` le refait par un autre chemin, et
/// verifie que 85 est bien sous lui.
const LAMPE_NIVEAU: f64 = 85.0;

/// Le bord droit sur du blanc a l'echelle 1, a la force 0,25, lu par
/// l'application (`lisere-aux-echelles.txt`, e1, f = 0,25, blanc, deploye :
/// « verre seul droit 86,00 ») : ce que ce banc doit redonner a un octet pres.
const JOUR_ECHELLE_1: f64 = 86.0;

/// L'intensite de la lampe : la luminance lineaire de `LAMPE_NIVEAU`.
fn lampe_intensite() -> f32 {
    lineaire(LAMPE_NIVEAU.round() as u8) as f32
}

/// LA DIRECTION DE L'INTEGRATION APRES L'EPREUVE : l'ARBITREE, plus la lampe
/// d'ecran de `DECISION-APRES-EPREUVE.md` E2, un ciel blanc au-dessus de
/// l'ecran dont le chant porte le reflet a toute heure. Une fonction et non une
/// constante : son intensite passe par `lineaire`, qui n'est pas constante.
fn arbitree_lampe() -> Direction {
    Direction {
        nom: "ARBITREE-LAMPE",
        lampe: lampe_intensite(),
        ciel: 1.0,
        ..ARBITREE
    }
}

/// LA DIRECTION DECIDEE PAR L'ARBITRE, a laquelle l'auteur a confie le choix.
///
/// La RETENUE a une seule difference pres : la force de la reflexion descend de
/// 0,30 a `FORCE_ARBITREE`, pour que la garantie de `MATIERE.md` §5.2 tienne
/// aussi sur un blanc pur. La couche nette reste la source brute (pas de
/// passe-bas) et la couleur du bord n'est pas touchee. Les raisons et les
/// chiffres sont dans `DECISION.md`. La RETENUE et PASSE-BAS restent nommees :
/// chaque tableau garde sa colonne d'AVANT.
const ARBITREE: Direction = Direction {
    nom: "ARBITREE",
    lisere: FORCE_ARBITREE,
    ..RETENUE
};

/// La proposition du partisan de la nuit, gardee nommee pour etre mesuree a
/// cote, telle qu'il l'a faite : force 0,24 et un passe-bas de 3 px sur la
/// couche nette. Rejetee par l'arbitrage (DECISION.md) : elle casse §5.4.
const FLOU_FORT: Direction = Direction {
    nom: "FLOU-FORT",
    lisere: 0.24,
    nette_sigma: 3.0,
    ..RETENUE
};

/// Les trois directions telles que `matiere.rs` les code AUJOURD'HUI. Ce sont
/// elles, l'etat du labo — pas les cibles de MATIERE.md §5, qui viennent du
/// modele corrige d'OPTIQUE.md et n'ont pas encore ete realisees.
const DALLE: Direction = Direction {
    nom: "A. dalle",
    sigma: 18.0,
    teinte: 0.80,
    saturation: 1.35,
    largeur: 8.0,
    indice: 1.50,
    dispersion: 0.020,
    epaisseur: 10.0,
    galbe: 1.0,
    lisere: 1.0,
    clarte: 0.35,
    integre: 1.0,
    modele: Modele::Esquisse,
    refraction: 0.0,
    fresnel: 0.0,
    beer: 0.0,
    prises: 8.0,
    profil: 0.0,
    vibrance_relative: false,
    nette_luminance: false,
    conge_relatif: false,
    nette_sigma: 0.0,
    lampe: 0.0,
    ciel: 0.0,
};

const LENTILLE: Direction = Direction {
    nom: "B. lentille",
    sigma: 18.0,
    teinte: 0.80,
    saturation: 1.35,
    largeur: 28.0,
    indice: 1.70,
    dispersion: 0.060,
    epaisseur: 26.0,
    galbe: 1.0,
    lisere: 0.6,
    clarte: 0.0,
    integre: 1.0,
    modele: Modele::Esquisse,
    refraction: 0.0,
    fresnel: 0.0,
    beer: 0.0,
    prises: 8.0,
    profil: 0.0,
    vibrance_relative: false,
    nette_luminance: false,
    conge_relatif: false,
    nette_sigma: 0.0,
    lampe: 0.0,
    ciel: 0.0,
};

const DEPOLI: Direction = Direction {
    nom: "C. depoli",
    sigma: 28.0,
    teinte: 0.80,
    saturation: 1.10,
    largeur: 0.0,
    indice: 1.50,
    dispersion: 0.0,
    epaisseur: 0.0,
    galbe: 0.0,
    lisere: 0.0,
    clarte: 0.0,
    integre: 1.0,
    modele: Modele::Esquisse,
    refraction: 0.0,
    fresnel: 0.0,
    beer: 0.0,
    prises: 8.0,
    profil: 0.0,
    vibrance_relative: false,
    nette_luminance: false,
    conge_relatif: false,
    nette_sigma: 0.0,
    lampe: 0.0,
    ciel: 0.0,
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

struct Verre {
    peripherique: wgpu::Device,
    file: wgpu::Queue,
    disposition: wgpu::BindGroupLayout,
    tuyau_flou: wgpu::RenderPipeline,
    tuyau_composer: wgpu::RenderPipeline,
    /// Les memes deux passes, rendues dans un format FLOTTANT. Voir `FIN`.
    tuyau_flou_fin: wgpu::RenderPipeline,
    tuyau_composer_fin: wgpu::RenderPipeline,
    /// Les quatre memes, mais issues de `verrier.wgsl`.
    tuyau_flou_v: wgpu::RenderPipeline,
    tuyau_composer_v: wgpu::RenderPipeline,
    tuyau_flou_v_fin: wgpu::RenderPipeline,
    tuyau_composer_v_fin: wgpu::RenderPipeline,
    format_fin: wgpu::TextureFormat,
    echantillonneur: wgpu::Sampler,
    /// L'horodateur GPU, quand l'adaptateur le porte.
    horodateur: Option<wgpu::QuerySet>,
    periode_ns: f32,
}

/// Le quantum de position, en pixels, que l'octet `o` laisse a la sonde a dents.
///
/// C'est le nombre qui a fait tomber deux instruments a la premiere mesure, et
/// il ne se devine pas : la sonde est une rampe lineaire dans la LUMIERE, rangee
/// dans une texture en sRGB. Un pas d'octet ne vaut donc pas la meme position
/// selon l'endroit de la rampe — 0,03 px dans le noir, 0,85 px dans le blanc,
/// un facteur VINGT-NEUF. Une mesure de deplacement lue sur huit bits porte
/// cette incertitude, et le banc doit l'annoncer au lieu de la subir.
fn quantum_octet(o: u8) -> f64 {
    let haut = lineaire(o.saturating_add(1));
    let bas = lineaire(o);
    (haut - bas).abs() * PERIODE
}

/// Le pire quantum de toute la rampe : le plancher de la sonde a huit bits.
fn plancher_huit_bits() -> f64 {
    (0..255u8).map(quantum_octet).fold(0.0f64, f64::max)
}

/// Ce que le rendu a coute, passe par passe, en millisecondes de GPU.
#[derive(Clone, Copy, Default)]
struct Cout {
    flou_h: f64,
    flou_v: f64,
    composer: f64,
}

impl Cout {
    fn total(&self) -> f64 {
        self.flou_h + self.flou_v + self.composer
    }
}

impl Verre {
    fn nouveau() -> (Verre, String, bool) {
        let instance = wgpu::Instance::default();
        let adaptateur = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .expect("aucun adaptateur");
        let nom = adaptateur.get_info().name;
        let horodatage = adaptateur
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY);
        // Rgba16Float est filtrable partout ; Rgba32Float ne l'est que si
        // l'adaptateur le declare. On prend le meilleur des deux et on DIT
        // lequel, parce que la precision du banc en depend directement.
        let f32_filtrable = adaptateur
            .features()
            .contains(wgpu::Features::FLOAT32_FILTERABLE);
        let mut demande = wgpu::Features::empty();
        if horodatage {
            demande |= wgpu::Features::TIMESTAMP_QUERY;
        }
        if f32_filtrable {
            demande |= wgpu::Features::FLOAT32_FILTERABLE;
        }
        let (peripherique, file) = pollster::block_on(adaptateur.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("mesure"),
                required_features: demande,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adaptateur.limits()),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            },
        ))
        .expect("peripherique refuse");
        peripherique.on_uncaptured_error(std::sync::Arc::new(|e| {
            eprintln!("!! wgpu : {e}");
        }));

        let module = peripherique.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("matiere"),
            source: wgpu::ShaderSource::Wgsl(include_str!("matiere.wgsl").into()),
        });

        let texture_entree = |b: u32| wgpu::BindGroupLayoutEntry {
            binding: b,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let disposition = peripherique.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("disposition mesure"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                texture_entree(2),
                texture_entree(3),
            ],
        });

        let agencement = peripherique.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("agencement mesure"),
            bind_group_layouts: &[Some(&disposition)],
            immediate_size: 0,
        });

        let module_v = peripherique.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("verrier"),
            source: wgpu::ShaderSource::Wgsl(include_str!("verrier.wgsl").into()),
        });

        let faire_de = |module: &wgpu::ShaderModule,
                        entree: &str,
                        nom: &str,
                        format: wgpu::TextureFormat| {
            peripherique.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(nom),
                layout: Some(&agencement),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("sommet"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(entree),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let format_fin = if f32_filtrable {
            wgpu::TextureFormat::Rgba32Float
        } else {
            wgpu::TextureFormat::Rgba16Float
        };
        let faire = |e: &str, n: &str, f: wgpu::TextureFormat| faire_de(&module, e, n, f);
        let faire_v = |e: &str, n: &str, f: wgpu::TextureFormat| faire_de(&module_v, e, n, f);
        let tuyau_flou = faire("flou", "flou", FORMAT);
        let tuyau_composer = faire("composer", "composer", FORMAT);
        let tuyau_flou_fin = faire("flou", "flou fin", format_fin);
        let tuyau_composer_fin = faire("composer", "composer fin", format_fin);
        let tuyau_flou_v = faire_v("flou", "flou verrier", FORMAT);
        let tuyau_composer_v = faire_v("composer", "composer verrier", FORMAT);
        let tuyau_flou_v_fin = faire_v("flou", "flou verrier fin", format_fin);
        let tuyau_composer_v_fin = faire_v("composer", "composer verrier fin", format_fin);

        let echantillonneur = peripherique.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("echantillonneur"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let horodateur = if horodatage {
            Some(peripherique.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("horodateur"),
                ty: wgpu::QueryType::Timestamp,
                count: 6,
            }))
        } else {
            None
        };
        let periode_ns = file.get_timestamp_period();

        (
            Verre {
                peripherique,
                file,
                disposition,
                tuyau_flou,
                tuyau_composer,
                tuyau_flou_fin,
                tuyau_composer_fin,
                tuyau_flou_v,
                tuyau_composer_v,
                tuyau_flou_v_fin,
                tuyau_composer_v_fin,
                format_fin,
                echantillonneur,
                horodateur,
                periode_ns,
            },
            nom,
            horodatage,
        )
    }

    fn texture(
        &self,
        l: u32,
        h: u32,
        usage: wgpu::TextureUsages,
        format: wgpu::TextureFormat,
    ) -> wgpu::Texture {
        self.peripherique.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: l,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    }

    fn groupe(
        &self,
        r: &Reglages,
        source: &wgpu::TextureView,
        net: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        use wgpu::util::DeviceExt;
        let tampon = self
            .peripherique
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("reglages"),
                contents: bytemuck::bytes_of(r),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        self.peripherique
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.disposition,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: tampon.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.echantillonneur),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(net),
                    },
                ],
            })
    }

    /// La source passee par un flou gaussien d'ecart-type `sigma`, en deux
    /// passes separees, horizontale puis verticale.
    ///
    /// C'est le meme nuanceur `flou` que celui de la couche floutee, avec un
    /// autre sigma : aucun calcul neuf, donc rien de neuf a croire. La passe est
    /// soumise avant la chaine et hors de tout horodatage — la case 5 mesure une
    /// image, pas un cout.
    fn passe_bas(
        &self,
        base: &Reglages,
        brut: &wgpu::TextureView,
        sigma: f32,
        tuyau: &wgpu::RenderPipeline,
    ) -> wgpu::Texture {
        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
        let moitie = self.texture(SL, SH, usage, FORMAT);
        let fini = self.texture(SL, SH, usage, FORMAT);
        let v_moitie = moitie.create_view(&Default::default());
        let v_fini = fini.create_view(&Default::default());
        let taps = (sigma * 3.0).ceil();
        let mut horizontal = *base;
        horizontal.flou = [1.0, 0.0, sigma, taps];
        let mut vertical = *base;
        vertical.flou = [0.0, 1.0, sigma, taps];
        let gh = self.groupe(&horizontal, brut, brut);
        let gv = self.groupe(&vertical, &v_moitie, brut);
        let mut enc = self.peripherique.create_command_encoder(&Default::default());
        for (groupe, cible) in [(&gh, &v_moitie), (&gv, &v_fini)] {
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: cible,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            passe.set_pipeline(tuyau);
            passe.set_bind_group(0, groupe, &[]);
            passe.draw(0..3, 0..1);
        }
        self.file.submit(Some(enc.finish()));
        fini
    }

    fn rendre(&self, fond: &[u8], d: &Direction) -> Vec<u8> {
        self.chaine(fond, d, 0, il(), ih()).0
    }

    /// Toute la chaine, avec l'horodatage GPU passe par passe quand il est
    /// demande. `tours` a zero rend une seule fois, sans mesurer.
    ///
    /// `cl` et `ch` donnent la taille de la CIBLE de composition. Elle vaut
    /// normalement l'ilot ; la faire varier est le temoin de l'horodateur.
    fn chaine(&self, fond: &[u8], d: &Direction, tours: usize, cl: u32, ch: u32) -> (Vec<u8>, Cout) {
        self.chaine_tracee(fond, d, tours, cl, ch, 1)
    }

    /// La meme chaine, mais la passe de composition trace `traces` fois le MEME
    /// triangle plein ecran.
    ///
    /// C'est ce qui allonge une reference de cout sans rien changer a ce qu'elle
    /// mesure d'autre : meme taille, meme forme, meme nuanceur, meme image. Il
    /// n'y a ni profondeur ni melange, donc chaque trace refait tout le travail
    /// de fragment. Les deux flous ne sont traces qu'une fois : ils ne sont pas
    /// ce qu'on compare.
    fn chaine_tracee(
        &self,
        fond: &[u8],
        d: &Direction,
        tours: usize,
        cl: u32,
        ch: u32,
        traces: u32,
    ) -> (Vec<u8>, Cout) {
        let brut = self.texture(
            SL,
            SH,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            FORMAT,
        );
        self.file.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &brut,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            fond,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SL * 4),
                rows_per_image: Some(SH),
            },
            wgpu::Extent3d {
                width: SL,
                height: SH,
                depth_or_array_layers: 1,
            },
        );

        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
        let inter = self.texture(SL, SH, usage, FORMAT);
        let floute = self.texture(SL, SH, usage, FORMAT);
        let fini = self.texture(
            cl,
            ch,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            FORMAT,
        );

        let v_brut = brut.create_view(&Default::default());
        let v_inter = inter.create_view(&Default::default());
        let v_floute = floute.create_view(&Default::default());
        let v_fini = fini.create_view(&Default::default());

        let taps = (d.sigma * 3.0).ceil();
        let base = Reglages {
            taille: [SL as f32, SH as f32, SL as f32, SH as f32],
            flou: [1.0, 0.0, d.sigma, taps],
            forme: [il() as f32 / 2.0, ih() as f32 / 2.0, rayon(), exposant()],
            teinte: [0.0, 0.0, 0.0, d.teinte],
            reglages: [d.saturation, MARGE as f32, 1.0, 1.0],
            biseau: [d.largeur, d.indice, d.dispersion, d.epaisseur],
            bord: match d.modele {
                Modele::Esquisse => [d.lisere, d.galbe, d.clarte, d.integre],
                Modele::Verrier => [
                    d.lisere,
                    d.profil,
                    d.clarte,
                    // 0 = la couche nette entre telle quelle ; 1 = la vibrance
                    // suit la clarte ; 2 = la couche nette n'apporte que de la
                    // luminance.
                    if d.nette_luminance {
                        2.0
                    } else if d.vibrance_relative {
                        1.0
                    } else {
                        0.0
                    },
                ],
            },
            couches: [d.refraction, d.fresnel, d.beer, d.prises],
            lampe: [d.lampe, d.ciel, 0.0, 0.0],
        };
        let mut vertical = base;
        vertical.flou = [0.0, 1.0, d.sigma, taps];
        let mut composer = base;
        composer.taille = [cl as f32, ch as f32, SL as f32, SH as f32];
        // La forme suit la cible. C'est ce qui rend le temoin 6.1 honnete : le
        // nuanceur de composition n'a pas le meme cout dedans, dehors et dans le
        // biseau, donc changer la taille de la cible SANS changer la forme
        // changerait aussi la PROPORTION de chaque cas, et le temps cesserait
        // d'etre proportionnel a l'aire pour une raison qui n'a rien a voir avec
        // l'horodateur. A forme semblable, la proportion est conservee.
        let echelle = cl as f32 / il() as f32;
        composer.forme = [
            cl as f32 / 2.0,
            ch as f32 / 2.0,
            rayon() * echelle,
            exposant(),
        ];
        if d.conge_relatif {
            // LE CONGE SUIT LA HAUTEUR DE L'OBJET. `b` et `e` ne sont pas des
            // reglages d'effet, ce sont les longueurs d'un solide : le rayon du
            // conge taille sur l'arete et l'epaisseur de la dalle. Quand
            // l'objet change de taille, elles changent avec lui, exactement
            // comme le rayon de coin et l'exposant de DESIGN.md §6 — qui se
            // lisent tous les deux sur la HAUTEUR, parce que ses deux bornes
            // sont fixes (36 et 148) et que celles de la largeur ne le sont
            // pas. La hauteur est aussi, aux deux etats, la PLUS PETITE
            // dimension du galet : c'est elle qui borne ce qu'un conge peut
            // occuper sans avaler l'objet.
            //
            // Le facteur vaut 1 a la taille deployee, ou b = e = 12 a ete
            // decide et mesure. Rien de ce qui est publie ne bouge.
            let f = ch as f32 / PLUS_GRAND_H as f32;
            composer.biseau[0] = d.largeur * f;
            composer.biseau[3] = d.epaisseur * f;
        } else {
            composer.biseau[0] = d.largeur * echelle;
        }

        let (t_flou, t_comp) = match d.modele {
            Modele::Esquisse => (&self.tuyau_flou, &self.tuyau_composer),
            Modele::Verrier => (&self.tuyau_flou_v, &self.tuyau_composer_v),
        };
        // LA COUCHE NETTE, brute ou passee par un passe-bas (`nette_sigma`).
        // Elle n'est liee qu'a la composition : les deux flous ne la lisent pas.
        let filtre = if d.nette_sigma > 0.0 {
            Some(self.passe_bas(&base, &v_brut, d.nette_sigma, t_flou))
        } else {
            None
        };
        let vue_filtre = filtre.as_ref().map(|t| t.create_view(&Default::default()));
        let v_net = vue_filtre.as_ref().unwrap_or(&v_brut);
        let g1 = self.groupe(&base, &v_brut, &v_brut);
        let g2 = self.groupe(&vertical, &v_inter, &v_brut);
        let g3 = self.groupe(&composer, &v_floute, v_net);
        let travail: [(&wgpu::BindGroup, &wgpu::TextureView, &wgpu::RenderPipeline); 3] = [
            (&g1, &v_inter, t_flou),
            (&g2, &v_floute, t_flou),
            (&g3, &v_fini, t_comp),
        ];

        let encoder = |horodater: bool| {
            let mut enc = self
                .peripherique
                .create_command_encoder(&Default::default());
            for (k, (groupe, cible, tuyau)) in travail.iter().enumerate() {
                let ecritures = match (&self.horodateur, horodater) {
                    (Some(qs), true) => Some(wgpu::RenderPassTimestampWrites {
                        query_set: qs,
                        beginning_of_pass_write_index: Some(2 * k as u32),
                        end_of_pass_write_index: Some(2 * k as u32 + 1),
                    }),
                    _ => None,
                };
                let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: cible,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    multiview_mask: None,
                    timestamp_writes: ecritures,
                    occlusion_query_set: None,
                });
                passe.set_pipeline(*tuyau);
                passe.set_bind_group(0, *groupe, &[]);
                // La troisieme passe est la composition : elle seule se trace
                // `traces` fois. Le nuanceur de sommet ignore l'indice
                // d'instance, donc les instances sont le meme triangle.
                let instances = if k == 2 { traces.max(1) } else { 1 };
                passe.draw(0..3, 0..instances);
            }
            enc
        };

        let mut cout = Cout::default();
        if tours > 0 && self.horodateur.is_some() {
            let qs = self.horodateur.as_ref().unwrap();
            let resolu = self.peripherique.create_buffer(&wgpu::BufferDescriptor {
                label: Some("horodatage resolu"),
                size: 256,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let lu = self.peripherique.create_buffer(&wgpu::BufferDescriptor {
                label: Some("horodatage lu"),
                size: 256,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            // Une chauffe : la premiere soumission porte la compilation du
            // nuanceur et les allocations, et elle n'a rien a faire dans la
            // mesure.
            self.file.submit(Some(encoder(false).finish()));
            self.peripherique
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();

            let mut mesures: Vec<[f64; 3]> = Vec::with_capacity(tours);
            for _ in 0..tours {
                let mut enc = encoder(true);
                enc.resolve_query_set(qs, 0..6, &resolu, 0);
                enc.copy_buffer_to_buffer(&resolu, 0, &lu, 0, 48);
                self.file.submit(Some(enc.finish()));
                self.peripherique
                    .poll(wgpu::PollType::wait_indefinitely())
                    .unwrap();
                let tranche = lu.slice(0..48);
                let (tx, rx) = std::sync::mpsc::channel();
                tranche.map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
                self.peripherique
                    .poll(wgpu::PollType::wait_indefinitely())
                    .unwrap();
                rx.recv().unwrap().unwrap();
                let vue = tranche.get_mapped_range().unwrap();
                let t: &[u64] = bytemuck::cast_slice(&vue[..48]);
                let ms = |a: u64, b: u64| {
                    (b.saturating_sub(a)) as f64 * self.periode_ns as f64 / 1e6
                };
                mesures.push([ms(t[0], t[1]), ms(t[2], t[3]), ms(t[4], t[5])]);
                drop(vue);
                lu.unmap();
            }
            for c in 0..3 {
                let mut v: Vec<f64> = mesures.iter().map(|m| m[c]).collect();
                v.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let m = v[v.len() / 2];
                match c {
                    0 => cout.flou_h = m,
                    1 => cout.flou_v = m,
                    _ => cout.composer = m,
                }
            }
        } else {
            self.file.submit(Some(encoder(false).finish()));
        }

        // La relecture, apres la mesure : elle ne doit pas la polluer.
        let ligne = cl * 4;
        let alignee = ligne.div_ceil(256) * 256;
        let lecture = self.peripherique.create_buffer(&wgpu::BufferDescriptor {
            label: Some("relecture"),
            size: (alignee * ch) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self
            .peripherique
            .create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &fini,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &lecture,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(alignee),
                    rows_per_image: Some(ch),
                },
            },
            wgpu::Extent3d {
                width: cl,
                height: ch,
                depth_or_array_layers: 1,
            },
        );
        self.file.submit(Some(enc.finish()));

        let tranche = lecture.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        tranche.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.peripherique
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        rx.recv().unwrap().unwrap();
        let vue = tranche.get_mapped_range().unwrap();
        let mut px = Vec::with_capacity((ligne * ch) as usize);
        for y in 0..ch {
            let dep = (y * alignee) as usize;
            px.extend_from_slice(&vue[dep..dep + ligne as usize]);
        }
        drop(vue);
        lecture.unmap();
        (px, cout)
    }

    /// La meme chaine, mais rendue dans un format FLOTTANT.
    ///
    /// Pourquoi elle existe, et c'est la correction la plus lourde de ce banc.
    /// La sonde a dents est une rampe lineaire dans la LUMIERE, rangee dans une
    /// texture en sRGB. Un pas d'octet n'y vaut donc pas la meme position selon
    /// l'endroit de la rampe : 0,03 px dans le noir, 0,85 px dans le blanc. Ce
    /// plancher de 0,85 px a fait echouer le temoin du zero de l'instrument 1
    /// (0,48 px mesure la ou zero etait attendu) ET celui de la valeur de
    /// l'instrument 3, dont la dispersion a mesurer ne vaut que 0,14 px — cinq
    /// fois SOUS le plancher de sa propre sonde.
    ///
    /// Aucun raffinement de la sonde ne rattrape un facteur vingt-neuf : le
    /// quantum varie comme la derivee de sRGB, et il faudrait une quarantaine de
    /// phases pour gagner un ordre de grandeur. On change donc de porteur, pas
    /// de sonde.
    ///
    /// Ce que cela change, et ce que cela ne change PAS. Le nuanceur est le
    /// meme, la geometrie est la meme, les positions echantillonnees sont les
    /// memes — seul le format de rangement change. En flottant il n'y a plus de
    /// codage sRGB a la lecture ni a l'ecriture, donc la sonde y porte
    /// directement sa valeur lineaire. Avec la teinte a zero et la saturation a
    /// un, la composition rend alors exactement la valeur resamplee. Le temoin
    /// 1.0 le PROUVE au lieu de l'affirmer : les deux formats doivent donner le
    /// meme champ, a la precision du plus grossier des deux.
    fn chaine_fine(&self, fond: &[f32], d: &Direction) -> Vec<f32> {
        let brut = self.texture(
            SL,
            SH,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            self.format_fin,
        );
        let octets: Vec<u8> = if self.format_fin == wgpu::TextureFormat::Rgba32Float {
            bytemuck::cast_slice(fond).to_vec()
        } else {
            // Rgba16Float : on encode nous-memes les demi-flottants.
            fond.iter().flat_map(|v| demi(*v).to_le_bytes()).collect()
        };
        let par_texel = if self.format_fin == wgpu::TextureFormat::Rgba32Float {
            16u32
        } else {
            8
        };
        self.file.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &brut,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &octets,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SL * par_texel),
                rows_per_image: Some(SH),
            },
            wgpu::Extent3d {
                width: SL,
                height: SH,
                depth_or_array_layers: 1,
            },
        );

        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
        let inter = self.texture(SL, SH, usage, self.format_fin);
        let floute = self.texture(SL, SH, usage, self.format_fin);
        let fini = self.texture(
            il(),
            ih(),
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            self.format_fin,
        );
        let v_brut = brut.create_view(&Default::default());
        let v_inter = inter.create_view(&Default::default());
        let v_floute = floute.create_view(&Default::default());
        let v_fini = fini.create_view(&Default::default());

        let taps = (d.sigma * 3.0).ceil();
        let base = Reglages {
            taille: [SL as f32, SH as f32, SL as f32, SH as f32],
            flou: [1.0, 0.0, d.sigma, taps],
            forme: [il() as f32 / 2.0, ih() as f32 / 2.0, rayon(), exposant()],
            teinte: [0.0, 0.0, 0.0, d.teinte],
            reglages: [d.saturation, MARGE as f32, 1.0, 1.0],
            biseau: [d.largeur, d.indice, d.dispersion, d.epaisseur],
            bord: match d.modele {
                Modele::Esquisse => [d.lisere, d.galbe, d.clarte, d.integre],
                Modele::Verrier => [
                    d.lisere,
                    d.profil,
                    d.clarte,
                    // 0 = la couche nette entre telle quelle ; 1 = la vibrance
                    // suit la clarte ; 2 = la couche nette n'apporte que de la
                    // luminance.
                    if d.nette_luminance {
                        2.0
                    } else if d.vibrance_relative {
                        1.0
                    } else {
                        0.0
                    },
                ],
            },
            couches: [d.refraction, d.fresnel, d.beer, d.prises],
            lampe: [d.lampe, d.ciel, 0.0, 0.0],
        };
        let mut vertical = base;
        vertical.flou = [0.0, 1.0, d.sigma, taps];
        let mut composer = base;
        composer.taille = [il() as f32, ih() as f32, SL as f32, SH as f32];
        if d.conge_relatif {
            // La meme loi que dans `chaine`, et pour la meme raison : `b` et `e`
            // sont les longueurs d'un solide, pas des reglages d'effet. Ce
            // chemin-ci sert les instruments de precision, qui travaillent tous
            // a la taille DEPLOYEE ou le facteur vaut exactement 1 — il ne
            // change donc rien a ce qui est publie, et c'est le temoin.
            let f = ih() as f32 / PLUS_GRAND_H as f32;
            composer.biseau[0] = d.largeur * f;
            composer.biseau[3] = d.epaisseur * f;
        }

        let g1 = self.groupe(&base, &v_brut, &v_brut);
        let g2 = self.groupe(&vertical, &v_inter, &v_brut);
        let g3 = self.groupe(&composer, &v_floute, &v_brut);
        let (t_flou, t_comp) = match d.modele {
            Modele::Esquisse => (&self.tuyau_flou_fin, &self.tuyau_composer_fin),
            Modele::Verrier => (&self.tuyau_flou_v_fin, &self.tuyau_composer_v_fin),
        };
        let travail: [(&wgpu::BindGroup, &wgpu::TextureView, &wgpu::RenderPipeline); 3] = [
            (&g1, &v_inter, t_flou),
            (&g2, &v_floute, t_flou),
            (&g3, &v_fini, t_comp),
        ];
        let mut enc = self
            .peripherique
            .create_command_encoder(&Default::default());
        for (groupe, cible, tuyau) in travail {
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: cible,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            passe.set_pipeline(tuyau);
            passe.set_bind_group(0, groupe, &[]);
            passe.draw(0..3, 0..1);
        }

        let ligne = il() * par_texel;
        let alignee = ligne.div_ceil(256) * 256;
        let lecture = self.peripherique.create_buffer(&wgpu::BufferDescriptor {
            label: Some("relecture fine"),
            size: (alignee * ih()) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &fini,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &lecture,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(alignee),
                    rows_per_image: Some(ih()),
                },
            },
            wgpu::Extent3d {
                width: il(),
                height: ih(),
                depth_or_array_layers: 1,
            },
        );
        self.file.submit(Some(enc.finish()));

        let tranche = lecture.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        tranche.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.peripherique
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        rx.recv().unwrap().unwrap();
        let vue = tranche.get_mapped_range().unwrap();
        let mut px = Vec::with_capacity((il() * ih() * 4) as usize);
        for y in 0..ih() {
            let dep = (y * alignee) as usize;
            let brut = &vue[dep..dep + ligne as usize];
            if par_texel == 16 {
                px.extend_from_slice(bytemuck::cast_slice::<u8, f32>(brut));
            } else {
                for k in 0..(il() * 4) as usize {
                    px.push(depuis_demi(u16::from_le_bytes([
                        brut[k * 2],
                        brut[k * 2 + 1],
                    ])));
                }
            }
        }
        drop(vue);
        lecture.unmap();
        px
    }

    /// Le champ de deplacement complet, a deux composantes.
    ///
    /// DEUX reductions, et chacune doit etre justifiee separement.
    ///
    /// 1. Le flou est ramene a sigma 1 et la teinte a zero. C'est l'objet du
    ///    temoin 1.2.
    /// 2. Le LISERE est eteint, et ce n'est pas un confort. Le nuanceur ecrit
    ///    `verre = verre + lumiere * exces * force`, ou `lumiere` est prise a
    ///    deux largeurs de biseau VERS L'INTERIEUR. C'est une ADDITION, et non
    ///    un deplacement d'echantillon. Une sonde qui decode une position est
    ///    aveugle a une addition — pire, elle la lit comme une position fausse.
    ///    Le temoin 1.5 le montre en chiffres. L'innocence de la coupure, elle,
    ///    est etablie autrement : le temoin 1.4 verifie que le champ lu sans
    ///    lisere reproduit le decalage de Snell calcule a partir des seuls
    ///    parametres du biseau, ou le lisere n'entre pas.
    fn champ_de(&self, d: &Direction, sigma: f32, teinte: f32) -> (Champ, Vec<[f64; 3]>) {
        let mut geo = *d;
        geo.sigma = sigma;
        geo.teinte = teinte;
        // La lampe est coupee pour la lecture : la sonde lit une geometrie, et
        // une lumiere qui remplace le pixel par un gris casserait la rampe la
        // ou elle gagne. Le deplacement mesure est celui du verre sans lampe.
        geo.lampe = 0.0;
        geo.saturation = 1.0;
        geo.lisere = 0.0;
        // Le modele du verrier ajoute la reflexion ET multiplie par (1-R) et
        // par Beer-Lambert. Une sonde de position est aveugle a l'addition et
        // FAUSSE sur la multiplication — c'est la meme raison qui met deja la
        // teinte a zero ici. On ne laisse donc que la refraction.
        geo.fresnel = 0.0;
        geo.beer = 0.0;
        self.champ_brut(&geo)
    }

    /// Le champ lu tel quel, sans aucune coupure — c'est ce que le temoin 1.5
    /// compare au precedent.
    fn champ_brut(&self, geo: &Direction) -> (Champ, Vec<[f64; 3]>) {
        let lire = |selon_y: bool| -> Vec<[f64; 3]> {
            let a = champ_fin(
                &self.chaine_fine(&sonde_fine(selon_y, PHASES[0]), geo),
                selon_y,
                PHASES[0],
            );
            let b = champ_fin(
                &self.chaine_fine(&sonde_fine(selon_y, PHASES[1]), geo),
                selon_y,
                PHASES[1],
            );
            fondre(&a, &b, selon_y)
        };
        let cx = lire(false);
        let cy = lire(true);
        (Champ::depuis(&cx, &cy), cx)
    }

    /// Le meme champ, mais lu par la chaine a HUIT BITS. Sert au seul temoin
    /// 1.0, qui compare les deux porteurs.
    fn champ_huit_bits(&self, d: &Direction, sigma: f32) -> Vec<[f64; 3]> {
        let mut geo = *d;
        geo.sigma = sigma;
        geo.teinte = 0.0;
        geo.saturation = 1.0;
        geo.lisere = 0.0;
        // Le modele du verrier ajoute la reflexion ET multiplie par (1-R) et
        // par Beer-Lambert. Une sonde de position est aveugle a l'addition et
        // FAUSSE sur la multiplication — c'est la meme raison qui met deja la
        // teinte a zero ici. On ne laisse donc que la refraction.
        geo.fresnel = 0.0;
        geo.beer = 0.0;
        let a = champ(&self.rendre(&sonde(false, PHASES[0]), &geo), false, PHASES[0]);
        let b = champ(&self.rendre(&sonde(false, PHASES[1]), &geo), false, PHASES[1]);
        fondre(&a, &b, false)
    }

    /// Le deplacement le long du bord GAUCHE, lu par la rampe SANS dent et
    /// moyenne sur soixante rangees.
    ///
    /// En flottant elle cesse d'etre l'instrument grossier qu'elle etait : son
    /// quantum tombe de 1,76 px — un octet pour toute la largeur de la source —
    /// a la precision du flottant. Elle devient donc une vraie mesure du champ
    /// a sigma 18, ou la sonde a dents ne peut pas aller.
    fn profil_rampe(&self, d: &Direction, sigma: f32) -> Vec<f64> {
        let mut geo = *d;
        geo.sigma = sigma;
        geo.teinte = 0.0;
        geo.saturation = 1.0;
        geo.lisere = 0.0;
        // Le modele du verrier ajoute la reflexion ET multiplie par (1-R) et
        // par Beer-Lambert. Une sonde de position est aveugle a l'addition et
        // FAUSSE sur la multiplication — c'est la meme raison qui met deja la
        // teinte a zero ici. On ne laisse donc que la refraction.
        geo.fresnel = 0.0;
        geo.beer = 0.0;
        let px = self.chaine_fine(&rampe_fine(false), &geo);
        (0..60u32)
            .map(|x| {
                let mut s = 0.0;
                let mut n = 0.0;
                for y in ih() / 2 - 30..ih() / 2 + 30 {
                    let i = ((y * il() + x) * 4 + 1) as usize;
                    let source = px[i] as f64 * SL as f64 - 0.5;
                    s += (x + MARGE) as f64 + 0.5 - source;
                    n += 1.0;
                }
                s / n
            })
            .collect()
    }
}

/// L'encodage d'un flottant en demi-precision, pour le repli `Rgba16Float`.
fn demi(v: f32) -> u16 {
    let b = v.to_bits();
    let signe = ((b >> 16) & 0x8000) as u16;
    let mut exp = ((b >> 23) & 0xff) as i32 - 127 + 15;
    let mant = b & 0x7f_ffff;
    if exp >= 0x1f {
        return signe | 0x7c00;
    }
    if exp <= 0 {
        // Denormalise, ou zero. La sonde ne descend jamais la, mais un banc qui
        // laisse un cas non traite finit par tomber dedans.
        if exp < -10 {
            return signe;
        }
        let m = (mant | 0x80_0000) >> (1 - exp) as u32;
        return signe | ((m + 0x1000) >> 13) as u16;
    }
    // Arrondi au plus proche, pair.
    let reste = mant & 0x1fff;
    let mut m = mant >> 13;
    if reste > 0x1000 || (reste == 0x1000 && (m & 1) == 1) {
        m += 1;
        if m == 0x400 {
            m = 0;
            exp += 1;
        }
    }
    signe | ((exp as u16) << 10) | m as u16
}

fn depuis_demi(h: u16) -> f32 {
    let signe = ((h & 0x8000) as u32) << 16;
    let exp = ((h >> 10) & 0x1f) as i32;
    let mant = (h & 0x3ff) as u32;
    if exp == 0 {
        if mant == 0 {
            return f32::from_bits(signe);
        }
        let mut e = -1i32;
        let mut m = mant;
        while m & 0x400 == 0 {
            m <<= 1;
            e -= 1;
        }
        m &= 0x3ff;
        return f32::from_bits(signe | (((e + 15 - 127 + 127) as u32) << 23) | (m << 13));
    }
    if exp == 0x1f {
        return f32::from_bits(signe | 0x7f80_0000 | (mant << 13));
    }
    f32::from_bits(signe | (((exp - 15 + 127) as u32) << 23) | (mant << 13))
}

/// La sonde en dents de scie, portee par un FLOTTANT : la valeur lineaire est
/// rangee telle quelle, sans passer par le codage sRGB qui coutait le plancher.
fn sonde_fine(selon_y: bool, phase: f64) -> Vec<f32> {
    let mut p = vec![1.0f32; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let t = if selon_y { y as f64 } else { x as f64 };
            let v = (((t + 0.5 + phase) / PERIODE).fract()) as f32;
            let i = ((y * SL + x) * 4) as usize;
            p[i] = v;
            p[i + 1] = v;
            p[i + 2] = v;
        }
    }
    p
}

/// La rampe sans dent, flottante.
fn rampe_fine(selon_y: bool) -> Vec<f32> {
    let mut p = vec![1.0f32; (SL * SH * 4) as usize];
    let n = if selon_y { SH } else { SL } as f64;
    for y in 0..SH {
        for x in 0..SL {
            let t = if selon_y { y as f64 } else { x as f64 };
            let v = ((t + 0.5) / n) as f32;
            let i = ((y * SL + x) * 4) as usize;
            p[i] = v;
            p[i + 1] = v;
            p[i + 2] = v;
        }
    }
    p
}

/// Le champ lu sur la chaine flottante. Meme deroulement que `champ`, mais la
/// valeur est deja lineaire : il n'y a plus de decodage sRGB a faire.
fn champ_fin(px: &[f32], selon_y: bool, phase: f64) -> Vec<[f64; 3]> {
    let mut sortie = vec![[0.0f64; 3]; (il() * ih()) as usize];
    for y in 0..ih() {
        for x in 0..il() {
            let i = ((y * il() + x) * 4) as usize;
            let t = if selon_y { y } else { x };
            let nominal = (t + MARGE) as f64 + 0.5;
            for c in 0..3 {
                let v = px[i + c] as f64;
                let fin = v * PERIODE - phase;
                let tours = ((nominal - fin) / PERIODE).round();
                sortie[(y * il() + x) as usize][c] = nominal - (fin + tours * PERIODE);
            }
        }
    }
    sortie
}

// ---------------------------------------------------------------------------

/// Le journal : tout ce qui s'affiche part aussi dans le fichier.
struct Journal(String);

impl Journal {
    fn dire(&mut self, s: &str) {
        println!("{s}");
        let _ = writeln!(self.0, "{s}");
    }
}

macro_rules! dire {
    ($j:expr, $($a:tt)*) => { $j.dire(&format!($($a)*)) };
}

/// Le verdict d'un temoin. Il est CALCULE, jamais ecrit a la main.
///
/// La regle qui l'impose a ete payee : la version precedente de ce banc portait
/// des verdicts en dur. Elle a imprime « l'instrument lit la hauteur qu'on lui a
/// posee » trois lignes sous un tableau disant « pic mesure 45,0, attendu 98,0 »,
/// et « le cout par megapixel se retrouve d'une taille a l'autre » sous une
/// colonne portant +140 %. Un verdict qui ne depend pas du nombre mesure n'est
/// pas un verdict, c'est une legende.
fn verdict(j: &mut Journal, passe: bool, oui: &str, non: &str) -> bool {
    if passe {
        dire!(j, "  VERDICT : {oui}");
    } else {
        dire!(j, "  VERDICT : {non}");
    }
    passe
}

// ---------------------------------------------------------------------------
// Les lieux de l'arete, et le modele du nuanceur recalcule ici
// ---------------------------------------------------------------------------

/// Un lieu de mesure sur l'arete, avec son repere local.
struct Lieu {
    nom: &'static str,
    /// En coordonnees de SOURCE : indice d'ilot + 0,5 + MARGE.
    centre: (f64, f64),
    /// Unitaire, vers l'exterieur.
    normale: (f64, f64),
    /// Unitaire, le long de l'arete.
    tangente: (f64, f64),
    /// Vrai seulement la ou l'arete est une DROITE. Le temoin 1.3 ne vaut que
    /// la, et il faut donc que le lieu le dise lui-meme.
    droit: bool,
}

fn normale_en(p: (f64, f64)) -> (f64, f64) {
    let e = 0.05;
    let gx = (distance((p.0 + e) as f32, p.1 as f32) - distance((p.0 - e) as f32, p.1 as f32))
        as f64
        / (2.0 * e);
    let gy = (distance(p.0 as f32, (p.1 + e) as f32) - distance(p.0 as f32, (p.1 - e) as f32))
        as f64
        / (2.0 * e);
    let n = (gx * gx + gy * gy).sqrt().max(1e-12);
    (gx / n, gy / n)
}

/// Trois lieux : une droite, l'entree du coin, et sa pointe. La courbure de
/// l'arete y vaut respectivement zero, un peu, et son maximum.
fn lieux() -> Vec<Lieu> {
    let coin = (rayon() as f64 - 0.5, rayon() as f64 - 0.5);
    let sur_le_coin = |deg: f64| -> (f64, f64) {
        let (s, c) = deg.to_radians().sin_cos();
        point_du_bord((coin.0 - rayon() as f64 * c, coin.1 - rayon() as f64 * s))
    };
    let bruts: [(&'static str, (f64, f64), bool); 3] = [
        ("bord droit", point_du_bord((4.0, ih() as f64 / 2.0)), true),
        ("entree du coin", sur_le_coin(15.0), false),
        ("pointe du coin", sur_le_coin(45.0), false),
    ];
    bruts
        .iter()
        .map(|(nom, p, droit)| {
            let n = normale_en(*p);
            Lieu {
                nom,
                centre: (p.0 + 0.5 + MARGE as f64, p.1 + 0.5 + MARGE as f64),
                normale: n,
                tangente: (-n.1, n.0),
                droit: *droit,
            }
        })
        .collect()
}

/// Le decalage lateral du nuanceur, recalcule ici a partir du modele qu'il
/// ECRIT — une refraction de Snell sur une face inclinee, traversee d'une dalle
/// d'epaisseur donnee. Sert de reponse connue d'avance aux temoins 1.4 et 3.3.
fn decalage(pente: f64, indice: f64, epaisseur: f64) -> f64 {
    let theta = pente.atan();
    let s = (theta.sin() / indice.max(1.0)).clamp(-1.0, 1.0);
    epaisseur * (theta - s.asin()).tan()
}

/// Tout ce que le conge determine a la distance `d`, recalcule ICI depuis
/// `OPTIQUE.md` §3 a §7 — sans rien demander au nuanceur. C'est la reponse
/// connue d'avance des temoins du modele `Verrier`.
///
/// Renvoie (deplacement vers l'interieur, reflectance exacte, portee de la
/// reflexion vers l'exterieur — negative si le rayon monte vers la chambre,
/// chemin d'absorption rapporte a l'epaisseur).
fn bord_en(d: f64, b: f64, n: f64, e: f64) -> (f64, f64, f64, f64) {
    let sinp = (1.0 + d / b).clamp(0.0, 1.0);
    let cosp = (1.0 - sinp * sinp).max(0.0).sqrt();
    let z = e - b + b * cosp;
    let sint = sinp / n;
    let cost = (1.0 - sint * sint).max(0.0).sqrt();
    // sin(phi - theta_t) et cos(phi - theta_t), sans une seule trigonometrie
    // inverse. `den` EST le cosinus du chemin, que Beer-Lambert reclame.
    let num = sinp * cost - cosp * sint;
    let den = (cosp * cost + sinp * sint).max(1e-9);
    let rs = (cosp - n * cost) / (cosp + n * cost).max(1e-9);
    let rp = (n * cosp - cost) / (n * cosp + cost).max(1e-9);
    let c2 = 1.0 - 2.0 * sinp * sinp;
    let s2 = 2.0 * sinp * cosp;
    let portee = if c2 < 0.0 { z * s2 / (-c2) } else { -1.0 };
    (z * num / den, 0.5 * (rs * rs + rp * rp), portee, (z / e) / den)
}

/// Le decalage attendu au centre d'un pixel situe a la distance `d` du bord.
///
/// Les deux modeles ne posent pas le meme objet, et leurs decalages n'ont meme
/// pas le meme SIGNE : l'esquisse echantillonne vers l'exterieur (elle creuse
/// une gorge), le verrier vers l'interieur (il bombe une arete). C'est
/// exactement le point 1 d'`OPTIQUE.md` §12, et le temoin 1.4 le voit.
fn decalage_attendu(d: f64, dd: &Direction, indice: f64) -> f64 {
    if dd.largeur <= 0.0 {
        return 0.0;
    }
    if dd.modele == Modele::Verrier {
        if dd.refraction <= 0.0 {
            return 0.0;
        }
        // La sonde ne lit pas le decalage du centre du pixel : le nuanceur en
        // moyenne `prises`, etalees sur l'empreinte. On moyenne les memes.
        let n = (dd.prises as i32).max(1);
        let mut s = 0.0;
        for k in 0..n {
            let dd_ = (d - 0.5 + (k as f64 + 0.5) / n as f64).min(0.0);
            s += bord_en(dd_, dd.largeur as f64, indice, dd.epaisseur as f64).0;
        }
        return -s / n as f64;
    }
    let t = (-d / dd.largeur as f64).clamp(0.0, 1.0);
    decalage(
        pente_biseau(t, dd.galbe as f64),
        indice,
        dd.epaisseur as f64,
    )
}

// ===========================================================================
// INSTRUMENT 1 — LA DISTORSION
// ===========================================================================

/// Les sept directions de droite, en degres depuis la TANGENTE a l'arete.
const ANGLES: [f64; 7] = [0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0];
const DEMI_DROITE: f64 = 48.0;

/// La plus grande fleche d'un lieu, et l'angle ou elle tombe.
fn fleche_max(c: &Champ, l: &Lieu) -> (f64, f64, f64) {
    let mut pire = (0.0f64, 0.0f64, 0.0f64);
    for a in ANGLES {
        let (s, co) = a.to_radians().sin_cos();
        let dir = (
            l.tangente.0 * co + l.normale.0 * s,
            l.tangente.1 * co + l.normale.1 * s,
        );
        let (f, _, r) = fleche(c, l.centre, dir, DEMI_DROITE);
        if f > pire.0 {
            pire = (f, a, r);
        }
    }
    pire
}

fn instrument_1(v: &Verre, j: &mut Journal, dirs: &[Direction]) -> bool {
    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "INSTRUMENT 1 — LA DISTORSION");
    dire!(j, "===========================================================");
    dire!(j, "Ce qu'il mesure : de combien une DROITE du fond se courbe en");
    dire!(j, "traversant le bord, en pixels, et jusqu'ou l'effet porte.");
    dire!(j, "");
    dire!(j, "Comment. Deux sondes en dents de scie, l'une en x l'autre en");
    dire!(j, "y, donnent le champ de deplacement a DEUX composantes sur");
    dire!(j, "chacun des {} pixels de l'ilot. L'image d'une droite s'en", il() * ih());
    dire!(j, "deduit par point fixe, et sa FLECHE est l'ecart maximal de");
    dire!(j, "cette image a la corde qui en joint les deux bouts.");
    dire!(j, "");
    dire!(j, "L'angle d'une droite est compte depuis la TANGENTE a l'arete,");
    dire!(j, "et non depuis l'axe des x : c'est la seule convention ou la");
    dire!(j, "meme valeur veut dire la meme chose sur un bord droit et dans");
    dire!(j, "un coin.");

    let lx = lieux();

    // -- TEMOIN 1.0 : le porteur --
    dire!(j, "");
    dire!(j, "-- TEMOIN 1.0 : changer de porteur ne change pas le champ --");
    dire!(j, "La sonde ne se lit plus sur huit bits mais dans un format");
    dire!(j, "flottant, et cette coupure-la se paie d'une preuve.");
    dire!(j, "");
    dire!(j, "La raison. La sonde est une rampe lineaire dans la LUMIERE, rangee");
    dire!(j, "dans une texture en sRGB : un pas d'octet n'y vaut donc pas la meme");
    dire!(j, "position selon l'endroit de la rampe. Calcule ici, ce quantum va de");
    dire!(j, "{:.3} px dans le noir a {:.3} px dans le blanc — un facteur {:.0}.", quantum_octet(1), plancher_huit_bits(), plancher_huit_bits() / quantum_octet(1));
    dire!(j, "C'est ce plancher, et non un defaut du nuanceur, qui a fait lire");
    dire!(j, "0,48 px la ou zero etait attendu.");
    dire!(j, "");
    dire!(j, "La preuve. Le nuanceur, la geometrie et les positions");
    dire!(j, "echantillonnees sont les memes : seul le rangement change. Les deux");
    dire!(j, "porteurs doivent donc donner le MEME champ, a la precision du plus");
    dire!(j, "grossier des deux.");
    let (c_fin, cx_fin) = v.champ_de(&DALLE, 1.0, 0.0);
    let cx_8 = v.champ_huit_bits(&DALLE, 1.0);
    let mut ecart_porteur = 0.0f64;
    let mut quantum_pire = 0.0f64;
    for y in 0..ih() {
        for x in 0..il() {
            if distance(x as f32, y as f32) > -1.0 {
                continue;
            }
            let i = (y * il() + x) as usize;
            ecart_porteur = ecart_porteur.max((cx_fin[i][1] - cx_8[i][1]).abs());
            quantum_pire = quantum_pire.max(plancher_huit_bits());
        }
    }
    dire!(j, "  ecart maximal entre les deux porteurs : {ecart_porteur:.3} px");
    dire!(j, "  plancher du porteur a huit bits       : {quantum_pire:.3} px");
    let t10 = verdict(
        j,
        ecart_porteur <= quantum_pire,
        "les deux porteurs lisent le meme champ, dans les bornes du plus\n  grossier. Le changement de format affine la lecture sans la\n  deplacer — c'est bien le meme objet qui est mesure.",
        "les deux porteurs ne lisent pas le meme champ. Le changement de\n  format n'est donc pas innocent, et la mesure fine ne vaut rien.",
    );
    let _ = c_fin;

    // -- TEMOIN 1.1 : le zero --
    dire!(j, "");
    dire!(j, "-- TEMOIN 1.1 : biseau eteint, le champ doit etre NUL --");
    let (c0, _) = v.champ_de(&TEMOIN, 1.0, 0.0);
    let (m0, x0, y0) = c0.module_max();
    let ext0 = c0.extinction();
    let mut f0 = 0.0f64;
    let mut r0 = 0.0f64;
    for l in &lx {
        let (f, _, r) = fleche_max(&c0, l);
        f0 = f0.max(f);
        r0 = r0.max(r);
    }
    dire!(j, "  module maximal du champ : {m0:.3} px   (au pixel {x0}, {y0})");
    dire!(j, "  extinction              : {ext0:.1} px");
    dire!(j, "  pire fleche sur {} droites : {f0:.3} px", lx.len() * ANGLES.len());
    dire!(j, "  pire residu du point fixe : {r0:.2e} px");
    let t11 = verdict(
        j,
        m0 < 0.05 && f0 < 0.05,
        "le champ est nul et aucune droite ne se courbe. L'instrument\n  ne fabrique pas de distorsion la ou il n'y en a pas.",
        "L'INSTRUMENT EST FAUX. Rien de ce qui suit ne vaut.",
    );

    // -- TEMOIN 1.2 : la reduction --
    dire!(j, "");
    dire!(j, "-- TEMOIN 1.2 : la reduction du banc est-elle innocente ? --");
    dire!(j, "Le champ se lit a sigma 1 et teinte 0, pas aux reglages de");
    dire!(j, "production. Il faut donc prouver que la reduction ne change pas");
    dire!(j, "ce qu'elle mesure — et le prouver AUTREMENT qu'en relisant le");
    dire!(j, "nuanceur, qui est justement l'objet mesure.");
    dire!(j, "");
    dire!(j, "La sonde a dents ne peut pas monter jusqu'a sigma 18 : ses dents");
    dire!(j, "sont a 48 px l'une de l'autre et le flou mord sur 54. On coupe");
    dire!(j, "donc la question en deux.");
    dire!(j, "");
    dire!(j, "  a) la sonde a dents, sur la plage ou elle est licite : sigma 1,");
    dire!(j, "     2 et 4, dont les portees valent 3, 6 et 12 px — toutes trois");
    dire!(j, "     sous les 24 px qui separent chaque pixel de la dent la plus");
    dire!(j, "     proche de la MEILLEURE des deux sondes.");
    let mut pire_sigma = 0.0f64;
    let (ref_c, _) = v.champ_de(&DALLE, 1.0, 0.0);
    for s in [2.0f32, 4.0] {
        let (c, _) = v.champ_de(&DALLE, s, 0.0);
        let mut e = 0.0f64;
        for i in 0..(il() * ih()) as usize {
            e = e.max((c.ux[i] - ref_c.ux[i]).abs());
            e = e.max((c.uy[i] - ref_c.uy[i]).abs());
        }
        dire!(j, "     sigma {s:>4.0} contre sigma 1 : ecart maximal {e:.4} px");
        pire_sigma = pire_sigma.max(e);
    }
    dire!(j, "");
    dire!(j, "  b) la rampe SANS dent, qui elle traverse sigma 18 — au prix de");
    dire!(j, "     sa precision : un octet vaut {:.3} px de position, donc ce", SL as f64 / 255.0);
    dire!(j, "     controle ne detecte un ecart qu'au-dela de ce quantum. Il");
    dire!(j, "     est dit tel quel : il borne, il ne mesure pas.");
    let r1 = v.profil_rampe(&DALLE, 1.0);
    let r18 = v.profil_rampe(&DALLE, 18.0);
    let mut e_rampe = 0.0f64;
    for k in 0..r1.len() {
        e_rampe = e_rampe.max((r1[k] - r18[k]).abs());
    }
    let quantum = SL as f64 / 255.0;
    dire!(j, "     sigma 18 contre sigma 1 : ecart maximal {e_rampe:.3} px");
    dire!(j, "     (quantum de la rampe : {quantum:.3} px)");
    let t12 = verdict(
        j,
        pire_sigma < 0.05 && e_rampe < 0.05,
        "le champ ne bouge pas quand le flou change, ni sur un rapport\n  de 1 a 4 lu par la sonde a dents, ni sur un rapport de 1 a 18 lu\n  par la rampe. La reduction est innocente.",
        "LA REDUCTION MODIFIE CE QU'ELLE MESURE. A ne pas utiliser.",
    );

    // -- TEMOIN 1.5 : pourquoi le lisere doit etre coupe --
    dire!(j, "");
    dire!(j, "-- TEMOIN 1.5 : la coupure du lisere est-elle NECESSAIRE ? --");
    dire!(j, "Le champ se lit lisere ETEINT. Une coupure ne se decrete pas : il");
    dire!(j, "faut montrer qu'elle est necessaire, et dire pourquoi elle est");
    dire!(j, "innocente.");
    dire!(j, "");
    dire!(j, "Necessaire. Le nuanceur ecrit `verre = verre + lumiere * exces *");
    dire!(j, "force`, ou `lumiere` est prise a DEUX LARGEURS DE BISEAU vers");
    dire!(j, "l'interieur. C'est une addition, et non un deplacement");
    dire!(j, "d'echantillon : une sonde qui decode une position ne peut pas la");
    dire!(j, "traverser. Voici ce qu'elle lit quand on la laisse allumee, sur le");
    dire!(j, "bord droit, la ou le decalage vaut quelques pixels :");
    dire!(j, "");
    dire!(j, "  direction          d    lisere eteint   lisere allume");
    dire!(j, "  ---------------------------------------------------------");
    let mut pire15 = 0.0f64;
    for d in dirs.iter().filter(|d| d.lisere > 0.0 && d.largeur > 0.0) {
        let mut allume = *d;
        allume.sigma = 1.0;
        allume.teinte = 0.0;
        allume.saturation = 1.0;
        let (_, cx_a) = v.champ_brut(&allume);
        let (_, cx_e) = v.champ_de(d, 1.0, 0.0);
        let y = ih() / 2;
        for k in 0..3u32 {
            let dd = -0.5 - k as f64;
            let a = cx_a[(y * il() + k) as usize][1];
            let e = cx_e[(y * il() + k) as usize][1];
            dire!(j, "  {:<16} {dd:>5.1} {e:>14.3} px {a:>13.3} px", d.nom);
            pire15 = pire15.max((a - e).abs());
        }
    }
    let t15 = verdict(
        j,
        pire15 > 1.0,
        &format!(
            "le lisere allume decale la lecture de {pire15:.1} px. La coupure est\n  donc necessaire, et non commode. Son innocence, elle, ne repose\n  pas sur cette ligne : c'est le temoin 1.4 qui l'etablit, en\n  retrouvant le decalage de Snell calcule a partir des seuls\n  parametres du biseau — ou le lisere n'entre pas."
        ),
        "le lisere ne perturbe pas la lecture : la coupure n'etait donc\n  pas necessaire, et il faut la retirer plutot que la justifier.",
    );

    // -- TEMOIN 1.3 : deux angles dont la fleche est nulle PAR GEOMETRIE --
    dire!(j, "");
    dire!(j, "-- TEMOIN 1.3 : un effet VIVANT dont la reponse reste connue --");
    dire!(j, "Un zero ne prouve qu'une chose : que l'instrument ne ment pas");
    dire!(j, "par exces. Il faut encore un cas ou l'effet est allume et ou la");
    dire!(j, "reponse se DEMONTRE.");
    dire!(j, "");
    dire!(j, "Sur un bord DROIT, le deplacement est porte par la normale et ne");
    dire!(j, "depend que de la distance au bord. Une droite PARALLELE a l'arete");
    dire!(j, "a donc tous ses points a la meme distance : elle est translatee en");
    dire!(j, "bloc, et reste droite. Une droite PERPENDICULAIRE voit ses points");
    dire!(j, "glisser le long d'elle-meme : elle reste sur la meme droite. Les");
    dire!(j, "deux fleches valent zero, exactement, biseau allume.");
    dire!(j, "");
    dire!(j, "LA LONGUEUR DE LA DROITE, et c'est la mesure qui l'a imposee. Ce");
    dire!(j, "temoin a d'abord echoue a 0,67 px, et l'instrument avait raison :");
    dire!(j, "le bord gauche n'est droit que sur {:.0} px — de y = {:.0} a y = {:.0},", ih() as f64 - 2.0 * rayon() as f64, rayon(), ih() as f32 - rayon());
    dire!(j, "entre les deux coins. Une droite de {:.0} px de demi-longueur entrait", DEMI_DROITE);
    dire!(j, "donc dans les DEUX coins, ou la normale tourne et ou la");
    dire!(j, "demonstration ne vaut plus. Ce n'etait pas une courbure fabriquee,");
    dire!(j, "c'etait une vraie courbure, mesuree hors du domaine du temoin.");
    let demi_droit = ih() as f64 / 2.0 - rayon() as f64 - 4.0;
    dire!(j, "La demi-longueur est donc bornee a {demi_droit:.0} px, qui tient dans le droit.");
    dire!(j, "");
    dire!(j, "  direction       0 deg (parallele)   90 deg (perpendiculaire)   45 deg (temoin vivant)");
    let mut t13 = true;
    for d in dirs.iter().filter(|d| d.largeur > 0.0) {
        let (c, _) = v.champ_de(d, 1.0, 0.0);
        let l = lx.iter().find(|l| l.droit).unwrap();
        let une = |a: f64| -> f64 {
            let (s, co) = a.to_radians().sin_cos();
            let dir = (
                l.tangente.0 * co + l.normale.0 * s,
                l.tangente.1 * co + l.normale.1 * s,
            );
            fleche(&c, l.centre, dir, demi_droit).0
        };
        let (p0, p90, p45) = (une(0.0), une(90.0), une(45.0));
        dire!(j, "  {:<16} {:>10.4} px {:>19.4} px {:>19.4} px", d.nom, p0, p90, p45);
        if p0 > 0.05 || p90 > 0.05 {
            t13 = false;
        }
    }
    let t13 = verdict(
        j,
        t13,
        "les deux angles dont la fleche se demontre nulle la rendent\n  nulle, biseau allume. L'instrument sait donc distinguer une\n  TRANSLATION d'une COURBURE — ce qui est toute sa raison d'etre.",
        "l'instrument confond un deplacement avec une courbure. FAUX.",
    );

    // -- TEMOIN 1.4 : la valeur du deplacement, et non seulement son existence --
    dire!(j, "");
    dire!(j, "-- TEMOIN 1.4 : le champ lu a-t-il la BONNE VALEUR ? --");
    dire!(j, "On compare, le long du bord droit, le deplacement lu au");
    dire!(j, "decalage que le modele du nuanceur pose — Snell sur une face");
    dire!(j, "inclinee, traversee d'une dalle — recalcule ici de zero. Un");
    dire!(j, "instrument peut voir un effet et se tromper de moitie sur sa");
    dire!(j, "taille ; celui-la ne le peut plus.");
    dire!(j, "");
    dire!(j, "  direction        d       lu        attendu     ecart");
    let mut t14 = true;
    for d in dirs.iter().filter(|d| d.largeur > 0.0) {
        let (_, cx) = v.champ_de(d, 1.0, 0.0);
        let y = ih() / 2;
        for k in 0..4u32 {
            let dd = -0.5 - k as f64;
            let lu = cx[(y * il() + k) as usize][1];
            let at = decalage_attendu(dd, d, d.indice as f64);
            dire!(j, "  {:<16} {:>5.1} {:>8.3} px {:>10.3} px {:>8.3} px", d.nom, dd, lu, at, lu - at);
            if (lu - at).abs() > 0.15 {
                t14 = false;
            }
        }
    }
    let t14 = verdict(
        j,
        t14,
        "le champ lu reproduit le decalage pose, a moins de 0,15 px. Ce\n  que l'instrument rapporte est donc bien des pixels de fond, et\n  non une grandeur arbitraire.",
        "le champ lu ne reproduit pas le decalage pose. Tant qu'on ne\n  sait pas lequel des deux a tort, l'instrument ne tranche rien.",
    );

    let valide = t10 && t11 && t12 && t13 && t14 && t15;

    // -- LA MESURE --
    dire!(j, "");
    if !valide {
        dire!(j, "-- PAS DE MESURE : un temoin a echoue, l'instrument est faux. --");
        return false;
    }
    dire!(j, "-- LA MESURE, sur l'etat actuel du labo --");
    dire!(j, "");
    dire!(j, "  direction         |u| max   ou (du bord)   extinction   fleche max   ou, et a quel angle");
    dire!(j, "  ----------------------------------------------------------------------------------------------");
    for d in dirs {
        let (c, _) = v.champ_de(d, 1.0, 0.0);
        let (m, mx, my) = c.module_max();
        let ou = -distance(mx as f32, my as f32) as f64;
        let mut pire = (0.0f64, 0.0f64, "");
        for l in &lx {
            let (f, a, _) = fleche_max(&c, l);
            if f > pire.0 {
                pire = (f, a, l.nom);
            }
        }
        // Un champ nul n'a pas de « lieu du maximum » : le dire plutot que de
        // rendre l'argmax d'une egalite, qui ne serait qu'un artefact de
        // balayage.
        let lieu_max = if m > 0.001 {
            format!("{ou:>9.1} px")
        } else {
            format!("{:>12}", "—")
        };
        let lieu_fleche = if pire.0 > 0.001 {
            format!("{} a {:.0} deg", pire.2, pire.1)
        } else {
            "—".to_string()
        };
        dire!(
            j,
            "  {:<16} {:>7.2} px {lieu_max} {:>11.1} px {:>11.2} px   {lieu_fleche}",
            d.nom, m, c.extinction(), pire.0
        );
    }

    dire!(j, "");
    dire!(j, "La fleche par ANGLE et par LIEU, direction A. C'est le tableau");
    dire!(j, "qui explique pourquoi une grille alignee sur les axes ne dirait");
    dire!(j, "rien : a 0 et a 90 degres la fleche est nulle par geometrie");
    dire!(j, "(temoin 1.3). Il faut de l'obliquite, et de la courbure d'arete.");
    dire!(j, "");
    dire!(j, "  lieu                0 deg    15     30     45     60     75     90");
    dire!(j, "  --------------------------------------------------------------------");
    let (ca, _) = v.champ_de(&DALLE, 1.0, 0.0);
    for l in &lx {
        let mut ligne = format!("  {:<18}", l.nom);
        for a in ANGLES {
            let (s, co) = a.to_radians().sin_cos();
            let dir = (
                l.tangente.0 * co + l.normale.0 * s,
                l.tangente.1 * co + l.normale.1 * s,
            );
            let _ = write!(ligne, "{:>6.2}", fleche(&ca, l.centre, dir, DEMI_DROITE).0);
            let _ = write!(ligne, " ");
        }
        dire!(j, "{ligne}");
    }

    // Le repli : la source doit rester croissante quand on avance.
    dire!(j, "");
    dire!(j, "Et le REPLI, qui est l'autre question que pose une droite : en");
    dire!(j, "avancant d'un pixel vers le coeur, le point de fond dont on tire");
    dire!(j, "la lumiere doit avancer lui aussi. S'il recule, l'image se");
    dire!(j, "retourne — c'est le regime de la loupe, et il se voit.");
    dire!(j, "");
    dire!(j, "  direction         source monotone   plus grand recul");
    dire!(j, "  ------------------------------------------------------");
    for d in dirs {
        let (_, cx) = v.champ_de(d, 1.0, 0.0);
        let y = ih() / 2;
        let mut recul = 0.0f64;
        for x in 1..80u32 {
            let s0 = (x - 1 + MARGE) as f64 + 0.5 - cx[(y * il() + x - 1) as usize][1];
            let s1 = (x + MARGE) as f64 + 0.5 - cx[(y * il() + x) as usize][1];
            recul = recul.max(-(s1 - s0));
        }
        dire!(
            j,
            "  {:<16} {:>15} {:>12.3} px",
            d.nom,
            if recul <= 0.0 { "oui" } else { "NON" },
            recul.max(0.0)
        );
    }

    for a in [0.0f64, 45.0, 90.0] {
        // L'ARBITREE commence par « A » comme la dalle : sans ce filtre, elle
        // ecraserait les planches de mire de la direction A.
        for d in dirs.iter().filter(|d| !d.nom.starts_with("ARBITREE")) {
            let px = v.rendre(&mire(a, 24.0), d);
            let court = match d.nom.chars().next().unwrap() {
                'A' => "A",
                'B' => "B",
                'C' => "C",
                _ => "temoin",
            };
            ecrire_png(
                &format!("captures/mesure/mire-{a:.0}-{court}.png"),
                &px,
                il(),
                ih(),
            );
        }
    }
    dire!(j, "");
    dire!(j, "  planches de mire ecrites dans captures/mesure/ (mire-0, -45, -90)");
    true
}


// ===========================================================================
// INSTRUMENT 2 — LA REFLECTANCE DU BORD
// ===========================================================================

/// Le niveau LINEAIRE du plat du verre sur un fond uni, calcule ici de zero.
///
/// La teinte est noire et se pose sur les octets encodes (VERRE-TAILLE.md §3) :
/// l'octet de sortie vaut donc exactement `(1 - teinte)` fois celui du fond.
/// C'est ce qui rend la prediction du temoin 2.2 fermee, sans rien demander au
/// nuanceur.
fn plat_lineaire(fond: u8, teinte: f64) -> f64 {
    let s = vers_srgb(lineaire(fond));
    depuis_srgb(s * (1.0 - teinte))
}

/// L'exces du lisere tel que le nuanceur l'ECRIT aujourd'hui : couche d'auteur
/// lineaire en distance quand `bord.w` est pose, Fresnel-Schlick pris au seul
/// centre du pixel sinon.
fn exces_auteur(d: f64, dd: &Direction) -> f64 {
    if dd.integre > 0.01 {
        (1.0 + d / dd.integre as f64).clamp(0.0, 1.0)
    } else {
        let t = (-d / (dd.largeur as f64).max(1e-6)).clamp(0.0, 1.0);
        let p = pente_biseau(t, dd.galbe as f64);
        let r0 = ((dd.indice as f64 - 1.0) / (dd.indice as f64 + 1.0)).powi(2);
        let cp = 1.0 / (1.0 + p * p).sqrt();
        (fresnel_schlick(cp, dd.indice as f64) - r0).max(0.0) / (1.0 - r0)
    }
}

/// Le niveau PREDIT au bac `d` sur un fond uni. Sur un fond uni la refraction ne
/// change rien — elle deplace l'echantillon dans une image constante — donc tout
/// ce qui reste est le plat, plus le lisere qui s'ajoute en lineaire.
fn niveau_predit(d: f64, fond: u8, dd: &Direction) -> f64 {
    if dd.modele == Modele::Verrier {
        return niveau_verrier(d, fond, dd);
    }
    let plat = plat_lineaire(fond, dd.teinte as f64);
    let ajout = if dd.lisere > 0.0 && dd.largeur > 0.0 {
        lineaire(fond) * exces_auteur(d, dd) * dd.lisere as f64
    } else {
        0.0
    };
    octet(plat + ajout) as f64
}

/// Le niveau predit par le modele du VERRIER au bac `d`, sur un fond uni.
///
/// La prediction est fermee, et c'est ce qui en fait un temoin : sur un fond uni
/// la refraction ne peut rien changer — elle deplace l'echantillon dans une
/// image constante — et la reflexion, ou qu'elle vise, ramene la MEME valeur.
/// Il ne reste donc que trois nombres, tous calculables ici de zero : le chemin
/// d'absorption, la part transmise `(1-R)` et la part reflechie `R`.
fn niveau_verrier(d: f64, fond: u8, dd: &Direction) -> f64 {
    let n = dd.indice as f64;
    let r0 = ((n - 1.0) / (n + 1.0)).powi(2);
    let l = lineaire(fond);
    let (b, e) = (dd.largeur as f64, dd.epaisseur as f64);

    let (mut transmis, mut chemin, mut reflet) = (1.0 - r0, 1.0, 0.0);
    if b > 0.0 && d > -b - 1.0 {
        let prises = (dd.prises as i32).max(1);
        let (mut st, mut sc, mut sr) = (0.0, 0.0, 0.0);
        for k in 0..prises {
            let dk = (d - 0.5 + (k as f64 + 0.5) / prises as f64).min(0.0);
            let (_, r, portee, ch) = bord_en(dk, b, n, e);
            st += 1.0 - r;
            sc += ch;
            // Sous 45 degres le rayon monte vers la chambre, qui est noire :
            // le terme vaut zero, et c'est la seule convention du modele.
            if portee >= 0.0 {
                sr += r * l;
            }
        }
        let inv = 1.0 / prises as f64;
        transmis = st * inv;
        chemin = sc * inv;
        reflet = sr * inv;
    }

    let tau = if dd.beer > 0.0 {
        1.0 - (1.0 - dd.teinte as f64).powf(chemin)
    } else {
        dd.teinte as f64
    };
    let mut verre = depuis_srgb(vers_srgb(l) * (1.0 - tau));
    if dd.fresnel > 0.0 {
        verre = verre * transmis + reflet * dd.lisere as f64;
    }
    octet(verre) as f64
}

/// Le niveau qu'un lisere de FRESNEL donnerait au meme endroit — exact non
/// polarise ou Schlick, moyenne sur l'empreinte du pixel.
fn niveau_fresnel(d: f64, fond: u8, dd: &Direction, exact: bool) -> f64 {
    if dd.modele == Modele::Verrier {
        // Le verrier EST du Fresnel exact moyenne sur l'empreinte : la colonne
        // n'a plus de sens comme comparaison, elle se confond avec la prediction.
        return niveau_verrier(d, fond, dd);
    }
    let plat = plat_lineaire(fond, dd.teinte as f64);
    let e = exces_fresnel(
        d,
        dd.largeur as f64,
        dd.indice as f64,
        dd.galbe as f64,
        exact,
    );
    octet(plat + lineaire(fond) * e * dd.lisere as f64) as f64
}

/// L'amplitude d'un profil entre deux distances, en niveaux.
fn amplitude(pr: &[(f64, f64, usize)], de: f64, a: f64) -> (f64, f64, f64) {
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    for (c, v, n) in pr {
        if *n == 0 || !v.is_finite() || *c > de || *c < a {
            continue;
        }
        mn = mn.min(*v);
        mx = mx.max(*v);
    }
    if mn.is_finite() {
        (mn, mx, mx - mn)
    } else {
        (f64::NAN, f64::NAN, f64::NAN)
    }
}

fn instrument_2(v: &Verre, j: &mut Journal, dirs: &[Direction]) -> bool {
    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "INSTRUMENT 2 — LA REFLECTANCE DU BORD");
    dire!(j, "===========================================================");
    dire!(j, "Ce qu'il mesure : le profil de luminance perpendiculaire a");
    dire!(j, "l'arete, de l'exterieur vers le coeur, en niveaux — et de");
    dire!(j, "combien il s'ecarte de ce que Fresnel predit au meme endroit.");
    dire!(j, "");
    dire!(j, "Comment. On n'interpole pas : on range les pixels par leur");
    dire!(j, "distance signee — que ce banc recalcule lui-meme, sans rien");
    dire!(j, "demander au nuanceur — et on moyenne par bande d'un pixel. Les");
    dire!(j, "bacs sont centres sur les DEMI-ENTIERS, parce que c'est la que");
    dire!(j, "les centres de pixels tombent sur un bord droit.");

    let clair: u8 = 226;
    let fond = uni(clair);

    // -- TEMOIN 2.1 : le plat --
    dire!(j, "");
    dire!(j, "-- TEMOIN 2.1 : lisere eteint, le profil doit etre PLAT --");
    dire!(j, "Le biseau, la refraction et la clarte restent ALLUMES : c'est ce");
    dire!(j, "qui rend le temoin utile. Un profil plat prouve alors que");
    dire!(j, "l'instrument ne fabrique pas de relief a partir de la seule");
    dire!(j, "geometrie, et non qu'il ne se passe rien.");
    let mut sans = DALLE;
    sans.lisere = 0.0;
    dire!(j, "");
    dire!(j, "LE BAC DE L'ARETE EST HORS DU TEMOIN, et il faut dire pourquoi.");
    dire!(j, "Le masque du galet s'anticrenele sur exactement un pixel, de");
    dire!(j, "d = -0,5 a d = +0,5. Sur un bord droit tous les centres de pixels");
    dire!(j, "tombent sur d = -0,5 pile, ou la couverture vaut encore un. Dans un");
    dire!(j, "coin ils tombent ou ils veulent : le bac de l'arete y contient donc");
    dire!(j, "des pixels DEJA entames par l'anticrenelage, qui melangent du fond");
    dire!(j, "intact au verre. Ce n'est pas l'instrument qui invente — c'est");
    dire!(j, "reellement dans l'image — mais cela ne se compare pas d'une zone a");
    dire!(j, "l'autre. La platitude se verifie donc a partir de d = -1,5, et le");
    dire!(j, "bac de l'arete est rendu a part, avec ce qu'il vaut.");
    let px_sans = v.rendre(&fond, &sans);
    let mut t21 = true;
    dire!(j, "");
    for (zone, nom) in [(Zone::Droit, "bord droit"), (Zone::Coin, "coin")] {
        let pr = profil_normal(&px_sans, zone, 32, TOLERANCE);
        let (mn, mx, amp) = amplitude(&pr, -1.5, -30.5);
        let arete = au(&pr, -0.5).unwrap_or(f64::NAN);
        dire!(
            j,
            "  {nom:<11} : de d = -1,5 a d = -30,5, min {mn:.1}  max {mx:.1}  amplitude {amp:.2} niveau   | arete d = -0,5 : {arete:.1}"
        );
        if !(amp < 1.0) {
            t21 = false;
        }
    }
    let t21 = verdict(
        j,
        t21,
        "le profil est plat a moins d'un niveau pres, dans les deux\n  zones, des que l'on sort de la rampe d'anticrenelage.\n  L'instrument ne fabrique pas de relief la ou il n'y en a pas.",
        "l'instrument voit un relief la ou le nuanceur n'en pose aucun. FAUX.",
    );

    // -- TEMOIN 2.2 : un relief dont la hauteur SE CALCULE --
    dire!(j, "");
    dire!(j, "-- TEMOIN 2.2 : un relief dont la hauteur SE CALCULE --");
    dire!(j, "Un profil plat ne prouve que l'absence de mensonge par exces. On");
    dire!(j, "allume donc le lisere sur une couche d'auteur de 4 px, a force");
    dire!(j, "0,50, sur un fond UNI — ou la refraction ne peut rien changer,");
    dire!(j, "puisqu'elle deplace l'echantillon dans une image constante. Le");
    dire!(j, "niveau attendu est alors ferme : le plat teinte, plus le fond");
    dire!(j, "multiplie par la rampe du lisere, ajoutes en LINEAIRE.");
    let mut connu = DALLE;
    connu.lisere = 0.5;
    connu.integre = 4.0;
    connu.clarte = 0.0;
    let px_connu = v.rendre(&fond, &connu);
    let pr_connu = profil_normal(&px_connu, Zone::Droit, 12, TOLERANCE);
    dire!(j, "");
    dire!(j, "    d       attendu      mesure     ecart");
    dire!(j, "    ------------------------------------------");
    let mut pire22 = 0.0f64;
    for k in 0..7 {
        let d = -0.5 - k as f64;
        let at = niveau_predit(d, clair, &connu);
        match au(&pr_connu, d) {
            Some(m) => {
                dire!(j, "  {d:>5.1} {at:>11.1} {m:>11.1} {:>9.1}", m - at);
                pire22 = pire22.max((m - at).abs());
            }
            None => dire!(j, "  {d:>5.1} {at:>11.1}        vide"),
        }
    }
    let t22 = verdict(
        j,
        pire22 <= 1.0,
        &format!(
            "l'instrument retrouve une hauteur calculee d'avance a {pire22:.1}\n  niveau pres, sur SEPT distances. Il voit donc un relief, et il\n  le voit a la bonne valeur."
        ),
        &format!(
            "l'instrument s'ecarte de {pire22:.1} niveaux de la hauteur posee.\n  Tant qu'on ne sait pas lequel des deux a tort, il ne tranche rien."
        ),
    );

    // -- TEMOIN 2.3 : la meme reponse sur une droite et dans un coin --
    dire!(j, "");
    dire!(j, "-- TEMOIN 2.3 : la meme reponse sur une droite et dans un coin --");
    dire!(j, "Le lisere ne depend que de la distance au bord. Un instrument qui");
    dire!(j, "rangerait mal les pixels du coin — ou les centres ne tombent plus");
    dire!(j, "sur les demi-entiers — donnerait deux profils differents. La");
    dire!(j, "rampe est elargie a 16 px pour que la variation DANS un bac reste");
    dire!(j, "petite : c'est la resolution de l'instrument, pas une commodite.");
    let mut large = connu;
    large.integre = 16.0;
    let px_large = v.rendre(&fond, &large);
    let pd = profil_normal(&px_large, Zone::Droit, 20, TOLERANCE);
    let pc = profil_normal(&px_large, Zone::Coin, 20, TOLERANCE);
    let mut pire23 = 0.0f64;
    dire!(j, "");
    dire!(j, "    d      bord droit      coin      ecart");
    dire!(j, "    ------------------------------------------");
    for k in 0..8 {
        let d = -0.5 - 2.0 * k as f64;
        if let (Some(a), Some(b)) = (au(&pd, d), au(&pc, d)) {
            dire!(j, "  {d:>5.1} {a:>13.1} {b:>11.1} {:>9.1}", b - a);
            pire23 = pire23.max((b - a).abs());
        }
    }
    let t23 = verdict(
        j,
        pire23 <= 3.0,
        &format!(
            "les deux zones donnent le meme profil a {pire23:.1} niveaux pres. Le\n  rangement par distance recalculee tient dans le coin, ou la\n  grille ne l'aide plus."
        ),
        &format!(
            "les deux zones divergent de {pire23:.1} niveaux. Ce que l'instrument\n  appelle « profil du bord » depend donc de l'endroit choisi."
        ),
    );

    let valide = t21 && t22 && t23;
    dire!(j, "");
    if !valide {
        dire!(j, "-- PAS DE MESURE : un temoin a echoue, l'instrument est faux. --");
        return false;
    }

    dire!(j, "-- LA MESURE, sur l'etat actuel du labo --");
    dire!(j, "Fond uni 226 : le pire cas, tout ce qui peut se reflechir est");
    dire!(
        j,
        "allume. Le plat du verre y vaut {} niveaux.",
        niveau_predit(-99.0, clair, &TEMOIN)
    );
    dire!(j, "");
    dire!(j, "  direction         zone         d=+0,5  d=-0,5  d=-1,5  d=-2,5  d=-3,5  d=-5,5  d=-7,5  d=-11,5");
    dire!(j, "  ------------------------------------------------------------------------------------------------");
    for d in dirs {
        let px = v.rendre(&fond, d);
        for (zone, nom) in [(Zone::Droit, "bord droit"), (Zone::Coin, "coin")] {
            let pr = profil_normal(&px, zone, 16, TOLERANCE);
            let mut ligne = format!("  {:<17} {:<11}", d.nom, nom);
            for dd in [0.5, -0.5, -1.5, -2.5, -3.5, -5.5, -7.5, -11.5] {
                match au(&pr, dd) {
                    Some(x) => {
                        let _ = write!(ligne, "{x:>8.1}");
                    }
                    None => {
                        let _ = write!(ligne, "{:>8}", "-");
                    }
                }
            }
            dire!(j, "{ligne}");
        }
    }

    dire!(j, "");
    dire!(j, "Et ce que FRESNEL donnerait au meme endroit, calcule ici de zero,");
    dire!(j, "sans rien demander au nuanceur. « auteur » est la couche que le");
    dire!(j, "nuanceur porte AUJOURD'HUI — une rampe lineaire de bord.w pixels ;");
    dire!(j, "« exact » est Fresnel non polarise moyenne sur l'empreinte du");
    dire!(j, "pixel ; « Schlick » l'approximation que le nuanceur utilise pour");
    dire!(j, "l'amplitude.");
    for d in dirs.iter().filter(|d| d.lisere > 0.0 && d.largeur > 0.0) {
        let px = v.rendre(&fond, d);
        let pr = profil_normal(&px, Zone::Droit, 16, TOLERANCE);
        dire!(j, "");
        dire!(
            j,
            "  {} — biseau {:.0} px, n = {:.2}, force {:.2}, couche d'auteur {:.0} px",
            d.nom,
            d.largeur,
            d.indice,
            d.lisere,
            d.integre
        );
        dire!(j, "    d      MESURE   auteur   ecart |  exact  Schlick   ecart a l'exact");
        dire!(j, "    -------------------------------------------------------------------");
        for k in 0..8 {
            let dd = -0.5 - k as f64;
            if let Some(m) = au(&pr, dd) {
                let a = niveau_predit(dd, clair, d);
                let fe = niveau_fresnel(dd, clair, d, true);
                let fs = niveau_fresnel(dd, clair, d, false);
                dire!(
                    j,
                    "  {dd:>5.1} {m:>10.1} {a:>8.1} {:>7.1} | {fe:>6.1} {fs:>8.1} {:>13.1}",
                    m - a,
                    m - fe
                );
            }
        }
    }
    true
}

// ===========================================================================
// INSTRUMENT 3 — LA DISPERSION
// ===========================================================================

/// L'ecart R-B du champ, en pixels, et ou il tombe. Le fond de la sonde est GRIS
/// sur les trois canaux : tout ecart mesure vient donc de la dispersion.
fn ecart_canaux(cx: &[[f64; 3]]) -> (f64, f64) {
    let mut pire = 0.0f64;
    let mut ou = 0.0f64;
    for y in 0..ih() {
        for x in 0..il() {
            let d = distance(x as f32, y as f32) as f64;
            if d > -1.0 {
                continue;
            }
            let t = cx[(y * il() + x) as usize];
            let e = t[0] - t[2];
            if e.abs() > pire.abs() {
                pire = e;
                ou = -d;
            }
        }
    }
    (pire, ou)
}

/// La frange VUE : le plus grand ecart entre canaux, en niveaux, dans la bande
/// du bord, sur un fond ACHROMATIQUE.
fn frange(px: &[u8], jusqu_a: f64) -> (f64, f64) {
    let mut pire = 0.0f64;
    let mut ou = 0.0f64;
    for y in 0..ih() {
        for x in 0..il() {
            let d = distance(x as f32, y as f32) as f64;
            if d > -0.5 || d < -jusqu_a {
                continue;
            }
            let i = ((y * il() + x) * 4) as usize;
            let e = (px[i] as f64 - px[i + 2] as f64).abs();
            if e > pire {
                pire = e;
                ou = -d;
            }
        }
    }
    (pire, ou)
}

fn instrument_3(v: &Verre, j: &mut Journal, dirs: &[Direction]) -> bool {
    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "INSTRUMENT 3 — LA DISPERSION");
    dire!(j, "===========================================================");
    dire!(j, "Ce qu'il mesure : l'ecart entre les canaux au bord, en PIXELS,");
    dire!(j, "et ce que l'oeil en recoit, en NIVEAUX. Les deux, parce qu'un");
    dire!(j, "ecart en pixels qui ne se pose pas sur la grille ne produit");
    dire!(j, "aucune frange, et qu'une frange est ce qui se voit.");
    dire!(j, "");
    dire!(j, "Le fond de la sonde est GRIS sur les trois canaux : tout ecart");
    dire!(j, "mesure vient donc de la dispersion et de rien d'autre.");

    // -- TEMOIN 3.1 : le zero, avec la refraction ALLUMEE --
    dire!(j, "");
    dire!(j, "-- TEMOIN 3.1 : dispersion nulle, ecart NUL --");
    dire!(j, "Le biseau et la refraction restent allumes : le champ vaut donc");
    dire!(j, "plusieurs pixels. C'est ce qui rend le zero utile — il dit que");
    dire!(j, "les trois canaux se deplacent ENSEMBLE, et non que rien ne bouge.");
    let mut sans = DALLE;
    sans.dispersion = 0.0;
    let (c_sans, cx_sans) = v.champ_de(&sans, 1.0, 0.0);
    let (e31, ou31) = ecart_canaux(&cx_sans);
    let (m31, _, _) = c_sans.module_max();
    dire!(j, "  module du champ (la refraction est bien active) : {m31:.2} px");
    dire!(j, "  ecart R-B maximal : {e31:.4} px   (a {ou31:.1} px du bord)");
    let t31 = verdict(
        j,
        e31.abs() < 0.01 && m31 > 0.5,
        "les trois canaux se deplacent ensemble alors que le champ vaut\n  plusieurs pixels. L'instrument ne fabrique pas de dispersion.",
        "l'instrument voit un ecart entre canaux la ou le nuanceur n'en\n  pose aucun. FAUX.",
    );

    // -- TEMOIN 3.2 : il doit VOIR --
    dire!(j, "");
    dire!(j, "-- TEMOIN 3.2 : une dispersion FORTE doit se voir --");
    dire!(j, "Un zero ne prouve rien s'il se confond avec un instrument aveugle.");
    let mut forte = DALLE;
    forte.dispersion = 0.30;
    let (_, cx_forte) = v.champ_de(&forte, 1.0, 0.0);
    let (e32, ou32) = ecart_canaux(&cx_forte);
    dire!(j, "  ecart R-B maximal a dispersion 0,30 : {e32:.3} px   (a {ou32:.1} px du bord)");
    let t32 = verdict(
        j,
        e32.abs() > 0.5,
        "il voit. Le zero du temoin 3.1 est donc un vrai zero, et non\n  un instrument qui ne regarde rien.",
        "l'instrument ne voit pas une dispersion pourtant forte. Son zero\n  ne prouve alors rien.",
    );

    // -- TEMOIN 3.3 : la BONNE VALEUR --
    dire!(j, "");
    dire!(j, "-- TEMOIN 3.3 : l'ecart lu a-t-il la BONNE VALEUR ? --");
    dire!(j, "On compare l'ecart lu au long du bord droit a celui que le modele");
    dire!(j, "du nuanceur pose — deux decalages de Snell pris a deux indices");
    dire!(j, "separes par la dispersion — recalcule ici de zero.");
    dire!(j, "");
    dire!(j, "  direction        d        lu       attendu     ecart");
    let mut t33 = true;
    let mut vus = 0;
    for d in dirs.iter().filter(|d| d.dispersion > 0.0 && d.largeur > 0.0) {
        let (_, cx) = v.champ_de(d, 1.0, 0.0);
        let y = ih() / 2;
        for k in 0..4u32 {
            let dd = -0.5 - k as f64;
            let t = cx[(y * il() + k) as usize];
            let lu = t[0] - t[2];
            let at = decalage_attendu(dd, d, d.indice as f64 - d.dispersion as f64 * 0.5)
                - decalage_attendu(dd, d, d.indice as f64 + d.dispersion as f64 * 0.5);
            dire!(
                j,
                "  {:<16} {:>5.1} {:>9.4} px {:>9.4} px {:>8.4} px",
                d.nom,
                dd,
                lu,
                at,
                lu - at
            );
            vus += 1;
            if (lu - at).abs() > 0.05 {
                t33 = false;
            }
        }
    }
    let t33 = verdict(
        j,
        t33 && vus > 0,
        "l'ecart lu reproduit celui que le modele pose, a moins de\n  0,05 px. Ce que l'instrument rapporte est donc bien de la\n  dispersion, et a la bonne taille.",
        "l'ecart lu ne reproduit pas celui du modele. L'instrument ne\n  tranche rien tant qu'on ne sait pas lequel des deux a tort.",
    );

    let valide = t31 && t32 && t33;
    dire!(j, "");
    if !valide {
        dire!(j, "-- PAS DE MESURE : un temoin a echoue, l'instrument est faux. --");
        return false;
    }

    dire!(j, "-- LA MESURE, sur l'etat actuel du labo --");
    dire!(j, "");
    dire!(j, "  direction        demandee   ecart R-B   ou du bord   frange sat 1,00   sat 1,35");
    dire!(j, "  --------------------------------------------------------------------------------------");
    let gris = fond_bureau_gris();
    for d in dirs {
        let (_, cx) = v.champ_de(d, 1.0, 0.0);
        let (e, ou) = ecart_canaux(&cx);
        let mut plat = *d;
        plat.saturation = 1.0;
        let (f1, _) = frange(&v.rendre(&gris, &plat), (d.largeur as f64 + 4.0).max(6.0));
        let (f2, ouf) = frange(&v.rendre(&gris, d), (d.largeur as f64 + 4.0).max(6.0));
        dire!(
            j,
            "  {:<16} {:>8.3} {:>11.3} px {:>10.1} px {:>13.0} niv {:>8.0} niv (a {ouf:.1} px)",
            d.nom,
            d.dispersion,
            e,
            ou,
            f1,
            f2
        );
    }
    dire!(j, "");
    dire!(j, "Le fond est ACHROMATIQUE : la frange comptee ne peut venir que de");
    dire!(j, "la dispersion. La colonne a saturation 1,35 est celle de");
    dire!(j, "production — la saturation AMPLIFIE l'ecart entre canaux, et c'est");
    dire!(j, "ce que l'oeil recoit vraiment.");
    true
}

// ===========================================================================
// INSTRUMENT 4 — LA MATIERE, ET NON L'OPACITE
// ===========================================================================

/// Le niveau moyen de la matiere, en OCTETS du canal vert, la ou l'encre se
/// pose. C'est le nombre qui porte la PORTE, et c'est celui de VERRE-TAILLE.md.
fn niveau_coeur(px: &[u8]) -> f64 {
    let mut s = 0.0;
    let mut n = 0.0;
    for y in 0..ih() {
        for x in 0..il() {
            if distance(x as f32, y as f32) > -ENCRE_LA_PLUS_PROCHE {
                continue;
            }
            s += px[((y * il() + x) * 4 + 1) as usize] as f64;
            n += 1.0;
        }
    }
    s / n
}

/// Les grandeurs qui, ENSEMBLE, disent matiere ou opacite.
struct Matiere {
    hf_bord_clair: f64,
    hf_coeur_clair: f64,
    hf_bord_sombre: f64,
    hf_coeur_sombre: f64,
    clair: f64,
    sombre: f64,
}

impl Matiere {
    fn porte(&self) -> f64 {
        self.clair - self.sombre
    }
    fn part(&self) -> f64 {
        self.porte() / 189.0 * 100.0
    }
}

fn matiere_de(v: &Verre, d: &Direction) -> Matiere {
    let bc = v.rendre(&fond_bureau(), d);
    let bs = v.rendre(&fond_bureau_sombre(), d);
    Matiere {
        hf_bord_clair: hf_bande(&bc, il(), ih(), 1, 9),
        hf_coeur_clair: hf_bande(&bc, il(), ih(), il() / 2 - 30, il() / 2 + 30),
        hf_bord_sombre: hf_bande(&bs, il(), ih(), 1, 9),
        hf_coeur_sombre: hf_bande(&bs, il(), ih(), il() / 2 - 30, il() / 2 + 30),
        clair: niveau_coeur(&v.rendre(&uni(226), d)),
        sombre: niveau_coeur(&v.rendre(&uni(37), d)),
    }
}

fn instrument_4(v: &Verre, j: &mut Journal, dirs: &[Direction]) -> bool {
    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "INSTRUMENT 4 — LA MATIERE, ET NON L'OPACITE");
    dire!(j, "===========================================================");
    dire!(j, "Le critere de l'orfevre, durci : « un motif qui disparait n'est");
    dire!(j, "pas une preuve de verre, c'est une preuve d'opacite ». Du verre");
    dire!(j, "efface la haute frequence ET porte la luminance d'ensemble.");
    dire!(j, "");
    dire!(j, "Une ligne qui n'afficherait qu'un des deux MENT. Les deux");
    dire!(j, "grandeurs sont donc sur la meme ligne, et sur les DEUX fonds :");
    dire!(j, "  — hf : l'energie de haute frequence, sur fond raye clair puis");
    dire!(j, "    sombre, au bord (1-8 px) et au coeur ;");
    dire!(j, "  — porte : l'ecart de niveau de l'ilot entre fond uni 226 et");
    dire!(j, "    fond uni 37, rapporte aux 189 niveaux qui les separent.");

    // -- TEMOIN 4.1 : l'opacite doit etre diagnostiquee comme telle --
    dire!(j, "");
    dire!(j, "-- TEMOIN 4.1 : une matiere OPAQUE doit echouer, et sur le BON critere --");
    dire!(j, "On demande une teinte de 1,00 : le verre devient du noir. Il doit");
    dire!(j, "REUSSIR le premier critere — plus aucune haute frequence — et");
    dire!(j, "ECHOUER au second — porte nulle. Un instrument qui ne les");
    dire!(j, "distinguerait pas appellerait cela du verre.");
    let mut opaque = TEMOIN;
    opaque.teinte = 1.0;
    let mo = matiere_de(v, &opaque);
    dire!(
        j,
        "  opaque : hf bord {:.2} / {:.2}   hf coeur {:.2} / {:.2}   porte {:.1} niveaux ({:.1} %)",
        mo.hf_bord_clair,
        mo.hf_bord_sombre,
        mo.hf_coeur_clair,
        mo.hf_coeur_sombre,
        mo.porte(),
        mo.part()
    );
    let t41 = verdict(
        j,
        mo.hf_coeur_clair < 0.05 && mo.porte().abs() < 0.5,
        "hf nulle ET porte nulle. L'opacite est diagnostiquee comme\n  opacite, et non comme du verre — c'est-a-dire que les deux\n  criteres ne se confondent pas.",
        "l'instrument ne separe pas les deux criteres. Il ne peut donc\n  pas distinguer du verre d'une dalle noire.",
    );

    // -- TEMOIN 4.2 : le fond NU doit tout porter --
    dire!(j, "");
    dire!(j, "-- TEMOIN 4.2 : un fond NU doit porter TOUTE sa structure et TOUTE sa porte --");
    dire!(j, "C'est le temoin qui compte le plus. Sans lui, une porte faible ne");
    dire!(j, "voudrait rien dire : elle pourrait venir de l'instrument. On");
    dire!(j, "eteint donc tout — teinte nulle, flou reduit a 0,3, aucun biseau —");
    dire!(j, "et l'ilot doit alors rendre le fond tel quel : porte 189, part 100 %.");
    let nu = Direction {
        nom: "fond nu",
        modele: Modele::Esquisse,
        refraction: 0.0,
        fresnel: 0.0,
        beer: 0.0,
        prises: 8.0,
        profil: 0.0,
        sigma: 0.3,
        teinte: 0.0,
        saturation: 1.0,
        largeur: 0.0,
        indice: 1.5,
        dispersion: 0.0,
        epaisseur: 0.0,
        galbe: 0.0,
        lisere: 0.0,
        clarte: 0.0,
        integre: 1.0,
        vibrance_relative: false,
        nette_luminance: false,
        conge_relatif: false,
        nette_sigma: 0.0,
        lampe: 0.0,
        ciel: 0.0,
    };
    let mn = matiere_de(v, &nu);
    dire!(
        j,
        "  fond nu : hf bord {:.2} / {:.2}   hf coeur {:.2} / {:.2}   porte {:.1} niveaux ({:.1} %)",
        mn.hf_bord_clair,
        mn.hf_bord_sombre,
        mn.hf_coeur_clair,
        mn.hf_coeur_sombre,
        mn.porte(),
        mn.part()
    );
    let t42 = verdict(
        j,
        mn.part() > 99.0 && mn.hf_coeur_clair > 10.0,
        "l'instrument enregistre une porte de 100 % et toute la\n  structure quand elles sont la. Une porte faible mesuree\n  ailleurs vient donc de la matiere, et non de l'instrument.",
        "l'instrument n'enregistre pas une transmission totale la ou\n  elle est totale. Aucune porte qu'il rapporte ne vaut.",
    );

    // -- TEMOIN 4.3 : la ligne publiee --
    dire!(j, "");
    dire!(j, "-- TEMOIN 4.3 : la ligne de VERRE-TAILLE.md §4 doit revenir --");
    let mt = matiere_de(v, &TEMOIN);
    dire!(
        j,
        "  verre / clair {:.1}   verre / sombre {:.1}   porte {:.1}   part {:.1} %",
        mt.clair,
        mt.sombre,
        mt.porte(),
        mt.part()
    );
    dire!(j, "  publie :      45,0                  7,0          38,0        20,1 %");
    let t43 = verdict(
        j,
        (mt.clair - 45.0).abs() < 0.5 && (mt.sombre - 7.0).abs() < 0.5,
        "l'instrument retrouve une ligne publiee qu'il n'a pas calculee.",
        "l'instrument ne retrouve pas la ligne publiee. L'une des deux\n  est fausse, et il faut dire laquelle avant d'aller plus loin.",
    );

    let valide = t41 && t42 && t43;
    dire!(j, "");
    if !valide {
        dire!(j, "-- PAS DE MESURE : un temoin a echoue, l'instrument est faux. --");
        return false;
    }

    dire!(j, "-- LA MESURE, sur l'etat actuel du labo --");
    dire!(j, "");
    dire!(j, "                       FOND CLAIR          FOND SOMBRE      |        LA PORTE");
    dire!(j, "  direction         hf bord  hf coeur | hf bord  hf coeur |  clair  sombre  porte   part");
    dire!(j, "  ------------------------------------------------------------------------------------------");
    for d in dirs {
        let m = matiere_de(v, d);
        dire!(
            j,
            "  {:<17} {:>7.2} {:>9.2} | {:>7.2} {:>9.2} | {:>6.1} {:>7.1} {:>6.1} {:>6.1} %",
            d.nom,
            m.hf_bord_clair,
            m.hf_coeur_clair,
            m.hf_bord_sombre,
            m.hf_coeur_sombre,
            m.clair,
            m.sombre,
            m.porte(),
            m.part()
        );
    }
    dire!(j, "");
    dire!(j, "Lecture, et les trois colonnes ne se lisent JAMAIS separement.");
    dire!(j, "La porte est la preuve de MATIERE : elle dit combien du fond");
    dire!(j, "traverse. La hf du coeur est la preuve de DIFFUSION : elle dit");
    dire!(j, "que le motif fin, lui, ne traverse pas. La hf du bord dit si le");
    dire!(j, "biseau REMET de la structure la ou le flou en avait enleve.");
    true
}

// ===========================================================================
// INSTRUMENT 5 — LA LISIBILITE, QUI NE SE NEGOCIE PAS
// ===========================================================================

const ENCRES: [(&str, (u8, u8, u8)); 3] = [
    ("#F2EFE9", (0xF2, 0xEF, 0xE9)),
    ("#8E8B86", (0x8E, 0x8B, 0x86)),
    ("#5E5B57", (0x5E, 0x5B, 0x57)),
];

fn instrument_5(v: &Verre, j: &mut Journal, dirs: &[Direction]) -> bool {
    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "INSTRUMENT 5 — LA LISIBILITE, QUI NE SE NEGOCIE PAS");
    dire!(j, "===========================================================");
    dire!(j, "Ce qu'il mesure : les trois contrastes WCAG de la palette de");
    dire!(j, "DESIGN.md §2 contre la matiere, sur le PIRE fond et au PIRE");
    dire!(j, "pixel de la zone d'encre — et non a la moyenne du centre.");
    dire!(j, "");
    dire!(j, "Le durcissement est la. Une moyenne cache le pixel qui fait");
    dire!(j, "disparaitre la lettre. La zone d'encre est tout ce qui est a plus");
    dire!(j, "de {ENCRE_LA_PLUS_PROCHE:.2} px du bord (OPTIQUE.md §9), et le contraste s'y prend");
    dire!(j, "sur le pixel le plus CLAIR, qui est le pire pour une encre sombre.");

    // -- TEMOIN 5.1 : la ligne du verre --
    dire!(j, "");
    dire!(j, "-- TEMOIN 5.1 : la ligne de VERRE-TAILLE.md §4 doit revenir --");
    let px_t = v.rendre(&uni(226), &TEMOIN);
    let lm = luminance_coeur(&px_t);
    let c51: Vec<f64> = ENCRES
        .iter()
        .map(|(_, e)| contraste_l(luminance_relative(e.0, e.1, e.2), lm))
        .collect();
    dire!(j, "  au coeur, en moyenne : {:.2} / {:.2} / {:.2}", c51[0], c51[1], c51[2]);
    dire!(j, "  publie               : 12,00 / 4,06 / 2,04");
    let ecarts51 = [
        (c51[0] - 12.00).abs(),
        (c51[1] - 4.06).abs(),
        (c51[2] - 2.04).abs(),
    ];
    let t51 = verdict(
        j,
        ecarts51.iter().all(|e| *e < 0.01),
        "l'instrument retrouve une ligne publiee qu'il n'a pas calculee.",
        "l'instrument ne retrouve pas la ligne publiee.",
    );

    // -- TEMOIN 5.2 : une SECONDE reponse connue, et differente --
    dire!(j, "");
    dire!(j, "-- TEMOIN 5.2 : une SECONDE reponse connue, tres eloignee de la premiere --");
    dire!(j, "Retrouver un seul nombre publie peut tenir du hasard. On demande");
    dire!(j, "donc l'ilot NOIR OPAQUE, dont VERRE.md §8 donne 18,30 / 6,19 / 3,11");
    dire!(j, "— trois valeurs qui n'ont rien a voir avec les trois premieres.");
    let mut opaque = TEMOIN;
    opaque.teinte = 1.0;
    let px_o = v.rendre(&uni(226), &opaque);
    let lo = luminance_coeur(&px_o);
    let c52: Vec<f64> = ENCRES
        .iter()
        .map(|(_, e)| contraste_l(luminance_relative(e.0, e.1, e.2), lo))
        .collect();
    dire!(j, "  ilot noir opaque : {:.2} / {:.2} / {:.2}", c52[0], c52[1], c52[2]);
    dire!(j, "  publie           : 18,30 / 6,19 / 3,11");
    let ecarts52 = [
        (c52[0] - 18.30).abs(),
        (c52[1] - 6.19).abs(),
        (c52[2] - 3.11).abs(),
    ];
    let t52 = verdict(
        j,
        ecarts52.iter().all(|e| *e < 0.01),
        "une seconde reponse connue, tres loin de la premiere, revient\n  elle aussi. L'echelle de l'instrument est juste sur toute sa\n  plage, et non calee sur un seul point.",
        "la seconde reponse connue ne revient pas. L'instrument est\n  juste en un point et faux ailleurs.",
    );

    let valide = t51 && t52;
    dire!(j, "");
    if !valide {
        dire!(j, "-- PAS DE MESURE : un temoin a echoue, l'instrument est faux. --");
        return false;
    }

    dire!(j, "-- LA MESURE, au pire pixel et sur le pire fond --");
    dire!(j, "");
    let fonds: [(&str, Vec<u8>); 6] = [
        ("uni 0", uni(0)),
        ("uni 37", uni(37)),
        ("uni 128", uni(128)),
        ("uni 226", uni(226)),
        ("uni 255", uni(255)),
        ("bureau raye", fond_bureau()),
    ];
    dire!(j, "  direction         pire fond     pire niveau   #F2EFE9   #8E8B86   #5E5B57");
    dire!(j, "  --------------------------------------------------------------------------");
    for d in dirs {
        let mut pire_l = 0.0f64;
        let mut pire_nom = "";
        for (nom, f) in &fonds {
            let px = v.rendre(f, d);
            let l = luminance_pire(&px);
            if l > pire_l {
                pire_l = l;
                pire_nom = nom;
            }
        }
        let c: Vec<f64> = ENCRES
            .iter()
            .map(|(_, e)| contraste_l(luminance_relative(e.0, e.1, e.2), pire_l))
            .collect();
        let niveau = octet(pire_l);
        dire!(
            j,
            "  {:<17} {:<13} {:>11} {:>9.2} {:>9.2} {:>9.2}",
            d.nom,
            pire_nom,
            niveau,
            c[0],
            c[1],
            c[2]
        );
    }
    dire!(j, "");
    dire!(j, "Et le meme, au COEUR moyen — pour dire de combien le pire pixel");
    dire!(j, "coute par rapport au chiffre publie, qui est une moyenne :");
    dire!(j, "");
    dire!(j, "  direction         coeur uni 226   #F2EFE9   ecart au pire pixel");
    dire!(j, "  ----------------------------------------------------------------");
    for d in dirs {
        let px = v.rendre(&uni(226), d);
        let lc = luminance_coeur(&px);
        let lp = luminance_pire(&px);
        let e = ENCRES[0].1;
        let cc = contraste_l(luminance_relative(e.0, e.1, e.2), lc);
        let cp = contraste_l(luminance_relative(e.0, e.1, e.2), lp);
        dire!(
            j,
            "  {:<17} {:>13} {:>9.2} {:>21.2}",
            d.nom,
            octet(lc),
            cc,
            cp - cc
        );
    }
    dire!(j, "");
    dire!(j, "Le plancher de DESIGN.md est 12,00 / 4,06 / 2,04 (VERRE-TAILLE.md");
    dire!(j, "§4). Toute direction qui descend sous cette ligne doit dire de");
    dire!(j, "combien, et pourquoi cela vaut le prix.");
    true
}

// ===========================================================================
// INSTRUMENT 6 — LE COUT, COMPOSITION ISOLEE DES FLOUS
// ===========================================================================

/// Une droite des moindres carres `y = a + b x`, et son plus grand residu.
fn ajuster(pts: &[(f64, f64)]) -> (f64, f64, f64) {
    let n = pts.len() as f64;
    let sx: f64 = pts.iter().map(|p| p.0).sum();
    let sy: f64 = pts.iter().map(|p| p.1).sum();
    let sxx: f64 = pts.iter().map(|p| p.0 * p.0).sum();
    let sxy: f64 = pts.iter().map(|p| p.0 * p.1).sum();
    let det = n * sxx - sx * sx;
    let b = (n * sxy - sx * sy) / det;
    let a = (sy - b * sx) / n;
    let r = pts
        .iter()
        .map(|p| (p.1 - (a + b * p.0)).abs())
        .fold(0.0f64, f64::max);
    (a, b, r)
}

const TOURS: usize = 60;

/// Combien de fois le temoin 6.1 balaie les quatre tailles. Un nombre PAIR :
/// les balayages se coupent en deux moities, et l'ecart entre les deux medianes
/// d'une meme taille donne le plancher de bruit — la reproductibilite de la
/// grandeur meme qui entre dans la droite, mesuree sur le meme intervalle de
/// temps qu'elle.
const BALAYAGES: usize = 10;

fn instrument_6(v: &Verre, j: &mut Journal, dirs: &[Direction], horodatage: bool) -> bool {
    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "INSTRUMENT 6 — LE COUT, COMPOSITION ISOLEE DES FLOUS");
    dire!(j, "===========================================================");
    dire!(j, "Ce qu'il mesure : le temps GPU de chaque passe, pris par");
    dire!(j, "l'horodateur du GPU lui-meme et non par la montre du processeur.");
    dire!(j, "La composition est ainsi ISOLEE des deux flous.");
    dire!(j, "");
    dire!(j, "C'est le premier point que ce banc corrige. `verre.rs` et");
    dire!(j, "`matiere.rs` chronometrent autour de `submit` et `poll` : ils");
    dire!(j, "mesurent un aller-retour de processeur, qui contient l'attente et");
    dire!(j, "la soumission, et qui ne peut par construction separer les trois");
    dire!(j, "passes. Ici chaque passe porte ses deux horodatages.");

    if !horodatage {
        dire!(j, "");
        dire!(j, "  L'ADAPTATEUR NE PORTE PAS L'HORODATAGE GPU. Cet instrument ne");
        dire!(j, "  peut pas etre construit sur cette machine, et rien de ce qu'il");
        dire!(j, "  dirait ne vaudrait. Le trou est nomme plutot que masque.");
        return false;
    }

    let fond = uni(226);

    // -- TEMOIN 6.1 : le temps doit suivre le NOMBRE DE PIXELS --
    dire!(j, "");
    dire!(j, "-- TEMOIN 6.1 : le temps doit suivre le NOMBRE DE PIXELS --");
    dire!(j, "La composition est un nuanceur de fragment sur un triangle plein");
    dire!(j, "ecran. Son temps doit s'ecrire `fixe + pente x aire` : une passe a");
    dire!(j, "un cout d'entree, puis un cout par pixel. On ne demande donc PAS");
    dire!(j, "une proportionnalite stricte — ce serait nier le cout d'entree —");
    dire!(j, "mais que les quatre tailles tombent sur UNE droite.");
    dire!(j, "");
    dire!(j, "CE QUE CE TOUR CORRIGE, et c'est double.");
    dire!(j, "");
    dire!(j, "1. Le seuil valait `0,1 x la plus grande des quatre mesures` — donc");
    dire!(j, "   RELATIF au travail. Mais le bruit de l'horodateur, lui, ne");
    dire!(j, "   retrecit pas quand le GPU accelere : sur un GPU aux horloges");
    dire!(j, "   hautes les quatre temps chutent d'un facteur quatre, le seuil les");
    dire!(j, "   suit, et le temoin tombe sans que rien n'ait change dans ce qu'il");
    dire!(j, "   juge. Il se declarait faux environ un lancement sur deux, et le");
    dire!(j, "   banc supprimait alors TOUTE la table de cout.");
    dire!(j, "");
    dire!(j, "2. Plus profond, et c'est la vraie cause : la droite etait ajustee");
    dire!(j, "   sur des MILLISECONDES. Or l'etat d'horloge agit de facon");
    dire!(j, "   MULTIPLICATIVE sur un temps GPU — le meme travail vaut de 0,0041");
    dire!(j, "   a 0,0369 ms selon le lancement. Les quatre tailles se mesurent");
    dire!(j, "   l'une apres l'autre ; l'horloge bouge entre elles ; l'ecart a la");
    dire!(j, "   droite contient donc cette derive, et aucune chauffe ne la");
    dire!(j, "   supprime entierement. Un plancher de bruit mesure sur des");
    dire!(j, "   millisecondes reste de l'ordre de l'effet cherche : essaye sur");
    dire!(j, "   cette machine, l'instrument refusait encore trois lancements sur");
    dire!(j, "   cinq.");
    dire!(j, "");
    dire!(j, "On ajuste donc la droite sur des RAPPORTS et non sur des durees. Dans");
    dire!(j, "chaque balayage, les quatre temps sont divises par celui de la plus");
    dire!(j, "grande taille, MESURE DANS LE MEME BALAYAGE. Un facteur d'horloge");
    dire!(j, "commun s'y annule exactement, et la structure affine survit a la");
    dire!(j, "division : (a + b x) / (a + b X) reste affine en x. C'est la meme");
    dire!(j, "lecon que la mesure appariee plus bas, appliquee au temoin.");
    dire!(j, "");
    dire!(j, "Le plancher de bruit se mesure alors sur ces rapports : les {BALAYAGES}");
    dire!(j, "balayages se coupent en DEUX MOITIES, et l'ecart entre les deux");
    dire!(j, "medianes d'une meme taille est la REPRODUCTIBILITE de la grandeur");
    dire!(j, "meme qui entre dans la droite, sur le meme intervalle de temps.");
    dire!(j, "");
    let tailles: [(u32, u32); 4] = [(160, 74), (320, 148), (480, 222), (640, 296)];
    // CHAUFFE JUSQU'A STABILISATION, et elle est jetee. Un GPU qui sort du repos
    // monte ses horloges PENDANT le balayage : l'etendue d'une meme taille
    // explose, le plancher devient plus grand que ce que la droite doit
    // resoudre, et l'instrument se declare incapable — ce qui est vrai, mais
    // evitable. Mesure sur cette machine, sans chauffe : le lancement qui suit
    // une periode d'inactivite rend un plancher de 0,0159 ms pour une portee de
    // 0,0430 ms, quand un lancement enchaine rend 0,0005 ms pour 0,0190.
    //
    // On ne chauffe donc pas un nombre de tours pose : on chauffe JUSQU'A ce que
    // deux mesures consecutives de la meme taille tombent a moins d'un tic
    // d'horodateur l'une de l'autre. Le critere est l'horloge elle-meme.
    let tic = v.periode_ns as f64 * 1e-6;
    let mut precedent = f64::MAX;
    let mut chauffe = 0usize;
    for _ in 0..40 {
        let (_, c) = v.chaine(&fond, &DALLE, TOURS, 640, 296);
        chauffe += 1;
        if (c.composer - precedent).abs() <= tic {
            break;
        }
        precedent = c.composer;
    }
    dire!(j, "  chauffe : {chauffe} tours jusqu'a deux mesures consecutives egales");
    dire!(j, "");
    dire!(j, "  cible          pixels   composition   rapport   1re moitie  2e moitie");
    dire!(j, "  -----------------------------------------------------------------------");
    // Les quatre temps de CHAQUE balayage, gardes ensemble : c'est ce qui permet
    // de diviser par un temps mesure dans le meme balayage.
    let mut balayages: Vec<[f64; 4]> = Vec::new();
    for _ in 0..BALAYAGES {
        let mut ligne = [0.0f64; 4];
        for (k, (cl, ch)) in tailles.iter().enumerate() {
            let (_, c) = v.chaine(&fond, &DALLE, TOURS, *cl, *ch);
            ligne[k] = c.composer;
        }
        balayages.push(ligne);
    }
    let mediane = |x: &[f64]| {
        let mut y = x.to_vec();
        y.sort_by(|a, b| a.partial_cmp(b).unwrap());
        y[y.len() / 2]
    };
    let mut rapports: [Vec<f64>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut absolus: [Vec<f64>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for l in &balayages {
        let reference = l[3].max(1e-9);
        for k in 0..4 {
            rapports[k].push(l[k] / reference);
            absolus[k].push(l[k]);
        }
    }
    let mut pts = Vec::new();
    let mut plancher = 0.0f64;
    for (k, (cl, ch)) in tailles.iter().enumerate() {
        let n = (cl * ch) as f64;
        let r = mediane(&rapports[k]);
        let r1 = mediane(&rapports[k][..BALAYAGES / 2]);
        let r2 = mediane(&rapports[k][BALAYAGES / 2..]);
        dire!(
            j,
            "  {cl:>4} x {ch:>4} {:>10.0} {:>11.4} ms {r:>9.4} {r1:>11.4} {r2:>10.4}",
            n,
            mediane(&absolus[k])
        );
        pts.push((n, r));
        plancher = plancher.max((r1 - r2).abs());
    }

    // LE TIC REEL DE L'HORODATEUR, mesure et non demande au pilote. Le pilote
    // annonce une periode de 1,0 ns ; les valeurs rendues, elles, tombent toutes
    // sur des multiples d'environ 0,00051 ms. La granularite annoncee et la
    // granularite effective different d'un facteur cinq cents, et c'est la
    // seconde qui fixe le bruit. On la lit dans les mesures : le plus petit
    // ecart non nul entre deux valeurs distinctes.
    let mut toutes: Vec<f64> = absolus.iter().flatten().copied().collect();
    toutes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut tic_mesure = f64::MAX;
    for w in toutes.windows(2) {
        let d = w[1] - w[0];
        if d > 1e-9 {
            tic_mesure = tic_mesure.min(d);
        }
    }
    // TEMOIN DU TIC : si c'est bien une granularite, TOUTES les valeurs sont des
    // multiples entiers du tic. On mesure le plus gros ecart au multiple le plus
    // proche ; il doit rester tres petit devant le tic lui-meme.
    let ecart_multiple = toutes
        .iter()
        .map(|x| {
            let q = (x / tic_mesure).round();
            (x - q * tic_mesure).abs()
        })
        .fold(0.0f64, f64::max);
    let tic_sain = tic_mesure.is_finite() && ecart_multiple < 0.1 * tic_mesure;
    dire!(j, "");
    dire!(j, "  tic annonce par le pilote : {:.2} ns", v.periode_ns);
    dire!(j, "  tic MESURE (plus petit ecart non nul) : {tic_mesure:.5} ms");
    dire!(
        j,
        "  temoin du tic : toute valeur est a {ecart_multiple:.6} ms du multiple le plus proche"
    );
    if !tic_sain {
        dire!(j, "  LE TIC MESURE N'EST PAS UNE GRANULARITE. Le plancher qui en");
        dire!(j, "  decoule ne vaut rien.");
    }
    // Le plancher ne peut pas etre plus fin que la quantification du rapport
    // lui-meme : un tic sur le temps de reference. C'est une borne DERIVEE de la
    // mesure precedente, pas un chiffre pose.
    let plancher_quant = tic_mesure / mediane(&absolus[3]).max(1e-9);
    dire!(j, "  quantification du rapport : {plancher_quant:.5}");
    plancher = plancher.max(plancher_quant);

    let (fixe, pente, residu) = ajuster(&pts);
    dire!(j, "");
    dire!(j, "  La colonne « composition » est en millisecondes et ne vaut que");
    dire!(j, "  comme ordre de grandeur : elle depend de l'etat d'horloge. C'est la");
    dire!(j, "  colonne « rapport », sans unite, qui porte le temoin.");
    dire!(j, "");
    dire!(
        j,
        "  droite ajustee sur les rapports : {fixe:.4} de cout d'entree + {:.4} par Mpx",
        pente * 1e6
    );
    dire!(j, "  plus grand residu a la droite : {residu:.5}");
    dire!(j, "");
    dire!(j, "  Le cout d'entree EXPLIQUE ce qu'un rapport de proportionnalite");
    dire!(
        j,
        "  aurait appele une anomalie : a 160 x 74 il pese {:.0} % du temps",
        fixe / pts[0].1 * 100.0
    );
    dire!(j, "  de la passe. Ce n'est pas l'horodateur qui derive, c'est le modele");
    dire!(j, "  a une seule pente qui est faux.");
    dire!(j, "");
    // Le SEUIL vaut un plancher, et ce n'est pas un facteur pose : le plancher
    // est deja l'ecart entre deux determinations INDEPENDANTES du point trace,
    // et un residu est l'ecart d'un point trace a une droite portee par des
    // points de meme nature. La grandeur comparee est donc la meme.
    let seuil = plancher;
    // Et le seuil a SON temoin, qui est le seul qui compte : il doit pouvoir
    // REFUSER quelque chose. Le modele faux que ce temoin existe pour ecarter
    // est la proportionnalite stricte `y = c x`, sans cout d'entree — c'est lui
    // que le tableau ci-dessus aurait accuse l'horodateur de contredire. Si le
    // bruit est assez gros pour avaler l'ecart a CE modele-la, un succes ne
    // prouve plus rien, et l'instrument doit le dire au lieu de passer.
    let sxy: f64 = pts.iter().map(|p| p.0 * p.1).sum();
    let sxx: f64 = pts.iter().map(|p| p.0 * p.0).sum();
    let c_prop = sxy / sxx;
    let residu_prop = pts
        .iter()
        .map(|p| (p.1 - c_prop * p.0).abs())
        .fold(0.0f64, f64::max);
    dire!(j, "  plancher de bruit mesure sur les rapports : {plancher:.5}");
    dire!(j, "  seuil retenu                              : {seuil:.5}");
    dire!(j, "  residu du modele SANS cout d'entree (y = c x) : {residu_prop:.5}");
    let discriminant = tic_sain && plancher > 0.0 && seuil < residu_prop;
    if !discriminant {
        dire!(j, "");
        dire!(j, "  LE SEUIL NE REFUSE PLUS RIEN. Il est plus large que l'ecart au");
        dire!(j, "  modele proportionnel, donc les deux modeles passeraient tous");
        dire!(j, "  les deux : le bruit de l'horodateur a mange la question. Rien");
        dire!(j, "  ne sera certifie sur ce lancement.");
    }
    let t61 = verdict(
        j,
        discriminant && pente > 0.0 && residu < seuil,
        &format!(
            "les quatre tailles tombent sur une droite a {residu:.5} pres (sans\n  unite : ce sont des rapports), sous le seuil de {seuil:.5} — et ce seuil\n  refuserait le modele sans cout d'entree, qui s'en ecarte de {residu_prop:.5}.\n  La pente est positive. L'horodateur mesure donc bien du travail\n  proportionnel a l'aire."
        ),
        &format!(
            "residu de {residu:.5}, seuil de {seuil:.5}, ecart du modele\n  proportionnel {residu_prop:.5}. L'instrument ne tranche pas."
        ),
    );

    // -- TEMOIN 6.2 : l'ATTRIBUTION --
    dire!(j, "");
    dire!(j, "-- TEMOIN 6.2 : le temps est-il attribue a la BONNE passe ? --");
    dire!(j, "C'est le temoin qui compte, et aucune duree totale ne peut le");
    dire!(j, "remplacer. On ne touche qu'au FLOU — sigma, donc le nombre de");
    dire!(j, "taps. Le temps des flous doit alors suivre les taps, et celui de");
    dire!(j, "la composition ne doit pas bouger d'un cheveu, puisque la");
    dire!(j, "composition ne lit pas sigma.");
    dire!(j, "");
    dire!(j, "  sigma   taps   flou h    flou v    COMPOSITION");
    dire!(j, "  ------------------------------------------------");
    let mut comps = Vec::new();
    let mut flous = Vec::new();
    for s in [6.0f32, 12.0, 18.0, 36.0] {
        let mut d = DALLE;
        d.sigma = s;
        let (_, c) = v.chaine(&fond, &d, TOURS, il(), ih());
        let taps = (s * 3.0).ceil() as f64 * 2.0 + 1.0;
        dire!(
            j,
            "  {s:>5.0} {taps:>6.0} {:>9.4} {:>9.4} {:>13.4} ms",
            c.flou_h,
            c.flou_v,
            c.composer
        );
        comps.push(c.composer);
        flous.push((taps, c.flou_h));
    }
    // La derive ne se lit PAS sur ce tableau, et c'est la correction de ce
    // chantier. Les quatre lignes y sont mesurees l'une apres l'autre, et
    // l'horloge du GPU bouge entre elles : sur cette machine, le meme travail
    // mesure a deux lancements donne de 0,0041 a 0,0369 ms, un facteur NEUF.
    // Un temoin qui compare deux mesures prises a des moments differents
    // mesure donc surtout l'etat d'horloge, et il passe ou echoue au hasard.
    // On reprend les deux sigmas extremes EN ALTERNANCE, ABBA, et l'on compare
    // les medianes appariees : la derive commune se retranche d'elle-meme.
    let mut c_bas = Vec::new();
    let mut c_haut = Vec::new();
    for _ in 0..12 {
        let mut a = DALLE;
        a.sigma = 6.0;
        let mut b = DALLE;
        b.sigma = 36.0;
        let (_, x1) = v.chaine(&fond, &a, 4, il(), ih());
        let (_, y1) = v.chaine(&fond, &b, 4, il(), ih());
        let (_, y2) = v.chaine(&fond, &b, 4, il(), ih());
        let (_, x2) = v.chaine(&fond, &a, 4, il(), ih());
        c_bas.push(0.5 * (x1.composer + x2.composer));
        c_haut.push(0.5 * (y1.composer + y2.composer));
    }
    let med = |mut x: Vec<f64>| {
        x.sort_by(|p, q| p.partial_cmp(q).unwrap());
        x[x.len() / 2]
    };
    let (cb, ch) = (med(c_bas), med(c_haut));
    dire!(j, "");
    dire!(j, "  La MEME comparaison, mesuree en alternance ABBA :");
    dire!(j, "    composition a sigma 6  : {cb:.5} ms");
    dire!(j, "    composition a sigma 36 : {ch:.5} ms");
    let derive = (ch - cb).abs();
    let mediane_comp = 0.5 * (cb + ch);
    let rapport = flous[3].1 / flous[0].1;
    let rapport_taps = flous[3].0 / flous[0].0;
    dire!(j, "");
    dire!(j, "  derive appariee de la composition : {derive:.5} ms");
    dire!(j, "  le flou croit d'un facteur {rapport:.2} pour {rapport_taps:.2} fois plus de taps");
    // Le seuil est RELATIF a la composition, et il a fallu une mesure pour le
    // comprendre. Ecrit en absolu — « moins de 0,002 ms » — il tenait quand la
    // composition valait 0,008 ms (12 % de derive acceptee) et tombait quand
    // elle valait 0,031 ms (6,6 % de derive refusee). Le meme instrument
    // declarait donc l'attribution fiable ou non selon l'etat d'horloge du GPU,
    // ce qui n'a rien a voir avec la question posee. Ce que le temoin demande
    // est que la composition ne bouge PAS PAR RAPPORT A ELLE-MEME pendant que
    // le flou bouge d'un facteur cinq : le seuil est donc une part, avec un
    // plancher absolu pour le cas ou la composition serait tres rapide.
    let part = derive / mediane_comp.max(1e-9);
    dire!(j, "  soit {:.1} % de la composition elle-meme", part * 100.0);
    let t62 = verdict(
        j,
        (part < 0.15 || derive < 0.001) && rapport > 2.0,
        &format!(
            "le flou suit ses taps et la composition ne bouge pas de plus\n  de {derive:.5} ms. Les deux horodatages mesurent donc bien deux\n  travaux distincts, et le temps n'est pas mis sur la mauvaise passe."
        ),
        &format!(
            "la composition derive de {derive:.5} ms quand seul le flou change.\n  L'attribution par passe n'est pas fiable."
        ),
    );

    let valide = t61 && t62;
    dire!(j, "");
    if !valide {
        dire!(j, "-- PAS DE MESURE : un temoin a echoue, l'instrument est faux. --");
        return false;
    }

    dire!(j, "-- LA MESURE, mediane de {TOURS} soumissions apres chauffe --");
    dire!(j, "");
    dire!(j, "  direction         flou h    flou v   COMPOSITION     total   part de 2,00 ms");
    dire!(j, "  ----------------------------------------------------------------------------");
    for d in dirs {
        let (_, c) = v.chaine(&fond, d, TOURS, il(), ih());
        dire!(
            j,
            "  {:<17} {:>7.4} {:>9.4} {:>12.4} ms {:>9.4} {:>13.1} %",
            d.nom,
            c.flou_h,
            c.flou_v,
            c.composer,
            c.total(),
            c.total() / 2.0 * 100.0
        );
    }

    dire!(j, "");
    dire!(j, "Et ce que chaque ingredient de la composition ajoute, SEUL — la");
    dire!(j, "difference se prend sur la passe de composition, les flous etant");
    dire!(j, "identiques d'une ligne a l'autre :");
    dire!(j, "");
    dire!(j, "  ingredient                          composition     ajout");
    dire!(j, "  -----------------------------------------------------------");
    let base = TEMOIN;
    let (_, c_base) = v.chaine(&fond, &base, TOURS, il(), ih());
    dire!(
        j,
        "  la face seule (verre.wgsl)          {:>8.4} ms        —",
        c_base.composer
    );
    let etapes: [(&str, fn(&mut Direction)); 4] = [
        ("+ le biseau et la refraction", |d| {
            d.largeur = 8.0;
            d.epaisseur = 10.0;
            d.galbe = 1.0;
        }),
        ("+ la dispersion", |d| {
            d.largeur = 8.0;
            d.epaisseur = 10.0;
            d.galbe = 1.0;
            d.dispersion = 0.02;
        }),
        ("+ la clarte (couche nette)", |d| {
            d.largeur = 8.0;
            d.epaisseur = 10.0;
            d.galbe = 1.0;
            d.dispersion = 0.02;
            d.clarte = 0.35;
        }),
        ("+ le lisere", |d| {
            d.largeur = 8.0;
            d.epaisseur = 10.0;
            d.galbe = 1.0;
            d.dispersion = 0.02;
            d.clarte = 0.35;
            d.lisere = 1.0;
        }),
    ];
    let mut precedent = c_base.composer;
    for (nom, poser) in etapes {
        let mut d = base;
        poser(&mut d);
        let (_, c) = v.chaine(&fond, &d, TOURS, il(), ih());
        dire!(
            j,
            "  {nom:<35} {:>8.4} ms {:>+9.4} ms",
            c.composer,
            c.composer - precedent
        );
        precedent = c.composer;
    }
    dire!(j, "");
    dire!(
        j,
        "L'horodateur a un plancher : il compte en tics de {:.1} ns, et la",
        v.periode_ns
    );
    dire!(j, "composition entiere ne dure que quelques microsecondes. Un ajout");
    dire!(j, "annonce a 0,0000 ms est donc un ajout SOUS le plancher, et non un");
    dire!(j, "ajout demontre nul.");

    // -- LA MESURE APPARIEE, et pourquoi le tableau ci-dessus ne suffit pas --
    dire!(j, "");
    dire!(j, "-- LA MESURE APPARIEE : ce que le VERRIER coute vraiment --");
    dire!(j, "Le tableau ci-dessus porte un biais que sa lecture ne montre pas.");
    dire!(j, "Les deux flous y sont IDENTIQUES d'une ligne a l'autre — meme");
    dire!(j, "sigma, meme nombre de taps, meme travail — et pourtant leurs temps");
    dire!(j, "s'etalent de 26 % du haut du tableau vers le bas. Le GPU monte ses");
    dire!(j, "horloges pendant le banc : une direction mesuree tard parait moins");
    dire!(j, "chere qu'une direction mesuree tot, et l'ordre du tableau suffit a");
    dire!(j, "inverser un classement. Mesurer deux directions l'une APRES l'autre");
    dire!(j, "ne dit donc rien de leur difference.");
    dire!(j, "");
    dire!(j, "On les mesure donc EN ALTERNANCE, et l'on prend la mediane des");
    dire!(j, "differences appariees.");
    dire!(j, "");
    dire!(j, "CE QUE L'ALTERNANCE RETRANCHE, ET CE QU'ELLE NE RETRANCHE PAS.");
    dire!(j, "Ce banc a longtemps ecrit que « la derive commune s'annule par");
    dire!(j, "construction ». C'est vrai de la derive ADDITIVE, et faux de ce qui");
    dire!(j, "se passe reellement : l'etat d'horloge agit de facon MULTIPLICATIVE");
    dire!(j, "sur du travail de GPU. Si le GPU tourne deux fois plus lentement, A");
    dire!(j, "et B doublent tous les deux — et leur DIFFERENCE double avec eux.");
    dire!(j, "Une difference appariee en millisecondes reste donc proportionnelle");
    dire!(j, "a l'etat d'horloge du lancement, et ne se compare pas d'un lancement");
    dire!(j, "a l'autre.");
    dire!(j, "");
    dire!(j, "Et le temoin A contre A ne peut PAS le voir : un facteur applique a");
    dire!(j, "zero reste zero. Il rend +0,0000 ms quel que soit l'etat d'horloge,");
    dire!(j, "donc il borne le bruit additif et rien d'autre. C'est pourquoi la");
    dire!(j, "colonne qui porte la conclusion est la DERNIERE, sans unite : le");
    dire!(j, "rapport B/A, dans lequel un facteur d'horloge commun s'annule");
    dire!(j, "exactement. C'est elle qu'on publie ; les millisecondes ne valent");
    dire!(j, "que comme ordre de grandeur du lancement.");
    // -- L'ALLONGEMENT DE LA REFERENCE --
    //
    // Le temoin 6.3 juge a deux tics rapportes a la reference, et la reference
    // etait UNE passe de composition a 320 x 148 : de trois a trente-six tics
    // selon l'etat d'horloge, donc une resolution de 6 a 67 %. Aucun rapport
    // publie n'y avait de chiffre significatif apres la virgule.
    dire!(j, "");
    dire!(j, "L'ALLONGEMENT DE LA REFERENCE. Une passe de composition a 320 x 148");
    dire!(j, "ne dure que quelques tics, et deux tics rapportes a si peu font une");
    dire!(j, "resolution de dizaines de pour cent. On allonge donc la reference");
    dire!(j, "sans changer ni la taille, ni la forme, ni le nuanceur : la passe de");
    dire!(j, "composition trace le MEME triangle k fois. Sans profondeur ni");
    dire!(j, "melange, chaque trace refait tout le travail de fragment.");
    dire!(j, "");
    dire!(j, "k double jusqu'a ce que la resolution laisse DEUX DECIMALES au plus");
    dire!(j, "grand rapport qu'OPTIQUE.md publie sur la reference la plus courte,");
    dire!(j, "V0 -> V6. La borne d'arrondi d'un rapport R est R x 2 tics / A ; deux");
    dire!(j, "decimales demandent qu'elle tombe sous 0,01. Le seuil vient donc de");
    dire!(j, "ce rapport, remesure a chaque k, et de la definition d'une decimale.");
    dire!(j, "");
    dire!(j, "      k   V0 (tics)   V6 (tics)   V6/V0   resolution     borne");
    dire!(j, "  -------------------------------------------------------------");
    let tic_utilisable = tic_sain && tic_mesure < f64::MAX;
    let mut traces = 1u32;
    let mut calibrage: Vec<(f64, f64)> = Vec::new();
    loop {
        let (_, ca) = v.chaine_tracee(&fond, &V0, 8, il(), ih(), traces);
        let (_, cb) = v.chaine_tracee(&fond, &V6, 8, il(), ih(), traces);
        let r = cb.composer / ca.composer.max(1e-9);
        let resolution = 2.0 * tic_mesure / ca.composer.max(1e-9);
        let borne = r * resolution;
        dire!(
            j,
            "  {traces:>5} {:>11.0} {:>11.0} {r:>7.2} {:>10.2} % {borne:>9.4}",
            ca.composer / tic_mesure,
            cb.composer / tic_mesure,
            resolution * 100.0
        );
        calibrage.push((traces as f64, ca.composer));
        if !tic_utilisable || borne <= 0.01 || traces >= 4096 {
            break;
        }
        traces *= 2;
    }
    if calibrage.len() > 1 {
        let (k1, a1) = calibrage[0];
        let (kn, an) = *calibrage.last().unwrap();
        let par_trace = (an - a1) / (kn - k1);
        let entree = a1 - par_trace;
        dire!(j, "");
        dire!(j, "  Ce que l'allongement change a la grandeur, et il faut le dire. Une");
        dire!(j, "  passe a un cout d'entree qui ne se paie qu'une fois. Lu sur les deux");
        dire!(
            j,
            "  bouts du calibrage de V0, il vaut {:.1} tics : {:.0} % de la passe a un trace,",
            entree / tic_mesure,
            100.0 * entree / a1
        );
        dire!(j, "  {:.1} % a {traces} traces — et le premier chiffre est lu sur une passe", 100.0 * entree / an);
        dire!(j, "  de quelques tics, donc a un tic pres. Les rapports qui suivent sont");
        dire!(j, "  ceux du TRAVAIL DE FRAGMENT de la composition, et non d'une passe");
        dire!(j, "  unique : le cout d'entree, commun a A et a B, n'y tasse plus l'ecart.");
    }
    if traces >= 4096 {
        dire!(j, "");
        dire!(j, "  LE CALIBRAGE A ATTEINT SON PLAFOND DE 4096 TRACES sans que la borne");
        dire!(j, "  tombe sous 0,01. Les decimales publiees plus bas le disent.");
    }

    let apparier = |a: &Direction, b: &Direction| -> (f64, f64, f64) {
        let mut da = Vec::new();
        let mut db = Vec::new();
        let mut diff = Vec::new();
        // ABBA, et non ABAB : dans un couple AB, si le GPU accelere pendant la
        // paire, B parait toujours moins cher. En bouclant A B B A, la derive
        // du premier ordre se retranche d'elle-meme.
        for _ in 0..16 {
            let (_, a1) = v.chaine_tracee(&fond, a, 4, il(), ih(), traces);
            let (_, b1) = v.chaine_tracee(&fond, b, 4, il(), ih(), traces);
            let (_, b2) = v.chaine_tracee(&fond, b, 4, il(), ih(), traces);
            let (_, a2) = v.chaine_tracee(&fond, a, 4, il(), ih(), traces);
            let ma = 0.5 * (a1.composer + a2.composer);
            let mb = 0.5 * (b1.composer + b2.composer);
            da.push(ma);
            db.push(mb);
            diff.push(mb - ma);
        }
        let med = |mut x: Vec<f64>| {
            x.sort_by(|p, q| p.partial_cmp(q).unwrap());
            x[x.len() / 2]
        };
        (med(da), med(db), med(diff))
    };

    dire!(j, "");
    dire!(j, "  millisecondes pour {traces} traces de la composition");
    dire!(j, "  paire                              A         B       B - A      B/A");
    dire!(j, "  ----------------------------------------------------------------------");
    let (a0, b0, d0) = apparier(&V0, &V0);
    dire!(
        j,
        "  V0 contre V0 (temoin)      {a0:>8.4} {b0:>9.4} {d0:>+10.4} ms {:>7.2}",
        b0 / a0.max(1e-9)
    );
    let v6_1 = Direction { prises: 1.0, ..V6 };
    let v6_4 = Direction { prises: 4.0, ..V6 };
    let v6_16 = Direction { prises: 16.0, ..V6 };
    let v6_32 = Direction { prises: 32.0, ..V6 };
    let paires: [(&str, &Direction, &Direction); 8] = [
        ("V0 -> V2 refraction     ", &V0, &V2),
        ("V0 -> V3 +Fresnel       ", &V0, &V3),
        ("V0 -> V6 tout           ", &V0, &V6),
        ("temoin verre.wgsl -> V6 ", &TEMOIN, &V6),
        ("V6 a 8 prises -> 1      ", &V6, &v6_1),
        ("V6 a 8 prises -> 4      ", &V6, &v6_4),
        ("V6 a 8 prises -> 16     ", &V6, &v6_16),
        ("V6 a 8 prises -> 32     ", &V6, &v6_32),
    ];
    let mut resultats = Vec::new();
    for (nom, a, b) in paires {
        let (ma, mb, md) = apparier(a, b);
        let rapport = mb / ma.max(1e-9);
        dire!(j, "  {nom}   {ma:>8.4} {mb:>9.4} {md:>+10.4} ms {rapport:>7.2}");
        resultats.push((nom, ma, md, rapport));
    }
    dire!(j, "");
    dire!(
        j,
        "  Le temoin de cette mesure est la PREMIERE ligne : la meme direction"
    );
    dire!(j, "  contre elle-meme doit rendre zero. Elle rend {d0:+.4} ms et un");
    dire!(j, "  rapport de {:.2}, et c'est le plancher de bruit ADDITIF de", b0 / a0.max(1e-9));
    dire!(j, "  l'instrument apparie. Toute difference plus petite que lui n'est");
    dire!(j, "  pas mesuree, elle est supposee. Il ne borne PAS le biais");
    dire!(j, "  multiplicatif, auquel il est aveugle par construction.");
    // Le temoin se juge dans l'unite que d1e5f94 a choisie : le RAPPORT, et non
    // la milliseconde. En millisecondes il admettait 40 % du signal, et un
    // rapport A contre A de 0,91 passait parce que la difference s'arrondissait
    // a +0,0000 ms.
    //
    // Son seuil ne se pose pas au juge : il derive du pas MESURE du chronometre.
    // Un rapport A contre A peut s'ecarter de 1 d'au plus un tic de chaque cote,
    // soit deux tics rapportes a la mediane de reference. C'est le seul nombre de
    // ce temoin, et c'est la definition meme de la resolution.
    //
    // La garde sur le tic n'est pas une precaution de forme. `tic_mesure` part de
    // f64::MAX et ne retrecit que s'il trouve deux valeurs distinctes ; sinon il
    // reste immense, `tic_sain` le laisse passer (f64::MAX est fini), et une
    // tolerance de deux tics engloberait tout. Le temoin redeviendrait aveugle
    // par construction — exactement la faute qu'il doit borner.
    let rapport0 = b0 / a0.max(1e-9);
    let ecart0 = (rapport0 - 1.0).abs();
    let tolerance0 = 2.0 * tic_mesure / a0.max(1e-9);
    let t63 = verdict(
        j,
        tic_utilisable && ecart0 <= tolerance0,
        &format!(
            "la meme direction contre elle-meme rend un rapport de {rapport0:.3},\n  a {ecart0:.3} de 1 pour une resolution de {tolerance0:.3} (deux tics\n  rapportes a la reference). Les rapports publies valent a cette\n  resolution pres, et pas mieux."
        ),
        &format!(
            "la meme direction contre elle-meme rend {rapport0:.3}, a {ecart0:.3} de 1,\n  pour une resolution de {tolerance0:.3}{}. L'instrument apparie ne\n  mesure pas une difference, il mesure son propre bruit.",
            if tic_utilisable { "" } else { " — et le tic mesure n'est pas utilisable" }
        ),
    );
    if t63 {
        dire!(j, "");
        dire!(j, "  CE QUI SE PUBLIE, et c'est la colonne sans unite :");
        dire!(
            j,
            "  resolution de ce lancement : {:.2} %, a {traces} traces",
            tolerance0 * 100.0
        );
        dire!(j, "  Chaque rapport porte sa borne d'arrondi, R x 2 tics / A avec SON A,");
        dire!(j, "  et autant de decimales que cette borne en laisse — l'unite du");
        dire!(j, "  dernier chiffre ecrit n'est jamais plus fine que la borne.");
        dire!(j, "");
        dire!(j, "    paire                      rapport     borne");
        dire!(j, "    ---------------------------------------------");
        for (nom, ma, md, rapport) in &resultats {
            let borne = rapport * 2.0 * tic_mesure / ma.max(1e-9);
            let decimales = (-borne.log10()).floor().clamp(0.0, 4.0) as usize;
            // Un ecart plus petit que sa borne n'est pas mesure, il est
            // suppose. On le dit a cote du chiffre plutot que de le taire.
            let sous = if (rapport - 1.0).abs() <= borne {
                "   (sous la borne)"
            } else {
                ""
            };
            dire!(
                j,
                "    {nom} {:>8.*} {borne:>9.4}   ({md:+.4} ms ce lancement){sous}",
                decimales,
                rapport
            );
        }
        // Ce que la borne ne dit pas, et que les premiers lancements a 64 et
        // 256 traces ont montre : V0 -> V6 y valait 2,71 puis 2,32, et dans le
        // second V0 -> V2 (refraction seule) coutait PLUS que V0 -> V3, qui la
        // contient. L'horloge change entre les paires ; le rapport ne s'en
        // protege qu'a l'interieur d'une paire, et le temoin A contre A n'est
        // qu'une paire. La borne est donc un plancher, pas une incertitude.
        dire!(j, "");
        dire!(j, "  Ces decimales sont celles que l'ARRONDI laisse dans ce lancement. La");
        dire!(j, "  borne n'est qu'un plancher : l'horloge change aussi d'une paire a");
        dire!(j, "  l'autre, et le temoin A contre A n'est qu'une paire. La");
        dire!(j, "  reproductibilite ne se lit que sur plusieurs journaux, et c'est elle");
        dire!(j, "  qui decide des chiffres publies quand elle est plus large.");
    }
    // Le verdict de 6.3 est RENDU, et non plus ecrase par un vrai. 6.1 et 6.2 ont
    // deja rendu faux plus haut s'ils echouaient ; arriver ici veut dire qu'ils
    // tiennent, et la validite de l'instrument depend donc de 6.3 seul. C'est lui
    // qui porte les chiffres publies : il doit pouvoir faire tomber l'instrument.
    t63
}


// ---------------------------------------------------------------------------
// INSTRUMENT 7 — LA MATIERE A TROIS TAILLES
// ---------------------------------------------------------------------------
//
// Les six premiers instruments ne connaissent que l'ilot DEPLOYE, et c'est le
// trou que ce tour a paye. Le galet REPLIE est l'etat PAR DEFAUT de l'ilot —
// 88 x 36, rayon 18, exposant 2,5 — et le ressort de `DESIGN.md` §6 relie les
// deux en 330 ms.
//
// Ce que cet instrument mesure est une PART et non une longueur, et c'est tout
// le point : 12 px de conge sur un objet de 148 px de haut sont un chant ; les
// memes 12 px sur un objet de 36 px de haut sont l'objet entier. L'oeil ne lit
// pas la largeur du conge, il lit sa part.

/// Les trois etats, tels que `DESIGN.md` §6 les pose. p se lit sur la HAUTEUR,
/// le rayon va de 18 a 48 et l'exposant de 2,5 a 4, interpoles lineairement.
/// La largeur repliee est celle de « 41:27 », 88 px.
const ETATS: [(&str, u32, u32, f32, f32); 3] = [
    ("replie    p = 0,00", 88, 36, 18.0, 2.50),
    ("mi-chemin p = 0,50", 204, 92, 33.0, 3.25),
    ("deploye   p = 1,00", 320, 148, 48.0, 4.00),
];

/// La part de l'aire du galet qui tombe dans un conge de largeur `b`, en %.
///
/// La geometrie est recalculee ici, comme partout dans ce banc : demander au
/// nuanceur ou est son propre conge ne prouverait rien.
fn part_du_conge(b: f64) -> f64 {
    let (mut dans, mut total) = (0usize, 0usize);
    for y in 0..ih() {
        for x in 0..il() {
            let d = distance(x as f32, y as f32) as f64;
            if d >= 0.0 {
                continue;
            }
            total += 1;
            if d > -b {
                dans += 1;
            }
        }
    }
    if total == 0 {
        0.0
    } else {
        100.0 * dans as f64 / total as f64
    }
}

/// La part des pixels du galet que la matiere ecarte de plus de `seuil`
/// niveaux du VERRE SEUL, a geometrie egale. C'est la mesure de « combien de
/// l'objet la matiere touche », et elle ne suppose rien de la forme de l'effet.
fn part_ecartee(a: &[u8], b: &[u8], seuil: i64) -> f64 {
    let (mut touches, mut total) = (0usize, 0usize);
    for y in 0..ih() {
        for x in 0..il() {
            if distance(x as f32, y as f32) as f64 >= -0.5 {
                continue;
            }
            total += 1;
            let i = ((y * il() + x) * 4) as usize;
            let e = (0..3)
                .map(|c| (a[i + c] as i64 - b[i + c] as i64).abs())
                .max()
                .unwrap();
            if e > seuil {
                touches += 1;
            }
        }
    }
    if total == 0 {
        0.0
    } else {
        100.0 * touches as f64 / total as f64
    }
}

/// Pose sur une image RENDUE un ecart dont la part touchee est connue d'avance.
///
/// Le temoin 7.1 ne prouve que le zero du compteur : un compteur qui ne lirait
/// rien le passerait aussi. Ici l'ecart existe, et chaque piege vise une facon
/// precise de mal compter :
///
///   - le rectangle COMPTE recoit +9 niveaux sur le seul canal BLEU. Un compteur
///     qui ne lirait que le vert, comme les pics du bord, n'y verrait rien ; et
///     9 est le plus petit ecart qui depasse le seuil de 8 ;
///   - le rectangle au SEUIL recoit exactement +8 sur le rouge, et ne doit pas
///     compter : le seuil est strict ;
///   - tout ce qui est HORS du galet (d >= -0,5) recoit +40 sur le vert, et ne
///     doit pas compter : le compteur ne lit que l'interieur ;
///   - le rectangle compte est dans le quart haut-gauche de la partie droite, et
///     le rectangle au seuil dans le quart bas-droit : une erreur de foulee ou
///     de ligne dans l'indice deplacerait l'ecart et changerait le compte.
///
/// Un ecart pose vers le haut qui depasserait 255 est pose vers le bas : sa
/// grandeur reste exacte. La reponse est `100 x aire / interieur`, ou l'aire est
/// celle d'un rectangle, largeur fois hauteur, et l'interieur le nombre de
/// pixels a d < -0,5. Rend l'image, la part attendue, et le nombre de pixels du
/// rectangle compte qui ne seraient PAS a l'interieur — zero, sans quoi c'est
/// la construction du temoin qui est fausse et non le compteur.
fn ecart_pose(a: &[u8]) -> (Vec<u8>, f64, usize) {
    let mut b = a.to_vec();
    let decaler = |b: &mut Vec<u8>, i: usize, k: i32| {
        let v = b[i] as i32;
        b[i] = (if v + k <= 255 { v + k } else { v - k }) as u8;
    };
    let r = rayon().ceil() as u32;
    let droite = il() - 2 * r;
    // Le quart haut-gauche et le quart bas-droit de la partie DROITE du galet,
    // a deux pixels du bord haut et du bord bas.
    let compte = (r, r + droite / 2, 2, ih() / 2);
    let seuil = (r + droite / 2, il() - r, ih() / 2, ih() - 2);
    let mut hors_interieur = 0usize;
    let mut interieur = 0usize;
    for y in 0..ih() {
        for x in 0..il() {
            let i = ((y * il() + x) * 4) as usize;
            let dedans = (distance(x as f32, y as f32) as f64) < -0.5;
            if dedans {
                interieur += 1;
            }
            if x >= compte.0 && x < compte.1 && y >= compte.2 && y < compte.3 {
                if !dedans {
                    hors_interieur += 1;
                }
                decaler(&mut b, i + 2, 9);
            } else if x >= seuil.0 && x < seuil.1 && y >= seuil.2 && y < seuil.3 {
                decaler(&mut b, i, 8);
            } else if !dedans {
                decaler(&mut b, i + 1, 40);
            }
        }
    }
    let aire = ((compte.1 - compte.0) * (compte.3 - compte.2)) as f64;
    (b, 100.0 * aire / interieur as f64, hors_interieur)
}

fn instrument_7(v: &Verre, j: &mut Journal) -> bool {
    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "INSTRUMENT 7 : LA MATIERE A TROIS TAILLES");
    dire!(j, "===========================================================");
    dire!(j, "Les six instruments precedents cablaient l'ilot deploye. Celui-ci");
    dire!(j, "pose la question qu'aucun d'eux ne pouvait poser : ce que la");
    dire!(j, "matiere fait a l'objet quand l'objet change de taille. Il mesure");
    dire!(j, "une PART, pas une longueur — 12 px de conge sur 148 px de haut");
    dire!(j, "sont un chant, les memes 12 px sur 36 px de haut sont l'objet.");

    let fond = fond_bureau_reel();
    let mut ok = true;

    // -- TEMOIN 7.1 : sans conge, il n'y a pas de conge --
    dire!(j, "");
    dire!(j, "-- TEMOIN 7.1 : sans conge, aucune part, a aucune taille --");
    let mut pire_vide = 0.0f64;
    let mut pire_ecart = 0.0f64;
    for (_, l, h, r, n) in ETATS {
        geometrie::poser(l, h, r, n);
        pire_vide = pire_vide.max(part_du_conge(0.0));
        let a = v.rendre(&fond, &V0);
        let b = v.rendre(&fond, &V0);
        pire_ecart = pire_ecart.max(part_ecartee(&a, &b, 8));
    }
    dire!(j, "  part du conge a b = 0, pire des trois tailles : {pire_vide:.1} %");
    dire!(j, "  part ecartee de V0 par V0, pire des trois      : {pire_ecart:.2} %");
    ok &= verdict(
        j,
        pire_vide == 0.0 && pire_ecart == 0.0,
        "un conge nul n'occupe rien, et le verre seul ne s'ecarte pas de\n  lui-meme. Les deux compteurs partent de zero.",
        "l'instrument compte quelque chose la ou il n'y a rien.",
    );

    // -- TEMOIN 7.2 : un conge aussi large que le galet l'occupe en entier --
    dire!(j, "");
    dire!(j, "-- TEMOIN 7.2 : un conge aussi large que le galet l'occupe en entier --");
    let mut pire_plein = 100.0f64;
    for (_, l, h, r, n) in ETATS {
        geometrie::poser(l, h, r, n);
        pire_plein = pire_plein.min(part_du_conge(h as f64));
    }
    dire!(j, "  part du conge a b = hauteur, pire des trois : {pire_plein:.1} %");
    ok &= verdict(
        j,
        pire_plein >= 99.99,
        "un conge aussi large que l'objet en occupe 100 %. L'echelle du\n  compteur est donc juste aux deux bouts.",
        "un conge aussi large que l'objet n'en occupe pas la totalite.",
    );

    // -- TEMOIN 7.3 : une part touchee connue d'avance, et non nulle --
    dire!(j, "");
    dire!(j, "-- TEMOIN 7.3 : une part touchee connue d'avance, et non nulle --");
    dire!(j, "Le zero du temoin 7.1 ne distingue pas un compteur juste d'un");
    dire!(j, "compteur aveugle. On pose donc sur le rendu du verre seul un ecart");
    dire!(j, "dont la part est connue : +9 niveaux sur le bleu dans un rectangle");
    dire!(j, "(doit compter), +8 sur le rouge dans un autre (au seuil, ne doit");
    dire!(j, "pas compter), +40 sur le vert hors du galet (ne doit pas compter).");
    dire!(j, "La reponse est l'aire du premier rectangle sur l'interieur. Le");
    dire!(j, "compte est entier : la tolerance est d'un demi-pixel, 50 / interieur");
    dire!(j, "points, et le compteur est exact ou faux.");
    dire!(j, "");
    dire!(j, "  etat                  posee      lue    ecart   tolerance");
    dire!(j, "  -----------------------------------------------------------");
    let mut positif = true;
    for (nom, l, h, r, n) in ETATS {
        geometrie::poser(l, h, r, n);
        let a = v.rendre(&fond, &V0);
        let (b, attendue, hors) = ecart_pose(&a);
        let lue = part_ecartee(&a, &b, 8);
        let interieur = (0..ih())
            .flat_map(|y| (0..il()).map(move |x| (x, y)))
            .filter(|&(x, y)| (distance(x as f32, y as f32) as f64) < -0.5)
            .count();
        let tolerance = 50.0 / interieur as f64;
        dire!(
            j,
            "  {nom}  {attendue:>7.3} % {lue:>7.3} % {:>+8.4} {tolerance:>9.4}",
            lue - attendue
        );
        positif &= hors == 0 && (lue - attendue).abs() <= tolerance;
        if hors != 0 {
            dire!(j, "  construction fausse : {hors} pixels du rectangle sont hors du galet");
        }
    }
    ok &= verdict(
        j,
        positif,
        "le compteur lit la part posee au pixel pres, a chacune des trois\n  tailles, sans compter l'ecart au seuil ni l'ecart hors du galet.\n  La part touchee mesuree plus bas est donc un compte juste, et non\n  un zero qui ne sait rien lire d'autre.",
        "le compteur ne lit pas la part posee. Les colonnes « touche » plus\n  bas ne valent rien.",
    );

    // -- LA MESURE --
    dire!(j, "");
    dire!(j, "-- LA MESURE : deux lois de conge, trois tailles --");
    dire!(j, "A gauche, `b` et `e` sont des LONGUEURS FIXES de 12 px, ce que la");
    dire!(j, "premiere realisation faisait. A droite, elles SUIVENT LA HAUTEUR :");
    dire!(j, "b = e = 12 x h / 148, la meme loi que DESIGN.md §6 applique deja au");
    dire!(j, "rayon de coin et a l'exposant, qui se lisent tous deux sur la");
    dire!(j, "hauteur. Le facteur vaut 1 a la taille deployee, donc la colonne de");
    dire!(j, "droite y est exactement la colonne de gauche.");
    dire!(j, "");
    dire!(j, "                          b fixe 12 px           b suit la hauteur");
    dire!(j, "  etat                   b   part  touche       b   part  touche");
    dire!(j, "  --------------------------------------------------------------------");
    let fixe = Direction { conge_relatif: false, ..V3 };
    let relatif = Direction { conge_relatif: true, ..V3 };
    let mut parts = Vec::new();
    for (nom, l, h, r, n) in ETATS {
        geometrie::poser(l, h, r, n);
        let b_rel = CONGE as f64 * h as f64 / PLUS_GRAND_H as f64;
        let p_fixe = part_du_conge(CONGE as f64);
        let p_rel = part_du_conge(b_rel);
        let v0 = v.rendre(&fond, &V0);
        let t_fixe = part_ecartee(&v0, &v.rendre(&fond, &fixe), 8);
        let t_rel = part_ecartee(&v0, &v.rendre(&fond, &relatif), 8);
        dire!(
            j,
            "  {nom} {:>5.1} {p_fixe:>6.1} % {t_fixe:>6.1} %  {b_rel:>6.2} {p_rel:>6.1} % {t_rel:>6.1} %",
            CONGE
        );
        parts.push((p_fixe, p_rel, t_fixe, t_rel));
    }
    geometrie::deploye();
    let etendue = |f: fn(&(f64, f64, f64, f64)) -> f64, p: &Vec<(f64, f64, f64, f64)>| -> f64 {
        p.iter().map(f).fold(0.0f64, f64::max) - p.iter().map(f).fold(1e9f64, f64::min)
    };
    let ecart_fixe = etendue(|p| p.0, &parts);
    let ecart_rel = etendue(|p| p.1, &parts);
    let touche_fixe = etendue(|p| p.2, &parts);
    let touche_rel = etendue(|p| p.3, &parts);
    dire!(j, "");
    dire!(j, "  etendue entre les trois tailles :");
    dire!(j, "    part du conge   b fixe {ecart_fixe:>6.1} points   b relatif {ecart_rel:>6.1} points");
    dire!(j, "    part touchee    b fixe {touche_fixe:>6.1} points   b relatif {touche_rel:>6.1} points");
    dire!(j, "");
    dire!(j, "  C'est cette etendue-la qui decide si l'objet GRANDIT ou se");
    dire!(j, "  TRANSMUTE pendant les 330 ms du ressort. Une part qui change de");
    dire!(j, "  facteur trois donne trois substances differentes ; une part qui");
    dire!(j, "  ne bouge pas donne un objet de la meme matiere a trois tailles.");
    ok
}

// ---------------------------------------------------------------------------
// INSTRUMENT 8 — LE BORD : SA MARCHE, SES DENTS, ET SON NIVEAU
// ---------------------------------------------------------------------------

/// Le gris dont la luminance vaut celle de `#5E5B57`, l'encre la moins lue de
/// la palette (`DESIGN.md` §2). C'est la borne que `MATIERE.md` §5.2 pose en
/// toutes lettres : aucune decoration de bord ne doit briller plus que le texte
/// le moins lu. Calcule ici, pas recopie.
fn encre_la_moins_lue() -> f64 {
    255.0 * vers_srgb(luminance_relative(0x5E, 0x5B, 0x57))
}

/// Le plus grand niveau atteint par le PREMIER PIXEL du bord, sur les bords
/// DROITS seulement.
///
/// C'est la grandeur que la garantie de `MATIERE.md` §5.2 nomme — « aucune
/// decoration de bord ne brille plus que le texte le moins lu » parle du pixel
/// le plus clair, pas d'une moyenne. Les coins sont exclus : la distance y
/// tombe n'importe ou dans le pixel, donc le premier bac y melange du verre et
/// du fond intact (c'est le temoin 2.3 du profil normal qui l'a etabli).
fn pic_du_bord_droit(px: &[u8]) -> f64 {
    let mut pic = 0.0f64;
    for y in 0..ih() {
        for x in 0..il() {
            let d = distance(x as f32, y as f32) as f64;
            if d > -0.45 || d < -1.5 {
                continue;
            }
            let dans_coin = ((x as f32) < rayon() || (x as f32) > il() as f32 - rayon())
                && ((y as f32) < rayon() || (y as f32) > ih() as f32 - rayon());
            if dans_coin {
                continue;
            }
            let i = ((y * il() + x) * 4) as usize;
            // En luminance, et non sur le vert : voir `niveau_luminance`.
            pic = pic.max(niveau_luminance(px[i], px[i + 1], px[i + 2]));
        }
    }
    pic
}

/// Le plus grand saut de niveau entre les DEUX COLONNES VOISINES qui encadrent
/// la distance signee `d_cible`, sur la partie strictement droite du bord
/// droit. Un verre n'a pas de marche a l'interieur de lui-meme.
fn saut_a(px: &[u8], d_cible: f64) -> (f64, usize) {
    let ym = ih() / 2;
    let mut colonne = None;
    for x in 1..il() - 2 {
        let d0 = distance(x as f32, ym as f32) as f64;
        let d1 = distance((x + 1) as f32, ym as f32) as f64;
        if d0 <= d_cible && d1 > d_cible && d0 < 0.0 {
            colonne = Some(x);
        }
    }
    let colonne = match colonne {
        Some(x) => x,
        None => return (f64::NAN, 0),
    };
    let ref_d = distance(colonne as f32, ym as f32) as f64;
    let (mut pire, mut n) = (0.0f64, 0usize);
    for y in 0..ih() {
        // On ne garde que les lignes ou le bord est VRAIMENT droit : la
        // distance de la colonne ne doit pas dependre de y.
        if ((distance(colonne as f32, y as f32) as f64) - ref_d).abs() > 0.02 {
            continue;
        }
        let i0 = ((y * il() + colonne) * 4) as usize;
        let i1 = ((y * il() + colonne + 1) * 4) as usize;
        pire = pire.max((px[i0 + 1] as f64 - px[i1 + 1] as f64).abs());
        n += 1;
    }
    (pire, n)
}

/// L'amplitude crete-a-crete de la luminance et la plus grande saturation
/// d'une bande du bord HAUT, prise entre deux distances signees.
fn bande_du_haut(px: &[u8], de: f64, a: f64) -> (f64, f64) {
    let (mut bas, mut haut, mut sat) = (255.0f64, 0.0f64, 0.0f64);
    let x0 = (rayon() as u32) + 12;
    let x1 = il() - (rayon() as u32) - 12;
    for y in 0..ih() / 2 {
        for x in x0..x1 {
            let d = distance(x as f32, y as f32) as f64;
            if d < de || d > a {
                continue;
            }
            let i = ((y * il() + x) * 4) as usize;
            let g = niveau_luminance(px[i], px[i + 1], px[i + 2]);
            bas = bas.min(g);
            haut = haut.max(g);
            let mx = px[i].max(px[i + 1]).max(px[i + 2]) as f64;
            let mn = px[i].min(px[i + 1]).min(px[i + 2]) as f64;
            sat = sat.max(mx - mn);
        }
    }
    (haut - bas, sat)
}

/// La meme bande, lue sur le FOND LUI-MEME, sans aucun rendu.
///
/// C'est le temoin qui decide la question de la couleur, et il ne peut pas se
/// deriver d'un rendu : une matiere n'invente pas de couleur tant qu'elle reste
/// SOUS la saturation de ce qu'elle laisse voir. Le pixel de cible (x, y) lit
/// la source en (x + marge, y + marge) — c'est le meme decalage que le
/// nuanceur.
fn bande_du_haut_source(fond: &[u8], de: f64, a: f64) -> (f64, f64) {
    let (mut bas, mut haut, mut sat) = (255.0f64, 0.0f64, 0.0f64);
    let x0 = (rayon() as u32) + 12;
    let x1 = il() - (rayon() as u32) - 12;
    for y in 0..ih() / 2 {
        for x in x0..x1 {
            let d = distance(x as f32, y as f32) as f64;
            if d < de || d > a {
                continue;
            }
            let i = (((y + MARGE) * SL + x + MARGE) * 4) as usize;
            let g = niveau_luminance(fond[i], fond[i + 1], fond[i + 2]);
            bas = bas.min(g);
            haut = haut.max(g);
            let mx = fond[i].max(fond[i + 1]).max(fond[i + 2]) as f64;
            let mn = fond[i].min(fond[i + 1]).min(fond[i + 2]) as f64;
            sat = sat.max(mx - mn);
        }
    }
    (haut - bas, sat)
}

/// Les DENTS au sens strict : ce qui varie LE LONG du bord, a distance fixe.
///
/// `bande_du_haut` prend l'amplitude crete-a-crete de toute la bande, et elle
/// melange deux choses : les dents, qui varient le long du bord, et le profil du
/// lisere, qui decroit en s'eloignant de l'arete. Un lisere parfait, sans aucune
/// dent, y rend encore l'ecart entre son premier pixel et son cinquieme. Ici on
/// ne compare que des pixels a la MEME distance : sur la partie droite du bord
/// haut, une rangee est a distance constante, et l'on rend la plus grande
/// amplitude d'une rangee, en luminance.
fn dents_le_long_du_bord(px: &[u8], de: f64, a: f64) -> f64 {
    let x0 = (rayon() as u32) + 12;
    let x1 = il() - (rayon() as u32) - 12;
    let mut pire = 0.0f64;
    for y in 0..ih() / 2 {
        let d = distance(x0 as f32, y as f32) as f64;
        if d < de || d > a {
            continue;
        }
        let (mut bas, mut haut) = (f64::INFINITY, f64::NEG_INFINITY);
        for x in x0..x1 {
            let i = ((y * il() + x) * 4) as usize;
            let l = niveau_luminance(px[i], px[i + 1], px[i + 2]);
            bas = bas.min(l);
            haut = haut.max(l);
        }
        pire = pire.max(haut - bas);
    }
    pire
}

/// Les deux lecteurs de dents, eprouves sur une image POSEE a la taille
/// courante.
///
/// Gris 100 ; les rangees 0 a 4 du bord haut en degrade PERPENDICULAIRE au bord
/// — 180, 160, 140, 120, 100 — sans aucune variation le long du bord. Puis 150
/// au lieu de 140, trois colonnes sur six, dans la rangee 2 : des dents de 10.
/// Deux pieges hors lecture : 255 dans la rangee 1 une colonne avant la partie
/// droite, et des dents de 50 dans la rangee 6, a d = -6,5.
///
/// Reponses connues : crete-a-crete 80 ; le long du bord 0 sans les dents, 10
/// avec.
fn dents_posees() -> (f64, f64, f64) {
    let r = rayon() as u32;
    let (x0, x1) = (r + 12, il() - r - 12);
    let mut px = vec![100u8; (il() * ih() * 4) as usize];
    let gris = |px: &mut Vec<u8>, x: u32, y: u32, v: u8| {
        let i = ((y * il() + x) * 4) as usize;
        px[i] = v;
        px[i + 1] = v;
        px[i + 2] = v;
    };
    for y in 0..5u32 {
        for x in 0..il() {
            gris(&mut px, x, y, (180 - 20 * y) as u8);
        }
    }
    let (ampl, _) = bande_du_haut(&px, -5.0, -0.45);
    let sans = dents_le_long_du_bord(&px, -5.0, -0.45);
    for x in x0..x1 {
        if x % 6 < 3 {
            gris(&mut px, x, 2, 150);
            gris(&mut px, x, 6, 150);
        }
    }
    gris(&mut px, x0 - 1, 1, 255);
    let avec = dents_le_long_du_bord(&px, -5.0, -0.45);
    (ampl, sans, avec)
}

/// Le contraste de l'encre la plus claire sur le verre, lu LOIN du bord.
///
/// C'est le temoin de tout balayage de force : la reflexion n'existe que dans
/// le conge, donc ce nombre ne doit bouger d'aucune ligne quand la force bouge.
/// Le point de lecture est a 30 px du bord, soit deux congés et demi plus a
/// l'interieur — hors de toute portee de l'effet.
fn contraste_du_coeur(px: &[u8]) -> f64 {
    let pr = profil_normal(px, Zone::Droit, 40, TOLERANCE);
    let coeur = au(&pr, -30.0).unwrap_or(f64::NAN);
    contraste_l(
        luminance_relative(0xF2, 0xEF, 0xE9),
        luminance_relative(coeur as u8, coeur as u8, coeur as u8),
    )
}

/// Un peigne de 3 px sur 3 qui ne change pas quand la cible change de taille.
///
/// C'est le piege que l'essai 9.C avoue : une cible plus petite lit la source a
/// partir du MEME coin, donc sur `fond_bureau` le galet replie tombe sur deux
/// bandes presque grises et ne voit plus de couleur. Ce fond-ci n'a qu'une
/// couleur sous ses raies et ne depend que de x modulo 6 : la bande du bord haut
/// voit le meme peigne, a la meme phase, a chaque taille. Les lignes de tailles
/// differentes se comparent donc entre elles.
fn fond_peigne(couleur: (u8, u8, u8)) -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let c = if (x % 6) < 3 { (0x10, 0x10, 0x10) } else { couleur };
            let i = ((y * SL + x) * 4) as usize;
            p[i] = c.0;
            p[i + 1] = c.1;
            p[i + 2] = c.2;
        }
    }
    p
}

/// La direction telle que le nuanceur la recoit a la hauteur courante : `b` et
/// `e` effectifs, et plus de loi. C'est ce qui permet a `niveau_verrier`, qui
/// ne connait que des longueurs, de predire le lisere a toute taille.
fn a_la_taille(d: &Direction) -> Direction {
    if d.conge_relatif {
        let f = ih() as f32 / PLUS_GRAND_H as f32;
        Direction {
            largeur: d.largeur * f,
            epaisseur: d.epaisseur * f,
            conge_relatif: false,
            ..*d
        }
    } else {
        *d
    }
}

/// Les deux lecteurs du bord, eprouves sur une image POSEE a la taille courante.
///
/// Ils choisissent leurs pixels par la distance et par le rayon, qui changent
/// tous deux avec la taille : c'est la qu'une lecture pourrait se tromper sans
/// que rien ne se voie. L'image est un gris 100 uni, sur lequel on pose :
///
///   - pour le pic : 200 sur UN pixel du premier rang du bord haut (doit etre
///     lu) ; 255 sur un pixel du premier rang DANS un coin, 255 a d = -2,5 et
///     255 hors du galet (aucun ne doit l'etre). Reponse : 200 ;
///   - pour la bande : 160 a d = -1,5 du bord haut (doit etre lu) ; 255 sur la
///     meme rangee une colonne avant `rayon + 12`, 255 a d = -5,5 et 255 dans
///     la moitie basse (aucun ne doit l'etre). Reponse : amplitude 60,
///     saturation 0.
///
/// Rend (pic lu, amplitude lue, saturation lue, le piege du coin a-t-il pu
/// etre pose).
fn lecteurs_poses() -> (f64, f64, f64, bool) {
    let gris = |px: &mut Vec<u8>, x: u32, y: u32, v: u8| {
        let i = ((y * il() + x) * 4) as usize;
        px[i] = v;
        px[i + 1] = v;
        px[i + 2] = v;
    };
    let base = vec![100u8; (il() * ih() * 4) as usize];
    let milieu = il() / 2;
    let r = rayon() as u32;

    let mut a = base.clone();
    gris(&mut a, milieu, 0, 200);
    gris(&mut a, milieu, 2, 255);
    gris(&mut a, 0, 0, 255);
    let coin = (0..r)
        .flat_map(|y| (0..r).map(move |x| (x, y)))
        .find(|&(x, y)| {
            let d = distance(x as f32, y as f32) as f64;
            (-1.5..=-0.45).contains(&d)
        });
    if let Some((x, y)) = coin {
        gris(&mut a, x, y, 255);
    }
    let pic = pic_du_bord_droit(&a);

    let mut b = base;
    gris(&mut b, milieu, 1, 160);
    gris(&mut b, r + 11, 1, 255);
    gris(&mut b, milieu, 5, 255);
    gris(&mut b, milieu, ih() - 2, 255);
    let (ampl, sat) = bande_du_haut(&b, -5.0, -0.45);
    (pic, ampl, sat, coin.is_some())
}

fn instrument_8(v: &Verre, j: &mut Journal) -> bool {
    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "INSTRUMENT 8 : LE BORD — SA MARCHE, SES DENTS, ET SON NIVEAU");
    dire!(j, "===========================================================");
    dire!(j, "Trois grandeurs que le banc ne mesurait pas, et qui decident");
    dire!(j, "chacune d'une chose que le projet interdit : une MARCHE a");
    dire!(j, "l'interieur du verre (DESIGN.md §8, « pas de trait de separation »),");
    dire!(j, "des DENTS colorees dans la bande de bord (§8, « pas de couleur »),");
    dire!(j, "et un LISERE plus clair que l'encre la moins lue (MATIERE.md §5.2).");
    geometrie::deploye();
    let reel = fond_bureau_reel();
    let bureau = fond_bureau();
    let mut ok = true;

    let marche = Direction { profil: 0.0, conge_relatif: true, ..V3 };
    let rampe = Direction { profil: 1.0, conge_relatif: true, ..V3 };
    let vibre = Direction {
        vibrance_relative: true,
        nette_luminance: false,
        ..rampe
    };
    let luma = Direction {
        nette_luminance: true,
        ..rampe
    };
    // La force 0,30 est celle que l'essai D plus bas retient, et elle est
    // citee ici parce que l'ordre du journal n'est pas celui de la deduction :
    // c'est la GARANTIE de MATIERE.md §5.2 qui la borne, et une fois bornee il
    // faut relire le peigne a cette force-la et non a 1,00.
    let rampe_30 = Direction { lisere: 0.30, ..rampe };
    let luma_30 = Direction { lisere: 0.30, ..luma };
    let rampe_00 = Direction { lisere: 0.0, ..rampe };
    let luma_00 = Direction { lisere: 0.0, ..luma };
    let colonnes: [(&str, &Direction); 9] = [
        ("verre seul", &V0),
        ("MARCHE", &marche),
        ("RAMPE", &rampe),
        ("RAMPE+vibrance", &vibre),
        ("RAMPE+luminance", &luma),
        ("RAMPE      f0,30", &rampe_30),
        ("RAMPE+lum. f0,30", &luma_30),
        ("RAMPE      f0,00", &rampe_00),
        ("RAMPE+lum. f0,00", &luma_00),
    ];
    let b = CONGE as f64;

    // -- A. LA COUTURE --
    dire!(j, "");
    dire!(j, "-- A. LA COUTURE : y a-t-il une marche a la frontiere du conge ? --");
    dire!(j, "La couche nette s'allume dans le conge et s'eteint au-dela. Si elle");
    dire!(j, "s'eteint D'UN COUP, le fond passe de net a flou en une colonne de");
    dire!(j, "pixels, et cela se lit comme un trait parallele au bord, a 12 px de");
    dire!(j, "lui, tout le tour de l'ilot. On mesure donc le saut de niveau entre");
    dire!(j, "les deux colonnes qui encadrent d = -12, sur la partie strictement");
    dire!(j, "droite du bord droit.");
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.1 : 8 px plus a l'interieur, ou aucune couture ne peut --");
    dire!(j, "-- exister, l'instrument doit rendre zero --");
    dire!(j, "");
    dire!(j, "  direction           saut a d = -12          TEMOIN a d = -20");
    dire!(j, "  ---------------------------------------------------------------");
    let mut temoin_pire = 0.0f64;
    for (nom, d) in colonnes {
        let px = v.rendre(&reel, d);
        let (s, n) = saut_a(&px, -b);
        let (t, _) = saut_a(&px, -b - 8.0);
        dire!(j, "  {nom:<18} {s:>7.1} niv ({n:>3} lignes) {t:>11.1} niv");
        temoin_pire = temoin_pire.max(t);
    }
    ok &= verdict(
        j,
        temoin_pire <= 2.0,
        "a 8 px plus a l'interieur, les trois directions rendent le meme\n  zero. L'instrument ne fabrique donc pas de marche : celle qu'il\n  rapporte a d = -12 est dans l'image.",
        "l'instrument voit une marche la ou il ne peut pas y en avoir.",
    );

    // -- B. LES DENTS --
    dire!(j, "");
    dire!(j, "-- B. LE PEIGNE : la bande de bord porte-t-elle des dents ? --");
    dire!(j, "Sur un bureau raye colore, la bande des cinq premiers pixels du");
    dire!(j, "bord haut est comparee au COEUR de la meme image. Le coeur est le");
    dire!(j, "temoin : il ne doit pas bouger d'une direction a l'autre, sans quoi");
    dire!(j, "l'ecart mesure au bord ne serait pas un ecart de bord.");
    dire!(j, "");
    dire!(j, "Et le SECOND temoin est le fond lui-meme, lu sans aucun rendu : une");
    dire!(j, "matiere n'invente pas de couleur tant qu'elle reste SOUS la");
    dire!(j, "saturation de ce qu'elle laisse voir. Au-dessus, elle en ajoute.");
    let mut coeurs = Vec::new();
    for (scene, f) in [("bureau raye colore", &bureau), ("bureau vraisemblable", &reel)] {
        dire!(j, "");
        dire!(j, "  {scene}");
        dire!(j, "  direction            bande 0-5 px          COEUR (temoin)");
        dire!(j, "                      ampl.  saturation     ampl.  saturation");
        dire!(j, "  -----------------------------------------------------------------");
        let (an, sn) = bande_du_haut_source(f, -5.0, -0.45);
        dire!(j, "  LE FOND NU           {an:>7.1} {sn:>10.0}           —          —");
        for (nom, d) in colonnes {
            let px = v.rendre(f, d);
            let (ab, sb) = bande_du_haut(&px, -5.0, -0.45);
            let (ac, sc) = bande_du_haut(&px, -40.0, -30.0);
            dire!(j, "  {nom:<18} {ab:>7.1} {sb:>10.0}    {ac:>8.1} {sc:>10.0}");
            if scene == "bureau raye colore" {
                coeurs.push((ac, sc));
            }
        }
    }
    let derive_coeur = coeurs
        .iter()
        .map(|c| (c.0 - coeurs[0].0).abs().max((c.1 - coeurs[0].1).abs()))
        .fold(0.0f64, f64::max);
    ok &= verdict(
        j,
        derive_coeur <= 1.0,
        "le coeur est le meme dans les trois images, a 1 niveau pres. Ce que\n  les colonnes de gauche montrent est donc bien du bord.",
        "le coeur bouge d'une direction a l'autre : la comparaison de bord\n  ne vaut rien.",
    );

    // -- C. LE NIVEAU DU LISERE, CONTRE L'ENCRE LA MOINS LUE --
    dire!(j, "");
    dire!(j, "-- C. LE LISERE BRILLE-T-IL PLUS QUE LE TEXTE LE MOINS LU ? --");
    dire!(
        j,
        "MATIERE.md §5.2 pose la garantie en toutes lettres : aucune decoration"
    );
    dire!(
        j,
        "de bord ne doit briller plus que #5E5B57, l'encre la moins lue. Ce gris"
    );
    dire!(j, "vaut {:.0} niveaux, calcule ici et non recopie.", encre_la_moins_lue());
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.3 : le verre seul, dont les quatre valeurs sont connues --");
    dire!(j, "-- d'avance : 45,0 / 45,0 / 34,7 / 42,0 --");
    dire!(j, "");
    dire!(j, "  scene     verre seul   MARCHE    RAMPE   +lumin.   l'encre");
    dire!(j, "  ---------------------------------------------------------------");
    let scenes: [(&str, Vec<u8>); 4] = [
        ("nuit  ", fond_horloge(false)),
        ("coupe ", fond_coupe()),
        ("bureau", fond_bureau()),
        ("reel  ", fond_bureau_reel()),
    ];
    let attendu = [45.0f64, 45.0, 34.7, 42.0];
    let mut pire_temoin = 0.0f64;
    let mut pire_lisere = 0.0f64;
    let mut verre_seul = Vec::new();
    for (k, (nom, f)) in scenes.iter().enumerate() {
        let lire = |d: &Direction| -> f64 { pic_du_bord_droit(&v.rendre(f, d)) };
        let l0 = lire(&V0);
        let lm = lire(&marche);
        let lr = lire(&rampe);
        let ll = lire(&luma);
        pire_temoin = pire_temoin.max((l0 - attendu[k]).abs());
        pire_lisere = pire_lisere.max(ll);
        verre_seul.push(l0);
        dire!(
            j,
            "  {nom}  {l0:>9.1} {lm:>8.1} {lr:>8.1} {ll:>8.1} {:>9.0}",
            encre_la_moins_lue()
        );
    }
    ok &= verdict(
        j,
        pire_temoin <= 0.6,
        "le verre seul rend les quatre valeurs connues d'avance a 0,6 niveau\n  pres. L'instrument lit donc bien le premier pixel du bord.",
        "le verre seul ne rend pas ses valeurs connues. Les colonnes de\n  droite ne valent rien.",
    );
    dire!(j, "");
    dire!(
        j,
        "  A pleine force la matiere porte le premier pixel du bord a {pire_lisere:.1}"
    );
    dire!(
        j,
        "  niveaux, soit {:+.0} par rapport a l'encre la moins lue. La garantie",
        pire_lisere - encre_la_moins_lue()
    );
    dire!(j, "  de MATIERE.md §5.2 est donc CASSEE a force 1,00.");

    // -- D. LA FORCE QUE LA GARANTIE LAISSE --
    dire!(j, "");
    dire!(j, "-- D. QUELLE FORCE LA GARANTIE LAISSE-T-ELLE ? --");
    dire!(j, "La force de la reflexion ne se choisit pas au juge : la garantie de");
    dire!(j, "MATIERE.md §5.2 la BORNE. On balaie donc, et l'on retient la plus");
    dire!(j, "grande force dont le premier pixel du bord reste sous l'encre la");
    dire!(j, "moins lue sur les QUATRE scenes — le pire cas, pas la moyenne.");
    dire!(j, "");
    dire!(j, "Le second critere est qu'un lisere eteint ne vaut rien : sur un");
    dire!(j, "bureau clair la matiere doit rester AU-DESSUS de ce que le verre");
    dire!(j, "seul fait deja, sinon elle ne se voit pas.");
    dire!(j, "");
    dire!(j, "  force   pire des 4 scenes          marge   bureau   contraste coeur");
    dire!(j, "  ---------------------------------------------------------------------");
    let encre = encre_la_moins_lue();
    let forces = [0.00f32, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.40, 0.50, 1.00];
    let mut retenue = 0.0f32;
    let mut retenue_pire = 0.0f64;
    let mut suite = Vec::new();
    let mut coeurs_force = Vec::new();
    for force in forces {
        let d = Direction {
            lisere: force,
            ..luma
        };
        let mut pire = 0.0f64;
        for (_, f) in scenes.iter() {
            pire = pire.max(pic_du_bord_droit(&v.rendre(f, &d)));
        }
        let px = v.rendre(&scenes[2].1, &d);
        let sur_bureau = pic_du_bord_droit(&px);
        let c_coeur = contraste_du_coeur(&px);
        let dit = if pire <= encre { "TIENT" } else { "CASSE" };
        dire!(
            j,
            "  {force:>5.2}   {pire:>10.1} niv {dit}  {:>+8.1} niv {sur_bureau:>8.1} {c_coeur:>12.2}",
            encre - pire
        );
        suite.push(pire);
        coeurs_force.push(c_coeur);
        if pire <= encre && force > retenue {
            retenue = force;
            retenue_pire = pire;
        }
    }
    dire!(j, "");
    dire!(
        j,
        "  La plus grande force que la garantie laisse est {retenue:.2}, a"
    );
    dire!(j, "  {retenue_pire:.1} niveaux pour un plafond de {encre:.0}.");

    // -- TEMOIN 8.4 --
    //
    // Le premier temoin ecrit ici demandait qu'a force nulle le bord retombe
    // sur le verre seul. Le banc l'a refuse — 43 contre 35 sur le bureau — et
    // il avait raison de le refuser : a force nulle il reste la REFRACTION,
    // qui deplace l'echantillon de bord vers l'interieur. Le temoin reclamait
    // donc une chose fausse. Ce qu'on peut affirmer d'avance de ce balayage est
    // autre chose, et c'est verifiable : la force multiplie un terme AJOUTE
    // (`reflet * u.bord.x`), donc la suite ne peut pas redescendre ; et ce
    // terme n'existe que dans le conge, donc le coeur ne peut pas bouger.
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.4 : ce que la force ne PEUT pas faire --");
    dire!(j, "Le nuanceur ajoute `reflet x force`. Deux consequences se");
    dire!(j, "verifient sans rien mesurer d'autre : la suite des pics est");
    dire!(j, "CROISSANTE, et le contraste du coeur ne bouge d'aucune ligne.");
    dire!(j, "");
    let mut recul = 0.0f64;
    for k in 1..suite.len() {
        recul = recul.max(suite[k - 1] - suite[k]);
    }
    let derive = coeurs_force
        .iter()
        .map(|c| (c - coeurs_force[0]).abs())
        .fold(0.0f64, f64::max);
    dire!(j, "  plus grand recul de la suite des pics : {recul:.1} niv");
    dire!(j, "  plus grande derive du contraste du coeur : {derive:.3}");
    ok &= verdict(
        j,
        recul <= 0.0 && derive <= 0.01,
        "la suite ne recule jamais et le coeur ne bouge pas. La force agit\n  donc bien sur un terme ajoute, et seulement dans le conge : le\n  balayage mesure la reflexion et rien d'autre.",
        "la suite recule, ou le coeur bouge. Le balayage ne mesure pas ce\n  qu'il pretend mesurer.",
    );
    dire!(j, "");
    dire!(j, "  A force nulle le bord vaut encore 45 / 43 / 43 / 44 niveaux");
    dire!(j, "  contre 45 / 45 / 35 / 42 pour le verre seul. Cet ecart-la est");
    dire!(j, "  la REFRACTION, qui ne s'eteint pas avec la force — et c'est");
    dire!(j, "  pourquoi le temoin ne peut pas demander qu'il disparaisse.");

    // -- E. LE LISERE ET LES DENTS AUX TROIS TAILLES --
    ok &= lisere_et_dents_aux_trois_tailles(v, j);
    geometrie::deploye();

    // -- F. LA GARANTIE SUR UN FOND COLORE --
    ok &= garantie_sur_fond_colore(v, j);
    geometrie::deploye();

    // -- G. LE PEIGNE ET LE LISERE : SE DECOUPLENT-ILS ? --
    ok &= peigne_et_lisere(v, j);
    geometrie::deploye();

    // -- H. L'ARBITRAGE --
    ok &= arbitrage(v, j);
    geometrie::deploye();
    ok
}

/// Une page de texte plein cadre : le blanc pur tel qu'un bureau le montre
/// vraiment, un document ouvert. Blanc 255, des lignes d'encre de 4 px au pas de
/// 11 px, des lettres de 5 px sur 7 et un blanc de mot toutes les cinq lettres.
/// C'est le fond qui manquait a l'essai 8.F : le blanc y est en aplat, jamais
/// traverse d'encre.
fn fond_page_texte() -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let ligne = (y % 11) < 4;
            let lettre = ((x / 7) % 5) != 4 && (x % 7) < 5;
            if ligne && lettre {
                let i = ((y * SL + x) * 4) as usize;
                p[i] = 0x20;
                p[i + 1] = 0x20;
                p[i + 2] = 0x20;
            }
        }
    }
    p
}

/// Le battement au changement de seconde, lu en LUMINANCE.
///
/// Le meme calcul que le chantier du verrier, essai D, a deux differences pres :
/// il lit la luminance et non le vert, et il se rend pour n'importe quelle
/// direction. Rend (plus grand ecart sur le bord, a d dans [-6 ; -0,45], plus
/// grand ecart au coeur, a d < -14).
fn battement_de_nuit(v: &Verre, d: &Direction, a: &[u8], b: &[u8]) -> (f64, f64) {
    let pa = v.rendre(a, d);
    let pb = v.rendre(b, d);
    let (mut bord, mut coeur) = (0.0f64, 0.0f64);
    for y in 0..ih() {
        for x in 0..il() {
            let dd = distance(x as f32, y as f32) as f64;
            if dd > -0.45 {
                continue;
            }
            let i = ((y * il() + x) * 4) as usize;
            let e = (niveau_luminance(pa[i], pa[i + 1], pa[i + 2])
                - niveau_luminance(pb[i], pb[i + 1], pb[i + 2]))
            .abs();
            if dd >= -6.0 {
                bord = bord.max(e);
            } else if dd < -(CONGE as f64 + 2.0) {
                coeur = coeur.max(e);
            }
        }
    }
    (bord, coeur)
}

/// Les dix-neuf fonds sur lesquels la garantie de `MATIERE.md` §5.2 se lit
/// (essai 8.H) : neuf scenes, puis dix aplats. L'essai de la lampe lit les
/// memes, dans le meme ordre.
fn fonds_de_la_garantie() -> Vec<(&'static str, Vec<u8>)> {
    let couleurs: [(&str, (u8, u8, u8)); 10] = [
        ("gris 226", (226, 226, 226)),
        ("blanc 255", (255, 255, 255)),
        ("rouge", (230, 40, 40)),
        ("vert", (40, 200, 60)),
        ("bleu", (40, 60, 230)),
        ("jaune", (240, 220, 40)),
        ("magenta", (220, 50, 220)),
        ("cyan", (40, 210, 220)),
        ("bande indigo", (0x40, 0x30, 0xC0)),
        ("bande azur", (0x30, 0xA0, 0xE0)),
    ];
    let mut fonds: Vec<(&'static str, Vec<u8>)> = vec![
        ("nuit", fond_horloge(false)),
        ("coupe", fond_coupe()),
        ("bureau raye colore", fond_bureau()),
        ("bureau reel", fond_bureau_reel()),
        ("bureau gris", fond_bureau_gris()),
        ("bureau sombre", fond_bureau_sombre()),
        ("peigne colore", fond_peigne((0x40, 0x30, 0xC0))),
        ("peigne gris", fond_peigne((0xE0, 0xE0, 0xE0))),
        ("page de texte", fond_page_texte()),
    ];
    for (nom, c) in couleurs {
        fonds.push((nom, uni_rvb(c)));
    }
    fonds
}

/// L'essai de la lampe d'ecran (`DECISION-APRES-EPREUVE.md` E2), au banc,
/// avant l'integration : les temoins du banc, chacun a reponse connue d'avance,
/// et l'epreuve qui double L (`mesure lampe epreuve`) et doit faire tomber les
/// temoins du noir et du blanc, et eux seuls.
fn lampe(v: &Verre, j: &mut Journal, epreuve: bool) -> bool {
    dire!(j, "");
    dire!(j, "-- I. LA LAMPE D'ECRAN : ce que le verre porte de lui-meme, a toute heure --");
    dire!(j, "L'auteur : « la lumiere n'est pas necessairement physique [...] ce n'est pas");
    dire!(j, "une question de lumiere physique mais de ce qui est de l'ecran ». La lampe");
    dire!(j, "est un ciel blanc au-dessus de l'ecran, dont le chant porte le reflet : poids");
    dire!(j, "(1 - n_y) / 2 sur la normale sortante du bord, profil de la reflectance de");
    dire!(j, "Fresnel moyennee sur l'empreinte, normalisee par celle du pixel du bord et");
    dire!(j, "ramenee a zero au pied du conge, combinee au reflet de l'ecran par le plus");
    dire!(j, "grand des deux en luminance, jamais par la somme. Son intensite L est le");
    dire!(j, "niveau que le bord droit lit sur du blanc le jour, a la force retenue.");
    let mut ok = true;
    geometrie::deploye();
    let plafond = encre_la_moins_lue();
    let seuil = plafond - TOLERANCE_ARRONDI;
    let mut avec = arbitree_lampe();
    if epreuve {
        avec.lampe *= 2.0;
    }
    let sans = Direction {
        lampe: 0.0,
        ..avec
    };
    let niveau_l = 255.0 * vers_srgb(avec.lampe as f64);
    let niveau_cote = 255.0 * vers_srgb(avec.lampe as f64 * 0.5);
    dire!(j, "");
    dire!(
        j,
        "  force {:.2} ; L = {:.4} en luminance lineaire, soit le niveau {niveau_l:.2}{}",
        avec.lisere,
        avec.lampe,
        if epreuve { " — EPREUVE : L est double" } else { "" }
    );
    dire!(j, "  plafond {plafond:.2}, seuil avec la marge d'un arrondi {seuil:.2}");

    // -- TEMOIN I.1 : L = 0 rend l'ARBITREE --
    dire!(j, "");
    dire!(j, "-- TEMOIN I.1 : la lampe a zero rend l'ARBITREE, a l'octet, sur les dix-neuf fonds --");
    let fonds = fonds_de_la_garantie();
    let differents = |a: &[u8], b: &[u8]| a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p[..3] != q[..3]).count();
    let mut pire_identite = 0usize;
    let mut detail = String::new();
    for (nom, f) in &fonds {
        let n = differents(&v.rendre(f, &ARBITREE), &v.rendre(f, &sans));
        detail += &format!(" {nom} {n} ;");
        pire_identite = pire_identite.max(n);
    }
    dire!(j, "  pixels differents, fond par fond :{detail}");
    dire!(j, "  plus grand nombre de pixels differents sur un fond : {pire_identite}");
    ok &= verdict(
        j,
        pire_identite == 0,
        "la lampe a zero est l'ARBITREE, a l'octet, sur les dix-neuf fonds. Ce\n  qui suit ne mesure que la lampe.",
        "la lampe a zero n'est pas l'ARBITREE : le terme ajoute ne s'eteint pas,\n  et rien de ce qui suit ne se lit.",
    );

    // -- LE NIVEAU DU JOUR, refait ici : le bord droit sur du blanc, sans lampe --
    dire!(j, "");
    dire!(j, "-- L'INTENSITE, refaite par un autre chemin : le bord droit sur du blanc, sans lampe --");
    let blanc = uni(255);
    let jour = pic_du_bord_droit(&v.rendre(&blanc, &sans));
    dire!(
        j,
        "  bord droit sur le blanc, force {:.2}, banc : {jour:.2} ; application, echelle 1 : {JOUR_ECHELLE_1:.2} ; L : {LAMPE_NIVEAU:.2}, le plus bas des neuf echelles",
        sans.lisere
    );
    ok &= verdict(
        j,
        (jour - JOUR_ECHELLE_1).abs() <= TOLERANCE_ARRONDI && LAMPE_NIVEAU <= jour + TOLERANCE_ARRONDI,
        "le banc et l'application lisent le meme bord droit sur le blanc a\n  l'echelle 1, a un octet pres, et L est sous lui : sur du blanc, la lampe\n  ne peut pas gagner ici.",
        "le banc et l'application ne lisent pas le meme bord droit sur le blanc,\n  ou L est au-dessus de lui : L ne derive pas du niveau du jour.",
    );

    // -- TEMOIN I.2 : noir pur, bord haut, premier pixel --
    dire!(j, "");
    dire!(j, "-- TEMOIN I.2 : sur du noir pur, le bord haut lit L, les cotes L/2, le bord bas rien --");
    let noir = uni(0);
    let pn = v.rendre(&noir, &avec);
    let lire = |px: &[u8], x: u32, y: u32| {
        let i = ((y * il() + x) * 4) as usize;
        niveau_luminance(px[i], px[i + 1], px[i + 2])
    };
    let haut = lire(&pn, il() / 2, 0);
    let cote = lire(&pn, 0, ih() / 2);
    let bas = lire(&pn, il() / 2, ih() - 1);
    dire!(
        j,
        "  bord haut {haut:.2} (attendu {niveau_l:.2}), cote gauche {cote:.2} (attendu {niveau_cote:.2}), bord bas {bas:.2} (attendu 0)"
    );
    let mut profil = String::new();
    let mut saut_max = 0.0f64;
    let mut precedent = haut;
    for y in 0..24 {
        let n = lire(&pn, il() / 2, y);
        profil += &format!(" {n:.1}");
        if y >= 4 {
            saut_max = saut_max.max((precedent - n).abs());
        }
        precedent = n;
    }
    dire!(j, "  profil du bord haut sur du noir, colonne du milieu, 24 pixels :{profil}");
    dire!(j, "  plus grand saut entre deux pixels voisins au-dela du quatrieme : {saut_max:.2} niveau(x)");
    let coin_noir = pic_du_coin(&pn);
    dire!(j, "  pire pixel du coin sur du noir : {coin_noir:.2} ; pire du bord droit : {:.2}", pic_du_bord_droit(&pn));
    ok &= verdict(
        j,
        (haut - niveau_l).abs() <= TOLERANCE_ARRONDI && haut <= seuil && bas == 0.0 && (cote - niveau_cote).abs() <= TOLERANCE_ARRONDI,
        "sur du noir, le bord haut lit L a un octet pres et sous le seuil, les\n  cotes la moitie, le bord bas rien : la lampe est un ciel, pas un anneau.",
        "sur du noir, le bord ne lit pas ce que la lampe promet, ou depasse le\n  seuil.",
    );

    // -- TEMOIN I.3 : blanc pur, rien de nouveau --
    dire!(j, "");
    dire!(j, "-- TEMOIN I.3 : sur du blanc pur, la lampe ne gagne nulle part : identique a l'ARBITREE --");
    let pa = v.rendre(&blanc, &ARBITREE);
    let pb = v.rendre(&blanc, &avec);
    let n_blanc = differents(&pa, &pb);
    dire!(
        j,
        "  pixels differents : {n_blanc} ; bord droit {:.2} contre {:.2}, coin {:.2} contre {:.2}",
        pic_du_bord_droit(&pa),
        pic_du_bord_droit(&pb),
        pic_du_coin(&pa),
        pic_du_coin(&pb)
    );
    ok &= verdict(
        j,
        n_blanc == 0,
        "sur du blanc, le reflet de l'ecran l'emporte partout : le jour ne change\n  pas d'un octet.",
        "sur du blanc, la lampe gagne quelque part : le jour a change.",
    );

    // -- TEMOIN I.4 : la garantie, dix-neuf fonds, avec la lampe --
    dire!(j, "");
    dire!(j, "-- TEMOIN I.4 : la borne du bord tient sur les dix-neuf fonds, lampe comprise --");
    dire!(j, "  fond                    bord droit     coin");
    let mut pire = (0.0f64, "");
    for (nom, f) in &fonds {
        let px = v.rendre(f, &avec);
        let (d, c) = (pic_du_bord_droit(&px), pic_du_coin(&px));
        dire!(j, "  {nom:<22} {d:>9.2} {c:>9.2}");
        if d.max(c) > pire.0 {
            pire = (d.max(c), nom);
        }
    }
    dire!(j, "  pire : {:.2} ({}) pour un seuil de {seuil:.2}", pire.0, pire.1);
    ok &= verdict(
        j,
        pire.0 <= seuil,
        "la borne tient sur les dix-neuf fonds avec la lampe : le maximum de deux\n  termes qui tiennent chacun tient.",
        "la borne casse sur un fond avec la lampe.",
    );

    // -- TEMOIN I.5 : la sonde --
    dire!(j, "");
    dire!(j, "-- TEMOIN I.5 : le champ de deplacement ne bouge pas (lampe coupee pour la lecture) --");
    let (_, ca) = v.champ_de(&ARBITREE, 1.0, 0.0);
    let (_, cb) = v.champ_de(&avec, 1.0, 0.0);
    let ecart = ca
        .iter()
        .zip(cb.iter())
        .map(|(p, q)| (0..3).map(|k| (p[k] - q[k]).abs()).fold(0.0f64, f64::max))
        .fold(0.0f64, f64::max);
    dire!(j, "  plus grand ecart du champ, en pixels : {ecart:.4} (la lecture coupe la lampe : l'egalite est de construction, et elle est verifiee)");
    ok &= verdict(
        j,
        ecart == 0.0,
        "la sonde lit le meme deplacement avec et sans lampe.",
        "la sonde lit un autre deplacement avec la lampe : la coupure ne coupe pas.",
    );

    // -- Les planches --
    for (nom, f) in [("noir", &noir), ("blanc", &blanc), ("bureau-reel", &fond_bureau_reel())] {
        ecrire_png(&format!("captures/mesure/lampe-{nom}-avec.png"), &v.rendre(f, &avec), il(), ih());
        ecrire_png(&format!("captures/mesure/lampe-{nom}-sans.png"), &v.rendre(f, &sans), il(), ih());
    }
    dire!(j, "");
    dire!(j, "  planches ecrites dans captures/mesure/lampe-<fond>-avec.png et -sans.png");
    ok
}

/// L'essai 8.H : les trois arbitrages que le troisieme tour a laisses ouverts,
/// mesures la ou les chiffres des partisans ne suffisaient pas.
fn arbitrage(v: &Verre, j: &mut Journal) -> bool {
    dire!(j, "");
    dire!(j, "-- H. L'ARBITRAGE : LA FORCE, LA COUCHE NETTE, LA COULEUR --");
    dire!(j, "L'auteur a confie la decision avec un critere : « le rendu optimal,");
    dire!(j, "realiste et beau ». Trois questions restaient ouvertes. Cet essai");
    dire!(j, "mesure ce qui manquait pour les trancher : la force derivee sur TOUS");
    dire!(j, "les fonds du banc, blanc pur et page de texte compris, avec la marge");
    dire!(j, "d'un arrondi ; puis les quatre directions nommees cote a cote, nuit");
    dire!(j, "comprise. Le raisonnement est dans DECISION.md.");
    let mut ok = true;
    geometrie::deploye();
    let plafond = encre_la_moins_lue();
    let forces: Vec<f32> = (20..=30).map(|k| k as f32 / 100.0).collect();

    // -- TEMOIN 8.13 --
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.13 : le premier pixel du blanc pur, predit a chaque force --");
    dire!(j, "Le balayage qui suit se decide sur le blanc. Sur un aplat, la refraction");
    dire!(j, "et la couche nette ne changent rien, et `niveau_verrier_rvb` predit les");
    dire!(j, "trois octets du premier pixel de zero, force comprise. Deux arrondis");
    dire!(j, "d'une meme valeur different d'un octet au plus : c'est la tolerance.");
    dire!(j, "");
    dire!(j, "  force   predit (vert)   lu (vert)   ecart");
    dire!(j, "  -----------------------------------------");
    let blanc = uni_rvb((255, 255, 255));
    let mut pire_blanc = 0i32;
    for f in &forces {
        let d = Direction { lisere: *f, ..RETENUE };
        let px = v.rendre(&blanc, &d);
        let i = ((il() / 2) * 4) as usize;
        let lu = [px[i], px[i + 1], px[i + 2]];
        let predit = niveau_verrier_rvb(-0.5, (255, 255, 255), &a_la_taille(&d));
        let ecart = (0..3)
            .map(|k| (lu[k] as i32 - predit[k] as i32).abs())
            .max()
            .unwrap();
        pire_blanc = pire_blanc.max(ecart);
        dire!(j, "  {f:>5.2} {:>11} {:>11} {ecart:>7}", predit[1], lu[1]);
    }
    ok &= verdict(
        j,
        pire_blanc <= 1,
        "les onze premiers pixels du blanc tombent sur leur prediction a un\n  octet pres. Le balayage lit donc la force qu'il pretend balayer.",
        "un premier pixel du blanc s'ecarte de sa prediction. Le balayage de\n  la force ne vaut rien.",
    );

    // -- LA FORCE, DERIVEE SUR TOUS LES FONDS --
    dire!(j, "");
    dire!(j, "-- LA FORCE QUE LA GARANTIE LAISSE, SUR TOUS LES FONDS DU BANC --");
    dire!(j, "L'essai D ne lisait que quatre scenes, sans les coins ; l'essai F lisait");
    dire!(j, "le blanc seul. On lit ici, au centieme, le PIRE de tous les fonds que le");
    dire!(j, "banc sait construire, bord droit ET coin. Le seuil n'est pas le plafond");
    dire!(j, "nu : un pixel qui y tient a moins d'un octet ne tient que par hasard");
    dire!(j, "d'arrondi (temoins 8.6, 8.9 et 8.13 : deux arrondis d'une meme valeur");
    dire!(j, "different d'un octet). La marge exigee est donc TOLERANCE_ARRONDI.");
    let fonds = fonds_de_la_garantie();
    let seuil = plafond - TOLERANCE_ARRONDI;
    dire!(j, "");
    dire!(
        j,
        "  {} fonds. Plafond {plafond:.2}, seuil avec la marge d'un arrondi {seuil:.2}.",
        fonds.len()
    );
    dire!(j, "");
    dire!(j, "  force   pire bord droit (fond)          pire coin (fond)             marge   verdict");
    dire!(j, "  -------------------------------------------------------------------------------------");
    let lire_garantie = |d: &Direction| {
        let (mut droit, mut coin) = ((0.0f64, ""), (0.0f64, ""));
        for (nom, f) in &fonds {
            let px = v.rendre(f, d);
            let a = pic_du_bord_droit(&px);
            let b = pic_du_coin(&px);
            if a > droit.0 {
                droit = (a, *nom);
            }
            if b > coin.0 {
                coin = (b, *nom);
            }
        }
        (droit, coin)
    };
    let mut force_derivee: Option<f32> = None;
    let mut encore = true;
    for f in &forces {
        let ((dr, fdr), (co, fco)) = lire_garantie(&Direction { lisere: *f, ..RETENUE });
        let pire = dr.max(co);
        let tient = pire <= seuil;
        if tient && encore {
            force_derivee = Some(*f);
        } else {
            encore = false;
        }
        dire!(
            j,
            "  {f:>5.2} {dr:>8.1} ({fdr:<20}) {co:>8.1} ({fco:<20}) {:>+6.1}   {}",
            plafond - pire,
            if tient { "TIENT" } else { "CASSE" }
        );
    }
    dire!(j, "");
    match force_derivee {
        Some(f) => {
            dire!(j, "  La plus grande force balayee qui tient sur tous les fonds, coins compris");
            dire!(j, "  et avec la marge d'un arrondi, est {f:.2}.");
            dire!(
                j,
                "  FORCE_ARBITREE vaut {FORCE_ARBITREE:.2} : {}",
                if (f - FORCE_ARBITREE).abs() < 1e-6 {
                    "c'est la force derivee."
                } else {
                    "ce n'est pas celle que ce banc derive a l'echelle 1 sur son\n  ruban ; c'est celle que l'application derive sur sa couverture exacte aux\n  quatre echelles (INTEGRATION-APRES-EPREUVE.md, pas 1), et c'est elle qui vaut."
                }
            );
        }
        None => dire!(j, "  Aucune force balayee ne tient sur tous les fonds, pas meme 0,20."),
    }

    // -- TEMOIN 8.14 --
    let directions: [&Direction; 5] = [&V0, &RETENUE, &PASSE_BAS, &ARBITREE, &FLOU_FORT];
    let nom_de = |d: &Direction| if d.nom == V0.nom { "verre seul" } else { d.nom };
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.14 : a la place par defaut, sur du noir, rien ne s'allume --");
    dire!(j, "DESIGN.md §6 pose l'ilot dans la zone vide au-dessus de la date. Sur un");
    dire!(j, "noir pur, la reflexion vaut R fois zero, la refraction deplace du noir,");
    dire!(j, "le passe-bas floute du noir et la teinte multiplie des octets nuls :");
    dire!(j, "chaque direction doit rendre 0 sur chaque pixel du galet.");
    dire!(j, "");
    let noir = uni(0);
    let mut pire_noir = 0u8;
    for d in directions {
        let px = v.rendre(&noir, d);
        let mut m = 0u8;
        for y in 0..ih() {
            for x in 0..il() {
                if distance(x as f32, y as f32) > -0.45 {
                    continue;
                }
                let i = ((y * il() + x) * 4) as usize;
                m = m.max(px[i]).max(px[i + 1]).max(px[i + 2]);
            }
        }
        dire!(j, "  {:<12} plus grand octet dans le galet : {m}", nom_de(d));
        pire_noir = pire_noir.max(m);
    }
    ok &= verdict(
        j,
        pire_noir == 0,
        "les cinq directions rendent un galet entierement noir sur du noir. La\n  nuit, a la place par defaut, aucun des trois arbitrages ne touche un\n  seul pixel.",
        "une direction allume un pixel sur du noir pur. Elle invente de la\n  lumiere, ou l'instrument lit hors du galet.",
    );

    // -- TEMOIN 8.15, et le battement --
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.15 : le battement de nuit, lu en luminance, et son coeur --");
    dire!(j, "L'ilot pose sur la rangee des chiffres, deux secondes consecutives, un fut");
    dire!(j, "deplace de 14 px. Le COEUR, a plus de 14 px du bord, est hors de tout");
    dire!(j, "conge, de toute reflexion et de tout passe-bas : il doit battre de la");
    dire!(j, "meme valeur, a un niveau pres, pour les cinq directions. S'il bouge,");
    dire!(j, "l'ecart lu au bord n'est pas un ecart de bord.");
    dire!(j, "");
    dire!(j, "  direction      battement du BORD   battement du COEUR");
    dire!(j, "  -----------------------------------------------------");
    let nuit_a = fond_horloge(false);
    let nuit_b = fond_horloge(true);
    let mut battements = Vec::new();
    for d in directions {
        let (bo, co) = battement_de_nuit(v, d, &nuit_a, &nuit_b);
        dire!(j, "  {:<12} {bo:>15.1} niv {co:>16.1} niv", nom_de(d));
        battements.push((bo, co));
    }
    let derive_coeur = battements
        .iter()
        .map(|b| (b.1 - battements[0].1).abs())
        .fold(0.0f64, f64::max);
    ok &= verdict(
        j,
        derive_coeur <= TOLERANCE_ARRONDI,
        "le coeur bat de la meme valeur pour les cinq directions. Ce que la\n  colonne du bord montre au-dessus de lui est donc du bord.",
        "le coeur ne bat pas pareil d'une direction a l'autre. La colonne du\n  bord ne se lit pas.",
    );

    // -- LES QUATRE DIRECTIONS COTE A COTE --
    dire!(j, "");
    dire!(j, "-- LES QUATRE DIRECTIONS NOMMEES, ET LE VERRE SEUL --");
    dire!(j, "Chaque grandeur est celle d'un essai deja valide, reprise sur les memes");
    dire!(j, "lecteurs : garantie de §5.2 (tous les fonds ci-dessus, coins compris),");
    dire!(j, "eclat du lisere (ce que le premier pixel ajoute au verre seul sur un");
    dire!(j, "gris 226, replie et deploye), cible §5.4 (pire des trois tailles), dents");
    dire!(j, "le long du bord (8.12) et saturation de la bande (8.B).");
    let gris226 = uni(226);
    let bureau_gris = fond_bureau_gris();
    let bureau_colore = fond_bureau();
    let peigne_gris = fond_peigne((0xE0, 0xE0, 0xE0));
    let page = fond_page_texte();
    let reel = fond_bureau_reel();
    let mut eclat_verre = [0.0f64; 2];
    struct Ligne {
        nom: &'static str,
        droit: f64,
        coin: f64,
        eclat: [f64; 2],
        s54: f64,
        sous_le_nu: bool,
        dents: [f64; 4],
        sat: f64,
    }
    let mut lignes: Vec<Ligne> = Vec::new();
    for d in directions {
        let ((droit, _), (coin, _)) = lire_garantie(d);
        let mut eclat = [0.0f64; 2];
        let mut cols = Vec::new();
        let mut sous_le_nu = true;
        for (k, (_, l, h, r, n)) in ETATS.iter().enumerate() {
            geometrie::poser(*l, *h, *r, *n);
            let px = v.rendre(&gris226, d);
            let e = pic_du_bord_droit(&px);
            if k == 0 {
                eclat[0] = e;
            }
            if k == 2 {
                eclat[1] = e;
            }
            let profond = -(*h as f64 / 2.0 - 2.0);
            let nu = hf_du_fond(&bureau_gris, -8.0, -1.0);
            let pg = v.rendre(&bureau_gris, d);
            let bord = hf_par_distance(&pg, -8.0, -1.0);
            let coeur = hf_par_distance(&pg, profond, profond + 7.0);
            cols.push(if coeur > 1e-6 { bord / coeur } else { f64::INFINITY });
            sous_le_nu &= bord <= nu;
        }
        geometrie::deploye();
        let s54 = cols.iter().cloned().fold(f64::INFINITY, f64::min);
        let pc = v.rendre(&bureau_colore, d);
        let (_, sat) = bande_du_haut(&pc, -5.0, -0.45);
        let dents = [
            dents_le_long_du_bord(&pc, -5.0, -0.45),
            dents_le_long_du_bord(&v.rendre(&peigne_gris, d), -5.0, -0.45),
            dents_le_long_du_bord(&v.rendre(&page, d), -5.0, -0.45),
            dents_le_long_du_bord(&v.rendre(&reel, d), -5.0, -0.45),
        ];
        if d.nom == V0.nom {
            eclat_verre = eclat;
        }
        lignes.push(Ligne {
            nom: nom_de(d),
            droit,
            coin,
            eclat,
            s54,
            sous_le_nu,
            dents,
            sat,
        });
    }
    dire!(j, "");
    dire!(j, "                 garantie, tous fonds        eclat sur 226       §5.4");
    dire!(j, "  direction      droit    coin   marge     replie   deploye     pire   <= nu");
    dire!(j, "  ---------------------------------------------------------------------------");
    for l in &lignes {
        dire!(
            j,
            "  {:<12} {:>7.1} {:>7.1} {:>+7.1} {:>+10.1} {:>+9.1} {:>8.1}   {}",
            l.nom,
            l.droit,
            l.coin,
            plafond - l.droit.max(l.coin),
            l.eclat[0] - eclat_verre[0],
            l.eclat[1] - eclat_verre[1],
            l.s54,
            if l.sous_le_nu { "oui" } else { "NON" }
        );
    }
    dire!(j, "");
    dire!(j, "                 dents le long du bord, bande 0-5 px          sat.     nuit");
    dire!(j, "  direction      raye    peigne    page    bureau reel     raye    battement");
    dire!(j, "  ---------------------------------------------------------------------------");
    for (l, b) in lignes.iter().zip(battements.iter()) {
        dire!(
            j,
            "  {:<12} {:>5.1} {:>9.1} {:>7.1} {:>13.1} {:>8.0} {:>9.1} niv",
            l.nom,
            l.dents[0],
            l.dents[1],
            l.dents[2],
            l.dents[3],
            l.sat,
            b.0
        );
    }

    // -- LES CIBLES DE LA DIRECTION ARBITREE --
    dire!(j, "");
    dire!(j, "-- CE QUE L'ARBITREE DOIT TENIR --");
    let a = lignes.iter().find(|l| l.nom == ARBITREE.nom).unwrap();
    let cibles = [
        (
            "§5.2 garantie, tous fonds, coins, marge d'un arrondi",
            a.droit.max(a.coin) <= seuil,
        ),
        ("§5.4 bande >= 10 x coeur aux trois tailles", a.s54 >= 10.0),
        ("§5.4 bande <= fond nu aux trois tailles", a.sous_le_nu),
        (
            "le lisere reste visible au deploye (eclat > 1 octet)",
            a.eclat[1] - eclat_verre[1] > TOLERANCE_ARRONDI,
        ),
        ("nuit, place par defaut : galet noir sur noir", pire_noir == 0),
    ];
    for (nom, tient) in cibles {
        dire!(j, "  {nom:<56} {}", if tient { "TIENT" } else { "CASSE" });
    }
    ok
}

/// Le niveau que la couche nette, SEULE a l'image, doit rendre au pixel de
/// cible (x, y) sur un fond qui ne depend que de x, apres un passe-bas
/// d'ecart-type `sigma` (0 : pas de passe-bas).
///
/// Le calcul recopie la chaine dans l'ordre. La passe horizontale moyenne la
/// source en LINEAIRE avec les poids gaussiens `exp(-k²/2σ²)` sur `ceil(3σ)`
/// pixels de chaque cote, puis l'ecrit en sRGB 8 bits. La passe verticale ne
/// change rien a une image qui ne depend pas de y. La composition pose ensuite
/// la teinte sur les octets encodes. Sur un gris, la vibrance ne fait rien.
fn nette_predite(fond: &[u8], x: u32, y: u32, sigma: f32, teinte: f64) -> u8 {
    let (sx, sy) = (x + MARGE, y + MARGE);
    let octet_source = |xx: u32| fond[((sy * SL + xx) * 4) as usize];
    let stocke = if sigma > 0.0 {
        let r = (sigma * 3.0).ceil() as i32;
        let (mut somme, mut poids) = (0.0f64, 0.0f64);
        for k in -r..=r {
            let w = (-0.5 * (k * k) as f64 / (sigma as f64 * sigma as f64)).exp();
            let xx = (sx as i32 + k).clamp(0, SL as i32 - 1) as u32;
            somme += lineaire(octet_source(xx)) * w;
            poids += w;
        }
        octet(somme / poids)
    } else {
        octet_source(sx)
    };
    octet(depuis_srgb(vers_srgb(lineaire(stocke)) * (1.0 - teinte)))
}

/// L'essai 8.G : la case 5 du troisieme tour.
fn peigne_et_lisere(v: &Verre, j: &mut Journal) -> bool {
    dire!(j, "");
    dire!(j, "-- G. LE PEIGNE ET LE LISERE : SE DECOUPLENT-ILS ? --");
    dire!(j, "L'objection 4 de l'esthete conclut que les dents et le lisere sont le");
    dire!(j, "meme phenomene : seule la force nulle eteint les dents, et elle eteint");
    dire!(j, "le lisere avec. Une hypothese a eprouver, pas a croire : les dents");
    dire!(j, "viendraient des HAUTES FREQUENCES de la couche nette ramenees au bord.");
    dire!(j, "La reflexion lit la meme liaison. Une couche nette passee par un");
    dire!(j, "passe-bas devrait donc garder la valeur MOYENNE du lisere et perdre ses");
    dire!(j, "dents. On balaie l'ecart-type du passe-bas sur la RETENUE.");
    dire!(j, "");
    dire!(j, "La RETENUE n'en est pas changee. Une direction nommee, PASSE-BAS, est");
    dire!(j, "posee a cote ; choisir entre elles est une decision d'auteur.");
    let mut ok = true;
    geometrie::deploye();
    let sigmas = [0.0f32, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0];
    let peigne_gris = fond_peigne((0xE0, 0xE0, 0xE0));

    // -- TEMOIN 8.10 --
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.10 : le passe-bas est bien le flou gaussien qu'on croit --");
    dire!(j, "Une direction qui ne montre QUE la couche nette dans le conge : clarte");
    dire!(j, "pleine en marche, sans refraction, sans reflexion. Sur le peigne gris,");
    dire!(j, "chaque pixel de la bande du bord haut se calcule alors de zero — le");
    dire!(j, "flou gaussien du peigne, ecrit en 8 bits, puis teinte. Deux arrondis");
    dire!(j, "d'une meme valeur different d'un octet au plus : c'est la tolerance.");
    dire!(j, "");
    dire!(j, "  sigma   pixels lus   pire ecart");
    dire!(j, "  -------------------------------");
    let mut pire_nette = 0i32;
    for s in sigmas {
        let d = Direction {
            nom: "couche nette seule",
            largeur: CONGE,
            clarte: 1.0,
            profil: 0.0,
            refraction: 0.0,
            fresnel: 0.0,
            beer: 0.0,
            lisere: 0.0,
            nette_luminance: false,
            vibrance_relative: false,
            conge_relatif: false,
            nette_sigma: s,
            ..V0
        };
        let px = v.rendre(&peigne_gris, &d);
        let r = rayon() as u32;
        let (mut pire, mut n) = (0i32, 0usize);
        for y in 1..11u32 {
            for x in (r + 12)..(il() - r - 12) {
                let i = ((y * il() + x) * 4) as usize;
                let attendu = nette_predite(&peigne_gris, x, y, s, d.teinte as f64);
                pire = pire.max((px[i] as i32 - attendu as i32).abs());
                n += 1;
            }
        }
        dire!(j, "  {s:>5.1} {n:>12} {pire:>12}");
        pire_nette = pire_nette.max(pire);
    }
    ok &= verdict(
        j,
        pire_nette <= 1,
        "a chaque sigma, la couche nette rendue tombe sur le flou gaussien\n  calcule a un octet pres. Le passe-bas est donc celui qu'on croit, et\n  le sigma 0 rend bien la source brute.",
        "la couche nette rendue ne tombe pas sur le flou calcule. Ce que les\n  tableaux suivants appellent sigma ne vaut rien.",
    );

    // -- TEMOIN 8.12 --
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.12 : les dents, separees du profil du lisere --");
    dire!(j, "L'amplitude crete-a-crete de la bande melange les dents, qui varient");
    dire!(j, "LE LONG du bord, et le profil du lisere, qui decroit en s'eloignant de");
    dire!(j, "l'arete. Un second lecteur ne compare que des pixels a la meme");
    dire!(j, "distance. Sur une image posee : un degrade perpendiculaire de 80, puis");
    dire!(j, "des dents de 10 dans une rangee, et deux pieges hors lecture. Reponses");
    dire!(j, "connues : crete-a-crete 80 ; le long du bord 0, puis 10.");
    let (ampl_pose, sans_pose, avec_pose) = dents_posees();
    dire!(j, "");
    dire!(
        j,
        "  crete-a-crete {ampl_pose:.3}   le long du bord, sans dents {sans_pose:.3}, avec {avec_pose:.3}"
    );
    ok &= verdict(
        j,
        (ampl_pose - 80.0).abs() < 1e-6 && sans_pose.abs() < 1e-6 && (avec_pose - 10.0).abs() < 1e-6,
        "le crete-a-crete voit le profil, le second lecteur ne voit que les\n  dents, et aucun des deux pieges n'est lu.",
        "le lecteur des dents ne rend pas les reponses posees. Sa colonne ne\n  vaut rien.",
    );

    // -- LES DENTS, et le premier temoin nul --
    dire!(j, "");
    dire!(j, "-- LES DENTS, bande 0-5 px du bord haut, en luminance --");
    dire!(j, "« crete » est l'amplitude de toute la bande, profil du lisere compris ;");
    dire!(j, "« le long » ne garde que ce qui varie a distance fixe : les dents.");
    dire!(j, "");
    dire!(j, "                        bureau raye colore                  peigne gris");
    dire!(j, "  direction         crete  le long  sat.  COEUR ampl. sat.   crete  le long");
    dire!(j, "  --------------------------------------------------------------------------");
    let bureau = fond_bureau();
    let lire_dents = |d: &Direction| {
        let px = v.rendre(&bureau, d);
        let (ab, sb) = bande_du_haut(&px, -5.0, -0.45);
        let lb = dents_le_long_du_bord(&px, -5.0, -0.45);
        let (ac, sc) = bande_du_haut(&px, -40.0, -30.0);
        let pg = v.rendre(&peigne_gris, d);
        let (ag, _) = bande_du_haut(&pg, -5.0, -0.45);
        let lg = dents_le_long_du_bord(&pg, -5.0, -0.45);
        (ab, sb, ac, sc, ag, lb, lg)
    };
    let (vb, vs, vac, vsc, vg, vlb, vlg) = lire_dents(&V0);
    dire!(
        j,
        "  verre seul        {vb:>5.1} {vlb:>8.1} {vs:>5.0} {vac:>11.1} {vsc:>4.0} {vg:>7.1} {vlg:>8.1}"
    );
    let mut coeurs = Vec::new();
    let mut dents = Vec::new();
    for s in sigmas {
        let d = Direction { nette_sigma: s, ..RETENUE };
        let (ab, sb, ac, sc, ag, lb, lg) = lire_dents(&d);
        dire!(
            j,
            "  RETENUE s = {s:>3.1}   {ab:>5.1} {lb:>8.1} {sb:>5.0} {ac:>11.1} {sc:>4.0} {ag:>7.1} {lg:>8.1}"
        );
        coeurs.push((ac, sc));
        dents.push((ab, sb, ag, lb, lg));
    }
    let derive_coeur = coeurs
        .iter()
        .map(|c| (c.0 - coeurs[0].0).abs().max((c.1 - coeurs[0].1).abs()))
        .fold(0.0f64, f64::max);

    // -- TEMOIN 8.11 --
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.11 : ce que le passe-bas ne PEUT pas changer --");
    dire!(j, "Un passe-bas ne change pas une image constante : sur un aplat, chaque");
    dire!(j, "sigma doit rendre la RETENUE a un octet pres, lisere compris. Et il ne");
    dire!(j, "touche pas le coeur, ou la couche nette ne s'allume pas : le coeur du");
    dire!(j, "tableau precedent ne doit pas bouger de plus d'un niveau.");
    dire!(j, "");
    let aplats = [
        ("gris 226", uni(226)),
        ("sombre 37", uni(37)),
        ("jaune", uni_rvb((240, 220, 40))),
    ];
    let mut pire_aplat = 0i32;
    for (nom, f) in &aplats {
        let a = v.rendre(f, &RETENUE);
        let mut pire = 0i32;
        for s in &sigmas[1..] {
            let b = v.rendre(f, &Direction { nette_sigma: *s, ..RETENUE });
            let e = a
                .iter()
                .zip(b.iter())
                .map(|(p, q)| (*p as i32 - *q as i32).abs())
                .max()
                .unwrap();
            pire = pire.max(e);
        }
        dire!(j, "  {nom:<10} pire ecart a la RETENUE, tous sigmas, toute l'image : {pire}");
        pire_aplat = pire_aplat.max(pire);
    }
    dire!(j, "  derive du coeur sur le bureau raye, tous sigmas : {derive_coeur:.1} niv");
    ok &= verdict(
        j,
        pire_aplat <= 1 && derive_coeur <= 1.0,
        "sur un aplat le passe-bas ne change rien, et le coeur ne bouge pas.\n  Ce que les dents perdent plus haut est donc du bord, et rien d'autre.",
        "le passe-bas change un aplat ou le coeur. Il fait autre chose que\n  filtrer la couche nette.",
    );

    // -- LE LISERE --
    dire!(j, "");
    dire!(j, "-- LE LISERE : la garantie de MATIERE.md §5.2, pire des quatre scenes --");
    dire!(j, "");
    dire!(j, "  sigma   bord droit    coin   marge coin");
    dire!(j, "  ---------------------------------------");
    let plafond = encre_la_moins_lue();
    let scenes = [fond_horloge(false), fond_coupe(), fond_bureau(), fond_bureau_reel()];
    let lire_lisere = |d: &Direction| {
        let (mut droit, mut coin) = (0.0f64, 0.0f64);
        for f in &scenes {
            let px = v.rendre(f, d);
            droit = droit.max(pic_du_bord_droit(&px));
            coin = coin.max(pic_du_coin(&px));
        }
        (droit, coin)
    };
    let mut liseres = Vec::new();
    for s in sigmas {
        let (droit, coin) = lire_lisere(&Direction { nette_sigma: s, ..RETENUE });
        dire!(j, "  {s:>5.1} {droit:>11.1} {coin:>7.1} {:>+11.1}", plafond - coin);
        liseres.push((droit, coin));
    }

    // -- LA CIBLE §5.4, qui borne le passe-bas --
    dire!(j, "");
    dire!(j, "-- CE QUE LE PASSE-BAS COUTE A LA CIBLE DE MATIERE.md §5.4 --");
    dire!(j, "Le passe-bas ne se regle pas a l'oeil. La couche nette existe pour que");
    dire!(j, "la refraction se voie, et §5.4 le chiffre : l'energie de structure de la");
    dire!(j, "bande 1-8 px doit valoir au moins DIX fois celle du centre, et jamais");
    dire!(j, "plus que le fond nu. On garde le plus grand sigma balaye qui tient les");
    dire!(j, "deux aux TROIS tailles, sans qu'un sigma plus petit ait echoue.");
    dire!(j, "");
    dire!(j, "  fond de bureau raye, achromatique : rapport bande / centre");
    dire!(j, "  sigma     replie   mi-chemin     deploye   bande <= fond nu   verdict");
    dire!(j, "  --------------------------------------------------------------------");
    let gris = fond_bureau_gris();
    let mut sigma_derive: Option<f32> = None;
    let mut encore = true;
    let mut rapports_54 = Vec::new();
    for s in sigmas {
        let d = Direction { nette_sigma: s, ..RETENUE };
        let mut cols = Vec::new();
        let mut sous_le_nu = true;
        for (_, l, h, r, n) in ETATS {
            geometrie::poser(l, h, r, n);
            let profond = -(h as f64 / 2.0 - 2.0);
            let nu = hf_du_fond(&gris, -8.0, -1.0);
            let px = v.rendre(&gris, &d);
            let bord = hf_par_distance(&px, -8.0, -1.0);
            let coeur = hf_par_distance(&px, profond, profond + 7.0);
            cols.push(if coeur > 1e-6 { bord / coeur } else { f64::INFINITY });
            sous_le_nu &= bord <= nu;
        }
        geometrie::deploye();
        let pire = cols.iter().cloned().fold(f64::INFINITY, f64::min);
        let tient = pire >= 10.0 && sous_le_nu;
        if tient && encore {
            sigma_derive = Some(s);
        } else {
            encore = false;
        }
        dire!(
            j,
            "  {s:>5.1} {:>10.1} {:>11.1} {:>11.1}   {:>16}   {}",
            cols[0],
            cols[1],
            cols[2],
            if sous_le_nu { "oui" } else { "NON" },
            if tient { "TIENT" } else { "CASSE" }
        );
        rapports_54.push(pire);
    }
    dire!(j, "");
    match sigma_derive {
        Some(s) => {
            dire!(j, "  Le plus grand sigma balaye qui garde §5.4 aux trois tailles est {s:.1} px.");
            dire!(
                j,
                "  SIGMA_PASSE_BAS vaut {SIGMA_PASSE_BAS:.1} px : {}",
                if (s - SIGMA_PASSE_BAS).abs() < 1e-6 {
                    "c'est le sigma derive."
                } else {
                    "ce N'EST PAS le sigma derive, et la direction\n  PASSE-BAS ne vaut rien tant qu'elle n'y a pas ete remise."
                }
            );
        }
        None => dire!(j, "  Aucun sigma balaye ne garde §5.4 aux trois tailles, pas meme 0."),
    }

    // -- LA DIRECTION NOMMEE, a cote de la RETENUE --
    dire!(j, "");
    dire!(j, "-- PASSE-BAS A COTE DE LA RETENUE, et du verre seul --");
    dire!(j, "");
    dire!(j, "Les dents et le lisere de PASSE-BAS sont rendus sous son propre nom ;");
    dire!(j, "sa colonne §5.4 est la ligne du balayage a son sigma, qui est la meme");
    dire!(j, "direction champ pour champ.");
    dire!(j, "");
    dire!(j, "                   dents, le long du bord     crete    lisere, 4 scenes    §5.4");
    dire!(j, "  direction          raye    gris    sat.     raye     droit    coin      pire");
    dire!(j, "  -------------------------------------------------------------------------------");
    let (vld, vlc) = lire_lisere(&V0);
    dire!(
        j,
        "  verre seul       {vlb:>6.1} {vlg:>7.1} {vs:>7.0} {vb:>8.1} {vld:>9.1} {vlc:>7.1}         —"
    );
    let (rb, rs, _, _, rg, rlb, rlg) = lire_dents(&RETENUE);
    let _ = rg;
    let (rld, rlc) = lire_lisere(&RETENUE);
    dire!(
        j,
        "  RETENUE          {rlb:>6.1} {rlg:>7.1} {rs:>7.0} {rb:>8.1} {rld:>9.1} {rlc:>7.1} {:>9.1}",
        rapports_54[0]
    );
    let (pb, ps, _, _, pg, plb, plg) = lire_dents(&PASSE_BAS);
    let _ = pg;
    let (pld, plc) = lire_lisere(&PASSE_BAS);
    let k = sigmas
        .iter()
        .position(|s| (*s - SIGMA_PASSE_BAS).abs() < 1e-6);
    let p54 = match k {
        Some(i) => format!("{:>9.1}", rapports_54[i]),
        None => "  non mesure".to_string(),
    };
    dire!(
        j,
        "  PASSE-BAS        {plb:>6.1} {plg:>7.1} {ps:>7.0} {pb:>8.1} {pld:>9.1} {plc:>7.1} {p54}"
    );
    let _ = &dents;
    ok
}

/// Un fond uni d'une couleur quelconque.
fn uni_rvb(c: (u8, u8, u8)) -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for k in 0..(SL * SH) as usize {
        p[4 * k] = c.0;
        p[4 * k + 1] = c.1;
        p[4 * k + 2] = c.2;
    }
    p
}

/// Le premier pixel predit sur un fond uni COLORE, canal par canal.
///
/// `niveau_verrier` suffit sur un gris, parce que la vibrance n'y fait rien. Sur
/// une couleur elle agit, et c'est ce que cette fonction ajoute : le nuanceur
/// pousse l'ecart au gris d'un facteur `saturation` en lineaire, avant la
/// teinte et sans borne. Un canal peut passer sous zero ; c'est l'ecriture dans
/// la cible qui l'ecrete, et `octet` fait de meme. Les conversions sRGB du banc
/// prennent la meme branche que celles du nuanceur, y compris sous zero. Le
/// reste — partage `(1-R)`, reflexion `R` nourrie par le fond, teinte sur les
/// octets — est celui de `niveau_verrier`, recopie pour un canal a la fois.
fn niveau_verrier_rvb(d: f64, fond: (u8, u8, u8), dd: &Direction) -> [u8; 3] {
    let n = dd.indice as f64;
    let r0 = ((n - 1.0) / (n + 1.0)).powi(2);
    let (b, e) = (dd.largeur as f64, dd.epaisseur as f64);
    let (mut transmis, mut chemin, mut part_reflechie) = (1.0 - r0, 1.0, 0.0);
    if b > 0.0 && d > -b - 1.0 {
        let prises = (dd.prises as i32).max(1);
        let (mut st, mut sc, mut sr) = (0.0, 0.0, 0.0);
        for k in 0..prises {
            let dk = (d - 0.5 + (k as f64 + 0.5) / prises as f64).min(0.0);
            let (_, r, portee, ch) = bord_en(dk, b, n, e);
            st += 1.0 - r;
            sc += ch;
            if portee >= 0.0 {
                sr += r;
            }
        }
        let inv = 1.0 / prises as f64;
        transmis = st * inv;
        chemin = sc * inv;
        part_reflechie = sr * inv;
    }
    let tau = if dd.beer > 0.0 {
        1.0 - (1.0 - dd.teinte as f64).powf(chemin)
    } else {
        dd.teinte as f64
    };
    let l = [lineaire(fond.0), lineaire(fond.1), lineaire(fond.2)];
    let gris = 0.2126 * l[0] + 0.7152 * l[1] + 0.0722 * l[2];
    let vibrance = dd.saturation as f64;
    let mut sortie = [0u8; 3];
    for c in 0..3 {
        let sature = gris + vibrance * (l[c] - gris);
        let mut verre = depuis_srgb(vers_srgb(sature) * (1.0 - tau));
        if dd.fresnel > 0.0 {
            verre = verre * transmis + l[c] * part_reflechie * dd.lisere as f64;
        }
        sortie[c] = octet(verre);
    }
    sortie
}

/// Les lecteurs du bord eprouves sur des pixels COLORES poses, a la taille
/// courante. Gris 100 uni ; un rouge pur et un bleu pur au premier rang du
/// bord haut, puis les memes dans la bande. Rend (pic lu, amplitude lue).
fn lecteurs_colores() -> (f64, f64) {
    let poser = |px: &mut Vec<u8>, x: u32, y: u32, c: (u8, u8, u8)| {
        let i = ((y * il() + x) * 4) as usize;
        px[i] = c.0;
        px[i + 1] = c.1;
        px[i + 2] = c.2;
    };
    let base = vec![100u8; (il() * ih() * 4) as usize];
    let milieu = il() / 2;
    let mut a = base.clone();
    poser(&mut a, milieu, 0, (255, 0, 0));
    poser(&mut a, milieu + 4, 0, (0, 0, 255));
    let pic = pic_du_bord_droit(&a);
    let mut b = base;
    poser(&mut b, milieu, 1, (255, 0, 0));
    poser(&mut b, milieu + 4, 1, (0, 0, 255));
    let (ampl, _) = bande_du_haut(&b, -5.0, -0.45);
    (pic, ampl)
}

/// L'essai 8.F : la garantie de `MATIERE.md` §5.2 lue en LUMINANCE, et
/// eprouvee sur des fonds colores.
fn garantie_sur_fond_colore(v: &Verre, j: &mut Journal) -> bool {
    dire!(j, "");
    dire!(j, "-- F. LA GARANTIE DE MATIERE.md §5.2 SUR UN FOND COLORE --");
    dire!(j, "La garantie compare une LUMINANCE : son plafond est le gris de meme");
    dire!(j, "luminance que #5E5B57. Les lecteurs du bord lisaient le seul canal");
    dire!(j, "vert, qui ne vaut la luminance que sur un gris. Ils lisent desormais");
    dire!(j, "la luminance relative, rendue en niveau de gris sRGB — et tous les");
    dire!(j, "tableaux de pic et de bande de ce banc avec eux.");
    let mut ok = true;
    let plafond = encre_la_moins_lue();
    geometrie::deploye();

    // -- TEMOIN 8.8 --
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.8 : la luminance, lue sur des pixels colores poses --");
    dire!(j, "Sur un gris 100 uni : un rouge pur au premier rang du bord haut et un");
    dire!(j, "bleu pur quatre pixels plus loin ; puis les memes dans la bande. Les");
    dire!(j, "reponses ne sortent que des poids de la luminance : pic 255 x");
    dire!(j, "srgb(0,2126), amplitude 255 x (srgb(0,2126) - srgb(0,0722)). Un");
    dire!(j, "lecteur du vert lirait 100 et 100 ; un lecteur du canal le plus fort,");
    dire!(j, "255 et 155.");
    let rouge = 255.0 * vers_srgb(0.2126);
    let bleu = 255.0 * vers_srgb(0.0722);
    let (pic, ampl) = lecteurs_colores();
    dire!(j, "");
    dire!(j, "  pic       : attendu {rouge:>8.3}   lu {pic:>8.3}");
    dire!(j, "  amplitude : attendue {:>7.3}   lue {ampl:>7.3}", rouge - bleu);
    ok &= verdict(
        j,
        (pic - rouge).abs() < 1e-6 && (ampl - (rouge - bleu)).abs() < 1e-6,
        "les deux lecteurs rendent la luminance des pixels colores poses, a\n  l'arrondi flottant pres, et non l'un de leurs canaux.",
        "un lecteur du bord ne rend pas la luminance d'un pixel colore. La\n  garantie ne se lit pas juste sur un fond colore.",
    );

    // -- TEMOIN 8.9 --
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.9 : le premier pixel, predit d'avance sur un aplat colore --");
    dire!(j, "Sur un aplat colore la refraction et la couche nette ne changent");
    dire!(j, "toujours rien, mais la vibrance agit. `niveau_verrier_rvb` la recopie");
    dire!(j, "et predit les trois octets du premier pixel. Deux arrondis d'une meme");
    dire!(j, "valeur different d'un octet au plus, par canal : c'est la tolerance.");
    dire!(j, "");
    dire!(j, "  fond           direction    predit          lu              ecart  lum.   vert");
    dire!(j, "  ---------------------------------------------------------------------------------");
    let couleurs: [(&str, (u8, u8, u8)); 10] = [
        ("gris 226", (226, 226, 226)),
        ("blanc 255", (255, 255, 255)),
        ("rouge", (230, 40, 40)),
        ("vert", (40, 200, 60)),
        ("bleu", (40, 60, 230)),
        ("jaune", (240, 220, 40)),
        ("magenta", (220, 50, 220)),
        ("cyan", (40, 210, 220)),
        ("bande indigo", (0x40, 0x30, 0xC0)),
        ("bande azur", (0x30, 0xA0, 0xE0)),
    ];
    let mut pire_octet = 0i32;
    let mut garantie = Vec::new();
    for (nom, c) in couleurs {
        let fond = uni_rvb(c);
        for (nom_dir, d) in [("verre seul", &V0), ("RETENUE", &RETENUE)] {
            let px = v.rendre(&fond, d);
            let i = ((il() / 2) * 4) as usize;
            let lu = [px[i], px[i + 1], px[i + 2]];
            let predit = niveau_verrier_rvb(-0.5, c, &a_la_taille(d));
            let ecart = (0..3)
                .map(|k| (lu[k] as i32 - predit[k] as i32).abs())
                .max()
                .unwrap();
            pire_octet = pire_octet.max(ecart);
            dire!(
                j,
                "  {nom:<14} {nom_dir:<11} ({:>3},{:>3},{:>3})   ({:>3},{:>3},{:>3})   {ecart:>3}  {:>5.1}  {:>5}",
                predit[0],
                predit[1],
                predit[2],
                lu[0],
                lu[1],
                lu[2],
                niveau_luminance(lu[0], lu[1], lu[2]),
                lu[1]
            );
            if nom_dir == "RETENUE" {
                garantie.push((nom, pic_du_bord_droit(&px), pic_du_coin(&px), lu[1]));
            }
        }
    }
    ok &= verdict(
        j,
        pire_octet <= 1,
        "les vingt premiers pixels tombent sur leur prediction a un octet pres\n  par canal. Le rendu sur un fond colore est donc compris, et sa\n  luminance se lit juste.",
        "un premier pixel colore s'ecarte de sa prediction de plus d'un octet.\n  Tant qu'on ne sait pas lequel des deux a tort, rien ne se tranche.",
    );

    // -- LA MESURE --
    dire!(j, "");
    dire!(j, "  LA GARANTIE, RETENUE, PREMIER PIXEL EN LUMINANCE, contre {plafond:.0} niveaux.");
    dire!(j, "  La colonne « vert » est ce que l'ancien lecteur aurait lu.");
    dire!(j, "");
    dire!(j, "  fond                 bord droit   coin    marge coin   vert   verdict");
    dire!(j, "  ----------------------------------------------------------------------");
    for (nom, scene) in [
        ("bureau raye colore", fond_bureau()),
        ("peigne colore", fond_peigne((0x40, 0x30, 0xC0))),
    ] {
        let px = v.rendre(&scene, &RETENUE);
        let mut vert = 0u8;
        for y in 0..ih() {
            for x in 0..il() {
                let dd = distance(x as f32, y as f32) as f64;
                let dans_coin = ((x as f32) < rayon() || (x as f32) > il() as f32 - rayon())
                    && ((y as f32) < rayon() || (y as f32) > ih() as f32 - rayon());
                if (-1.5..=-0.45).contains(&dd) && !dans_coin {
                    vert = vert.max(px[((y * il() + x) * 4 + 1) as usize]);
                }
            }
        }
        garantie.push((nom, pic_du_bord_droit(&px), pic_du_coin(&px), vert));
    }
    let mut pire_droit = 0.0f64;
    let mut pire_coin = 0.0f64;
    for (nom, droit, coin, vert) in &garantie {
        pire_droit = pire_droit.max(*droit);
        pire_coin = pire_coin.max(*coin);
        dire!(
            j,
            "  {nom:<20} {droit:>8.1} {coin:>8.1} {:>+10.1} {vert:>7}   {}",
            plafond - coin,
            if droit.max(*coin) <= plafond { "TIENT" } else { "CASSE" }
        );
    }
    dire!(j, "");
    dire!(
        j,
        "  pire de ces fonds : {pire_droit:.1} au bord droit, {pire_coin:.1} dans le coin, pour un"
    );
    dire!(j, "  plafond de {plafond:.0}.");

    // -- CE QUE LE BLANC LAISSERAIT --
    dire!(j, "");
    dire!(j, "  LE BLANC PUR n'est dans aucune des quatre scenes de l'essai D, dont");
    dire!(j, "  le fond le plus clair vaut 242. Le meme balayage que l'essai D, sur le");
    dire!(j, "  seul blanc 255 et coins compris, pour dire quelle force la garantie y");
    dire!(j, "  laisserait. La RETENUE n'en est pas changee : c'est a l'auteur.");
    dire!(j, "");
    dire!(j, "  force   bord droit    coin    marge   verdict");
    dire!(j, "  ---------------------------------------------");
    let blanc = uni_rvb((255, 255, 255));
    let mut tenable = 0.0f32;
    for force in [0.00f32, 0.05, 0.10, 0.15, 0.20, 0.22, 0.24, 0.26, 0.28, 0.30] {
        let d = Direction {
            lisere: force,
            ..RETENUE
        };
        let px = v.rendre(&blanc, &d);
        let droit = pic_du_bord_droit(&px);
        let coin = pic_du_coin(&px);
        let pire = droit.max(coin);
        let tient = pire <= plafond;
        if tient && force > tenable {
            tenable = force;
        }
        dire!(
            j,
            "  {force:>5.2} {droit:>10.1} {coin:>9.1} {:>+8.1}   {}",
            plafond - pire,
            if tient { "TIENT" } else { "CASSE" }
        );
    }
    dire!(j, "");
    dire!(
        j,
        "  Sur le blanc pur, coins compris, la plus grande force balayee qui tienne"
    );
    dire!(j, "  est {tenable:.2}.");
    ok
}

/// L'essai 8.E : les grandeurs 8.B, 8.C et 8.D, reprises au galet replie et a
/// mi-course. C'est la reserve de l'objection 2 de l'esthete : l'eclat du
/// lisere et l'amplitude des dents n'avaient ete mesures qu'au deploye.
fn lisere_et_dents_aux_trois_tailles(v: &Verre, j: &mut Journal) -> bool {
    dire!(j, "");
    dire!(j, "-- E. LE LISERE ET LES DENTS AUX TROIS TAILLES --");
    dire!(j, "Les essais B, C et D ne lisent que le galet DEPLOYE, ou le facteur");
    dire!(j, "du conge relatif vaut 1. Rien ne disait que le lisere et les dents");
    dire!(j, "gardent la meme proportion quand le conge tombe a 7,46 px puis a");
    dire!(j, "2,92 px. Les memes lectures, aux trois etats de l'instrument 7.");
    let mut ok = true;
    let plafond = encre_la_moins_lue();
    let dirs: [(&str, &Direction); 3] =
        [("verre seul", &V0), ("ANCIENNE", &ANCIENNE), ("RETENUE", &RETENUE)];

    // -- TEMOIN 8.5 : les deux lecteurs, a chaque taille --
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.5 : les lecteurs du bord, sur une image posee a chaque taille --");
    dire!(j, "Ils choisissent leurs pixels par la distance et par le rayon, qui");
    dire!(j, "changent avec la taille. Sur un gris 100 uni : 200 pose au premier");
    dire!(j, "rang du bord haut, 255 dans le coin, a d = -2,5 et hors du galet ;");
    dire!(j, "160 pose dans la bande, 255 juste avant elle, sous elle et dans la");
    dire!(j, "moitie basse. Reponses exactes : pic 200, amplitude 60, saturation 0.");
    dire!(j, "");
    dire!(j, "  etat                  pic   ampl.   sat.");
    dire!(j, "  -----------------------------------------");
    let mut lecteurs_justes = true;
    for (nom, l, h, r, n) in ETATS {
        geometrie::poser(l, h, r, n);
        let (pic, ampl, sat, coin) = lecteurs_poses();
        dire!(j, "  {nom}  {pic:>5.0} {ampl:>7.0} {sat:>6.0}");
        // Les lecteurs rendent une luminance flottante : sur un gris elle vaut
        // l'octet a l'aller-retour sRGB pres, soit bien moins d'un millionieme.
        lecteurs_justes &=
            coin && (pic - 200.0).abs() < 1e-6 && (ampl - 60.0).abs() < 1e-6 && sat == 0.0;
    }
    ok &= verdict(
        j,
        lecteurs_justes,
        "les deux lecteurs rendent les reponses posees, a chaque taille, sans\n  lire ni le coin, ni l'interieur, ni la moitie basse.",
        "un lecteur du bord se trompe de pixels a une taille au moins. Les\n  tableaux suivants ne valent rien.",
    );

    // -- TEMOIN 8.6 : le lisere predit a chaque taille, sur un aplat --
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.6 : le lisere, predit d'avance a chaque taille --");
    dire!(j, "Sur un fond uni la refraction et la couche nette ne changent rien :");
    dire!(j, "elles deplacent l'echantillon dans une image constante. Le premier");
    dire!(j, "pixel du bord se calcule donc de zero par `niveau_verrier`, avec le");
    dire!(j, "`b` et le `e` que le nuanceur recoit a cette taille — 12 x h / 148");
    dire!(j, "pour la RETENUE, 12 pour l'ANCIENNE. La prediction est ronde a");
    dire!(j, "l'octet, le rendu aussi : deux arrondis d'une meme valeur different");
    dire!(j, "d'un niveau au plus, et c'est la tolerance.");
    dire!(j, "");
    dire!(j, "  etat                fond   direction      b      predit   lu    ecart");
    dire!(j, "  ----------------------------------------------------------------------");
    let mut pire_prediction = 0.0f64;
    // (etat, fond, direction) -> niveau lu, pour le tableau des proportions.
    let mut lu = std::collections::HashMap::new();
    for (k, (nom, l, h, r, n)) in ETATS.iter().enumerate() {
        geometrie::poser(*l, *h, *r, *n);
        for (nom_fond, fond) in [("226", 226u8), (" 37", 37u8)] {
            let image = uni(fond);
            for (nom_dir, d) in dirs {
                let efficace = a_la_taille(d);
                let predit = niveau_verrier(-0.5, fond, &efficace)
                    .max(niveau_verrier(-1.5, fond, &efficace));
                let mesure = pic_du_bord_droit(&v.rendre(&image, d));
                dire!(
                    j,
                    "  {nom}  {nom_fond}   {nom_dir:<12} {:>5.2} {predit:>8.0} {mesure:>5.0} {:>+7.0}",
                    efficace.largeur,
                    mesure - predit
                );
                pire_prediction = pire_prediction.max((mesure - predit).abs());
                lu.insert((k, fond, nom_dir), mesure);
            }
        }
    }
    ok &= verdict(
        j,
        pire_prediction <= 1.0,
        "les dix-huit premiers pixels lus tombent sur leur prediction a un\n  niveau pres. Le lisere est donc lu juste a chaque taille, et la\n  prediction dit ce qui le fait changer : le `b` que recoit le nuanceur.",
        "le lisere lu s'ecarte de sa prediction a une taille au moins. Tant\n  qu'on ne sait pas lequel des deux a tort, rien ne se tranche.",
    );

    // -- LA PROPORTION DU LISERE --
    dire!(j, "");
    dire!(j, "  L'ECLAT DU LISERE : ce que la matiere ajoute au verre seul, sur le");
    dire!(j, "  premier pixel, et sa part de ce qu'elle ajoute au deploye.");
    dire!(j, "");
    dire!(j, "  etat                fond   ANCIENNE   part    RETENUE   part");
    dire!(j, "  ---------------------------------------------------------------");
    for (k, (nom, ..)) in ETATS.iter().enumerate() {
        for fond in [226u8, 37u8] {
            let eclat = |k: usize, dir: &str| lu[&(k, fond, dir)] - lu[&(k, fond, "verre seul")];
            let part = |k: usize, dir: &str| {
                let reference = eclat(2, dir);
                if reference.abs() < 0.5 {
                    f64::NAN
                } else {
                    100.0 * eclat(k, dir) / reference
                }
            };
            dire!(
                j,
                "  {nom}  {fond:>3}   {:>+6.0} niv {:>5.0} %  {:>+6.0} niv {:>5.0} %",
                eclat(k, "ANCIENNE"),
                part(k, "ANCIENNE"),
                eclat(k, "RETENUE"),
                part(k, "RETENUE")
            );
        }
    }

    // -- LA GARANTIE A CHAQUE TAILLE --
    dire!(j, "");
    dire!(j, "  LA GARANTIE DE MATIERE.md §5.2 A CHAQUE TAILLE : la RETENUE, pire des");
    dire!(j, "  quatre scenes de l'essai D, au bord droit et dans le coin, contre");
    dire!(j, "  l'encre la moins lue ({plafond:.0} niveaux).");
    dire!(j, "");
    dire!(j, "  etat                bord droit   marge    coin    marge");
    dire!(j, "  --------------------------------------------------------");
    let scenes = [fond_horloge(false), fond_coupe(), fond_bureau(), fond_bureau_reel()];
    for (nom, l, h, r, n) in ETATS {
        geometrie::poser(l, h, r, n);
        let (mut droit, mut coin) = (0.0f64, 0.0f64);
        for f in &scenes {
            let px = v.rendre(f, &RETENUE);
            droit = droit.max(pic_du_bord_droit(&px));
            coin = coin.max(pic_du_coin(&px));
        }
        dire!(
            j,
            "  {nom}  {droit:>8.0} {:>+8.1} {coin:>7.0} {:>+8.1}",
            plafond - droit,
            plafond - coin
        );
    }

    // -- LES DENTS A CHAQUE TAILLE --
    dire!(j, "");
    dire!(j, "-- TEMOIN 8.7 : le coeur du peigne ne bouge pas, a chaque taille --");
    dire!(j, "Les dents se lisent sur un peigne qui ne depend pas de la fenetre de");
    dire!(j, "source. Le coeur est la bande des pixels les plus profonds du bord");
    dire!(j, "haut, de h/2 - 1 a h/2 - 4 px : hors de tout conge, meme de celui de");
    dire!(j, "l'ANCIENNE au replie (12 px sur 18). Il ne doit pas bouger d'une");
    dire!(j, "direction a l'autre de plus d'un niveau, comme au temoin de l'essai B.");
    let peignes = [
        ("peigne colore", fond_peigne((0x40, 0x30, 0xC0))),
        ("peigne gris", fond_peigne((0xE0, 0xE0, 0xE0))),
    ];
    let mut derive_coeur = 0.0f64;
    let mut dents = std::collections::HashMap::new();
    for (scene, f) in &peignes {
        dire!(j, "");
        dire!(j, "  {scene}");
        dire!(j, "  etat                direction      bande 0-5 px     COEUR (temoin)");
        dire!(j, "                                    ampl.   sat.     ampl.   sat.");
        dire!(j, "  ------------------------------------------------------------------");
        for (k, (nom, l, h, r, n)) in ETATS.iter().enumerate() {
            geometrie::poser(*l, *h, *r, *n);
            let profond = -(*h as f64 / 2.0 - 1.0);
            let mut reference: Option<(f64, f64)> = None;
            for (nom_dir, d) in dirs {
                let px = v.rendre(f, d);
                let (ab, sb) = bande_du_haut(&px, -5.0, -0.45);
                let (ac, sc) = bande_du_haut(&px, profond, profond + 3.0);
                dire!(
                    j,
                    "  {nom}  {nom_dir:<12} {ab:>7.0} {sb:>6.0}   {ac:>7.0} {sc:>6.0}"
                );
                let (a0, s0) = *reference.get_or_insert((ac, sc));
                derive_coeur = derive_coeur.max((ac - a0).abs()).max((sc - s0).abs());
                dents.insert((*scene, k, nom_dir), (ab, sb));
            }
        }
    }
    ok &= verdict(
        j,
        derive_coeur <= 1.0,
        "le coeur est le meme pour les trois directions, a chaque taille et\n  sur les deux peignes. Ce que la bande montre est donc du bord.",
        "le coeur bouge d'une direction a l'autre : la bande ne mesure pas\n  que le bord.",
    );

    dire!(j, "");
    dire!(j, "  LES DENTS : ce que la matiere ajoute au verre seul dans la bande, et");
    dire!(j, "  sa part de ce qu'elle ajoute au deploye.");
    dire!(j, "");
    dire!(j, "  scene          etat                grandeur   ANCIENNE  part  RETENUE  part");
    dire!(j, "  ---------------------------------------------------------------------------");
    for (scene, _) in &peignes {
        // Le peigne gris n'a pas de saturation a lire : elle y vaut zero par
        // construction, et sa part serait une division par zero.
        let grandeurs: &[(&str, usize)] = if *scene == "peigne gris" {
            &[("ampl.", 0)]
        } else {
            &[("ampl.", 0), ("sat. ", 1)]
        };
        for (k, (nom, ..)) in ETATS.iter().enumerate() {
            for &(grandeur, champ) in grandeurs {
                let ajout = |k: usize, dir: &str| {
                    let m = dents[&(*scene, k, dir)];
                    let v0 = dents[&(*scene, k, "verre seul")];
                    if champ == 0 { m.0 - v0.0 } else { m.1 - v0.1 }
                };
                let part = |k: usize, dir: &str| {
                    let reference = ajout(2, dir);
                    if reference.abs() < 0.5 {
                        f64::NAN
                    } else {
                        100.0 * ajout(k, dir) / reference
                    }
                };
                dire!(
                    j,
                    "  {scene:<14} {nom}  {grandeur}   {:>+6.0} {:>5.0} %  {:>+6.0} {:>5.0} %",
                    ajout(k, "ANCIENNE"),
                    part(k, "ANCIENNE"),
                    ajout(k, "RETENUE"),
                    part(k, "RETENUE")
                );
            }
        }
    }
    ok
}

// ---------------------------------------------------------------------------
// INSTRUMENT 9 — LES TROIS ANGLES MORTS DU TOUR DE CORRECTION
// ---------------------------------------------------------------------------
//
// Les instruments 7 et 8 repondent chacun a une objection, et leurs chiffres
// tiennent. Mais ils partagent trois angles morts, et ce sont exactement les
// trois endroits ou l'objection portait :
//
//   1. LE FOND. L'instrument 7 ne rend QUE `fond_bureau_reel`, un gris moyen.
//      Le galet replie n'a jamais ete rendu sur un fond clair uni ni sur un
//      fond sombre uni — or c'est la, sur un aplat, que le moindre relief de
//      bord dessine un cadre, puisque rien d'autre ne l'y cache.
//   2. LE COIN. `saut_a` ne lit que « la partie strictement droite du bord
//      droit », `pic_du_bord_droit` ecarte explicitement les coins, et
//      `bande_du_haut` commence a `rayon + 12`. Les trois objections de bord
//      sont donc mesurees partout SAUF la ou `OPTIQUE.md` §1 dit que l'analyse
//      plane n'est plus exacte.
//   3. LA TAILLE. Tout l'instrument 8 travaille a la taille DEPLOYEE, ou le
//      facteur du conge relatif vaut exactement 1. La couture, le peigne et le
//      niveau du lisere n'ont donc jamais ete lus sur le galet REPLIE, qui est
//      l'etat par defaut.
//
// Cet instrument ne rejuge rien : il rend les memes grandeurs aux endroits que
// les deux precedents ne pouvaient pas atteindre.

/// Le plus grand niveau du premier pixel du bord, DANS LES COINS seulement.
///
/// C'est le complement exact de `pic_du_bord_droit`, qui les ecarte. La meme
/// fenetre `d ∈ [−1,5 ; −0,45]`, le meme canal.
fn pic_du_coin(px: &[u8]) -> f64 {
    let mut pic = 0.0f64;
    for y in 0..ih() {
        for x in 0..il() {
            let d = distance(x as f32, y as f32) as f64;
            if d > -0.45 || d < -1.5 {
                continue;
            }
            let dans_coin = ((x as f32) < rayon() || (x as f32) > il() as f32 - rayon())
                && ((y as f32) < rayon() || (y as f32) > ih() as f32 - rayon());
            if !dans_coin {
                continue;
            }
            let i = ((y * il() + x) * 4) as usize;
            pic = pic.max(niveau_luminance(px[i], px[i + 1], px[i + 2]));
        }
    }
    pic
}

/// Le saut entre les deux bacs du profil qui encadrent `d_cible`, DANS LE COIN.
///
/// Le pendant de `saut_a` pour l'autre zone. Il ne peut pas se lire en colonnes
/// de pixels — dans un coin, deux colonnes voisines ne sont pas a une distance
/// constante du bord — donc il se lit sur le profil range par distance, avec la
/// tolerance qui a deja ete eprouvee par le temoin 2.3.
/// `largeur` est la largeur du conge, positive : la frontiere cherchee est donc
/// a la distance signee `-largeur`. Les bacs etant poses sur les demi-entiers
/// negatifs, les deux qui l'encadrent sont `-(k+0,5)` et `-(k+1,5)` avec
/// `k = floor(largeur - 0,5)`.
fn saut_coin(pr: &[(f64, f64, usize)], largeur: f64) -> Option<f64> {
    let k = (largeur - 0.5).floor();
    if k < 0.0 {
        return None;
    }
    let a = au(pr, -(k + 0.5))?;
    let b = au(pr, -(k + 1.5))?;
    Some((a - b).abs())
}

/// Jusqu'ou le bord se voit, sur un APLAT.
///
/// Sur un fond uni, le verre seul est parfaitement plat a l'interieur : tout ce
/// qui s'ecarte du coeur de plus de `seuil` niveaux est de la matiere de bord,
/// et rien d'autre. On rend donc la plus grande distance a l'arete a laquelle
/// l'ecart tient encore. C'est la grandeur que l'objection 1 demandait et que
/// personne n'avait mesuree : non pas « combien fait le conge » mais « sur
/// quelle epaisseur de l'objet la matiere se voit ».
fn portee_visible(px: &[u8], seuil: f64) -> (f64, f64) {
    let jusqu_a = (ih() / 2) as i32 - 2;
    let pr = profil_normal(px, Zone::Droit, jusqu_a, TOLERANCE);
    // Le coeur : le bac le plus profond qui contienne des pixels.
    let coeur = pr
        .iter()
        .filter(|(_, v, n)| *n > 0 && v.is_finite())
        .map(|(c, v, _)| (*c, *v))
        .fold((0.0f64, f64::NAN), |acc, (c, v)| if c < acc.0 { (c, v) } else { acc });
    let mut portee = 0.0f64;
    for (c, v, n) in &pr {
        if *n == 0 || !v.is_finite() || *c >= 0.0 {
            continue;
        }
        if (v - coeur.1).abs() > seuil {
            portee = portee.max(-c);
        }
    }
    (portee, coeur.1)
}

/// Relire un PNG que ce banc vient d'ecrire, en RGBA 8 bits.
fn lire_png(chemin: &str) -> (Vec<u8>, u32, u32) {
    let f = std::fs::File::open(chemin).unwrap();
    let mut lecteur = png::Decoder::new(std::io::BufReader::new(f))
        .read_info()
        .unwrap();
    let mut tampon = vec![0u8; lecteur.output_buffer_size()];
    let info = lecteur.next_frame(&mut tampon).unwrap();
    tampon.truncate(info.buffer_size());
    (tampon, info.width, info.height)
}

/// Assembler des vignettes en grille, sur un fond neutre, avec un grossissement
/// entier.
///
/// `PLANCHE-trois-tailles.png` a ete assemblee par un outil qui n'est pas dans
/// le depot : la planche est publiee, son fabricant non, et elle n'est donc pas
/// refaisable. Celle-ci sort du meme binaire que ses vignettes.
///
/// Les vignettes n'ont pas la meme taille d'une ligne a l'autre — c'est tout le
/// sujet — donc chaque colonne prend la largeur de sa plus large et chaque
/// ligne la hauteur de sa plus haute, et la vignette est posee EN HAUT A GAUCHE
/// de sa case. Centrer ferait croire a un cadrage qui n'existe pas.
fn assembler(sortie: &str, lignes: &[Vec<String>], grossir: u32, fond: u8) {
    let mut vignettes: Vec<Vec<(Vec<u8>, u32, u32)>> = Vec::new();
    for ligne in lignes {
        vignettes.push(ligne.iter().map(|n| lire_png(n)).collect());
    }
    let colonnes = vignettes.iter().map(|l| l.len()).max().unwrap_or(0);
    let mut larg = vec![0u32; colonnes];
    let mut haut = vec![0u32; vignettes.len()];
    for (i, ligne) in vignettes.iter().enumerate() {
        for (k, (_, l, h)) in ligne.iter().enumerate() {
            larg[k] = larg[k].max(*l);
            haut[i] = haut[i].max(*h);
        }
    }
    let marge = 10u32;
    let total_l = (larg.iter().sum::<u32>() + marge * (colonnes as u32 + 1)) * grossir;
    let total_h = (haut.iter().sum::<u32>() + marge * (vignettes.len() as u32 + 1)) * grossir;
    let mut px = vec![255u8; (total_l * total_h * 4) as usize];
    for i in (0..px.len()).step_by(4) {
        px[i] = fond;
        px[i + 1] = fond;
        px[i + 2] = fond;
    }
    let mut oy = marge;
    for (i, ligne) in vignettes.iter().enumerate() {
        let mut ox = marge;
        for (k, (v, l, h)) in ligne.iter().enumerate() {
            for y in 0..*h {
                for x in 0..*l {
                    let s = ((y * l + x) * 4) as usize;
                    for gy in 0..grossir {
                        for gx in 0..grossir {
                            let cx = (ox + x) * grossir + gx;
                            let cy = (oy + y) * grossir + gy;
                            let d = ((cy * total_l + cx) * 4) as usize;
                            px[d] = v[s];
                            px[d + 1] = v[s + 1];
                            px[d + 2] = v[s + 2];
                            px[d + 3] = 255;
                        }
                    }
                }
            }
            ox += larg[k] + marge;
        }
        oy += haut[i] + marge;
    }
    ecrire_png(sortie, &px, total_l, total_h);
}

/// L'energie de haute frequence dans une bande definie par la DISTANCE SIGNEE.
///
/// `hf_bande` la definit par une plage de colonnes et lit `h/2 - 30 .. h/2 + 30`
/// de lignes : elle ne peut pas servir sur un galet de 36 px de haut. Celle-ci
/// suit le bord tout du long, coins compris, et vaut a toute taille.
fn hf_par_distance(px: &[u8], de: f64, a: f64) -> f64 {
    let (mut somme, mut n) = (0.0f64, 0.0f64);
    for y in 0..ih() {
        for x in 1..il() {
            let d = distance(x as f32, y as f32) as f64;
            if d < de || d > a {
                continue;
            }
            if (distance((x - 1) as f32, y as f32) as f64) > 0.0 {
                continue;
            }
            let i = ((y * il() + x) * 4) as usize;
            let k = ((y * il() + x - 1) * 4) as usize;
            for c in 0..3 {
                somme += (px[i + c] as f64 - px[k + c] as f64).abs();
                n += 1.0;
            }
        }
    }
    if n == 0.0 {
        f64::NAN
    } else {
        somme / n
    }
}

/// La meme energie, lue sur le FOND lui-meme. C'est le plafond de
/// `MATIERE.md` §5.4 : « un bord plus net que la realite est un bord faux ».
fn hf_du_fond(fond: &[u8], de: f64, a: f64) -> f64 {
    let (mut somme, mut n) = (0.0f64, 0.0f64);
    for y in 0..ih() {
        for x in 1..il() {
            let d = distance(x as f32, y as f32) as f64;
            if d < de || d > a {
                continue;
            }
            let i = (((y + MARGE) * SL + x + MARGE) * 4) as usize;
            let k = (((y + MARGE) * SL + x - 1 + MARGE) * 4) as usize;
            for c in 0..3 {
                somme += (fond[i + c] as f64 - fond[k + c] as f64).abs();
                n += 1.0;
            }
        }
    }
    if n == 0.0 {
        f64::NAN
    } else {
        somme / n
    }
}

/// Les cinq etats du ressort, et non trois. Meme loi que `DESIGN.md` §6 : tout
/// se lit sur la hauteur, qui va de 36 a 148 ; le rayon suit de 18 a 48 et
/// l'exposant de 2,5 a 4, lineairement en p.
fn etat_du_ressort(p: f64) -> (u32, u32, f32, f32) {
    let l = (88.0 + p * (320.0 - 88.0)).round() as u32;
    let h = (36.0 + p * (148.0 - 36.0)).round() as u32;
    ((l / 2) * 2, (h / 2) * 2, (18.0 + p * 30.0) as f32, (2.5 + p * 1.5) as f32)
}

fn instrument_9(v: &Verre, j: &mut Journal) -> bool {
    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "INSTRUMENT 9 : LES TROIS ANGLES MORTS");
    dire!(j, "===========================================================");
    dire!(j, "Les instruments 7 et 8 ne mentent pas, mais ils regardent tous");
    dire!(j, "les deux a cote de trois endroits : le FOND UNI (7 ne rend que le");
    dire!(j, "bureau vraisemblable), le COIN (8 l'ecarte explicitement de ses");
    dire!(j, "trois mesures) et la TAILLE REPLIEE (8 travaille au deploye, ou le");
    dire!(j, "facteur du conge relatif vaut 1). Or l'objection portait sur ces");
    dire!(j, "trois endroits-la.");

    let mut ok = true;
    let clair = uni(226);
    let sombre = uni(37);
    let reel = fond_bureau_reel();
    let bureau = fond_bureau();
    let dirs: [(&str, &Direction); 3] =
        [("verre seul", &V0), ("ANCIENNE", &ANCIENNE), ("RETENUE", &RETENUE)];

    // -- TEMOIN 9.1 : sur un aplat, le verre seul n'a AUCUNE portee de bord --
    dire!(j, "");
    dire!(j, "-- TEMOIN 9.1 : sur un aplat, le verre seul est plat --");
    dire!(j, "La portee visible se mesure comme un ecart au coeur. Si elle");
    dire!(j, "n'est pas nulle pour le verre SEUL, qui n'a ni conge ni lisere,");
    dire!(j, "c'est l'instrument qui fabrique du bord et non la matiere.");
    let mut pire_temoin = 0.0f64;
    for p in [0.0, 0.5, 1.0] {
        let (l, h, r, n) = etat_du_ressort(p);
        geometrie::poser(l, h, r, n);
        for f in [&clair, &sombre] {
            let (portee, _) = portee_visible(&v.rendre(f, &V0), 2.0);
            pire_temoin = pire_temoin.max(portee);
        }
    }
    dire!(j, "  pire portee du verre seul, 3 tailles x 2 aplats : {pire_temoin:.1} px");
    ok &= verdict(
        j,
        pire_temoin == 0.0,
        "le verre seul ne porte aucun bord sur un aplat. Ce que les lignes\n  suivantes mesurent est donc bien ajoute par la matiere.",
        "l'instrument voit du bord la ou il n'y en a pas.",
    );

    // -- A. LE GALET REPLIE SUR LES DEUX APLATS --
    dire!(j, "");
    dire!(j, "-- A. LE GALET REPLIE, SUR FOND CLAIR ET SUR FOND SOMBRE --");
    dire!(j, "L'objection 1 ne disait pas « le conge est large », elle disait");
    dire!(j, "« la matiere avale l'objet ». La grandeur est donc une PART DE");
    dire!(j, "L'OBJET : sur quelle fraction de la demi-hauteur le bord se voit");
    dire!(j, "encore. A 36 px de haut, la demi-hauteur vaut 18 px.");
    dire!(j, "");
    dire!(j, "  etat        fond     direction     portee   part de la   pic du");
    dire!(j, "                                     visible  demi-hauteur  bord");
    dire!(j, "  ---------------------------------------------------------------------");
    let mut part_retenue_replie = 0.0f64;
    let mut part_ancienne_replie = 0.0f64;
    for p in [0.0, 0.5, 1.0] {
        let (l, h, r, n) = etat_du_ressort(p);
        geometrie::poser(l, h, r, n);
        let nom_etat = match p {
            x if x == 0.0 => "replie   ",
            x if x == 0.5 => "mi-chemin",
            _ => "deploye  ",
        };
        for (nom_fond, fond) in [("clair ", &clair), ("sombre", &sombre)] {
            for (nom_dir, d) in dirs {
                let px = v.rendre(fond, d);
                let (portee, _) = portee_visible(&px, 2.0);
                let part = 100.0 * portee / (h as f64 / 2.0);
                let (pic, _, _) = pic_du_bord(&px, -1.5);
                dire!(
                    j,
                    "  {nom_etat} {nom_fond}  {nom_dir:<12} {portee:>5.1} px  {part:>6.1} %   {pic:>6.1} niv"
                );
                if p == 0.0 && nom_fond == "clair " {
                    if nom_dir == "RETENUE" {
                        part_retenue_replie = part;
                    }
                    if nom_dir == "ANCIENNE" {
                        part_ancienne_replie = part;
                    }
                }
                let court = match nom_dir {
                    "verre seul" => "verre",
                    "ANCIENNE" => "ancienne",
                    _ => "retenue",
                };
                let etat = nom_etat.trim();
                ecrire_png(
                    &format!("captures/mesure/esthete-{etat}-{}-{court}.png", nom_fond.trim()),
                    &px,
                    il(),
                    ih(),
                );
            }
        }
    }
    dire!(j, "");
    dire!(
        j,
        "  Sur fond clair, au galet replie : la matiere se voit sur {part_ancienne_replie:.0} % de la"
    );
    dire!(
        j,
        "  demi-hauteur sous l'ancienne loi, {part_retenue_replie:.0} % sous la retenue."
    );

    // -- B. LE COIN --
    dire!(j, "");
    dire!(j, "-- B. LE COIN, QUE LES TROIS MESURES DU BORD ECARTENT --");
    dire!(j, "`saut_a` lit « la partie strictement droite du bord droit », et");
    dire!(j, "`pic_du_bord_droit` saute les coins. Les memes deux grandeurs sont");
    dire!(j, "reprises ici DANS le coin. La couture se lit sur le profil range");
    dire!(j, "par distance, avec la tolerance du temoin 2.3 ; son temoin est le");
    dire!(j, "meme saut 8 px plus a l'interieur, ou aucune frontiere de conge");
    dire!(j, "ne passe.");
    dire!(j, "");
    dire!(j, "  etat        direction      b      couture   temoin    pic coin  pic droit");
    dire!(j, "  --------------------------------------------------------------------------");
    let mut pire_couture_rampe = 0.0f64;
    let mut pire_couture_temoin = 0.0f64;
    let mut pire_coin_retenue = 0.0f64;
    for p in [0.0, 0.5, 1.0] {
        let (l, h, r, n) = etat_du_ressort(p);
        geometrie::poser(l, h, r, n);
        let nom_etat = match p {
            x if x == 0.0 => "replie   ",
            x if x == 0.5 => "mi-chemin",
            _ => "deploye  ",
        };
        for (nom_dir, d) in dirs {
            // Le verre seul n'a pas de conge : on lit son profil a la MEME
            // distance que celle ou la RETENUE pose sa frontiere, sans quoi le
            // temoin ne serait pas au meme endroit que la mesure.
            let b = if d.largeur > 0.0 {
                if d.conge_relatif {
                    d.largeur as f64 * h as f64 / PLUS_GRAND_H as f64
                } else {
                    d.largeur as f64
                }
            } else {
                CONGE as f64 * h as f64 / PLUS_GRAND_H as f64
            };
            let px = v.rendre(&reel, d);
            let jusqu_a = (h / 2) as i32 - 2;
            let pr = profil_normal(&px, Zone::Coin, jusqu_a, TOLERANCE);
            let couture = saut_coin(&pr, b);
            let temoin = saut_coin(&pr, b + 8.0);
            let coin = pic_du_coin(&px);
            let droit = pic_du_bord_droit(&px);
            let ec = |x: Option<f64>| match x {
                Some(v) => format!("{v:>6.1}"),
                None => "     —".to_string(),
            };
            dire!(
                j,
                "  {nom_etat} {nom_dir:<12} {b:>5.2}  {} niv {} niv  {coin:>6.1}    {droit:>6.1}",
                ec(couture),
                ec(temoin)
            );
            if nom_dir == "RETENUE" {
                pire_couture_rampe = pire_couture_rampe.max(couture.unwrap_or(0.0));
                pire_couture_temoin = pire_couture_temoin.max(temoin.unwrap_or(0.0));
                pire_coin_retenue = pire_coin_retenue.max(coin);
            }
        }
    }
    dire!(j, "");
    dire!(j, "  La couture de la RETENUE dans le coin vaut au pire");
    dire!(j, "  {pire_couture_rampe:.1} niveaux, contre {pire_couture_temoin:.1} pour son propre temoin a 8 px");
    dire!(j, "  plus a l'interieur. Un saut qui ne depasse pas son temoin n'est");
    dire!(j, "  pas une frontiere : c'est le grain du profil.");
    dire!(j, "");
    dire!(j, "  Le pic du COIN de la RETENUE vaut au pire {pire_coin_retenue:.1} niveaux, pour un");
    dire!(j, "  plafond de {:.0} — l'encre la moins lue de DESIGN.md §2.", encre_la_moins_lue());

    // -- B bis. LA GARANTIE, RELUE DANS LE COIN, SUR LES QUATRE SCENES --
    dire!(j, "");
    dire!(j, "-- B bis. LA GARANTIE DE MATIERE.md §5.2, RELUE DANS LE COIN --");
    dire!(j, "L'essai 8.D retient la force 0,30 parce qu'elle tient sous l'encre");
    dire!(j, "la moins lue « sur les QUATRE scenes ». Mais il lit avec");
    dire!(j, "`pic_du_bord_droit`, qui ECARTE les coins. La meme garantie, aux");
    dire!(j, "memes quatre scenes, lue la ou elle n'avait pas ete lue.");
    dire!(j, "");
    geometrie::deploye();
    let plafond = encre_la_moins_lue();
    let scenes: [(&str, Vec<u8>); 4] = [
        ("nuit  ", fond_horloge(false)),
        ("coupe ", fond_coupe()),
        ("bureau", fond_bureau()),
        ("reel  ", fond_bureau_reel()),
    ];
    dire!(j, "  scene    pic droit   pic coin   ce que le coin ajoute   verdict");
    dire!(j, "  ------------------------------------------------------------------");
    let mut pire_droit_4 = 0.0f64;
    let mut pire_coin_4 = 0.0f64;
    for (nom, f) in scenes.iter() {
        let px = v.rendre(f, &RETENUE);
        let droit = pic_du_bord_droit(&px);
        let coin = pic_du_coin(&px);
        pire_droit_4 = pire_droit_4.max(droit);
        pire_coin_4 = pire_coin_4.max(coin);
        dire!(
            j,
            "  {nom}   {droit:>6.1}     {coin:>6.1}      {:>+6.1} niv          {}",
            coin - droit,
            if coin <= plafond { "TIENT" } else { "CASSE" }
        );
    }
    dire!(j, "");
    dire!(
        j,
        "  pire des quatre : {pire_droit_4:.0} au bord droit, {pire_coin_4:.0} dans le coin, pour un"
    );
    dire!(
        j,
        "  plafond de {plafond:.0}. La marge que l'essai 8.D annonce passe donc de"
    );
    dire!(
        j,
        "  {:+.1} a {:+.1} niveaux quand on inclut les coins.",
        plafond - pire_droit_4,
        plafond - pire_coin_4
    );

    // -- C. LE PEIGNE, AU GALET REPLIE --
    dire!(j, "");
    dire!(j, "-- C. LE PEIGNE, LU LA OU IL N'AVAIT PAS ETE LU --");
    dire!(j, "L'essai 8.B lit la bande de bord du seul galet DEPLOYE, et sur un");
    dire!(j, "bord HAUT prive de ses coins. La meme lecture, aux trois tailles.");
    dire!(j, "Le coeur de la meme image est le temoin : il ne doit pas bouger.");
    dire!(j, "");
    dire!(j, "UN PIEGE DE CE TABLEAU, ET IL EST A MOI. Une cible plus petite lit");
    dire!(j, "la source a partir du MEME coin, donc le galet replie ne regarde pas");
    dire!(j, "le meme morceau de bureau que le deploye — sur ce fond-ci, il tombe");
    dire!(j, "sur deux bandes presque grises. Les lignes ne se comparent donc pas");
    dire!(j, "entre tailles ; seule la comparaison d'une ligne a son propre verre");
    dire!(j, "seul, sur la meme image, veut dire quelque chose.");
    dire!(j, "");
    dire!(j, "  bureau raye colore");
    dire!(j, "  etat        direction      bande 0-5 px       COEUR (temoin)");
    dire!(j, "                            ampl.  saturation  ampl.  saturation");
    dire!(j, "  -----------------------------------------------------------------");
    let mut pire_sat_bande = 0.0f64;
    let mut pire_sat_coeur = 0.0f64;
    for p in [0.0, 0.5, 1.0] {
        let (l, h, r, n) = etat_du_ressort(p);
        geometrie::poser(l, h, r, n);
        let nom_etat = match p {
            x if x == 0.0 => "replie   ",
            x if x == 0.5 => "mi-chemin",
            _ => "deploye  ",
        };
        for (nom_dir, d) in dirs {
            let px = v.rendre(&bureau, d);
            let (ab, sb) = bande_du_haut(&px, -5.0, -0.45);
            let profond = -(h as f64 / 2.0 - 3.0);
            let (ac, sc) = bande_du_haut(&px, profond, profond + 5.0);
            dire!(
                j,
                "  {nom_etat} {nom_dir:<12} {ab:>6.1} {sb:>10.0}  {ac:>6.1} {sc:>10.0}"
            );
            if nom_dir == "RETENUE" {
                pire_sat_bande = pire_sat_bande.max(sb);
                pire_sat_coeur = pire_sat_coeur.max(sc);
            }
            if p != 0.5 {
                let court = match nom_dir {
                    "verre seul" => "verre",
                    "ANCIENNE" => "ancienne",
                    _ => "retenue",
                };
                let etat = nom_etat.trim();
                ecrire_png(
                    &format!("captures/mesure/esthete-peigne-{etat}-{court}.png"),
                    &px,
                    il(),
                    ih(),
                );
            }
        }
    }
    dire!(j, "");
    dire!(j, "  La RETENUE monte au pire a {pire_sat_bande:.0} de saturation dans la bande du");
    dire!(j, "  bord, pour un coeur qui reste a {pire_sat_coeur:.0}. DESIGN.md §8 dit « pas de");
    dire!(j, "  couleur » ; le rapport des deux est le chiffre qui decide.");

    // -- D. LE RESSORT, EN CINQ POINTS --
    dire!(j, "");
    dire!(j, "-- D. LE RESSORT EN CINQ POINTS, ET NON TROIS --");
    dire!(j, "Trois points ne disent pas si une courbe est continue. Les cinq");
    dire!(j, "etats que le ressort traverse en 330 ms, sur le meme bureau.");
    dire!(j, "");
    dire!(j, "  p       taille      b      part du   part     portee   pic du");
    dire!(j, "                             conge    ecartee   visible   bord");
    dire!(j, "  ---------------------------------------------------------------------");
    let mut suite_part: Vec<f64> = Vec::new();
    let mut suite_portee: Vec<f64> = Vec::new();
    for k in 0..5 {
        let p = k as f64 / 4.0;
        let (l, h, r, n) = etat_du_ressort(p);
        geometrie::poser(l, h, r, n);
        let b = CONGE as f64 * h as f64 / PLUS_GRAND_H as f64;
        let part = part_du_conge(b);
        let v0 = v.rendre(&reel, &V0);
        let px = v.rendre(&reel, &RETENUE);
        let touche = part_ecartee(&v0, &px, 8);
        let sur_clair = v.rendre(&clair, &RETENUE);
        let (portee, _) = portee_visible(&sur_clair, 2.0);
        let (pic, _, _) = pic_du_bord(&px, -1.5);
        dire!(
            j,
            "  {p:.2}  {l:>3} x {h:>3}  {b:>5.2}  {part:>6.1} %  {touche:>6.1} %  {portee:>5.1} px  {pic:>6.1} niv"
        );
        suite_part.push(touche);
        suite_portee.push(portee / (h as f64 / 2.0));
        ecrire_png(
            &format!("captures/mesure/esthete-ressort-{k}.png"),
            &px,
            il(),
            ih(),
        );
    }
    geometrie::deploye();
    let etendue = |s: &[f64]| s.iter().cloned().fold(0.0f64, f64::max) - s.iter().cloned().fold(1e9f64, f64::min);
    dire!(j, "");
    dire!(j, "  etendue de la part ecartee sur les cinq points : {:.1} points", etendue(&suite_part));
    dire!(
        j,
        "  etendue de la portee rapportee a la demi-hauteur : {:.1} %",
        100.0 * etendue(&suite_portee)
    );
    dire!(j, "");
    dire!(j, "  La seconde ligne est celle qui decide de l'objection 2 : si la");
    dire!(j, "  matiere occupe la meme FRACTION de l'objet a chaque instant, ce");
    dire!(j, "  qu'on voit grandir est un objet ; si la fraction change, c'est");
    dire!(j, "  une substance qui se transmute.");

    // -- E. CE QUE LE CONGE RELATIF COUTE A LA CIBLE DE MATIERE.md §5.4 --
    dire!(j, "");
    dire!(j, "-- E. CE QUE LE CONGE RELATIF COUTE A LA CIBLE §5.4 --");
    dire!(j, "Faire suivre `b` a la hauteur repond a l'objection 1, mais ce n'est");
    dire!(j, "pas gratuit : a p = 0 le conge ne fait plus que 2,92 px, donc le");
    dire!(j, "deplacement, qui est proportionnel a `e`, tombe dans le meme");
    dire!(j, "rapport. `MATIERE.md` §5.4 demande que l'energie de structure de la");
    dire!(j, "bande de bord vaille DIX FOIS celle du centre, et reste SOUS celle");
    dire!(j, "du fond nu. La cible est ecrite pour l'ilot deploye ; elle n'a");
    dire!(j, "jamais ete lue sur l'etat que l'auteur voit toute la nuit.");
    dire!(j, "");
    dire!(j, "  fond de bureau raye, achromatique");
    dire!(j, "  etat        direction     hf bande 1-8   hf coeur   rapport   fond nu");
    dire!(j, "  ---------------------------------------------------------------------");
    let gris = fond_bureau_gris();
    for p in [0.0, 0.5, 1.0] {
        let (l, h, r, n) = etat_du_ressort(p);
        geometrie::poser(l, h, r, n);
        let nom_etat = match p {
            x if x == 0.0 => "replie   ",
            x if x == 0.5 => "mi-chemin",
            _ => "deploye  ",
        };
        let profond = -(h as f64 / 2.0 - 2.0);
        let nu = hf_du_fond(&gris, -8.0, -1.0);
        for (nom_dir, d) in [("verre seul", &V0), ("RETENUE", &RETENUE)] {
            let px = v.rendre(&gris, d);
            let bord = hf_par_distance(&px, -8.0, -1.0);
            let coeur = hf_par_distance(&px, profond, profond + 7.0);
            let rapport = if coeur > 1e-6 { bord / coeur } else { f64::INFINITY };
            dire!(
                j,
                "  {nom_etat} {nom_dir:<12} {bord:>9.2}   {coeur:>9.2}   {rapport:>6.1}   {nu:>7.2}"
            );
        }
    }
    geometrie::deploye();
    dire!(j, "");
    dire!(j, "  La cible de §5.4 est « bande >= 10 x coeur » et « bande <= fond nu ».");
    dire!(j, "  Elle est ecrite pour le deploye ; la lire aux trois tailles dit ce");
    dire!(j, "  que la reponse a l'objection 1 a coute a la reponse de §5.4.");

    // -- LES PLANCHES, assemblees par le meme binaire que leurs vignettes --
    let c = |n: &str| format!("captures/mesure/{n}.png");
    let ligne = |etat: &str, fond: &str| {
        vec![
            c(&format!("esthete-{etat}-{fond}-verre")),
            c(&format!("esthete-{etat}-{fond}-ancienne")),
            c(&format!("esthete-{etat}-{fond}-retenue")),
        ]
    };
    // Une colonne par direction : verre seul, ANCIENNE, RETENUE. Une ligne par
    // taille. Le verre seul est la premiere colonne parce que c'est lui le
    // temoin : ce qui n'est pas dans sa vignette est ajoute par la matiere.
    assembler(
        &c("PLANCHE-esthete-clair"),
        &[ligne("replie", "clair"), ligne("mi-chemin", "clair"), ligne("deploye", "clair")],
        2,
        118,
    );
    assembler(
        &c("PLANCHE-esthete-sombre"),
        &[ligne("replie", "sombre"), ligne("mi-chemin", "sombre"), ligne("deploye", "sombre")],
        2,
        118,
    );
    assembler(
        &c("PLANCHE-esthete-peigne"),
        &[
            vec![
                c("esthete-peigne-deploye-verre"),
                c("esthete-peigne-deploye-ancienne"),
                c("esthete-peigne-deploye-retenue"),
            ],
            vec![
                c("esthete-peigne-replie-verre"),
                c("esthete-peigne-replie-ancienne"),
                c("esthete-peigne-replie-retenue"),
            ],
        ],
        2,
        118,
    );
    assembler(
        &c("PLANCHE-esthete-ressort"),
        &[(0..5).map(|k| c(&format!("esthete-ressort-{k}"))).collect()],
        1,
        118,
    );
    dire!(j, "");
    dire!(j, "  planches assemblees : PLANCHE-esthete-clair.png, -sombre.png,");
    dire!(j, "  -peigne.png, -ressort.png. Colonnes : verre seul, ANCIENNE,");
    dire!(j, "  RETENUE. L'assemblage sort du meme binaire que les vignettes.");
    ok
}

// ===========================================================================
// LE CHANTIER DU VERRIER — les quatre questions que la realisation doit trancher
// ===========================================================================

/// Le plus grand niveau atteint sur les pixels du bord, et ou.
///
/// La fenetre commence a `-0,45` et non a `-0,6`, et cette demi-decimale a
/// coute un tour de banc a `MATIERE.md` §0 : le seul pixel de lisere d'un bord
/// droit tombe exactement a `d = -0,5`, et une fenetre ouverte a `-0,6`
/// l'excluait. On mesure donc a partir de la ou le masque vaut encore un.
fn pic_du_bord(px: &[u8], jusqu_a: f64) -> (f64, u32, u32) {
    let mut pic = 0.0f64;
    let mut ou = (0u32, 0u32);
    for y in 0..ih() {
        for x in 0..il() {
            let d = distance(x as f32, y as f32) as f64;
            if d > -0.45 || d < jusqu_a {
                continue;
            }
            let i = ((y * il() + x) * 4) as usize;
            let n = niveau_luminance(px[i], px[i + 1], px[i + 2]);
            if n > pic {
                pic = n;
                ou = (x, y);
            }
        }
    }
    (pic, ou.0, ou.1)
}

fn chantier_du_verrier(v: &Verre, j: &mut Journal) {
    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "LE CHANTIER DU VERRIER — ce que la realisation doit trancher");
    dire!(j, "===========================================================");

    // -- A. COMBIEN DE PRISES SUR L'EMPREINTE --
    dire!(j, "");
    dire!(j, "-- A. COMBIEN DE PRISES SUR L'EMPREINTE DU PIXEL --");
    dire!(j, "La montee de Fresnel est sous-pixellaire : prise au centre du");
    dire!(j, "pixel elle manque tout l'effet, et sa valeur depend du hasard de");
    dire!(j, "la grille. Le nuanceur la moyenne donc sur l'empreinte, et le");
    dire!(j, "nombre de prises est le seul reglage que ce chantier ait a poser.");
    dire!(j, "Il ne se choisit pas au juge : on le compare a la meme moyenne");
    dire!(j, "portee a 256 prises, qui est la valeur vers laquelle tout converge.");
    let clair = 226u8;
    let fond = uni(clair);
    let reference = Direction { prises: 256.0, ..V6 };
    dire!(j, "");
    dire!(j, "  prises     d=-0,5   d=-1,5   d=-2,5   d=-3,5   pire ecart a 256");
    dire!(j, "  ----------------------------------------------------------------");
    let mut pires = Vec::new();
    for n in [1.0f32, 2.0, 4.0, 8.0, 16.0, 32.0] {
        let d = Direction { prises: n, ..V6 };
        let px = v.rendre(&fond, &d);
        let pr = profil_normal(&px, Zone::Droit, 12, TOLERANCE);
        let mut ligne = format!("  {:>6.0}  ", n);
        let mut pire = 0.0f64;
        for k in 0..4 {
            let dk = -0.5 - k as f64;
            let m = au(&pr, dk).unwrap_or(f64::NAN);
            let at = niveau_verrier(dk, clair, &reference);
            let _ = write!(ligne, "{m:>8.1} ");
            pire = pire.max((m - at).abs());
        }
        let _ = write!(ligne, "  {pire:>12.1} niv");
        dire!(j, "{ligne}");
        pires.push((n, pire));
    }
    let retenu = pires
        .iter()
        .find(|(_, e)| *e <= 1.0)
        .map(|(n, _)| *n)
        .unwrap_or(32.0);
    dire!(j, "");
    dire!(
        j,
        "  Le premier nombre de prises qui tient sous UN niveau d'ecart a la"
    );
    dire!(j, "  moyenne exacte est {retenu:.0}. Un niveau est le pas de quantification");
    dire!(j, "  d'un octet : au-dessous, la difference n'existe pas a l'image.");

    // -- B. DE QUELLE COULEUR EST LE LISERE --
    dire!(j, "");
    dire!(j, "-- B. LE LISERE PREND-IL LA COULEUR DE CE QU'IL TOUCHE ? --");
    dire!(j, "C'est la question qui separe une matiere d'un trait. Un lisere");
    dire!(j, "peint vaut la meme chose partout ; un lisere nourri par l'ecran");
    dire!(j, "change avec lui. Le fond est coupe en deux, clair 226 a gauche et");
    dire!(j, "sombre 37 a droite, et l'on lit le bord GAUCHE puis le bord DROIT.");
    let coupe = fond_coupe();
    dire!(j, "");
    dire!(j, "  direction          bord gauche (fond clair)   bord droit (fond sombre)   rapport");
    dire!(j, "  ----------------------------------------------------------------------------------");
    let bord_lateral = |px: &[u8], gauche: bool| -> f64 {
        let x = if gauche { 0u32 } else { il() - 1 };
        let mut s = 0.0;
        let mut n = 0.0;
        for y in ih() / 2 - 20..ih() / 2 + 20 {
            let i = ((y * il() + x) * 4) as usize;
            s += px[i + 1] as f64;
            n += 1.0;
        }
        s / n
    };
    for d in [&TEMOIN, &DALLE, &V3, &V6] {
        let px = v.rendre(&coupe, d);
        let (g, dr) = (bord_lateral(&px, true), bord_lateral(&px, false));
        dire!(
            j,
            "  {:<18} {g:>18.1} niv {dr:>22.1} niv {:>11.2}",
            d.nom,
            g / dr.max(1e-6)
        );
    }
    ecrire_png("captures/mesure/verrier-coupe-V6.png", &v.rendre(&coupe, &V6), il(), ih());
    ecrire_png("captures/mesure/verrier-coupe-esquisse.png", &v.rendre(&coupe, &DALLE), il(), ih());

    // -- C. LE PIC SUR LA SCENE DE NUIT --
    dire!(j, "");
    dire!(j, "-- C. LE PIC DU BORD SUR LA SCENE DE NUIT --");
    dire!(j, "`MATIERE.md` §5.2 pose un plafond de 45 niveaux, et il n'est pas");
    dire!(j, "derive de l'optique : c'est le niveau que le verre entier atteint");
    dire!(j, "deja contre un ecran allume. Un bord plus clair que cela, dans le");
    dire!(j, "noir, mettrait un contour la ou le projet en refuse un.");
    let nuit = fond_horloge(false);
    dire!(j, "");
    dire!(j, "  direction          pic du bord   ou           verdict contre 45");
    dire!(j, "  --------------------------------------------------------------------");
    for d in [&TEMOIN, &DALLE, &V3, &V6] {
        let px = v.rendre(&nuit, d);
        let (p, x, y) = pic_du_bord(&px, -6.0);
        // Le plafond de 45 est un OCTET pose sur un gris. Le pic se lit
        // desormais en luminance flottante, et l'encre de la nuit, #F2EFE9, est
        // un blanc chaud : le verre seul y lit 45,1, un dixieme au-dessus de ce
        // que le vert lisait. Un gris ne sait pas afficher 45,1 ; la luminance
        // d'un pixel colore n'a pas de sens plus fin que le pas de l'octet
        // auquel on la compare. On compare donc le niveau arrondi a l'octet,
        // sans quoi le temoin depasserait le plafond qu'il definit.
        dire!(
            j,
            "  {:<18} {p:>9.1} niv   ({x:>3},{y:>3})   {}",
            d.nom,
            if p.round() <= 45.0 { "TIENT" } else { "DEPASSE" }
        );
    }
    ecrire_png("captures/mesure/verrier-nuit-V6.png", &v.rendre(&nuit, &V6), il(), ih());
    ecrire_png("captures/mesure/verrier-nuit-esquisse.png", &v.rendre(&nuit, &DALLE), il(), ih());

    // -- D. LE BATTEMENT AU CHANGEMENT DE SECONDE --
    dire!(j, "");
    dire!(j, "-- D. LE BATTEMENT AU CHANGEMENT DE SECONDE --");
    dire!(j, "C'est l'essai que `MATIERE.md` §7 reclame en premier, et le seul");
    dire!(j, "qui puisse disqualifier tout l'effet. Un lisere nourri par l'ecran");
    dire!(j, "change quand l'ecran change. Si l'ilot est pose sur la rangee des");
    dire!(j, "chiffres, la portion de bord que traverse un fut bascule CHAQUE");
    dire!(j, "SECONDE, a la peripherie du regard, dans le noir.");
    dire!(j, "");
    dire!(j, "On rend donc la meme scene a deux secondes consecutives — un fut");
    dire!(j, "deplace de 14 px — et l'on compte le plus grand ecart en niveaux");
    dire!(j, "sur les pixels du BORD, puis sur l'ilot entier.");
    let nuit2 = fond_horloge(true);
    dire!(j, "");
    let battement = |d: &Direction| -> (f64, f64) {
        let a = v.rendre(&nuit, d);
        let b = v.rendre(&nuit2, d);
        let (mut bord, mut coeur) = (0.0f64, 0.0f64);
        for y in 0..ih() {
            for x in 0..il() {
                let dd = distance(x as f32, y as f32) as f64;
                if dd > -0.45 {
                    continue;
                }
                let i = ((y * il() + x) * 4) as usize;
                let e = (a[i + 1] as f64 - b[i + 1] as f64).abs();
                // Le coeur doit etre HORS du conge, sans quoi la colonne de
                // controle mesure l'effet qu'elle est censee servir de temoin.
                // Prise a -6 px elle tombait en plein dans la bande de clarte,
                // et rapportait 47 niveaux qui etaient ceux de l'effet.
                if dd >= -6.0 {
                    bord = bord.max(e);
                } else if dd < -(CONGE as f64 + 2.0) {
                    coeur = coeur.max(e);
                }
            }
        }
        (bord, coeur)
    };
    dire!(j, "  direction          battement du BORD   battement du COEUR");
    dire!(j, "  -------------------------------------------------------------");
    for d in [&TEMOIN, &DALLE, &V3, &V6] {
        let (bo, co) = battement(d);
        dire!(j, "  {:<18} {bo:>13.1} niv {co:>16.1} niv", d.nom);
    }
    dire!(j, "");
    dire!(j, "  La colonne de droite est le TEMOIN de celle de gauche : elle dit");
    dire!(j, "  ce que le changement de seconde fait deja au verre LOIN du bord,");
    dire!(j, "  ou aucun effet de ce chantier n'intervient. Un battement de bord");
    dire!(j, "  qui ne la depasse pas n'ajoute rien a ce que l'ilot fait deja.");

    // -- E. LA FORCE DE LA REFLEXION, RESOLUE PAR LA LOI DU PROJET --
    dire!(j, "");
    dire!(j, "-- E. JUSQU'OU LA REFLEXION PEUT ALLER DANS CETTE CHAMBRE --");
    dire!(j, "La physique donne une force de 1,00 et elle ne se discute pas :");
    dire!(j, "c'est ce qu'une dalle de verre renvoie. Les essais C et D disent");
    dire!(j, "ce que cette force coute ICI, et ce n'est pas negociable non plus.");
    dire!(j, "On balaie donc la force et on lit les trois grandeurs qui decident :");
    dire!(j, "le pic de nuit, le battement de seconde, et ce qui reste du lisere");
    dire!(j, "sur un bureau ordinaire — car un lisere eteint ne vaut rien.");
    dire!(j, "");
    dire!(j, "  force   pic de nuit   battement   lisere sur 226   contraste du coeur");
    dire!(j, "  ------------------------------------------------------------------------");
    let encre = (0xF2u8, 0xEFu8, 0xE9u8);
    for force in [-1.0f32, 0.0, 0.05, 0.10, 0.15, 0.20, 0.30, 0.50, 1.00] {
        // -1 est le temoin de ce tableau : le verre SEUL, sans biseau du tout.
        // Il dit ce que la scene de nuit fait deja a l'ilot, et c'est contre
        // lui que se lit tout le reste de la colonne.
        let d = if force < 0.0 {
            V0
        } else {
            Direction { lisere: force, ..V3 }
        };
        let (p, _, _) = pic_du_bord(&v.rendre(&nuit, &d), -6.0);
        let (bo, _) = battement(&d);
        let px = v.rendre(&fond, &d);
        let pr = profil_normal(&px, Zone::Droit, 12, TOLERANCE);
        let l = au(&pr, -0.5).unwrap_or(f64::NAN);
        let coeur = au(&pr, -11.5).unwrap_or(f64::NAN);
        let c = contraste_l(
            luminance_relative(encre.0, encre.1, encre.2),
            luminance_relative(coeur as u8, coeur as u8, coeur as u8),
        );
        if force < 0.0 {
            dire!(
                j,
                "  temoin {p:>10.1} niv {bo:>9.1} niv {l:>13.1} niv {c:>17.2}"
            );
        } else {
            dire!(
                j,
                "  {force:>5.2} {p:>11.1} niv {bo:>9.1} niv {l:>13.1} niv {c:>17.2}"
            );
        }
    }
    dire!(j, "");
    dire!(j, "  Le coeur ne bouge pas d'une ligne a l'autre : la force du lisere");
    dire!(j, "  ne touche PAS la zone d'encre, et la colonne de droite est la");
    dire!(j, "  pour le prouver plutot que pour l'affirmer.");

    // -- E bis. LA POSITION PAR DEFAUT, QUE LA SCENE DE NUIT AVAIT SUPPOSEE --
    dire!(j, "");
    dire!(j, "-- E bis. ET A LA PLACE OU L'ILOT SE TROUVE VRAIMENT --");
    dire!(j, "Les essais C, D et E supposent tous l'ilot POSE SUR LA RANGEE DES");
    dire!(j, "CHIFFRES. C'est une position que l'auteur peut choisir, et ce n'est");
    dire!(j, "pas celle qu'il trouve : `DESIGN.md` §6 pose l'ilot « centre");
    dire!(j, "horizontalement, a 24 px du bord haut — la zone que l'horloge laisse");
    dire!(j, "VIDE au-dessus de la date ». Derriere lui, par defaut, il n'y a donc");
    dire!(j, "que du noir pur. Mesurer le pire cas sans mesurer le cas normal");
    dire!(j, "aurait fait passer une exception pour la regle.");
    let noir = uni(0);
    dire!(j, "");
    dire!(j, "  direction          pic du bord   coeur   ce que le bord AJOUTE");
    dire!(j, "  ------------------------------------------------------------------");
    for d in [&V0, &V3, &V6] {
        let px = v.rendre(&noir, d);
        let (p, _, _) = pic_du_bord(&px, -6.0);
        let c = niveau_coeur(&px);
        dire!(j, "  {:<18} {p:>9.1} niv {c:>7.1} {:>17.1} niv", d.nom, p - c);
    }
    dire!(j, "");
    dire!(j, "  Sur du noir, la reflexion n'a RIEN a reflechir : elle vaut R fois");
    dire!(j, "  zero. L'acquis de `VERRE-TAILLE.md` §5 — « la nuit, l'ilot ne");
    dire!(j, "  dessine plus de cadre » — n'est donc pas entame a la place ou");
    dire!(j, "  l'ilot se tient par defaut, et c'est mesure et non suppose.");

    // -- F. LA COUCHE NETTE : COMBIEN, ET SELON QUEL PROFIL --
    dire!(j, "");
    dire!(j, "-- F. LA COUCHE NETTE RAMENEE AU BISEAU --");
    dire!(j, "Sans elle la refraction est INVISIBLE : le flou a sigma 18 a deja");
    dire!(j, "detruit la structure que le biseau aurait du tordre, et l'energie");
    dire!(j, "de structure du bord retombe a 0,00. Avec elle a pleine force, la");
    dire!(j, "planche montre autre chose que ce que le chiffre annoncait : sur un");
    dire!(j, "bureau raye a 3 px, l'anneau de 12 px devient un PEIGNE. La mesure");
    dire!(j, "de `MATIERE.md` §5.4 ne l'attrape pas — elle borne le bord a ne pas");
    dire!(j, "etre PLUS net que la realite, et un peigne ne l'est pas : il l'est");
    dire!(j, "exactement autant.");
    dire!(j, "");
    dire!(j, "  profil       force   hf du bord   hf du coeur   pic de nuit   battement");
    dire!(j, "  --------------------------------------------------------------------------");
    let bureau = fond_bureau();
    for (nom, profil, force) in [
        ("aucune    ", 0.0f32, 0.0f32),
        ("marche    ", 0.0, 0.35),
        ("marche    ", 0.0, 0.60),
        ("marche    ", 0.0, 1.00),
        ("rampe (1-t)^2", 1.0, 0.35),
        ("rampe (1-t)^2", 1.0, 0.60),
        ("rampe (1-t)^2", 1.0, 1.00),
    ] {
        let d = Direction {
            clarte: force,
            profil,
            ..V3
        };
        let px = v.rendre(&bureau, &d);
        let hb = hf_bande(&px, il(), ih(), 1, 8);
        let hc = hf_bande(&px, il(), ih(), 24, il() / 2);
        let (p, _, _) = pic_du_bord(&v.rendre(&nuit, &d), -6.0);
        let (bo, _) = battement(&d);
        dire!(
            j,
            "  {nom:<13} {force:>5.2} {hb:>11.2} {hc:>13.2} {p:>11.1} niv {bo:>9.1} niv"
        );
    }
    dire!(j, "");
    dire!(j, "  La colonne « hf du coeur » est le temoin : la couche nette ne doit");
    dire!(j, "  rien changer loin du bord, et si elle bouge c'est que la marche");
    dire!(j, "  deborde du conge.");

    // -- LES PLANCHES, sur lesquelles le reglage se choisit --
    let reel = fond_bureau_reel();
    ecrire_png("captures/mesure/planche-reel-fond.png", &reel[..], SL, SH);
    for (nom, f) in [
        ("nuit", &nuit),
        ("bureau", &bureau),
        ("coupe", &coupe),
        ("reel", &reel),
    ] {
        ecrire_png(
            &format!("captures/mesure/planche-{nom}-0-temoin.png"),
            &v.rendre(f, &V0),
            il(),
            ih(),
        );
        for force in [0.10f32, 0.15, 0.30, 1.00] {
            let d = Direction { lisere: force, ..V3 };
            ecrire_png(
                &format!("captures/mesure/planche-{nom}-{:03.0}.png", force * 100.0),
                &v.rendre(f, &d),
                il(),
                ih(),
            );
        }
        ecrire_png(
            &format!("captures/mesure/planche-{nom}-beer.png"),
            &v.rendre(f, &V6),
            il(),
            ih(),
        );
    }
    for (nom, profil, force) in [
        ("poli-000", 0.0f32, 0.0f32),
        ("poli-035", 0.0, 0.35),
        ("poli-100", 0.0, 1.00),
        ("rampe-100", 1.0, 1.00),
    ] {
        let d = Direction { clarte: force, profil, ..V3 };
        ecrire_png(
            &format!("captures/mesure/planche-reel-{nom}.png"),
            &v.rendre(&reel, &d),
            il(),
            ih(),
        );
    }
    dire!(j, "");
    dire!(j, "  planches du verrier ecrites : planche-<scene>-<force>.png");
}

// ===========================================================================

/// La planche que l'instrument 7 ne produit pas, et qui manquait.
///
/// L'objection de l'esthete etait VISUELLE : le galet replie devenait « un
/// cadre gris perce d'une fenetre rectangulaire claire ». Une etendue qui
/// tombe de 52,4 a 1,1 point est convaincante, mais elle ne se regarde pas.
/// Ces six rendus se regardent : trois tailles, deux lois de conge, le meme
/// fond de bureau vraisemblable.
fn planche_trois_tailles(v: &Verre) {
    let fond = fond_bureau_reel();
    // Les trois etats que le ressort traverse, avec le rayon et l'exposant que
    // DESIGN.md §6 fait deja suivre la hauteur.
    let etats: [(&str, u32, u32, f32, f32); 3] = [
        ("replie", 88, 36, 18.0, 2.5),
        ("mi-chemin", 204, 92, 33.0, 3.25),
        ("deploye", 320, 148, 48.0, 4.0),
    ];
    for (nom, l, h, r, n) in etats {
        geometrie::poser(l, h, r, n);
        for (loi, d) in [("fixe", &ANCIENNE), ("relatif", &RETENUE)] {
            let px = v.rendre(&fond, d);
            ecrire_png(
                &format!("captures/mesure/tailles-{nom}-{loi}.png"),
                &px,
                il(),
                ih(),
            );
        }
    }
    // On repose la geometrie par defaut : un banc qui laisse un etat derriere
    // lui fait mentir la mesure suivante.
    geometrie::deploye();
}

/// Les vignettes que le second esthete relit, et rien d'autre.
///
/// Les planches du premier passage n'avaient que trois etats sur aplat, et
/// seulement la retenue aux deux etats intermediaires du ressort : on ne pouvait
/// donc rien retrancher du verre seul a p = 0,25 ni a p = 0,75. Ici chaque
/// direction a sa vignette a chaque etat et sur chaque fond, et la quatrieme
/// colonne est la retenue SANS REFLEXION : c'est elle qui separe ce que la couche
/// nette apporte au bord de ce que le lisere y apporte.
fn vignettes_du_second_esthete(v: &Verre) {
    const SANS_REFLET: Direction = Direction {
        nom: "RETENUE sans reflet",
        lisere: 0.0,
        ..RETENUE
    };
    let fonds: [(&str, Vec<u8>); 8] = [
        ("clair", uni(226)),
        ("sombre", uni(37)),
        ("raye", fond_bureau()),
        ("raye-gris", fond_bureau_gris()),
        ("nuit", fond_horloge(false)),
        ("nuit-suivante", fond_horloge(true)),
        ("coupe", fond_coupe()),
        ("reel", fond_bureau_reel()),
    ];
    let dirs: [(&str, &Direction); 4] = [
        ("verre", &V0),
        ("ancienne", &ANCIENNE),
        ("retenue", &RETENUE),
        ("sans-reflet", &SANS_REFLET),
    ];
    std::fs::create_dir_all("captures/mesure/tour2").unwrap();
    for k in 0..5 {
        let (l, h, r, n) = etat_du_ressort(k as f64 / 4.0);
        geometrie::poser(l, h, r, n);
        for (nf, f) in fonds.iter() {
            for (nd, d) in dirs {
                let px = v.rendre(f, d);
                ecrire_png(
                    &format!("captures/mesure/tour2/{nf}-p{k}-{nd}.png"),
                    &px,
                    il(),
                    ih(),
                );
            }
        }
    }
    geometrie::deploye();
}

fn main() {
    let (v, adaptateur, horodatage) = Verre::nouveau();
    std::fs::create_dir_all("captures/mesure").unwrap();
    let mut j = Journal(String::new());
    let j = &mut j;

    dire!(j, "LE BANC DU METROLOGUE — six instruments, et le temoin de chacun");
    dire!(j, "adaptateur        : {adaptateur}");
    dire!(
        j,
        "horodatage GPU    : {}",
        if horodatage { "disponible" } else { "ABSENT" }
    );
    dire!(j, "nuanceurs eprouves: matiere.wgsl (esquisse) pour A, B, C — INCHANGE ;");
    dire!(j, "                    verrier.wgsl pour V0 a V6, qui porte la RETENUE.");
    dire!(j, "");
    dire!(j, "La regle de ce banc, et c'est la seule : un instrument qui n'a pas");
    dire!(j, "ete valide sur un cas dont la reponse etait connue D'AVANCE ne sert");
    dire!(j, "a juger de rien. Quand un temoin echoue, la mesure n'est pas");
    dire!(j, "imprimee — un chiffre faux est pire qu'un chiffre absent, parce");
    dire!(j, "qu'on s'en sert.");

    let dirs = [
        TEMOIN, DALLE, LENTILLE, DEPOLI, V0, V2_FLOUE, V2, V3, V6, ANCIENNE, RETENUE,
    ];

    // `mesure cout` ne lance que l'instrument 6, sur un GPU qui n'a encore rien
    // fait. C'est ce qui permet de savoir si un temps mesure a la fin d'un banc
    // long est le temps du travail ou celui d'un GPU chaud.
    if std::env::args().any(|a| a == "cout") {
        dire!(j, "");
        dire!(j, "LANCEMENT FROID : instrument 6 seul, GPU au repos.");
        instrument_6(&v, j, &dirs, horodatage);
        std::fs::write("captures/mesure/cout-froid.txt", &j.0).unwrap();
        return;
    }

    // `mesure integration` ecrit, pour le pas 2 de l'integration, les fonds du
    // banc et le rendu de l'ARBITREE sur chacun, deploye et replie. L'application
    // rend ensuite les memes fonds par son propre chemin (`--capture-ilot
    // --verre-fond`), et `outils/verre-contre-banc.py` compare. Rien d'autre ne
    // tourne, et le journal n'est pas reecrit.
    if std::env::args().any(|a| a == "integration") {
        let dossier = "../../captures/integration/pas2-banc";
        std::fs::create_dir_all(dossier).unwrap();
        let blanc = vec![255u8; (SL * SH * 4) as usize];
        let fonds: [(&str, Vec<u8>); 7] = [
            ("page-texte", fond_page_texte()),
            ("bureau-reel", fond_bureau_reel()),
            ("bureau-raye", fond_bureau()),
            ("bureau-sombre", fond_bureau_sombre()),
            ("horloge", fond_horloge(false)),
            ("coupe", fond_coupe()),
            ("blanc", blanc),
        ];
        // Les deux etats de DESIGN.md §6 a l'echelle 1 : deploye, et la pastille
        // repliee de 88 x 36 que l'application rend pour « 41:27 ».
        let etats: [(&str, u32, u32, f32, f32); 2] =
            [("deploye", 320, 148, 48.0, 4.0), ("replie", 88, 36, 18.0, 2.5)];
        for (nom, fond) in &fonds {
            ecrire_png(&format!("{dossier}/{nom}-fond.png"), fond, SL, SH);
            for (etat, l, h, r, n) in etats {
                geometrie::poser(l, h, r, n);
                let px = v.rendre(fond, &ARBITREE);
                ecrire_png(&format!("{dossier}/{nom}-{etat}-arbitree.png"), &px, l, h);
            }
            geometrie::deploye();
            println!("{nom} : fond {SL}x{SH}, ARBITREE deploye et replie");
        }
        return;
    }

    // `mesure esthete` ne rend que les vignettes du second esthete, et
    // n'ecrit pas le journal : les chiffres sont lus sur ces images par
    // `outils/esthete-tour2.py`.
    if std::env::args().any(|a| a == "esthete") {
        vignettes_du_second_esthete(&v);
        return;
    }

    // `mesure lampe` : la direction ARBITREE-LAMPE (`DECISION-APRES-EPREUVE.md`
    // E2) par les instruments 1 a 5, puis l'essai de la lampe et ses temoins ;
    // `mesure lampe epreuve` double L, et les temoins du noir et du blanc
    // doivent tomber. Le journal va dans `lampe.txt` (ou `lampe-epreuve.txt`),
    // et `banc.txt` n'est pas reecrit.
    if std::env::args().any(|a| a == "lampe") {
        let epreuve = std::env::args().any(|a| a == "epreuve");
        let dirs_lampe = [
            TEMOIN, DALLE, LENTILLE, DEPOLI, V0, V2_FLOUE, V2, V3, V6, ANCIENNE, RETENUE, ARBITREE,
            arbitree_lampe(),
        ];
        let v1 = instrument_1(&v, j, &dirs_lampe);
        let v2 = instrument_2(&v, j, &dirs_lampe);
        let v3 = instrument_3(&v, j, &dirs_lampe);
        let v4 = instrument_4(&v, j, &dirs_lampe);
        let v5 = instrument_5(&v, j, &dirs_lampe);
        let vl = lampe(&v, j, epreuve);
        dire!(j, "");
        dire!(j, "===========================================================");
        dire!(j, "L'ETAT DES INSTRUMENTS, AVEC LA LAMPE");
        dire!(j, "===========================================================");
        for (nom, ok) in [
            ("1. la distorsion", v1),
            ("2. la reflectance du bord", v2),
            ("3. la dispersion", v3),
            ("4. la matiere, et non l'opacite", v4),
            ("5. la lisibilite", v5),
            ("I. la lampe d'ecran", vl),
        ] {
            dire!(j, "  {nom:<34} {}", if ok { "VALIDE — ses mesures valent" } else { "FAUX — ses mesures ne valent rien" });
        }
        std::fs::write(
            if epreuve { "captures/mesure/lampe-epreuve.txt" } else { "captures/mesure/lampe.txt" },
            &j.0,
        )
        .unwrap();
        return;
    }

    // La direction ARBITREE passe aussi par les instruments 1 a 5 : ce sont
    // eux qui disent qu'elle ne deforme, ne disperse et ne degrade rien de plus
    // que la RETENUE. Elle ne passe pas par l'instrument 6 : elle a le meme
    // nuanceur et les memes liaisons que la RETENUE, seule la valeur d'un
    // uniforme change, et un instrument instable n'a rien a dire de plus.
    let dirs_arbitre = [
        TEMOIN, DALLE, LENTILLE, DEPOLI, V0, V2_FLOUE, V2, V3, V6, ANCIENNE, RETENUE, ARBITREE,
    ];
    let v1 = instrument_1(&v, j, &dirs_arbitre);
    let v2 = instrument_2(&v, j, &dirs_arbitre);
    let v3 = instrument_3(&v, j, &dirs_arbitre);
    let v4 = instrument_4(&v, j, &dirs_arbitre);
    let v5 = instrument_5(&v, j, &dirs_arbitre);
    let v6 = instrument_6(&v, j, &dirs, horodatage);
    let v7 = instrument_7(&v, j);
    let v8 = instrument_8(&v, j);
    let v9 = instrument_9(&v, j);
    chantier_du_verrier(&v, j);

    // Les planches.
    for (nom, f) in [
        ("clair", uni(226)),
        ("sombre", uni(37)),
        ("bureau", fond_bureau()),
        ("bureau-sombre", fond_bureau_sombre()),
        ("bureau-gris", fond_bureau_gris()),
    ] {
        for d in &dirs {
            let px = v.rendre(&f, d);
            let court = match d.nom.chars().next().unwrap() {
                'A' => "A",
                'B' => "B",
                'C' => "C",
                _ => "temoin",
            };
            ecrire_png(&format!("captures/mesure/{court}-{nom}.png"), &px, il(), ih());
        }
    }

    dire!(j, "");
    dire!(j, "===========================================================");
    dire!(j, "L'ETAT DES SIX INSTRUMENTS");
    dire!(j, "===========================================================");
    let etat = [
        ("1. la distorsion", v1),
        ("2. la reflectance du bord", v2),
        ("3. la dispersion", v3),
        ("4. la matiere, et non l'opacite", v4),
        ("5. la lisibilite", v5),
        ("6. le cout", v6),
        ("7. la matiere a trois tailles", v7),
        ("8. le bord : marche, dents, niveau", v8),
        ("9. les trois angles morts", v9),
    ];
    for (nom, ok) in etat {
        dire!(
            j,
            "  {nom:<34} {}",
            if ok {
                "VALIDE — ses mesures valent"
            } else {
                "FAUX — ses mesures ne valent rien"
            }
        );
    }
    let combien = etat.iter().filter(|(_, ok)| *ok).count();
    dire!(j, "");
    dire!(j, "  {combien} instruments sur {} sont valides.", etat.len());
    dire!(j, "");
    planche_trois_tailles(&v);
    dire!(j, "  planches ecrites dans captures/mesure/");
    dire!(j, "  dont tailles-<etat>-<loi>.png : le galet aux trois tailles,");
    dire!(j, "  sous conge fixe et sous conge relatif, sur le meme fond.");

    std::fs::write("captures/mesure/banc.txt", &j.0).unwrap();
}
