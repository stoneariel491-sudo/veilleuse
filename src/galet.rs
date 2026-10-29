//! Le pipeline du galet : un seul quad, et deux matieres.
//!
//! - **Le galet noir**, un uniforme et aucune texture, tel qu'il etait. C'est le
//!   repli : il peint tant qu'aucun bureau capture n'est televerse, et des que
//!   la capture est perdue.
//! - **Le verre taille** de `DECISION.md` (direction ARBITREE) : deux couches du
//!   bureau capture sous le galet, la nette et la floutee, et le nuanceur du
//!   banc porte dans `galet.wgsl`.

use wgpu::util::DeviceExt;

use crate::gpu::Gpu;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniformes {
    pub taille: [f32; 2],
    pub centre: [f32; 2],
    pub demi: [f32; 2],
    pub rayon: f32,
    /// Exposant de la superellipse du coin. 2 redonne l'arc de cercle.
    pub n: f32,
    pub alpha_par_pixel: f32,
    /// Largeur, en pixels, du ruban de transition du bord LE PLUS VIF : c'est
    /// elle que la decoupe GDI suit. 1 au repos. Le nuanceur, lui, calcule le
    /// ruban de chaque bord (M3, `vitesse`).
    pub adoucissement: f32,
    /// L'opacite du fondu d'apparition ou de retrait, 1 au repos.
    pub opacite: f32,
    pub bourrage: f32,
    /// M3 (DECISION-APRES-EPREUVE.md) : la vitesse d'un bord lateral (la moitie
    /// de celle de la largeur) et celle du bord bas (celle de la hauteur), en
    /// pixels par seconde ; le bord haut ne bouge pas. Le ruban d'un bord suit
    /// sa vitesse NORMALE : `1 + ruban_max × min(1, |v·n| / v_pointe)`.
    pub vitesse: [f32; 2],
    /// La vitesse de pointe d'une course complete de la hauteur (l'etalon).
    pub v_pointe: f32,
    /// `3 × echelle − 1` : le ruban monte a 3 px logiques au plus.
    pub ruban_max: f32,
    /// Le theme de contraste du systeme (DESIGN-ILOT.md §8.1, pas 7), en
    /// lineaire : le fond du galet (`Window`) et 1 si le theme est actif ; la
    /// bordure (`WindowText`) et sa largeur en pixels ; la surbrillance
    /// (`Highlight`) et 1 si une action est survolee ; sa cible (x, y, l, h),
    /// en pixels de la fenetre. Tout a zero hors du theme.
    pub fond: [f32; 4],
    pub bord: [f32; 4],
    pub surbrillance: [f32; 4],
    pub zone: [f32; 4],
}

/// La silhouette du galet, rangee de pixels par rangee de pixels : pour chacune,
/// la PREMIERE colonne que la decoupe doit laisser passer, comptee depuis le
/// bord gauche du galet. La forme etant symetrique, la meme valeur sert a droite.
///
/// C'est le MIROIR EXACT de la superellipse de `galet.wgsl` — meme formule
/// `q.x^n + q.y^n = r^n`, meme exposant, meme rayon — et c'est la seule facon
/// qu'une decoupe GDI ne recoupe pas ce que le nuanceur vient de peindre. Une
/// region tiree de `CreateRoundRectRgn` decrit un arc de CERCLE : a rayon egal
/// la superellipse passe partout a l'exterieur du cercle (a 45° elle est a
/// 0,841·r du centre du coin contre 0,707·r), donc le cercle mord le coin de
/// pres de vingt pixels sur la premiere rangee et jette du meme coup le bord
/// adouci — mesure a l'ecran : la silhouette suivait le cercle, exposant ajuste
/// 1,91, et la transition valait 242 → 0 sans une seule valeur intermediaire.
///
/// La silhouette n'est PAS la courbe nue : c'est le premier pixel que le
/// nuanceur encre, ruban de transition compris. La difference n'est pas un
/// detail. Le ruban vaut un pixel mesure PERPENDICULAIREMENT au bord ; la ou le
/// coin est presque horizontal — la premiere rangee — un pixel de ruban
/// s'etale sur plus de six pixels d'horizontale. Une marge posee en x se
/// tromperait donc exactement la ou le coin est le plus plat, et la region
/// remordrait ce qu'elle vient de sauver. On resout plutot, rangee par rangee,
/// la meme inegalite que le nuanceur : `distance < ruban / 2`.
pub fn silhouette(l: f32, h: f32, rayon: f32, n: f32, ruban: f32) -> Vec<i32> {
    let rangees = h.round().max(0.0) as usize;
    let r = rayon.min(l / 2.0).min(h / 2.0).max(0.0);
    let seuil = ruban.max(1.0) * 0.5;
    let n = n.max(1.0);
    let mut colonnes = Vec::with_capacity(rangees);
    for i in 0..rangees {
        // Centre du pixel, mesure depuis le bord haut du galet.
        let ty = i as f32 + 0.5;
        // `q.y` du nuanceur : positif dans le coin, negatif le long du bord.
        let qy = if ty < r {
            r - ty
        } else if ty > h - r {
            r - (h - ty)
        } else {
            0.0
        };
        if qy <= 0.0 {
            colonnes.push(0);
            continue;
        }
        // La distance decroit quand on rentre : dichotomie sur la colonne, une
        // dizaine de pas au lieu du balayage complet de la rangee.
        let (mut bas, mut haut) = (0i32, r.ceil() as i32);
        while bas < haut {
            let milieu = (bas + haut) / 2;
            let qx = r - (milieu as f32 + 0.5);
            if distance(qx, qy, r, n) < seuil {
                haut = milieu;
            } else {
                bas = milieu + 1;
            }
        }
        colonnes.push(bas);
    }
    colonnes
}

/// La distance signee au galet de `galet.wgsl`, en pixels, au point (px, py)
/// d'un galet l x h pose en (0, 0). Negative dedans ; la couverture est pleine
/// a -0,5 et au-dela. Le relevé s'en sert pour lire l'interieur exact : une
/// silhouette retrecie de quelques pixels en x se trompe la ou le coin est
/// presque horizontal (voir `silhouette`).
pub fn distance_au_galet(px: f32, py: f32, l: f32, h: f32, rayon: f32, n: f32) -> f32 {
    let qx = (px - l / 2.0).abs() - l / 2.0 + rayon;
    let qy = (py - h / 2.0).abs() - h / 2.0 + rayon;
    if qx > 0.0 && qy > 0.0 {
        distance(qx, qy, rayon, n.max(1.0))
    } else {
        qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - rayon
    }
}

/// La distance signee du coin, telle que `galet.wgsl` la calcule — division par
/// la norme du gradient comprise, sans quoi le ruban s'epaissirait de 19 % a la
/// pointe du coin.
fn distance(qx: f32, qy: f32, r: f32, n: f32) -> f32 {
    if qx <= 0.0 {
        return qy - r;
    }
    let f = (qx.powf(n) + qy.powf(n)).powf(1.0 / n);
    let gradient = (qx.powf(n - 1.0).powi(2) + qy.powf(n - 1.0).powi(2)).sqrt()
        / f.powf(n - 1.0).max(1e-6);
    (f - r) / gradient.max(1e-6)
}

