//! Les polices embarquees, et l'atlas hors-ecran ou les chiffres sont graves
//! avant d'etre pivotes.
//!
//! La pose du texte dans une fenetre, elle, vit dans `panneau.rs` : ce fichier
//! ne s'occupe plus que de ce qui doit tourner en 3D.

use glyphon::{
    Buffer, Cache, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea, TextAtlas,
    TextBounds, TextRenderer, Viewport,
};

use crate::geometrie::{Geometrie, CARTES};
use crate::gpu::Gpu;
use crate::panneau::{attrs, Fonte};

// Les trois fichiers sont dans le binaire : l'application ne touche jamais le
// disque ni le reseau pour ses polices, et la chaine de secours ne sert jamais.
const BARLOW_CONDENSED_800: &[u8] = include_bytes!("../assets/fonts/BarlowCondensed-ExtraBold.ttf");
const BARLOW_400: &[u8] = include_bytes!("../assets/fonts/Barlow-Regular.ttf");
const BARLOW_500: &[u8] = include_bytes!("../assets/fonts/Barlow-Medium.ttf");

/// Format de l'atlas hors-ecran. sRGB, comme les deux cibles finales : une
/// couleur ecrite en F2EFE9 se relit en F2EFE9 d'un bout a l'autre.
pub const FORMAT_ATLAS: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub struct Typographie {
    pub systeme: FontSystem,
    pub swash: SwashCache,
    pub cache: Cache,
}

impl Typographie {
    pub fn nouvelle(gpu: &Gpu) -> Typographie {
        // Base VIDE : aucune police du systeme n'est scrutee, donc aucune ne
        // peut se substituer a Barlow sans qu'on le voie.
        let mut base = glyphon::fontdb::Database::new();
        base.load_font_data(BARLOW_CONDENSED_800.to_vec());
        base.load_font_data(BARLOW_400.to_vec());
        base.load_font_data(BARLOW_500.to_vec());
        let systeme = FontSystem::new_with_locale_and_db("fr-CA".to_string(), base);

        Typographie {
            systeme,
            swash: SwashCache::new(),
            cache: Cache::new(&gpu.peripherique),
        }
    }

    pub fn familles(&self) -> Vec<String> {
        let mut noms: Vec<String> = self
            .systeme
            .db()
            .faces()
            .map(|f| f.families[0].0.clone())
            .collect();
        noms.sort();
        noms.dedup();
        noms
    }
}

/// L'atlas hors-ecran des six cartes.
///
/// glyphon n'accepte ni matrice ni rotation : `TextArea` n'expose que `left`,
/// `top`, `scale` et `bounds`. Le volet ne peut donc pas etre obtenu en faisant
/// tourner du texte. On grave les chiffres dans une texture, et c'est cette
/// texture que le volet echantillonne.
///
/// Une seule texture, en grille : six colonnes (les six cartes), deux rangees
/// (en haut la valeur NOUVELLE, en bas l'ANCIENNE). Le nuanceur choisit l'une
/// ou l'autre par ses seules coordonnees de texture, sans changer de groupe de
/// liaison entre les quads.
pub struct AtlasChiffres {
    /// Gardee pour la duree de vie : c'est la vue qui sert au rendu, mais la
    /// texture doit lui survivre.
    #[allow(dead_code)]
    pub texture: wgpu::Texture,
    pub vue: wgpu::TextureView,
    pub largeur: u32,
    pub hauteur: u32,
    cellule_l: u32,
    cellule_h: u32,
    base: f32,
    tampons: Vec<Buffer>,
    atlas: TextAtlas,
    rendu: TextRenderer,
    ecran: Viewport,
    /// [0] = valeurs nouvelles, [1] = valeurs anciennes.
    pub valeurs: [[u8; CARTES]; 2],
    sale: bool,
}

