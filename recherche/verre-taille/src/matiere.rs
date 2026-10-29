//! Le banc de la MATIERE : ce qui se mesure quand on demande au verre taille
//! d'etre du verre et non un fond floute.
//!
//! Il ne capture pas le bureau. Ce n'est pas un renoncement, c'est le contraire :
//! on mesure ici une GEOMETRIE — de combien le fond se deplace sous le biseau,
//! de combien les canaux se separent, ou passe le lisere — et une geometrie ne
//! se mesure pas contre un fond inconnu. Les fonds sont donc synthetiques et
//! leur reponse est connue d'avance.
//!
//! **La sonde.** Le fond porte une rampe en dents de scie de periode 96 px,
//! LINEAIRE DANS L'ESPACE LINEAIRE. Une rampe lineaire est invariante par un
//! flou symetrique : elle traverse donc la chaine sans se deformer, et la
//! valeur lue dans l'image finale dit exactement DE QUEL PIXEL DU FOND elle
//! vient. On lit le champ de deplacement dans l'image que l'oeil voit, et non
//! dans le code qui l'a produit.
//!
//! **Le temoin.** Avec les quatre parametres du biseau a zero, le nuanceur doit
//! rendre `verre.wgsl` a l'identique : deplacement nul sur tous les canaux, et
//! contraste egal a la ligne de reference de VERRE-TAILLE.md §4 — 12,00 / 4,06
//! / 2,04 pour une porte de 38. Sans ce temoin, aucun chiffre mesure ici ne
//! prouverait quoi que ce soit.

/// L'ilot deploye, aux mesures de DESIGN.md §6.
const IL: u32 = 320;
const IH: u32 = 148;
const RAYON: f32 = 48.0;
const EXPOSANT: f32 = 4.0;
/// Ce que le flou doit pouvoir mordre autour de l'ilot.
const MARGE: u32 = 64;
const SL: u32 = IL + 2 * MARGE;
const SH: u32 = IH + 2 * MARGE;
/// La periode de la sonde, et la PHASE qui la decale.
///
/// La periode donne la plage sans ambiguite : le deroulement contre la position
/// nominale est juste tant que le deplacement reste sous la demi-periode, soit
/// 48 px. La phase existe pour une raison qui a coute un tour de banc : avec
/// une periode de 64 et une marge de 64, une DENT tombait exactement sur le
/// bord gauche du galet, le flou la brouillait, et le temoin lisait 19 px de
/// deplacement la ou il devait lire zero. Une dent doit se tenir loin de ce
/// qu'on mesure.
const PERIODE: f64 = 96.0;
const PHASE: f64 = 48.0;

// ---------------------------------------------------------------------------
// sRGB
// ---------------------------------------------------------------------------

fn vers_srgb(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn depuis_srgb(c: f64) -> f64 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn octet(lineaire: f64) -> u8 {
    (vers_srgb(lineaire.clamp(0.0, 1.0)) * 255.0).round() as u8
}

fn lineaire(o: u8) -> f64 {
    depuis_srgb(o as f64 / 255.0)
}

// ---------------------------------------------------------------------------
// Les fonds
// ---------------------------------------------------------------------------

fn uni(v: u8) -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for i in 0..(SL * SH) as usize {
        p[i * 4] = v;
        p[i * 4 + 1] = v;
        p[i * 4 + 2] = v;
    }
    p
}

/// La sonde : dents de scie en x, identiques sur les trois canaux. Grise, donc
/// la saturation ne peut pas melanger les canaux entre eux — sans quoi l'ecart
/// chromatique mesure serait celui de la saturation et non celui du verre.
fn sonde() -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let v = octet(((x as f64 + 0.5 + PHASE) / PERIODE).fract());
            let i = ((y * SL + x) * 4) as usize;
            p[i] = v;
            p[i + 1] = v;
            p[i + 2] = v;
        }
    }
    p
}

/// La vraie scene de la veilleuse : du noir, et un fut de chiffre. C'est le
/// seul fond honnete pour juger le lisere, parce que c'est le seul qui existe
/// la nuit — l'horloge est noire, et sa seule lumiere est l'encre.
fn fond_horloge() -> Vec<u8> {
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let i = ((y * SL + x) * 4) as usize;
            // Deux futs verticaux de 56 px, l'encre de DESIGN.md.
            let encre = (x >= 96 && x < 152) || (x >= 268 && x < 324);
            let c = if encre { (0xF2, 0xEF, 0xE9) } else { (0, 0, 0) };
            p[i] = c.0;
            p[i + 1] = c.1;
            p[i + 2] = c.2;
        }
    }
    p
}

/// Un fond de bureau ordinaire : des bandes larges pour la couleur, des rayures
/// fines pour que le flou ait quelque chose a detruire.
fn fond_bureau() -> Vec<u8> {
    let bandes: [(u8, u8, u8); 6] = [
        (0xE0, 0xE4, 0xE8),
        (0xE8, 0xEF, 0xF2),
        (0x40, 0x30, 0xC0),
        (0x30, 0xA0, 0xE0),
        (0xD8, 0xDC, 0xE0),
        (0xF0, 0xF0, 0xF0),
    ];
    let mut p = vec![255u8; (SL * SH * 4) as usize];
    for y in 0..SH {
        for x in 0..SL {
            let b = bandes[((x * 6) / SL).min(5) as usize];
            let raie = (x % 6) < 3;
            let c = if raie { (0x10, 0x10, 0x10) } else { b };
            let i = ((y * SL + x) * 4) as usize;
            p[i] = c.0;
            p[i + 1] = c.1;
            p[i + 2] = c.2;
        }
    }
    p
}