/// La direction ARBITREE de `DECISION.md`, parametre par parametre.
///
/// Toute valeur absente d'ici est hors de la decision. Les modeles eteints par
/// la decision (Beer-Lambert, dispersion, galbe, passe-bas de la couche nette)
/// ne sont pas portes dans `galet.wgsl`, et n'ont donc pas de constante.
///
/// **Les longueurs sont en pixels LOGIQUES**, comme toutes celles de
/// `DESIGN.md` : le banc les a mesurees sur l'ilot de 320 x 148, qui est la
/// taille logique, a l'echelle 1. L'application les multiplie par l'echelle de
/// l'ecran, exactement comme le rayon du coin (48 logiques, 60 physiques a
/// 1,25). Le conge et l'epaisseur suivent en plus la HAUTEUR du galet
/// (`conge_relatif`), ce qui les met deja en pixels physiques.
pub mod arbitree {
    /// Ecart-type du flou de la couche floutee, en linear.
    pub const SIGMA: f32 = 18.0;
    /// Teinte noire, posee sur les octets encodes.
    pub const TEINTE: f32 = 0.80;
    /// Vibrance de la couche floutee.
    pub const SATURATION: f32 = 1.35;
    /// Conge `b` et epaisseur `e`, a la hauteur de reference.
    pub const CONGE: f32 = 12.0;
    pub const EPAISSEUR: f32 = 12.0;
    /// La hauteur a laquelle `CONGE` et `EPAISSEUR` valent leur valeur : celle
    /// du galet deploye, en pixels logiques.
    pub const HAUTEUR_DU_CONGE: f32 = 148.0;
    /// N-BK7.
    pub const INDICE: f32 = 1.51680;
    /// Snell sur la couche nette : 1 allume, 0 eteint.
    pub const REFRACTION: f32 = 1.0;
    /// La couche nette ramenee au biseau.
    pub const CLARTE: f32 = 1.0;
    /// Prises sur l'empreinte du pixel.
    pub const PRISES: f32 = 8.0;
    /// La force de la reflexion, PAR ECHELLE : decision de l'auteur
    /// (`INTEGRATION-APRES-EPREUVE.md`, pas 1 bis).
    ///
    /// ~~0,25, derivee par l'essai 8.H~~ : c'etait la plus grande force qui
    /// tient la borne de `DESIGN.md` §8 sur le ruban du banc, a l'echelle 1.
    /// ~~0,19~~ : c'etait la plus grande force qui la tient aux quatre echelles
    /// 1, 1,25, 1,5 et 2 a la fois. Le pire pixel du bord monte avec
    /// l'echelle, parce que le conge la suit et que le premier pixel physique
    /// lit alors une tranche plus raide de Fresnel ; une force unique se
    /// derivait donc a l'echelle 2 et eteignait le lisere partout ailleurs.
    /// L'auteur a tranche : « Une par echelle ». A chaque echelle que Windows
    /// propose, la force est la plus grande du balayage au centieme dont le
    /// pire pixel de la lecture 3 (le verre seul, depremultiplie) tient sous
    /// 90,37 sur les huit fonds et les deux etats, lue par l'application
    /// elle-meme sur l'Intel UHD (`captures/integration-apres-epreuve/pas1/
    /// lisere-aux-echelles*.txt`).
    ///
    /// Entre deux echelles mesurees, la regle prudente : la force de la plus
    /// proche echelle mesuree AU-DESSUS ; au-dela de la derniere, la sienne ;
    /// sous la premiere, la sienne. L'instrument `--lisere` l'emporte toujours.
    ///
    /// Le pire pixel a la force retenue, par echelle : 88,47 / 89,82 / 88,51 /
    /// 88,51 / 88,51 / 89,20 / 89,88 / 90,25 / 89,88 ; la force du centieme
    /// au-dessus casse partout (90,62 a 92,35).
    pub const FORCES: [(f32, f32); 9] = [
        (1.0, 0.25),
        (1.25, 0.24),
        (1.5, 0.21),
        (1.75, 0.20),
        (2.0, 0.19),
        (2.25, 0.19),
        (2.5, 0.19),
        (3.0, 0.18),
        (3.5, 0.17),
    ];

    /// La force a cette echelle, par la table et la regle prudente.
    pub fn force(echelle: f32) -> f32 {
        for (e, f) in FORCES {
            if echelle <= e + 1e-4 {
                return f;
            }
        }
        FORCES[FORCES.len() - 1].1
    }
    /// La marge du banc : ce que la capture prend autour du galet, et la portee
    /// au-dela de laquelle la reflexion est ecretee. Le noyau du flou s'arrete a
    /// `ceil(3 sigma)` = 54 px ; le banc en capture 64.
    pub const MARGE: f32 = 64.0;
}

/// La lampe d'ecran (`DECISION-APRES-EPREUVE.md` E2 ; `INTEGRATION-APRES-
/// EPREUVE.md`, pas 2) : une lumiere blanche au-dessus de l'ecran, dont le
/// chant du verre porte le reflet a toute heure, et que le givre renvoie sans
/// teinte. L'auteur : « la lumiere n'est pas necessairement physique [...] ce
/// n'est pas une question de lumiere physique mais de ce qui est de l'ecran ».
pub mod lampe {
    /// Le niveau que le bord droit du verre DEPLOYE lit sur du blanc pur a la
    /// force de chaque echelle, au plus bas des neuf echelles : 86 / 87 / 85 /
    /// 86 / 86 / 87 / 88 / 88 / 88 de 1 a 3,5, donc 85, a 1,5 (`captures/
    /// integration-apres-epreuve/pas1/lisere-aux-echelles.txt`,
    /// `lisere-e*.txt`). Sur du noir, le bord haut du deploye lit ce niveau ;
    /// sur du blanc, l'ecran l'emporte a toutes les echelles, et rien ne change.
    pub const NIVEAU_DEPLOYE: f32 = 85.0;
    /// Le meme niveau pour le galet REPLIE : 67 / 70 / 69 / 70 / 70 / 71 / 73 /
    /// 73 / 74 de 1 a 3,5, donc 67, a l'echelle 1 (memes fichiers, lignes
    /// « blanc replie »). Le conge suit la hauteur du galet (`preparer_verre`),
    /// et sur le replie le premier pixel physique couvre une tranche bien plus
    /// large du conge : sa reflectance moyennee est plus basse, et une lampe a
    /// 85 y gagnait sur le blanc (62 a 219 pixels changes par echelle,
    /// `INTEGRATION-APRES-EPREUVE.md`, pas 2). La regle de l'arbitre, « sur du
    /// noir, le bord haut lit ce que le bord lit sur du blanc le jour », vaut
    /// donc par etat.
    pub const NIVEAU_REPLIE: f32 = 67.0;
    /// 1 : la lampe est un ciel, ponderee par `(1 - n_y) / 2` sur la normale
    /// sortante du bord. Une lampe uniforme dessinerait un anneau.
    pub const CIEL: f32 = 1.0;

