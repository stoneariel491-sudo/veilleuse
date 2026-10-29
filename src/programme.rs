//! Le programme de la nuit : les blocs, lus dans un fichier editable a cote de
//! l'executable.
//!
//! Aucune duree n'est ecrite en dur dans le code. Celui qui lit change de livre,
//! de rythme et de chapitre ; il doit pouvoir changer ses blocs sans recompiler
//! quoi que ce soit. Le fichier s'appelle `veilleuse.json` et se trouve dans le
//! dossier de l'executable ; s'il manque, il est ecrit avec le programme par
//! defaut plutot que refuse — l'application doit demarrer, toujours.

use std::path::{Path, PathBuf};

use crate::json::{self, Valeur};

/// Un bloc de travail : on lit, puis on ferme le livre.
#[derive(Clone, Debug, PartialEq)]
pub struct Bloc {
    pub lecture_ms: f64,
    pub livre_ferme_ms: f64,
    /// « 214 à 236 ». Facultatif : un bloc sans pages annoncees se dit
    /// simplement « Bloc 3 ».
    pub pages: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Programme {
    pub nuit: u32,
    pub blocs: Vec<Bloc>,
}

/// 50 minutes de lecture, 10 minutes livre ferme, sept fois : la nuit 1.
/// Sept blocs d'une heure couvrent 22 h – 5 h et laissent deux heures de marge
/// avant 7 h, parce qu'une nuit qui commence en retard ne doit pas finir en
/// dette.
const LECTURE_DEFAUT_MIN: f64 = 50.0;
const LIVRE_FERME_DEFAUT_MIN: f64 = 10.0;
const BLOCS_DEFAUT: usize = 7;

/// Le nom du fichier, a cote de l'executable.
pub const NOM_FICHIER: &str = "veilleuse.json";

/// Le journal, a cote du fichier de blocs. Il n'existe que quand quelque chose
/// s'est mal passe.
pub const NOM_JOURNAL: &str = "veilleuse.log";

/// Ce qui s'est mal passe au chargement.
///
/// `fautif` separe les deux cas : un fichier qu'on n'a pas pu utiliser — la
/// seule chose dont l'utilisateur doit etre averti — et la simple creation du
/// fichier au premier lancement, qui n'est pas un incident.
pub struct Plainte {
    pub texte: String,
    pub fautif: bool,
}

const MINUTE_MS: f64 = 60_000.0;

impl Programme {
    pub fn defaut() -> Programme {
        Programme {
            nuit: 1,
            blocs: (0..BLOCS_DEFAUT)
                .map(|_| Bloc {
                    lecture_ms: LECTURE_DEFAUT_MIN * MINUTE_MS,
                    livre_ferme_ms: LIVRE_FERME_DEFAUT_MIN * MINUTE_MS,
                    pages: None,
                })
                .collect(),
        }
    }

    /// Le fichier tel qu'il est ecrit quand il manque. Il porte ses propres
    /// explications : c'est le seul endroit ou l'utilisateur reglera quelque
    /// chose, et il l'ouvrira sans documentation sous la main.
    pub fn json_par_defaut() -> String {
        let mut s = String::from(
            "{\n  \"_lisez_moi\": \"Les durees sont en minutes. \\\"pages\\\" est facultatif.\",\n  \"nuit\": 1,\n  \"blocs\": [\n",
        );
        for i in 0..BLOCS_DEFAUT {
            s.push_str(&format!(
                "    {{ \"lecture\": {LECTURE_DEFAUT_MIN}, \"livre_ferme\": {LIVRE_FERME_DEFAUT_MIN}, \"pages\": \"\" }}{}\n",
                if i + 1 < BLOCS_DEFAUT { "," } else { "" }
            ));
        }
        s.push_str("  ]\n}\n");
        s
    }

    /// Le chemin du fichier : a cote de l'executable, jamais dans le dossier
    /// courant. Lance depuis le menu Demarrer, le dossier courant est
    /// `C:\Windows\system32` et le fichier serait introuvable.
    pub fn chemin() -> PathBuf {
        match std::env::current_exe() {
            Ok(exe) => exe
                .parent()
                .map(|d| d.join(NOM_FICHIER))
                .unwrap_or_else(|| PathBuf::from(NOM_FICHIER)),
            Err(_) => PathBuf::from(NOM_FICHIER),
        }
    }

