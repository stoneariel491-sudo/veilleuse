//! La mecanique du volet : les courbes, les durees, l'etat de chaque carte.
//!
//! Un vrai volet d'aeroport. La carte est coupee en deux par le pli ; au
//! changement, la moitie HAUTE affiche deja la NOUVELLE valeur et la moitie
//! BASSE encore l'ANCIENNE. Une carte tombe par-dessus : sa moitie haute porte
//! l'ancienne et pivote de 0 a -90 deg, puis sa moitie basse porte la nouvelle
//! et pivote de +90 a 0 deg.

use std::f32::consts::FRAC_PI_2;

/// Une Bezier cubique de CSS : les points extremes sont (0,0) et (1,1), seuls
/// les deux points de controle sont donnes.
#[derive(Clone, Copy)]
pub struct Bezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl Bezier {
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Bezier {
        Bezier { x1, y1, x2, y2 }
    }

    fn axe(p1: f32, p2: f32, t: f32) -> f32 {
        let u = 1.0 - t;
        3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t
    }

    /// L'avancement pour une fraction de temps donnee.
    ///
    /// Une Bezier de CSS donne x et y en fonction d'un parametre interne ; il
    /// faut donc l'inverser en x. Vingt bissections ramenent l'erreur sous
    /// 10^-6, soit un millieme de pixel a cette echelle.
    pub fn avancement(&self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let (mut bas, mut haut) = (0.0f32, 1.0f32);
        for _ in 0..20 {
            let milieu = 0.5 * (bas + haut);
            if Bezier::axe(self.x1, self.x2, milieu) < x {
                bas = milieu;
            } else {
                haut = milieu;
            }
        }
        Bezier::axe(self.y1, self.y2, 0.5 * (bas + haut))
    }
}

/// Il tombe, il accelere.
pub const CHUTE: Bezier = Bezier::new(0.32, 0.0, 0.67, 0.0);
/// Il se pose, il ralentit. Aucun rebond, aucun depassement : la reference est
/// silencieuse.
pub const POSE: Bezier = Bezier::new(0.33, 1.0, 0.68, 1.0);

/// Durees de la bascule, en millisecondes.
pub const CHUTE_MS: f32 = 160.0;
pub const POSE_MS: f32 = 200.0;
/// Les secondes tournent soixante fois plus souvent : leur volet est plus
/// court, et la rangee reste au repos 70 % de chaque seconde.
pub const CHUTE_MS_SECONDES: f32 = 130.0;
pub const POSE_MS_SECONDES: f32 = 170.0;

// --- Le variateur de DESIGN.md §7 -------------------------------------------

/// La courbe `ease-in-out` de CSS, telle quelle.
pub const VARIATEUR: Bezier = Bezier::new(0.42, 0.0, 0.58, 1.0);

/// Descente vers le gris quand le livre se ferme. Lente : « la piece
/// s'assombrit un peu » n'est pas un evenement, c'est un glissement.
pub const BAISSE_MS: f32 = 1200.0;
/// Remontee a la reprise. Deux fois plus rapide : quand le travail reprend, la
/// lumiere doit etre la avant les yeux, pas apres.
pub const REMONTEE_MS: f32 = 600.0;

/// La lampe sur variateur : une valeur de 0 (pleine lumiere) a 1 (basse).
///
/// Le parametre suivi est le temps LINEAIRE le long de la course ; c'est la
/// courbe qui le plie ensuite. Une interruption ne fait donc que renverser le
/// sens depuis la position courante, et la valeur sortie reste continue. Sa
/// derivee, elle, change de signe d'un coup — c'est ce que fait une
/// `ease-in-out` a ses propres bouts, et sur 1200 ms entre deux gris voisins
/// cela ne se voit pas.
pub struct Variateur {
    /// Avancement lineaire dans [0, 1].
    t: f32,
}

impl Variateur {
    pub fn allume() -> Variateur {
        Variateur { t: 0.0 }
    }

