//! La disposition, entierement exprimee en F, le corps des chiffres.
//!
//! DESIGN.md §4 donne toutes les proportions ; ce fichier ne fait que les
//! traduire en pixels, en arrondissant la ou un demi-pixel se verrait.

// --- Le corps des chiffres -------------------------------------------------

/// F = min(0,225 x largeur, 0,42 x hauteur). Le plafond en hauteur protege les
/// ecrans 4:3 et 16:10 ; a 1920x1080 c'est la largeur qui commande, F = 432 px.
const F_PAR_LARGEUR: f32 = 0.225;
const F_PAR_HAUTEUR: f32 = 0.42;

// --- La carte --------------------------------------------------------------

/// Largeur de carte = l'avance tabulaire de Barlow Condensed 800, mesuree a
/// 0,5200 em avec la fonction OpenType `tnum` (ARCHITECTURE.md §5).
const CARTE_L: f32 = 0.52;
/// Hauteur de carte : l'encre des chiffres (0,726 F) plus sa respiration.
const CARTE_H: f32 = 0.84;
/// Le pli est a mi-hauteur de carte. Comme l'encre est centree dans la carte,
/// c'est aussi le milieu exact des chiffres.
const PLI: f32 = CARTE_H / 2.0;
/// Epaisseur du pli : 2 px a 1080p, 3 px a 1440p, 4 px en 4K.
const EPAISSEUR_PLI: f32 = 0.0045;

// --- L'encre des chiffres, mesuree dans le fichier de police ----------------

/// Hauteur de l'encre au-dessus de la ligne de base : le plus haut des dix
/// chiffres (le 8, a 714/1000 em) et non la hauteur de capitale nominale
/// (700/1000), sinon le pli manquerait le milieu visuel de quatre pixels.
const ENCRE_HAUT: f32 = 0.714;
/// Debordement sous la ligne de base : le 0 descend a -12/1000 em.
const ENCRE_BAS: f32 = -0.012;

// --- Les ecarts ------------------------------------------------------------

/// Entre les deux cartes d'un meme nombre.
const ECART_INTRA: f32 = 0.03;
/// Entre heures, minutes et secondes. Pas de deux-points : l'ecart suffit.
const ECART_INTER: f32 = 0.28;

// --- La perspective --------------------------------------------------------

/// Distance de l'oeil, posee sur CHAQUE carte et jamais partagee : 4 x la
/// hauteur de carte. Chaque volet tombe droit, comme une mecanique vue de face.
const PERSPECTIVE: f32 = 4.0 * CARTE_H;

// --- Les deux lignes de texte ----------------------------------------------

/// Corps de la date, et de la legende.
const CORPS_DATE: f32 = 0.075;
const CORPS_LEGENDE: f32 = 0.04;
/// Hauteur de capitale de Barlow (sCapHeight = 700/1000 em), utilisee pour
/// centrer le bloc verticalement.
const CAPITALE_BARLOW: f32 = 0.70;
/// La date pose sa ligne de base a 0,28 F au-dessus du haut des chiffres.
const DATE_AU_DESSUS: f32 = 0.28;
/// La legende pose son haut de capitale a 0,30 F sous la ligne de base.
const LEGENDE_AU_DESSOUS: f32 = 0.30;
/// Interlettrages, en em de leur propre corps.
pub const INTERLETTRAGE_DATE: f32 = 0.02;
pub const INTERLETTRAGE_LEGENDE: f32 = 0.32;

/// Les six cartes : dizaines et unites d'heure, de minute, de seconde.
pub const CARTES: usize = 6;

