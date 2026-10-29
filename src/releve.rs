//! Le relevé de la capture du bureau, dans l'application REELLE.
//!
//! `--releve-bureau=dossier` ouvre les vraies fenetres (horloge plein ecran,
//! ilot au-dessus), deroule un script borne, ecrit ce qu'il a vu et quitte. Le
//! laboratoire avait tout mesure sur ses propres fenetres GDI ; ce script refait
//! les memes epreuves sur la fenetre de l'ilot telle qu'elle est : surface wgpu,
//! transparence par pixel, region de decoupe, ressorts.
//!
//! Chaque verdict a son temoin, un cas dont la reponse est connue d'avance :
//!
//! - **DPI** : la texture dupliquee est toujours en pixels physiques, les
//!   coordonnees de la sortie ne le sont que si la conscience du DPI a ete
//!   declaree a temps. Les deux tailles doivent etre egales. `--epreuve-dpi`
//!   declare le processus inconscient : elles doivent alors differer.
//! - **Exclusion** : six images de la MEME region, galet deploye au repos :
//!   cache, exclu, cache, NON exclu, exclusion retablie, cache. L'exclu et le
//!   retabli doivent etre identiques au cache ; le non-exclu doit en differer,
//!   sans quoi le zero ne prouverait rien (`VERRE-TAILLE.md` §1).
//! - **Le fond, lui, n'est pas a nous.** Cette machine est partagee : une autre
//!   fenetre peut passer devant l'horloge ou prendre le focus pendant l'essai.
//!   Les trois images cachees, intercalees, doivent donc etre identiques entre
//!   elles. Si elles ne le sont pas, la tentative est NON CONCLUANTE — ni vraie
//!   ni fausse — et elle est recommencee, trois fois au plus.
//! - **Les tailles** : au repos, la region suivie doit egaler la boite de la
//!   decoupe GDI relue a Windows, elargie de la marge. C'est un chemin
//!   independant des mesures dont la region est tiree.
//!
//! Le script est pilote par un fil qui envoie un `Tic` toutes les 16 ms, comme
//! la boucle a 60 Hz du banc de regime. Il ne touche ni au reveil de l'horloge
//! ni a son `ControlFlow`.

use std::path::PathBuf;
use std::time::Instant;

use winit::event_loop::ActiveEventLoop;

use crate::bureau::{Compteurs, Conscience, Rect, Region, Tour};
use crate::ilot::{CompteursVerre, Ilot, Mesures};

/// Delai maximal pour obtenir une image presentee apres une demande de copie.
/// L'horloge presente au moins une fois par seconde ; trois secondes laissent
/// passer trois de ses bascules.
const DELAI_COPIE: f64 = 3.0;

/// Delai maximal pour qu'un ressort atteigne la geometrie demandee. La course
/// complete demande 521 ms (`ilot.rs`, `poser_region`).
const DELAI_FORME: f64 = 3.0;

/// Attente entre un changement d'etat de la fenetre (visibilite, affinite) et
/// la demande de copie : le compositeur doit avoir presente l'etat nouveau.
const ATTENTE_ETAT: f64 = 0.4;

const TENTATIVES: u32 = 3;

