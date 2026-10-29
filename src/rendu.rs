//! Le pipeline des quads : c'est lui qui peint les cartes et les volets.

use wgpu::util::DeviceExt;

use crate::gpu::Gpu;

/// Un quad, a plat, avec de quoi le faire tourner.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Quad {
    /// x, y, largeur, hauteur en pixels d'ecran, avant rotation.
    pub rect: [f32; 4],
    /// u0, v0, u1, v1 dans l'atlas des chiffres.
    pub uv: [f32; 4],
    /// angle en radians, y du pli en pixels, distance de l'oeil, lumiere 0/1.
    pub par: [f32; 4],
    /// Multiplicateur de l'encre, en LINEAIRE, et un quatrieme mot inutilise
    /// qui garde l'alignement sur seize octets. L'atlas est grave une fois pour
    /// toutes en `#F2EFE9` ; c'est ce facteur qui fait baisser les chiffres vers
    /// `#8E8B86` quand le livre se ferme, sans regraver quoi que ce soit.
    pub teinte: [f32; 4],
}

/// Vingt-quatre quads suffisent : six cartes, quatre couches au plus.
const CAPACITE: usize = 32;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniformes {
    taille: [f32; 2],
    remplissage: [f32; 2],
}

pub struct RenduQuads {
    pipeline: wgpu::RenderPipeline,
    groupe: wgpu::BindGroup,
    tampon_uniformes: wgpu::Buffer,
    tampon_quads: wgpu::Buffer,
    nombre: u32,
}

impl RenduQuads {
    pub fn nouveau(
        gpu: &Gpu,
        atlas: &wgpu::TextureView,
        format: wgpu::TextureFormat,
    ) -> RenduQuads {
        let module = gpu
            .peripherique
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("volet"),
                source: wgpu::ShaderSource::Wgsl(include_str!("volet.wgsl").into()),
            });

        let tampon_uniformes =
            gpu.peripherique
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("uniformes du volet"),
                    contents: bytemuck::bytes_of(&Uniformes {
                        taille: [1.0, 1.0],
                        remplissage: [0.0, 0.0],
                    }),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let tampon_quads = gpu.peripherique.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quads"),
            size: (CAPACITE * std::mem::size_of::<Quad>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Filtrage lineaire : pendant la rotation le quad ne tombe plus sur la
        // grille de pixels. Au repos, le quad couvre exactement sa cellule, les
        // centres d'echantillon tombent sur les centres de texel et le lineaire
        // redonne le texel exact — donc aucun flou a l'arret.
        let echantillonneur = gpu
            .peripherique
            .create_sampler(&wgpu::SamplerDescriptor {
                label: Some("echantillonneur de l'atlas"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            });

        let disposition =
            gpu.peripherique
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("liaisons du volet"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
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
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let groupe = gpu
            .peripherique
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("liaisons du volet"),
                layout: &disposition,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: tampon_uniformes.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(atlas),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&echantillonneur),
                    },
                ],
            });

        let agencement = gpu
            .peripherique
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("agencement du volet"),
                bind_group_layouts: &[Some(&disposition)],
                immediate_size: 0,
            });

        let attributs =
            wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4];
        let pipeline = gpu
            .peripherique
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("volet"),
                layout: Some(&agencement),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Quad>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &attributs,
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        // Aucun melange : les volets sont opaques et se cachent
                        // les uns les autres. C'est cette occlusion qui fait la
                        // mecanique du volet.
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    // Aucun ecartement de face : un volet qui passe par 90 deg
                    // change d'orientation, et il doit rester visible.
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        RenduQuads {
            pipeline,
            groupe,
            tampon_uniformes,
            tampon_quads,
            nombre: 0,
        }
    }

    pub fn televerser(&mut self, gpu: &Gpu, taille: (u32, u32), quads: &[Quad]) {
        assert!(quads.len() <= CAPACITE, "trop de quads pour le tampon");
        gpu.file.write_buffer(
            &self.tampon_uniformes,
            0,
            bytemuck::bytes_of(&Uniformes {
                taille: [taille.0 as f32, taille.1 as f32],
                remplissage: [0.0, 0.0],
            }),
        );
        gpu.file
            .write_buffer(&self.tampon_quads, 0, bytemuck::cast_slice(quads));
        self.nombre = quads.len() as u32;
    }

    pub fn dessiner(&self, passe: &mut wgpu::RenderPass<'_>) {
        if self.nombre == 0 {
            return;
        }
        passe.set_pipeline(&self.pipeline);
        passe.set_bind_group(0, &self.groupe, &[]);
        passe.set_vertex_buffer(0, self.tampon_quads.slice(..));
        passe.draw(0..6, 0..self.nombre);
    }
}