// ---------------------------------------------------------------------------
// Les mesures
// ---------------------------------------------------------------------------

fn luminance_relative(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * lineaire(r) + 0.7152 * lineaire(g) + 0.0722 * lineaire(b)
}

fn contraste(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let la = luminance_relative(a.0, a.1, a.2);
    let lb = luminance_relative(b.0, b.1, b.2);
    let (h, bas) = if la > lb { (la, lb) } else { (lb, la) };
    (h + 0.05) / (bas + 0.05)
}

/// L'energie de haute frequence : la moyenne des sauts entre pixels voisins.
fn haute_frequence(px: &[u8], l: u32, h: u32) -> f64 {
    let mut s = 0.0;
    let mut n = 0.0;
    for y in 0..h {
        for x in 1..l {
            let a = ((y * l + x) * 4) as usize;
            let b = ((y * l + x - 1) * 4) as usize;
            for c in 0..3 {
                s += (px[a + c] as f64 - px[b + c] as f64).abs();
                n += 1.0;
            }
        }
    }
    s / n
}

/// La moyenne de la zone centrale, la ou l'encre se pose. Elle exclut de plein
/// droit le biseau : c'est exactement la question « le bord a-t-il coute la
/// lisibilite du texte ».
fn moyenne_centre(px: &[u8], l: u32, h: u32) -> (u8, u8, u8) {
    let (x0, x1) = (l / 2 - 60, l / 2 + 60);
    let (y0, y1) = (h / 2 - 30, h / 2 + 30);
    let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * l + x) * 4) as usize;
            r += px[i] as u64;
            g += px[i + 1] as u64;
            b += px[i + 2] as u64;
            n += 1;
        }
    }
    ((r / n) as u8, (g / n) as u8, (b / n) as u8)
}

/// Le deplacement lu par la sonde, canal par canal, le long du bord GAUCHE.
///
/// On lit la valeur, on la convertit en position dans le fond, et on la deroule
/// contre la position nominale : la dent fait 96 px et le deplacement attendu
/// ne depasse pas 48, donc le tour est sans ambiguite.
///
/// Renvoie, pour chaque x de 0 a `jusqu_a`, les trois deplacements en pixels,
/// comptes POSITIFS vers l'exterieur du galet.
fn deplacement(px: &[u8], jusqu_a: u32) -> Vec<(f64, f64, f64)> {
    let mut sortie = Vec::new();
    // Le bord gauche est droit entre y = 48 et y = 100 (rayon 48, hauteur 148).
    // On moyenne sur vingt lignes centrees, pour que le bruit de quantification
    // tombe d'un facteur racine de vingt.
    let lignes: Vec<u32> = (IH / 2 - 10..IH / 2 + 10).collect();
    for x in 0..jusqu_a {
        let mut acc = [0.0f64; 3];
        let mut n = 0.0;
        for &y in &lignes {
            let i = ((y * IL + x) * 4) as usize;
            // Le pixel x lit le fond en x + MARGE + 0,5 : c'est le centre du
            // texel, et l'oublier laisserait un demi-pixel dans chaque chiffre.
            let nominal = (x + MARGE) as f64 + 0.5;
            for c in 0..3 {
                let v = lineaire(px[i + c]);
                let fin = v * PERIODE - PHASE;
                let tours = ((nominal - fin) / PERIODE).round();
                let source = fin + tours * PERIODE;
                // Vers la gauche = vers l'exterieur = positif.
                acc[c] += nominal - source;
            }
            n += 1.0;
        }
        sortie.push((acc[0] / n, acc[1] / n, acc[2] / n));
    }
    sortie
}

/// La distance signee au galet, en Rust. Elle double celle du nuanceur, et
/// c'est voulu : une mesure qui demanderait au nuanceur ou est son propre bord
/// ne prouverait rien. Ici, le banc sait ou est le bord independamment.
fn distance(px_x: f32, px_y: f32) -> f32 {
    let p = (px_x + 0.5 - IL as f32 / 2.0, px_y + 0.5 - IH as f32 / 2.0);
    let q = (
        p.0.abs() - IL as f32 / 2.0 + RAYON,
        p.1.abs() - IH as f32 / 2.0 + RAYON,
    );
    if q.0 > 0.0 && q.1 > 0.0 {
        let n = EXPOSANT;
        let f = (q.0.powf(n) + q.1.powf(n)).powf(1.0 / n);
        let g = ((q.0.powf(n - 1.0)).powi(2) + (q.1.powf(n - 1.0)).powi(2)).sqrt()
            / f.powf(n - 1.0);
        (f - RAYON) / g
    } else {
        let m = (q.0.max(0.0), q.1.max(0.0));
        (m.0 * m.0 + m.1 * m.1).sqrt() + q.0.max(q.1).min(0.0) - RAYON
    }
}

