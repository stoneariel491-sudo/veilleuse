//! Le cout d'une trame de la transformation, mesure au GPU (`GIVRE.md` §4).
//!
//! La methode est celle de `RECETTES.md` (« Un temps GPU absolu n'est pas
//! reproductible d'un lancement a l'autre », « ABBA retranche l'additif, et pas
//! le reste », « allonger une reference de cout sans changer ce qu'elle mesure ») :
//!
//! - un horodateur GPU (`TIMESTAMP_QUERY`) au debut et a la fin de la passe ;
//! - la passe de l'ilot trace le galet `k` fois, `k` doublant jusqu'a ce que
//!   la borne d'arrondi du rapport publie tombe sous 0,01 ;
//! - chaque paire (A, B) se mesure en alternance A B B A, seize tours, et l'on
//!   publie la mediane du rapport B/A et de B - A ; la premiere paire est le
//!   temoin A contre A, qui doit rendre 1 ;
//! - rien ne se compare a un temps absolu d'un autre lancement : l'outil
//!   s'appelle trois fois et `GIVRE.md` publie les trois rapports.
//!
//! Ce qui est mesure : la passe « ilot » entiere (galet trace k fois, plus le
//! contenu une fois), pour le galet opaque, le verre, et le givre a trois
//! instants ; puis les deux passes de flou du verre contre les deux du givre.

use std::fmt::Write as _;

use crate::gpu::Gpu;
use crate::ilot::Vitrine;
use crate::options::Options;
use crate::texte::Typographie;

/// Lit les deux horodatages et rend leur ecart en millisecondes.
struct Lecteur {
    horodateur: wgpu::QuerySet,
    resolution: wgpu::Buffer,
    lecture: wgpu::Buffer,
    periode_ns: f32,
}

