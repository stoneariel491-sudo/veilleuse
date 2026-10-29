//! Le ressort a amortissement critique qui pilote le deploiement de l'ilot.
//!
//! DESIGN.md §6 : raideur 400, amortissement 40, masse 1. C'est exactement le
//! cas critique, puisque l'amortissement critique vaut 2·sqrt(k·m) = 2·sqrt(400)
//! = 40. Il se pose sans jamais depasser, et il n'y a pas de rebond a regler
//! parce qu'il n'y en a pas.
//!
//! Il est integre PAR SA SOLUTION EXACTE, pas par petits pas. Deux raisons, et
//! la seconde compte plus que la premiere :
//!
//! 1. un pas d'Euler avec un ressort de pulsation 20 rad/s est stable a 60 Hz
//!    mais deja mou ; a une trame perdue de 33 ms il se met a depasser, ce que
//!    DESIGN.md interdit ;
//! 2. la solution exacte ne depend que de la position et de la vitesse au debut
//!    du pas. C'est la definition meme de l'interruptibilite demandee : on
//!    change la cible, l'etat physique reste, et le mouvement continue depuis ou
//!    il en etait — sans reprise a zero, sans saut.

/// Pulsation propre : sqrt(raideur / masse) = sqrt(400) = 20 rad/s.
pub const PULSATION: f32 = 20.0;

/// Vitesse de pointe d'une course complete de `course` unites, en unites par
/// seconde.
///
/// Partant du repos, un ressort a amortissement critique suit
/// `x(t) = c·(1 − (1 + wt)·e^{−wt})` et `v(t) = c·w²·t·e^{−wt}` : la vitesse
/// culmine a `t = 1/w` et y vaut `c·w/e`. C'est l'etalon qui rend la vitesse
/// courante comparable d'un axe a l'autre et d'une echelle a l'autre.
pub fn vitesse_de_pointe(course: f32) -> f32 {
    course * PULSATION / std::f32::consts::E
}

/// La vitesse NORMALISEE qu'un deploiement complet aurait a l'instant ou il
/// passe par `p`. Elle ne sert qu'a la capture, qui fige un deploiement sans
/// faire tourner de ressort et n'a donc aucune vitesse a lire.
///
/// `p = 1 − (1 + u)·e^{−u}` ne s'inverse pas en forme fermee ; vingt pas de
/// Newton sur une fonction monotone suffisent tres largement.
pub fn vitesse_normalisee_a(p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }
    let mut u = 1.0f32;
    for _ in 0..20 {
        let e = (-u).exp();
        let f = 1.0 - (1.0 + u) * e - p;
        // d/du [1 − (1+u)e^{−u}] = u·e^{−u}
        let df = u * e;
        if df.abs() < 1e-9 {
            break;
        }
        u = (u - f / df).max(1e-6);
    }
    // v(u)/v_pic = u·e^{1−u}
    (u * (1.0 - u).exp()).clamp(0.0, 1.0)
}

/// Seuils d'arret, **en pixels**. En dessous, on se pose exactement sur la cible
/// et on cesse de demander des trames.
///
/// L'unite n'est pas un detail de presentation : ces seuils ont d'abord valu
/// 1·10⁻³ et 5·10⁻² a l'epoque ou un ressort unique portait un p normalise dans
/// [0, 1]. Le passage a deux ressorts EN PIXELS les a laisses tels quels, et ils
/// se sont mis a valoir un milliemme de pixel et cinq centiemes de pixel par
/// seconde — mille fois trop serres pour l'unite nouvelle. Le prix se mesure :
/// une ouverture du galet dessinait alors pendant 771 ms, dont les trois cents
/// dernieres sont une quarantaine de trames qui deplacent moins d'un pixel. Et
/// comme la decoupe de la fenetre est retiree tant que le ressort bouge, ces
/// 771 ms sont aussi la duree pendant laquelle l'ilot prend toute sa fenetre en
/// surface de souris.
///
/// Un dixieme de pixel de course restante ne peut plus rien changer a ce qui se
/// voit : la geometrie est arrondie au pixel entier avant d'atteindre le
/// nuanceur comme la region. Le seuil reste dix fois plus serre que le
/// demi-pixel qui deciderait vraiment de l'arrondi, ce qui garde de la marge
/// sans laisser trainer la queue du calcul.
///
/// Durees mesurees par simulation a 144 Hz, sur les deux vraies courses de
/// l'ilot a l'echelle 1,25 : **521 ms** pour la largeur (110 → 400 px) et
/// **486 ms** pour la hauteur (45 → 185 px). DESIGN.md §6 annonce « ≈ 330 ms » —
/// les deux sont vrais, et le second est celui de l'oeil : a 330 ms le ressort a
/// parcouru 99 % de sa course.
const REPOS_POSITION: f32 = 0.1;
/// Le seuil de vitesse se deduit de celui de position : il est immobile s'il ne
/// parcourra pas un dixieme de pixel a la trame suivante. 10 px/s font 0,07 px
/// par trame a 144 Hz. C'est LUI qui decidait de l'arret quand les deux seuils
/// etaient normalises ; en pixels, c'est la position qui decide, et le seuil de
/// vitesse ne fait plus que fermer la porte derriere elle.
const REPOS_VITESSE: f32 = 10.0;

