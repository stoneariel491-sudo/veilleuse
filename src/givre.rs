//! Le givre : la croissance des cristaux sur le galet, calculee UNE fois sur le
//! processeur et ecrite dans une texture que le nuanceur lit a chaque trame.
//!
//! Pourquoi une croissance et non un bruit : un bruit pose sur l'image se lit
//! comme de la fumee ou du marbre (`JUGE-DESIGN.md` G1). Le givre d'une vitre est
//! une croissance : des pointes qui partent des defauts, avancent a vitesse a
//! peu pres constante, se ramifient a 60° (la maille hexagonale de la glace), et
//! laissent derriere elles une mosaique de petits cristaux qui s'epaissit plus
//! lentement. C'est ce que la production simule (Lipton, Museth, Sutherland,
//! SIGGRAPH 2013 : des pointes advectees qui laissent des lignes) et ce que la
//! physique decrit (Libbrecht : croissance limitee par la diffusion, instabilite
//! de branchement). `GIVRE.md` §1 cite les sources.
//!
//! Ce que la texture porte, texel par texel, sur le rectangle du galet deploye :
//!
//! - **R, l'arrivee `tau`** dans [0, 1] : l'instant, rapporte au dernier cristal,
//!   ou le givre atteint ce texel. Les pointes des fougeres arrivent en premier,
//!   la mosaique suit, et le coeur du galet est le dernier. Le nuanceur compare
//!   `tau` a la croissance `g` : c'est tout le front.
//! - **G, la modulation `mu / 4`** : ce que la structure fait a la lumiere
//!   transmise, de moyenne 1 sur le galet. Les tiges guident la lumiere et sont
//!   plus claires ; les aretes des cellules la devient et sont plus sombres. La
//!   moyenne vaut 1 pour que le givre ne change pas la luminance moyenne du verre
//!   qu'il precede : la structure redistribue la lumiere, elle n'en invente pas.
//! - **B, l'epaisseur `rho`** : 1 sur les tiges, moins dans la mosaique. La fonte
//!   s'en sert : le mince fond avant l'epais, et les squelettes des fougeres
//!   restent les derniers sur le verre.
//! - **A** : plein.
//!
//! Tout est en pixels LOGIQUES, sur le galet deploye de `DESIGN.md` §6 ; la
//! texture est sur-echantillonnee deux fois, et le nuanceur la lit en
//! coordonnees normalisees du galet. Le meme motif sert donc a toutes les
//! echelles, comme toutes les cotes de la charte.
//!
//! Le generateur est deterministe : meme graine, memes octets. C'est ce qui rend
//! les planches refaisables et les temoins comparables a l'octet.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Sur-echantillonnage de la texture par rapport aux pixels logiques.
pub const SURECHANTILLON: u32 = 2;

/// La graine du motif. Une constante : le givre de l'ilot est toujours le meme,
/// comme le serait celui d'une vitre donnee.
pub const GRAINE: u64 = 0x6976_7265_2032_3032;

