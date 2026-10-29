//! Ce qui est peint : le noir, les six cartes, et leurs volets.
//!
//! DESIGN.md §2 : `#000000` pur, parce que sur OLED l'ecran s'eteint et qu'un
//! gris de fond dessinerait le rectangle du moniteur dans la nuit.

use crate::geometrie::{Geometrie, CARTES};
use crate::gpu::Gpu;
use crate::options::Options;
use crate::palette;
use crate::panneau::{Ancre, Fonte, Ligne, Panneau};
use crate::rendu::{Quad, RenduQuads};
use crate::temps;
use crate::texte::{AtlasChiffres, Typographie};
use crate::volet::{Carte, Etat, Variateur};

/// Les deux rangees de l'atlas.
const NOUVELLE: u32 = 0;
const ANCIENNE: u32 = 1;

/// Hauteur de capitale de Barlow (sCapHeight = 700/1000 em).
const CAPITALE: f32 = 0.70;

pub struct Scene {
    pub geo: Geometrie,
    atlas: AtlasChiffres,
    quads: RenduQuads,
    panneau: Panneau,
    /// Les lignes de la trame en cours : la date au-dessus, la legende dessous.
    lignes: Vec<Ligne>,
    cartes: Vec<Carte>,
    /// La lampe sur variateur de DESIGN.md §7 : les chiffres baissent quand le
    /// livre se ferme.
    variateur: Variateur,
    /// Le livre est-il ferme ? C'est le chrono qui le dit, la scene ne fait que
    /// suivre.
    lumiere_basse: bool,
    date: String,
    /// La lumiere qui glisse sur la face du volet pendant la rotation.
    lumiere: bool,
    /// Le systeme demande moins de mouvement : les chiffres changent d'un coup.
    sans_volet: bool,
    legende: String,
}

impl Scene {
    pub fn nouvelle(
        gpu: &Gpu,
        typo: &mut Typographie,
        format: wgpu::TextureFormat,
        largeur: u32,
        hauteur: u32,
        opt: &Options,
    ) -> Scene {
        let geo = Geometrie::nouvelle(largeur, hauteur);
        let atlas = AtlasChiffres::nouveau(gpu, typo, &geo);
        let quads = RenduQuads::nouveau(gpu, &atlas.vue, format);
        // La taille vient de la geometrie et non des arguments : c'est elle qui
        // fait foi sur ce que la scene couvre.
        let panneau = Panneau::nouveau(gpu, typo, format, geo.largeur, geo.hauteur);

        let instant = temps::maintenant();
        let instant = match opt.heure {
            Some((h, m, s)) => temps::Instant {
                heure: h,
                minute: m,
                seconde: s,
                ..instant
            },
            None => instant,
        };
        let chiffres = instant.chiffres();

        // Les deux cartes des secondes ont leur propre duree, plus courte.
        let cartes = (0..CARTES)
            .map(|i| Carte::nouvelle(chiffres[i], i >= 4))
            .collect();

        let mut scene = Scene {
            geo,
            atlas,
            quads,
            panneau,
            lignes: Vec::new(),
            cartes,
            variateur: Variateur::allume(),
            lumiere_basse: false,
            date: String::new(),
            lumiere: opt.lumiere,
            sans_volet: crate::systeme::mouvement_reduit(),
            legende: opt.legende.clone(),
        };

        // --phase fige la bascule vers la seconde suivante a un instant precis,
        // pour la juger image par image. Toutes les cartes qui changent
        // basculent, et comme les secondes ont une duree plus courte que les
        // minutes et les heures, elles n'en sont pas au meme point : c'est
        // exactement ce qu'il faut voir a minuit.
        if let Some(p) = opt.phase {
            let suivants = instant.seconde_suivante().chiffres();
            for i in 0..CARTES {
                scene.cartes[i].basculer(suivants[i]);
                // Seules les cartes qui changent vraiment sont figees : sinon
                // on ferait tourner un volet portant deux fois le meme chiffre.
                if scene.cartes[i].en_mouvement() {
                    scene.cartes[i].figer(p);
                }
            }
        }

        scene.accorder_atlas(typo);
        scene.date = instant.date_francaise();
        scene.poser_texte();
        scene
    }