/// Le lisere vu par le bord DROIT et par la DIAGONALE du coin, en niveaux.
///
/// Les deux doivent donner la meme chose. S'ils different, le lisere n'est pas
/// une propriete de la matiere mais un accident de la grille de pixels : il
/// s'allume la ou un centre de pixel tombe sur l'arete et s'eteint ailleurs.
/// C'est le genre de defaut qu'on ne voit pas sur une capture de face et qui
/// fait scintiller un coin des que l'ilot bouge d'un demi-pixel.
fn lisere(px: &[u8], bande: f32) -> (u8, u8) {
    let mut droit = 0u8;
    let mut coin = 0u8;
    for y in 0..IH {
        for x in 0..IL {
            let d = distance(x as f32, y as f32);
            // La fenetre commence a d = -0,45 : c'est la que le masque atteint
            // 1 — smoothstep(-0,5 ; 0,5) vaut zero des -0,5 — donc plus rien du
            // fond exterieur ne traverse et ce qu'on lit est de la matiere. Sur
            // un bord droit, les centres de pixels tombent exactement sur
            // -0,5 / -1,5 / -2,5 ; dans un coin, ils tombent ou ils veulent, et
            // c'est toute la question.
            if d > -0.45 || d < -bande {
                continue;
            }
            let v = px[((y * IL + x) * 4 + 1) as usize];
            // Le coin : les deux coordonnees sont dans la zone de courbure.
            let dans_coin = (x as f32) < RAYON && (y as f32) < RAYON
                || (x as f32) < RAYON && (y as f32) > IH as f32 - RAYON
                || (x as f32) > IL as f32 - RAYON && (y as f32) < RAYON
                || (x as f32) > IL as f32 - RAYON && (y as f32) > IH as f32 - RAYON;
            if dans_coin {
                coin = coin.max(v);
            } else {
                droit = droit.max(v);
            }
        }
    }
    (droit, coin)
}

/// L'energie de haute frequence dans une BANDE verticale, comptee depuis le
/// bord gauche. C'est la mesure qui tranche la question de l'orfevre : « un
/// motif qui disparait n'est pas une preuve de verre, c'est une preuve
/// d'opacite ». Si le bord ne remet pas de structure la ou le flou en a
/// enleve, il n'y a rien a voir, quelle que soit la physique du calcul.
fn hf_bande(px: &[u8], l: u32, h: u32, x0: u32, x1: u32) -> f64 {
    let mut somme = 0.0;
    let mut n = 0.0;
    for y in h / 2 - 30..h / 2 + 30 {
        for x in x0.max(1)..x1 {
            let a = ((y * l + x) * 4) as usize;
            let b = ((y * l + x - 1) * 4) as usize;
            for c in 0..3 {
                somme += (px[a + c] as f64 - px[b + c] as f64).abs();
                n += 1.0;
            }
        }
    }
    somme / n
}

/// La frange coloree, en NIVEAUX.
///
/// Le deplacement chromatique se compte en pixels, mais ce que l'oeil recoit
/// est une COULEUR : l'ecart entre le canal le plus haut et le plus bas, la ou
/// il est le pire. Le fond de la sonde etant gris, tout ecart vient de la
/// dispersion et de rien d'autre. Un ecart de zero niveau veut dire que la
/// frange n'existe pas dans l'image, quel que soit le calcul qui l'a voulue.
fn frange(px: &[u8], l: u32, h: u32) -> u8 {
    let mut pire = 0u8;
    for y in 0..h {
        for x in 0..l {
            let i = ((y * l + x) * 4) as usize;
            let hi = px[i].max(px[i + 1]).max(px[i + 2]);
            let lo = px[i].min(px[i + 1]).min(px[i + 2]);
            pire = pire.max(hi - lo);
        }
    }
    pire
}

/// Le profil de clarte le long d'une ligne horizontale, du bord vers le centre.
fn profil(px: &[u8], jusqu_a: u32) -> Vec<f64> {
    let y = IH / 2;
    (0..jusqu_a)
        .map(|x| {
            let i = ((y * IL + x) * 4) as usize;
            luminance_relative(px[i], px[i + 1], px[i + 2])
        })
        .collect()
}

fn ecrire_png(chemin: &str, px: &[u8], l: u32, h: u32) {
    let f = std::fs::File::create(chemin).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), l, h);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(px).unwrap();
}

// ---------------------------------------------------------------------------
// Le rendu
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Reglages {
    taille: [f32; 4],
    flou: [f32; 4],
    forme: [f32; 4],
    teinte: [f32; 4],
    reglages: [f32; 4],
    biseau: [f32; 4],
    bord: [f32; 4],
}

/// Une direction artistique, telle qu'un implementeur la recevrait.
#[derive(Clone, Copy)]
struct Direction {
    nom: &'static str,
    sigma: f32,
    teinte: f32,
    saturation: f32,
    /// largeur du biseau, px
    largeur: f32,
    indice: f32,
    dispersion: f32,
    /// epaisseur de la dalle, px
    epaisseur: f32,
    /// hauteur du biseau rapportee a sa largeur
    galbe: f32,
    lisere: f32,
    clarte: f32,
    /// 1 = le lisere est integre sur la largeur du pixel, 0 = echantillonne
    /// au seul centre.
    integre: f32,
}

const TEMOIN: Direction = Direction {
    nom: "temoin (verre.wgsl)",
    sigma: 18.0,
    teinte: 0.80,
    saturation: 1.35,
    largeur: 0.0,
    indice: 1.5,
    dispersion: 0.0,
    epaisseur: 0.0,
    galbe: 0.0,
    lisere: 0.0,
    clarte: 0.0,
    integre: 1.0,
};

struct Verre {
    peripherique: wgpu::Device,
    file: wgpu::Queue,
    disposition: wgpu::BindGroupLayout,
    tuyau_flou: wgpu::RenderPipeline,
    tuyau_composer: wgpu::RenderPipeline,
    echantillonneur: wgpu::Sampler,
}

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