/// Vitesse des pointes principales, en pixels logiques par tic. C'est l'unite
/// de temps de la croissance : un tic est le temps qu'une pointe met a faire un
/// pixel.
const VITESSE_TIGE: f32 = 1.0;
/// Les rameaux lateraux et leurs sous-rameaux vont moins vite : ils partent
/// plus tard et restent plus courts que la tige, ce qui donne la forme en plume.
const VITESSE_RAMEAU: f32 = 0.8;
const VITESSE_SOUS_RAMEAU: f32 = 0.55;
/// La mosaique entre les fougeres s'epaissit bien plus lentement que les
/// pointes n'avancent : c'est ce qui fait que les plumes precedent le voile.
const VITESSE_MOSAIQUE: f32 = 0.2;
/// Angle des rameaux sur leur tige : la maille hexagonale de la glace.
const ANGLE_RAMEAU: f32 = 60.0;
/// Espacement des rameaux le long d'une tige, et des sous-rameaux le long d'un
/// rameau, en pixels logiques.
const PAS_RAMEAU: f32 = 3.5;
const PAS_SOUS_RAMEAU: f32 = 2.5;
/// Longueur d'une plume : une tige s'arrete entre ces deux bornes, tiree au
/// sort, si le bord ou un autre cristal ne l'a pas arretee avant. Sur une
/// vitre, une plume fait quelques centimetres ; sur le galet, un quart a un
/// tiers de sa largeur.
const LONGUEUR_TIGE: (f32, f32) = (55.0, 110.0);
/// Courbure d'une tige, en degres par pixel, tiree au sort entre plus et moins
/// cette valeur : une tige de givre est un arc, jamais une droite.
const COURBURE_TIGE: f32 = 0.4;
/// Longueurs maximales des rameaux et sous-rameaux. Chaque rameau tire la
/// sienne (`0,25 + 0,75 u²` de la valeur), et elle decroit de la racine de la
/// tige a sa pointe (`0,35 + 0,65 x (1 - avancement)`) : c'est la forme en
/// plume, large a la base et effilee au bout, et son irregularite.
const LONGUEUR_RAMEAU: f32 = 32.0;
const LONGUEUR_SOUS_RAMEAU: f32 = 7.0;
/// Un rameau sur quatre manque : une plume n'est pas un peigne.
const RAMEAU_MANQUANT: f32 = 0.25;
/// Un rameau ne porte des sous-rameaux que s'il est plus long que ceci.
const RAMEAU_RAMIFIE: f32 = 10.0;
/// Largeurs des tiges, rameaux et sous-rameaux, en pixels logiques.
const LARGEURS: [f32; 3] = [2.0, 1.2, 0.8];
/// Une tige s'amincit vers sa pointe : sa largeur vaut sa base a la racine et
/// `AMINCISSEMENT` fois moins au bout de `PORTEE_AMINCISSEMENT` pixels.
const AMINCISSEMENT: f32 = 0.55;
const PORTEE_AMINCISSEMENT: f32 = 150.0;
/// Espacement du semis des cellules de la mosaique, en pixels logiques.
const PAS_CELLULE: f32 = 3.0;
/// Epaisseur de la mosaique, de base et de dispersion, sur [0, 1].
const EPAISSEUR_MOSAIQUE: (f32, f32) = (0.35, 0.25);
/// Ce que la structure fait a la lumiere, avant que la moyenne soit ramenee a
/// un : une tige guide la lumiere et vaut `1 + GAIN_TIGE` fois la mosaique ;
/// une cellule de la mosaique tire au sort sa clarte entre `CELLULE.0` et
/// `CELLULE.0 + CELLULE.1` ; une arete de cellule devie la lumiere et perd
/// `PERTE_ARETE`.
const GAIN_TIGE: f32 = 1.6;
const CELLULE: (f32, f32) = (0.5, 1.0);
const PERTE_ARETE: f32 = 0.4;
/// Largeur d'une arete de cellule, en pixels logiques.
const LARGEUR_ARETE: f32 = 0.5;
/// Sites de nucleation le long des grands bords (haut et bas) et des petits
/// (gauche et droite), en plus des quatre coins.
const SITES_GRAND_BORD: usize = 3;
const SITES_PETIT_BORD: usize = 1;
/// Retard maximal des sites de bord, en tics.
const RETARD_BORD: f32 = 12.0;
/// Defauts interieurs (poussieres) : leur nombre, leur retard, leur eloignement
/// minimal du bord.
const DEFAUTS: usize = 2;
const RETARD_DEFAUT: (f32, f32) = (40.0, 80.0);
const RECUL_DEFAUT: f32 = 25.0;
/// Pas d'integration des pointes, en pixels logiques.
const PAS: f32 = 0.5;
/// Garde-fou : au-dela, la croissance est arretee.
const TICS_MAXIMUM: f32 = 600.0;

/// La texture des cristaux et ce qu'il faut savoir pour la lire.
pub struct Cristaux {
    pub largeur: u32,
    pub hauteur: u32,
    /// RGBA8, ligne par ligne, du haut vers le bas.
    pub texels: Vec<u8>,
    /// Largeur du front de croissance, en unites de `tau` : le demi-espacement
    /// des cellules rapporte a la course de la mosaique. Le nuanceur allume une
    /// cellule sur cette largeur.
    pub largeur_front: f32,
    /// Duree de la croissance, en tics : l'arrivee du dernier cristal.
    pub duree: f32,
    /// Ce que la croissance a fait, pour le journal des captures.
    pub sites: usize,
    pub segments: usize,
}

// --- Le hasard, deterministe ------------------------------------------------

struct Hasard(u64);

impl Hasard {
    fn nouveau(graine: u64) -> Hasard {
        Hasard(graine ^ 0x9E37_79B9_7F4A_7C15)
    }

