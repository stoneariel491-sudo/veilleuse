//! Les drapeaux de la ligne de commande.
//!
//! `--capture` est le seul outil de jugement du projet : il rend une image
//! hors-ecran, sans fenetre, et l'ecrit en PNG. `--phase` fige la bascule a un
//! instant precis pour la regarder image par image.

use std::path::PathBuf;

pub struct Options {
    /// Chemin du PNG a produire. Aucun chemin : on ouvre la fenetre.
    pub capture: Option<PathBuf>,
    /// Taille du rendu en capture, et de la fenetre hors plein ecran.
    pub taille: (u32, u32),
    /// Instant fige de la bascule, dans [0, 1] sur les 360 ms totales.
    pub phase: Option<f32>,
    /// La lumiere qui glisse sur la face du volet pendant la rotation.
    pub lumiere: bool,
    /// Heure imposee `HH:MM:SS`, pour que deux captures soient comparables.
    pub heure: Option<(u32, u32, u32)>,
    /// Fenetre a taille fixe plutot que plein ecran, pour regarder de pres.
    pub fenetre: bool,
    /// Imprime la precision du reveil et le nombre de trames par bascule.
    pub trace: bool,
    /// Texte de la legende. DESIGN.md §3 lui fait porter la phase de lecture ;
    /// sans ce drapeau, c'est le chrono qui la dicte.
    pub legende: String,
    /// La legende a-t-elle ete imposee a la main ? Si oui, le chrono ne la
    /// touche plus : c'est ce qui permet de capturer une legende choisie.
    pub legende_imposee: bool,

    // --- L'ilot ------------------------------------------------------------
    /// Chemin du PNG de l'ilot.
    pub capture_ilot: Option<PathBuf>,
    /// Deploiement fige, dans [0, 1] — la valeur que le ressort aurait.
    pub p: f32,
    /// Echelle de l'ecran, pour capturer l'ilot comme il sortirait d'un
    /// moniteur a 150 %.
    pub echelle: f32,
    /// Etat impose du chrono : « livre », « pause », « fini ».
    pub etat: Option<String>,
    /// Temps restant impose, « MM:SS », pour que deux captures soient
    /// comparables.
    pub restant: Option<f64>,
    /// Exposant de la superellipse impose a la main, pour la planche qui tranche
    /// celui des coins. Sans drapeau, l'exposant suit le deploiement.
    pub exposant: Option<f32>,
    /// Rayon du coin impose a la main, le second instrument de la meme planche :
    /// a exposant egal, une superellipse mange MOINS de coin qu'un cercle, et
    /// juger l'exposant sans pouvoir bouger le rayon reviendrait a juger deux
    /// choses a la fois.
    ///
    /// Il est en pixels PHYSIQUES, et cela se voit desormais : en capture
    /// hors-ecran l'echelle vaut 1 par defaut et la distinction ne se posait
    /// pas, mais la fenetre, elle, tourne a 1,25 sur cette machine. Le rayon
    /// livre — 48 logiques — s'y impose donc par `--rayon=60`.
    pub rayon: Option<f32>,

    // --- Le forcage du repli -------------------------------------------------
    // Sans lui, la branche de repli ne serait exercable qu'en changeant les
    // reglages de la machine. Ce sont ceux de l'auteur, pas les notres : c'est
    // exactement pourquoi cette branche n'avait jamais tourne.
    /// Force « le systeme demande moins de mouvement », quel que soit le reglage.
    pub mouvement_reduit: Option<bool>,
    /// Les quatre autres reglages du systeme (DESIGN-ILOT.md §8.1, pas 7),
    /// forces de la meme facon : `--transparence=coupee|active`,
    /// `--batterie=economiseur|secteur`, `--contraste=on|off`,
    /// `--taille-texte=<pour cent>`.
    pub transparence_coupee: Option<bool>,
    pub economiseur: Option<bool>,
    pub contraste: Option<bool>,
    pub taille_texte: Option<u32>,