    pub fn avancer(&mut self, dt: f32, vers_le_bas: bool) {
        let pas = if vers_le_bas {
            dt / BAISSE_MS
        } else {
            -dt / REMONTEE_MS
        };
        self.t = (self.t + pas).clamp(0.0, 1.0);
    }

    /// 0 = pleine lumiere, 1 = basse.
    pub fn niveau(&self) -> f32 {
        VARIATEUR.avancement(self.t)
    }

    /// Pose le variateur au bout de sa course, sans transition. Sert aux
    /// captures.
    pub fn figer(&mut self, en_bas: bool) {
        self.t = if en_bas { 1.0 } else { 0.0 };
    }

    pub fn en_mouvement(&self, vers_le_bas: bool) -> bool {
        if vers_le_bas {
            self.t < 1.0
        } else {
            self.t > 0.0
        }
    }
}

/// Ce qu'une carte est en train de faire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Etat {
    /// Rien ne bouge : deux moities fixes, toutes deux a la nouvelle valeur.
    Repos,
    /// Le volet HAUT tombe, angle de 0 a -PI/2. Le volet bas n'existe pas
    /// encore : les deux ne se croisent jamais.
    Chute(f32),
    /// Le volet BAS se pose, angle de +PI/2 a 0. Le volet haut a disparu.
    Pose(f32),
}

pub struct Carte {
    pub ancienne: u8,
    pub nouvelle: u8,
    /// Millisecondes ecoulees depuis le debut de la bascule ; hors bascule,
    /// vaut la duree totale et l'etat est Repos.
    ecoule: f32,
    chute_ms: f32,
    pose_ms: f32,
}

impl Carte {
    pub fn nouvelle(valeur: u8, secondes: bool) -> Carte {
        let (chute_ms, pose_ms) = if secondes {
            (CHUTE_MS_SECONDES, POSE_MS_SECONDES)
        } else {
            (CHUTE_MS, POSE_MS)
        };
        Carte {
            ancienne: valeur,
            nouvelle: valeur,
            ecoule: chute_ms + pose_ms,
            chute_ms,
            pose_ms,
        }
    }

    pub fn total(&self) -> f32 {
        self.chute_ms + self.pose_ms
    }

    pub fn en_mouvement(&self) -> bool {
        self.ecoule < self.total()
    }

    /// Lance une bascule vers `valeur`.
    ///
    /// Interruption : l'etat courant est la verite, pas la cible. Si la carte
    /// tombe encore, le volet haut porte toujours l'ancienne valeur et la
    /// moitie haute fixe porte deja la nouvelle — il suffit donc de changer
    /// cette derniere, et le mouvement continue sans saut ni reprise a zero.
    /// Si elle se pose, l'ancienne valeur a disparu de l'ecran : c'est la
    /// valeur en train d'arriver qui devient l'ancienne.
    pub fn basculer(&mut self, valeur: u8) {
        if valeur == self.nouvelle {
            return;
        }
        match self.etat() {
            Etat::Repos => {
                self.ancienne = self.nouvelle;
                self.nouvelle = valeur;
                self.ecoule = 0.0;
            }
            Etat::Chute(_) => {
                // Le volet haut continue sa chute avec l'ancienne valeur.
                self.nouvelle = valeur;
            }
            Etat::Pose(_) => {
                self.ancienne = self.nouvelle;
                self.nouvelle = valeur;
                self.ecoule = 0.0;
            }
        }
    }

    /// Fait avancer l'animation de `dt` millisecondes.
    pub fn avancer(&mut self, dt: f32) {
        if self.en_mouvement() {
            self.ecoule = (self.ecoule + dt).min(self.total());
        }
    }

    /// Change de valeur sans volet, d'un coup. Sert quand le systeme demande
    /// moins de mouvement.
    pub fn poser_sec(&mut self, valeur: u8) {
        self.ancienne = valeur;
        self.nouvelle = valeur;
        self.ecoule = self.total();
    }

