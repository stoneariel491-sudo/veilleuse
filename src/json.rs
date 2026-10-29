//! Un lecteur JSON minimal, ecrit pour le seul fichier que l'application lit :
//! le programme des blocs, edite a la main a cote de l'executable.
//!
//! Pourquoi pas une bibliotheque : le fichier est lu une fois au demarrage et
//! tient en vingt lignes. Ce qui compte ici n'est pas la vitesse, c'est le
//! message d'erreur — on le veut en francais et avec un numero de ligne, parce
//! qu'il sera lu a trois heures du matin par celui qui vient d'editer le
//! fichier. Le projet ne prend pas non plus de bibliotheque de dates (§ Cargo) :
//! la meme regle s'applique ici.
//!
//! Ce qui est accepte est RFC 8259 sans extension : pas de commentaire, pas de
//! virgule finale, pas de nombre hexadecimal. Ce qui est refuse est signale,
//! jamais ignore en silence.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Valeur {
    Nul,
    Booleen(bool),
    Nombre(f64),
    Texte(String),
    Liste(Vec<Valeur>),
    /// Un vecteur et non une table : l'ordre d'ecriture est conserve, et un
    /// objet de cinq champs se parcourt plus vite qu'il ne se hache.
    Objet(Vec<(String, Valeur)>),
}

impl Valeur {
    pub fn champ(&self, nom: &str) -> Option<&Valeur> {
        match self {
            Valeur::Objet(champs) => champs.iter().find(|(c, _)| c == nom).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn nombre(&self) -> Option<f64> {
        match self {
            Valeur::Nombre(n) => Some(*n),
            _ => None,
        }
    }

    pub fn texte(&self) -> Option<&str> {
        match self {
            Valeur::Texte(t) => Some(t),
            _ => None,
        }
    }

    pub fn liste(&self) -> Option<&[Valeur]> {
        match self {
            Valeur::Liste(l) => Some(l),
            _ => None,
        }
    }
}

/// Une erreur de lecture, avec l'endroit exact ou elle a ete vue.
#[derive(Debug, PartialEq)]
pub struct Erreur {
    pub ligne: usize,
    pub colonne: usize,
    pub message: String,
}

impl fmt::Display for Erreur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ligne {}, colonne {} : {}", self.ligne, self.colonne, self.message)
    }
}

pub fn lire(source: &str) -> Result<Valeur, Erreur> {
    let mut l = Lecteur {
        octets: source.as_bytes(),
        i: 0,
    };
    l.blancs();
    let v = l.valeur()?;
    l.blancs();
    if l.i < l.octets.len() {
        return Err(l.erreur("du texte suit la fin de la valeur"));
    }
    Ok(v)
}

struct Lecteur<'a> {
    octets: &'a [u8],
    i: usize,
}

impl<'a> Lecteur<'a> {
    /// La ligne et la colonne se recomptent depuis le debut : l'erreur est
    /// produite au plus une fois par lecture, le cout n'existe pas.
    fn erreur(&self, message: &str) -> Erreur {
        let (mut ligne, mut colonne) = (1usize, 1usize);
        for &o in &self.octets[..self.i.min(self.octets.len())] {
            if o == b'\n' {
                ligne += 1;
                colonne = 1;
            } else {
                colonne += 1;
            }
        }
        Erreur {
            ligne,
            colonne,
            message: message.to_string(),
        }
    }

    fn regarder(&self) -> Option<u8> {
        self.octets.get(self.i).copied()
    }

    fn blancs(&mut self) {
        while matches!(self.regarder(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    /// Avale `mot` s'il est la ; sinon ne bouge pas.
    fn avaler(&mut self, mot: &[u8]) -> bool {
        if self.octets[self.i..].starts_with(mot) {
            self.i += mot.len();
            true
        } else {
            false
        }
    }

    fn valeur(&mut self) -> Result<Valeur, Erreur> {
        match self.regarder() {
            None => Err(self.erreur("une valeur est attendue, le fichier s'arrete")),
            Some(b'{') => self.objet(),
            Some(b'[') => self.liste(),
            Some(b'"') => Ok(Valeur::Texte(self.texte()?)),
            Some(b't') if self.avaler(b"true") => Ok(Valeur::Booleen(true)),
            Some(b'f') if self.avaler(b"false") => Ok(Valeur::Booleen(false)),
            Some(b'n') if self.avaler(b"null") => Ok(Valeur::Nul),
            Some(b'-' | b'0'..=b'9') => self.nombre(),
            Some(o) => Err(self.erreur(&format!(
                "une valeur est attendue, pas « {} »",
                o as char
            ))),
        }
    }

    fn objet(&mut self) -> Result<Valeur, Erreur> {
        self.i += 1; // l'accolade ouvrante
        let mut champs = Vec::new();
        self.blancs();
        if self.regarder() == Some(b'}') {
            self.i += 1;
            return Ok(Valeur::Objet(champs));
        }
        loop {
            self.blancs();
            if self.regarder() != Some(b'"') {
                return Err(self.erreur("un nom de champ entre guillemets est attendu"));
            }
            let nom = self.texte()?;
            self.blancs();
            if self.regarder() != Some(b':') {
                return Err(self.erreur("« : » est attendu apres le nom du champ"));
            }
            self.i += 1;
            self.blancs();
            let valeur = self.valeur()?;
            champs.push((nom, valeur));
            self.blancs();
            match self.regarder() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Valeur::Objet(champs));
                }
                _ => return Err(self.erreur("« , » ou « } » est attendu")),
            }
        }
    }

