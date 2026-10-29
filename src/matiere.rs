//! La matiere du galet, entre le noir opaque et le verre : deux ressorts en
//! cascade, la croissance du givre `g` puis sa fonte `f`, tous deux dans [0, 1].
//!
//! Les constantes sont celles de `DESIGN.md` §6 (raideur 400, amortissement
//! critique 40, masse 1) : une matiere qui change dans l'ilot bouge comme l'ilot
//! qui se deploie, la charte n'a qu'un seul mouvement. L'ordre est celui de la
//! physique : en montant, la fonte ne part que quand le givre a atteint son
//! dernier cristal ; en descendant, le givre ne se retire que quand le verre a
//! regele. L'etat `(g, f)` est la verite, pas la cible : un clic pendant la
//! transformation change les cibles, et chaque ressort repart de sa position et
//! de sa vitesse (`DESIGN.md` §6, « Interruption »).
//!
//! Les ressorts travaillent en CENTIEMES de course, et non dans [0, 1] : les
//! seuils de repos de `ressort.rs` sont en pixels (0,1 de position, 10 par
//! seconde), et poses sur une course de 1 ils arreteraient la matiere au dixieme
//! de sa course. Sur 100, le repos tombe au millieme de la course, sous tout ce
//! que le front peut montrer (`RECETTES.md`, « Des seuils de repos ont une
//! UNITE »).

use crate::ressort::Ressort;

/// La course des deux ressorts.
const COURSE: f32 = 100.0;

/// Le relais : la fonte part quand la croissance a allume son dernier cristal,
/// c'est-a-dire quand `g x (1 + w)` atteint 1, `w` etant la largeur du front en
/// unites d'arrivee. Le seuil derive donc de la largeur du front que le
/// generateur du givre rend ; celui-ci est la valeur pour le galet deploye
/// (`givre::Cristaux::largeur_front`, imprime par `--capture-cristaux`), arrondi
/// au-dessus. En descendant, le givre ne se retire que quand la fonte est
/// revenue sous le meme ecart a zero.
pub const RELAIS: f32 = 0.965;

/// Le choix de l'auteur, retenu d'un lancement a l'autre (DESIGN.md §6,
/// « Matiere » ; DECISION-APRES-EPREUVE.md F) : `%LOCALAPPDATA%\Veilleuse\
/// matiere.txt` contient « verre » ou « opaque ». Un seul choix pour la
/// machine, pas un par ecran : c'est la matiere de l'ilot, pas sa place. Un
/// fichier absent ou illisible vaut opaque.
pub fn fichier() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(std::path::PathBuf::from(base).join("Veilleuse").join("matiere.txt"))
}

/// Le choix force par un mode scripte (`--matiere=verre|opaque` sur les
/// vraies fenetres : l'oeil, les relevés) : 0 le fichier decide, 1 le verre,
/// 2 l'opaque. Force, le choix ne se lit ni ne s'ecrit dans `matiere.txt`.
static CHOIX_FORCE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn forcer_le_choix(verre: Option<bool>) {
    let etat = match verre {
        None => 0,
        Some(true) => 1,
        Some(false) => 2,
    };
    CHOIX_FORCE.store(etat, std::sync::atomic::Ordering::Relaxed);
}