/// Au-dela de ce nombre de temps caracteristiques, l'exponentielle vaut 4·10⁻¹⁸ :
/// le ressort est pose, et il n'y a plus rien a calculer. Le garde-fou sert a la
/// machine qui sort de veille avec un ecart de plusieurs minutes.
const HORIZON: f32 = 40.0;

#[derive(Clone, Copy, Debug)]
pub struct Ressort {
    pub position: f32,
    pub vitesse: f32,
    pub cible: f32,
}

impl Ressort {
    /// Un ressort pose, immobile, sur `valeur`.
    pub fn pose(valeur: f32) -> Ressort {
        Ressort {
            position: valeur,
            vitesse: 0.0,
            cible: valeur,
        }
    }

    /// Change la cible, et RIEN d'autre. La position et la vitesse sont l'etat
    /// physique : elles ne se touchent pas depuis l'exterieur.
    pub fn viser(&mut self, cible: f32) {
        self.cible = cible;
    }

    pub fn au_repos(&self) -> bool {
        (self.position - self.cible).abs() < REPOS_POSITION && self.vitesse.abs() < REPOS_VITESSE
    }

    /// Avance de `dt` SECONDES.
    ///
    /// Solution du cas critique, ecart d = x0 − cible et pulsation w :
    ///   x(t) = cible + (d + (v0 + w·d)·t)·e^(−w·t)
    ///   v(t) = ((v0 + w·d)·(1 − w·t) − w·d)·e^(−w·t)
    pub fn avancer(&mut self, dt: f32) {
        if self.au_repos() {
            self.position = self.cible;
            self.vitesse = 0.0;
            return;
        }
        // Une trame tres longue se pose, elle ne se tronque pas : ecreter dt
        // ferait AVANCER MOINS le ressort qu'il n'aurait du, et l'ilot
        // resterait entrouvert apres une sortie de veille.
        let dt = dt.max(0.0);
        if dt * PULSATION > HORIZON {
            self.position = self.cible;
            self.vitesse = 0.0;
            return;
        }
        let d = self.position - self.cible;
        let c = self.vitesse + PULSATION * d;
        let e = (-PULSATION * dt).exp();
        self.position = self.cible + (d + c * dt) * e;
        self.vitesse = (c * (1.0 - PULSATION * dt) - PULSATION * d) * e;
        if self.au_repos() {
            self.position = self.cible;
            self.vitesse = 0.0;
        }
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    /// Un pas de trame a 144 Hz, la cadence mesuree de cette machine.
    const PAS: f32 = 1.0 / 144.0;
    /// Les deux vraies courses de l'ilot, en pixels physiques a l'echelle 1,25 :
    /// la largeur va de 110 a 400, la hauteur de 45 a 185. Les essais sont
    /// ecrits DANS CETTE UNITE parce que les seuils de repos y sont : un essai
    /// de course unite les rendrait muets.
    const REPLIE_L: f32 = 110.0;
    const DEPLOYE_L: f32 = 400.0;
    const REPLIE_H: f32 = 45.0;
    const DEPLOYE_H: f32 = 185.0;

    #[test]
    fn il_se_pose_sans_jamais_depasser() {
        // DESIGN.md §5 et §8 : aucun rebond, aucun depassement.
        let mut r = Ressort::pose(REPLIE_L);
        r.viser(DEPLOYE_L);
        let mut precedent = REPLIE_L;
        for _ in 0..200 {
            r.avancer(PAS);
            assert!(
                r.position <= DEPLOYE_L + 1e-3,
                "il a depasse : {}",
                r.position
            );
            assert!(r.position >= precedent - 1e-3, "il est reparti en arriere");
            precedent = r.position;
        }
        assert!(r.au_repos());
    }

    #[test]
    fn il_a_fini_sa_course_pour_l_oeil_a_330_ms() {
        // DESIGN.md §6 annonce « ≈ 330 ms ». Ce n'est pas l'instant ou le
        // calcul s'arrete, c'est celui ou le mouvement est fini pour l'oeil.
        let course = DEPLOYE_L - REPLIE_L;
        let mut r = Ressort::pose(REPLIE_L);
        r.viser(DEPLOYE_L);
        for _ in 0..24 {
            r.avancer(PAS); // 167 ms : la moitie du temps annonce
        }
        let fait = (r.position - REPLIE_L) / course;
        assert!(fait < 0.95, "trop rapide a mi-course : {fait}");
        for _ in 0..24 {
            r.avancer(PAS); // 333 ms
        }
        let fait = (r.position - REPLIE_L) / course;
        // 99 % de la course : sur les 290 px de largeur, il reste moins de 3 px.
        assert!(fait > 0.98, "trop lent a 330 ms : {fait}");
        assert!(!r.au_repos(), "le dernier pour cent se dessine encore");
    }

    #[test]
    fn le_dessin_s_arrete_avant_six_cents_millisecondes() {
        // Le pendant du precedent, et le chiffre qui compte vraiment : tant que
        // le ressort bouge, `Ilot` retire la decoupe de la fenetre, donc l'ilot
        // prend toute sa fenetre en surface de souris. Cette borne est celle de
        // la gene, pas celle du calcul.
        //
        // Avec les seuils normalises d'avant — un milliemme de pixel — la
        // largeur mettait 771 ms.
        for (depart, cible, borne) in [
            (REPLIE_L, DEPLOYE_L, 540.0),
            (REPLIE_H, DEPLOYE_H, 500.0),
            (DEPLOYE_L, REPLIE_L, 540.0),
            (DEPLOYE_H, REPLIE_H, 500.0),
        ] {
            let mut r = Ressort::pose(depart);
            r.viser(cible);
            let mut trames = 0;
            while !r.au_repos() && trames < 5000 {
                r.avancer(PAS);
                trames += 1;
            }
            let ms = trames as f32 * PAS * 1000.0;
            assert!(
                ms < borne,
                "{depart} -> {cible} dessine encore apres {ms:.0} ms"
            );
            assert_eq!(r.position, cible, "il doit finir exactement sur la cible");
        }
    }

    #[test]
    fn une_interruption_repart_de_la_position_et_de_la_vitesse_courantes() {
        // Le coeur de DESIGN.md §6 : « l'etat physique est la verite, pas la
        // cible ». Un clic en plein deploiement ne fait que changer la cible.
        let mut r = Ressort::pose(REPLIE_H);
        r.viser(DEPLOYE_H);
        for _ in 0..12 {
            r.avancer(PAS);
        }
        let (p, v) = (r.position, r.vitesse);
        assert!(
            p > REPLIE_H + 14.0 && p < DEPLOYE_H - 14.0,
            "l'essai n'a de sens qu'en plein vol : {p}"
        );
        assert!(v > 0.0, "il devrait encore monter");

        r.viser(REPLIE_H);
        assert_eq!(r.position, p, "la position a saute au changement de cible");
        assert_eq!(r.vitesse, v, "la vitesse a ete remise a zero");

        // Il monte encore un peu — c'est l'inertie, et c'est voulu — puis
        // redescend, sans jamais retomber sous le galet replie.
        let mut sommet: f32 = p;
        for _ in 0..200 {
            r.avancer(PAS);
            sommet = sommet.max(r.position);
            assert!(r.position >= REPLIE_H - 0.1, "il est passe sous le replie");
        }
        assert!(sommet > p, "l'elan acquis devrait se voir");
        assert!(sommet < DEPLOYE_H, "il ne doit pas finir de se deployer");
        assert!(r.au_repos() && r.position == REPLIE_H);
    }

    #[test]
    fn une_trame_perdue_ne_le_fait_pas_depasser() {
        // Le vrai piege d'une integration par petits pas : un pas de 33 ms.
        for pas in [PAS, 1.0 / 60.0, 1.0 / 30.0, 0.1] {
            let mut r = Ressort::pose(REPLIE_L);
            r.viser(DEPLOYE_L);
            for _ in 0..100 {
                r.avancer(pas);
                assert!(
                    r.position <= DEPLOYE_L + 1e-3,
                    "depassement avec un pas de {pas} s : {}",
                    r.position
                );
            }
            assert!(r.au_repos(), "pas pose avec un pas de {pas} s");
        }
    }

    #[test]
    fn un_pas_enorme_le_pose_au_lieu_de_l_envoyer_ailleurs() {
        // La machine sort de veille : dt vaut des minutes.
        let mut r = Ressort::pose(REPLIE_L);
        r.viser(DEPLOYE_L);
        r.avancer(600.0);
        assert!(
            r.position.is_finite() && r.au_repos(),
            "position = {}",
            r.position
        );
        assert!((r.position - DEPLOYE_L).abs() < 1e-3);
    }

    #[test]
    fn au_repos_il_ne_bouge_plus_du_tout() {
        let mut r = Ressort::pose(0.42);
        for _ in 0..100 {
            r.avancer(PAS);
        }
        assert_eq!(r.position, 0.42);
        assert_eq!(r.vitesse, 0.0);
    }
}
