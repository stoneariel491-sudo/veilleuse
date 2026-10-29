//! Le verre taille : on ne demande plus le verre a Windows, on le fait.
//!
//! La chaine, et pourquoi chaque maillon est la :
//!
//! 1. la fenetre est **exclue de la capture** — prouve par `sonde` : sans le
//!    drapeau elle se voit a 115 200 pixels pres, avec le drapeau a zero. Donc
//!    aucun bouclage ;
//! 2. on **capture la region du bureau** sous elle, plus une marge pour que le
//!    flou ait de quoi mordre ;
//! 3. on **floute en espace lineaire** (vue sRGB), ce que DWM ne fait pas ;
//! 4. on **teinte et on sature**, avec nos propres valeurs ;
//! 5. on **masque par NOTRE superellipse**. C'est le point qui rend tout
//!    l'exercice utile : le verre du systeme est un rectangle que rien ne
//!    decoupe, celui-ci a la forme qu'on veut.

use verre_taille::bureau::Capteur;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::*;

/// La planche de fond : assez grande pour porter l'ilot et sa marge.
const PX: i32 = 300;
const PY: i32 = 240;
const PL: i32 = 900;
const PH: i32 = 560;

/// L'ilot deploye, aux mesures de DESIGN.md §6.
const IL: u32 = 320;
const IH: u32 = 148;
/// Rayon deploye retenu par l'orfevre : 48, pour garder la silhouette du galet
/// une fois passe a la superellipse.
const RAYON: f32 = 48.0;
const EXPOSANT: f32 = 4.0;

/// Ce que le flou doit pouvoir mordre autour de l'ilot.
const MARGE: u32 = 64;

// ---------------------------------------------------------------------------
// Les fenetres
// ---------------------------------------------------------------------------

/// 0 = raye clair, 1 = raye sombre, 2 = uni 226, 3 = uni 37. Les deux unis
/// reprennent EXACTEMENT les fonds de l'orfevre, sinon la comparaison ment.
pub static MODE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

unsafe extern "system" fn proc_planche(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if m == WM_PAINT {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(h, &mut ps);
        let mut r = RECT::default();
        let _ = GetClientRect(h, &mut r);

        // Des bandes larges pour juger la couleur, des rayures fines pour
        // mesurer le flou : sans haute frequence, un flou ne se prouve pas.
        let mode = MODE.load(std::sync::atomic::Ordering::Relaxed);
        let sombre = mode == 1 || mode == 3;
        let bandes: [u32; 6] = match mode {
            1 => [0x00101214, 0x00241E1A, 0x00402818, 0x00182838, 0x00101214, 0x00282828],
            2 => [0x00E2E2E2; 6],
            3 => [0x00252525; 6],
            _ => [0x00E0E4E8, 0x00E8EFF2, 0x004030C0, 0x0030A0E0, 0x00D8DCE0, 0x00F0F0F0],
        };
        let larg = (r.right - r.left) / bandes.len() as i32;
        for (i, c) in bandes.iter().enumerate() {
            let p = CreateSolidBrush(COLORREF(*c));
            let bande = RECT {
                left: r.left + larg * i as i32,
                top: r.top,
                right: r.left + larg * (i as i32 + 1),
                bottom: r.bottom,
            };
            FillRect(dc, &bande, p);
            let _ = DeleteObject(p.into());
        }
        // Rayures fines, 3 px pleins / 3 px vides, sur toute la hauteur.
        if mode >= 2 {
            // Fonds unis : aucune rayure, pour comparer des moyennes pures.
            let _ = EndPaint(h, &ps);
            return LRESULT(0);
        }
        let raie = CreateSolidBrush(COLORREF(if sombre { 0x00303436 } else { 0x00101010 }));
        let mut x = r.left;
        while x < r.right {
            let b = RECT {
                left: x,
                top: r.top + 40,
                right: (x + 3).min(r.right),
                bottom: r.bottom - 40,
            };
            FillRect(dc, &b, raie);
            x += 6;
        }
        let _ = DeleteObject(raie.into());
        let _ = EndPaint(h, &ps);
        return LRESULT(0);
    }
    if m == WM_DESTROY {
        PostQuitMessage(0);
        return LRESULT(0);
    }
    DefWindowProcW(h, m, w, l)
}