fn choix_force() -> Option<bool> {
    match CHOIX_FORCE.load(std::sync::atomic::Ordering::Relaxed) {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
}

/// Vrai si le verre est choisi.
pub fn choix_memorise() -> bool {
    if let Some(force) = choix_force() {
        return force;
    }
    let Some(chemin) = fichier() else {
        return false;
    };
    std::fs::read_to_string(chemin)
        .map(|t| t.trim() == "verre")
        .unwrap_or(false)
}

/// Retient le choix. Un choix qui ne s'ecrit pas n'est pas une panne : l'ilot
/// repartira opaque, et on ne s'en plaint pas.
pub fn memoriser(verre: bool) {
    if choix_force().is_some() {
        return;
    }
    let Some(chemin) = fichier() else {
        return;
    };
    if let Some(parent) = chemin.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(chemin, if verre { "verre\n" } else { "opaque\n" });
}

#[derive(Clone, Copy, Debug)]
pub struct Matiere {
    givre: Ressort,
    fonte: Ressort,
    /// La cible : vrai vers le verre, faux vers le galet opaque.
    vers_le_verre: bool,
}

impl Matiere {
    /// Posee, immobile, sur le galet opaque ou sur le verre.
    pub fn posee(verre: bool) -> Matiere {
        let v = if verre { COURSE } else { 0.0 };
        Matiere {
            givre: Ressort::pose(v),
            fonte: Ressort::pose(v),
            vers_le_verre: verre,
        }
    }

    /// Change la cible, et rien d'autre.
    pub fn viser(&mut self, verre: bool) {
        self.vers_le_verre = verre;
    }

    pub fn vers_le_verre(&self) -> bool {
        self.vers_le_verre
    }

    /// `(g, f)` dans [0, 1].
    pub fn etat(&self) -> (f32, f32) {
        (
            (self.givre.position / COURSE).clamp(0.0, 1.0),
            (self.fonte.position / COURSE).clamp(0.0, 1.0),
        )
    }

    pub fn au_repos(&self) -> bool {
        self.givre.au_repos() && self.fonte.au_repos()
    }

    /// Avance de `dt` secondes. Les cibles sont posees ICI, a chaque pas, parce
    /// que le relais depend de l'etat courant : un ressort retenu vise sa
    /// propre position, et s'y pose sans saut.
    pub fn avancer(&mut self, dt: f32) {
        let (g, f) = self.etat();
        if self.vers_le_verre {
            self.givre.viser(COURSE);
            if g >= RELAIS {
                self.fonte.viser(COURSE);
            } else {
                self.fonte.viser(self.fonte.position);
            }
        } else {
            self.fonte.viser(0.0);
            if f <= 1.0 - RELAIS {
                self.givre.viser(0.0);
            } else {
                self.givre.viser(self.givre.position);
            }
        }
        self.givre.avancer(dt);
        self.fonte.avancer(dt);
    }

    /// Pose la matiere d'un coup sur sa cible : le systeme demande moins de
    /// mouvement (`DESIGN.md` §5, point 6), ou la capture fige un instant.
    pub fn poser(&mut self) {
        *self = Matiere::posee(self.vers_le_verre);
    }

    /// L'etat `(g, f)` a l'instant `t` secondes d'une transformation partie du
    /// repos, vers le verre (`aller`) ou vers le galet opaque, avec un
    /// changement de cible a `interruption` secondes s'il y en a un. Pas de
    /// trame de 1/144 s, la cadence de l'ecran de l'auteur : c'est ce que la
    /// fenetre ferait, au pas pres.
    pub fn a_l_instant(t: f32, aller: bool, interruption: Option<f32>) -> Matiere {
        const PAS: f32 = 1.0 / 144.0;
        let mut m = Matiere::posee(!aller);
        m.viser(aller);
        let mut ecoule = 0.0f32;
        let mut interrompue = false;
        while ecoule + 1e-6 < t {
            let pas = PAS.min(t - ecoule);
            // La cible change au debut de la premiere trame qui suit l'instant
            // de l'interruption : c'est ce que ferait un clic entre deux trames.
            if let Some(i) = interruption {
                if !interrompue && ecoule >= i - 1e-6 {
                    m.viser(!aller);
                    interrompue = true;
                }
            }
            m.avancer(pas);
            ecoule += pas;
        }
        m
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    const PAS: f32 = 1.0 / 144.0;

    /// La cascade : le givre part au clic, la fonte ne part qu'au relais, et
    /// aucun des deux ne depasse ni ne recule.
    #[test]
    fn la_fonte_attend_le_dernier_cristal() {
        let mut m = Matiere::posee(false);
        m.viser(true);
        let (mut g_prec, mut f_prec) = (0.0f32, 0.0f32);
        let mut depart_fonte = None;
        for trame in 0..300 {
            m.avancer(PAS);
            let (g, f) = m.etat();
            assert!(g >= g_prec - 1e-5 && f >= f_prec - 1e-5, "recul a la trame {trame}");
            assert!(g <= 1.0 && f <= 1.0);
            if f > 0.0 && depart_fonte.is_none() {
                depart_fonte = Some((trame, g));
            }
            if f > 0.0 {
                assert!(g >= RELAIS - 1e-3, "la fonte a demarre avant le relais : g = {g}");
            }
            g_prec = g;
            f_prec = f;
        }
        let (trame, g) = depart_fonte.expect("la fonte n'est jamais partie");
        // Le relais a 0,965 tombe a 256 ms sur le ressort de la charte.
        let ms = trame as f32 * PAS * 1000.0;
        assert!((240.0..275.0).contains(&ms), "depart de la fonte a {ms} ms, g = {g}");
        assert!(m.au_repos(), "pas posee apres 300 trames");
        assert_eq!(m.etat(), (1.0, 1.0));
    }

    /// L'oeil voit la fin vers 600 ms, et le calcul s'arrete avant 900 ms :
    /// deux durees, toutes deux vraies (`RECETTES.md`).
    #[test]
    fn deux_durees() {
        let m = Matiere::a_l_instant(0.600, true, None);
        let (g, f) = m.etat();
        assert!(g > 0.999 && f > 0.98, "a 600 ms : g = {g}, f = {f}");
        let mut m = Matiere::posee(false);
        m.viser(true);
        let mut trames = 0;
        while !m.au_repos() && trames < 1000 {
            m.avancer(PAS);
            trames += 1;
        }
        let ms = trames as f32 * PAS * 1000.0;
        assert!(ms < 900.0, "encore en calcul a {ms} ms");
    }

    /// Le retour est le chemin inverse : la fonte revient d'abord, le givre ne
    /// se retire qu'ensuite.
    #[test]
    fn le_retour_regele_avant_de_retirer_le_givre() {
        let mut m = Matiere::posee(true);
        m.viser(false);
        for _ in 0..300 {
            m.avancer(PAS);
            let (g, f) = m.etat();
            if g < 1.0 - 1e-4 {
                assert!(f <= 1.0 - RELAIS + 1e-3, "le givre recule avant que le verre ait regele : f = {f}");
            }
        }
        assert!(m.au_repos());
        assert_eq!(m.etat(), (0.0, 0.0));
    }

    /// L'interruption : un clic a mi-course ne fait que changer la cible, et la
    /// position ne saute pas.
    #[test]
    fn une_interruption_ne_saute_pas() {
        let mut m = Matiere::posee(false);
        m.viser(true);
        for _ in 0..12 {
            m.avancer(PAS);
        }
        let avant = m.etat();
        assert!(avant.0 > 0.2 && avant.0 < 0.9, "pas en plein vol : {avant:?}");
        m.viser(false);
        assert_eq!(m.etat(), avant, "la cible a deplace l'etat");
        m.avancer(PAS);
        let apres = m.etat();
        assert!((apres.0 - avant.0).abs() < 0.08, "saut : {avant:?} -> {apres:?}");
        for _ in 0..400 {
            m.avancer(PAS);
        }
        assert!(m.au_repos());
        assert_eq!(m.etat(), (0.0, 0.0));
        // Et par le chemin de la capture, au meme instant.
        let c = Matiere::a_l_instant(12.0 * PAS + PAS, true, Some(12.0 * PAS));
        assert!((c.etat().0 - apres.0).abs() < 1e-4, "{:?} contre {apres:?}", c.etat());
    }

    /// Le temoin qui peut tomber : au relais force a zero, la fonte partirait
    /// avec le givre. On le verifie a l'envers, sur `a_l_instant` : a 100 ms le
    /// givre est parti et la fonte non.
    #[test]
    fn a_cent_millisecondes_le_givre_seul() {
        let (g, f) = Matiere::a_l_instant(0.100, true, None).etat();
        assert!(g > 0.5 && g < 0.7, "g = {g}");
        assert_eq!(f, 0.0);
    }
}
