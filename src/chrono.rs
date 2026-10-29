//! Le chrono d'etude : ou l'on en est dans la nuit, et ce qu'il reste.
//!
//! La machine tient en trois phases et un drapeau. Elle avance sur le temps
//! REEL mesure entre deux reveils, jamais sur un compte de trames : une machine
//! qui se met en veille pendant vingt minutes doit se reveiller au bon endroit
//! du programme, pas vingt minutes en retard.

use crate::programme::Programme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// On lit. Le temps restant est ce qui compte, il est blanc.
    Lecture,
    /// Le livre est ferme : on recupere de memoire, les yeux au repos.
    LivreFerme,
    /// Tous les blocs sont passes.
    Fini,
}

pub struct Chrono {
    programme: Programme,
    /// Rang du bloc en cours, a partir de 0.
    bloc: usize,
    phase: Phase,
    /// Millisecondes restant a la phase en cours.
    restant_ms: f64,
    en_pause: bool,
}

impl Chrono {
    pub fn nouveau(programme: Programme) -> Chrono {
        let restant_ms = programme.blocs[0].lecture_ms;
        Chrono {
            programme,
            bloc: 0,
            phase: Phase::Lecture,
            restant_ms,
            en_pause: false,
        }
    }

    // --- Le temps qui passe -------------------------------------------------

    /// Avance de `dt_ms` millisecondes reelles.
    ///
    /// Le debordement est REPORTE sur la phase suivante, et la boucle tourne
    /// tant qu'il reste du debordement : un ecart de deux heures — une mise en
    /// veille — traverse plusieurs blocs d'un coup et s'arrete au bon endroit,
    /// au lieu de coller la machine sur une phase terminee.
    pub fn avancer(&mut self, dt_ms: f64) {
        if self.en_pause || self.phase == Phase::Fini || dt_ms <= 0.0 {
            return;
        }
        self.restant_ms -= dt_ms;
        while self.restant_ms <= 0.0 && self.phase != Phase::Fini {
            let debordement = -self.restant_ms;
            self.enchainer();
            self.restant_ms -= debordement;
        }
        if self.phase == Phase::Fini {
            self.restant_ms = 0.0;
        }
    }

    /// La bascule automatique : lecture -> livre ferme -> bloc suivant.
    fn enchainer(&mut self) {
        match self.phase {
            Phase::Lecture => {
                self.phase = Phase::LivreFerme;
                self.restant_ms = self.programme.blocs[self.bloc].livre_ferme_ms;
            }
            Phase::LivreFerme => self.aller_au_bloc(self.bloc + 1),
            Phase::Fini => {}
        }
    }

    fn aller_au_bloc(&mut self, rang: usize) {
        if rang < self.programme.blocs.len() {
            self.bloc = rang;
            self.phase = Phase::Lecture;
            self.restant_ms = self.programme.blocs[rang].lecture_ms;
        } else {
            // Le rang reste sur le dernier bloc : « bloc 7 sur 7 » est ce qu'il
            // faut lire a la fin, pas « bloc 8 sur 7 ».
            self.phase = Phase::Fini;
            self.restant_ms = 0.0;
        }
    }

    // --- Les trois actions --------------------------------------------------

    /// Pause, ou reprise. L'action n'a pas de sens une fois la nuit finie.
    pub fn basculer_pause(&mut self) {
        if self.phase != Phase::Fini {
            self.en_pause = !self.en_pause;
        }
    }

    /// « Livre fermé » : on ferme le livre maintenant, avant la fin de la
    /// lecture. La pause de recuperation demarre entiere — elle ne se raccourcit
    /// pas parce qu'on l'a prise en avance, c'est tout son interet.
    pub fn fermer_le_livre(&mut self) {
        if self.phase == Phase::Lecture {
            self.phase = Phase::LivreFerme;
            self.restant_ms = self.programme.blocs[self.bloc].livre_ferme_ms;
            self.en_pause = false;
        }
    }