    /// splitmix64 : court, sans etat cache, et de qualite suffisante pour un
    /// motif.
    fn suivant(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Un reel dans [0, 1).
    fn reel(&mut self) -> f32 {
        (self.suivant() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn entre(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.reel()
    }
}

// --- Les pointes ------------------------------------------------------------

struct Pointe {
    x: f32,
    y: f32,
    /// Cap, en radians.
    cap: f32,
    vitesse: f32,
    /// 0 tige, 1 rameau, 2 sous-rameau.
    profondeur: u8,
    /// Instant courant de la pointe, en tics.
    t: f32,
    /// Distance parcourue depuis le dernier rameau, et cote du prochain.
    depuis_rameau: f32,
    cote: f32,
    /// Longueur parcourue, et longueur maximale (infinie pour une tige).
    longueur: f32,
    longueur_max: f32,
    /// Longueur sous laquelle la pointe ne teste pas la collision : elle est
    /// encore sur sa tige mere.
    franchise: f32,
    /// Courbure propre, en radians par pixel.
    courbure: f32,
}

/// Un morceau de branche, pour la rasterisation.
struct Segment {
    x0: f32,
    y0: f32,
    t0: f32,
    x1: f32,
    y1: f32,
    t1: f32,
    largeur: f32,
}

// --- La croissance ----------------------------------------------------------

/// Fait pousser le givre sur un galet de `l` x `h` pixels logiques, aux coins de
/// rayon `rayon` et d'exposant `n` (la superellipse de `galet.wgsl`).
pub fn pousser(l: f32, h: f32, rayon: f32, n: f32, graine: u64) -> Cristaux {
    pousser_avec(l, h, rayon, n, graine, true)
}

/// La meme pousse, avec ou sans la correction des tiges tardives (m-g3).
/// `false` est l'epreuve du temoin (`--epreuve-tiges`) : la texture d'avant, qui
/// en compte 2 298, et le rendu d'avant la correction.
pub fn pousser_avec(l: f32, h: f32, rayon: f32, n: f32, graine: u64, tiges_precoces: bool) -> Cristaux {
    let mut hasard = Hasard::nouveau(graine);
    let distance = |x: f32, y: f32| crate::galet::distance_au_galet(x, y, l, h, rayon, n);

    // Le semis des pointes : les coins, les bords, puis les defauts interieurs.
    let mut pointes: Vec<Pointe> = Vec::new();
    let mut sites = 0usize;
    let semer = |pointes: &mut Vec<Pointe>, hasard: &mut Hasard, x: f32, y: f32, cap: f32, t: f32| {
        pointes.push(Pointe {
            x,
            y,
            cap,
            vitesse: VITESSE_TIGE,
            profondeur: 0,
            t,
            depuis_rameau: 0.0,
            cote: 1.0,
            longueur: 0.0,
            longueur_max: hasard.entre(LONGUEUR_TIGE.0, LONGUEUR_TIGE.1),
            franchise: 0.0,
            courbure: hasard.entre(-COURBURE_TIGE, COURBURE_TIGE).to_radians(),
        });
    };
    let rad = |deg: f32| deg.to_radians();

    // Les coins : le point de la superellipse a 45°, deux tiges de part et
    // d'autre de la diagonale rentrante. Un coin de vitre est le point le plus
    // froid et le plus charge en defauts du cadre : il part le premier.
    let recul = rayon * (1.0 - 0.5f32.powf(1.0 / n));
    for (sx, sy) in [(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
        let cx = if sx > 0.0 { recul } else { l - recul };
        let cy = if sy > 0.0 { recul } else { h - recul };
        let diagonale = sy.atan2(sx);
        for ecart in [-25.0f32, 25.0] {
            let cap = diagonale + rad(ecart + hasard.entre(-6.0, 6.0));
            let t = hasard.entre(0.0, 3.0);
            semer(&mut pointes, &mut hasard, cx, cy, cap, t);
        }
        sites += 1;
    }
    // Les bords droits : des sites repartis avec un peu de jeu, une tige chacun,
    // qui rentre a peu pres perpendiculairement.
    let mut bord = |pointes: &mut Vec<Pointe>, hasard: &mut Hasard, nombre: usize, horizontal: bool, cote: f32| {
        let course = if horizontal { l - 2.0 * rayon } else { h - 2.0 * rayon };
        for i in 0..nombre {
            let u = rayon + course * (i as f32 + 0.5 + hasard.entre(-0.3, 0.3)) / nombre as f32;
            let (x, y, normale) = if horizontal {
                (u, if cote > 0.0 { 0.0 } else { h }, if cote > 0.0 { rad(90.0) } else { rad(-90.0) })
            } else {
                (if cote > 0.0 { 0.0 } else { l }, u, if cote > 0.0 { 0.0 } else { rad(180.0) })
            };
            // Une plume par site, en eventail : deux tiges de part et d'autre
            // de la normale, comme aux coins.
            let biais = rad(hasard.entre(-15.0, 15.0));
            let retard = hasard.entre(0.0, RETARD_BORD);
            for ecart in [-25.0f32, 25.0] {
                let cap = normale + biais + rad(ecart + hasard.entre(-6.0, 6.0));
                semer(pointes, hasard, x, y, cap, retard);
            }
            sites += 1;
        }
    };
    bord(&mut pointes, &mut hasard, SITES_GRAND_BORD, true, 1.0);
    bord(&mut pointes, &mut hasard, SITES_GRAND_BORD, true, -1.0);
    bord(&mut pointes, &mut hasard, SITES_PETIT_BORD, false, 1.0);
    bord(&mut pointes, &mut hasard, SITES_PETIT_BORD, false, -1.0);
    // Les defauts interieurs : une poussiere fait une petite rosace a trois
    // branches, en retard sur les bords.
    let mut poses = 0;
    let mut essais = 0;
    while poses < DEFAUTS && essais < 1000 {
        essais += 1;
        let x = hasard.entre(0.0, l);
        let y = hasard.entre(0.0, h);
        if distance(x, y) > -RECUL_DEFAUT {
            continue;
        }
        let t = hasard.entre(RETARD_DEFAUT.0, RETARD_DEFAUT.1);
        let cap = hasard.entre(0.0, std::f32::consts::TAU);
        for k in 0..3 {
            semer(&mut pointes, &mut hasard, x, y, cap + k as f32 * rad(120.0), t);
        }
        poses += 1;
        sites += 1;
    }

    // La grille de gel, au pixel logique : l'instant ou chaque case a gele, ou
    // l'infini. Une pointe qui entre dans une case gelee par une autre branche
    // s'arrete : les fougeres ne se traversent pas.
    let (gl, gh) = (l.ceil() as usize + 1, h.ceil() as usize + 1);
    let mut gel = vec![f32::INFINITY; gl * gh];
    let case_de = |x: f32, y: f32| -> Option<usize> {
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let (i, j) = (x as usize, y as usize);
        if i >= gl || j >= gh {
            None
        } else {
            Some(j * gl + i)
        }
    };

    let mut segments: Vec<Segment> = Vec::new();
    // Les pointes avancent dans l'ordre du temps : on prend toujours la plus en
    // retard, pour que la collision se decide dans l'ordre reel de la
    // croissance et non dans l'ordre du tableau.
    let mut actives: Vec<Pointe> = pointes;
    while !actives.is_empty() {
        let mut rang = 0;
        for (i, p) in actives.iter().enumerate() {
            if p.t < actives[rang].t {
                rang = i;
            }
        }
        let mut p = actives.swap_remove(rang);
        if p.t > TICS_MAXIMUM {
            continue;
        }
        let (x0, y0, t0) = (p.x, p.y, p.t);
        // Le cap derive un peu, et la tige suit sa courbure propre : une
        // tige n'est jamais droite.
        let derive = if p.profondeur == 0 { 3.0 } else { 5.0 };
        p.cap += rad(hasard.entre(-derive, derive)) * PAS + p.courbure * PAS;
        let x1 = x0 + p.cap.cos() * PAS;
        let y1 = y0 + p.cap.sin() * PAS;
        let t1 = t0 + PAS / p.vitesse;
        // Hors du galet, ou sur un cristal deja la : la pointe meurt.
        if distance(x1, y1) > -0.3 {
            continue;
        }
        let Some(case) = case_de(x1, y1) else { continue };
        if p.longueur >= p.franchise && gel[case] < t1 - 1.5 {
            continue;
        }
        if gel[case] > t1 {
            gel[case] = t1;
        }
        let amincie = 1.0 - (1.0 - AMINCISSEMENT) * (p.longueur / PORTEE_AMINCISSEMENT).min(1.0);
        segments.push(Segment {
            x0,
            y0,
            t0,
            x1,
            y1,
            t1,
            largeur: LARGEURS[p.profondeur as usize] * amincie,
        });
        p.x = x1;
        p.y = y1;
        p.t = t1;
        p.longueur += PAS;
        p.depuis_rameau += PAS;
        // Les rameaux : a 60° de la tige, en alternant les cotes.
        let (pas_rameau, vitesse, longueur_max) = match p.profondeur {
            0 => (PAS_RAMEAU, VITESSE_RAMEAU, LONGUEUR_RAMEAU),
            1 => (PAS_SOUS_RAMEAU, VITESSE_SOUS_RAMEAU, LONGUEUR_SOUS_RAMEAU),
            _ => (f32::INFINITY, 0.0, 0.0),
        };
        if p.depuis_rameau >= pas_rameau * hasard.entre(0.75, 1.25) {
            p.depuis_rameau = 0.0;
            let cap = p.cap + p.cote * rad(ANGLE_RAMEAU + hasard.entre(-8.0, 8.0));
            p.cote = -p.cote;
            // La plupart des rameaux sont courts, quelques-uns longs ; un sur
            // quatre manque ; seuls les longs se ramifient a leur tour.
            let u = hasard.reel();
            let avancement = (p.longueur / p.longueur_max).min(1.0);
            let longueur_max = longueur_max * (0.25 + 0.75 * u * u) * (0.35 + 0.65 * (1.0 - avancement));
            if hasard.reel() >= RAMEAU_MANQUANT {
                let profondeur = if p.profondeur == 0 && longueur_max < RAMEAU_RAMIFIE { 2 } else { p.profondeur + 1 };
                actives.push(Pointe {
                    x: x1,
                    y: y1,
                    cap,
                    vitesse,
                    profondeur,
                    t: t1,
                    depuis_rameau: hasard.entre(0.0, 2.0),
                    cote: if hasard.reel() < 0.5 { 1.0 } else { -1.0 },
                    longueur: 0.0,
                    longueur_max,
                    franchise: LARGEURS[p.profondeur as usize] + 1.0,
                    courbure: hasard.entre(-1.5, 1.5).to_radians(),
                });
            }
        }
        if p.longueur < p.longueur_max {
            actives.push(p);
        }
    }

    // La mosaique : depuis le squelette, la couche s'epaissit a sa propre
    // vitesse. Dijkstra sur la grille logique, seize voisins pour une distance
    // presque euclidienne.
    let mut arrivee = vec![f32::INFINITY; gl * gh];
    let mut tas = BinaryHeap::new();
    for (i, &g) in gel.iter().enumerate() {
        if g.is_finite() {
            arrivee[i] = g;
            tas.push(Noeud { t: g, case: i });
        }
    }
    let voisins: [(i32, i32); 16] = [
        (1, 0), (-1, 0), (0, 1), (0, -1),
        (1, 1), (1, -1), (-1, 1), (-1, -1),
        (2, 1), (2, -1), (-2, 1), (-2, -1),
        (1, 2), (1, -2), (-1, 2), (-1, -2),
    ];
    while let Some(Noeud { t, case }) = tas.pop() {
        if t > arrivee[case] {
            continue;
        }
        let (i, j) = ((case % gl) as i32, (case / gl) as i32);
        for (di, dj) in voisins {
            let (ni, nj) = (i + di, j + dj);
            if ni < 0 || nj < 0 || ni >= gl as i32 || nj >= gh as i32 {
                continue;
            }
            if distance(ni as f32, nj as f32) > 1.0 {
                continue;
            }
            let voisin = (nj as usize) * gl + ni as usize;
            let nt = t + ((di * di + dj * dj) as f32).sqrt() / VITESSE_MOSAIQUE;
            if nt < arrivee[voisin] {
                arrivee[voisin] = nt;
                tas.push(Noeud { t: nt, case: voisin });
            }
        }
    }

    // Le semis des cellules : un site par case d'une grille de PAS_CELLULE, avec
    // du jeu (la construction de Worley). Chaque cellule gele d'un bloc, a
    // l'instant de son site : le front avance cristal par cristal.
    let (cl, ch) = ((l / PAS_CELLULE).ceil() as usize + 2, (h / PAS_CELLULE).ceil() as usize + 2);
    let mut cellules: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(cl * ch);
    for j in 0..ch {
        for i in 0..cl {
            let x = (i as f32 - 1.0 + hasard.reel()) * PAS_CELLULE;
            let y = (j as f32 - 1.0 + hasard.reel()) * PAS_CELLULE;
            let t = case_de(x.clamp(0.0, l - 0.01), y.clamp(0.0, h - 0.01))
                .map(|c| arrivee[c])
                .unwrap_or(f32::INFINITY)
                + hasard.entre(-3.0, 3.0);
            let rho = EPAISSEUR_MOSAIQUE.0 + EPAISSEUR_MOSAIQUE.1 * hasard.reel();
            cellules.push((x, y, t, rho));
        }
    }

    // La rasterisation, sur-echantillonnee : les tiges d'abord (couverture
    // anticrenelee et instant le long du segment), puis la mosaique la ou les
    // tiges ne sont pas.
    let s = SURECHANTILLON as f32;
    let (tl, th) = ((l * s).round() as usize, (h * s).round() as usize);
    let mut couverture = vec![0.0f32; tl * th];
    let mut t_tige = vec![f32::INFINITY; tl * th];
    for seg in &segments {
        let demi = seg.largeur / 2.0 + 0.5 / s;
        let (xa, xb) = (seg.x0.min(seg.x1) - demi, seg.x0.max(seg.x1) + demi);
        let (ya, yb) = (seg.y0.min(seg.y1) - demi, seg.y0.max(seg.y1) + demi);
        let (ia, ib) = (((xa * s).floor().max(0.0)) as usize, ((xb * s).ceil()).min(tl as f32) as usize);
        let (ja, jb) = (((ya * s).floor().max(0.0)) as usize, ((yb * s).ceil()).min(th as f32) as usize);
        let (dx, dy) = (seg.x1 - seg.x0, seg.y1 - seg.y0);
        let long2 = (dx * dx + dy * dy).max(1e-6);
        for j in ja..jb {
            for i in ia..ib {
                let (px, py) = ((i as f32 + 0.5) / s, (j as f32 + 0.5) / s);
                let u = (((px - seg.x0) * dx + (py - seg.y0) * dy) / long2).clamp(0.0, 1.0);
                let (qx, qy) = (seg.x0 + u * dx, seg.y0 + u * dy);
                let d = ((px - qx).powi(2) + (py - qy).powi(2)).sqrt();
                // Couverture d'un texel par la tige : un ruban d'un texel.
                let c = ((seg.largeur / 2.0 + 0.5 / s - d) * s).clamp(0.0, 1.0);
                if c > 0.0 {
                    let k = j * tl + i;
                    couverture[k] = couverture[k].max(c);
                    let t = seg.t0 + u * (seg.t1 - seg.t0);
                    if t < t_tige[k] {
                        t_tige[k] = t;
                    }
                }
            }
        }
    }

    let mut tau = vec![0.0f32; tl * th];
    let mut rho = vec![0.0f32; tl * th];
    let mut arete = vec![0.0f32; tl * th];
    let mut dedans = vec![false; tl * th];
    for j in 0..th {
        for i in 0..tl {
            let k = j * tl + i;
            let (px, py) = ((i as f32 + 0.5) / s, (j as f32 + 0.5) / s);
            dedans[k] = distance(px, py) < 0.0;
            // Les deux sites les plus proches, cherches sur les neuf cases
            // voisines du semis.
            let (ci, cj) = ((px / PAS_CELLULE) as i64 + 1, (py / PAS_CELLULE) as i64 + 1);
            let (mut f1, mut f2, mut site) = (f32::INFINITY, f32::INFINITY, 0usize);
            for dj in -1..=1i64 {
                for di in -1..=1i64 {
                    let (ni, nj) = (ci + di, cj + dj);
                    if ni < 0 || nj < 0 || ni >= cl as i64 || nj >= ch as i64 {
                        continue;
                    }
                    let m = (nj as usize) * cl + ni as usize;
                    let (sx, sy, _, _) = cellules[m];
                    let d = ((px - sx).powi(2) + (py - sy).powi(2)).sqrt();
                    if d < f1 {
                        f2 = f1;
                        f1 = d;
                        site = m;
                    } else if d < f2 {
                        f2 = d;
                    }
                }
            }
            let (_, _, t_cellule, rho_cellule) = cellules[site];
            let c = couverture[k];
            // Un texel de tige porte l'arrivee la plus precoce de ce qu'il
            // recouvre : une rosace tardive n'ecrit pas son arrivee par-dessus
            // une mosaique deja gelee (m-g3, `DECISION-APRES-EPREUVE.md` G).
            // Sans ce minimum, la tige d'un defaut interieur, semee a 40 tics
            // ou plus, restait noire au milieu d'une mosaique deja blanche :
            // les cristaux noirs en creux que le juge a vus.
            let t_tiges = if !t_tige[k].is_finite() {
                t_cellule
            } else if tiges_precoces {
                t_tige[k].min(t_cellule)
            } else {
                t_tige[k]
            };
            tau[k] = t_cellule * (1.0 - c) + t_tiges * c;
            rho[k] = rho_cellule * (1.0 - c) + c;
            arete[k] = (1.0 - (f2 - f1) / LARGEUR_ARETE).clamp(0.0, 1.0) * (1.0 - c);
        }
    }

    // La meme regle a la frontiere des cellules : un texel de tige, ou de son
    // ruban, n'arrive jamais apres un texel de mosaique qu'il touche. Les
    // cellules tirent leur instant a plus ou moins trois tics, et une tige qui
    // passe d'une cellule a l'autre heritait de la plus tardive. Deux passes,
    // chacune lue sur l'etat d'avant elle, donc sans propagation le long de la
    // tige : le ruban et le coeur prennent d'abord la plus precoce des
    // voisines de mosaique (couverture nulle) ; puis le coeur (couverture
    // pleine) prend la plus precoce de toutes ses voisines de moindre
    // couverture, ruban compris, parce que sur les octets un ruban mince se
    // lit comme de la mosaique et que le temoin le compte ainsi.
    let voisins_8 = |i: usize, j: usize| {
        let mut v = Vec::with_capacity(8);
        for dj in -1..=1i64 {
            for di in -1..=1i64 {
                let (ni, nj) = (i as i64 + di, j as i64 + dj);
                if (di == 0 && dj == 0) || ni < 0 || nj < 0 || ni >= tl as i64 || nj >= th as i64 {
                    continue;
                }
                v.push((nj as usize) * tl + ni as usize);
            }
        }
        v
    };
    for passe in 0..(if tiges_precoces { 2 } else { 0 }) {
        let tau_avant = tau.clone();
        for j in 0..th {
            for i in 0..tl {
                let k = j * tl + i;
                // Le coeur, a la seconde passe : ce dont l'epaisseur s'arrondit
                // a 255 sur les octets, c'est-a-dire une couverture de 0,99 au
                // moins, et non 1,0 exactement.
                let concerne = if passe == 0 { couverture[k] > 0.0 } else { couverture[k] >= 0.99 };
                if !concerne {
                    continue;
                }
                let mut plus_tot = tau_avant[k];
                for v in voisins_8(i, j) {
                    let moindre = if passe == 0 { couverture[v] <= 0.0 } else { couverture[v] < 0.99 };
                    if moindre && dedans[v] {
                        plus_tot = plus_tot.min(tau_avant[v]);
                    }
                }
                tau[k] = plus_tot;
            }
        }
    }

    // Normalisation : le dernier cristal du galet arrive a 1. Les texels hors du
    // galet, jamais vus, sont poses a zero.
    let mut duree = 0.0f32;
    for k in 0..tl * th {
        if dedans[k] && tau[k].is_finite() {
            duree = duree.max(tau[k]);
        }
    }
    let duree = duree.max(1.0);
    // La modulation, de moyenne exactement 1 sur le galet.
    let (mut somme_rho, mut compte) = (0.0f64, 0usize);
    for k in 0..tl * th {
        if dedans[k] {
            somme_rho += rho[k] as f64;
            compte += 1;
        }
    }
    let rho_moyen = if compte > 0 { (somme_rho / compte as f64) as f32 } else { 0.5 };
    let mut mu = vec![1.0f32; tl * th];
    let mut somme_mu = 0.0f64;
    for k in 0..tl * th {
        // La clarte de la cellule se relit sur son epaisseur, qui est une
        // fonction affine du meme tirage.
        let u = ((rho[k] - EPAISSEUR_MOSAIQUE.0) / EPAISSEUR_MOSAIQUE.1).clamp(0.0, 1.0);
        let cellule = CELLULE.0 + CELLULE.1 * u;
        let tige = 1.0 + GAIN_TIGE * couverture[k];
        mu[k] = (tige * (cellule * (1.0 - couverture[k]) + couverture[k]) * (1.0 - PERTE_ARETE * arete[k])).max(0.05);
        if dedans[k] {
            somme_mu += mu[k] as f64;
        }
    }
    let _ = rho_moyen;
    let mu_moyen = if compte > 0 { (somme_mu / compte as f64) as f32 } else { 1.0 };

    let mut texels = Vec::with_capacity(tl * th * 4);
    for k in 0..tl * th {
        let t = if dedans[k] && tau[k].is_finite() { (tau[k] / duree).clamp(0.0, 1.0) } else { 0.0 };
        // Sur quatre : une tige vaut jusqu'a trois fois la moyenne, et le
        // nuanceur plafonne a trois (`GIVRE_GAIN_MAXIMUM`).
        let m = (mu[k] / mu_moyen / 4.0).clamp(0.0, 1.0);
        texels.push((t * 255.0).round() as u8);
        texels.push((m * 255.0).round() as u8);
        texels.push((rho[k].clamp(0.0, 1.0) * 255.0).round() as u8);
        texels.push(255);
    }

    Cristaux {
        largeur: tl as u32,
        hauteur: th as u32,
        texels,
        largeur_front: (PAS_CELLULE / 2.0) / (VITESSE_MOSAIQUE * duree),
        duree,
        sites,
        segments: segments.len(),
    }
}

impl Cristaux {
    /// Le temoin de m-g3, lu sur les OCTETS de la texture, comme le nuanceur
    /// les lit : le nombre de texels de tige (B = 255, la couverture pleine)
    /// dont l'arrivee (R) est posterieure a celle d'un texel de mosaique
    /// contigu (B <= 153, c'est-a-dire une epaisseur de cellule, 0,6 au plus,
    /// sans ruban de tige). Doit valoir 0 ; la texture d'avant m-g3 en compte
    /// plus de 0, et `captures/integration-apres-epreuve/pas3/tiges-tardives.py`
    /// refait le compte sur le PNG par un autre chemin.
    pub fn tiges_tardives(&self) -> usize {
        let (l, h) = (self.largeur as usize, self.hauteur as usize);
        let texel = |i: usize, j: usize| {
            let k = (j * l + i) * 4;
            (self.texels[k], self.texels[k + 2])
        };
        let mut compte = 0;
        for j in 0..h {
            for i in 0..l {
                let (tau, rho) = texel(i, j);
                if rho != 255 || tau == 0 {
                    continue;
                }
                let mut tardive = false;
                for dj in -1..=1i64 {
                    for di in -1..=1i64 {
                        let (ni, nj) = (i as i64 + di, j as i64 + dj);
                        if (di == 0 && dj == 0) || ni < 0 || nj < 0 || ni >= l as i64 || nj >= h as i64 {
                            continue;
                        }
                        let (tv, rv) = texel(ni as usize, nj as usize);
                        if rv <= 153 && tv != 0 && tau > tv {
                            tardive = true;
                        }
                    }
                }
                if tardive {
                    compte += 1;
                }
            }
        }
        compte
    }
}

#[derive(PartialEq)]
struct Noeud {
    t: f32,
    case: usize,
}

impl Eq for Noeud {}

impl Ord for Noeud {
    fn cmp(&self, autre: &Self) -> Ordering {
        // Le tas est un tas-max : on inverse pour sortir le plus petit temps.
        autre.t.partial_cmp(&self.t).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for Noeud {
    fn partial_cmp(&self, autre: &Self) -> Option<Ordering> {
        Some(self.cmp(autre))
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    fn galet() -> Cristaux {
        pousser(320.0, 148.0, 48.0, 4.0, GRAINE)
    }

    fn texel(c: &Cristaux, x: f32, y: f32) -> [u8; 4] {
        let i = ((x * SURECHANTILLON as f32) as u32).min(c.largeur - 1) as usize;
        let j = ((y * SURECHANTILLON as f32) as u32).min(c.hauteur - 1) as usize;
        let k = (j * c.largeur as usize + i) * 4;
        [c.texels[k], c.texels[k + 1], c.texels[k + 2], c.texels[k + 3]]
    }

    /// Meme graine, memes octets : c'est ce qui rend les planches refaisables.
    #[test]
    fn la_meme_graine_donne_les_memes_octets() {
        let a = galet();
        let b = galet();
        assert_eq!(a.texels, b.texels);
        let c = pousser(320.0, 148.0, 48.0, 4.0, GRAINE + 1);
        assert_ne!(a.texels, c.texels, "une autre graine doit changer le motif");
    }

    /// Le givre nait au bord et finit au coeur : l'arrivee au bord est petite,
    /// celle du centre est grande. C'est le contraire de la fente centrale que
    /// le juge a vue sur la transformation dessinee.
    #[test]
    fn il_nait_au_bord_et_finit_au_coeur() {
        let c = galet();
        let coeur = texel(&c, 160.0, 74.0)[0];
        let mut bord = 0u32;
        let mut n = 0u32;
        for x in [20.0, 80.0, 160.0, 240.0, 300.0] {
            for y in [1.0, 147.0] {
                bord += texel(&c, x, y)[0] as u32;
                n += 1;
            }
        }
        let bord = bord / n;
        assert!(bord < 90, "le bord arrive trop tard : {bord}");
        assert!(coeur > 140, "le coeur arrive trop tot : {coeur}");
        assert!(coeur as u32 > bord + 60, "coeur {coeur}, bord {bord}");
    }

    /// La modulation est de moyenne 1 : la structure redistribue la lumiere,
    /// elle n'en invente pas. Lue sur les octets, la moyenne de G (`mu / 4`)
    /// doit valoir 63,75 a l'arrondi pres, et la dispersion doit exister (sinon
    /// il n'y a pas de structure).
    #[test]
    fn la_structure_redistribue_sans_inventer() {
        let c = galet();
        let (mut somme, mut n, mut min, mut max) = (0.0f64, 0usize, 255u8, 0u8);
        for j in 0..c.hauteur as usize {
            for i in 0..c.largeur as usize {
                let (x, y) = ((i as f32 + 0.5) / 2.0, (j as f32 + 0.5) / 2.0);
                if crate::galet::distance_au_galet(x, y, 320.0, 148.0, 48.0, 4.0) < 0.0 {
                    let g = c.texels[(j * c.largeur as usize + i) * 4 + 1];
                    somme += g as f64;
                    n += 1;
                    min = min.min(g);
                    max = max.max(g);
                }
            }
        }
        let moyenne = somme / n as f64;
        assert!((moyenne - 63.75).abs() < 1.0, "moyenne de mu/4 : {moyenne}");
        assert!(max as i32 - min as i32 > 40, "pas de structure : {min}..{max}");
    }

    /// m-g3 : aucune tige n'arrive apres la mosaique qu'elle touche. Le
    /// compte doit etre 0 ; la texture d'avant en comptait plus de 0 (le juge
    /// les a vus, `juge-6`), et le script du pas 3 le refait sur le PNG.
    #[test]
    fn aucune_tige_n_arrive_apres_sa_mosaique() {
        let c = galet();
        assert_eq!(c.tiges_tardives(), 0);
    }

    /// L'epreuve du temoin precedent : un motif sans structure (rho constant)
    /// ferait tomber la dispersion. On ne peut pas l'injecter sans changer le
    /// generateur ; on verifie donc la borne dans l'autre sens, sur un texel
    /// de tige : rho = 255 la ou une tige passe, ce qui suppose que la
    /// rasterisation ecrit bien l'epaisseur pleine.
    #[test]
    fn les_tiges_sont_pleines() {
        let c = galet();
        let plein = (0..c.texels.len() / 4).filter(|k| c.texels[k * 4 + 2] == 255).count();
        assert!(plein > 2000, "trop peu de texels de tige pleine : {plein}");
        assert!(c.segments > 5000, "trop peu de segments : {}", c.segments);
        assert!(c.sites >= 4 + 2 * SITES_GRAND_BORD + 2 * SITES_PETIT_BORD);
        assert!(c.largeur_front > 0.005 && c.largeur_front < 0.1, "front : {}", c.largeur_front);
    }
}