    /// La luminance lineaire d'un niveau sRGB sur 255.
    fn lineaire(niveau: f32) -> f32 {
        let c = niveau / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    /// Le niveau sRGB sur 255 d'une luminance lineaire : pour le journal.
    pub fn niveau(l: f32) -> f32 {
        let c = if l <= 0.0031308 {
            l * 12.92
        } else {
            1.055 * l.powf(1.0 / 2.4) - 0.055
        };
        c * 255.0
    }

    /// L'intensite L de la trame : la luminance lineaire de `NIVEAU_REPLIE` au
    /// galet replie, de `NIVEAU_DEPLOYE` au galet deploye, et entre les deux
    /// lineaire en la hauteur du galet, par `suit` = hauteur / hauteur du
    /// deploye, le rapport meme qui fait suivre le conge (`preparer_verre`).
    /// Pourquoi la corde tient : la lecture du bord sur le blanc croit avec la
    /// largeur du conge et de moins en moins vite, la corde passe sous elle, et
    /// l'ecran l'emporte a toute hauteur ; c'est mesure a mi-course
    /// (`INTEGRATION-APRES-EPREUVE.md`, pas 2), pas seulement deduit.
    pub fn intensite(suit: f32) -> f32 {
        let suit_replie = crate::ilot::REPLIE_H / crate::ilot::DEPLOYE_H;
        let t = ((suit - suit_replie) / (1.0 - suit_replie)).clamp(0.0, 1.0);
        let (bas, haut) = (lineaire(NIVEAU_REPLIE), lineaire(NIVEAU_DEPLOYE));
        bas + t * (haut - bas)
    }

    /// La reflectance de Fresnel exacte, non polarisee, au point du conge de
    /// distance signee `d` : `bord_en` de `galet.wgsl`, mot pour mot, pour la
    /// seule reflectance.
    fn reflectance(d: f32, b: f32, n: f32) -> f32 {
        let sinp = (1.0 + d / b).clamp(0.0, 1.0);
        let cosp = (1.0 - sinp * sinp).max(0.0).sqrt();
        let sint = sinp / n;
        let cost = (1.0 - sint * sint).max(0.0).sqrt();
        let rs = (cosp - n * cost) / (cosp + n * cost).max(1e-6);
        let rp = (n * cosp - cost) / (n * cosp + cost).max(1e-6);
        0.5 * (rs * rs + rp * rp)
    }

    /// La reflectance moyennee sur l'empreinte du pixel du bord (de d = -1 a
    /// d = 0, `prises` prises), qui normalise le profil de la lampe. Elle ne
    /// depend que du conge, de l'indice et des prises, donc d'une trame : elle
    /// se calcule ici une fois, et non huit fois par pixel du conge dans le
    /// nuanceur. Mesure avant ce portage : la lampe coutait 11 % d'une passe
    /// du verre (`INTEGRATION-APRES-EPREUVE.md`, pas 2), pour 2 % predits par
    /// l'arbitre avec « une reflectance de plus par pixel ».
    pub fn reflectance_du_bord(b: f32, n: f32, prises: u32) -> f32 {
        let n = n.max(1.0);
        if b <= 0.0 {
            return 0.0;
        }
        let mut s = 0.0;
        for k in 0..prises.max(1) {
            let dd = -1.0 + (k as f32 + 0.5) / prises.max(1) as f32;
            s += reflectance(dd.min(0.0), b, n);
        }
        s / prises.max(1) as f32
    }
}

/// La marge capturee autour du galet, de chaque cote, en pixels PHYSIQUES.
///
/// Le pas 1 la posait a 64 px physiques, avec cette reserve : « si sigma suit
/// l'echelle de l'ecran, la marge doit la suivre ». Sigma la suit (voir
/// `arbitree`) : a 1,25 le noyau du flou atteint `ceil(3 x 22,5)` = 68 px, et
/// une marge restee a 64 aurait lu ses quatre derniers pixels dans le bord
/// recopie de la region. Arrondie au-dessus pour ne jamais passer sous le noyau.
pub fn marge(echelle: f32) -> i32 {
    (arbitree::MARGE * echelle).ceil() as i32
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ReglagesVerre {
    flou: [f32; 4],
    source: [f32; 4],
    textures: [f32; 4],
    matiere: [f32; 4],
    biseau: [f32; 4],
    /// La transformation (`GIVRE.md`) : x = croissance, y = fonte, zw = origine
    /// de la fonte en pixels de fenetre. Le tuyau du verre ne les lit pas.
    givre: [f32; 4],
    /// x = portee maximale depuis l'origine, y = largeur du front, z = pas de
    /// lecture du flou du givre, w = largeur du menisque en pixels.
    givre2: [f32; 4],
    /// xy = taille de la texture givree, zw = taille de la region givree.
    givre3: [f32; 4],
    /// La lampe d'ecran : x = intensite L en luminance lineaire, y = ciel.
    lampe: [f32; 4],
}

/// Les reglages de la transformation, tels que le nuanceur `givre` les lit.
pub mod transformation {
    /// Ecart-type du flou de la couche givree, en pixels logiques, sur la
    /// couche NETTE. Le verre floute a 18 ; un verre depoli diffuse bien plus
    /// (Walter et al. 2007, `GIVRE.md` §1.1). Le flou est tire de la couche
    /// deja floutee, donc l'ecart-type ajoute vaut sqrt(40² - 18²).
    pub const SIGMA: f32 = 40.0;
    /// La couche givree est calculee a cette fraction de la resolution.
    pub const PAS: f32 = 2.0;
    /// Deplacement du menisque sur la bande du front de fonte, en pixels
    /// logiques.
    pub const MENISQUE: f32 = 3.0;
}

/// Format des trois couches du verre : sRGB, pour que l'echantillonnage decode
/// et que l'ecriture reencode, donc que le flou se fasse en lineaire. C'est le
/// format du banc.
const FORMAT_COUCHES: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// La region du bureau televersee : son coin dans le bureau virtuel et sa
/// taille, qui peut etre plus petite que les textures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    pub x: i32,
    pub y: i32,
    pub l: u32,
    pub h: u32,
}

/// Les trois couches : la nette (le bureau tel quel), l'intermediaire du flou
/// horizontal, et la floutee. Gardees d'une trame a l'autre, et refaites
/// seulement quand une region depasse leur taille.
struct Couches {
    taille: (u32, u32),
    nette: wgpu::Texture,
    vue_inter: wgpu::TextureView,
    vue_floutee: wgpu::TextureView,
    groupe_h: wgpu::BindGroup,
    groupe_v: wgpu::BindGroup,
    groupe_verre: wgpu::BindGroup,
    // --- la transformation ---
    /// Les deux couches du givre, a demi-resolution : l'intermediaire du flou
    /// horizontal et la couche givree.
    taille_givree: (u32, u32),
    vue_inter_givre: wgpu::TextureView,
    vue_givree: wgpu::TextureView,
    groupe_givre_h: wgpu::BindGroup,
    groupe_givre_v: wgpu::BindGroup,
    /// Le groupe du tuyau `givre`, qui lie aussi la texture des cristaux : il
    /// n'existe que quand elle existe.
    groupe_givre: Option<wgpu::BindGroup>,
}

/// La texture des cristaux (`givre.rs`), televersee une fois.
struct CristauxGpu {
    vue: wgpu::TextureView,
    largeur_front: f32,
}

pub struct RenduGalet {
    pipeline: wgpu::RenderPipeline,
    /// L'image entiere d'un etat, pour le fondu enchaine de « moins de
    /// mouvement » (m5, pas 7) : la trame s'y peint en entier, galet et
    /// contenu, puis l'image se pose sur la fenetre a l'opacite du ressort par
    /// `tuyau_copie`, en melange premultiplie. C'est la seule facon qu'un
    /// fondu d'IMAGE soit exact : fondre chaque element a part additionne
    /// leurs alphas (mesure `pas7/reglages.txt`, premiere lecture).
    format: wgpu::TextureFormat,
    disposition_image: wgpu::BindGroupLayout,
    tuyau_copie: wgpu::RenderPipeline,
    image: Option<(u32, u32, wgpu::TextureView, wgpu::BindGroup)>,
    groupe: wgpu::BindGroup,
    tampon: wgpu::Buffer,
    // --- le verre ---
    tuyau_verre: wgpu::RenderPipeline,
    tuyau_flou: wgpu::RenderPipeline,
    disposition_verre: wgpu::BindGroupLayout,
    disposition_flou: wgpu::BindGroupLayout,
    echantillonneur: wgpu::Sampler,
    tampon_verre: wgpu::Buffer,
    tampon_h: wgpu::Buffer,
    tampon_v: wgpu::Buffer,
    couches: Option<Couches>,
    source: Option<Source>,
    // --- la transformation (GIVRE.md) : rien de ceci ne tourne tant que la
    // matiere est au repos, noire ou verre. ---
    tuyau_givre: wgpu::RenderPipeline,
    tuyau_flou_givre: wgpu::RenderPipeline,
    disposition_givre: wgpu::BindGroupLayout,
    tampon_givre_h: wgpu::Buffer,
    tampon_givre_v: wgpu::Buffer,
    cristaux: Option<CristauxGpu>,
    /// La couche givree est-elle celle du fond televerse ? Elle ne se refait
    /// que pour un fond neuf, et seulement pendant une transformation.
    givree_a_jour: bool,
    /// L'echelle du dernier fond televerse : le flou du givre en a besoin.
    echelle: f32,
    /// Force du lisere imposee par l'instrument `--lisere` : le balayage qui
    /// derive `arbitree::FORCES` sur la couverture exacte. `None` est le chemin
    /// normal, et la fenetre ne le pose jamais.
    pub lisere: Option<f32>,
    /// L'epreuve du temoin de la table (`--epreuve-echelle`) : la table est lue
    /// a une autre echelle que celle de la trame (la suivante de Windows, un
    /// quart au-dessus), et le temoin « sans drapeau identique a `--lisere`
    /// egal a la force de la table » doit tomber. Jamais pose par la fenetre.
    pub epreuve_echelle: bool,
    /// L'intensite de la lampe imposee par l'instrument `--lampe` (0 la coupe,
    /// le double fait tomber les temoins du noir et du blanc). `None` est le
    /// chemin normal : `lampe::intensite()`. Jamais pose par la fenetre.
    pub lampe: Option<f32>,
    /// La pousse des cristaux, lancee dans un fil des la construction (m-g1,
    /// `DECISION-APRES-EPREUVE.md` G) : 365 a 483 ms mesures par le juge, qui
    /// feraient lire g = 0,995 a la premiere image d'une transformation s'ils
    /// tombaient dans la trame du clic, c'est-a-dire le « switch brutal » que
    /// l'auteur refuse. `assurer_cristaux` la rejoint a la premiere
    /// transformation, d'ordinaire finie depuis longtemps.
    pousse: Option<std::thread::JoinHandle<(crate::givre::Cristaux, std::time::Duration)>>,
    /// L'instant de la construction, donc du depart du fil de la pousse.
    construit: std::time::Instant,
    /// Ce que la premiere transformation a attendu de la pousse (m-g1), releve
    /// par `assurer_cristaux` la premiere fois qu'il est appele.
    pousse_relevee: Option<PousseRelevee>,
    /// L'epreuve du temoin de la pousse (`--epreuve-pousse`) : la pousse se
    /// fait dans la trame, comme avant ; la premiere image de la premiere
    /// transformation doit alors lire g > 0,9. Jamais posee par la fenetre.
    pub epreuve_pousse: bool,
    /// L'epreuve du temoin des tiges tardives (`--epreuve-tiges`) : la texture
    /// pousse sans la correction m-g3, dans la trame. Jamais posee par la fenetre.
    pub epreuve_tiges: bool,
}

/// Le releve de la pousse des cristaux a la premiere transformation (m-g1,
/// `DECISION-APRES-EPREUVE.md` G) : la duree de la pousse elle-meme, l'instant
/// de l'ordre depuis la construction du rendu, et ce que la trame a attendu.
/// Dans le fil, l'attente est ce qui restait de la pousse a l'ordre, d'ordinaire
/// rien ; dans la trame (`--epreuve-pousse`, `--epreuve-tiges`), c'est la pousse
/// entiere.
#[derive(Clone, Copy, Debug)]
pub struct PousseRelevee {
    pub duree: std::time::Duration,
    pub ordre: std::time::Duration,
    pub attente: std::time::Duration,
    pub dans_le_fil: bool,
}

/// Ce que la transformation demande a la trame, en plus du fond.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transformation {
    /// Croissance et fonte, dans [0, 1].
    pub givre: f32,
    pub fonte: f32,
    /// Origine de la fonte, en pixels de fenetre.
    pub origine: (f32, f32),
    /// Le galet deploye en pixels logiques, pour faire pousser les cristaux :
    /// largeur, hauteur, rayon, exposant.
    pub galet: (f32, f32, f32, f32),
}