impl Verre {
    fn nouveau() -> (Verre, String) {
        let instance = wgpu::Instance::default();
        let adaptateur = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .expect("aucun adaptateur");
        let nom = adaptateur.get_info().name;
        let (peripherique, file) = pollster::block_on(adaptateur.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("matiere"),
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adaptateur.limits()),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            },
        ))
        .expect("peripherique refuse");
        peripherique.on_uncaptured_error(std::sync::Arc::new(|e| {
            eprintln!("!! wgpu : {e}");
        }));

        let module = peripherique.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("matiere"),
            source: wgpu::ShaderSource::Wgsl(include_str!("matiere.wgsl").into()),
        });

        let texture_entree = |b: u32| wgpu::BindGroupLayoutEntry {
            binding: b,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let disposition =
            peripherique.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("disposition matiere"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    texture_entree(2),
                    texture_entree(3),
                ],
            });

        let agencement = peripherique.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("agencement matiere"),
            bind_group_layouts: &[Some(&disposition)],
            immediate_size: 0,
        });

        let faire = |entree: &str, nom: &str| {
            peripherique.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(nom),
                layout: Some(&agencement),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("sommet"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entree),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let tuyau_flou = faire("flou", "flou");
        let tuyau_composer = faire("composer", "composer");

        let echantillonneur = peripherique.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("echantillonneur"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        (
            Verre {
                peripherique,
                file,
                disposition,
                tuyau_flou,
                tuyau_composer,
                echantillonneur,
            },
            nom,
        )
    }

    fn texture(&self, l: u32, h: u32, usage: wgpu::TextureUsages) -> wgpu::Texture {
        self.peripherique.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: l,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage,
            view_formats: &[],
        })
    }

    fn groupe(
        &self,
        r: &Reglages,
        source: &wgpu::TextureView,
        net: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        use wgpu::util::DeviceExt;
        let tampon = self
            .peripherique
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("reglages"),
                contents: bytemuck::bytes_of(r),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        self.peripherique.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.disposition,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tampon.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.echantillonneur),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(net),
                },
            ],
        })
    }

    /// Toute la chaine. `perceptif` vaut 1 : la teinte se pose sur les octets,
    /// c'est la conclusion de VERRE-TAILLE.md §3 et elle ne se rediscute pas.
    /// Renvoie l'image finale et le temps GPU, en millisecondes.
    fn tailler(&self, fond: &[u8], d: &Direction) -> (Vec<u8>, f64) {
        self.chaine(fond, d, true)
    }

    /// Un seul tour complet, sans boucle de mesure.
    fn tailler_une_fois(&self, fond: &[u8], d: &Direction) -> Vec<u8> {
        self.chaine(fond, d, false).0
    }

    fn chaine(&self, fond: &[u8], d: &Direction, mesurer: bool) -> (Vec<u8>, f64) {
        let brut = self.texture(
            SL,
            SH,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        self.file.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &brut,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            fond,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SL * 4),
                rows_per_image: Some(SH),
            },
            wgpu::Extent3d {
                width: SL,
                height: SH,
                depth_or_array_layers: 1,
            },
        );

        let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
        let inter = self.texture(SL, SH, usage);
        let floute = self.texture(SL, SH, usage);
        let fini = self.texture(
            IL,
            IH,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );

        let v_brut = brut.create_view(&Default::default());
        let v_inter = inter.create_view(&Default::default());
        let v_floute = floute.create_view(&Default::default());
        let v_fini = fini.create_view(&Default::default());

        let taps = (d.sigma * 3.0).ceil();
        let base = Reglages {
            taille: [SL as f32, SH as f32, SL as f32, SH as f32],
            flou: [1.0, 0.0, d.sigma, taps],
            forme: [IL as f32 / 2.0, IH as f32 / 2.0, RAYON, EXPOSANT],
            teinte: [0.0, 0.0, 0.0, d.teinte],
            reglages: [d.saturation, MARGE as f32, 1.0, 1.0],
            biseau: [d.largeur, d.indice, d.dispersion, d.epaisseur],
            bord: [d.lisere, d.galbe, d.clarte, d.integre],
        };
        let mut vertical = base;
        vertical.flou = [0.0, 1.0, d.sigma, taps];
        let mut composer = base;
        composer.taille = [IL as f32, IH as f32, SL as f32, SH as f32];

        let g1 = self.groupe(&base, &v_brut, &v_brut);
        let g2 = self.groupe(&vertical, &v_inter, &v_brut);
        let g3 = self.groupe(&composer, &v_floute, &v_brut);
        let travail: [(&wgpu::BindGroup, &wgpu::TextureView, &wgpu::RenderPipeline); 3] = [
            (&g1, &v_inter, &self.tuyau_flou),
            (&g2, &v_floute, &self.tuyau_flou),
            (&g3, &v_fini, &self.tuyau_composer),
        ];

        let encoder = |verre: &Verre| {
            let mut enc = verre
                .peripherique
                .create_command_encoder(&Default::default());
            for (groupe, cible, tuyau) in travail {
                let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
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
                    multiview_mask: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                passe.set_pipeline(tuyau);
                passe.set_bind_group(0, groupe, &[]);
                passe.draw(0..3, 0..1);
            }
            enc.finish()
        };

        let mut median = 0.0;
        if mesurer {
            // Une premiere soumission pour que rien de la compilation ni de
            // l'allocation ne tombe dans la mesure.
            self.file.submit(Some(encoder(self)));
            self.peripherique
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();
            let mut temps = Vec::new();
            for _ in 0..30 {
                let t0 = std::time::Instant::now();
                self.file.submit(Some(encoder(self)));
                self.peripherique
                    .poll(wgpu::PollType::wait_indefinitely())
                    .unwrap();
                temps.push(t0.elapsed().as_secs_f64() * 1000.0);
            }
            temps.sort_by(|a, b| a.partial_cmp(b).unwrap());
            median = temps[temps.len() / 2];
        } else {
            self.file.submit(Some(encoder(self)));
        }

        // La relecture, apres la mesure : elle ne doit pas la polluer.
        let ligne = IL * 4;
        let alignee = ligne.div_ceil(256) * 256;
        let lecture = self.peripherique.create_buffer(&wgpu::BufferDescriptor {
            label: Some("relecture"),
            size: (alignee * IH) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self
            .peripherique
            .create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &fini,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &lecture,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(alignee),
                    rows_per_image: Some(IH),
                },
            },
            wgpu::Extent3d {
                width: IL,
                height: IH,
                depth_or_array_layers: 1,
            },
        );
        self.file.submit(Some(enc.finish()));

        let tranche = lecture.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        tranche.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.peripherique
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        rx.recv().unwrap().unwrap();
        let vue = tranche.get_mapped_range().unwrap();
        let mut px = Vec::with_capacity((ligne * IH) as usize);
        for y in 0..IH {
            let dep = (y * alignee) as usize;
            px.extend_from_slice(&vue[dep..dep + ligne as usize]);
        }
        drop(vue);
        lecture.unmap();
        (px, median)
    }
}