    // --- La capture du bureau sous l'ilot ------------------------------------
    /// Dossier du relevé de la capture. L'application ouvre ses vraies fenetres,
    /// deroule un script borne (repos, temoin d'exclusion, deplacement, repli,
    /// reouverture), ecrit les images capturees et `releve.txt`, puis QUITTE
    /// d'elle-meme. C'est la preuve que la capture marche dans l'application
    /// reelle, et non dans le laboratoire.
    pub releve_bureau: Option<PathBuf>,
    /// L'epreuve du temoin DPI : le processus se declare inconscient du DPI
    /// avant la boucle, et le relevé doit le voir. Rien d'autre ne s'en sert.
    pub epreuve_dpi: bool,

    // --- Le verre, hors-ecran ------------------------------------------------
    /// Un PNG pose sous le galet de `--capture-ilot`, a la place du bureau que
    /// la fenetre capturerait. C'est ce qui permet de rendre le verre de
    /// l'application sur le fond meme que le banc a mesure.
    pub verre_fond: Option<PathBuf>,
    /// Position, dans ce PNG, du pixel (0, 0) de la surface. Par defaut, le
    /// galet tombe a la marge du PNG, comme au banc.
    pub verre_decalage: Option<(i32, i32)>,
    /// Le galet sans son contenu : le banc ne peint aucun texte.
    pub sans_contenu: bool,
    /// L'epreuve du temoin de la boucle, dans le relevé : le verre televerse
    /// meme une copie identique a son fond.
    pub epreuve_boucle: bool,
    /// L'epreuve du temoin du repli, dans le relevé : le verre garde son fond
    /// quand la capture est perdue.
    pub epreuve_repli: bool,
    /// Nom (ou morceau de nom) de l'adaptateur de la capture hors-ecran.
    pub adaptateur: Option<String>,
    /// L'epreuve du temoin de la sonde : le fil de la sonde du bureau n'est
    /// pas lance, et le relevé doit le voir.
    pub epreuve_sans_sonde: bool,

    // --- L'epreuve de l'ouverture (OUVERTURE-LE-BLANC-ETEINT.md) ------------
    /// `--epreuve-ouverture=ETAGE` : le lancement reel est rejoue jusqu'a
    /// l'etage nomme, puis il s'arrete net. Rien n'est jamais presente, le
    /// processus tient deux secondes et quitte. Ce qui separe deux etages
    /// voisins est ce que la mesure attribue.
    ///
    /// Les etages, du plus nu au plus complet :
    ///   - `fenetres` : les deux fenetres de l'application, le clic qui
    ///     traverse et le plein ecran. **Aucun appel a wgpu.**
    ///   - `surface`  : le meme, plus l'instance wgpu, les deux surfaces et
    ///     l'ouverture du peripherique. Aucune chaine d'echange.
    ///   - `chaine`   : le meme, plus la chaine d'echange de l'horloge,
    ///     configuree a la taille de la fenetre et jamais presentee.
    pub epreuve_ouverture: Option<String>,
    /// `--voie-d-ouverture=VOIE` : rejoue une des voies ECARTEES de
    /// l'arbitrage de `OUVERTURE-LE-BLANC-ETEINT.md`. Sans ce drapeau,
    /// l'application se comporte comme la decision l'a tranche : les deux
    /// fenetres naissent invisibles et ne paraissent qu'une fois peintes.
    ///   - `avant`            : le comportement d'avant la correction — les
    ///     fenetres paraissent des leur creation. C'est ainsi que se remesure
    ///     le defaut lui-meme, sur le binaire du jour ;
    ///   - `fond-noir`        : elles paraissent des leur creation, mais leur
    ///     surface de redirection est noircie par GDI (proposition B) ;
    ///   - `sans-redirection` : elles paraissent des leur creation et n'ont
    ///     plus de surface de redirection du tout,
    ///     `WS_EX_NOREDIRECTIONBITMAP` — mesure faite, l'horloge n'atteint
    ///     alors plus jamais l'ecran.
    pub voie_d_ouverture: Option<String>,

    // --- L'oeil sur l'ilot reel ---------------------------------------------
    /// Dossier de l'oeil : l'application ouvre ses vraies fenetres, pose l'ilot
    /// sur des fonds connus (la nuit de l'horloge, un blanc pur, un aplat
    /// colore), le capture tel que Windows le compose, replie, a mi-course et
    /// deploye, ecrit les images et `oeil.txt`, puis QUITTE d'elle-meme.
    pub oeil: Option<PathBuf>,
    /// Les epreuves des temoins de l'oeil, separees par des virgules :
    /// `noir`, `exclu`, `hors`, `fond`.
    pub epreuve_oeil: Option<String>,