    /// « Suivant » : le bloc suivant, depuis n'importe quelle phase. Depuis la
    /// lecture, la recuperation est sautee — on la saute deliberement, c'est
    /// une decision de celui qui lit, pas un oubli.
    pub fn bloc_suivant(&mut self) {
        if self.phase != Phase::Fini {
            self.en_pause = false;
            self.aller_au_bloc(self.bloc + 1);
        }
    }

    /// Impose le temps restant. Ne sert qu'aux captures, comme `Carte::figer` :
    /// juger l'ilot ne doit pas demander d'attendre cinquante minutes.
    pub fn forcer_restant(&mut self, ms: f64) {
        self.restant_ms = ms.max(0.0);
    }

    // --- Ce qui se lit ------------------------------------------------------

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn en_pause(&self) -> bool {
        self.en_pause
    }

    /// La lumiere baisse des que le livre se ferme — et « pause » suit la meme
    /// regle (DESIGN.md §7). La nuit finie, elle reste basse : plus rien n'est
    /// demande a personne.
    pub fn lumiere_basse(&self) -> bool {
        self.en_pause || self.phase != Phase::Lecture
    }

    /// « LECTURE », « LIVRE FERMÉ », « PAUSE ». La pause l'emporte sur la phase :
    /// c'est l'etat dans lequel on vient de se mettre, donc celui qu'on cherche
    /// a l'ecran.
    pub fn phase_affichee(&self) -> &'static str {
        if self.phase == Phase::Fini {
            "NUIT TERMINÉE"
        } else if self.en_pause() {
            "PAUSE"
        } else if self.phase == Phase::Lecture {
            "LECTURE"
        } else {
            "LIVRE FERMÉ"
        }
    }

    /// « 41:27 ». Au-dela d'une heure — un bloc peut etre regle a ce qu'on veut
    /// dans le fichier — la forme passe a « 1:05:00 ».
    ///
    /// Le reste est ARRONDI VERS LE HAUT : sans cela, « 0:00 » s'afficherait
    /// pendant toute la derniere seconde alors qu'il en reste encore.
    pub fn temps_restant(&self) -> String {
        let secondes = (self.restant_ms.max(0.0) / 1000.0).ceil() as u64;
        let (h, m, s) = (secondes / 3600, (secondes / 60) % 60, secondes % 60);
        if h > 0 {
            format!("{h}:{m:02}:{s:02}")
        } else {
            format!("{m}:{s:02}")
        }
    }

    /// « Bloc 3 · pages 214 à 236 » pendant la lecture ; pendant la
    /// recuperation, DESIGN.md §6 demande le bilan : « Bloc 3 terminé ·
    /// 22 pages ».
    pub fn sous_ligne(&self) -> String {
        if self.phase == Phase::Fini {
            return format!("{} blocs faits", self.programme.blocs.len());
        }
        let rang = self.bloc + 1;
        let bloc = &self.programme.blocs[self.bloc];
        if self.phase == Phase::LivreFerme {
            return match bloc.pages_lues() {
                Some(n) => format!("Bloc {rang} terminé · {n} pages"),
                None => format!("Bloc {rang} terminé"),
            };
        }
        match &bloc.pages {
            Some(pages) => format!("Bloc {rang} · pages {pages}"),
            None => format!("Bloc {rang}"),
        }
    }

    /// Le mot de la premiere action, qui change de sens selon l'etat.
    pub fn mot_pause(&self) -> &'static str {
        if self.en_pause {
            "REPRENDRE"
        } else {
            "PAUSE"
        }
    }

    /// Une action eteinte n'a rien a faire dans l'etat courant. DESIGN.md §6 ne
    /// donne qu'une couleur au repos et une au survol ; une action sans effet se
    /// dit donc en n'etant pas proposee comme principale.
    pub fn action_disponible(&self, action: Action) -> bool {
        match action {
            Action::Pause => self.phase != Phase::Fini,
            Action::Suivant => self.phase != Phase::Fini,
            // La matiere se choisit a toute phase, la nuit terminee comprise
            // (DECISION-APRES-EPREUVE.md G, N4 : la ligne des actions ne porte
            // alors que VERRE / GALET).
            Action::Matiere => true,
        }
    }

    pub fn agir(&mut self, action: Action) {
        match action {
            Action::Pause => self.basculer_pause(),
            Action::Suivant => self.bloc_suivant(),
            // L'ilot l'execute lui-meme (`Ilot::basculer_matiere`) : rien ici.
            Action::Matiere => {}
        }
    }
}

