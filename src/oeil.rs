//! L'oeil sur l'ilot REEL : ce que l'auteur verra, pose sur des fonds connus.
//!
//! `--oeil=dossier` ouvre les vraies fenetres et pose l'ilot, contenu montre,
//! sur la rangee des chiffres, que `DESIGN.md` §4 centre verticalement. Il le
//! capture tel que Windows le COMPOSE dans neuf scenes : trois fonds (la nuit de
//! l'horloge, un blanc pur, un aplat colore) croises avec trois formes (replie,
//! mi-course, deploye). Les images vont dans le dossier, les verdicts dans
//! `oeil.txt`, et l'application quitte d'elle-meme.
//!
//! **Pourquoi l'image composee demande une danse.** La capture ne voit l'ilot
//! que s'il n'est pas exclu, et un verre qui capture le bureau pendant qu'il
//! n'est pas exclu refracterait son propre reflet. Chaque scene pose donc le
//! verre ilot exclu, le FIGE (le suivi est suspendu, le fond ne change plus),
//! puis leve l'exclusion le temps d'une image, comme `verre-avant` au pas 2 :
//!
//! 1. `dessous` : ilot exclu, ce qu'il y a sous lui ;
//! 2. `compose` : ilot non exclu, ce que Windows compose et que l'auteur voit ;
//! 3. `dessous-2` : exclusion retablie, le fond encore.
//!
//! **Le fond connu.** La nuit est l'horloge elle-meme. Le blanc et la couleur
//! sont une toile GDI plein ecran (`toile.rs`) dont chaque octet est celui qu'on
//! demande. La couleur est `#0078D4`, l'accent par defaut de Windows, le bleu le
//! plus courant d'un bureau colore.
//!
//! **La mi-course est figee.** On ne peut pas capturer une trame au vol : le
//! verre en mouvement capture le bureau a chaque trame, et l'ilot non exclu s'y
//! verrait. La fenetre peint donc la geometrie que les ressorts auraient en
//! passant par p = 0,5, vitesse et ruban compris, sans decoupe comme pendant le
//! mouvement. Ce qui manque a cette image est dit : le retard d'une trame du
//! fond pendant un vrai mouvement n'y est pas.
//!
//! **Les temoins, chacun a reponse connue, chacun avec son epreuve**
//! (`--epreuve-oeil=noir,exclu,hors,fond`) :
//!
//! - *fond stable et porte par le verre* : `dessous` = `dessous-2` partout,
//!   `compose` = `dessous` hors du rectangle de la fenetre, et le fond que le
//!   verre a televerse = `dessous` sur leur recouvrement. Reponse : 0 pixel.
//!   Sinon la tentative est NON CONCLUANTE et recommencee, trois fois au plus.
//!   Epreuve `fond` : la toile passe a 254 entre `compose` et `dessous-2`, sur
//!   la seule scene « blanc-deploye » ;
//! - *l'image composee montre l'ilot* : a l'interieur du galet, `compose`
//!   differe de `dessous`. Epreuve `exclu` : l'exclusion n'est pas levee ;
//! - *hors du galet, le bureau* : dans la fenetre, la ou la couverture du galet
//!   est nulle, `compose` = `dessous`. Reponse : 0. Epreuve `hors` : la zone
//!   est etendue au ruban, ou la couverture ne l'est pas ;
//! - *verre pose* : un fond etait televerse au moment de `compose`. Epreuve
//!   `noir` : la capture est fermee juste avant, le galet redevient noir ;
//! - *sur un aplat, le coeur du galet lit la valeur calculee* : hors du conge,
//!   le verre d'un aplat ne depend que de la couleur, et `coeur_attendu` la
//!   calcule sur le processeur depuis la formule de `galet.wgsl`, sans passer
//!   par le GPU. Sur le blanc : 50. Tolerance : un arrondi, 1 niveau (la marge
//!   de `DECISION.md`). Epreuve `noir` : le coeur lit 0.