    // --- La transformation, hors-ecran (GIVRE.md) --------------------------
    /// Croissance du givre et fonte, dans [0, 1], imposees a `--capture-ilot`.
    /// Sans ces drapeaux ni `--instant`, la capture est celle d'avant.
    pub givre: Option<f32>,
    pub fonte: Option<f32>,
    /// L'instant de la transformation, en millisecondes : `(g, f)` sont alors
    /// tires des deux ressorts de `matiere.rs`, comme la fenetre le fera.
    pub instant: Option<f32>,
    /// `--retour` : la transformation part du verre vers le galet opaque.
    pub retour: bool,
    /// `--interruption=MS` : la cible change a cet instant.
    pub interruption: Option<f32>,
    /// D'ou part la fonte, en pixels logiques de la fenetre.
    pub origine: Option<(f32, f32)>,
    /// Ecrit la texture des cristaux (`givre.rs`) en PNG, et quitte.
    pub capture_cristaux: Option<PathBuf>,
    /// Mesure le cout GPU d'une trame de la transformation, ecrit le journal, et
    /// quitte. Demande `--verre-fond`.
    pub mesure_givre: Option<PathBuf>,

    // --- La derivation de la force, hors-ecran ------------------------------
    /// Force du lisere (la reflexion du chant) imposee a `--capture-ilot`, pour
    /// le balayage qui la derive sur la couverture exacte de l'application,
    /// echelle par echelle (`DECISION-APRES-EPREUVE.md` A1 ; decision de
    /// l'auteur, `INTEGRATION-APRES-EPREUVE.md` pas 1 bis). C'est un
    /// instrument : sans lui, la force est celle de la table de `galet.rs` a
    /// l'echelle de la trame, et rien ne change.
    pub lisere: Option<f32>,
    /// L'epreuve du temoin de la table : la table est lue un quart d'echelle
    /// au-dessus de l'echelle de la trame, et le temoin « sans drapeau
    /// identique a `--lisere` egal a la force de la table » doit tomber.
    pub epreuve_echelle: bool,
    /// L'intensite de la lampe d'ecran imposee a `--capture-ilot`, en luminance
    /// lineaire (`--lampe=0` la coupe ; le double fait tomber les temoins du
    /// noir et du blanc). Un instrument : sans lui, `galet::lampe::intensite`,
    /// qui suit la hauteur du galet.
    pub lampe: Option<f32>,
    /// L'epreuve du temoin de la pousse des cristaux (m-g1) : la pousse se fait
    /// dans la trame de la premiere transformation, comme avant, et la premiere
    /// image doit alors lire g > 0,9 au lieu de g <= 0,05.
    pub epreuve_pousse: bool,
    /// L'epreuve du temoin des tiges tardives (m-g3) : la texture pousse sans la
    /// correction, et les rendus du givre redeviennent ceux d'avant.
    pub epreuve_tiges: bool,
    /// `--ordre=MS` : a `--capture-ilot`, la transformation est ordonnee MS ms
    /// apres la construction du rendu, pour relever ce que la premiere image
    /// attend de la pousse des cristaux (m-g1) a un instant choisi : 0 est le
    /// pire cas, 1000 le plus tot qu'un geste de l'auteur peut la demander (un
    /// appui de 400 ms, puis un clic). Sans lui, des que la capture est prete.
    pub ordre: Option<u64>,
    /// `--matiere=verre|opaque` : a `--capture-ilot`, la matiere visee, dont
    /// l'encre et le mot dependent (le gris du verre, VERRE / GALET). Sans lui,
    /// le verre si un fond est pose, le galet opaque sinon. Sur les vraies
    /// fenetres d'un mode scripte (l'oeil, les relevés), la matiere de depart,
    /// sans lire ni ecrire `matiere.txt`.
    pub matiere_visee: Option<bool>,
    /// `--survol=matiere|pause|suivant` : a `--capture-ilot`, l'action sous le
    /// pointeur (son encre vive, et la sous-ligne de la matiere).
    pub survol: Option<String>,
    /// `--ilot-deploye` : l'ilot nait deploye, pour la mesure du cout
    /// (DECISION-APRES-EPREUVE.md A3, pas 8). Un drapeau d'essai qui ne change
    /// rien au chemin normal : la cible est posee avant la premiere trame, et
    /// il n'arrive de nulle part, comme le replie.
    pub ilot_deploye: bool,
    /// `--releve-matiere=DOSSIER` : ouvre les vraies fenetres, attend trois
    /// secondes, ecrit les compteurs de la capture et la matiere dans
    /// `DOSSIER/releve-matiere.txt`, et quitte. Le temoin de A3 : en matiere
    /// opaque, aucune sonde, aucun tour, aucun televersement.
    pub releve_matiere: Option<PathBuf>,
    /// `--releve-presence=DOSSIER` : ouvre les vraies fenetres avec la vraie
    /// regle de presence (les autres modes scriptes la forcent), les laisse huit
    /// secondes a un script qui passe des fenetres devant l'horloge, ecrit ce
    /// que l'ilot a fait dans `DOSSIER/releve-presence.txt`, et quitte.
    pub releve_presence: Option<PathBuf>,
    /// La duree du relevé de la presence, en secondes (8 par defaut) : le banc
    /// des gestes en a besoin de plus.
    pub duree: Option<u64>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            capture: None,
            // La resolution de reference de DESIGN.md §3 : F = 432 px.
            taille: (1920, 1080),
            phase: None,
            lumiere: true,
            heure: None,
            fenetre: false,
            trace: false,
            legende: "LECTURE".to_string(),
            legende_imposee: false,
            capture_ilot: None,
            // Deploye : c'est l'etat qui porte le plus d'information, donc celui
            // qu'on regarde le plus souvent.
            p: 1.0,
            echelle: 1.0,
            etat: None,
            restant: None,
            exposant: None,
            rayon: None,
            mouvement_reduit: None,
            transparence_coupee: None,
            economiseur: None,
            contraste: None,
            taille_texte: None,
            releve_bureau: None,
            epreuve_dpi: false,
            verre_fond: None,
            verre_decalage: None,
            sans_contenu: false,
            epreuve_boucle: false,
            epreuve_repli: false,
            adaptateur: None,
            epreuve_sans_sonde: false,
            epreuve_ouverture: None,
            voie_d_ouverture: None,
            oeil: None,
            epreuve_oeil: None,
            givre: None,
            fonte: None,
            instant: None,
            retour: false,
            interruption: None,
            origine: None,
            capture_cristaux: None,
            mesure_givre: None,
            lisere: None,
            epreuve_echelle: false,
            lampe: None,
            epreuve_pousse: false,
            epreuve_tiges: false,
            ordre: None,
            matiere_visee: None,
            survol: None,
            ilot_deploye: false,
            releve_matiere: None,
            releve_presence: None,
            duree: None,
        }
    }
}