/// Les deux actions de l'ilot, dans leur ordre d'affichage (DESIGN.md §6).
///
/// « Livre fermé » a ete une troisieme action et n'en est plus une : le §6 en
/// nomme deux, et la commande existe toujours cote chrono
/// (`fermer_le_livre`, atteignable par `--etat=livre`). Lui rendre une place a
/// l'ecran est une decision de produit, pas d'apparence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Pause,
    Suivant,
    /// L'action de la matiere, « VERRE » sur le galet opaque et « GALET » sur
    /// le verre (DESIGN.md §6 ; DECISION-APRES-EPREUVE.md D1). Elle n'est pas
    /// une action du chrono : l'ilot la nomme, la place a droite de la ligne
    /// des actions et l'execute lui-meme ; elle n'est pas dans `ACTIONS`.
    Matiere,
}

pub const ACTIONS: [Action; 2] = [Action::Pause, Action::Suivant];

impl Action {
    /// En CAPITALES, comme DESIGN.md §6 ligne 3 l'ecrit : « capitales,
    /// interlettrage 0,18 em ». L'interlettrage etait la, la casse non — et de
    /// la bas-de-casse espacee est le geste que cette typographie ne fait jamais.
    /// La petite capitale espacee est la signature du dessin : elle porte
    /// LECTURE, LIVRE FERME, la legende sous les chiffres.
    pub fn mot(self, chrono: &Chrono) -> &'static str {
        match self {
            Action::Pause => chrono.mot_pause(),
            Action::Suivant => "SUIVANT",
            // Nommee par la vitrine, selon la matiere visee.
            Action::Matiere => "",
        }
    }
}

#[cfg(test)]
mod essais {
    use super::*;
    use crate::programme::Bloc;

    /// Un programme de deux blocs courts, pour que les essais tiennent en
    /// millisecondes plutot qu'en heures.
    fn programme_court() -> Programme {
        Programme {
            nuit: 1,
            blocs: vec![
                Bloc {
                    lecture_ms: 1000.0,
                    livre_ferme_ms: 200.0,
                    pages: Some("214 à 236".into()),
                },
                Bloc {
                    lecture_ms: 500.0,
                    livre_ferme_ms: 100.0,
                    pages: None,
                },
            ],
        }
    }

    #[test]
    fn la_nuit_commence_sur_la_lecture_du_premier_bloc() {
        let c = Chrono::nouveau(programme_court());
        assert_eq!(c.phase(), Phase::Lecture);
        assert_eq!(c.temps_restant(), "0:01");
        assert_eq!(c.bloc, 0);
        assert_eq!(c.sous_ligne(), "Bloc 1 · pages 214 à 236");
        assert!(!c.lumiere_basse());
    }

    #[test]
    fn la_bascule_est_automatique_et_reporte_le_debordement() {
        let mut c = Chrono::nouveau(programme_court());
        c.avancer(1000.0);
        assert_eq!(c.phase(), Phase::LivreFerme);
        assert_eq!(c.sous_ligne(), "Bloc 1 terminé · 22 pages");
        assert!(c.lumiere_basse());
        // 150 ms de plus : il reste 50 ms sur les 200 de recuperation, et
        // surtout PAS 200 — le debordement ne se perd pas.
        c.avancer(150.0);
        assert!((c.restant_ms - 50.0).abs() < 1e-9, "restant = {}", c.restant_ms);
    }

    #[test]
    fn une_mise_en_veille_traverse_les_blocs_sans_rester_coincee() {
        // Le cas qui casserait une machine qui n'enchainerait qu'une phase par
        // reveil : un seul saut de plusieurs blocs.
        let mut c = Chrono::nouveau(programme_court());
        c.avancer(1000.0 + 200.0 + 500.0 + 50.0);
        assert_eq!(c.phase(), Phase::LivreFerme);
        assert_eq!(c.bloc, 1);
        assert!((c.restant_ms - 50.0).abs() < 1e-9);
    }