impl Transformation {
    /// La matiere est-elle a l'une de ses deux extremites ? Le galet opaque et
    /// le verre se peignent alors par leurs tuyaux d'avant, a l'identique.
    pub fn au_repos(&self) -> Option<bool> {
        if self.givre <= 0.0 && self.fonte <= 0.0 {
            Some(false)
        } else if self.givre >= 1.0 && self.fonte >= 1.0 {
            Some(true)
        } else {
            None
        }
    }
}

impl RenduGalet {
    pub fn nouveau(gpu: &Gpu, format: wgpu::TextureFormat) -> RenduGalet {
        let module = gpu
            .peripherique
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("galet"),
                source: wgpu::ShaderSource::Wgsl(include_str!("galet.wgsl").into()),
            });

        let tampon = gpu
            .peripherique
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("uniformes du galet"),
                contents: bytemuck::bytes_of(&Uniformes {
                    taille: [1.0, 1.0],
                    centre: [0.0, 0.0],
                    demi: [0.0, 0.0],
                    rayon: 0.0,
                    n: 2.0,
                    alpha_par_pixel: 1.0,
                    adoucissement: 1.0,
                    opacite: 1.0,
                    bourrage: 0.0,
                    vitesse: [0.0, 0.0],
                    v_pointe: 1.0,
                    ruban_max: 0.0,
                    fond: [0.0; 4],
                    bord: [0.0; 4],
                    surbrillance: [0.0; 4],
                    zone: [0.0; 4],
                }),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let disposition =
            gpu.peripherique
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("liaisons du galet"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let groupe = gpu
            .peripherique
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("liaisons du galet"),
                layout: &disposition,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tampon.as_entire_binding(),
                }],
            });

        let agencement = gpu
            .peripherique
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("agencement du galet"),
                bind_group_layouts: &[Some(&disposition)],
                immediate_size: 0,
            });

        let tuyau_noir = |melange: Option<wgpu::BlendState>| {
            gpu.peripherique
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("galet"),
                    layout: Some(&agencement),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            // Aucun melange : le galet ECRIT l'alpha de la
                            // fenetre, il ne le compose pas. C'est ce que DWM
                            // lira pour decider de ce qui traverse.
                            blend: melange,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                })
        };
        let pipeline = tuyau_noir(None);

        // --- Le verre -------------------------------------------------------
        //
        // Deux tuyaux de plus sur le MEME module : le flou, qui ecrit les
        // couches, et le verre, qui lit l'uniforme du galet (groupe 0) et ses
        // couches (groupe 1). Le tuyau noir ci-dessus ne change pas d'un octet :
        // c'est lui qui peint tant qu'aucun fond n'est la.
        let uniforme = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let echantillonnage = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let disposition_verre =
            gpu.peripherique
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("couches du verre"),
                    entries: &[uniforme, echantillonnage, texture(2), texture(3)],
                });
        let disposition_flou =
            gpu.peripherique
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("source du flou"),
                    entries: &[uniforme, echantillonnage, texture(3)],
                });
        // Le tuyau de la transformation lit deux couches de plus : la givree
        // et les cristaux. Sa disposition est a part, pour que celle du verre
        // reste ce qu'elle etait.
        let disposition_givre =
            gpu.peripherique
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("couches du givre"),
                    entries: &[uniforme, echantillonnage, texture(2), texture(3), texture(4), texture(5)],
                });
        let tuyau = |nom: &str,
                     entree: &str,
                     dispositions: &[Option<&wgpu::BindGroupLayout>],
                     cible: wgpu::TextureFormat,
                     melange: Option<wgpu::BlendState>| {
            let agencement =
                gpu.peripherique
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some(nom),
                        bind_group_layouts: dispositions,
                        immediate_size: 0,
                    });
            gpu.peripherique
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(nom),
                    layout: Some(&agencement),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some(entree),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: cible,
                            blend: melange,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                })
        };
        let tuyau_verre = tuyau(
            "verre du galet",
            "verre",
            &[Some(&disposition), Some(&disposition_verre)],
            format,
            None,
        );
        let tuyau_flou = tuyau("flou du verre", "flou", &[None, Some(&disposition_flou)], FORMAT_COUCHES, None);
        let tuyau_givre = tuyau(
            "givre du galet",
            "givre",
            &[Some(&disposition), Some(&disposition_givre)],
            format,
            None,
        );
        let tuyau_flou_givre =
            tuyau("flou du givre", "flou_givre", &[None, Some(&disposition_flou)], FORMAT_COUCHES, None);
        // L'image entiere posee a l'opacite (le fondu enchaine, m5) : un tuyau
        // qui lit une texture de la taille de la fenetre, pixel a pixel, et la
        // pose par-dessus la vue en melange premultiplie.
        let disposition_image =
            gpu.peripherique
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("image entiere"),
                    entries: &[texture(2)],
                });
        let tuyau_copie = tuyau(
            "image entiere, par-dessus",
            "copie",
            &[Some(&disposition), Some(&disposition_image)],
            format,
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );
        let echantillonneur = gpu.peripherique.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("couches du verre"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let reglages = |nom| {
            gpu.peripherique.create_buffer(&wgpu::BufferDescriptor {
                label: Some(nom),
                size: std::mem::size_of::<ReglagesVerre>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        RenduGalet {
            pipeline,
            format,
            disposition_image,
            tuyau_copie,
            image: None,
            groupe,
            tampon,
            tuyau_verre,
            tuyau_flou,
            disposition_verre,
            disposition_flou,
            echantillonneur,
            tampon_verre: reglages("reglages du verre"),
            tampon_h: reglages("flou horizontal"),
            tampon_v: reglages("flou vertical"),
            couches: None,
            source: None,
            tuyau_givre,
            tuyau_flou_givre,
            disposition_givre,
            tampon_givre_h: reglages("flou du givre, horizontal"),
            tampon_givre_v: reglages("flou du givre, vertical"),
            cristaux: None,
            givree_a_jour: false,
            echelle: 1.0,
            lisere: None,
            epreuve_echelle: false,
            lampe: None,
            // Le galet deploye, en pixels logiques : la texture des cristaux
            // est la meme a toutes les echelles, et une seule pousse suffit.
            // Le fil rend aussi la duree de sa pousse, pour le releve de m-g1.
            pousse: Some(std::thread::spawn(|| {
                let debut = std::time::Instant::now();
                let c = crate::givre::pousser(
                    crate::ilot::DEPLOYE_L,
                    crate::ilot::DEPLOYE_H,
                    crate::ilot::DEPLOYE_RAYON,
                    crate::ilot::DEPLOYE_N,
                    crate::givre::GRAINE,
                );
                (c, debut.elapsed())
            })),
            construit: std::time::Instant::now(),
            pousse_relevee: None,
            epreuve_pousse: false,
            epreuve_tiges: false,
        }
    }

    /// L'intensite de la lampe de cette trame : celle de l'instrument s'il est
    /// pose, sinon celle de la hauteur courante du galet (`lampe::intensite`).
    fn lampe_intensite(&self, suit: f32) -> f32 {
        self.lampe.unwrap_or_else(|| lampe::intensite(suit))
    }

    /// Ce que la premiere transformation a attendu de la pousse des cristaux
    /// (m-g1), `None` tant qu'aucune transformation n'a ete peinte.
    pub fn pousse_relevee(&self) -> Option<PousseRelevee> {
        self.pousse_relevee
    }

    /// L'age de ce rendu, depuis sa construction, donc depuis le depart du fil
    /// de la pousse.
    pub fn age(&self) -> std::time::Duration {
        self.construit.elapsed()
    }

    /// La force du lisere de cette trame : celle de l'instrument s'il est pose,
    /// sinon celle de la table a l'echelle de la trame en cours (la fenetre
    /// qui change d'ecran change d'echelle, et la force la suit).
    fn force(&self, echelle: f32) -> f32 {
        if let Some(f) = self.lisere {
            return f;
        }
        let lue = if self.epreuve_echelle { echelle + 0.25 } else { echelle };
        arbitree::force(lue)
    }

    /// L'uniforme part AVANT la passe, comme pour les quads du volet : une
    /// ecriture de file glissee au milieu d'une passe est licite mais range
    /// mal, et la premiere fois qu'il y aura deux galets a peindre elle serait
    /// fausse.
    pub fn televerser(&self, gpu: &Gpu, u: &Uniformes) {
        gpu.file.write_buffer(&self.tampon, 0, bytemuck::bytes_of(u));
    }

    /// La region du bureau actuellement dans le verre, s'il y en a une.
    pub fn source(&self) -> Option<Source> {
        self.source
    }

    /// Le verre n'a plus de fond : le galet redevient noir opaque a la
    /// prochaine trame. Les couches restent allouees pour le retour du fond.
    pub fn oublier_fond(&mut self) {
        self.source = None;
    }

    fn faire_couches(&self, gpu: &Gpu, taille: (u32, u32)) -> Couches {
        let faire = |nom, usage| {
            gpu.peripherique.create_texture(&wgpu::TextureDescriptor {
                label: Some(nom),
                size: wgpu::Extent3d {
                    width: taille.0,
                    height: taille.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT_COUCHES,
                usage,
                view_formats: &[],
            })
        };
        let lue = wgpu::TextureUsages::TEXTURE_BINDING;
        let nette = faire("couche nette", lue | wgpu::TextureUsages::COPY_DST);
        let inter = faire("flou horizontal", lue | wgpu::TextureUsages::RENDER_ATTACHMENT);
        let floutee = faire("couche floutee", lue | wgpu::TextureUsages::RENDER_ATTACHMENT);
        let vue_nette = nette.create_view(&Default::default());
        let vue_inter = inter.create_view(&Default::default());
        let vue_floutee = floutee.create_view(&Default::default());
        // Les couches du givre, a demi-resolution, arrondie au-dessus.
        let pas = transformation::PAS as u32;
        let taille_givree = (taille.0.div_ceil(pas).max(1), taille.1.div_ceil(pas).max(1));
        let faire_demi = |nom| {
            gpu.peripherique.create_texture(&wgpu::TextureDescriptor {
                label: Some(nom),
                size: wgpu::Extent3d {
                    width: taille_givree.0,
                    height: taille_givree.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT_COUCHES,
                usage: lue | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        };
        let inter_givre = faire_demi("flou du givre, horizontal");
        let givree = faire_demi("couche givree");
        let vue_inter_givre = inter_givre.create_view(&Default::default());
        let vue_givree = givree.create_view(&Default::default());
        let groupe = |nom, disposition, tampon: &wgpu::Buffer, vues: &[(u32, &wgpu::TextureView)]| {
            let mut entrees = vec![
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tampon.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.echantillonneur),
                },
            ];
            for (binding, vue) in vues {
                entrees.push(wgpu::BindGroupEntry {
                    binding: *binding,
                    resource: wgpu::BindingResource::TextureView(vue),
                });
            }
            gpu.peripherique
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(nom),
                    layout: disposition,
                    entries: &entrees,
                })
        };
        let groupe_h = groupe("flou horizontal", &self.disposition_flou, &self.tampon_h, &[(3, &vue_nette)]);
        let groupe_v = groupe("flou vertical", &self.disposition_flou, &self.tampon_v, &[(3, &vue_inter)]);
        let groupe_verre = groupe(
            "couches du verre",
            &self.disposition_verre,
            &self.tampon_verre,
            &[(2, &vue_floutee), (3, &vue_nette)],
        );
        // Le flou du givre lit la couche FLOUTEE (pas de deux), puis son
        // intermediaire (pas de un) : la source va a la meme liaison 3 que la
        // nette pour le flou du verre.
        let groupe_givre_h = groupe(
            "flou du givre, horizontal",
            &self.disposition_flou,
            &self.tampon_givre_h,
            &[(3, &vue_floutee)],
        );
        let groupe_givre_v = groupe(
            "flou du givre, vertical",
            &self.disposition_flou,
            &self.tampon_givre_v,
            &[(3, &vue_inter_givre)],
        );
        Couches {
            taille,
            nette,
            vue_inter,
            vue_floutee,
            groupe_h,
            groupe_v,
            groupe_verre,
            taille_givree,
            vue_inter_givre,
            vue_givree,
            groupe_givre_h,
            groupe_givre_v,
            groupe_givre: None,
        }
    }

    /// Le groupe du tuyau `givre`, fait quand la texture des cristaux et les
    /// couches existent toutes les deux.
    fn groupe_givre(&self, gpu: &Gpu, c: &Couches, cristaux: &CristauxGpu) -> wgpu::BindGroup {
        let vue_nette = c.nette.create_view(&Default::default());
        gpu.peripherique
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("couches du givre"),
                layout: &self.disposition_givre,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.tampon_verre.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.echantillonneur),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&c.vue_floutee),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&vue_nette),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&c.vue_givree),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&cristaux.vue),
                    },
                ],
            })
    }

    /// Fait pousser les cristaux du galet deploye et les televerse, une fois.
    fn assurer_cristaux(&mut self, gpu: &Gpu, galet: (f32, f32, f32, f32)) {
        if self.cristaux.is_some() {
            return;
        }
        // La pousse du fil, si c'est bien le galet deploye qu'elle a fait
        // pousser et si l'epreuve ne l'interdit pas ; sinon, dans la trame.
        let deploye = (
            crate::ilot::DEPLOYE_L,
            crate::ilot::DEPLOYE_H,
            crate::ilot::DEPLOYE_RAYON,
            crate::ilot::DEPLOYE_N,
        );
        // Le releve de m-g1 : ce que cette premiere transformation attend.
        let ordre = self.construit.elapsed();
        let debut = std::time::Instant::now();
        let (c, duree, dans_le_fil) = match self.pousse.take() {
            Some(fil) if galet == deploye && !self.epreuve_pousse && !self.epreuve_tiges => {
                let (c, duree) = fil.join().expect("la pousse des cristaux a echoue dans son fil");
                (c, duree, true)
            }
            _ => {
                let c = crate::givre::pousser_avec(galet.0, galet.1, galet.2, galet.3, crate::givre::GRAINE, !self.epreuve_tiges);
                (c, debut.elapsed(), false)
            }
        };
        self.pousse_relevee = Some(PousseRelevee {
            duree,
            ordre,
            attente: debut.elapsed(),
            dans_le_fil,
        });
        let texture = gpu.peripherique.create_texture(&wgpu::TextureDescriptor {
            label: Some("cristaux du givre"),
            size: wgpu::Extent3d {
                width: c.largeur,
                height: c.hauteur,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Des donnees, pas des couleurs : lineaire.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.file.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &c.texels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(c.largeur * 4),
                rows_per_image: Some(c.hauteur),
            },
            wgpu::Extent3d {
                width: c.largeur,
                height: c.hauteur,
                depth_or_array_layers: 1,
            },
        );
        self.cristaux = Some(CristauxGpu {
            vue: texture.create_view(&Default::default()),
            largeur_front: c.largeur_front,
        });
        if let Some(couches) = self.couches.as_mut() {
            couches.groupe_givre = None;
        }
    }

    /// Tire la couche givree de la couche floutee : deux passes a
    /// demi-resolution. Ne se refait que pour un fond neuf. Rend les
    /// millisecondes GPU des deux passes si un horodateur est donne.
    fn assurer_givree(&mut self, gpu: &Gpu, horodateur: Option<&wgpu::QuerySet>) {
        let (Some(s), Some(c)) = (self.source, self.couches.as_ref()) else {
            return;
        };
        if self.givree_a_jour && horodateur.is_none() {
            return;
        }
        let pas = transformation::PAS;
        // L'ecart-type ajoute a la couche floutee, en pixels de la demi-resolution.
        let sigma_total = transformation::SIGMA * self.echelle;
        let sigma_verre = arbitree::SIGMA * self.echelle;
        let sigma = (sigma_total * sigma_total - sigma_verre * sigma_verre).max(1.0).sqrt() / pas;
        let noyau = (3.0 * sigma).ceil();
        let region_g = ((s.l as f32 / pas).ceil(), (s.h as f32 / pas).ceil());
        let vide = [0.0f32; 4];
        let horizontale = ReglagesVerre {
            flou: [1.0, 0.0, sigma, noyau],
            // La source est la couche floutee, a pleine resolution.
            source: [0.0, 0.0, s.l as f32, s.h as f32],
            textures: [c.taille.0 as f32, c.taille.1 as f32, 0.0, 0.0],
            matiere: vide,
            biseau: vide,
            givre: vide,
            givre2: [0.0, 0.0, pas, 0.0],
            givre3: vide,
            lampe: vide,
        };
        let verticale = ReglagesVerre {
            flou: [0.0, 1.0, sigma, noyau],
            // La source est l'intermediaire, deja a demi-resolution.
            source: [0.0, 0.0, region_g.0, region_g.1],
            textures: [c.taille_givree.0 as f32, c.taille_givree.1 as f32, 0.0, 0.0],
            givre2: [0.0, 0.0, 1.0, 0.0],
            ..horizontale
        };
        gpu.file.write_buffer(&self.tampon_givre_h, 0, bytemuck::bytes_of(&horizontale));
        gpu.file.write_buffer(&self.tampon_givre_v, 0, bytemuck::bytes_of(&verticale));
        let mut enc = gpu
            .peripherique
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("flou du givre"),
            });
        for (rang, (groupe, cible)) in
            [(&c.groupe_givre_h, &c.vue_inter_givre), (&c.groupe_givre_v, &c.vue_givree)].into_iter().enumerate()
        {
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("flou du givre"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: cible,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: horodateur.map(|h| wgpu::RenderPassTimestampWrites {
                    query_set: h,
                    beginning_of_pass_write_index: if rang == 0 { Some(0) } else { None },
                    end_of_pass_write_index: if rang == 1 { Some(1) } else { None },
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            passe.set_viewport(0.0, 0.0, region_g.0, region_g.1, 0.0, 1.0);
            passe.set_pipeline(&self.tuyau_flou_givre);
            passe.set_bind_group(1, groupe, &[]);
            passe.draw(0..6, 0..1);
        }
        gpu.file.submit(Some(enc.finish()));
        self.givree_a_jour = true;
    }

    /// Refait les deux passes du flou du verre, pour les horodater : c'est
    /// l'instrument du cout, et rien d'autre ne l'appelle.
    pub fn refaire_flou_du_verre(&self, gpu: &Gpu, horodateur: &wgpu::QuerySet) {
        let Some(c) = self.couches.as_ref() else { return };
        let Some(s) = self.source else { return };
        let mut enc = gpu
            .peripherique
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("flou du verre, mesure"),
            });
        for (rang, (groupe, cible)) in [(&c.groupe_h, &c.vue_inter), (&c.groupe_v, &c.vue_floutee)].into_iter().enumerate() {
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("flou du verre"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: cible,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: Some(wgpu::RenderPassTimestampWrites {
                    query_set: horodateur,
                    beginning_of_pass_write_index: if rang == 0 { Some(0) } else { None },
                    end_of_pass_write_index: if rang == 1 { Some(1) } else { None },
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            passe.set_viewport(0.0, 0.0, s.l as f32, s.h as f32, 0.0, 1.0);
            passe.set_pipeline(&self.tuyau_flou);
            passe.set_bind_group(1, groupe, &[]);
            passe.draw(0..6, 0..1);
        }
        gpu.file.submit(Some(enc.finish()));
    }

    /// Les deux passes du flou du givre, horodatees. Instrument du cout.
    pub fn refaire_flou_du_givre(&mut self, gpu: &Gpu, horodateur: &wgpu::QuerySet) {
        self.assurer_givree(gpu, Some(horodateur));
    }

    /// Televerse une region du bureau dans la couche nette et en tire la couche
    /// floutee : deux passes separees, en lineaire, sigma et noyau du banc mis
    /// a l'echelle de l'ecran.
    ///
    /// Le flou ne se refait qu'ICI, donc seulement quand le bureau a vraiment
    /// change : une trame de l'ilot sans fond neuf ne coute que la composition.
    /// `plancher` est la plus grande region que l'ilot puisse demander ; les
    /// couches sont allouees au moins a cette taille, pour que le deploiement
    /// ne les refasse pas a chaque trame.
    pub fn televerser_fond(
        &mut self,
        gpu: &Gpu,
        pixels: &[u8],
        source: Source,
        echelle: f32,
        plancher: (u32, u32),
    ) {
        let octets = source.l as usize * source.h as usize * 4;
        if source.l == 0 || source.h == 0 || pixels.len() < octets {
            self.source = None;
            return;
        }
        let trop_petites = self
            .couches
            .as_ref()
            .is_none_or(|c| c.taille.0 < source.l || c.taille.1 < source.h);
        if trop_petites {
            let taille = (source.l.max(plancher.0), source.h.max(plancher.1));
            self.couches = Some(self.faire_couches(gpu, taille));
        }
        let c = self.couches.as_ref().expect("allouees juste au-dessus");
        gpu.file.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &c.nette,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels[..octets],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(source.l * 4),
                rows_per_image: Some(source.h),
            },
            wgpu::Extent3d {
                width: source.l,
                height: source.h,
                depth_or_array_layers: 1,
            },
        );

        let sigma = arbitree::SIGMA * echelle;
        // Le noyau du banc : `ceil(3 sigma)` de chaque cote.
        let noyau = (3.0 * sigma).ceil();
        let base = ReglagesVerre {
            flou: [1.0, 0.0, sigma, noyau],
            source: [0.0, 0.0, source.l as f32, source.h as f32],
            textures: [c.taille.0 as f32, c.taille.1 as f32, 0.0, 0.0],
            matiere: [0.0; 4],
            biseau: [0.0; 4],
            givre: [0.0; 4],
            givre2: [0.0; 4],
            givre3: [0.0; 4],
            lampe: [0.0; 4],
        };
        let vertical = ReglagesVerre {
            flou: [0.0, 1.0, sigma, noyau],
            ..base
        };
        gpu.file.write_buffer(&self.tampon_h, 0, bytemuck::bytes_of(&base));
        gpu.file.write_buffer(&self.tampon_v, 0, bytemuck::bytes_of(&vertical));

        let mut enc = gpu
            .peripherique
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("flou du verre"),
            });
        for (groupe, cible) in [(&c.groupe_h, &c.vue_inter), (&c.groupe_v, &c.vue_floutee)] {
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("flou du verre"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: cible,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // La passe ne couvre que la region : ses pixels sont ceux de la
            // region, et la texture au-dela n'est jamais lue (`dans` borne).
            passe.set_viewport(0.0, 0.0, source.l as f32, source.h as f32, 0.0, 1.0);
            passe.set_pipeline(&self.tuyau_flou);
            passe.set_bind_group(1, groupe, &[]);
            passe.draw(0..6, 0..1);
        }
        gpu.file.submit(Some(enc.finish()));
        self.source = Some(source);
        // Un fond neuf : la couche givree, si une transformation la demande,
        // sera refaite de lui.
        self.givree_a_jour = false;
        self.echelle = echelle;
    }

    /// Ecrit les reglages de composition. Rend faux s'il n'y a pas de fond : le
    /// galet doit alors etre peint noir.
    ///
    /// `origine` : le pixel (0, 0) de la surface, dans le bureau virtuel.
    /// `hauteur` : la hauteur COURANTE du galet, en pixels physiques.
    pub fn preparer_verre(&self, gpu: &Gpu, origine: (i32, i32), hauteur: f32, echelle: f32) -> bool {
        let (Some(s), Some(c)) = (self.source, self.couches.as_ref()) else {
            return false;
        };
        // Le conge suit la hauteur : `12 x h / 148` en logiques, et comme h et
        // 148 prennent la meme echelle, le rapport se lit en physiques.
        let suit = hauteur / (arbitree::HAUTEUR_DU_CONGE * echelle);
        let r = ReglagesVerre {
            flou: [0.0; 4],
            source: [
                (origine.0 - s.x) as f32,
                (origine.1 - s.y) as f32,
                s.l as f32,
                s.h as f32,
            ],
            textures: [
                c.taille.0 as f32,
                c.taille.1 as f32,
                arbitree::MARGE * echelle,
                arbitree::PRISES,
            ],
            matiere: [
                arbitree::TEINTE,
                arbitree::SATURATION,
                self.force(echelle),
                arbitree::CLARTE,
            ],
            biseau: [
                arbitree::CONGE * echelle * suit,
                arbitree::INDICE,
                arbitree::EPAISSEUR * echelle * suit,
                arbitree::REFRACTION,
            ],
            // Au repos sur le verre : le tuyau du verre ne lit pas ces champs.
            givre: [1.0, 1.0, 0.0, 0.0],
            givre2: [0.0; 4],
            givre3: [0.0; 4],
            lampe: [
                self.lampe_intensite(suit),
                lampe::CIEL,
                lampe::reflectance_du_bord(arbitree::CONGE * echelle * suit, arbitree::INDICE, arbitree::PRISES as u32),
                0.0,
            ],
        };
        gpu.file.write_buffer(&self.tampon_verre, 0, bytemuck::bytes_of(&r));
        true
    }

    /// Ecrit les reglages d'une trame de transformation, par-dessus ceux du
    /// verre que `preparer_verre` vient d'ecrire, et s'assure des deux couches
    /// que le tuyau `givre` lit. `preparer_verre` doit avoir rendu vrai.
    ///
    /// `origine` et `hauteur` sont ceux de `preparer_verre` ; `galet_l` et
    /// `galet_h` la taille COURANTE du galet en pixels physiques, pour la portee
    /// de la fonte. Rend faux si la transformation ne peut pas se peindre.
    pub fn preparer_givre(
        &mut self,
        gpu: &Gpu,
        origine: (i32, i32),
        hauteur: f32,
        echelle: f32,
        centre: (f32, f32),
        demi: (f32, f32),
        t: &Transformation,
    ) -> bool {
        if self.source.is_none() || self.couches.is_none() {
            return false;
        }
        self.assurer_cristaux(gpu, t.galet);
        self.assurer_givree(gpu, None);
        let cristaux = self.cristaux.as_ref().expect("assures juste au-dessus");
        if self.couches.as_ref().is_some_and(|c| c.groupe_givre.is_none()) {
            let groupe = self.groupe_givre(gpu, self.couches.as_ref().unwrap(), cristaux);
            self.couches.as_mut().unwrap().groupe_givre = Some(groupe);
        }
        let (s, c) = (self.source.unwrap(), self.couches.as_ref().unwrap());
        // La portee de la fonte : le coin du galet le plus loin de l'origine.
        let mut portee = 1.0f32;
        for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let (cx, cy) = (centre.0 + sx * demi.0, centre.1 + sy * demi.1);
            portee = portee.max(((cx - t.origine.0).powi(2) + (cy - t.origine.1).powi(2)).sqrt());
        }
        let suit = hauteur / (arbitree::HAUTEUR_DU_CONGE * echelle);
        let pas = transformation::PAS;
        let r = ReglagesVerre {
            flou: [0.0; 4],
            source: [
                (origine.0 - s.x) as f32,
                (origine.1 - s.y) as f32,
                s.l as f32,
                s.h as f32,
            ],
            textures: [
                c.taille.0 as f32,
                c.taille.1 as f32,
                arbitree::MARGE * echelle,
                arbitree::PRISES,
            ],
            matiere: [
                arbitree::TEINTE,
                arbitree::SATURATION,
                self.force(echelle),
                arbitree::CLARTE,
            ],
            biseau: [
                arbitree::CONGE * echelle * suit,
                arbitree::INDICE,
                arbitree::EPAISSEUR * echelle * suit,
                arbitree::REFRACTION,
            ],
            givre: [t.givre, t.fonte, t.origine.0, t.origine.1],
            givre2: [
                portee,
                cristaux.largeur_front,
                pas,
                transformation::MENISQUE * echelle,
            ],
            givre3: [
                c.taille_givree.0 as f32,
                c.taille_givree.1 as f32,
                (s.l as f32 / pas).ceil(),
                (s.h as f32 / pas).ceil(),
            ],
            lampe: [
                self.lampe_intensite(suit),
                lampe::CIEL,
                lampe::reflectance_du_bord(arbitree::CONGE * echelle * suit, arbitree::INDICE, arbitree::PRISES as u32),
                0.0,
            ],
        };
        gpu.file.write_buffer(&self.tampon_verre, 0, bytemuck::bytes_of(&r));
        true
    }

    /// Peint le galet : de verre si `verre` et qu'un fond est la, noir sinon.
    /// Une transformation en cours (`Some`, hors de ses deux extremites) passe
    /// par le tuyau `givre` ; a ses extremites, par les tuyaux d'avant. Le
    /// galet est trace `fois` fois, un dans l'application : l'instrument du
    /// cout allonge ainsi sa reference sans changer ce qu'il mesure
    /// (RECETTES.md).
    pub fn dessiner_fois(
        &self,
        passe: &mut wgpu::RenderPass<'_>,
        verre: bool,
        transformation: Option<Transformation>,
        fois: u32,
    ) {
        let fond = self.couches.as_ref().filter(|_| verre && self.source.is_some());
        let fois = fois.max(1);
        if let Some(c) = fond {
            match transformation.map(|t| t.au_repos()) {
                // Pas de transformation, ou posee sur le verre : le tuyau du
                // verre, tel quel.
                None | Some(Some(true)) => {
                    passe.set_pipeline(&self.tuyau_verre);
                    passe.set_bind_group(0, &self.groupe, &[]);
                    passe.set_bind_group(1, &c.groupe_verre, &[]);
                    passe.draw(0..6, 0..fois);
                    return;
                }
                // En cours : le tuyau du givre, s'il a ses couches.
                Some(None) => {
                    if let Some(g) = c.groupe_givre.as_ref() {
                        passe.set_pipeline(&self.tuyau_givre);
                        passe.set_bind_group(0, &self.groupe, &[]);
                        passe.set_bind_group(1, g, &[]);
                        passe.draw(0..6, 0..fois);
                        return;
                    }
                }
                // Posee sur le galet opaque : le tuyau noir, tel quel.
                Some(Some(false)) => {}
            }
        }
        passe.set_pipeline(&self.pipeline);
        passe.set_bind_group(0, &self.groupe, &[]);
        passe.draw(0..6, 0..fois);
    }

    /// La vue de l'image entiere, a la taille de la fenetre (refaite si elle
    /// change) : la trame d'un etat s'y peint en entier.
    pub fn image(&mut self, gpu: &Gpu, largeur: u32, hauteur: u32) -> wgpu::TextureView {
        let a_refaire = !matches!(&self.image, Some((l, h, _, _)) if *l == largeur && *h == hauteur);
        if a_refaire {
            let texture = gpu.peripherique.create_texture(&wgpu::TextureDescriptor {
                label: Some("image entiere"),
                size: wgpu::Extent3d { width: largeur, height: hauteur, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let vue = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let groupe = gpu.peripherique.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("image entiere"),
                layout: &self.disposition_image,
                entries: &[wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&vue) }],
            });
            self.image = Some((largeur, hauteur, vue, groupe));
        }
        self.image.as_ref().map(|(_, _, v, _)| v.clone()).expect("image entiere creee")
    }

    /// Pose l'image entiere par-dessus la passe, a l'opacite des uniformes.
    pub fn poser_l_image(
        &self,
        passe: &mut wgpu::RenderPass<'_>,
        ) {
        let Some((_, _, _, groupe)) = self.image.as_ref() else { return };
        passe.set_pipeline(&self.tuyau_copie);
        passe.set_bind_group(0, &self.groupe, &[]);
        passe.set_bind_group(1, groupe, &[]);
        passe.draw(0..6, 0..1);
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    /// La couverture de `galet.wgsl`, refaite ici a l'identique — division par
    /// la norme du gradient comprise. C'est l'etalon : si la silhouette de la
    /// region tombe a l'interieur de ce que ceci encre, la decoupe mord.
    fn couverture(px: f32, py: f32, l: f32, h: f32, r: f32, n: f32, ruban: f32) -> f32 {
        let qx = (px - l / 2.0).abs() - l / 2.0 + r;
        let qy = (py - h / 2.0).abs() - h / 2.0 + r;
        let d = if qx > 0.0 && qy > 0.0 {
            let f = (qx.powf(n) + qy.powf(n)).powf(1.0 / n);
            let gradient = (qx.powf(n - 1.0).powi(2) + qy.powf(n - 1.0).powi(2)).sqrt()
                / f.powf(n - 1.0).max(1e-6);
            (f - r) / gradient.max(1e-6)
        } else {
            let m = (qx.max(0.0), qy.max(0.0));
            (m.0 * m.0 + m.1 * m.1).sqrt() + qx.max(qy).min(0.0) - r
        };
        (0.5 - d / ruban.max(1.0)).clamp(0.0, 1.0)
    }

    /// LE contrat. Sans lui, la superellipse n'arrive jamais a l'ecran : c'est
    /// exactement le defaut qu'une region `CreateRoundRectRgn` produisait.
    #[test]
    fn la_decoupe_ne_mord_jamais_l_encre() {
        for &(l, h, r, n) in &[
            (400.0f32, 185.0f32, 60.0f32, 4.0f32),
            (110.0, 45.0, 22.5, 2.5),
            (320.0, 148.0, 48.0, 4.0),
            (250.0, 96.0, 33.0, 3.2),
        ] {
            for &ruban in &[1.0f32, 2.4, 4.75] {
                let sil = silhouette(l, h, r, n, ruban);
                for (i, &col) in sil.iter().enumerate() {
                    let py = i as f32 + 0.5;
                    // La premiere colonne encree de cette rangee.
                    let mut premiere = None;
                    for x in 0..(l.round() as usize) {
                        if couverture(x as f32 + 0.5, py, l, h, r, n, ruban) > 0.0 {
                            premiere = Some(x as i32);
                            break;
                        }
                    }
                    let Some(premiere) = premiere else { continue };
                    assert!(
                        col <= premiere,
                        "rangee {i} ({l}x{h}, r={r}, n={n}, ruban={ruban}) : \
                         la region commence a {col}, l'encre a {premiere}"
                    );
                }
            }
        }
    }

    /// L'exposant 2 doit redonner l'arc de cercle : c'est le temoin qui dit que
    /// la formule n'a pas derive. On compare au cercle DILATE du demi-ruban,
    /// puisque c'est bien ce que le nuanceur encre — `sqrt(qx² + qy²) < r + ½`.
    #[test]
    fn l_exposant_deux_redonne_le_cercle() {
        let (l, h, r) = (400.0f32, 185.0f32, 60.0f32);
        let sil = silhouette(l, h, r, 2.0, 1.0);
        for (i, &col) in sil.iter().enumerate().take(r as usize) {
            let qy = r - (i as f32 + 0.5);
            let qx = ((r + 0.5) * (r + 0.5) - qy * qy).max(0.0).sqrt();
            let attendu = (r - qx - 0.5).ceil().max(0.0) as i32;
            assert!(
                (col - attendu).abs() <= 1,
                "rangee {i} : {col} contre {attendu} pour le cercle"
            );
        }
    }

    /// Et le nombre qui resume l'objection : a rayon egal, la superellipse
    /// passe DEHORS du cercle, donc elle mange moins de coin. C'est cet ecart,
    /// dix-neuf pixels sur la premiere rangee, qu'une region de cercle recoupait.
    #[test]
    fn la_superellipse_mange_moins_de_coin_que_le_cercle() {
        let (l, h, r) = (400.0f32, 185.0f32, 60.0f32);
        let cercle = silhouette(l, h, r, 2.0, 1.0);
        let superellipse = silhouette(l, h, r, 4.0, 1.0);
        assert_eq!(cercle[0], 49);
        assert_eq!(superellipse[0], 30);
        for (i, (c, s)) in cercle.iter().zip(&superellipse).enumerate() {
            assert!(s <= c, "rangee {i} : la superellipse rentre sous le cercle");
        }
    }
}