impl Options {
    /// Met le chrono dans l'etat demande par la ligne de commande. Sert aux
    /// captures : sans cela, juger « livre ferme » demanderait d'attendre
    /// cinquante minutes.
    pub fn appliquer_au_chrono(&self, chrono: &mut crate::chrono::Chrono) {
        if let Some(ms) = self.restant {
            chrono.forcer_restant(ms);
        }
        match self.etat.as_deref() {
            Some("livre") => chrono.fermer_le_livre(),
            Some("pause") => chrono.basculer_pause(),
            Some("fini") => {
                while chrono.phase() != crate::chrono::Phase::Fini {
                    chrono.bloc_suivant();
                }
            }
            Some(autre) => eprintln!("etat inconnu, ignore : {autre}"),
            None => {}
        }
        // Le temps impose vient APRES l'etat : fermer le livre remet le compte
        // a la duree entiere de la recuperation, ce qui ecraserait le reglage.
        if let Some(ms) = self.restant {
            chrono.forcer_restant(ms);
        }
    }

    /// Un script borne pilote-t-il les fenetres (relevé ou oeil) ? L'ilot ne
    /// memorise alors pas sa position et ne suit pas le focus.
    pub fn scripte(&self) -> bool {
        self.releve_bureau.is_some()
            || self.oeil.is_some()
            || self.releve_matiere.is_some()
            || self.releve_presence.is_some()
    }