    #[test]
    fn la_nuit_se_termine_et_s_y_tient() {
        let mut c = Chrono::nouveau(programme_court());
        c.avancer(1_000_000.0);
        assert_eq!(c.phase(), Phase::Fini);
        assert_eq!(c.temps_restant(), "0:00");
        assert_eq!(c.bloc, 1, "le rang reste sur le dernier bloc, jamais un de plus");
        assert!(c.lumiere_basse());
        // Rien ne doit plus bouger.
        c.avancer(10_000.0);
        c.bloc_suivant();
        c.basculer_pause();
        c.fermer_le_livre();
        assert_eq!(c.phase(), Phase::Fini);
        assert!(!c.en_pause());
    }

    #[test]
    fn la_pause_arrete_vraiment_le_temps() {
        let mut c = Chrono::nouveau(programme_court());
        c.avancer(400.0);
        c.basculer_pause();
        assert_eq!(c.phase_affichee(), "PAUSE");
        assert!(c.lumiere_basse(), "la pause baisse la lumiere comme livre ferme");
        c.avancer(10_000.0);
        assert!((c.restant_ms - 600.0).abs() < 1e-9, "restant = {}", c.restant_ms);
        assert_eq!(c.phase(), Phase::Lecture, "la pause ne fait pas enchainer");
        c.basculer_pause();
        assert_eq!(c.mot_pause(), "PAUSE");
        c.avancer(100.0);
        assert!((c.restant_ms - 500.0).abs() < 1e-9);
    }

    #[test]
    fn fermer_le_livre_donne_la_recuperation_entiere() {
        let mut c = Chrono::nouveau(programme_court());
        c.avancer(100.0);
        c.fermer_le_livre();
        assert_eq!(c.phase(), Phase::LivreFerme);
        assert_eq!(c.restant_ms, 200.0, "la recuperation demarre entiere");
        // Deux fois de suite, la seconde ne relance pas la recuperation.
        c.avancer(50.0);
        c.fermer_le_livre();
        assert_eq!(c.restant_ms, 150.0);
    }

    #[test]
    fn suivant_saute_au_bloc_suivant_depuis_n_importe_ou() {
        let mut c = Chrono::nouveau(programme_court());
        c.bloc_suivant();
        assert_eq!(c.phase(), Phase::Lecture);
        assert_eq!(c.bloc, 1);
        assert_eq!(c.restant_ms, 500.0);
        assert_eq!(c.sous_ligne(), "Bloc 2", "ce bloc n'annonce pas de pages");
        // Depuis le dernier bloc, « suivant » termine la nuit.
        c.bloc_suivant();
        assert_eq!(c.phase(), Phase::Fini);
    }

    #[test]
    fn suivant_leve_la_pause() {
        let mut c = Chrono::nouveau(programme_court());
        c.basculer_pause();
        c.bloc_suivant();
        assert!(!c.en_pause(), "reprendre le travail leve la pause");
    }

    #[test]
    fn le_temps_restant_s_arrondit_vers_le_haut() {
        let mut c = Chrono::nouveau(Programme::defaut());
        assert_eq!(c.temps_restant(), "50:00");
        // Une milliseconde plus tard, il reste encore 49:59,999 : on lit 50:00,
        // et non 49:59, sinon la premiere seconde durerait un clin d'oeil.
        c.avancer(1.0);
        assert_eq!(c.temps_restant(), "50:00");
        c.avancer(999.0);
        assert_eq!(c.temps_restant(), "49:59");
        c.avancer(49.0 * 60_000.0 + 32.0 * 1000.0);
        assert_eq!(c.temps_restant(), "0:27");
    }

    #[test]
    fn un_bloc_de_plus_d_une_heure_se_dit_en_heures() {
        let p = Programme {
            nuit: 2,
            blocs: vec![Bloc {
                lecture_ms: 65.0 * 60_000.0,
                livre_ferme_ms: 60_000.0,
                pages: None,
            }],
        };
        assert_eq!(Chrono::nouveau(p).temps_restant(), "1:05:00");
    }
}
