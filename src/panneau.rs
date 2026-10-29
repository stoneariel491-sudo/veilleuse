//! La pose du texte dans une fenetre : une liste de lignes, chacune avec sa
//! fonte, son corps, sa couleur et son point d'ancrage.
//!
//! Ce fichier remplace la paire date + legende ecrite en dur de l'etape 6. Le
//! projet ne s'arrete pas a l'horloge : la meme application portera une vue de
//! lecture en prose et des objectifs a recuperer de memoire. Un panneau ne sait
//! donc rien de ce qu'il affiche — il prend des lignes et les pose. Le texte
//! LONG est prevu des maintenant : une ligne a une `largeur` disponible, et
//! au-dela cosmic-text passe a la ligne tout seul.
//!
//! Ce qui n'est PAS fait, et qu'il faudra ecrire le jour de la prose : l'ancrage
//! agit sur le bloc entier, pas ligne par ligne. Pour du texte justifie ou
//! centre sur plusieurs lignes il faudra passer par l'alignement propre de
//! cosmic-text (`Buffer::set_align`). Pour une ligne unique — tout ce que
//! l'horloge et l'ilot demandent — la mesure directe est exacte.

use glyphon::cosmic_text::{FeatureTag, FontFeatures};
use glyphon::{
    Attrs, Buffer, Color, Family, Metrics, Resolution, Shaping, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport, Weight,
};

use crate::gpu::Gpu;
use crate::texte::Typographie;

/// Les trois Barlow embarquees, et rien d'autre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fonte {
    /// Barlow Condensed 800 : les chiffres. Tabulaires, toujours.
    Chiffres,
    /// Barlow 400 : la date, les sous-lignes. Fine, elle ne s'impose pas.
    Courante,
    /// Barlow 500 : les capitales espacees — legendes, phases, actions.
    Moyenne,
}

/// D'ou part `x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ancre {
    Gauche,
    Centre,
    Droite,
}

/// Une ligne a poser.
#[derive(Clone, Debug)]
pub struct Ligne {
    pub contenu: String,
    pub fonte: Fonte,
    /// Corps en pixels.
    pub corps: f32,
    /// Interlettrage en em. L'interlettrage de cosmic-text s'ajoute a l'avance
    /// NORMALISEE du glyphe : on lui passe la valeur de DESIGN.md telle quelle.
    pub interlettrage: f32,
    /// Interligne en pixels. Ne sert que si le texte passe a la ligne.
    pub interligne: f32,
    pub x: f32,
    /// Ligne de base de la PREMIERE ligne du bloc.
    pub base: f32,
    pub ancre: Ancre,
    pub couleur: Color,
    /// Largeur disponible avant le passage a la ligne.
    pub largeur: f32,
    /// Fenetre de decoupe, en pixels [x, y, largeur, hauteur]. Ce qui deborde
    /// n'est pas peint. Sans elle, la ligne est decoupee par le panneau entier.
    pub bornes: Option<[f32; 4]>,
}

impl Ligne {
    /// Une ligne simple, a poser sur une seule ligne : la largeur disponible est
    /// prise assez grande pour qu'aucun passage a la ligne n'arrive.
    pub fn simple(contenu: impl Into<String>, fonte: Fonte, corps: f32, couleur: Color) -> Ligne {
        Ligne {
            contenu: contenu.into(),
            fonte,
            corps,
            interlettrage: 0.0,
            interligne: corps,
            x: 0.0,
            base: 0.0,
            ancre: Ancre::Gauche,
            couleur,
            largeur: f32::INFINITY,
            bornes: None,
        }
    }

    pub fn espacee(mut self, interlettrage: f32) -> Ligne {
        self.interlettrage = interlettrage;
        self
    }

    pub fn a(mut self, x: f32, base: f32) -> Ligne {
        self.x = x;
        self.base = base;
        self
    }

    pub fn ancree(mut self, ancre: Ancre) -> Ligne {
        self.ancre = ancre;
        self
    }