impl Lecteur {
    fn nouveau(gpu: &Gpu) -> Lecteur {
        let horodateur = gpu.peripherique.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("horodateur du givre"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let resolution = gpu.peripherique.create_buffer(&wgpu::BufferDescriptor {
            label: Some("resolution des horodatages"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let lecture = gpu.peripherique.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lecture des horodatages"),
            size: 16,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Lecteur {
            horodateur,
            resolution,
            lecture,
            periode_ns: gpu.file.get_timestamp_period(),
        }
    }

    /// Apres une passe horodatee : resout, relit, rend les millisecondes.
    fn lire(&self, gpu: &Gpu) -> f64 {
        let mut enc = gpu
            .peripherique
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("resolution"),
            });
        enc.resolve_query_set(&self.horodateur, 0..2, &self.resolution, 0);
        enc.copy_buffer_to_buffer(&self.resolution, 0, &self.lecture, 0, 16);
        gpu.file.submit(Some(enc.finish()));
        let tranche = self.lecture.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        tranche.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        gpu.peripherique
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        rx.recv().unwrap().unwrap();
        let ms = {
            let brut = tranche.get_mapped_range().unwrap();
            let t0 = u64::from_le_bytes(brut[0..8].try_into().unwrap());
            let t1 = u64::from_le_bytes(brut[8..16].try_into().unwrap());
            t1.wrapping_sub(t0) as f64 * self.periode_ns as f64 / 1e6
        };
        self.lecture.unmap();
        ms
    }
}

fn mediane(mut x: Vec<f64>) -> f64 {
    x.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = x.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        x[n / 2]
    } else {
        0.5 * (x[n / 2 - 1] + x[n / 2])
    }
}

/// Mesure et ecrit le journal. `vitrine` est composee et son fond televerse.
pub fn mesurer(
    gpu: &Gpu,
    horodate: bool,
    typo: &mut Typographie,
    vitrine: &mut Vitrine,
    vue: &wgpu::TextureView,
    opt: &Options,
    journal: &std::path::Path,
) -> Result<(), String> {
    let mut j = String::new();
    let _ = writeln!(j, "Le cout d'une trame de la transformation, mesure au GPU.");
    let _ = writeln!(j, "adaptateur : {}", gpu.nom());
    if !horodate {
        let _ = writeln!(j, "HORODATEUR INDISPONIBLE sur cet adaptateur : rien n'est mesure.");
        std::fs::write(journal, &j).map_err(|e| e.to_string())?;
        return Err("horodateur indisponible".into());
    }
    let lecteur = Lecteur::nouveau(gpu);
    let _ = writeln!(j, "periode de l'horodateur : {:.3} ns", lecteur.periode_ns);
    let _ = writeln!(j, "fond : {}", opt.verre_fond.as_ref().map(|p| p.display().to_string()).unwrap_or_default());
    let _ = writeln!(j, "echelle : {}", opt.echelle);

    // Les variantes : le galet opaque, le verre, et le givre a trois instants
    // des ressorts (le front en plein vol, la fonte a mi-course, la fin).
    // Chaque variante porte son etat de matiere et, depuis la lampe d'ecran
    // (`INTEGRATION-APRES-EPREUVE.md`, pas 2), l'intensite de lampe imposee :
    // `Some(0.0)` la coupe, `None` est le chemin normal. La reference est le
    // verre SANS lampe, c'est-a-dire le verre d'avant le pas 2 : les rapports
    // restent comparables a ceux de `GIVRE.md`, et la paire « sans lampe ->
    // avec lampe » mesure ce que la lampe coute.
    let instants = [0.150f32, 0.330, 0.500];
    type Variante = (String, Option<(f32, f32)>, Option<f32>);
    let mut variantes: Vec<Variante> = vec![
        ("galet opaque".into(), Some((0.0, 0.0)), Some(0.0)),
        ("verre sans lampe (le verre d'avant)".into(), None, Some(0.0)),
        ("verre avec lampe (chemin normal)".into(), None, None),
        ("verre (transformation posee)".into(), Some((1.0, 1.0)), None),
    ];
    for t in instants {
        let (g, f) = crate::matiere::Matiere::a_l_instant(t, true, None).etat();
        variantes.push((format!("givre a {:.0} ms (g {g:.3}, f {f:.3})", t * 1000.0), Some((g, f)), None));
    }

    let mut trame = |vitrine: &mut Vitrine, matiere: Option<(f32, f32)>, lampe: Option<f32>, fois: u32| -> f64 {
        vitrine.matiere = matiere;
        vitrine.imposer_lampe(lampe);
        vitrine.dessiner_fois(gpu, typo, vue, fois, Some(&lecteur.horodateur));
        lecteur.lire(gpu)
    };

    // Le calibrage : k double jusqu'a ce que la borne d'arrondi du rapport
    // (givre 330 ms contre verre) tombe sous 0,01, ou que k atteigne 512.
    let tic = lecteur.periode_ns as f64 / 1e6;
    let _ = writeln!(j, "\nCALIBRAGE. k double jusqu'a ce que R x 2 tics / A tombe sous 0,01.");
    let _ = writeln!(j, "     k    verre (ms)   givre 330 (ms)     B/A   borne");
    let mut k = 1u32;
    let givre_330 = variantes[5].1;
    loop {
        let a = mediane((0..6).map(|_| trame(vitrine, None, Some(0.0), k)).collect());
        let b = mediane((0..6).map(|_| trame(vitrine, givre_330, None, k)).collect());
        let r = b / a.max(1e-9);
        let borne = r * 2.0 * tic / a.max(1e-9);
        let _ = writeln!(j, "  {k:>4}  {a:>11.4}  {b:>15.4}  {r:>7.3}  {borne:.4}");
        if borne <= 0.01 || k >= 512 {
            break;
        }
        k *= 2;
    }
    let _ = writeln!(j, "k retenu : {k}");

    // Les paires, en A B B A, seize tours.
    type Etat = (Option<(f32, f32)>, Option<f32>);
    let apparier = |vitrine: &mut Vitrine, a: Etat, b: Etat, trame: &mut dyn FnMut(&mut Vitrine, Option<(f32, f32)>, Option<f32>, u32) -> f64| {
        let (mut da, mut db, mut dd) = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..16 {
            let a1 = trame(vitrine, a.0, a.1, k);
            let b1 = trame(vitrine, b.0, b.1, k);
            let b2 = trame(vitrine, b.0, b.1, k);
            let a2 = trame(vitrine, a.0, a.1, k);
            let (ma, mb) = (0.5 * (a1 + a2), 0.5 * (b1 + b2));
            da.push(ma);
            db.push(mb);
            dd.push(mb - ma);
        }
        (mediane(da), mediane(db), mediane(dd))
    };
    let _ = writeln!(j, "\nPAIRES, en alternance A B B A, seize tours, medianes ; galet trace {k} fois.");
    let _ = writeln!(j, "  {:<44} {:>10} {:>10} {:>11} {:>7}", "paire", "A (ms)", "B (ms)", "B - A (ms)", "B/A");
    let reference = variantes[1].clone();
    let paires: Vec<(String, Etat, Etat)> = {
        let mut p = vec![(format!("{} contre lui-meme (temoin)", reference.0), (reference.1, reference.2), (reference.1, reference.2))];
        for (nom, m, l) in variantes.iter() {
            if nom != &reference.0 {
                p.push((format!("{} -> {}", reference.0, nom), (reference.1, reference.2), (*m, *l)));
            }
        }
        p
    };
    let mut lignes = Vec::new();
    for (nom, a, b) in &paires {
        let (ma, mb, md) = apparier(vitrine, *a, *b, &mut trame);
        let r = mb / ma.max(1e-9);
        let _ = writeln!(j, "  {nom:<44} {ma:>10.4} {mb:>10.4} {md:>+11.4} {r:>7.3}");
        lignes.push((nom.clone(), r));
    }
    vitrine.matiere = None;
    vitrine.imposer_lampe(None);

    // Les flous : les deux passes du verre contre les deux du givre.
    let _ = writeln!(j, "\nLES FLOUS, deux passes chacun, en alternance A B B A, seize tours.");
    let flou = |vitrine: &mut Vitrine, givre: bool| -> f64 {
        vitrine.refaire_flou(gpu, givre, &lecteur.horodateur);
        lecteur.lire(gpu)
    };
    let (mut da, mut db, mut dd) = (Vec::new(), Vec::new(), Vec::new());
    let (mut ta, mut tb) = (Vec::new(), Vec::new());
    for _ in 0..16 {
        let a1 = flou(vitrine, false);
        let b1 = flou(vitrine, true);
        let b2 = flou(vitrine, true);
        let a2 = flou(vitrine, false);
        let t1 = flou(vitrine, false);
        let t2 = flou(vitrine, false);
        da.push(0.5 * (a1 + a2));
        db.push(0.5 * (b1 + b2));
        dd.push(0.5 * (b1 + b2) - 0.5 * (a1 + a2));
        ta.push(t1);
        tb.push(t2);
    }
    let (ma, mb, md) = (mediane(da), mediane(db), mediane(dd));
    let (mta, mtb) = (mediane(ta), mediane(tb));
    let _ = writeln!(j, "  {:<44} {:>10.4} {:>10.4} {:>+11.4} {:>7.3}", "flou du verre contre lui-meme (temoin)", mta, mtb, mtb - mta, mtb / mta.max(1e-9));
    let _ = writeln!(j, "  {:<44} {ma:>10.4} {mb:>10.4} {md:>+11.4} {:>7.3}", "flou du verre -> flou du givre", mb / ma.max(1e-9));

    let _ = writeln!(j, "\nCe qui se publie est le rapport B/A, dans ce lancement ; les millisecondes ne se comparent pas d'un lancement a l'autre (RECETTES.md).");
    if let Some(parent) = journal.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(journal, &j).map_err(|e| e.to_string())?;
    print!("{j}");
    Ok(())
}