    /// La date au-dessus, la legende en dessous, toutes deux centrees.
    ///
    /// Les couleurs sont recalculees a chaque trame parce que le variateur les
    /// fait bouger ; le faconnage, lui, ne repart que si le TEXTE change — c'est
    /// `Panneau::preparer` qui s'en assure, sur l'empreinte de la ligne.
    fn poser_texte(&mut self) {
        let g = &self.geo;
        let milieu = g.largeur as f32 / 2.0;
        let niveau = self.variateur.niveau();
        self.lignes = vec![
            Ligne::simple(&self.date, Fonte::Courante, g.corps_date, palette::SECONDAIRE)
                .espacee(crate::geometrie::INTERLETTRAGE_DATE)
                .a(milieu, g.date_base)
                .ancree(Ancre::Centre),
            // DESIGN.md §7 : livre ferme, la legende devient le texte le plus
            // lumineux de sa taille, a l'endroit exact ou l'oeil descend apres
            // avoir lu l'heure. Elle suit la meme enveloppe que les chiffres,
            // sinon le mot changerait d'un coup au milieu d'un fondu de 1200 ms.
            Ligne::simple(
                &self.legende,
                Fonte::Moyenne,
                g.corps_legende,
                palette::melange(palette::LEGENDE, palette::SECONDAIRE, niveau),
            )
            .espacee(crate::geometrie::INTERLETTRAGE_LEGENDE)
            // La legende pose son HAUT DE CAPITALE a legende_haut ; sa ligne de
            // base est donc une hauteur de capitale plus bas.
            .a(milieu, g.legende_haut + CAPITALE * g.corps_legende)
            .ancree(Ancre::Centre),
        ];
    }

    /// Fige le variateur a un bout de sa course. Ne sert qu'aux captures, comme
    /// `Carte::figer` : juger « livre ferme » ne doit pas demander d'attendre
    /// les 1200 ms du fondu.
    pub fn figer_variateur(&mut self) {
        self.variateur.figer(self.lumiere_basse);
    }

    /// Ce que le chrono dicte a l'horloge : le mot de la legende, et si la
    /// lumiere doit baisser.
    ///
    /// Renvoie vrai quand le SENS du variateur vient de changer. L'appelant en a
    /// besoin : c'est a cet instant que la course de 1200 ms commence, et sa
    /// premiere trame ne doit pas mesurer le temps ecoule depuis le dernier
    /// dessin de l'horloge.
    pub fn poser_phase(&mut self, legende: &str, lumiere_basse: bool) -> bool {
        if self.legende != legende {
            self.legende = legende.to_string();
        }
        let change = self.lumiere_basse != lumiere_basse;
        self.lumiere_basse = lumiere_basse;
        change
    }

    /// Met l'atlas au diapason des cartes : rangee du haut les valeurs
    /// nouvelles, rangee du bas les anciennes.
    fn accorder_atlas(&mut self, typo: &mut Typographie) {
        for i in 0..CARTES {
            let (neuve, vieille) = (self.cartes[i].nouvelle, self.cartes[i].ancienne);
            self.atlas.poser(typo, 0, i, neuve);
            self.atlas.poser(typo, 1, i, vieille);
        }
    }

