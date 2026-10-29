//! La capture : rendre une image hors-ecran et l'ecrire en PNG.
//!
//! Elle ne demande aucune fenetre. C'est le seul moyen honnete de juger un
//! rendu : on regarde ce que la carte graphique a reellement produit, pas ce
//! que le programme croit avoir dessine.

use std::path::Path;

use crate::gpu::Gpu;

/// Format de la cible en capture. Il est sRGB, comme la surface de la fenetre
/// (`Bgra8UnormSrgb`), pour que les deux chemins encodent les couleurs
/// identiquement ; l'ordre des composantes est celui du PNG, donc aucune
/// permutation d'octets a l'ecriture.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Rend `dessiner` dans une texture hors-ecran et ecrit le resultat en PNG.
///
/// Elle prend un dessinateur et non une scene : l'horloge et l'ilot passent tous
/// deux par ici, et la capture n'a pas a savoir lequel des deux elle regarde.
pub fn capturer(
    gpu: &Gpu,
    largeur: u32,
    hauteur: u32,
    chemin: &Path,
    dessiner: impl FnOnce(&Gpu, &wgpu::TextureView),
) -> std::io::Result<()> {
    let cible = gpu.peripherique.create_texture(&wgpu::TextureDescriptor {
        label: Some("cible de capture"),
        size: wgpu::Extent3d {
            width: largeur,
            height: hauteur,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let vue = cible.create_view(&wgpu::TextureViewDescriptor::default());

    dessiner(gpu, &vue);

    // copy_texture_to_buffer exige des lignes multiples de 256 octets ; a
    // 1920 px c'est deja vrai, mais compter dessus casserait a la premiere
    // autre largeur.
    let ligne_octets = largeur * 4;
    let ligne_alignee = ligne_octets.div_ceil(256) * 256;

    let lecture = gpu.peripherique.create_buffer(&wgpu::BufferDescriptor {
        label: Some("tampon de relecture"),
        size: (ligne_alignee * hauteur) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut enc = gpu
        .peripherique
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("copie de capture"),
        });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &cible,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &lecture,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ligne_alignee),
                rows_per_image: Some(hauteur),
            },
        },
        wgpu::Extent3d {
            width: largeur,
            height: hauteur,
            depth_or_array_layers: 1,
        },
    );
    gpu.file.submit(Some(enc.finish()));

    let tranche = lecture.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    tranche.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    gpu.peripherique
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    rx.recv().unwrap().unwrap();

    let brut = tranche.get_mapped_range().unwrap();
    let mut pixels = Vec::with_capacity((ligne_octets * hauteur) as usize);
    for y in 0..hauteur {
        let debut = (y * ligne_alignee) as usize;
        pixels.extend_from_slice(&brut[debut..debut + ligne_octets as usize]);
    }
    drop(brut);
    lecture.unmap();

    if let Some(parent) = chemin.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let fichier = std::fs::File::create(chemin)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(fichier), largeur, hauteur);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut ecrivain = enc.write_header()?;
    ecrivain.write_image_data(&pixels)?;
    ecrivain.finish()?;
    Ok(())
}