    /// Pose la bascule a un instant precis, en fraction de sa duree totale.
    /// Sert a juger l'animation image par image plutot qu'en la regardant
    /// tourner.
    pub fn figer(&mut self, phase: f32) {
        self.ecoule = phase.clamp(0.0, 1.0) * self.total();
    }

    pub fn etat(&self) -> Etat {
        if self.ecoule >= self.total() {
            return Etat::Repos;
        }
        if self.ecoule < self.chute_ms {
            let a = CHUTE.avancement(self.ecoule / self.chute_ms);
            Etat::Chute(-FRAC_PI_2 * a)
        } else {
            let a = POSE.avancement((self.ecoule - self.chute_ms) / self.pose_ms);
            Etat::Pose(FRAC_PI_2 * (1.0 - a))
        }
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    #[test]
    fn les_courbes_partent_de_zero_et_arrivent_a_un() {
        for courbe in [CHUTE, POSE] {
            assert!(courbe.avancement(0.0).abs() < 1e-4);
            assert!((courbe.avancement(1.0) - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn aucun_depassement_nulle_part() {
        // DESIGN.md §5 : aucun rebond, aucun depassement. La reference est
        // silencieuse.
        for courbe in [CHUTE, POSE] {
            let mut precedent = 0.0;
            for i in 0..=1000 {
                let a = courbe.avancement(i as f32 / 1000.0);
                assert!((-1e-4..=1.0 + 1e-4).contains(&a), "avancement {a} hors bornes");
                assert!(a >= precedent - 1e-4, "la courbe recule");
                precedent = a;
            }
        }
    }

    #[test]
    fn il_tombe_en_accelerant_et_se_pose_en_ralentissant() {
        assert!(CHUTE.avancement(0.5) < 0.3, "la chute devrait accelerer");
        assert!(POSE.avancement(0.5) > 0.7, "la pose devrait ralentir");
    }

    #[test]
    fn la_seconde_bascule_en_130_puis_170_ms() {
        let mut carte = Carte::nouvelle(4, true);
        assert_eq!(carte.total(), 300.0);
        carte.basculer(5);
        assert!(matches!(carte.etat(), Etat::Chute(_)));
        carte.avancer(129.0);
        assert!(matches!(carte.etat(), Etat::Chute(_)));
        carte.avancer(2.0);
        assert!(matches!(carte.etat(), Etat::Pose(_)));
        carte.avancer(169.0);
        assert_eq!(carte.etat(), Etat::Repos);
        // Le piege de ARCHITECTURE.md §7 : a la fin, aucun volet ne subsiste,
        // sinon les deux couches composeraient leurs bords adoucis et l'encre
        // s'epaissirait.
        assert_eq!(carte.nouvelle, 5);
        assert!(!carte.en_mouvement());
    }

    #[test]
    fn les_angles_restent_dans_le_quart_de_tour() {
        let mut carte = Carte::nouvelle(0, false);
        assert_eq!(carte.total(), 360.0);
        carte.basculer(1);
        for _ in 0..400 {
            match carte.etat() {
                Etat::Chute(a) => assert!((-FRAC_PI_2 - 1e-4..=1e-4).contains(&a)),
                Etat::Pose(a) => assert!((-1e-4..=FRAC_PI_2 + 1e-4).contains(&a)),
                Etat::Repos => {}
            }
            carte.avancer(1.0);
        }
    }

    #[test]
    fn une_bascule_interrompue_ne_repart_pas_de_zero() {
        let mut carte = Carte::nouvelle(8, true);
        carte.basculer(9);
        carte.avancer(60.0);
        let Etat::Chute(avant) = carte.etat() else {
            panic!("elle devrait tomber");
        };
        // Le volet haut porte toujours le 8 : sa chute continue, seule la
        // valeur qui arrive change.
        carte.basculer(0);
        let Etat::Chute(apres) = carte.etat() else {
            panic!("elle devrait tomber encore");
        };
        assert_eq!(avant, apres, "la chute a saute");
        assert_eq!(carte.ancienne, 8);
        assert_eq!(carte.nouvelle, 0);
    }
}