    /// Ce qui oblige a refaconner. La couleur et la position n'en font pas
    /// partie : elles se changent a chaque trame sans rien recalculer.
    fn empreinte(&self) -> Empreinte {
        Empreinte {
            contenu: self.contenu.clone(),
            fonte: self.fonte,
            corps: self.corps.to_bits(),
            interlettrage: self.interlettrage.to_bits(),
            interligne: self.interligne.to_bits(),
            largeur: self.largeur.to_bits(),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Empreinte {
    contenu: String,
    fonte: Fonte,
    corps: u32,
    interlettrage: u32,
    interligne: u32,
    largeur: u32,
}

pub fn attrs(fonte: Fonte, interlettrage: f32) -> Attrs<'static> {
    match fonte {
        Fonte::Chiffres => {
            // `tnum` force l'avance tabulaire : sans elle « 1111 » est plus
            // etroit que « 0000 », et le galet replie changerait de largeur en
            // cours de minute. `kern` est coupee : entre chiffres tabulaires le
            // crenage est nul de toute facon, et l'ecrire ferme la question.
            let mut features = FontFeatures::new();
            features.enable(FeatureTag::new(b"tnum"));
            features.disable(FeatureTag::new(b"kern"));
            Attrs::new()
                .family(Family::Name("Barlow Condensed"))
                .weight(Weight(800))
                .font_features(features)
                .letter_spacing(interlettrage)
        }
        Fonte::Courante => Attrs::new()
            .family(Family::Name("Barlow"))
            .weight(Weight(400))
            .letter_spacing(interlettrage),
        Fonte::Moyenne => Attrs::new()
            .family(Family::Name("Barlow"))
            .weight(Weight(500))
            .letter_spacing(interlettrage),
    }
}

struct Tampon {
    tampon: Buffer,
    empreinte: Empreinte,
    /// Largeur du texte, en pixels, interlettrage final deduit.
    largeur: f32,
    /// Position verticale de la ligne de base dans le tampon.
    ligne_y: f32,
}

pub struct Panneau {
    atlas: TextAtlas,
    rendu: TextRenderer,
    ecran: Viewport,
    tampons: Vec<Tampon>,
    taille: (u32, u32),
}

impl Panneau {
    pub fn nouveau(
        gpu: &Gpu,
        typo: &mut Typographie,
        format: wgpu::TextureFormat,
        largeur: u32,
        hauteur: u32,
    ) -> Panneau {
        let mut atlas = TextAtlas::new(&gpu.peripherique, &gpu.file, &typo.cache, format);
        let rendu = TextRenderer::new(
            &mut atlas,
            &gpu.peripherique,
            wgpu::MultisampleState::default(),
            None,
        );
        let mut ecran = Viewport::new(&gpu.peripherique, &typo.cache);
        ecran.update(
            &gpu.file,
            Resolution {
                width: largeur,
                height: hauteur,
            },
        );
        Panneau {
            atlas,
            rendu,
            ecran,
            tampons: Vec::new(),
            taille: (largeur, hauteur),
        }
    }

    /// Faconne ce qui a change, et rien d'autre. A appeler avant de lire les
    /// largeurs et avant de dessiner.
    pub fn preparer(&mut self, typo: &mut Typographie, lignes: &[Ligne]) {
        self.tampons.truncate(lignes.len());
        for (i, ligne) in lignes.iter().enumerate() {
            let empreinte = ligne.empreinte();
            if let Some(t) = self.tampons.get(i) {
                if t.empreinte == empreinte {
                    continue;
                }
            }
            let mut tampon = Buffer::new(
                &mut typo.systeme,
                Metrics::new(ligne.corps, ligne.interligne),
            );
            // Une hauteur libre : le tampon garde toutes ses lignes, c'est la
            // fenetre qui decide de ce qui se voit.
            tampon.set_size(Some(ligne.largeur), None);
            // Shaping::Advanced est le chemin HarfRust, le seul qui applique les
            // tables OpenType — donc la seule facon d'obtenir `tnum`.
            tampon.set_text(
                &ligne.contenu,
                &attrs(ligne.fonte, ligne.interlettrage),
                Shaping::Advanced,
                None,
            );
            tampon.shape_until_scroll(&mut typo.systeme, false);

            // L'interlettrage ajoute une avance APRES la derniere lettre ; sans
            // la retrancher, une ligne centree penche a gauche.
            let queue = ligne.interlettrage * ligne.corps;
            let largeur = tampon
                .layout_runs()
                .map(|r| r.line_w)
                .fold(0.0f32, f32::max)
                - queue;
            let ligne_y = tampon.layout_runs().next().map(|r| r.line_y).unwrap_or(0.0);

            let t = Tampon {
                tampon,
                empreinte,
                largeur: largeur.max(0.0),
                ligne_y,
            };
            match self.tampons.get_mut(i) {
                Some(place) => *place = t,
                None => self.tampons.push(t),
            }
        }
    }

    /// Largeur du texte de la ligne `i`, en pixels. `preparer` doit avoir ete
    /// appele avec la meme liste.
    pub fn largeur(&self, i: usize) -> f32 {
        self.tampons.get(i).map(|t| t.largeur).unwrap_or(0.0)
    }

    /// Abscisse du bord gauche du texte, une fois l'ancrage applique.
    pub fn gauche(&self, lignes: &[Ligne], i: usize) -> f32 {
        let l = self.largeur(i);
        match lignes[i].ancre {
            Ancre::Gauche => lignes[i].x,
            Ancre::Centre => lignes[i].x - l / 2.0,
            Ancre::Droite => lignes[i].x - l,
        }
    }

    pub fn dessiner(
        &mut self,
        gpu: &Gpu,
        typo: &mut Typographie,
        lignes: &[Ligne],
        passe: &mut wgpu::RenderPass<'_>,
    ) {
        if lignes.is_empty() {
            return;
        }
        // Les abscisses se calculent AVANT de scinder l'emprunt : `gauche` lit
        // `self` en entier, et les zones qui suivent tiennent une reference sur
        // les tampons pendant que `prepare` a besoin du reste en mutable.
        // Arrondi a l'entier : l'encre posee sur la grille de pixels n'est pas
        // reechantillonnee, donc elle ne s'adoucit pas.
        let gauches: Vec<f32> = (0..lignes.len())
            .map(|i| self.gauche(lignes, i).round())
            .collect();
        let Panneau {
            atlas,
            rendu,
            ecran,
            tampons,
            taille,
        } = self;
        let (l, h) = *taille;
        let zones: Vec<TextArea> = lignes
            .iter()
            .enumerate()
            .filter(|(_, ligne)| ligne.couleur.a() > 0)
            .map(|(i, ligne)| {
                let t = &tampons[i];
                TextArea {
                    buffer: &t.tampon,
                    left: gauches[i],
                    top: (ligne.base - t.ligne_y).round(),
                    scale: 1.0,
                    bounds: match ligne.bornes {
                        Some([bx, by, bl, bh]) => TextBounds {
                            left: bx.round() as i32,
                            top: by.round() as i32,
                            right: (bx + bl).round() as i32,
                            bottom: (by + bh).round() as i32,
                        },
                        None => TextBounds {
                            left: 0,
                            top: 0,
                            right: l as i32,
                            bottom: h as i32,
                        },
                    },
                    default_color: ligne.couleur,
                    custom_glyphs: &[],
                }
            })
            .collect();
        if zones.is_empty() {
            return;
        }

        rendu
            .prepare(
                &gpu.peripherique,
                &gpu.file,
                &mut typo.systeme,
                atlas,
                ecran,
                zones,
                &mut typo.swash,
            )
            .expect("preparation du texte impossible");
        rendu
            .render(atlas, ecran, passe)
            .expect("rendu du texte impossible");
    }
}