    /// Lit le fichier, ou l'ecrit s'il manque. Ne echoue jamais : le second
    /// membre porte ce qui s'est mal passe, a imprimer, pendant que
    /// l'application demarre quand meme avec le programme par defaut.
    pub fn charger() -> (Programme, Option<Plainte>) {
        let chemin = Programme::chemin();
        let fautif = |texte| Some(Plainte { texte, fautif: true });
        match std::fs::read_to_string(&chemin) {
            Ok(source) => match Programme::analyser(&source) {
                Ok(p) => (p, None),
                Err(e) => (
                    Programme::defaut(),
                    fautif(format!(
                        "{} : {e}
  -> le programme par defaut est utilise ; le fichier n'est pas touche.",
                        chemin.display()
                    )),
                ),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let ecrit = Programme::ecrire_defaut(&chemin);
                (
                    Programme::defaut(),
                    match ecrit {
                        Ok(()) => Some(Plainte {
                            texte: format!("{} a ete cree.", chemin.display()),
                            fautif: false,
                        }),
                        Err(e) => fautif(format!("{} n'a pas pu etre cree : {e}", chemin.display())),
                    },
                )
            }
            Err(e) => (
                Programme::defaut(),
                fautif(format!("{} illisible : {e}", chemin.display())),
            ),
        }
    }

    /// Le chemin du journal, a cote du fichier de blocs.
    pub fn chemin_journal() -> PathBuf {
        Programme::chemin().with_file_name(NOM_JOURNAL)
    }

    /// Ecrit la plainte a cote du fichier de blocs, ou efface le journal quand
    /// il n'y a rien a dire.
    ///
    /// En release le binaire est un sous-systeme « windows » : lance depuis
    /// l'Explorateur ou un raccourci — le seul usage reel — il n'a aucune sortie
    /// d'erreur attachee, et le message n'existerait pour personne. Le fichier
    /// de blocs etant le seul reglage que l'utilisateur touchera, il doit
    /// pouvoir savoir pourquoi son horloge est repartie sur le programme par
    /// defaut.
    ///
    /// Le journal est EFFACE quand le chargement se passe bien : un vieux
    /// journal qui traine ment autant qu'un message absent.
    pub fn consigner(plainte: Option<&Plainte>, quand: crate::temps::Instant) {
        let chemin = Programme::chemin_journal();
        match plainte {
            Some(p) if p.fautif => {
                let _ = std::fs::write(
                    &chemin,
                    format!(
                        "{} a {:02}:{:02}:{:02}
{}
",
                        quand.date_francaise(),
                        quand.heure,
                        quand.minute,
                        quand.seconde,
                        p.texte
                    ),
                );
            }
            _ => {
                let _ = std::fs::remove_file(&chemin);
            }
        }
    }

    fn ecrire_defaut(chemin: &Path) -> std::io::Result<()> {
        std::fs::write(chemin, Programme::json_par_defaut())
    }

    /// Traduit le JSON en programme. Chaque refus dit ce qui manque et ou.
    pub fn analyser(source: &str) -> Result<Programme, String> {
        // Une marque d'ordre des octets est retiree avant tout : sous Windows,
        // « Enregistrer sous » propose « UTF-8 avec BOM », et le fichier revient
        // alors avec U+FEFF en tete. Le refuser rendrait le seul reglage de
        // l'application illisible pour une raison que personne ne voit a
        // l'ecran — le caractere est invisible dans l'editeur.
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
        let racine = json::lire(source).map_err(|e| e.to_string())?;

        let nuit = match racine.champ("nuit") {
            Some(v) => v
                .nombre()
                .filter(|n| *n >= 0.0)
                .ok_or("« nuit » doit etre un nombre positif")? as u32,
            None => 1,
        };

        let liste = racine
            .champ("blocs")
            .and_then(Valeur::liste)
            .ok_or("« blocs » manque, ou n'est pas une liste")?;
        if liste.is_empty() {
            return Err("« blocs » est vide : il faut au moins un bloc".into());
        }

        let mut blocs = Vec::with_capacity(liste.len());
        for (i, brut) in liste.iter().enumerate() {
            let rang = i + 1;
            let duree = |nom: &str| -> Result<f64, String> {
                let minutes = brut
                    .champ(nom)
                    .and_then(Valeur::nombre)
                    .ok_or_else(|| format!("bloc {rang} : « {nom} » manque ou n'est pas un nombre"))?;
                if !(minutes > 0.0) {
                    return Err(format!("bloc {rang} : « {nom} » doit etre strictement positif"));
                }
                Ok(minutes * MINUTE_MS)
            };
            let pages = brut
                .champ("pages")
                .and_then(Valeur::texte)
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_string);
            blocs.push(Bloc {
                lecture_ms: duree("lecture")?,
                livre_ferme_ms: duree("livre_ferme")?,
                pages,
            });
        }