    /// Les quads a peindre.
    ///
    /// Chaque carte est coupee en deux par le pli. Le pli n'est pas PEINT : on
    /// retire aux deux moities les lignes qu'il occupe, et c'est le fond qui
    /// traverse l'encre. Un gris propre se verrait hors du chiffre et
    /// revelerait la carte ; du noir peint sur du noir serait du travail
    /// invisible.
    ///
    /// Ordre de depot : les deux moities fixes d'abord, le volet ensuite. Les
    /// quads sont OPAQUES et sans melange, donc le volet cache la moitie fixe
    /// qu'il recouvre — c'est cette occlusion qui fait la mecanique.
    fn quads(&self) -> Vec<Quad> {
        let g = &self.geo;
        let (al, ah) = (self.atlas.largeur as f32, self.atlas.hauteur as f32);
        let mut quads = Vec::with_capacity(CARTES * 3);

        for i in 0..CARTES {
            let x = g.carte_x[i];
            let cx = i as f32 * g.carte_l as f32;
            let pli = g.carte_y + g.pli as f32;
            let lumiere = if self.lumiere { 1.0 } else { 0.0 };

            // Une moitie prend dans l'atlas exactement les lignes qu'elle
            // couvre a l'ecran : un texel pour un pixel, aucun reechantillon.
            // DESIGN.md §7 : les chiffres descendent de #F2EFE9 a #8E8B86 quand
            // le livre se ferme. L'atlas n'est pas regrave — c'est un simple
            // facteur sur l'encre, calcule une fois par trame.
            let teinte = palette::teinte(palette::melange(
                palette::ENCRE,
                palette::SECONDAIRE,
                self.variateur.niveau(),
            ));

            let decoupe = |haut: u32, hauteur: u32, rangee: u32, angle: f32, lum: f32| Quad {
                rect: [
                    x,
                    g.carte_y + haut as f32,
                    g.carte_l as f32,
                    hauteur as f32,
                ],
                uv: [
                    cx / al,
                    (rangee * g.carte_h + haut) as f32 / ah,
                    (cx + g.carte_l as f32) / al,
                    (rangee * g.carte_h + haut + hauteur) as f32 / ah,
                ],
                par: [angle, pli, g.perspective, lum],
                teinte,
            };

            let etat = self.cartes[i].etat();
            let bas_fixe = if etat == Etat::Repos { NOUVELLE } else { ANCIENNE };

            // Moitie haute fixe : deja la NOUVELLE valeur.
            quads.push(decoupe(0, g.haut_h, NOUVELLE, 0.0, 0.0));
            // Moitie basse fixe : encore l'ANCIENNE tant que le volet tombe.
            quads.push(decoupe(g.bas_y, g.bas_h, bas_fixe, 0.0, 0.0));

            match etat {
                Etat::Repos => {}
                // Le volet haut porte l'ancienne valeur et tombe vers
                // l'observateur.
                Etat::Chute(a) => quads.push(decoupe(0, g.haut_h, ANCIENNE, a, lumiere)),
                // Le volet bas porte la nouvelle et se pose par-dessus la
                // moitie basse fixe.
                Etat::Pose(a) => quads.push(decoupe(g.bas_y, g.bas_h, NOUVELLE, a, lumiere)),
            }
        }
        quads
    }

    /// Fait avancer les bascules ET le variateur de `dt` millisecondes.
    pub fn avancer(&mut self, dt: f32) {
        for carte in &mut self.cartes {
            carte.avancer(dt);
        }
        self.variateur.avancer(dt, self.lumiere_basse);
    }

    pub fn en_mouvement(&self) -> bool {
        self.cartes.iter().any(|c| c.en_mouvement())
            || self.variateur.en_mouvement(self.lumiere_basse)
    }

    /// Pose l'heure. Seules les cartes dont le chiffre change basculent : les
    /// dizaines de secondes toutes les dix secondes, les heures presque jamais.
    pub fn poser_heure(&mut self, typo: &mut Typographie, instant: temps::Instant) {
        let chiffres = instant.chiffres();
        for i in 0..CARTES {
            if self.sans_volet {
                self.cartes[i].poser_sec(chiffres[i]);
            } else {
                self.cartes[i].basculer(chiffres[i]);
            }
        }
        self.accorder_atlas(typo);
        self.date = instant.date_francaise();
    }

    pub fn dessiner(
        &mut self,
        gpu: &Gpu,
        typo: &mut Typographie,
        cible: &wgpu::TextureView,
        taille: (u32, u32),
    ) {
        if self.atlas.a_redessiner() {
            self.atlas.graver(gpu, typo);
        }
        self.poser_texte();
        let Scene { panneau, lignes, .. } = self;
        panneau.preparer(typo, lignes);
        let quads = self.quads();
        self.quads.televerser(gpu, taille, &quads);

        let mut enc = gpu
            .peripherique
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("trame"),
            });
        {
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("passe principale"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: cible,
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
            self.quads.dessiner(&mut passe);
            // La date et la legende ne tournent pas : elles sont peintes
            // directement dans la fenetre, par-dessus les cartes.
            let Scene { panneau, lignes, .. } = self;
            panneau.dessiner(gpu, typo, lignes, &mut passe);
        }
        gpu.file.submit(Some(enc.finish()));
    }

    pub fn chiffres(&self) -> Vec<u8> {
        self.cartes.iter().map(|c| c.nouvelle).collect()
    }

    pub fn etats(&self) -> Vec<Etat> {
        self.cartes.iter().map(|c| c.etat()).collect()
    }
}