use std::path::PathBuf;
use std::time::Instant;

use winit::event_loop::ActiveEventLoop;

use crate::bureau::{Rect, Region, Tour};
use crate::galet::arbitree;
use crate::ilot::{Ilot, Mesures};
use crate::toile::Toile;

/// Attente entre un changement d'etat de la fenetre et la copie suivante : le
/// compositeur doit avoir presente l'etat nouveau (celle du relevé).
const ATTENTE_ETAT: f64 = 0.4;
/// Delai maximal pour obtenir une image presentee (celui du relevé).
const DELAI_COPIE: f64 = 3.0;
/// Delai maximal pour que le galet prenne sa forme et que le verre recoive le
/// fond de sa nouvelle place.
const DELAI_POSE: f64 = 4.0;
const TENTATIVES: u32 = 3;
/// L'instant « mi-course » : la moitie du deploiement, relue sur la hauteur.
pub const P_MI_COURSE: f32 = 0.5;
pub const BLANC: [u8; 3] = [255, 255, 255];
/// L'accent par defaut de Windows.
pub const COULEUR: [u8; 3] = [0x00, 0x78, 0xD4];
/// La couleur de la toile sous l'epreuve `fond`, entre `compose` et `dessous-2`.
const BLANC_EPREUVE: [u8; 3] = [254, 254, 254];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fond {
    Nuit,
    Uni([u8; 3]),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Forme {
    Replie,
    MiCourse,
    Deploye,
}

struct Scene {
    nom: &'static str,
    fond: Fond,
    forme: Forme,
}

const SCENES: [Scene; 9] = [
    Scene { nom: "nuit-replie", fond: Fond::Nuit, forme: Forme::Replie },
    Scene { nom: "nuit-mi-course", fond: Fond::Nuit, forme: Forme::MiCourse },
    Scene { nom: "nuit-deploye", fond: Fond::Nuit, forme: Forme::Deploye },
    Scene { nom: "blanc-replie", fond: Fond::Uni(BLANC), forme: Forme::Replie },
    Scene { nom: "blanc-mi-course", fond: Fond::Uni(BLANC), forme: Forme::MiCourse },
    Scene { nom: "blanc-deploye", fond: Fond::Uni(BLANC), forme: Forme::Deploye },
    Scene { nom: "couleur-replie", fond: Fond::Uni(COULEUR), forme: Forme::Replie },
    Scene { nom: "couleur-mi-course", fond: Fond::Uni(COULEUR), forme: Forme::MiCourse },
    Scene { nom: "couleur-deploye", fond: Fond::Uni(COULEUR), forme: Forme::Deploye },
];

#[derive(Default)]
struct Epreuves {
    noir: bool,
    exclu: bool,
    hors: bool,
    fond: bool,
}

// --- Le coeur attendu, sur le processeur ---------------------------------------

fn vers_srgb(c: f64) -> f64 {
    if c > 0.0031308 {
        1.055 * c.max(0.0).powf(1.0 / 2.4) - 0.055
    } else {
        c * 12.92
    }
}

fn depuis_srgb(c: f64) -> f64 {
    if c > 0.04045 {
        ((c.max(0.0) + 0.055) / 1.055).powf(2.4)
    } else {
        c / 12.92
    }
}

/// Le coeur du galet pose sur un aplat `rgb`, en octets, calcule depuis la
/// formule de `galet.wgsl` et non par lui.
///
/// Hors du conge, sur un aplat, chaque terme se simplifie : le flou d'un aplat
/// est l'aplat, la refraction ne deplace rien de visible, la clarte vaut
/// (1 - 1)^2 = 0, aucune prise ne reflechit. Il reste la vibrance, la teinte sur
/// les octets et la part transmise a incidence normale, 1 - r0. La cible sRGB
/// borne l'ecriture a [0, 1] et l'arrondit.
pub fn coeur_attendu(rgb: [u8; 3]) -> [u8; 3] {
    const LUMA: [f64; 3] = [0.2126, 0.7152, 0.0722];
    let lin = rgb.map(|c| depuis_srgb(c as f64 / 255.0));
    let gris: f64 = (0..3).map(|k| lin[k] * LUMA[k]).sum();
    let n = arbitree::INDICE as f64;
    let r0 = ((n - 1.0) / (n + 1.0)).powi(2);
    lin.map(|c| {
        let sature = gris + (c - gris) * arbitree::SATURATION as f64;
        let verre = depuis_srgb(vers_srgb(sature) * (1.0 - arbitree::TEINTE as f64)) * (1.0 - r0);
        let sortie = depuis_srgb(vers_srgb(verre));
        (vers_srgb(sortie.clamp(0.0, 1.0)) * 255.0).round() as u8
    })
}

// --- Les lectures d'image --------------------------------------------------------

fn pixel(r: &Region, x: i32, y: i32) -> Option<[u8; 3]> {
    let (i, j) = (x - r.x, y - r.y);
    if i < 0 || j < 0 || i >= r.largeur as i32 || j >= r.hauteur as i32 {
        return None;
    }
    let k = ((j as u32 * r.largeur + i as u32) * 4) as usize;
    Some([r.pixels[k], r.pixels[k + 1], r.pixels[k + 2]])
}

/// Pixels differents entre `a` et `b` sur leur recouvrement, en ne gardant que
/// ceux que `garder` accepte. Rend (differents, lus).
fn ecart(a: &Region, b: &Region, garder: impl Fn(i32, i32) -> bool) -> (u64, u64) {
    let (mut n, mut tot) = (0, 0);
    for j in 0..a.hauteur as i32 {
        for i in 0..a.largeur as i32 {
            let (x, y) = (a.x + i, a.y + j);
            if !garder(x, y) {
                continue;
            }
            let (Some(p), Some(q)) = (pixel(a, x, y), pixel(b, x, y)) else {
                continue;
            };
            tot += 1;
            if p != q {
                n += 1;
            }
        }
    }
    (n, tot)
}

/// Ce que l'oeil sait de la fenetre et du galet au moment de `compose`.
#[derive(Clone, Copy)]
struct Geometrie {
    fenetre: Rect,
    m: Mesures,
    ruban: f32,
}

impl Geometrie {
    /// La distance signee au galet d'un point du bureau, en pixels.
    fn distance(&self, x: i32, y: i32) -> f32 {
        let (gx, gy) = (self.fenetre.x + self.m.x as i32, self.fenetre.y + self.m.y as i32);
        crate::galet::distance_au_galet(
            (x - gx) as f32 + 0.5,
            (y - gy) as f32 + 0.5,
            self.m.l,
            self.m.h,
            self.m.rayon,
            self.m.n,
        )
    }

    fn dans_fenetre(&self, x: i32, y: i32) -> bool {
        let f = self.fenetre;
        x >= f.x && x < f.x + f.l && y >= f.y && y < f.y + f.h
    }

    /// Le conge `b` du galet courant, en pixels physiques : `12 x h / 148`.
    fn conge(&self) -> f32 {
        arbitree::CONGE * self.m.h / arbitree::HAUTEUR_DU_CONGE
    }
}

pub struct Oeil {
    dossier: PathBuf,
    ep: Epreuves,
    adaptateur: String,
    echelle: f64,
    toile: Option<Toile>,
    rang: usize,
    pas: u32,
    debut_pas: Instant,
    debut: Instant,
    tentative: u32,
    copie_exigee: Option<u64>,
    texte: String,
    verdicts: Vec<(String, bool)>,
    focus_perdus: u32,
    fond_verre: Option<Region>,
    dessous: Option<Region>,
    compose: Option<Region>,
    dessous_2: Option<Region>,
    pose_au_compose: bool,
    geometrie: Option<Geometrie>,
}

fn copier(r: &Region) -> Region {
    Region {
        pixels: r.pixels.clone(),
        x: r.x,
        y: r.y,
        largeur: r.largeur,
        hauteur: r.hauteur,
    }
}

impl Oeil {
    pub fn nouveau(dossier: PathBuf, epreuves: &str, adaptateur: String, echelle: f64) -> Oeil {
        let mut ep = Epreuves::default();
        for mot in epreuves.split(',').map(str::trim).filter(|m| !m.is_empty()) {
            match mot {
                "noir" => ep.noir = true,
                "exclu" => ep.exclu = true,
                "hors" => ep.hors = true,
                "fond" => ep.fond = true,
                autre => eprintln!("epreuve de l'oeil inconnue, ignoree : {autre}"),
            }
        }
        let maintenant = Instant::now();
        Oeil {
            dossier,
            ep,
            adaptateur,
            echelle,
            toile: None,
            rang: 0,
            pas: 0,
            debut_pas: maintenant,
            debut: maintenant,
            tentative: 1,
            copie_exigee: None,
            texte: String::new(),
            verdicts: Vec::new(),
            focus_perdus: 0,
            fond_verre: None,
            dessous: None,
            compose: None,
            dessous_2: None,
            pose_au_compose: false,
            geometrie: None,
        }
    }

    pub fn focus_perdu(&mut self) {
        self.focus_perdus += 1;
    }

    fn dire(&mut self, ligne: impl AsRef<str>) {
        println!("{}", ligne.as_ref());
        self.texte.push_str(ligne.as_ref());
        self.texte.push('\n');
    }

    fn verdict(&mut self, nom: String, vrai: bool, detail: String) {
        self.dire(format!(
            "  VERDICT {nom} : {}  ({detail})",
            if vrai { "VRAI" } else { "FAUX" }
        ));
        self.verdicts.push((nom, vrai));
    }

    fn passer(&mut self, pas: u32) {
        self.pas = pas;
        self.debut_pas = Instant::now();
    }

    /// Une image de la region deployee, la capture de l'ilot suspendue. Rend
    /// `Some` quand c'est fini, avec l'image ou sans elle.
    fn prendre(ilot: &mut Ilot, t: f64) -> Option<Option<Region>> {
        // L'ilot se repeint : c'est ce qui fait presenter une image au bureau
        // quand rien d'autre n'y bouge, toile immobile comprise.
        ilot.fenetre.request_redraw();
        let Some(r) = ilot.region_deployee() else {
            return Some(None);
        };
        if ilot.suivi.tour(r) == Tour::Neuve {
            return Some(ilot.suivi.region.take());
        }
        if t > DELAI_COPIE {
            return Some(None);
        }
        None
    }

    /// Un pas du script. Rend vrai quand l'oeil a fini.
    pub fn pas(&mut self, ilot: &mut Ilot, el: &ActiveEventLoop) -> bool {
        let t = self.debut_pas.elapsed().as_secs_f64();
        let scene = &SCENES[self.rang.min(SCENES.len() - 1)];
        match self.pas {
            // --- installation : la toile, et l'ilot sur la rangee des chiffres
            0 => {
                let _ = ilot.suivre_le_bureau();
                if t < 1.0 {
                    return false;
                }
                self.dire("=== l'oeil sur l'ilot reel ===");
                self.dire(format!("adaptateur wgpu : {}", self.adaptateur));
                self.dire(format!("echelle de la fenetre de l'ilot : {}", self.echelle));
                self.dire(format!(
                    "alpha par pixel : {} ; epreuves : noir {} exclu {} hors {} fond {}",
                    ilot.alpha_par_pixel(),
                    self.ep.noir,
                    self.ep.exclu,
                    self.ep.hors,
                    self.ep.fond
                ));
                let affinite = ilot.affinite;
                self.verdict(
                    "exclusion posee".into(),
                    affinite == Some(17),
                    format!("affinite relue {affinite:?}"),
                );
                if let Some(ecran) = ilot.fenetre.current_monitor() {
                    let (o, s) = (ecran.position(), ecran.size());
                    self.toile = Toile::nouvelle(o.x, o.y, s.width as i32, s.height as i32);
                    let taille = ilot.fenetre.outer_size();
                    ilot.fenetre.set_outer_position(winit::dpi::PhysicalPosition::new(
                        o.x + (s.width as i32 - taille.width as i32) / 2,
                        o.y + (s.height as i32 - taille.height as i32) / 2,
                    ));
                    self.dire(format!(
                        "ecran ({}, {}) {}x{} ; toile {}",
                        o.x,
                        o.y,
                        s.width,
                        s.height,
                        if self.toile.is_some() { "creee" } else { "REFUSEE" }
                    ));
                }
                self.passer(1);
            }
            // --- la scene : son fond, sa forme --------------------------------
            1 => {
                self.dire(format!("--- {} (tentative {}) ---", scene.nom, self.tentative));
                match scene.fond {
                    Fond::Nuit => {
                        if let Some(toile) = self.toile.as_mut() {
                            toile.masquer();
                        }
                    }
                    Fond::Uni(c) => {
                        if let Some(toile) = self.toile.as_mut() {
                            toile.montrer(c);
                        }
                    }
                }
                match scene.forme {
                    Forme::Replie => {
                        ilot.p_fige = None;
                        ilot.viser(false);
                    }
                    Forme::MiCourse => {
                        ilot.p_fige = Some(P_MI_COURSE);
                    }
                    Forme::Deploye => {
                        ilot.p_fige = None;
                        ilot.viser(true);
                    }
                }
                ilot.suivi_suspendu = false;
                ilot.affinite = ilot.exclure_de_la_capture(true);
                ilot.montrer_contenu(true);
                self.copie_exigee = None;
                self.passer(2);
            }
            // --- la pose : forme atteinte, verre sur le fond de sa place ------
            2 => {
                ilot.fenetre.request_redraw();
                let _ = ilot.suivre_le_bureau();
                // La toile vient d'etre peinte : une copie est exigee apres
                // qu'elle a eu le temps d'etre presentee, et le verre n'est pret
                // qu'une fois une image neuve servie depuis.
                if t >= ATTENTE_ETAT && self.copie_exigee.is_none() {
                    ilot.suivi.exiger_copie();
                    self.copie_exigee = Some(ilot.suivi.compteurs.neuves);
                }
                let forme = match scene.forme {
                    Forme::Replie => {
                        !ilot.deploye() && !ilot.en_mouvement() && ilot.boite_de_decoupe().is_some()
                    }
                    Forme::Deploye => {
                        ilot.deploye() && !ilot.en_mouvement() && ilot.boite_de_decoupe().is_some()
                    }
                    Forme::MiCourse => (ilot.mesures_galet().p - P_MI_COURSE).abs() < 0.01,
                };
                let a_sa_place = match (ilot.fond_capture(), ilot.region_suivie()) {
                    (Some(f), Some(r)) => {
                        (f.x, f.y, f.largeur as i32, f.hauteur as i32) == (r.x, r.y, r.l, r.h)
                    }
                    _ => false,
                };
                let servie = self
                    .copie_exigee
                    .is_some_and(|n| ilot.suivi.compteurs.neuves > n);
                let pret = forme && a_sa_place && servie && ilot.verre_pose();
                if (pret && t >= 2.0 * ATTENTE_ETAT) || t >= DELAI_POSE {
                    if !pret {
                        self.dire(format!(
                            "  pas pret en {DELAI_POSE} s : forme {forme}, fond a sa place {a_sa_place}, image servie {servie}, verre pose {}",
                            ilot.verre_pose()
                        ));
                    }
                    ilot.suivi_suspendu = true;
                    self.fond_verre = ilot.fond_capture().map(copier);
                    if self.ep.noir {
                        // La capture perdue, comme `--epreuve-repli` au rebours :
                        // le galet redevient noir, puis la duplication peut se
                        // rouvrir pour les images de l'oeil, le suivi restant
                        // suspendu.
                        ilot.suivi.empecher_ouverture(true);
                        let _ = ilot.suivre_le_bureau();
                        ilot.suivi.empecher_ouverture(false);
                    }
                    ilot.suivi.exiger_copie();
                    self.dessous = None;
                    self.compose = None;
                    self.dessous_2 = None;
                    self.passer(3);
                }
            }
            3 => {
                if self.ep.noir && t < ATTENTE_ETAT {
                    ilot.fenetre.request_redraw();
                    return false;
                }
                if let Some(image) = Self::prendre(ilot, t) {
                    self.dessous = image;
                    if !self.ep.exclu {
                        ilot.affinite = ilot.exclure_de_la_capture(false);
                    }
                    self.passer(4);
                }
            }
            4 => {
                ilot.fenetre.request_redraw();
                if t >= ATTENTE_ETAT {
                    ilot.suivi.exiger_copie();
                    self.pose_au_compose = ilot.verre_pose();
                    self.geometrie = ilot.fenetre.inner_position().ok().map(|p| {
                        let s = ilot.fenetre.inner_size();
                        Geometrie {
                            fenetre: Rect { x: p.x, y: p.y, l: s.width as i32, h: s.height as i32 },
                            m: ilot.mesures_galet(),
                            ruban: ilot.ruban_galet(),
                        }
                    });
                    self.passer(5);
                }
            }
            5 => {
                if let Some(image) = Self::prendre(ilot, t) {
                    self.compose = image;
                    ilot.affinite = ilot.exclure_de_la_capture(true);
                    if self.ep.fond && scene.nom == "blanc-deploye" {
                        if let Some(toile) = self.toile.as_mut() {
                            toile.montrer(BLANC_EPREUVE);
                        }
                    }
                    self.passer(6);
                }
            }
            6 => {
                ilot.fenetre.request_redraw();
                if t >= ATTENTE_ETAT {
                    ilot.suivi.exiger_copie();
                    self.passer(7);
                }
            }
            7 => {
                if let Some(image) = Self::prendre(ilot, t) {
                    self.dessous_2 = image;
                    ilot.suivi_suspendu = false;
                    self.conclure_scene();
                }
            }
            _ => {
                if let Some(toile) = self.toile.as_mut() {
                    toile.masquer();
                }
                self.toile = None;
                ilot.p_fige = None;
                self.ecrire_fin();
                el.exit();
                return true;
            }
        }
        false
    }

    fn conclure_scene(&mut self) {
        let scene = &SCENES[self.rang];
        let nom = scene.nom;
        // Les images sortent de l'oeil le temps du jugement, qui ecrit dans
        // l'oeil, et y reviennent pour etre ecrites sur le disque.
        let (dessous, compose, dessous_2, fond_verre) = (
            self.dessous.take(),
            self.compose.take(),
            self.dessous_2.take(),
            self.fond_verre.take(),
        );
        let (Some(d), Some(c), Some(d2), Some(g)) = (
            dessous.as_ref(),
            compose.as_ref(),
            dessous_2.as_ref(),
            self.geometrie,
        ) else {
            self.dire("  une image manque : tentative NON CONCLUANTE");
            return self.recommencer_ou_juger(false);
        };

        // --- le fond stable, et porte par le verre ---------------------------
        let meme_geo = (d.x, d.y, d.largeur, d.hauteur) == (d2.x, d2.y, d2.largeur, d2.hauteur)
            && (d.x, d.y, d.largeur, d.hauteur) == (c.x, c.y, c.largeur, c.hauteur);
        let (s1, t1) = ecart(d, d2, |_, _| true);
        let (s2, t2) = ecart(c, d, |x, y| !g.dans_fenetre(x, y));
        let (s3, t3) = match fond_verre.as_ref() {
            Some(f) => ecart(f, d, |_, _| true),
            None => (1, 0),
        };
        let stable = meme_geo && s1 == 0 && s2 == 0 && s3 == 0 && t1 > 0 && t2 > 0 && t3 > 0;
        self.dire(format!(
            "  dessous contre dessous-2 : {s1} sur {t1} ; compose contre dessous hors de la fenetre : {s2} sur {t2} ; fond du verre contre dessous : {s3} sur {t3}"
        ));

        // --- l'ilot dans l'image, et rien hors du galet ------------------------
        let (b_n, b_t) = ecart(c, d, |x, y| g.distance(x, y) <= -0.5);
        let seuil_hors = if self.ep.hors { -0.5 } else { 0.5 * g.ruban.max(1.0) };
        let (h_n, h_t) = ecart(c, d, |x, y| g.dans_fenetre(x, y) && g.distance(x, y) >= seuil_hors);

        // --- le coeur sur un aplat ---------------------------------------------
        let mut coeur = None;
        if let Fond::Uni(couleur) = scene.fond {
            let attendu = coeur_attendu(couleur);
            let limite = -(g.conge() + 1.5);
            let mut compte: std::collections::HashMap<[u8; 3], u64> = Default::default();
            let mut proches = 0u64;
            let mut lus = 0u64;
            for j in 0..c.hauteur as i32 {
                for i in 0..c.largeur as i32 {
                    let (x, y) = (c.x + i, c.y + j);
                    if g.distance(x, y) > limite {
                        continue;
                    }
                    let Some(p) = pixel(c, x, y) else { continue };
                    lus += 1;
                    *compte.entry(p).or_default() += 1;
                    if (0..3).all(|k| p[k].abs_diff(attendu[k]) <= 1) {
                        proches += 1;
                    }
                }
            }
            let mode = compte.into_iter().max_by_key(|(_, n)| *n).map(|(p, _)| p);
            coeur = Some((attendu, mode, proches, lus, limite));
        }

        if !stable && self.tentative < TENTATIVES {
            self.dire(format!("  tentative {} NON CONCLUANTE : le fond a change", self.tentative));
            return self.recommencer_ou_juger(false);
        }
        let note = if stable { "" } else { " [tentatives epuisees]" };
        self.verdict(
            format!("{nom} : fond stable et porte par le verre"),
            stable,
            format!("{s1} + {s2} + {s3} pixels changes, tentative {}{note}", self.tentative),
        );
        self.verdict(
            format!("{nom} : l'image composee montre l'ilot"),
            b_n > 0,
            format!("{b_n} pixels differents du dessous sur {b_t} a couverture pleine"),
        );
        self.verdict(
            format!("{nom} : hors du galet, le bureau"),
            h_t > 0 && h_n == 0,
            format!("{h_n} pixels differents sur {h_t} dans la fenetre, distance >= {seuil_hors:.2}"),
        );
        let pose = self.pose_au_compose;
        self.verdict(
            format!("{nom} : verre pose"),
            pose,
            format!("fond televerse au moment de l'image : {pose}"),
        );
        if let Some((attendu, mode, proches, lus, limite)) = coeur {
            let juste = mode.is_some_and(|m| (0..3).all(|k| m[k].abs_diff(attendu[k]) <= 1));
            self.verdict(
                format!("{nom} : le coeur lit la valeur calculee"),
                juste,
                format!(
                    "attendu {attendu:?}, le plus frequent {mode:?} ; {proches} pixels a 1 niveau pres sur {lus} du coeur (distance <= {limite:.2})"
                ),
            );
        }
        let fond = match scene.fond {
            Fond::Nuit => "nuit".to_string(),
            Fond::Uni(c) => format!("{},{},{}", c[0], c[1], c[2]),
        };
        let f = fond_verre
            .as_ref()
            .map(|f| format!("{},{},{},{}", f.x, f.y, f.largeur, f.hauteur))
            .unwrap_or_else(|| "aucun".into());
        let ligne = format!(
            "OEIL scene={nom} fond={fond} echelle={} fenetre={},{},{},{} galet={},{},{},{} rayon={} n={} ruban={} p={} region={},{},{},{} fond_verre={f} pose={pose}",
            self.echelle,
            g.fenetre.x,
            g.fenetre.y,
            g.fenetre.l,
            g.fenetre.h,
            g.m.x,
            g.m.y,
            g.m.l,
            g.m.h,
            g.m.rayon,
            g.m.n,
            g.ruban,
            g.m.p,
            c.x,
            c.y,
            c.largeur,
            c.hauteur,
        );
        self.dire(ligne);
        self.dessous = dessous;
        self.compose = compose;
        self.dessous_2 = dessous_2;
        self.fond_verre = fond_verre;
        self.ecrire_images(nom);
        self.recommencer_ou_juger(true);
    }

    /// Passe a la scene suivante si `jugee`, sinon recommence celle-ci.
    fn recommencer_ou_juger(&mut self, jugee: bool) {
        if !jugee && self.tentative < TENTATIVES {
            self.tentative += 1;
            self.passer(1);
            return;
        }
        if !jugee {
            let nom = SCENES[self.rang].nom;
            self.verdict(
                format!("{nom} : fond stable et porte par le verre"),
                false,
                format!("images absentes apres {TENTATIVES} tentatives"),
            );
        }
        self.tentative = 1;
        self.rang += 1;
        self.passer(if self.rang < SCENES.len() { 1 } else { 99 });
    }

    fn ecrire_images(&mut self, nom: &str) {
        let _ = std::fs::create_dir_all(&self.dossier);
        let images = [
            ("dessous", self.dessous.as_ref()),
            ("compose", self.compose.as_ref()),
            ("dessous-2", self.dessous_2.as_ref()),
            ("fond-verre", self.fond_verre.as_ref()),
        ];
        let mut plaintes = Vec::new();
        for (quoi, image) in images {
            if let Some(r) = image {
                let chemin = self.dossier.join(format!("{nom}-{quoi}.png"));
                if let Err(e) = crate::releve::ecrire_png(&chemin, r) {
                    plaintes.push(format!("!! {} : {e}", chemin.display()));
                }
            }
        }
        for p in plaintes {
            self.dire(p);
        }
    }

    fn ecrire_fin(&mut self) {
        self.dire(format!(
            "--- fin ({:.2} s) ; pertes de focus de l'ilot (ignorees) : {}",
            self.debut.elapsed().as_secs_f64(),
            self.focus_perdus
        ));
        let justes = self.verdicts.iter().filter(|v| v.1).count();
        let total = self.verdicts.len();
        self.dire(format!("verdicts : {justes} sur {total}"));
        let _ = std::fs::create_dir_all(&self.dossier);
        let chemin = self.dossier.join("oeil.txt");
        match std::fs::write(&chemin, &self.texte) {
            Ok(()) => println!("-> {}", chemin.display()),
            Err(e) => println!("!! {} : {e}", chemin.display()),
        }
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    /// Le coeur sur le blanc, refait a la main : vers_srgb(1) = 1, fois 0,2,
    /// decode 0,03310, fois 1 - r0 = 0,95787 (N-BK7), soit 0,03171, encode
    /// 0,19550, soit 49,85 octets : 50.
    #[test]
    fn le_coeur_du_blanc_vaut_cinquante() {
        assert_eq!(coeur_attendu(BLANC), [50, 50, 50]);
    }

    /// Sur le noir, le verre est noir (temoin 8.14 du banc).
    #[test]
    fn le_coeur_du_noir_est_noir() {
        assert_eq!(coeur_attendu([0, 0, 0]), [0, 0, 0]);
    }
}
