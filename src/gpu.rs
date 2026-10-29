//! L'ouverture du peripherique graphique, partagee par la fenetre et par la capture.

/// Le peripherique et sa file, plus l'instance qui les a produits.
///
/// L'horloge est entierement noire opaque : elle n'a aucun besoin d'un
/// adaptateur qui sache heriter de la composition du systeme (ARCHITECTURE.md
/// §1.4). N'importe quel adaptateur convient, on prend celui que wgpu prefere.
pub struct Gpu {
    /// Gardee pour la duree de vie : la surface en descend, et rien ne garantit
    /// qu'un dorsal survive a la disparition de son instance.
    #[allow(dead_code)]
    pub instance: wgpu::Instance,
    pub adaptateur: wgpu::Adapter,
    pub peripherique: wgpu::Device,
    pub file: wgpu::Queue,
}

impl Gpu {
    /// Ouvre un peripherique compatible avec la surface donnee, ou sans surface
    /// du tout quand on ne fait que capturer (`surface = None`).
    pub fn ouvrir(instance: wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Gpu {
        let adaptateur = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: surface,
            // Champ obligatoire en wgpu 30, absent des exemples plus anciens.
            apply_limit_buckets: false,
        }))
        .expect("aucun adaptateur graphique disponible");

        Gpu::depuis(instance, adaptateur)
    }

    /// Ouvre, sans surface, le premier adaptateur dont le nom contient `nom`
    /// (casse ignoree, Vulkan d'abord), ou celui que wgpu prefere sans nom.
    ///
    /// Instrument de mesure : la fenetre de l'ilot peint sur l'adaptateur qui
    /// sait heriter de la composition (l'Intel UHD sur cette machine), la
    /// capture hors-ecran sur celui que wgpu prefere (la NVIDIA). Rendre les
    /// deux sur le MEME adaptateur est le seul moyen de separer un ecart
    /// d'adaptateur d'un ecart de rendu.
    pub fn ouvrir_nomme(instance: wgpu::Instance, nom: Option<&str>) -> Gpu {
        let Some(nom) = nom else {
            return Gpu::ouvrir(instance, None);
        };
        let voulu = nom.to_lowercase();
        let adaptateurs = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
        let porte = |a: &wgpu::Adapter| a.get_info().name.to_lowercase().contains(&voulu);
        let rang = adaptateurs
            .iter()
            .position(|a| porte(a) && a.get_info().backend == wgpu::Backend::Vulkan)
            .or_else(|| adaptateurs.iter().position(porte));
        match rang {
            Some(i) => {
                let adaptateur = adaptateurs.into_iter().nth(i).expect("rang trouve juste avant");
                Gpu::depuis(instance, adaptateur)
            }
            None => {
                eprintln!("aucun adaptateur ne porte « {nom} » : celui de wgpu est pris");
                Gpu::ouvrir(instance, None)
            }
        }
    }

    /// Ouvre, sans surface, l'adaptateur nomme (ou celui de wgpu) avec
    /// l'horodateur GPU s'il le porte. Rend aussi si l'horodateur est la.
    /// Instrument du cout (`cout_givre.rs`) : l'application n'en demande jamais.
    pub fn ouvrir_nomme_horodate(instance: wgpu::Instance, nom: Option<&str>) -> (Gpu, bool) {
        let adaptateurs = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
        let voulu = nom.map(|n| n.to_lowercase());
        let porte = |a: &wgpu::Adapter| match &voulu {
            Some(v) => a.get_info().name.to_lowercase().contains(v.as_str()),
            None => true,
        };
        let rang = adaptateurs
            .iter()
            .position(|a| porte(a) && a.get_info().backend == wgpu::Backend::Vulkan)
            .or_else(|| adaptateurs.iter().position(porte));
        let adaptateur = match rang {
            Some(i) => adaptateurs.into_iter().nth(i).expect("rang trouve juste avant"),
            None => pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
                apply_limit_buckets: false,
            }))
            .expect("aucun adaptateur graphique disponible"),
        };
        let horodate = adaptateur.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        let demande = if horodate {
            wgpu::Features::TIMESTAMP_QUERY
        } else {
            wgpu::Features::empty()
        };
        (Gpu::depuis_avec(instance, adaptateur, demande), horodate)
    }

    /// Ouvre le peripherique d'un adaptateur deja choisi.
    pub fn depuis(instance: wgpu::Instance, adaptateur: wgpu::Adapter) -> Gpu {
        Gpu::depuis_avec(instance, adaptateur, wgpu::Features::empty())
    }

    /// Le meme, avec les fonctions demandees. L'application n'en demande
    /// aucune ; l'instrument du cout demande l'horodateur.
    fn depuis_avec(instance: wgpu::Instance, adaptateur: wgpu::Adapter, fonctions: wgpu::Features) -> Gpu {
        let (peripherique, file) = pollster::block_on(adaptateur.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("peripherique veilleuse"),
                required_features: fonctions,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adaptateur.limits()),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            },
        ))
        .expect("ouverture du peripherique refusee");

        // Une erreur de validation silencieuse coute des heures : on la fait crier.
        peripherique.on_uncaptured_error(std::sync::Arc::new(|e| {
            eprintln!("!! erreur wgpu non capturee : {e}");
        }));

        Gpu {
            instance,
            adaptateur,
            peripherique,
            file,
        }
    }

    /// Ouvre un peripherique qui convient aux DEUX fenetres, en preferant celui
    /// qui saura rendre l'ilot transparent.
    ///
    /// ARCHITECTURE.md §1.4 : sur cette machine, un seul couple dorsal /
    /// adaptateur sur six laisse passer l'alpha par pixel, et le marqueur fiable
    /// n'est pas le mode demande — c'est la presence d'`Inherit` dans les modes
    /// annonces par la surface de l'ilot. `PowerPreference` ne suffit pas et
    /// n'est pas portable : il faut parcourir les adaptateurs et regarder.
    ///
    /// Si aucun ne l'annonce, on en prend un qui sache au moins peindre les deux
    /// surfaces : l'ilot sera alors decoupe par sa seule region GDI, coins
    /// creneles, et il tournera quand meme. Cette porte de sortie est ecrite ici
    /// des le premier jour, comme ARCHITECTURE.md §9 l'exigeait.
    pub fn ouvrir_pour_deux(
        instance: wgpu::Instance,
        horloge: &wgpu::Surface<'_>,
        ilot: &wgpu::Surface<'_>,
    ) -> Gpu {
        let mut repli: Option<wgpu::Adapter> = None;
        let mut choisi: Option<wgpu::Adapter> = None;
        // `enumerate_adapters` est asynchrone en wgpu 30, et l'attente est ici
        // sans consequence : on est encore au demarrage, rien n'est affiche.
        let adaptateurs = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()));
        for adaptateur in adaptateurs {
            let ch = horloge.get_capabilities(&adaptateur);
            let ci = ilot.get_capabilities(&adaptateur);
            // Une surface que l'adaptateur ne sait pas peindre n'annonce aucun
            // format : c'est ainsi qu'on ecarte les couples incompatibles.
            if ch.formats.is_empty() || ci.formats.is_empty() {
                continue;
            }
            if ci.alpha_modes.contains(&wgpu::CompositeAlphaMode::Inherit) {
                choisi = Some(adaptateur);
                break;
            }
            if repli.is_none() {
                repli = Some(adaptateur);
            }
        }

        match choisi.or(repli) {
            Some(adaptateur) => Gpu::depuis(instance, adaptateur),
            // Aucun adaptateur enumere ne convient : on retombe sur le choix de
            // wgpu, qui au pire refusera bruyamment.
            None => Gpu::ouvrir(instance, Some(horloge)),
        }
    }

    pub fn nom(&self) -> String {
        let info = self.adaptateur.get_info();
        format!("{} ({:?})", info.name, info.backend)
    }
}