impl AtlasChiffres {
    pub fn nouveau(gpu: &Gpu, typo: &mut Typographie, geo: &Geometrie) -> AtlasChiffres {
        let cellule_l = geo.carte_l;
        let cellule_h = geo.carte_h;
        let largeur = cellule_l * CARTES as u32;
        let hauteur = cellule_h * 2;

        let texture = gpu.peripherique.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas des chiffres"),
            size: wgpu::Extent3d {
                width: largeur,
                height: hauteur,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT_ATLAS,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let vue = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Un tampon PAR chiffre, pas un pour la rangee : c'est ce qui permet de
        // ne refaconner que la carte qui change.
        let metriques = Metrics::new(geo.f, geo.f);
        let tampons = (0..CARTES * 2)
            .map(|_| {
                let mut b = Buffer::new(&mut typo.systeme, metriques);
                b.set_size(Some(cellule_l as f32 * 2.0), Some(geo.f * 2.0));
                b
            })
            .collect();

        let mut atlas = TextAtlas::new(
            &gpu.peripherique,
            &gpu.file,
            &typo.cache,
            FORMAT_ATLAS,
        );
        let rendu = TextRenderer::new(
            &mut atlas,
            &gpu.peripherique,
            wgpu::MultisampleState::default(),
            None,
        );
        let mut ecran = Viewport::new(&gpu.peripherique, &typo.cache);
        // La resolution donnee a glyphon est celle de la TEXTURE, pas celle de
        // la fenetre.
        ecran.update(&gpu.file, Resolution { width: largeur, height: hauteur });

        AtlasChiffres {
            texture,
            vue,
            largeur,
            hauteur,
            cellule_l,
            cellule_h,
            base: geo.base,
            tampons,
            atlas,
            rendu,
            ecran,
            valeurs: [[0; CARTES]; 2],
            sale: true,
        }
    }

    /// Pose une valeur. `rangee` : 0 = nouvelle, 1 = ancienne.
    pub fn poser(&mut self, typo: &mut Typographie, rangee: usize, carte: usize, chiffre: u8) {
        if self.valeurs[rangee][carte] == chiffre && !self.sale {
            return;
        }
        self.valeurs[rangee][carte] = chiffre;
        let texte = [b'0' + chiffre] ;
        let texte = std::str::from_utf8(&texte).unwrap().to_string();
        let tampon = &mut self.tampons[rangee * CARTES + carte];
        // Shaping::Advanced est le chemin HarfRust, le seul qui applique les
        // tables OpenType — donc la seule facon d'obtenir `tnum`.
        tampon.set_text(&texte, &attrs(Fonte::Chiffres, 0.0), Shaping::Advanced, None);
        tampon.shape_until_scroll(&mut typo.systeme, false);
        self.sale = true;
    }

    pub fn a_redessiner(&self) -> bool {
        self.sale
    }

    /// Grave les douze chiffres dans la texture.
    pub fn graver(&mut self, gpu: &Gpu, typo: &mut Typographie) {
        let mut zones = Vec::with_capacity(CARTES * 2);
        for rangee in 0..2 {
            for carte in 0..CARTES {
                let tampon = &self.tampons[rangee * CARTES + carte];
                // `top` est le haut du tampon ; on veut la ligne de base a
                // `base` dans la cellule, d'ou le retrait de line_y. Arrondi a
                // l'entier : la cellule et le quad sont en pixels entiers, donc
                // le glyphe doit l'etre aussi, sinon l'encre est resamplee et
                // s'adoucit pour rien.
                let ligne_y = tampon.layout_runs().next().map(|r| r.line_y).unwrap_or(0.0);
                let x = (carte as u32 * self.cellule_l) as f32;
                let y = (rangee as u32 * self.cellule_h) as f32;
                zones.push(TextArea {
                    buffer: tampon,
                    left: x,
                    top: (y + self.base - ligne_y).round(),
                    scale: 1.0,
                    bounds: TextBounds {
                        left: x as i32,
                        top: y as i32,
                        right: (x + self.cellule_l as f32) as i32,
                        bottom: (y + self.cellule_h as f32) as i32,
                    },
                    default_color: crate::palette::ENCRE,
                    custom_glyphs: &[],
                });
            }
        }

        self.rendu
            .prepare(
                &gpu.peripherique,
                &gpu.file,
                &mut typo.systeme,
                &mut self.atlas,
                &self.ecran,
                zones,
                &mut typo.swash,
            )
            .expect("preparation du texte impossible");

        let mut enc = gpu
            .peripherique
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gravure des chiffres"),
            });
        {
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("atlas des chiffres"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.vue,
                    resolve_target: None,
                    depth_slice: None,
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
            self.rendu
                .render(&self.atlas, &self.ecran, &mut passe)
                .expect("rendu du texte impossible");
        }
        gpu.file.submit(Some(enc.finish()));
        self.sale = false;
    }
}