unsafe fn pomper(ms: u64) {
    let fin = std::time::Instant::now() + std::time::Duration::from_millis(ms);
    let mut msg = MSG::default();
    while std::time::Instant::now() < fin {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
}

// ---------------------------------------------------------------------------
// Les mesures
// ---------------------------------------------------------------------------

fn luminance_relative(r: u8, g: u8, b: u8) -> f64 {
    let c = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * c(r) + 0.7152 * c(g) + 0.0722 * c(b)
}

fn contraste(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let la = luminance_relative(a.0, a.1, a.2);
    let lb = luminance_relative(b.0, b.1, b.2);
    let (h, bas) = if la > lb { (la, lb) } else { (lb, la) };
    (h + 0.05) / (bas + 0.05)
}

/// L'energie de haute frequence : la moyenne des sauts entre pixels voisins.
/// C'est elle qui dit si un flou a vraiment eu lieu, et de combien.
fn haute_frequence(pixels: &[u8], l: u32, h: u32) -> f64 {
    let mut somme = 0.0;
    let mut n = 0.0;
    let lum = |i: usize| {
        0.2126 * pixels[i] as f64 + 0.7152 * pixels[i + 1] as f64 + 0.0722 * pixels[i + 2] as f64
    };
    for y in 0..h {
        for x in 0..l - 1 {
            let i = ((y * l + x) * 4) as usize;
            somme += (lum(i + 4) - lum(i)).abs();
            n += 1.0;
        }
    }
    somme / n
}

/// La moyenne d'une zone centrale, la ou l'encre se poserait.
fn moyenne_centre(pixels: &[u8], l: u32, h: u32) -> (u8, u8, u8) {
    let (x0, x1) = (l / 4, 3 * l / 4);
    let (y0, y1) = (h / 4, 3 * h / 4);
    let mut s = (0f64, 0f64, 0f64);
    let mut n = 0f64;
    for y in y0..y1 {
        for x in x0..x1 {
            let i = ((y * l + x) * 4) as usize;
            s.0 += pixels[i] as f64;
            s.1 += pixels[i + 1] as f64;
            s.2 += pixels[i + 2] as f64;
            n += 1.0;
        }
    }
    (
        (s.0 / n).round() as u8,
        (s.1 / n).round() as u8,
        (s.2 / n).round() as u8,
    )
}

/// La luminance perceptuelle d'une couleur moyenne. Le « porte » se mesure la,
/// et non sur un canal : la saturation gonfle le rouge et ferait mentir le
/// chiffre sur un fond colore.
fn lum(c: (u8, u8, u8)) -> f64 {
    0.2126 * c.0 as f64 + 0.7152 * c.1 as f64 + 0.0722 * c.2 as f64
}

fn ecrire_png(chemin: &str, pixels: &[u8], l: u32, h: u32) {
    std::fs::create_dir_all("captures").ok();
    let f = std::fs::File::create(chemin).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), l, h);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(pixels).unwrap();
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
}

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
    fn nouveau() -> Verre {
        let instance = wgpu::Instance::default();
        let adaptateur = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .expect("aucun adaptateur");
        let (peripherique, file) = pollster::block_on(adaptateur.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("verre taille"),
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
            label: Some("verre"),
            source: wgpu::ShaderSource::Wgsl(include_str!("verre.wgsl").into()),
        });

        let disposition =
            peripherique.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("disposition verre"),
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        let agencement =
            peripherique.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("agencement verre"),
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

        Verre {
            peripherique,
            file,
            disposition,
            tuyau_flou,
            tuyau_composer,
            echantillonneur,
        }
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

    /// Toute la chaine, d'une region du bureau a l'ilot fini.
    #[allow(clippy::too_many_arguments)]
    fn tailler(
        &self,
        source: &[u8],
        sl: u32,
        sh: u32,
        sigma: f32,
        teinte: [f32; 4],
        saturation: f32,
        luminosite: f32,
        perceptif: f32,
        cible: (u32, u32),
    ) -> Vec<u8> {
        let brut = self.texture(
            sl,
            sh,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        self.file.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &brut,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            source,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(sl * 4),
                rows_per_image: Some(sh),
            },
            wgpu::Extent3d {
                width: sl,
                height: sh,
                depth_or_array_layers: 1,
            },
        );

        let inter = self.texture(
            sl,
            sh,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let floute = self.texture(
            sl,
            sh,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let (cl, ch) = cible;
        let fini = self.texture(
            cl,
            ch,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );

        let v_brut = brut.create_view(&Default::default());
        let v_inter = inter.create_view(&Default::default());
        let v_floute = floute.create_view(&Default::default());
        let v_fini = fini.create_view(&Default::default());

        let rayon_taps = (sigma * 3.0).ceil();
        let base = Reglages {
            taille: [sl as f32, sh as f32, sl as f32, sh as f32],
            flou: [1.0, 0.0, sigma, rayon_taps],
            forme: [IL as f32 / 2.0, IH as f32 / 2.0, RAYON, EXPOSANT],
            teinte,
            reglages: [saturation, MARGE as f32, luminosite, perceptif],
        };
        let mut vertical = base;
        vertical.flou = [0.0, 1.0, sigma, rayon_taps];
        let mut composer = base;
        composer.taille = [cl as f32, ch as f32, sl as f32, sh as f32];
        // En vitrine la cible couvre toute la source : plus de decalage a faire.
        if cl == sl {
            composer.reglages[1] = 0.0;
        }

        let g1 = self.groupe(&base, &v_brut, &v_brut);
        let g2 = self.groupe(&vertical, &v_inter, &v_brut);
        let g3 = self.groupe(&composer, &v_floute, &v_brut);

        let mut enc = self
            .peripherique
            .create_command_encoder(&Default::default());
        for (groupe, cible, tuyau) in [
            (&g1, &v_inter, &self.tuyau_flou),
            (&g2, &v_floute, &self.tuyau_flou),
            (&g3, &v_fini, &self.tuyau_composer),
        ] {
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

        let ligne = cl * 4;
        let alignee = ligne.div_ceil(256) * 256;
        let lecture = self.peripherique.create_buffer(&wgpu::BufferDescriptor {
            label: Some("relecture"),
            size: (alignee * ch) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
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
                    rows_per_image: Some(ch),
                },
            },
            wgpu::Extent3d {
                width: cl,
                height: ch,
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
        let mut pixels = Vec::with_capacity((ligne * ch) as usize);
        for y in 0..ch {
            let d = (y * alignee) as usize;
            pixels.extend_from_slice(&vue[d..d + ligne as usize]);
        }
        drop(vue);
        lecture.unmap();
        pixels
    }
}

// ---------------------------------------------------------------------------

fn main() -> windows::core::Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let module = GetModuleHandleW(None)?;

        let c_planche = windows::core::w!("VerrePlanche");
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc_planche),
            hInstance: module.into(),
            lpszClassName: c_planche,
            ..Default::default()
        });
        let c_ilot = windows::core::w!("VerreIlot");
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc_planche),
            hInstance: module.into(),
            lpszClassName: c_ilot,
            hbrBackground: CreateSolidBrush(COLORREF(0x00101010)),
            ..Default::default()
        });

        let planche = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            c_planche,
            windows::core::w!(""),
            WS_POPUP | WS_VISIBLE,
            PX,
            PY,
            PL,
            PH,
            None,
            None,
            Some(module.into()),
            None,
        )?;
        let _ = ShowWindow(planche, SW_SHOWNOACTIVATE);

        // L'ilot, centre sur la planche.
        let ix = PX + (PL - IL as i32) / 2;
        let iy = PY + (PH - IH as i32) / 2;
        let ilot = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            c_ilot,
            windows::core::w!(""),
            WS_POPUP | WS_VISIBLE,
            ix,
            iy,
            IL as i32,
            IH as i32,
            None,
            None,
            Some(module.into()),
            None,
        )?;
        let _ = ShowWindow(ilot, SW_SHOWNOACTIVATE);
        // Sans cela, l'ilot se verrait lui-meme et le verre boucler ait.
        SetWindowDisplayAffinity(ilot, WDA_EXCLUDEFROMCAPTURE)?;
        pomper(700);

        let capteur = Capteur::ouvrir(ix + IL as i32 / 2, iy + IH as i32 / 2)?;
        eprintln!("adaptateur : {}", capteur.adaptateur);

        let sl = IL + 2 * MARGE;
        let sh = IH + 2 * MARGE;
        let mut tour = 0;
        let mut prendre = |mode: u8| {
            MODE.store(mode, std::sync::atomic::Ordering::Relaxed);
            let _ = InvalidateRect(Some(planche), None, true);
            pomper(200);
            capteur
                .region(ix - MARGE as i32, iy - MARGE as i32, sl, sh, 60, || {
                    // La duplication ne rend que les CHANGEMENTS : un bureau
                    // immobile ne produit rien, d'ou ce deplacement d'un pixel.
                    tour += 1;
                    let d = if tour % 2 == 0 { 0 } else { 1 };
                    let _ = SetWindowPos(
                        planche,
                        None,
                        PX + d,
                        PY,
                        PL,
                        PH,
                        SWP_NOACTIVATE | SWP_NOZORDER,
                    );
                    pomper(10);
                })
                .expect("capture impossible")
        };

        let clair = prendre(0);
        let sombre = prendre(1);
        let uni_clair = prendre(2);
        let uni_sombre = prendre(3);
        ecrire_png("captures/10-derriere-clair.png", &clair.pixels, clair.largeur, clair.hauteur);
        ecrire_png("captures/11-derriere-sombre.png", &sombre.pixels, sombre.largeur, sombre.hauteur);

        let verre = Verre::nouveau();
        let hf_clair = haute_frequence(&clair.pixels, clair.largeur, clair.hauteur);

        println!();
        println!(
            "derriere : clair {:?} hf {:.2} | sombre {:?}",
            moyenne_centre(&clair.pixels, clair.largeur, clair.hauteur),
            hf_clair,
            moyenne_centre(&sombre.pixels, sombre.largeur, sombre.hauteur)
        );
        println!();
        println!("  teinte  espace       hf    verre/clair   verre/sombre   porte  F2EFE9  8E8B86  5E5B57");
        println!("  ---------------------------------------------------------------------------------------");

        let alphas: [f32; 7] = [0.70, 0.80, 0.86, 0.90, 0.93, 0.96, 1.00];
        for perceptif in [0.0f32, 1.0] {
            for a in alphas {
                let mut rendu = |r: &verre_taille::bureau::Region| {
                    verre.tailler(
                        &r.pixels,
                        r.largeur,
                        r.hauteur,
                        18.0,
                        [0.0, 0.0, 0.0, a],
                        1.35,
                        1.0,
                        perceptif,
                        (IL, IH),
                    )
                };
                let pc = rendu(&clair);
                let ps = rendu(&sombre);
                let mc = moyenne_centre(&pc, IL, IH);
                let ms = moyenne_centre(&ps, IL, IH);
                // Ce que le verre PORTE du fond : sans cet ecart, ce n'est pas
                // du verre, c'est de la peinture.
                let porte = (mc.0 as i32 - ms.0 as i32).abs();
                let hf = haute_frequence(&pc, IL, IH);
                // Le contraste se juge sur le PIRE cas, c'est-a-dire le fond clair.
                println!(
                    "  {:5.2}  {:9}  {:5.2}  ({:3},{:3},{:3})  ({:3},{:3},{:3})  {:5}  {:6.2}  {:6.2}  {:6.2}",
                    a,
                    if perceptif > 0.5 { "octets" } else { "lineaire" },
                    hf,
                    mc.0, mc.1, mc.2,
                    ms.0, ms.1, ms.2,
                    porte,
                    contraste((0xF2, 0xEF, 0xE9), mc),
                    contraste((0x8E, 0x8B, 0x86), mc),
                    contraste((0x5E, 0x5B, 0x57), mc)
                );
                let nom = format!(
                    "captures/2{}-{:02.0}-clair.png",
                    if perceptif > 0.5 { 1 } else { 0 },
                    a * 100.0
                );
                ecrire_png(&nom, &pc, IL, IH);
            }
        }

        println!();
        println!("--- sur fonds UNIS, aux valeurs memes de l'orfevre ---");
        println!(
            "derriere : clair {:?} sombre {:?} (ecart de luminance {:.0})",
            moyenne_centre(&uni_clair.pixels, uni_clair.largeur, uni_clair.hauteur),
            moyenne_centre(&uni_sombre.pixels, uni_sombre.largeur, uni_sombre.hauteur),
            lum(moyenne_centre(&uni_clair.pixels, uni_clair.largeur, uni_clair.hauteur))
                - lum(moyenne_centre(&uni_sombre.pixels, uni_sombre.largeur, uni_sombre.hauteur))
        );
        println!();
        println!("  teinte  espace     verre/clair   verre/sombre   porte  part   F2EFE9  8E8B86  5E5B57");
        println!("  -------------------------------------------------------------------------------------");
        let ecart_fond = lum(moyenne_centre(&uni_clair.pixels, uni_clair.largeur, uni_clair.hauteur))
            - lum(moyenne_centre(&uni_sombre.pixels, uni_sombre.largeur, uni_sombre.hauteur));
        for perceptif in [0.0f32, 1.0] {
            for a in alphas {
                let mut rendu = |r: &verre_taille::bureau::Region| {
                    verre.tailler(&r.pixels, r.largeur, r.hauteur, 18.0,
                        [0.0, 0.0, 0.0, a], 1.35, 1.0, perceptif, (IL, IH))
                };
                let mc = moyenne_centre(&rendu(&uni_clair), IL, IH);
                let ms = moyenne_centre(&rendu(&uni_sombre), IL, IH);
                let porte = lum(mc) - lum(ms);
                println!(
                    "  {:5.2}  {:9}  ({:3},{:3},{:3})  ({:3},{:3},{:3})  {:5.1}  {:4.1}%  {:6.2}  {:6.2}  {:6.2}",
                    a,
                    if perceptif > 0.5 { "octets" } else { "lineaire" },
                    mc.0, mc.1, mc.2, ms.0, ms.1, ms.2,
                    porte,
                    100.0 * porte / ecart_fond,
                    contraste((0xF2, 0xEF, 0xE9), mc),
                    contraste((0x8E, 0x8B, 0x86), mc),
                    contraste((0x5E, 0x5B, 0x57), mc)
                );
            }
        }

        println!();
        println!("reference, noir opaque #000000 : porte 0, contrastes 18,30  6,19  3,11");
        println!("DWM acrylique noir 90 %, mesure par l'orfevre : porte 10, F2EFE9 a 12,2");

        // La preuve qui se regarde : l'ilot pose sur sa planche, a taille
        // reelle, avec autour le bureau intact.
        for (a, perceptif, nom) in [
            (0.80f32, 1.0f32, "30-vitrine-octets80"),
            (0.86, 1.0, "31-vitrine-octets86"),
            (0.80, 0.0, "32-vitrine-lineaire80"),
            (1.0, 1.0, "33-vitrine-opaque"),
        ] {
            let px = verre.tailler(
                &clair.pixels, clair.largeur, clair.hauteur, 18.0,
                [0.0, 0.0, 0.0, a], 1.35, 1.0, perceptif, (sl, sh),
            );
            ecrire_png(&format!("captures/{}.png", nom), &px, sl, sh);
        }

        // --- Ce que ca coute, sans quoi rien n'est livrable ---------------
        //
        // Deux couts distincts, qu'il ne faut pas confondre : ACQUERIR une
        // region du bureau (duplication + copie + relecture CPU) et la TAILLER
        // (deux passes de flou plus la composition, sur le GPU).
        MODE.store(0, std::sync::atomic::Ordering::Relaxed);
        let _ = InvalidateRect(Some(planche), None, true);
        pomper(150);

        let mut couts_acq = Vec::new();
        let mut couts_rendu = Vec::new();
        for i in 0..40 {
            let t0 = std::time::Instant::now();
            let r = capteur.region(ix - MARGE as i32, iy - MARGE as i32, sl, sh, 30, || {
                let d = if i % 2 == 0 { 0 } else { 1 };
                let _ = SetWindowPos(planche, None, PX + d, PY, PL, PH,
                    SWP_NOACTIVATE | SWP_NOZORDER);
                pomper(2);
            });
            let t1 = std::time::Instant::now();
            if let Ok(r) = r {
                let _ = verre.tailler(&r.pixels, r.largeur, r.hauteur, 18.0,
                    [0.0, 0.0, 0.0, 0.80], 1.35, 1.0, 1.0, (IL, IH));
                let t2 = std::time::Instant::now();
                couts_acq.push((t1 - t0).as_secs_f64() * 1000.0);
                couts_rendu.push((t2 - t1).as_secs_f64() * 1000.0);
            }
        }
        let mediane = |v: &mut Vec<f64>| {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if v.is_empty() { 0.0 } else { v[v.len() / 2] }
        };
        let n = couts_rendu.len();
        println!();
        println!("--- cout par trame, sur {} mesures ---", n);
        println!(
            "  acquisition (duplication + copie + relecture) : {:.2} ms median",
            mediane(&mut couts_acq)
        );
        println!(
            "  taille (deux flous + composition, GPU)        : {:.2} ms median",
            mediane(&mut couts_rendu)
        );
        println!("  budget d'une trame a 144 Hz                    : 6,94 ms");

        let _ = DestroyWindow(ilot);
        let _ = DestroyWindow(planche);
        Ok(())
    }
}