pub struct Geometrie {
    pub largeur: u32,
    pub hauteur: u32,
    /// Corps des chiffres, en pixels.
    pub f: f32,
    /// Taille d'une carte, en pixels entiers.
    pub carte_l: u32,
    pub carte_h: u32,
    /// Ligne du pli, depuis le haut de la carte. Vaut exactement carte_h / 2.
    pub pli: u32,
    /// Hauteur du quad de la moitie haute : le pli mange ses dernieres lignes.
    pub haut_h: u32,
    /// Premiere ligne de la moitie basse, depuis le haut de la carte.
    pub bas_y: u32,
    pub bas_h: u32,
    /// Ligne de base du chiffre, depuis le haut de la carte.
    pub base: f32,
    /// Coin haut-gauche des cartes, a l'ecran.
    pub carte_y: f32,
    pub carte_x: [f32; CARTES],
    pub perspective: f32,
    /// Ligne de base de la date, et haut de capitale de la legende, a l'ecran.
    pub date_base: f32,
    pub legende_haut: f32,
    pub corps_date: f32,
    pub corps_legende: f32,
}

impl Geometrie {
    pub fn nouvelle(largeur: u32, hauteur: u32) -> Geometrie {
        let f = (F_PAR_LARGEUR * largeur as f32).min(F_PAR_HAUTEUR * hauteur as f32);

        // La hauteur de carte est arrondie a un nombre PAIR de pixels pour que
        // les deux moities se rejoignent sans decalage d'un pixel.
        let pli = (PLI * f).round().max(1.0) as u32;
        let carte_h = pli * 2;
        let carte_l = (CARTE_L * f).round().max(1.0) as u32;

        // Le pli est une bande centree sur la ligne du pli : on ne le peint
        // pas, on retire les lignes qu'il occupe aux deux moities. Le fond
        // traverse alors l'encre, exactement comme DESIGN.md §2 le demande.
        let epaisseur = (EPAISSEUR_PLI * f).round().max(1.0) as u32;
        let haut_h = (pli as f32 - epaisseur as f32 / 2.0).round().max(1.0) as u32;
        let bas_y = (haut_h + epaisseur).min(carte_h - 1);
        let bas_h = carte_h - bas_y;

        // L'encre est centree dans la carte ; sa moitie tombe donc sur le pli.
        let encre_h = ENCRE_HAUT - ENCRE_BAS;
        let marge = (CARTE_H - encre_h) / 2.0;
        let base = ((marge + ENCRE_HAUT) * f).round();

        let ecart_intra = (ECART_INTRA * f).round();
        let ecart_inter = (ECART_INTER * f).round();
        let rangee = 6.0 * carte_l as f32 + 3.0 * ecart_intra + 2.0 * ecart_inter;
        let gauche = ((largeur as f32 - rangee) / 2.0).round();

        let mut carte_x = [0.0f32; CARTES];
        let mut x = gauche;
        for (i, place) in carte_x.iter_mut().enumerate() {
            *place = x;
            x += carte_l as f32;
            // Apres une unite (i impair) on change de groupe, sauf a la fin.
            x += if i % 2 == 1 { ecart_inter } else { ecart_intra };
        }

        // Bloc date + chiffres + legende centre verticalement. Le haut du bloc
        // est le haut de capitale de la date, le bas celui de la legende.
        let encre_haut_carte = (marge * f).round(); // haut de l'encre dans la carte
        let au_dessus = DATE_AU_DESSUS * f + CAPITALE_BARLOW * CORPS_DATE * f - encre_haut_carte;
        let au_dessous =
            base - carte_h as f32 + LEGENDE_AU_DESSOUS * f + CAPITALE_BARLOW * CORPS_LEGENDE * f;
        let bloc = au_dessus + carte_h as f32 + au_dessous;
        let carte_y = ((hauteur as f32 - bloc) / 2.0 + au_dessus).round();

        Geometrie {
            largeur,
            hauteur,
            f,
            carte_l,
            carte_h,
            pli,
            haut_h,
            bas_y,
            bas_h,
            base,
            carte_y,
            carte_x,
            perspective: PERSPECTIVE * f,
            date_base: carte_y + encre_haut_carte - DATE_AU_DESSUS * f,
            legende_haut: carte_y + base + LEGENDE_AU_DESSOUS * f,
            corps_date: CORPS_DATE * f,
            corps_legende: CORPS_LEGENDE * f,
        }
    }

    /// Largeur totale de la rangee, en pixels.
    pub fn rangee(&self) -> f32 {
        self.carte_x[CARTES - 1] + self.carte_l as f32 - self.carte_x[0]
    }
}