    fn liste(&mut self) -> Result<Valeur, Erreur> {
        self.i += 1; // le crochet ouvrant
        let mut elements = Vec::new();
        self.blancs();
        if self.regarder() == Some(b']') {
            self.i += 1;
            return Ok(Valeur::Liste(elements));
        }
        loop {
            self.blancs();
            elements.push(self.valeur()?);
            self.blancs();
            match self.regarder() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Valeur::Liste(elements));
                }
                _ => return Err(self.erreur("« , » ou « ] » est attendu")),
            }
        }
    }

    fn texte(&mut self) -> Result<String, Erreur> {
        self.i += 1; // le guillemet ouvrant
        let mut sortie = String::new();
        loop {
            let Some(o) = self.regarder() else {
                return Err(self.erreur("le guillemet fermant manque"));
            };
            match o {
                b'"' => {
                    self.i += 1;
                    return Ok(sortie);
                }
                b'\\' => {
                    self.i += 1;
                    let Some(e) = self.regarder() else {
                        return Err(self.erreur("une echappee inachevee"));
                    };
                    self.i += 1;
                    match e {
                        b'"' => sortie.push('"'),
                        b'\\' => sortie.push('\\'),
                        b'/' => sortie.push('/'),
                        b'b' => sortie.push('\u{8}'),
                        b'f' => sortie.push('\u{c}'),
                        b'n' => sortie.push('\n'),
                        b'r' => sortie.push('\r'),
                        b't' => sortie.push('\t'),
                        b'u' => sortie.push(self.echappee_unicode()?),
                        autre => {
                            self.i -= 1;
                            return Err(self.erreur(&format!(
                                "« \\{} » n'est pas une echappee connue",
                                autre as char
                            )));
                        }
                    }
                }
                // Les caracteres de commande doivent etre echappes (RFC 8259).
                0x00..=0x1F => return Err(self.erreur("un caractere de commande doit etre echappe")),
                _ => {
                    // On avance d'un caractere UTF-8 entier, jamais d'un octet :
                    // « à » ne doit pas se couper en deux.
                    let debut = self.i;
                    self.i += 1;
                    while matches!(self.regarder(), Some(0x80..=0xBF)) {
                        self.i += 1;
                    }
                    match std::str::from_utf8(&self.octets[debut..self.i]) {
                        Ok(morceau) => sortie.push_str(morceau),
                        Err(_) => {
                            self.i = debut;
                            return Err(self.erreur("le fichier n'est pas en UTF-8"));
                        }
                    }
                }
            }
        }
    }

    /// `\uXXXX`, et la paire de substitution qui porte les caracteres hors du
    /// plan de base. Sans elle, un emoji dans le fichier ferait echouer la
    /// lecture pour une raison incomprehensible.
    fn echappee_unicode(&mut self) -> Result<char, Erreur> {
        let haut = self.quatre_hexa()?;
        if (0xD800..0xDC00).contains(&haut) {
            if !self.avaler(b"\\u") {
                return Err(self.erreur("la moitie basse d'une paire de substitution manque"));
            }
            let bas = self.quatre_hexa()?;
            if !(0xDC00..0xE000).contains(&bas) {
                return Err(self.erreur("la moitie basse d'une paire de substitution est invalide"));
            }
            let point = 0x10000 + ((haut - 0xD800) << 10) + (bas - 0xDC00);
            return char::from_u32(point).ok_or_else(|| self.erreur("point de code invalide"));
        }
        char::from_u32(haut).ok_or_else(|| self.erreur("point de code invalide"))
    }

    fn quatre_hexa(&mut self) -> Result<u32, Erreur> {
        if self.i + 4 > self.octets.len() {
            return Err(self.erreur("quatre chiffres hexadecimaux sont attendus"));
        }
        let mut valeur = 0u32;
        for k in 0..4 {
            let o = self.octets[self.i + k];
            let chiffre = match o {
                b'0'..=b'9' => (o - b'0') as u32,
                b'a'..=b'f' => (o - b'a') as u32 + 10,
                b'A'..=b'F' => (o - b'A') as u32 + 10,
                _ => return Err(self.erreur("quatre chiffres hexadecimaux sont attendus")),
            };
            valeur = valeur * 16 + chiffre;
        }
        self.i += 4;
        Ok(valeur)
    }

    /// La grammaire des nombres de JSON, telle quelle : un signe optionnel, une
    /// partie entiere sans zero de tete, une partie decimale, un exposant.
    /// `f64::from_str` accepte bien plus que ca (« inf », « 0x1p3 ») ; on
    /// delimite donc nous-memes avant de lui passer la tranche.
    fn nombre(&mut self) -> Result<Valeur, Erreur> {
        let debut = self.i;
        if self.regarder() == Some(b'-') {
            self.i += 1;
        }
        match self.regarder() {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                while matches!(self.regarder(), Some(b'0'..=b'9')) {
                    self.i += 1;
                }
            }
            _ => return Err(self.erreur("un chiffre est attendu")),
        }
        if self.regarder() == Some(b'.') {
            self.i += 1;
            if !matches!(self.regarder(), Some(b'0'..=b'9')) {
                return Err(self.erreur("un chiffre est attendu apres le point decimal"));
            }
            while matches!(self.regarder(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        if matches!(self.regarder(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.regarder(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !matches!(self.regarder(), Some(b'0'..=b'9')) {
                return Err(self.erreur("un chiffre est attendu dans l'exposant"));
            }
            while matches!(self.regarder(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        // La tranche ne contient plus que des caracteres ASCII de la grammaire.
        let texte = std::str::from_utf8(&self.octets[debut..self.i]).unwrap();
        match texte.parse::<f64>() {
            Ok(n) if n.is_finite() => Ok(Valeur::Nombre(n)),
            _ => {
                self.i = debut;
                Err(self.erreur("ce nombre ne tient pas dans un flottant"))
            }
        }
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    fn lu(source: &str) -> Valeur {
        lire(source).expect("cette source devrait se lire")
    }

    #[test]
    fn les_valeurs_simples() {
        assert_eq!(lu("null"), Valeur::Nul);
        assert_eq!(lu(" true "), Valeur::Booleen(true));
        assert_eq!(lu("false"), Valeur::Booleen(false));
        assert_eq!(lu("0"), Valeur::Nombre(0.0));
        assert_eq!(lu("-12.5e2"), Valeur::Nombre(-1250.0));
        assert_eq!(lu("\"\""), Valeur::Texte(String::new()));
    }

    #[test]
    fn un_objet_garde_l_ordre_de_ses_champs() {
        let v = lu(r#"{ "b": 1, "a": 2 }"#);
        let Valeur::Objet(champs) = &v else { panic!("un objet etait attendu") };
        assert_eq!(champs[0].0, "b");
        assert_eq!(champs[1].0, "a");
        assert_eq!(v.champ("a").and_then(Valeur::nombre), Some(2.0));
        assert_eq!(v.champ("c"), None);
    }

    #[test]
    fn les_accents_et_les_echappees_survivent() {
        // Le fichier des blocs porte « pages 214 à 236 » : l'accent doit
        // traverser le lecteur sans se couper en deux octets.
        assert_eq!(lu(r#""214 à 236""#).texte(), Some("214 à 236"));
        assert_eq!(lu(r#""\u00e0""#).texte(), Some("à"));
        assert_eq!(lu(r#""a\nb\t\"c\"""#).texte(), Some("a\nb\t\"c\""));
        // Paire de substitution : U+1F319 (croissant de lune).
        assert_eq!(lu(r#""\ud83c\udf19""#).texte(), Some("🌙"));
    }

    #[test]
    fn une_liste_imbriquee_se_lit() {
        let v = lu(r#"{"blocs":[{"lecture":50},{"lecture":25.5}]}"#);
        let blocs = v.champ("blocs").and_then(Valeur::liste).unwrap();
        assert_eq!(blocs.len(), 2);
        assert_eq!(blocs[1].champ("lecture").and_then(Valeur::nombre), Some(25.5));
    }

    #[test]
    fn les_conteneurs_vides_se_lisent() {
        assert_eq!(lu("{}"), Valeur::Objet(vec![]));
        assert_eq!(lu("[]"), Valeur::Liste(vec![]));
    }

    #[test]
    fn l_erreur_donne_la_ligne_et_la_colonne() {
        // Trois lignes, la virgule finale est sur la troisieme.
        let e = lire("{\n  \"a\": 1,\n}").unwrap_err();
        assert_eq!(e.ligne, 3, "l'erreur devrait etre signalee ligne 3 : {e}");
    }

    #[test]
    fn ce_qui_doit_etre_refuse_l_est() {
        // Chacun de ces cas a deja fait perdre du temps a quelqu'un ; le
        // lecteur doit dire non, jamais deviner.
        for mauvais in [
            "",                      // vide
            "{",                     // accolade non fermee
            "[1, 2",                 // crochet non ferme
            "{\"a\" 1}",             // deux-points manquant
            "{a: 1}",                // nom sans guillemets
            "[1, 2,]",               // virgule finale
            "01",                    // zero de tete
            "1.",                    // point decimal sans chiffre
            ".5",                    // pas de partie entiere
            "1e",                    // exposant vide
            "\"a",                   // guillemet non ferme
            "\"\\x\"",               // echappee inconnue
            "// commentaire\n1",     // pas de commentaire en JSON
            "nan",                   // pas un litteral JSON
            "1 2",                   // du texte apres la valeur
        ] {
            assert!(
                lire(mauvais).is_err(),
                "« {mauvais} » aurait du etre refuse"
            );
        }
    }
}