        Ok(Programme { nuit, blocs })
    }
}

impl Bloc {
    /// Le nombre de pages du bloc, tire de « 214 à 236 ». DESIGN.md §6 annonce
    /// « 22 pages » pour cet intervalle : c'est bien l'ECART qui est compte, pas
    /// le nombre de feuillets touches. On garde sa convention.
    pub fn pages_lues(&self) -> Option<u32> {
        let pages = self.pages.as_deref()?;
        let mut nombres = pages
            .split(|c: char| !c.is_ascii_digit())
            .filter(|m| !m.is_empty())
            .filter_map(|m| m.parse::<u32>().ok());
        let debut = nombres.next()?;
        let fin = nombres.next()?;
        fin.checked_sub(debut)
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    #[test]
    fn le_programme_par_defaut_est_celui_de_la_nuit_1() {
        let p = Programme::defaut();
        assert_eq!(p.nuit, 1);
        assert_eq!(p.blocs.len(), 7);
        assert_eq!(p.blocs[0].lecture_ms, 50.0 * 60_000.0);
        assert_eq!(p.blocs[0].livre_ferme_ms, 10.0 * 60_000.0);
    }

    #[test]
    fn le_fichier_par_defaut_se_relit_en_le_programme_par_defaut() {
        // Le piege serait d'ecrire un fichier que l'application ne sait pas
        // relire : au prochain lancement elle le refuserait sans rien dire de
        // plus qu'un message.
        let relu = Programme::analyser(&Programme::json_par_defaut())
            .expect("le fichier ecrit doit se relire");
        assert_eq!(relu, Programme::defaut());
    }

    #[test]
    fn une_marque_d_ordre_des_octets_ne_fait_pas_refuser_le_fichier() {
        // « UTF-8 avec BOM » est un choix a un clic dans l'Explorateur ; le
        // caractere est invisible, et le refus serait incomprehensible.
        let p = Programme::analyser("\u{feff}{\"blocs\":[{\"lecture\":50,\"livre_ferme\":10}]}")
            .expect("le BOM doit etre ignore");
        assert_eq!(p.blocs.len(), 1);
    }

    #[test]
    fn les_pages_et_les_durees_se_lisent() {
        let p = Programme::analyser(
            r#"{ "nuit": 3, "blocs": [ { "lecture": 25, "livre_ferme": 5, "pages": "214 à 236" } ] }"#,
        )
        .unwrap();
        assert_eq!(p.nuit, 3);
        assert_eq!(p.blocs[0].lecture_ms, 1_500_000.0);
        assert_eq!(p.blocs[0].pages.as_deref(), Some("214 à 236"));
        assert_eq!(p.blocs[0].pages_lues(), Some(22));
    }

    #[test]
    fn un_bloc_sans_pages_n_en_annonce_aucune() {
        let p = Programme::analyser(r#"{"blocs":[{"lecture":1,"livre_ferme":1,"pages":"  "}]}"#)
            .unwrap();
        assert_eq!(p.blocs[0].pages, None);
        assert_eq!(p.blocs[0].pages_lues(), None);
        assert_eq!(p.nuit, 1, "sans « nuit », c'est la premiere");
    }

    #[test]
    fn ce_qui_manque_est_dit_et_non_devine() {
        for (source, attendu) in [
            (r#"{"nuit":1}"#, "blocs"),
            (r#"{"blocs":[]}"#, "vide"),
            (r#"{"blocs":[{"livre_ferme":10}]}"#, "lecture"),
            (r#"{"blocs":[{"lecture":0,"livre_ferme":10}]}"#, "positif"),
            (r#"{"blocs":[{"lecture":-5,"livre_ferme":10}]}"#, "positif"),
            (r#"{"blocs":[{"lecture":"50","livre_ferme":10}]}"#, "lecture"),
        ] {
            let e = Programme::analyser(source).unwrap_err();
            assert!(
                e.contains(attendu),
                "« {source} » : le message « {e} » devrait parler de « {attendu} »"
            );
        }
    }

    #[test]
    fn le_rang_du_bloc_fautif_est_donne() {
        let e = Programme::analyser(
            r#"{"blocs":[{"lecture":50,"livre_ferme":10},{"lecture":50}]}"#,
        )
        .unwrap_err();
        assert!(e.contains("bloc 2"), "le rang devrait etre donne : {e}");
    }
}