impl Verre {
    /// Le meme travail, mesure a la PORTEE du banc `verre` : allocation des
    /// textures, televersement de la region, rendu, relecture vers le CPU.
    ///
    /// Elle existe pour une seule raison : expliquer un ecart. Le rendu seul se
    /// mesure ici autour de 0,2 ms, et VERRE-TAILLE.md §6 annonce 0,82 ms pour
    /// « deux flous et la composition ». Les deux sont vrais, ils ne comptent
    /// pas la meme chose, et c'est la seconde qui a ete mal nommee.
    fn aller_retour(&self, fond: &[u8], d: &Direction) -> f64 {
        let mut t = Vec::new();
        for _ in 0..20 {
            let t0 = std::time::Instant::now();
            let _ = self.tailler_une_fois(fond, d);
            t.push(t0.elapsed().as_secs_f64() * 1000.0);
        }
        t.sort_by(|a, b| a.partial_cmp(b).unwrap());
        t[t.len() / 2]
    }
}

// ---------------------------------------------------------------------------

fn main() {
    let (verre, adaptateur) = Verre::nouveau();
    println!("adaptateur : {adaptateur}");
    std::fs::create_dir_all("captures/matiere").unwrap();

    // Les trois directions. Leurs valeurs sont des CIBLES posees par la
    // direction artistique ; ce banc est la pour dire ce qu'elles rendent.
    let dalle = Direction {
        nom: "A. dalle",
        sigma: 18.0,
        teinte: 0.80,
        saturation: 1.35,
        largeur: 8.0,
        indice: 1.50,
        dispersion: 0.020,
        epaisseur: 10.0,
        galbe: 1.0,
        lisere: 1.0,
        clarte: 0.35,
        integre: 1.0,
};
    let lentille = Direction {
        nom: "B. lentille",
        sigma: 18.0,
        teinte: 0.80,
        saturation: 1.35,
        largeur: 28.0,
        indice: 1.70,
        dispersion: 0.060,
        epaisseur: 26.0,
        galbe: 1.0,
        lisere: 0.6,
        clarte: 0.0,
            integre: 1.0,
};
    let depoli = Direction {
        nom: "C. depoli",
        sigma: 28.0,
        teinte: 0.80,
        saturation: 1.10,
        largeur: 0.0,
        indice: 1.50,
        dispersion: 0.0,
        epaisseur: 0.0,
        galbe: 0.0,
        lisere: 0.0,
        clarte: 0.0,
        integre: 1.0,
};
    let directions = [TEMOIN, dalle, lentille, depoli];

    let sonde_px = sonde();
    let clair = uni(226);
    let sombre = uni(37);
    let horloge = fond_horloge();
    let bureau = fond_bureau();

    // -----------------------------------------------------------------------
    println!();
    println!("== 1. LE CHAMP DE DEPLACEMENT, lu par la sonde ==");
    println!("Sonde en dents de scie, periode 96 px, lineaire dans l'espace lineaire.");
    println!("Flou ramene a sigma 1 : le deplacement est une geometrie, il ne depend");
    println!("pas du flou, et la dent de scie doit survivre pour etre lue.");
    println!("Deplacement compte POSITIF vers l'exterieur du galet, bord gauche,");
    println!("moyenne de vingt lignes.");
    println!();
    println!("x = 0 n'est pas porte au tableau : le masque y vaut une demi-couverture,");
    println!("donc la moitie du pixel est du fond intact et le deplacement lu y est");
    println!("divise par deux. Le premier pixel pleinement dans la matiere est x = 1.");
    println!();
    println!("  direction         x=1    x=2    x=3    x=4    x=6    x=8   x=12   x=16   x=24   x=32");
    println!("  ------------------------------------------------------------------------------------");
    let mut resume: Vec<(&str, f64, usize)> = Vec::new();
    for d in directions {
        let mut geo = d;
        geo.sigma = 1.0;
        geo.teinte = 0.0;
        geo.saturation = 1.0;
        let (px, _) = verre.tailler(&sonde_px, &geo);
        let dep = deplacement(&px, 40);
        let mut l = format!("  {:16}", d.nom);
        for x in [1usize, 2, 3, 4, 6, 8, 12, 16, 24, 32] {
            l += &format!(" {:6.2}", dep[x].1);
        }
        println!("{l}");
        ecrire_png(
            &format!("captures/matiere/sonde-{}.png", d.nom.split('.').next().unwrap()),
            &px,
            IL,
            IH,
        );
        // Ce que l'implementeur doit viser, et le metrologue verifier : de
        // combien le fond bouge au plus, et sur quelle largeur ca se voit.
        // « Se voit » a une definition : un demi-pixel, en dessous duquel la
        // grille des pixels ne peut plus porter le deplacement.
        let pic = dep[1..].iter().map(|t| t.1).fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
        let large = dep[1..].iter().rposition(|t| t.1.abs() >= 0.5).map(|i| i + 2).unwrap_or(0);
        resume.push((d.nom, pic, large));
    }
    println!();
    println!("  direction        deplacement max   largeur ou il depasse 0,5 px");
    println!("  -----------------------------------------------------------------");
    for (nom, pic, large) in &resume {
        println!("  {nom:16} {pic:10.2} px     {large:3} px");
    }

    // -----------------------------------------------------------------------
    println!();
    println!("== 2. L'ECART CHROMATIQUE, en pixels ==");
    println!("Le meme essai, canal rouge moins canal bleu. Le bleu refracte plus,");
    println!("donc l'ecart est negatif quand la dispersion fait son travail.");
    println!();
    println!("  direction        max |R-B|   la ou            R a ce point   B a ce point");
    println!("  ---------------------------------------------------------------------------");
    for d in directions {
        let mut geo = d;
        geo.sigma = 1.0;
        geo.teinte = 0.0;
        geo.saturation = 1.0;
        let (px, _) = verre.tailler(&sonde_px, &geo);
        let dep = deplacement(&px, 40);
        let mut pire = 0.0f64;
        let mut ou = 0usize;
        for (i, t) in dep.iter().enumerate().skip(1) {
            if (t.0 - t.2).abs() > pire.abs() {
                pire = t.0 - t.2;
                ou = i;
            }
        }
        println!(
            "  {:16} {:8.2} px   x = {:2} px      {:8.2} px    {:8.2} px",
            d.nom, pire, ou, dep[ou].0, dep[ou].2
        );
    }

    println!();
    println!("Combien de dispersion faut-il pour qu'une frange se VOIE ? Un ecart");
    println!("sous un pixel ne peut pas se poser sur la grille. Balayage sur la");
    println!("direction A, tout le reste egal :");
    println!();
    println!("  dispersion   max |R-B|   la ou     frange vue");
    println!("  --------------------------------------------------");
    for disp in [0.02f32, 0.05, 0.10, 0.20, 0.35, 0.50] {
        let mut geo = dalle;
        geo.sigma = 1.0;
        geo.teinte = 0.0;
        geo.saturation = 1.0;
        geo.dispersion = disp;
        let (px, _) = verre.tailler(&sonde_px, &geo);
        let dep = deplacement(&px, 40);
        let mut pire = 0.0f64;
        let mut ou = 0usize;
        for (i, t) in dep.iter().enumerate().skip(1) {
            if (t.0 - t.2).abs() > pire.abs() {
                pire = t.0 - t.2;
                ou = i;
            }
        }
        let f = frange(&px, IL, IH);
        if pire == 0.0 {
            println!("  {disp:8.2}      aucune      -        {f:3} niveaux");
        } else {
            println!("  {disp:8.2}   {pire:8.2} px   x = {ou:2} px  {f:3} niveaux");
        }
    }

    println!();
    println!("Le biseau se regle par DEUX nombres, et ils ne font pas la meme chose :");
    println!("sa LARGEUR dit sur combien de pixels la deformation s'etale, son");
    println!("EPAISSEUR dit de combien elle deplace. Balayage, tout le reste egal");
    println!("aux valeurs de la direction A :");
    println!();
    println!("  largeur  epaisseur   deplacement max   au-dela de 0,5 px sur");
    println!("  -------------------------------------------------------------");
    for (larg, ep) in [
        (6.0f32, 6.0f32), (6.0, 12.0),
        (8.0, 10.0), (8.0, 20.0), (8.0, 30.0),
        (12.0, 20.0), (12.0, 30.0),
        (16.0, 30.0), (16.0, 45.0),
    ] {
        let mut geo = dalle;
        geo.sigma = 1.0;
        geo.teinte = 0.0;
        geo.saturation = 1.0;
        geo.largeur = larg;
        geo.epaisseur = ep;
        let (px, _) = verre.tailler(&sonde_px, &geo);
        let dep = deplacement(&px, 40);
        let pic = dep[1..].iter().map(|t| t.1).fold(0.0f64, |a, b| if b.abs() > a.abs() { b } else { a });
        let large = dep[1..].iter().rposition(|t| t.1.abs() >= 0.5).map(|i| i + 2).unwrap_or(0);
        println!("  {larg:5.0}px  {ep:7.0}px   {pic:12.2} px   {large:14} px");
    }

    // -----------------------------------------------------------------------
    println!();
    println!("== 3. LA LISIBILITE AU CENTRE ==");
    println!("Fonds unis 226 et 37, ceux de l'orfevre. Reglages de production.");
    println!("La ligne du temoin DOIT redonner 45 / 7, porte 38, et 12,00 / 4,06 / 2,04.");
    println!();
    println!("  direction        verre/clair  verre/sombre  porte   F2EFE9   8E8B86   5E5B57    hf");
    println!("  --------------------------------------------------------------------------------------");
    for d in directions {
        let (pc, _) = verre.tailler(&clair, &d);
        let (ps, _) = verre.tailler(&sombre, &d);
        let mc = moyenne_centre(&pc, IL, IH);
        let ms = moyenne_centre(&ps, IL, IH);
        println!(
            "  {:16}  ({:3},{:3},{:3})  ({:3},{:3},{:3})  {:5}  {:7.2}  {:7.2}  {:7.2}  {:5.2}",
            d.nom,
            mc.0,
            mc.1,
            mc.2,
            ms.0,
            ms.1,
            ms.2,
            mc.0 as i32 - ms.0 as i32,
            contraste((0xF2, 0xEF, 0xE9), mc),
            contraste((0x8E, 0x8B, 0x86), mc),
            contraste((0x5E, 0x5B, 0x57), mc),
            haute_frequence(&pc, IL, IH),
        );
    }

    // -----------------------------------------------------------------------
    println!();
    println!("== 4. LE PROFIL DE CLARTE DU BORD ==");
    println!("Luminance relative le long de la ligne mediane, du bord vers le centre.");
    println!("Fond uni 226 : le pire cas pour un lisere, puisque tout ce qui peut se");
    println!("reflechir est allume. La marge interieure du texte est a 24 px");
    println!("(DESIGN.md §6) : un lisere qui depasse 24 px touche l'encre.");
    println!();
    println!("  direction        x=0    x=1    x=2    x=3    x=4    x=6    x=8   x=12   x=20   x=24   plat");
    println!("  ------------------------------------------------------------------------------------------");
    for d in directions {
        let (pc, _) = verre.tailler(&clair, &d);
        let pr = profil(&pc, 80);
        let mut l = format!("  {:16}", d.nom);
        for x in [0usize, 1, 2, 3, 4, 6, 8, 12, 20, 24] {
            l += &format!(" {:6.3}", pr[x]);
        }
        l += &format!(" {:6.3}", pr[79]);
        println!("{l}");
    }
    println!();
    println!("Le meme, sur fond uni 37 — la nuit, l'ilot sur une horloge noire :");
    println!("  direction        x=0    x=1    x=2    x=3    x=4    x=6    x=8   x=12   x=20   x=24   plat");
    println!("  ------------------------------------------------------------------------------------------");
    for d in directions {
        let (ps, _) = verre.tailler(&sombre, &d);
        let pr = profil(&ps, 80);
        let mut l = format!("  {:16}", d.nom);
        for x in [0usize, 1, 2, 3, 4, 6, 8, 12, 20, 24] {
            l += &format!(" {:6.3}", pr[x]);
        }
        l += &format!(" {:6.3}", pr[79]);
        println!("{l}");
    }

    // -----------------------------------------------------------------------
    println!();
    println!("== 5. CE QUE CA COUTE ==");
    println!("Deux flous et la composition, sur le GPU, mediane de trente soumissions.");
    println!("Le budget d'une trame a 144 Hz est de 6,94 ms ; la loi du projet en");
    println!("reserve 2,00 ms a l'ensemble.");
    println!();
    println!("  direction        rendu seul   aller-retour complet");
    println!("  -----------------------------------------------------");
    for d in directions {
        let (_, ms) = verre.tailler(&bureau, &d);
        let tour = verre.aller_retour(&bureau, &d);
        println!("  {:16} {:7.3} ms  {:14.3} ms", d.nom, ms, tour);
    }
    println!();
    println!("La seconde colonne mesure ce que mesurait le banc `verre` : allocation");
    println!("des quatre textures, televersement de la region, rendu, PUIS relecture");
    println!("vers le CPU. VERRE-TAILLE.md §6 l'appelait « deux flous et la");
    println!("composition, sur le GPU » et l'annoncait a 0,82 ms. La premiere colonne");
    println!("est ce travail-la, seul. L'application ne fait ni relecture ni");
    println!("allocation par trame : c'est la premiere colonne qui la concerne.");

    // -----------------------------------------------------------------------
    println!();
    println!("== 5bis. LE LISERE TIENT-IL DANS LES COINS ? ==");
    println!("Valeur la plus haute entre 0,6 et 6 px sous l'arete — le ruban");
    println!("d'anticrenelage est exclu. Fond uni 226, le plat du verre y vaut 45.");
    println!("Le bord droit et le coin doivent donner la MEME chose : un lisere qui");
    println!("ne vaut que sur les bords droits est un accident de grille, pas de la");
    println!("matiere. L'ecart est rapporte a la hauteur du lisere au-dessus de 45.");
    println!();
    println!("  direction        lisere large   bord droit   coin   ecart");
    println!("  ------------------------------------------------------------");
    for d in [dalle, lentille] {
        for i in [0.0f32, 1.0, 1.5, 2.0, 3.0] {
            let mut v = d;
            v.integre = i;
            let (px, _) = verre.tailler(&clair, &v);
            let (dr, co) = lisere(&px, 6.0);
            let ecart = if dr.max(co) == 45 {
                0.0
            } else {
                100.0 * (dr as f64 - co as f64).abs() / (dr.max(co) as f64 - 45.0)
            };
            println!(
                "  {:16} {:8.1} px   {:8}   {:4}   {:6.0} %",
                d.nom, i, dr, co, ecart
            );
        }
    }

    println!();
    println!("Et ce que le lisere PESE, en niveaux, selon sa force et sa largeur.");
    println!("Direction A, tout le reste egal. « pic » est la valeur la plus haute");
    println!("dans la matiere ; le plat du verre vaut 45 sur fond clair, 7 sur fond");
    println!("sombre, et 10 a 15 sur la scene de nuit.");
    println!();
    println!("  force  largeur   pic / clair   pic / sombre   pic / horloge");
    println!("  ----------------------------------------------------------------");
    for (force, larg) in [
        (1.0f32, 0.0f32), (0.5, 0.0),
        (0.25, 1.0), (0.15, 1.0), (0.10, 1.0),
        (0.25, 1.5), (0.15, 1.5),
        (0.25, 2.0), (0.15, 2.0),
    ] {
        let mut v = dalle;
        v.lisere = force;
        v.integre = larg;
        let pic = |fond: &Vec<u8>| -> u8 {
            let (px, _) = verre.tailler(fond, &v);
            let mut m = 0u8;
            for y in 0..IH {
                for x in 0..IL {
                    if distance(x as f32, y as f32) <= -0.45 {
                        m = m.max(px[((y * IL + x) * 4 + 1) as usize]);
                    }
                }
            }
            m
        };
        println!(
            "  {:5.2}  {:5.1}px   {:11}   {:12}   {:13}",
            force,
            larg,
            pic(&clair),
            pic(&sombre),
            pic(&horloge)
        );
    }

    // -----------------------------------------------------------------------
    println!();
    println!("== 6. CE QUE LE BORD REMET, ET CE QUE LE CENTRE A PERDU ==");
    println!("Energie de haute frequence par bande, fond de bureau raye. Le fond nu");
    println!("en porte {:.2}. Le flou a sigma 18 l'efface ; la question est de savoir", haute_frequence(&bureau, SL, SH));
    println!("si le bord en remet, et sur quelle largeur.");
    println!();
    println!("  direction        bande 1-8 px   bande 8-24 px   centre (>=24 px)");
    println!("  ---------------------------------------------------------------------");
    for d in directions {
        let (px, _) = verre.tailler(&bureau, &d);
        println!(
            "  {:16} {:10.2}   {:13.2}   {:16.2}",
            d.nom,
            hf_bande(&px, IL, IH, 1, 8),
            hf_bande(&px, IL, IH, 8, 24),
            hf_bande(&px, IL, IH, 24, IL / 2),
        );
    }
    println!();
    println!("Qui, du biseau ou du lisere, remet cette structure ? Direction A,");
    println!("chaque ingredient seul. C'est l'essai qui juge a l'image la these");
    println!("d'OPTIQUE.md §8 : refracter une image DEJA FLOUTEE ne se voit presque");
    println!("pas, et ce qui se voit vient de la couche NETTE.");
    println!();
    println!("  ingredient                       bande 1-8 px   centre");
    println!("  ----------------------------------------------------------");
    for (nom, refr, lis, cla) in [
        ("rien (le verre d'aujourd'hui)", false, 0.0f32, 0.0f32),
        ("le biseau seul, sur la floutee", true, 0.0, 0.0),
        ("le lisere seul", false, 1.0, 0.0),
        ("la clarte seule (couche nette)", false, 0.0, 0.35),
        ("biseau + clarte", true, 0.0, 0.35),
        ("tout", true, 1.0, 0.35),
    ] {
        let mut v = dalle;
        if !refr {
            v.epaisseur = 0.0;
        }
        v.lisere = lis;
        v.clarte = cla;
        if !refr && lis == 0.0 && cla == 0.0 {
            v.largeur = 0.0;
        }
        let (px, _) = verre.tailler(&bureau, &v);
        println!(
            "  {:32} {:12.2}   {:6.2}",
            nom,
            hf_bande(&px, IL, IH, 1, 8),
            hf_bande(&px, IL, IH, 24, IL / 2)
        );
    }

    println!();
    println!("Et ce que la CLARTE AU BORD y fait a elle seule, direction A, tout le");
    println!("reste egal :");
    println!();
    println!("  clarte    bande 1-8 px   centre   F2EFE9 au centre");
    println!("  ------------------------------------------------------");
    for c in [0.0f32, 0.15, 0.35, 0.60, 1.00] {
        let mut v = dalle;
        v.clarte = c;
        let (px, _) = verre.tailler(&bureau, &v);
        let (pc, _) = verre.tailler(&clair, &v);
        println!(
            "  {:5.2}  {:12.2}   {:6.2}   {:16.2}",
            c,
            hf_bande(&px, IL, IH, 1, 8),
            hf_bande(&px, IL, IH, 24, IL / 2),
            contraste((0xF2, 0xEF, 0xE9), moyenne_centre(&pc, IL, IH)),
        );
    }

    // -----------------------------------------------------------------------
    println!();
    println!("== 7. LES PLANCHES ==");
    for d in directions {
        let court = d.nom.split('.').next().unwrap();
        for (nom, fond) in [
            ("horloge", &horloge),
            ("bureau", &bureau),
            ("clair", &clair),
            ("sombre", &sombre),
        ] {
            let (px, _) = verre.tailler(fond, &d);
            ecrire_png(
                &format!("captures/matiere/{court}-{nom}.png"),
                &px,
                IL,
                IH,
            );
        }
    }
    println!("  ecrites dans captures/matiere/");
}
