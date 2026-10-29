//! L'heure locale, et la date en francais.
//!
//! `GetLocalTime` donne tout ce dont l'horloge a besoin — heure locale, jour de
//! la semaine, millisecondes — sans aucune bibliotheque de dates.

#[derive(Clone, Copy, Debug)]
pub struct Instant {
    pub annee: u32,
    pub mois: u32,
    pub jour: u32,
    /// 0 = dimanche.
    pub jour_semaine: u32,
    pub heure: u32,
    pub minute: u32,
    pub seconde: u32,
    pub milli: u32,
}

const JOURS: [&str; 7] = [
    "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
];
// Accentues : cette ligne est AFFICHEE, ce n'est ni un commentaire ni un
// identifiant. Sans accents, la date de l'horloge serait fausse trois mois sur
// douze. Les polices embarquees portent le e aigu et le u circonflexe.
const MOIS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

impl Instant {
    /// « mardi 7 octobre 2025 » — en minuscules, fine et petite, parce qu'elle
    /// n'est lue qu'une fois par nuit.
    pub fn date_francaise(&self) -> String {
        let jour = JOURS[(self.jour_semaine as usize).min(6)];
        let mois = MOIS[(self.mois as usize).clamp(1, 12) - 1];
        // Le premier du mois se dit « 1er » en francais ; les autres non.
        if self.jour == 1 {
            format!("{jour} 1er {mois} {}", self.annee)
        } else {
            format!("{jour} {} {mois} {}", self.jour, self.annee)
        }
    }

    /// Les six chiffres de la rangee : dizaines et unites d'heure, de minute,
    /// de seconde. Format 24 h, jamais AM/PM.
    pub fn chiffres(&self) -> [u8; 6] {
        [
            (self.heure / 10) as u8,
            (self.heure % 10) as u8,
            (self.minute / 10) as u8,
            (self.minute % 10) as u8,
            (self.seconde / 10) as u8,
            (self.seconde % 10) as u8,
        ]
    }

    /// La seconde suivante, avec ses retenues. Le quantieme n'est pas avance :
    /// cette fonction ne sert qu'a fabriquer des captures, ou seule la rangee
    /// de chiffres compte.
    pub fn seconde_suivante(&self) -> Instant {
        let mut s = *self;
        s.seconde += 1;
        if s.seconde >= 60 {
            s.seconde = 0;
            s.minute += 1;
        }
        if s.minute >= 60 {
            s.minute = 0;
            s.heure += 1;
        }
        if s.heure >= 24 {
            s.heure = 0;
        }
        s
    }

    /// Millisecondes restant avant la prochaine frontiere de seconde.
    pub fn avant_la_seconde(&self) -> u64 {
        (1000 - self.milli.min(999)) as u64
    }
}

#[cfg(windows)]
pub fn maintenant() -> Instant {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    let mut t = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut t) };
    Instant {
        annee: t.wYear as u32,
        mois: t.wMonth as u32,
        jour: t.wDay as u32,
        jour_semaine: t.wDayOfWeek as u32,
        heure: t.wHour as u32,
        minute: t.wMinute as u32,
        seconde: t.wSecond as u32,
        milli: t.wMilliseconds as u32,
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    fn le(jour: u32, mois: u32, jour_semaine: u32) -> Instant {
        Instant {
            annee: 2025,
            mois,
            jour,
            jour_semaine,
            heure: 23,
            minute: 25,
            seconde: 44,
            milli: 0,
        }
    }

    #[test]
    fn les_trois_mois_a_accent_sont_accentues() {
        assert_eq!(le(7, 2, 5).date_francaise(), "vendredi 7 février 2025");
        assert_eq!(le(7, 8, 4).date_francaise(), "jeudi 7 août 2025");
        assert_eq!(le(7, 12, 0).date_francaise(), "dimanche 7 décembre 2025");
    }

    #[test]
    fn le_premier_du_mois_se_dit_1er() {
        assert_eq!(le(1, 10, 3).date_francaise(), "mercredi 1er octobre 2025");
    }
}

#[cfg(not(windows))]
pub fn maintenant() -> Instant {
    // L'horloge est ecrite pour Windows ; ailleurs elle affiche une heure fixe
    // plutot que de refuser de compiler.
    Instant {
        annee: 2025,
        mois: 10,
        jour: 7,
        jour_semaine: 2,
        heure: 23,
        minute: 25,
        seconde: 44,
        milli: 0,
    }
}