    /// La regle de presence est-elle celle de l'application (l'ilot retire
    /// tant que l'horloge est devant), ou forcee a « present » pour un
    /// instrument qui a besoin de l'ilot sous les yeux (le relevé de la
    /// capture, l'oeil, le relevé de la matiere) ?
    pub fn presence_reelle(&self) -> bool {
        self.releve_presence.is_some() || !self.scripte()
    }

    /// L'etat `(g, f)` de la transformation demande a la capture, s'il y en a
    /// un : `--givre`/`--fonte` a la main, ou `--instant` par les ressorts.
    pub fn matiere(&self) -> Option<(f32, f32)> {
        if let Some(ms) = self.instant {
            let m = crate::matiere::Matiere::a_l_instant(
                ms / 1000.0,
                !self.retour,
                self.interruption.map(|i| i / 1000.0),
            );
            return Some(m.etat());
        }
        if self.givre.is_none() && self.fonte.is_none() {
            return None;
        }
        Some((
            self.givre.unwrap_or(0.0).clamp(0.0, 1.0),
            self.fonte.unwrap_or(0.0).clamp(0.0, 1.0),
        ))
    }

    pub fn lire() -> Options {
        let mut o = Options::default();
        for arg in std::env::args().skip(1) {
            let (cle, valeur) = match arg.split_once('=') {
                Some((c, v)) => (c, Some(v)),
                None => (arg.as_str(), None),
            };
            match cle {
                "--capture" => {
                    o.capture = Some(PathBuf::from(
                        valeur.unwrap_or("captures/veilleuse.png").to_string(),
                    ))
                }
                "--taille" => {
                    if let Some((l, h)) = valeur.and_then(|v| v.split_once('x')) {
                        if let (Ok(l), Ok(h)) = (l.parse(), h.parse()) {
                            o.taille = (l, h);
                        }
                    }
                }
                "--phase" => o.phase = valeur.and_then(|v| v.parse().ok()),
                "--lumiere" => o.lumiere = valeur.map(|v| v != "0").unwrap_or(true),
                "--heure" => o.heure = valeur.and_then(lire_heure),
                "--fenetre" => o.fenetre = true,
                "--trace" => o.trace = true,
                "--legende" => {
                    if let Some(v) = valeur {
                        o.legende = v.to_uppercase();
                        o.legende_imposee = true;
                    }
                }
                "--capture-ilot" => {
                    o.capture_ilot = Some(PathBuf::from(
                        valeur.unwrap_or("captures/ilot.png").to_string(),
                    ))
                }
                "--p" => o.p = valeur.and_then(|v| v.parse().ok()).unwrap_or(1.0),
                "--echelle" => o.echelle = valeur.and_then(|v| v.parse().ok()).unwrap_or(1.0),
                "--etat" => o.etat = valeur.map(str::to_string),
                "--restant" => o.restant = valeur.and_then(lire_duree),
                "--exposant" => o.exposant = valeur.and_then(|v| v.parse().ok()),
                "--rayon" => o.rayon = valeur.and_then(|v| v.parse().ok()),
                "--mouvement-reduit" => {
                    o.mouvement_reduit = Some(valeur.map(|v| v != "0").unwrap_or(true))
                }
                "--transparence" => o.transparence_coupee = Some(valeur != Some("active")),
                "--batterie" => o.economiseur = Some(valeur != Some("secteur")),
                "--contraste" => o.contraste = Some(valeur != Some("off")),
                "--taille-texte" => o.taille_texte = valeur.and_then(|v| v.parse().ok()),
                // L'ancien verre du systeme est retire : le verre taille le
                // remplace (`VERRE-TAILLE.md` §11). Le dire vaut mieux qu'un
                // « drapeau inconnu » muet sur la raison.
                "--verre" | "--verre-indisponible" => eprintln!(
                    "{cle} est retire : le fond translucide du systeme a cede la place au verre taille (VERRE-TAILLE.md §11)"
                ),
                "--releve-bureau" => {
                    o.releve_bureau = Some(PathBuf::from(
                        valeur.unwrap_or("captures/bureau").to_string(),
                    ))
                }
                "--epreuve-dpi" => o.epreuve_dpi = true,
                "--verre-fond" => o.verre_fond = valeur.map(PathBuf::from),
                "--verre-decalage" => {
                    o.verre_decalage = valeur.and_then(|v| {
                        let (x, y) = v.split_once(',')?;
                        Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
                    })
                }
                "--sans-contenu" => o.sans_contenu = true,
                "--adaptateur" => o.adaptateur = valeur.map(str::to_string),
                "--epreuve-boucle" => o.epreuve_boucle = true,
                "--epreuve-repli" => o.epreuve_repli = true,
                "--epreuve-sans-sonde" => o.epreuve_sans_sonde = true,
                "--epreuve-ouverture" => o.epreuve_ouverture = valeur.map(str::to_string),
                "--voie-d-ouverture" => o.voie_d_ouverture = valeur.map(str::to_string),
                "--oeil" => {
                    o.oeil = Some(PathBuf::from(valeur.unwrap_or("captures/oeil").to_string()))
                }
                "--epreuve-oeil" => o.epreuve_oeil = valeur.map(str::to_string),
                "--givre" => o.givre = valeur.and_then(|v| v.parse().ok()),
                "--fonte" => o.fonte = valeur.and_then(|v| v.parse().ok()),
                "--instant" => o.instant = valeur.and_then(|v| v.parse().ok()),
                "--retour" => o.retour = true,
                "--interruption" => o.interruption = valeur.and_then(|v| v.parse().ok()),
                "--origine" => {
                    o.origine = valeur.and_then(|v| {
                        let (x, y) = v.split_once(',')?;
                        Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
                    })
                }
                "--capture-cristaux" => {
                    o.capture_cristaux = Some(PathBuf::from(
                        valeur.unwrap_or("captures/givre/cristaux.png").to_string(),
                    ))
                }
                "--mesure-givre" => {
                    o.mesure_givre = Some(PathBuf::from(
                        valeur.unwrap_or("captures/givre/cout.txt").to_string(),
                    ))
                }
                "--lisere" => o.lisere = valeur.and_then(|v| v.parse().ok()),
                "--epreuve-echelle" => o.epreuve_echelle = true,
                "--lampe" => o.lampe = valeur.and_then(|v| v.parse().ok()),
                "--epreuve-pousse" => o.epreuve_pousse = true,
                "--epreuve-tiges" => o.epreuve_tiges = true,
                "--ordre" => o.ordre = valeur.and_then(|v| v.parse().ok()),
                "--matiere" => o.matiere_visee = valeur.map(|v| v == "verre"),
                "--survol" => o.survol = valeur.map(str::to_string),
                "--ilot-deploye" => o.ilot_deploye = true,
                "--releve-matiere" => {
                    o.releve_matiere = Some(PathBuf::from(
                        valeur.unwrap_or("captures/integration-apres-epreuve/pas4"),
                    ))
                }
                "--releve-presence" => {
                    o.releve_presence = Some(PathBuf::from(
                        valeur.unwrap_or("captures/integration-apres-epreuve/pas5"),
                    ))
                }
                "--duree" => o.duree = valeur.and_then(|v| v.parse().ok()),
                autre => eprintln!("drapeau inconnu, ignore : {autre}"),
            }
        }
        o
    }
}

/// « MM:SS » ou « H:MM:SS », en millisecondes.
fn lire_duree(v: &str) -> Option<f64> {
    let mut total = 0.0;
    for morceau in v.split(':') {
        total = total * 60.0 + morceau.parse::<f64>().ok()?;
    }
    Some(total * 1000.0)
}

fn lire_heure(v: &str) -> Option<(u32, u32, u32)> {
    let mut p = v.split(':');
    let h = p.next()?.parse().ok()?;
    let m = p.next()?.parse().ok()?;
    let s = p.next().unwrap_or("0").parse().ok()?;
    Some((h, m, s))
}