/// La sequence du temoin : nom, fenetre visible, fenetre exclue.
const SEQUENCE: [(&str, bool, bool); 6] = [
    ("cache-1", false, true),
    ("exclu", true, true),
    ("cache-2", false, true),
    ("non-exclu", true, false),
    ("exclu-retabli", true, true),
    ("cache-3", false, true),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Etape {
    Installation,
    ReposReplie,
    AvantDeploiement,
    Deploiement,
    ReposDeploye,
    Temoin,
    Repeint,
    Deplacement,
    AvantRepli,
    Verre,
    SondeSeule,
    Repli,
    Reouverture,
    Fin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sous {
    Appliquer,
    Attendre,
    Copier,
}

/// Ce qu'on sait d'une phase au moment ou elle s'ouvre.
struct Phase {
    nom: &'static str,
    debut: Instant,
    compteurs: Compteurs,
    durees: usize,
    verre: CompteursVerre,
}

pub struct Releve {
    dossier: PathBuf,
    epreuve_dpi: bool,
    conscience_declaree: Conscience,
    adaptateur_wgpu: String,
    echelle: f64,
    etape: Etape,
    debut_etape: Instant,
    debut: Instant,
    texte: String,
    phase: Option<Phase>,
    // --- le temoin d'exclusion ---
    temoin: Option<Rect>,
    images: Vec<(&'static str, Region)>,
    tentative: u32,
    rang: usize,
    sous: Sous,
    tentative_valide: bool,
    raison: Option<String>,
    // --- le reste ---
    origine: Option<winit::dpi::PhysicalPosition<i32>>,
    decalage: i32,
    attente_suivie: u32,
    attente_max: u32,
    tailles: Vec<(i32, i32)>,
    ouvertures_avant: u64,
    focus_perdus: u32,
    verdicts: Vec<(&'static str, bool, String)>,
    // --- le verre en direct ---
    verre_fait: bool,
    verre_rang: u32,
    images_verre: Vec<(&'static str, Region)>,
    fond_verre: Option<Region>,
    fenetre_verre: Option<(i32, i32)>,
    /// Le verre etait-il pose : avant la perte, pendant, apres le retour.
    poses_verre: [bool; 3],
    /// La place de l'ilot avant la sequence du verre, rendue a la fin.
    origine_verre: Option<winit::dpi::PhysicalPosition<i32>>,
    // --- la sonde seule ---
    sonde_seule: bool,
    sonde_rang: u32,
}

/// Le galet est-il deploye, au repos, a sa geometrie finale ?
fn au_repos_deploye(ilot: &Ilot) -> bool {
    !ilot.en_mouvement()
        && ilot.region_suivie().is_some()
        && ilot.region_suivie() == ilot.region_deployee()
}

impl Releve {
    pub fn nouveau(
        dossier: PathBuf,
        epreuve_dpi: bool,
        conscience_declaree: Conscience,
        adaptateur_wgpu: String,
        echelle: f64,
    ) -> Releve {
        let maintenant = Instant::now();
        Releve {
            dossier,
            epreuve_dpi,
            conscience_declaree,
            adaptateur_wgpu,
            echelle,
            etape: Etape::Installation,
            debut_etape: maintenant,
            debut: maintenant,
            texte: String::new(),
            phase: None,
            temoin: None,
            images: Vec::new(),
            tentative: 0,
            rang: 0,
            sous: Sous::Appliquer,
            tentative_valide: true,
            raison: None,
            origine: None,
            decalage: 0,
            attente_suivie: 0,
            attente_max: 0,
            tailles: Vec::new(),
            ouvertures_avant: 0,
            focus_perdus: 0,
            verdicts: Vec::new(),
            verre_fait: false,
            verre_rang: 0,
            images_verre: Vec::new(),
            fond_verre: None,
            fenetre_verre: None,
            poses_verre: [false; 3],
            origine_verre: None,
            sonde_seule: false,
            sonde_rang: 0,
        }
    }

    /// L'ilot a perdu le focus. Pendant le relevé il ne se replie pas : le focus
    /// appartient a l'environnement (une autre fenetre de la machine), et le
    /// suivre changerait la geometrie sous le temoin. On le compte.
    pub fn focus_perdu(&mut self) {
        self.focus_perdus += 1;
    }

    /// La sonde du bureau a-t-elle la main ? Seulement pendant la phase
    /// « sonde seule », ou le relevé ne fait lui-meme aucun tour.
    pub fn laisse_sonder(&self) -> bool {
        self.sonde_seule
    }

    fn dire(&mut self, ligne: impl AsRef<str>) {
        println!("{}", ligne.as_ref());
        self.texte.push_str(ligne.as_ref());
        self.texte.push('\n');
    }

    fn verdict(&mut self, nom: &'static str, vrai: bool, detail: String) {
        self.dire(format!(
            "  VERDICT {nom} : {}  ({detail})",
            if vrai { "VRAI" } else { "FAUX" }
        ));
        self.verdicts.push((nom, vrai, detail));
    }

    fn passer(&mut self, etape: Etape) {
        self.etape = etape;
        self.debut_etape = Instant::now();
    }

    fn remettre_l_horloge(&mut self) {
        self.debut_etape = Instant::now();
    }

    fn ouvrir_phase(&mut self, ilot: &mut Ilot, nom: &'static str) {
        ilot.suivi.compteurs.plus_grand_sale = None;
        self.phase = Some(Phase {
            nom,
            debut: Instant::now(),
            compteurs: ilot.suivi.compteurs.clone(),
            durees: ilot.suivi.durees.as_ref().map_or(0, Vec::len),
            verre: ilot.verre_compteurs.clone(),
        });
    }

    fn fermer_phase(&mut self, ilot: &mut Ilot) {
        let Some(p) = self.phase.take() else { return };
        let c = &ilot.suivi.compteurs;
        let d = |a: u64, b: u64| a - b;
        let v = &ilot.verre_compteurs;
        let mut neuves: Vec<f64> = Vec::new();
        let mut vides: Vec<f64> = Vec::new();
        if let Some(durees) = ilot.suivi.durees.as_ref() {
            for (tour, ms) in &durees[p.durees..] {
                match tour {
                    Tour::Neuve => neuves.push(*ms),
                    Tour::Inchange => vides.push(*ms),
                    _ => {}
                }
            }
        }
        let lignes = [
            format!(
                "--- {} ({:.2} s) ---",
                p.nom,
                p.debut.elapsed().as_secs_f64()
            ),
            format!(
                "  tours {}  neuves {} (dont forcees {})  inchanges {}  attentes {}  perdus {}  fermes {}  ouvertures {}",
                d(c.tours, p.compteurs.tours),
                d(c.neuves, p.compteurs.neuves),
                d(c.forcees, p.compteurs.forcees),
                d(c.inchanges, p.compteurs.inchanges),
                d(c.attentes, p.compteurs.attentes),
                d(c.perdus, p.compteurs.perdus),
                d(c.fermes, p.compteurs.fermes),
                d(c.ouvertures, p.compteurs.ouvertures),
            ),
            format!(
                "  neuves non forcees, par cause : sans metadonnees {}  deplacement {}  rectangle sale {} (le plus grand : {:?})",
                d(c.par_sans_metadonnees, p.compteurs.par_sans_metadonnees),
                d(c.par_deplacement, p.compteurs.par_deplacement),
                d(c.par_sale, p.compteurs.par_sale),
                c.plus_grand_sale,
            ),
            format!(
                "  tour avec copie : median {}  p95 {}   tour a vide : median {}  p95 {}",
                quantile(&mut neuves, 0.5),
                quantile(&mut neuves, 0.95),
                quantile(&mut vides, 0.5),
                quantile(&mut vides, 0.95),
            ),
            format!(
                "  verre : trames de l'ilot {}  copies identiques au fond {}  differentes {}  televersements {}  retours au noir {}  tours de sonde {}",
                d(v.trames, p.verre.trames),
                d(v.identiques, p.verre.identiques),
                d(v.differentes, p.verre.differentes),
                d(v.televersements, p.verre.televersements),
                d(v.oublis, p.verre.oublis),
                d(v.sondes, p.verre.sondes),
            ),
        ];
        for l in lignes {
            self.dire(l);
        }
    }

    /// Un pas du script. Rend vrai quand le relevé est termine.
    pub fn pas(&mut self, ilot: &mut Ilot, el: &ActiveEventLoop) -> bool {
        if ilot.suivi.durees.is_none() {
            ilot.suivi.durees = Some(Vec::new());
        }
        let t = self.debut_etape.elapsed().as_secs_f64();
        match self.etape {
            Etape::Installation => {
                let _ = ilot.suivre_le_bureau();
                if t >= 1.0 {
                    self.ecrire_installation(ilot);
                    if self.epreuve_dpi {
                        self.passer(Etape::Fin);
                    } else {
                        self.ouvrir_phase(ilot, "repos replie, horloge dessous");
                        self.passer(Etape::ReposReplie);
                    }
                }
            }
            Etape::ReposReplie => {
                let _ = ilot.suivre_le_bureau();
                if t >= 3.0 {
                    self.fermer_phase(ilot);
                    self.passer(Etape::AvantDeploiement);
                }
            }
            Etape::AvantDeploiement => {
                // Le deploiement ne se mesure que depuis le galet replie au
                // repos : un clic venu d'ailleurs a pu le deployer.
                let _ = ilot.suivre_le_bureau();
                if ilot.deploye() {
                    ilot.viser(false);
                }
                if (!ilot.deploye() && !ilot.en_mouvement() && t >= 0.05) || t >= DELAI_FORME {
                    ilot.viser(true);
                    self.tailles.clear();
                    self.ouvrir_phase(ilot, "deploiement (la region change a chaque trame)");
                    self.passer(Etape::Deploiement);
                }
            }
            Etape::Deploiement => {
                let _ = ilot.suivre_le_bureau();
                if let Some(r) = ilot.region_suivie() {
                    self.tailles.push((r.l, r.h));
                }
                // Au repos ET decoupe posee : la region GDI n'est posee qu'a
                // l'arret du ressort, et c'est elle qui sert de reference.
                if (t >= 0.3 && !ilot.en_mouvement() && ilot.boite_de_decoupe().is_some())
                    || t >= DELAI_FORME
                {
                    self.fermer_phase(ilot);
                    self.verifier_tailles(ilot, true);
                    self.ouvrir_phase(ilot, "repos deploye");
                    self.passer(Etape::ReposDeploye);
                }
            }
            Etape::ReposDeploye => {
                let _ = ilot.suivre_le_bureau();
                if t >= 2.0 {
                    self.verdict_de_la_boucle(ilot);
                    self.fermer_phase(ilot);
                    self.dire("--- temoin d'exclusion ---");
                    self.tentative = 1;
                    self.rang = 0;
                    self.sous = Sous::Appliquer;
                    self.passer(Etape::Temoin);
                }
            }
            Etape::Temoin => self.pas_du_temoin(ilot, t),
            Etape::Verre => self.pas_du_verre(ilot, t),
            Etape::SondeSeule => self.pas_de_la_sonde(ilot, t),
            Etape::Repeint => {
                ilot.fenetre.request_redraw();
                let _ = ilot.suivre_le_bureau();
                if t >= 2.0 {
                    self.fermer_phase(ilot);
                    self.origine = ilot.fenetre.outer_position().ok();
                    self.ouvrir_phase(ilot, "deplacement de 2 px par tic, bureau calme");
                    self.passer(Etape::Deplacement);
                }
            }
            Etape::Deplacement => {
                if let Some(o) = self.origine {
                    self.decalage += if t < 1.0 { 2 } else { -2 };
                    self.decalage = self.decalage.max(0);
                    ilot.fenetre.set_outer_position(winit::dpi::PhysicalPosition::new(
                        o.x + self.decalage,
                        o.y,
                    ));
                }
                match ilot.suivre_le_bureau() {
                    Some(Tour::Attente) => {
                        self.attente_suivie += 1;
                        self.attente_max = self.attente_max.max(self.attente_suivie);
                    }
                    _ => self.attente_suivie = 0,
                }
                if t >= 2.0 {
                    if let Some(o) = self.origine {
                        ilot.fenetre.set_outer_position(o);
                    }
                    self.fermer_phase(ilot);
                    self.dire(format!(
                        "  plus longue attente d'une image a la nouvelle position : {} tics",
                        self.attente_max
                    ));
                    self.passer(Etape::AvantRepli);
                }
            }
            Etape::AvantRepli => {
                let _ = ilot.suivre_le_bureau();
                if !ilot.deploye() {
                    ilot.viser(true);
                }
                if !self.verre_fait && t >= 0.05 && au_repos_deploye(ilot) {
                    self.dire("--- le verre en direct : pose, perte de la capture, retour ---");
                    ilot.montrer_contenu(false);
                    // Sur la rangee des chiffres, que DESIGN.md §4 centre
                    // verticalement. C'est la place ou DECISION.md dit que
                    // l'auteur pose l'ilot, et la seule de l'ecran ou le verre a
                    // de la lumiere a montrer : en haut, sur le noir, le verre est
                    // noir (temoin 8.14 du banc), et un galet noir ne s'y
                    // distinguerait pas d'un verre fige. Mesure au premier
                    // relevé : 3 961 pixels non noirs sur 70 968.
                    self.origine_verre = ilot.fenetre.outer_position().ok();
                    let ecran = ilot.fenetre.current_monitor().map(|m| m.size());
                    if let (Some(o), Some(ecran)) = (self.origine_verre, ecran) {
                        let h = ilot.fenetre.outer_size().height as i32;
                        ilot.fenetre.set_outer_position(winit::dpi::PhysicalPosition::new(
                            o.x,
                            (ecran.height as i32 - h) / 2,
                        ));
                    }
                    self.verre_rang = 0;
                    self.passer(Etape::Verre);
                } else if (t >= 0.05 && au_repos_deploye(ilot)) || t >= DELAI_FORME {
                    if !self.verre_fait {
                        self.verdict("verre pose en direct", false, format!("galet jamais au repos deploye en {DELAI_FORME} s"));
                    }
                    ilot.viser(false);
                    self.tailles.clear();
                    self.ouvrir_phase(ilot, "repli (la region suit la taille du galet)");
                    self.passer(Etape::Repli);
                }
            }
            Etape::Repli => {
                let _ = ilot.suivre_le_bureau();
                if let Some(r) = ilot.region_suivie() {
                    self.tailles.push((r.l, r.h));
                }
                if (t >= 0.3 && !ilot.en_mouvement() && ilot.boite_de_decoupe().is_some())
                    || t >= DELAI_FORME
                {
                    self.fermer_phase(ilot);
                    self.verifier_tailles(ilot, false);
                    self.ouvertures_avant = ilot.suivi.compteurs.ouvertures;
                    ilot.suivi.fermer();
                    self.ouvrir_phase(ilot, "reouverture SIMULEE (duplication fermee a la main)");
                    self.passer(Etape::Reouverture);
                }
            }
            Etape::Reouverture => {
                let tour = ilot.suivre_le_bureau();
                let rouverte = ilot.suivi.compteurs.ouvertures > self.ouvertures_avant;
                if (rouverte && tour == Some(Tour::Neuve)) || t >= 4.0 {
                    self.fermer_phase(ilot);
                    self.verdict(
                        "reouverture",
                        rouverte && tour == Some(Tour::Neuve),
                        format!(
                            "duplication rouverte : {rouverte}, image neuve apres {t:.3} s ; perte d'acces REELLE non provoquee"
                        ),
                    );
                    self.passer(Etape::Fin);
                }
            }
            Etape::Fin => {
                self.ecrire_fin(ilot);
                el.exit();
                return true;
            }
        }
        false
    }

    // --- La boucle -----------------------------------------------------------

    /// Au repos, chaque trame de l'ilot doit avoir une cause : une seconde
    /// entamee du chrono, ou un fond dont les octets ont vraiment change, plus
    /// une trame pour la frontiere de la phase. C'est la borne, et chacun de
    /// ses termes est compte, pas choisi. Sans la comparaison des octets,
    /// chaque trame de l'ilot exclu peut relancer une copie de sa propre
    /// fenetre (pas 1), donc une trame : `--epreuve-boucle` doit faire tomber
    /// ce verdict.
    fn verdict_de_la_boucle(&mut self, ilot: &Ilot) {
        let Some(p) = self.phase.as_ref() else { return };
        let v = &ilot.verre_compteurs;
        let secondes = p.debut.elapsed().as_secs_f64();
        let trames = v.trames - p.verre.trames;
        let differentes = v.differentes - p.verre.differentes;
        let identiques = v.identiques - p.verre.identiques;
        let borne = secondes.ceil() as u64 + differentes + 1;
        let detail = format!(
            "{trames} trames en {secondes:.2} s, borne {borne} = secondes entamees + {differentes} fonds changes + 1 ; {identiques} copies identiques au fond"
        );
        self.verdict("le verre ne boucle pas au repos", trames <= borne, detail);
    }

    // --- Le verre en direct ---------------------------------------------------

    fn avancer_verre(&mut self) {
        self.verre_rang += 1;
        self.remettre_l_horloge();
    }

    /// Copie la region deployee avec la capture de l'ilot lui-meme, suspendue
    /// pour lui, comme le temoin d'exclusion. Rend vrai quand c'est fait.
    fn prendre(&mut self, ilot: &mut Ilot, nom: &'static str, t: f64) -> bool {
        let Some(r) = ilot.region_deployee() else {
            self.dire(format!("  pas de region pour « {nom} »"));
            return true;
        };
        if ilot.suivi.tour(r) == Tour::Neuve {
            if let Some(region) = ilot.suivi.region.take() {
                self.images_verre.push((nom, region));
            }
            return true;
        }
        if t > DELAI_COPIE {
            self.dire(format!("  aucune image presentee en {DELAI_COPIE} s pour « {nom} »"));
            return true;
        }
        false
    }

    /// Trois images de l'ilot NON exclu, contenu cache, galet deploye :
    ///
    /// 1. `verre-avant` : le verre pose sur le bureau vivant ;
    /// 2. `verre-perdu` : la duplication fermee et empechee de rouvrir.
    ///    Reponse connue : l'ilot redevient le galet noir, donc AUCUN pixel non
    ///    noir a l'interieur du galet. Une image figee du verre en aurait
    ///    presque partout, et `verre-avant` en est le temoin ;
    /// 3. `verre-retour` : l'empechement leve, le verre revient.
    ///
    /// La capture de l'ilot n'est jamais active quand il n'est pas exclu : soit
    /// elle est suspendue, soit elle est empechee. Chaque changement
    /// d'affinite attend `ATTENTE_ETAT` avant la copie suivante.
    fn pas_du_verre(&mut self, ilot: &mut Ilot, t: f64) {
        match self.verre_rang {
            0 => {
                // Le fond doit etre celui de la NOUVELLE place : la region a
                // change, la copie forcee attend la prochaine presentation, et
                // l'horloge presente au moins une fois par seconde.
                let _ = ilot.suivre_le_bureau();
                let a_sa_place = match (ilot.fond_capture(), ilot.region_deployee()) {
                    (Some(f), Some(r)) => {
                        (f.x, f.y, f.largeur as i32, f.hauteur as i32) == (r.x, r.y, r.l, r.h)
                    }
                    _ => false,
                };
                let pret = a_sa_place && ilot.verre_pose() && au_repos_deploye(ilot);
                if (pret && t >= ATTENTE_ETAT) || t >= DELAI_COPIE {
                    if !pret {
                        self.dire(format!(
                            "  le verre n'a pas rejoint la rangee des chiffres en {DELAI_COPIE} s (fond a sa place {a_sa_place})"
                        ));
                    }
                    self.poses_verre[0] = ilot.verre_pose();
                    self.fond_verre = ilot.fond_capture().map(|f| Region {
                        pixels: f.pixels.clone(),
                        x: f.x,
                        y: f.y,
                        largeur: f.largeur,
                        hauteur: f.hauteur,
                    });
                    self.fenetre_verre = ilot.fenetre.inner_position().ok().map(|p| (p.x, p.y));
                    ilot.suivi_suspendu = true;
                    ilot.affinite = ilot.exclure_de_la_capture(false);
                    self.avancer_verre();
                }
            }
            1 | 9 => {
                if t >= ATTENTE_ETAT {
                    ilot.suivi.exiger_copie();
                    self.avancer_verre();
                }
            }
            2 => {
                if self.prendre(ilot, "verre-avant", t) {
                    self.avancer_verre();
                }
            }
            3 => {
                ilot.suivi.empecher_ouverture(true);
                ilot.suivi_suspendu = false;
                self.avancer_verre();
            }
            4 => {
                let _ = ilot.suivre_le_bureau();
                if t >= ATTENTE_ETAT {
                    self.poses_verre[1] = ilot.verre_pose();
                    ilot.suivi_suspendu = true;
                    ilot.suivi.empecher_ouverture(false);
                    ilot.suivi.exiger_copie();
                    self.avancer_verre();
                }
            }
            5 => {
                if self.prendre(ilot, "verre-perdu", t) {
                    self.avancer_verre();
                }
            }
            6 => {
                ilot.affinite = ilot.exclure_de_la_capture(true);
                self.avancer_verre();
            }
            7 => {
                if t >= ATTENTE_ETAT {
                    ilot.suivi_suspendu = false;
                    ilot.suivi.exiger_copie();
                    self.avancer_verre();
                }
            }
            8 => {
                let _ = ilot.suivre_le_bureau();
                if (ilot.verre_pose() && t >= 0.1) || t >= DELAI_COPIE {
                    self.poses_verre[2] = ilot.verre_pose();
                    ilot.suivi_suspendu = true;
                    ilot.affinite = ilot.exclure_de_la_capture(false);
                    self.avancer_verre();
                }
            }
            10 => {
                if self.prendre(ilot, "verre-retour", t) {
                    self.avancer_verre();
                }
            }
            _ => {
                ilot.affinite = ilot.exclure_de_la_capture(true);
                ilot.suivi_suspendu = false;
                ilot.montrer_contenu(true);
                self.conclure_verre(ilot);
                self.verre_fait = true;
                // La sonde seule se juge sur les secondes, qui changent sous
                // l'ilot immobile. Leur centre se calcule : la rangee fait
                // 3,77 F de large, centree (six cartes de 0,52 F, trois ecarts
                // de 0,03 F, deux de 0,28 F, DESIGN.md §4), et la paire des
                // secondes, 1,07 F, en tient l'extremite droite : centre a
                // W/2 + 1,35 F, avec F = min(0,225 W ; 0,42 H) (DESIGN.md §3).
                let ecran = ilot.fenetre.current_monitor().map(|m| m.size());
                if let (Some(ecran), Ok(o)) = (ecran, ilot.fenetre.outer_position()) {
                    let (w, h) = (ecran.width as f64, ecran.height as f64);
                    let f = (0.225 * w).min(0.42 * h);
                    let largeur = ilot.fenetre.outer_size().width as f64;
                    let x = (w / 2.0 + 1.35 * f - largeur / 2.0).round() as i32;
                    ilot.fenetre.set_outer_position(winit::dpi::PhysicalPosition::new(x, o.y));
                }
                self.sonde_rang = 0;
                self.passer(Etape::SondeSeule);
            }
        }
    }

    /// La sonde seule. L'ilot est pose, immobile, sur les secondes de
    /// l'horloge, dont la carte des unites tourne chaque seconde (DESIGN.md
    /// §5) : le bureau change sous lui sans qu'il bouge. Pendant trois
    /// secondes le relevé ne fait AUCUN tour de capture ; seuls la sonde (le
    /// fil de `main`, a la cadence de l'ecran) et les trames de l'ilot en
    /// font. Deux verdicts, chacun avec son epreuve :
    ///
    /// - la sonde fait des tours : `--epreuve-sans-sonde` doit le faire tomber ;
    /// - chaque trame a sa cause, meme borne qu'au repos : `--epreuve-boucle`
    ///   doit le faire tomber.
    ///
    /// Le nombre de fonds televerses est rapporte, sans verdict : c'est ce que
    /// l'epreuve sans sonde permet de comparer.
    fn pas_de_la_sonde(&mut self, ilot: &mut Ilot, t: f64) {
        if self.sonde_rang == 0 {
            let _ = ilot.suivre_le_bureau();
            let a_sa_place = match (ilot.fond_capture(), ilot.region_deployee()) {
                (Some(f), Some(r)) => {
                    (f.x, f.y, f.largeur as i32, f.hauteur as i32) == (r.x, r.y, r.l, r.h)
                }
                _ => false,
            };
            let pret = a_sa_place && ilot.verre_pose() && au_repos_deploye(ilot);
            if (pret && t >= ATTENTE_ETAT) || t >= DELAI_COPIE {
                if !pret {
                    self.dire(format!(
                        "  le verre n'a pas rejoint les secondes en {DELAI_COPIE} s (fond a sa place {a_sa_place})"
                    ));
                }
                self.ouvrir_phase(ilot, "sonde seule, ilot immobile sur les secondes");
                self.sonde_seule = true;
                self.sonde_rang = 1;
                self.remettre_l_horloge();
            }
            return;
        }
        if t < 3.0 {
            return;
        }
        self.sonde_seule = false;
        if let Some(p) = self.phase.as_ref() {
            let v = &ilot.verre_compteurs;
            let secondes = p.debut.elapsed().as_secs_f64();
            let sondes = v.sondes - p.verre.sondes;
            let trames = v.trames - p.verre.trames;
            let differentes = v.differentes - p.verre.differentes;
            let televersements = v.televersements - p.verre.televersements;
            let identiques = v.identiques - p.verre.identiques;
            let borne = secondes.ceil() as u64 + differentes + 1;
            self.verdict(
                "la sonde fait avancer la capture sous un ilot immobile",
                sondes > 0,
                format!("{sondes} tours de sonde en {secondes:.2} s ; {differentes} fonds changes, {televersements} televersements, {identiques} copies identiques"),
            );
            self.verdict(
                "la sonde ne fait pas boucler le verre",
                trames <= borne,
                format!("{trames} trames, borne {borne} = secondes entamees + {differentes} fonds changes + 1"),
            );
        }
        self.fermer_phase(ilot);
        if let Some(o) = self.origine_verre.take() {
            ilot.fenetre.set_outer_position(o);
        }
        self.passer(Etape::AvantRepli);
    }

    fn conclure_verre(&mut self, ilot: &Ilot) {
        let m = ilot.mesures_galet();
        let Some((wx, wy)) = self.fenetre_verre else {
            self.verdict("verre pose en direct", false, "position de la fenetre illisible".into());
            return;
        };
        let (gx, gy) = (wx + m.x as i32, wy + m.y as i32);
        let mut lus = Vec::new();
        for nom in ["verre-avant", "verre-perdu", "verre-retour"] {
            let e = self
                .images_verre
                .iter()
                .find(|(n, _)| *n == nom)
                .map(|(_, r)| interieur_non_noir(r, gx, gy, m));
            lus.push((nom, e));
        }
        for (nom, e) in &lus {
            let ligne = match e {
                Some((non, tot, moy)) => format!(
                    "  {nom} : {non} pixels non noirs sur {tot} a l'interieur du galet, moyenne ({:.1}, {:.1}, {:.1})",
                    moy[0], moy[1], moy[2]
                ),
                None => format!("  {nom} : image absente"),
            };
            self.dire(ligne);
        }
        let [pose_avant, pose_perdu, pose_retour] = self.poses_verre;
        let (avant, perdu, retour) = (lus[0].1, lus[1].1, lus[2].1);
        self.verdict(
            "verre pose en direct",
            pose_avant && avant.is_some_and(|e| e.1 > 0 && e.0 > 0),
            format!("fond televerse {pose_avant}, non noirs {:?}", avant.map(|e| (e.0, e.1))),
        );
        self.verdict(
            "capture perdue : galet noir, sans image figee",
            !pose_perdu && perdu.is_some_and(|e| e.1 > 0 && e.0 == 0),
            format!("fond televerse {pose_perdu}, non noirs {:?} (temoin avant : {:?})", perdu.map(|e| (e.0, e.1)), avant.map(|e| e.0)),
        );
        self.verdict(
            "verre revenu apres reouverture",
            pose_retour && retour.is_some_and(|e| e.1 > 0 && e.0 > 0),
            format!("fond televerse {pose_retour}, non noirs {:?}", retour.map(|e| (e.0, e.1))),
        );
        // La ligne que `outils/verre-en-direct.py` relit pour refaire le rendu
        // hors-ecran sur le meme fond et le comparer a ce que DWM a compose.
        let fond = self.fond_verre.as_ref().map(|f| (f.x, f.y, f.largeur, f.hauteur));
        let image = self
            .images_verre
            .iter()
            .find(|(n, _)| *n == "verre-avant")
            .map(|(_, r)| (r.x, r.y, r.largeur, r.hauteur));
        if let (Some(f), Some(a)) = (fond, image) {
            self.dire(format!(
                "VERRE-DIRECT echelle={} fenetre={wx},{wy} galet={},{},{},{} fond={},{},{},{} avant={},{},{},{}",
                self.echelle, m.x, m.y, m.l, m.h, f.0, f.1, f.2, f.3, a.0, a.1, a.2, a.3
            ));
        }
    }

    // --- Le temoin d'exclusion ----------------------------------------------

    fn pas_du_temoin(&mut self, ilot: &mut Ilot, t: f64) {
        let (nom, visible, exclu) = SEQUENCE[self.rang];
        match self.sous {
            Sous::Appliquer => {
                if self.rang == 0 {
                    // Debut d'une tentative : le galet doit etre deploye au
                    // repos, sinon la region du temoin ne serait pas la sienne.
                    if !au_repos_deploye(ilot) {
                        ilot.suivi_suspendu = false;
                        ilot.fenetre.set_visible(true);
                        if !ilot.deploye() {
                            ilot.viser(true);
                        }
                        if t < DELAI_FORME {
                            return;
                        }
                        self.dire(format!(
                            "  tentative {} : le galet n'atteint pas le repos deploye en {DELAI_FORME} s",
                            self.tentative
                        ));
                    }
                    self.temoin = ilot.region_deployee();
                    self.images.clear();
                    self.tentative_valide = true;
                    self.raison = None;
                    ilot.suivi_suspendu = true;
                    let temoin = self.temoin;
                    let n = self.tentative;
                    self.dire(format!("  tentative {n}, region du temoin {temoin:?}"));
                }
                ilot.fenetre.set_visible(visible);
                let a = ilot.exclure_de_la_capture(exclu);
                ilot.affinite = a;
                self.sous = Sous::Attendre;
                self.remettre_l_horloge();
            }
            Sous::Attendre => {
                if let Some(r) = self.temoin {
                    ilot.suivi.tour(r);
                }
                if t >= ATTENTE_ETAT {
                    let attendue = if exclu { 17 } else { 0 };
                    if ilot.affinite != Some(attendue) {
                        self.tentative_valide = false;
                        self.raison = Some(format!(
                            "affinite relue {:?} pour « {nom} », {attendue} attendue",
                            ilot.affinite
                        ));
                    }
                    if visible && !au_repos_deploye(ilot) {
                        self.tentative_valide = false;
                        self.raison = Some(format!("geometrie de l'ilot changee avant « {nom} »"));
                    }
                    ilot.suivi.exiger_copie();
                    self.sous = Sous::Copier;
                    self.remettre_l_horloge();
                }
            }
            Sous::Copier => {
                let Some(r) = self.temoin else {
                    self.tentative_valide = false;
                    self.raison = Some("pas de region de temoin".into());
                    self.conclure_temoin(ilot);
                    return;
                };
                let prise = ilot.suivi.tour(r) == Tour::Neuve;
                if prise {
                    if let Some(region) = ilot.suivi.region.take() {
                        self.images.push((nom, region));
                    }
                } else if t <= DELAI_COPIE {
                    return;
                } else {
                    self.tentative_valide = false;
                    self.raison = Some(format!(
                        "aucune image presentee en {DELAI_COPIE} s pour « {nom} »"
                    ));
                }
                if visible && !au_repos_deploye(ilot) {
                    self.tentative_valide = false;
                    self.raison = Some(format!("geometrie de l'ilot changee pendant « {nom} »"));
                }
                self.rang += 1;
                self.sous = Sous::Appliquer;
                self.remettre_l_horloge();
                if self.rang == SEQUENCE.len() {
                    self.conclure_temoin(ilot);
                }
            }
        }
    }

    fn image(&self, nom: &str) -> Option<&Region> {
        self.images.iter().find(|(n, _)| *n == nom).map(|(_, r)| r)
    }

    fn conclure_temoin(&mut self, ilot: &mut Ilot) {
        // L'etat normal, quoi qu'il arrive ensuite.
        ilot.fenetre.set_visible(true);
        ilot.affinite = ilot.exclure_de_la_capture(true);
        ilot.suivi_suspendu = false;

        let marge = ilot.marge();
        let galet = self.temoin.map(|r| Rect {
            x: r.x + marge,
            y: r.y + marge,
            l: r.l - 2 * marge,
            h: r.h - 2 * marge,
        });
        let comparer = |a: &str, b: &str, s: &Releve| match (s.image(a), s.image(b)) {
            (Some(x), Some(y)) => ecart(x, y, galet),
            _ => None,
        };
        let c2 = comparer("cache-1", "cache-2", self);
        let c3 = comparer("cache-1", "cache-3", self);
        let e_exclu = comparer("cache-1", "exclu", self);
        let e_non = comparer("cache-1", "non-exclu", self);
        let e_ret = comparer("cache-1", "exclu-retabli", self);

        let mut lignes = Vec::new();
        for (nom, e) in [
            ("cache-2", c2),
            ("cache-3", c3),
            ("exclu", e_exclu),
            ("non-exclu", e_non),
            ("exclu-retabli", e_ret),
        ] {
            lignes.push(match e {
                Some(e) => format!(
                    "  cache-1 contre {nom} : {} px differents sur {} dans le galet (pire {}), {} sur {} dans toute la region (pire {})",
                    e.galet, e.galet_total, e.pire_galet, e.region, e.region_total, e.pire_region
                ),
                None => format!("  cache-1 contre {nom} : image absente ou geometrie differente"),
            });
        }
        for l in lignes {
            self.dire(l);
        }

        let zero = |e: Option<Ecart>| e.is_some_and(|e| e.region == 0);
        let fond_stable = zero(c2) && zero(c3);
        if !fond_stable {
            self.tentative_valide = false;
            self.raison.get_or_insert_with(|| {
                "le fond a change entre les images cachees : une autre fenetre est passee".into()
            });
        }
        if !self.tentative_valide {
            let n = self.tentative;
            let raison = self.raison.clone().unwrap_or_default();
            self.dire(format!("  tentative {n} NON CONCLUANTE : {raison}"));
            if self.tentative < TENTATIVES {
                self.tentative += 1;
                self.rang = 0;
                self.sous = Sous::Appliquer;
                self.remettre_l_horloge();
                return;
            }
            for nom in [
                "temoin : la capture voit l'ilot non exclu",
                "ilot absent de la capture",
                "ilot absent apres retablissement",
            ] {
                self.verdict(nom, false, format!("non concluant apres {TENTATIVES} tentatives"));
            }
        } else {
            let n = self.tentative;
            self.dire(format!("  tentative {n} concluante : fond stable sur les trois images cachees"));
            self.verdict(
                "temoin : la capture voit l'ilot non exclu",
                e_non.is_some_and(|e| e.galet > 0),
                format!("{:?} px dans le galet", e_non.map(|e| e.galet)),
            );
            self.verdict(
                "ilot absent de la capture",
                zero(e_exclu),
                format!("{:?} px dans la region", e_exclu.map(|e| e.region)),
            );
            self.verdict(
                "ilot absent apres retablissement",
                zero(e_ret),
                format!("{:?} px dans la region", e_ret.map(|e| e.region)),
            );
        }
        self.ouvrir_phase(ilot, "l'ilot se repeint a chaque tic, region fixe");
        self.passer(Etape::Repeint);
    }

    // --- Les ecritures ------------------------------------------------------

    fn ecrire_installation(&mut self, ilot: &mut Ilot) {
        self.dire("=== relevé de la capture du bureau sous l'ilot ===");
        let c = self.conscience_declaree.clone();
        self.dire(format!(
            "conscience du DPI declaree avant la boucle : {} (code {:#010x}) ; relue juste apres : {} (2 = par moniteur) ; couche de compatibilite de cet executable : {:?}{}",
            if c.acceptee { "acceptee" } else { "REFUSEE" },
            c.erreur,
            c.lue,
            c.couche,
            if self.epreuve_dpi { "  [EPREUVE : processus declare inconscient]" } else { "" }
        ));
        self.dire(format!("echelle de la fenetre de l'ilot : {}", self.echelle));
        self.dire(format!("adaptateur wgpu : {}", self.adaptateur_wgpu));
        self.dire(format!(
            "affinite de l'ilot relue apres construction : {:?} (17 = exclu de la capture)",
            ilot.affinite
        ));
        let affinite = ilot.affinite == Some(17);
        self.verdict("exclusion posee", affinite, format!("{:?}", ilot.affinite));
        match ilot.suivi.capteur() {
            Some(c) => {
                let s = c.sortie;
                let ligne = format!(
                    "duplication : adaptateur {} ; sortie ({}, {}) {}x{} ; texture {}x{} ; conscience relue a l'ouverture {}",
                    c.adaptateur, s.x, s.y, s.l, s.h, c.texture.0, c.texture.1, c.conscience
                );
                let accord = c.dpi_accorde();
                let detail = format!(
                    "coordonnees {}x{} contre texture {}x{}",
                    s.l, s.h, c.texture.0, c.texture.1
                );
                self.dire(ligne);
                self.verdict("DPI accorde", accord, detail);
            }
            None => {
                let plainte = ilot.suivi.plainte.clone();
                self.dire(format!("duplication NON ouverte : {plainte:?}"));
                self.verdict("DPI accorde", false, "pas de duplication".into());
            }
        }
        self.dire(format!("region suivie : {:?}", ilot.region_suivie()));
    }

    /// La region doit avoir suivi la taille du galet : plusieurs tailles vues
    /// pendant la course, et au repos la region ENTIERE (position et taille)
    /// egale a la boite de la decoupe GDI posee sur la fenetre, elargie de la
    /// marge. La boite est relue a Windows : c'est un chemin independant des
    /// mesures dont la region suivie est tiree, et il peut donc la contredire.
    fn verifier_tailles(&mut self, ilot: &mut Ilot, deploye: bool) {
        let mut distinctes = self.tailles.clone();
        distinctes.sort();
        distinctes.dedup();
        let finale = ilot.region_suivie();
        let marge = ilot.marge();
        let attendue = ilot.boite_de_decoupe().map(|b| Rect {
            x: b.x - marge,
            y: b.y - marge,
            l: b.l + 2 * marge,
            h: b.h + 2 * marge,
        });
        let vrai = distinctes.len() >= 2 && finale.is_some() && finale == attendue;
        let nom = if deploye {
            "region suit le deploiement"
        } else {
            "region suit le repli"
        };
        let detail = format!(
            "{} tailles distinctes, de {:?} a {:?} ; finale {:?}, attendue {:?}",
            distinctes.len(),
            distinctes.first(),
            distinctes.last(),
            finale,
            attendue
        );
        self.verdict(nom, vrai, detail);
        self.tailles.clear();
    }

    fn ecrire_fin(&mut self, ilot: &mut Ilot) {
        let c = ilot.suivi.compteurs.clone();
        self.dire(format!(
            "--- total ({:.2} s) : {c:?}",
            self.debut.elapsed().as_secs_f64()
        ));
        self.dire(format!(
            "pertes de focus de l'ilot pendant le relevé (ignorees, sans repli) : {}",
            self.focus_perdus
        ));
        if !self.epreuve_dpi {
            self.dire(format!("perte d'acces reelle pendant le relevé : {}", c.perdus));
        }
        let justes = self.verdicts.iter().filter(|v| v.1).count();
        let total = self.verdicts.len();
        self.dire(format!("verdicts : {justes} sur {total}"));

        let _ = std::fs::create_dir_all(&self.dossier);
        for (nom, region) in &self.images_verre {
            let chemin = self.dossier.join(format!("{nom}.png"));
            if let Err(e) = ecrire_png(&chemin, region) {
                println!("!! {} : {e}", chemin.display());
            }
        }
        if let Some(f) = &self.fond_verre {
            let chemin = self.dossier.join("verre-fond.png");
            if let Err(e) = ecrire_png(&chemin, f) {
                println!("!! {} : {e}", chemin.display());
            }
        }
        for (nom, region) in &self.images {
            let chemin = self.dossier.join(format!("bureau-{nom}.png"));
            match ecrire_png(&chemin, region) {
                Ok(()) => println!("-> {}", chemin.display()),
                Err(e) => println!("!! {} : {e}", chemin.display()),
            }
        }
        let chemin = self.dossier.join("releve.txt");
        match std::fs::write(&chemin, &self.texte) {
            Ok(()) => println!("-> {}", chemin.display()),
            Err(e) => println!("!! {} : {e}", chemin.display()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Ecart {
    region: u64,
    region_total: u64,
    pire_region: u8,
    galet: u64,
    galet_total: u64,
    pire_galet: u8,
}

/// Pixels differents entre deux regions de meme geometrie, sur toute la region
/// et dans le rectangle du galet.
fn ecart(a: &Region, b: &Region, galet: Option<Rect>) -> Option<Ecart> {
    if (a.x, a.y, a.largeur, a.hauteur) != (b.x, b.y, b.largeur, b.hauteur) {
        return None;
    }
    let mut e = Ecart {
        region: 0,
        region_total: a.largeur as u64 * a.hauteur as u64,
        pire_region: 0,
        galet: 0,
        galet_total: 0,
        pire_galet: 0,
    };
    for j in 0..a.hauteur {
        for i in 0..a.largeur {
            let k = ((j * a.largeur + i) * 4) as usize;
            let d = (0..3)
                .map(|c| a.pixels[k + c].abs_diff(b.pixels[k + c]))
                .max()
                .unwrap_or(0);
            let (x, y) = (a.x + i as i32, a.y + j as i32);
            let dedans =
                galet.is_some_and(|g| x >= g.x && x < g.x + g.l && y >= g.y && y < g.y + g.h);
            if dedans {
                e.galet_total += 1;
            }
            if d > 0 {
                e.region += 1;
                e.pire_region = e.pire_region.max(d);
                if dedans {
                    e.galet += 1;
                    e.pire_galet = e.pire_galet.max(d);
                }
            }
        }
    }
    Some(e)
}

/// Les pixels non noirs a l'interieur du galet, sa moyenne, et le nombre de
/// pixels lus. L'interieur est ce que le nuanceur couvre ENTIEREMENT : distance
/// signee au plus -0,5, la regle meme de `galet.wgsl`. Le galet noir y est
/// exactement (0, 0, 0).
///
/// ~~La silhouette retrecie de deux pixels en x~~ : c'etait la premiere
/// realisation, et le premier relevé sur la rangee des chiffres l'a fait
/// tomber. Six pixels « non noirs » dans le galet perdu, qui etaient tous dans
/// le ruban, la ou le coin presque horizontal etale un pixel de transition sur
/// plus de deux pixels d'horizontale. Relu par la distance, hors de
/// l'application : 0 pixel non noir a couverture pleine, 156 dans le ruban.
fn interieur_non_noir(r: &Region, gx: i32, gy: i32, m: Mesures) -> (u64, u64, [f64; 3]) {
    let (l, h) = (m.l as i32, m.h as i32);
    let (mut non, mut tot, mut somme) = (0u64, 0u64, [0.0f64; 3]);
    for j in 0..h {
        for i in 0..l {
            let d = crate::galet::distance_au_galet(i as f32 + 0.5, j as f32 + 0.5, m.l, m.h, m.rayon, m.n);
            if d > -0.5 {
                continue;
            }
            let (x, y) = (gx + i - r.x, gy + j - r.y);
            if x < 0 || y < 0 || x >= r.largeur as i32 || y >= r.hauteur as i32 {
                continue;
            }
            let k = ((y as u32 * r.largeur + x as u32) * 4) as usize;
            let p = &r.pixels[k..k + 3];
            tot += 1;
            if p.iter().any(|&c| c != 0) {
                non += 1;
            }
            for c in 0..3 {
                somme[c] += p[c] as f64;
            }
        }
    }
    let n = tot.max(1) as f64;
    (non, tot, [somme[0] / n, somme[1] / n, somme[2] / n])
}

fn quantile(v: &mut [f64], q: f64) -> String {
    if v.is_empty() {
        return "—".into();
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    format!("{:.3} ms", v[(((v.len() - 1) as f64) * q).round() as usize])
}

pub(crate) fn ecrire_png(chemin: &std::path::Path, r: &Region) -> Result<(), String> {
    let f = std::fs::File::create(chemin).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), r.largeur, r.hauteur);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(&r.pixels).map_err(|e| e.to_string())
}
