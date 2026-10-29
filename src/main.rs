//! Veilleuse — l'horloge a volets et l'ilot de concentration.
//!
//! Deux fenetres, un seul peripherique graphique :
//!
//! - **l'horloge**, plein ecran, noire opaque, `set_cursor_hittest(false)` pose
//!   une fois pour toutes. Elle ne recoit rien et ne bloque rien ;
//! - **l'ilot**, petit, transparent, toujours au-dessus, atteignable.
//!
//! Ce n'est pas une preference : ARCHITECTURE.md §2 a montre qu'une fenetre
//! unique obligerait a basculer le drapeau de survol selon la position du
//! curseur, donc a interroger `GetCursorPos` quarante fois par seconde toute la
//! nuit, pour une horloge qui ne devrait dessiner qu'une fois par seconde.
//!
//! Et un troisieme chemin, sans aucune fenetre : la capture hors-ecran, qui sert
//! a juger le rendu sans le regarder tourner.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod bureau;
mod capture;
mod chrono;
mod cout_givre;
mod galet;
mod givre;
mod matiere;
mod geometrie;
mod gpu;
mod ilot;
mod json;
mod options;
mod palette;
mod panneau;
mod programme;
#[cfg(windows)]
mod releve;
mod rendu;
mod ressort;
mod scene;
mod systeme;
mod temps;
mod texte;
#[cfg(windows)]
mod oeil;
#[cfg(windows)]
mod toile;
mod volet;

use std::sync::Arc;

use chrono::Chrono;
use gpu::Gpu;
use ilot::Ilot;
use options::Options;
use programme::Programme;
use scene::Scene;
use texte::Typographie;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window, WindowId, WindowLevel};

/// Win+D et Win+M doivent reduire l'horloge pour que le bureau passe devant
/// (DESIGN-ILOT.md §1.2). Winit ote le bouton de reduction aux fenetres plein
/// ecran et les marque « application plein ecran » aupres de la barre des
/// taches, ce qui les soustrait a Win+D (`systeme.rs`) : on remet l'un, on ote
/// l'autre, a chaque entree en plein ecran. Le plein ecran lui-meme ne change
/// pas : la fenetre couvre l'ecran, sans bord. Rend la ligne du journal.
fn rendre_reductible_par_win_d(fenetre: &Window) -> String {
    let hwnd = hwnd_de(fenetre);
    format!(
        "reductible par Win+D (WS_MINIMIZEBOX {}, marque plein ecran de la barre des taches {})",
        if crate::systeme::rendre_reductible(hwnd) { "pose" } else { "REFUSE" },
        match crate::systeme::demarquer_plein_ecran(hwnd) {
            Ok(()) => "otee".to_string(),
            Err(e) => format!("PAS OTEE : {e}"),
        }
    )
}

/// La poignee Win32 d'une fenetre, 0 s'il n'y en a pas.
fn hwnd_de(fenetre: &Window) -> isize {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match fenetre.window_handle().map(|p| p.as_raw()) {
        Ok(RawWindowHandle::Win32(w)) => w.hwnd.get(),
        _ => 0,
    }
}

/// Programme le reveil sur la prochaine frontiere de seconde de l'horloge du
/// systeme. Mesure sur cette machine : le reveil arrive avec moins d'une
/// demi-milliseconde de retard, et toujours en retard, jamais en avance.
fn armer(el: &ActiveEventLoop, instant: temps::Instant, geste: Option<std::time::Instant>) {
    let attente = std::time::Duration::from_millis(instant.avant_la_seconde());
    let seconde = std::time::Instant::now() + attente;
    // Un geste a echeance (l'appui maintenu, le depart du pointeur) reveille
    // plus tot ; rien n'est interroge entre-temps.
    el.set_control_flow(ControlFlow::WaitUntil(geste.map_or(seconde, |g| g.min(seconde))));
}

/// Noircit le fond d'une fenetre qui n'a pas encore d'image.
///
/// POURQUOI. Une fenetre de haut niveau superposee que Windows montre avant que
/// rien ne l'ait peinte est composee **blanc pur** — c'est sa surface de
/// redirection. La table des douze barreaux de `OUVERTURE-LE-BLANC-ETEINT.md`
/// l'etablit : le blanc ne depend ni du cadre, ni du style etendu, ni de la
/// taille ; il ne tient qu'a ce que la fenetre soit superposee et montree.
///
/// Ce que fait cette fonction est le geste le plus court qui l'eteigne : elle
/// ecrit du noir dans cette surface, tout de suite, par GDI. Elle ne passe pas
/// par la brosse de la classe — winit possede la classe, et la brosse
/// n'agirait qu'a un `WM_ERASEBKGND`, c'est-a-dire a la premiere pompe de
/// messages, qui n'arrive qu'a la fin de `resumed`, bien apres le blanc.
///
/// Elle rend le nombre de pixels noircis, ou 0 si la fenetre n'a pas de
/// poignee : le journal le dit, plutot que de laisser croire que c'est fait.
#[cfg(windows)]
fn noircir_le_fond(fenetre: &Window) -> i64 {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        FillRect, GetDC, GetStockObject, ReleaseDC, BLACK_BRUSH, HBRUSH,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

    let hwnd = hwnd_de(fenetre) as HWND;
    if hwnd.is_null() {
        return 0;
    }
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    unsafe {
        if GetClientRect(hwnd, &mut r) == 0 {
            return 0;
        }
        let dc = GetDC(hwnd);
        if dc.is_null() {
            return 0;
        }
        FillRect(dc, &r, GetStockObject(BLACK_BRUSH) as HBRUSH);
        ReleaseDC(hwnd, dc);
    }
    (r.right - r.left) as i64 * (r.bottom - r.top) as i64
}

/// L'instant du depart du processus, pour dater les lignes du journal qui
/// disent ou passe le temps de la boucle (diagnostic du pas 4).
static DEPART: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn depuis_le_depart() -> f64 {
    DEPART.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1e3
}

/// L'epreuve de l'ouverture (`OUVERTURE-LE-BLANC-ETEINT.md`) : la construction
/// s'arrete a l'etage nomme, rien n'est jamais presente, et le processus quitte
/// de lui-meme. Deux secondes, parce que le blanc qu'il s'agit d'attribuer en
/// tient 727 (`OUVERTURE-LE-NOIR.md` §1) : l'etage doit survivre a l'episode
/// entier, sans quoi son silence ne prouverait que sa brievete.
///
/// L'attente se fait **sur le fil de la boucle, dans `resumed`**, et non sur un
/// fil a part. Deux raisons, et la seconde est la bonne :
///   - rendre la main a `resumed` libererait `horloge_f` et `ilot_f`, donc
///     detruirait les deux fenetres : mesure faite, DWM les efface alors en
///     130 ms par son animation de fermeture, et l'etage ne tiendrait pas ;
///   - le vrai lancement passe 576 ms DANS `resumed`, pompe a l'arret, et c'est
///     pendant cette attente-la que le blanc parait. Attendre ici, c'est donc
///     tenir l'etat exact du lancement reel, et non une imitation.
///
/// L'arret est un `exit` : la boucle n'a pas d'etat a cet instant — `resumed`
/// n'ira jamais au bout — et rien de ce qui se sauvegarde n'existe encore.
fn tenir_l_epreuve_de_l_ouverture(etage: &str) -> ! {
    println!(
        "epreuve de l'ouverture : etage « {etage} » atteint a {:.0} ms du depart ; \
         rien ne sera presente, arret dans 2000 ms",
        depuis_le_depart()
    );
    std::thread::sleep(std::time::Duration::from_millis(2000));
    println!("epreuve de l'ouverture : arret a {:.0} ms du depart", depuis_le_depart());
    std::process::exit(0);
}

fn main() {
    DEPART.get_or_init(std::time::Instant::now);
    let opt = Options::lire();
    // Les forcages des reglages du systeme, AVANT les captures hors ecran qui
    // reviennent tot : leurs temoins sont ceux du pas 7.
    systeme::forcer_mouvement_reduit(opt.mouvement_reduit);
    systeme::forcer_transparence_coupee(opt.transparence_coupee);
    systeme::forcer_economiseur(opt.economiseur);
    systeme::forcer_contraste(opt.contraste);
    systeme::forcer_taille_du_texte(opt.taille_texte);
    let (programme, plainte) = Programme::charger();
    if let Some(p) = &plainte {
        eprintln!("{}", p.texte);
    }
    // La sortie d'erreur n'existe qu'au lancement depuis un terminal ; le
    // journal, lui, existe toujours.
    Programme::consigner(plainte.as_ref(), temps::maintenant());
    let mut chrono = Chrono::nouveau(programme);
    opt.appliquer_au_chrono(&mut chrono);

    if let Some(chemin) = opt.capture.clone() {
        capturer_horloge(&opt, &chrono, &chemin);
        return;
    }
    if let Some(chemin) = opt.capture_cristaux.clone() {
        capturer_cristaux(&chemin, opt.epreuve_tiges);
        return;
    }
    if let Some(journal) = opt.mesure_givre.clone() {
        mesurer_givre(&opt, &chrono, &journal);
        return;
    }
    if let Some(chemin) = opt.capture_ilot.clone() {
        capturer_ilot(&opt, &chrono, &chemin);
        return;
    }



    // La conscience du DPI, AVANT toute fenetre et tout DXGI : les fenetres
    // naissent dans `resumed` et wgpu s'ouvre apres elles. Sans elle, la
    // duplication rendrait des coordonnees a l'echelle (1536 x 864) pour une
    // texture en pixels physiques (1920 x 1080), `VERRE-TAILLE.md` §2. winit la
    // declare aussi dans `EventLoop::new`, mais ce qui est paye ne doit pas
    // dependre de l'interieur d'une dependance : on la pose ici, et la
    // duplication la relit a chaque ouverture.
    #[cfg(windows)]
    let conscience = bureau::declarer_conscience_dpi(opt.epreuve_dpi);

    // L'identifiant de modele d'application, pose AVANT toute fenetre : c'est
    // lui qui apparie la fenetre en cours et le raccourci epingle, et Microsoft
    // demande qu'il soit pose « avant que l'application presente la moindre
    // interface ». Sans lui, Windows en calcule un par heuristiques.
    #[cfg(windows)]
    match systeme::declarer_identifiant_application() {
        Ok(id) => println!("identifiant d'application : {id}"),
        Err(e) => eprintln!("identifiant d'application refuse, {e}"),
    }

    let el = EventLoop::<Evenement>::with_user_event()
        .build()
        .expect("boucle d'evenements indisponible");
    // Rien n'est dessine tant que rien ne change : l'attente est la regle et le
    // reveil l'exception.
    el.set_control_flow(ControlFlow::Wait);
    if opt.scripte() {
        // Le relevé avance par tics de 16 ms venus d'un fil a part : il ne
        // touche ainsi ni au reveil de l'horloge ni a son ControlFlow. Le meme
        // fil borne la duree, quoi qu'il arrive a la boucle. L'oeil pose neuf
        // scenes de trois images chacune, recommencees trois fois au plus : il
        // a droit a plus.
        let proxy = el.create_proxy();
        let limite = if opt.oeil.is_some() { 150 } else { 60 };
        std::thread::spawn(move || {
            let debut = std::time::Instant::now();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(16));
                if debut.elapsed() > std::time::Duration::from_secs(limite) {
                    eprintln!("script : {limite} s depassees, arret force");
                    std::process::exit(3);
                }
                if proxy.send_event(Evenement::Tic).is_err() {
                    break;
                }
            }
        });
    }
    let mut app = Application {
        proxy: el.create_proxy(),
        opt,
        chrono,
        etat: None,
        plein_ecran: true,
        derniere_pendule: std::time::Instant::now(),
        #[cfg(windows)]
        conscience,
        #[cfg(windows)]
        releve: None,
        #[cfg(windows)]
        oeil: None,
        releve_matiere: None,
        releve_presence: None,
        seconde_posee: None,
        maj: false,
        sonde_vue: false,
        trame_ilot_vue: false,
        trame_horloge_vue: false,
        presence_lue: false,
    };
    el.run_app(&mut app).expect("boucle d'evenements interrompue");
}

// --- Les deux captures -------------------------------------------------------

fn capturer_horloge(opt: &Options, chrono: &Chrono, chemin: &std::path::Path) {
    // Aucune fenetre : l'instance se cree sans poignee d'affichage.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let gpu = Gpu::ouvrir(instance, None);
    println!("adaptateur : {}", gpu.nom());
    let mut typo = Typographie::nouvelle(&gpu);
    let (l, h) = opt.taille;
    let mut scene = Scene::nouvelle(&gpu, &mut typo, capture::FORMAT, l, h, opt);
    println!("familles chargees : {:?}", typo.familles());
    let g = &scene.geo;
    println!(
        "F = {:.1}  carte {}x{}  pli {}  rangee {:.1} px ({:.0} % de la largeur)",
        g.f,
        g.carte_l,
        g.carte_h,
        g.pli,
        g.rangee(),
        100.0 * g.rangee() / g.largeur as f32
    );
    if !opt.legende_imposee {
        scene.poser_phase(chrono.phase_affichee(), chrono.lumiere_basse());
    }
    // La capture est un instant fige : le variateur ne peut pas s'y ecouler, on
    // le pose donc directement au bout de sa course.
    scene.figer_variateur();
    println!(
        "chiffres : {:?}  etats : {:?}  lumiere {}",
        scene.chiffres(),
        scene.etats(),
        if chrono.lumiere_basse() { "basse" } else { "pleine" }
    );
    capture::capturer(&gpu, l, h, chemin, |gpu, vue| {
        scene.dessiner(gpu, &mut typo, vue, (l, h))
    })
    .expect("ecriture du PNG impossible");
    println!("-> {}", chemin.display());
}

fn capturer_ilot(opt: &Options, chrono: &Chrono, chemin: &std::path::Path) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let gpu = Gpu::ouvrir_nomme(instance, opt.adaptateur.as_deref());
    println!("adaptateur : {}", gpu.nom());
    let mut typo = Typographie::nouvelle(&gpu);
    // La taille du texte du systeme (N7) multiplie l'echelle demandee, comme
    // dans l'application ; le temoin du pas 7 la force.
    let facteur_texte = crate::systeme::taille_du_texte();
    let e = opt.echelle * facteur_texte;
    println!("taille du texte : facteur {facteur_texte:.2} ; echelle effective {e}");
    let (l, h) = (
        (ilot::DEPLOYE_L * e).round() as u32,
        (ilot::DEPLOYE_H * e).round() as u32,
    );
    // En capture on suppose toujours l'alpha par pixel disponible : c'est le
    // rendu qu'il faut juger, pas le repli.
    let mut vitrine =
        ilot::Vitrine::nouvelle(&gpu, &mut typo, capture::FORMAT, (l, h), e, true);
    vitrine.exposant = opt.exposant;
    vitrine.rayon = opt.rayon;
    vitrine.sans_contenu = opt.sans_contenu;
    vitrine.imposer_lisere(opt.lisere);
    if let Some(f) = opt.lisere {
        println!("lisere : force imposee {f:.2} (instrument, hors chemin normal)");
    } else {
        println!(
            "lisere : force {:.2} de la table a l'echelle {e}{}",
            galet::arbitree::force(if opt.epreuve_echelle { e + 0.25 } else { e }),
            if opt.epreuve_echelle { " + 0,25 (EPREUVE : table lue a la mauvaise echelle)" } else { "" }
        );
    }
    vitrine.imposer_epreuve_echelle(opt.epreuve_echelle);
    vitrine.imposer_lampe(opt.lampe);
    vitrine.imposer_epreuve_pousse(opt.epreuve_pousse);
    vitrine.imposer_epreuve_tiges(opt.epreuve_tiges);
    if opt.epreuve_tiges {
        println!("cristaux : EPREUVE, la texture d'avant m-g3 (tiges tardives non corrigees)");
    }
    // La matiere visee : le verre si un fond est pose, sauf `--matiere` ; le
    // mot VERRE / GALET est peint (l'alpha par pixel est suppose en capture),
    // et la sous-ligne annonce le triple clic sous le pointeur.
    // Les reglages du systeme (pas 7, DESIGN-ILOT.md §8.1) : transparence
    // coupee, economiseur de batterie ou theme de contraste, le verre n'est
    // pas permis ; le galet est opaque, sans action de matiere, quel que soit
    // le choix. Le temoin du pas 7 force chacun et compare au pixel.
    let verre_permis = crate::systeme::verre_permis();
    vitrine.matiere_visee = verre_permis.is_ok() && opt.matiere_visee.unwrap_or(opt.verre_fond.is_some());
    vitrine.matiere_possible = verre_permis.is_ok();
    if let Err(pourquoi) = verre_permis {
        println!("reglages : {pourquoi} ; le verre n'est pas permis, galet opaque sans action de matiere");
    }
    vitrine.contraste = crate::systeme::theme_de_contraste();
    if let Some(c) = vitrine.contraste {
        println!(
            "contraste : fond {:?} texte {:?} surbrillance {:?} texte de surbrillance {:?} ; bordure {} px logiques",
            c.fond, c.texte, c.surbrillance, c.texte_surbrillance, 2.0
        );
    }
    vitrine.imposer_survol(match opt.survol.as_deref() {
        Some("matiere") => Some(chrono::Action::Matiere),
        Some("pause") => Some(chrono::Action::Pause),
        Some("suivant") => Some(chrono::Action::Suivant),
        _ => None,
    });
    println!(
        "matiere : {} visee ; mot « {} » ; geste « {} »{}",
        if vitrine.matiere_visee { "verre" } else { "galet opaque" },
        if vitrine.matiere_visee { "GALET" } else { "VERRE" },
        ilot::GESTE_MATIERE,
        opt.survol.as_deref().map(|s| format!(" ; survol : {s}")).unwrap_or_default()
    );
    vitrine.composer(&mut typo, chrono, opt.p);
    let m = vitrine.mesures();
    for (nom, b) in vitrine.boites() {
        println!("boite {nom} : {:.2} {:.2} {:.2} {:.2}", b[0], b[1], b[2], b[3]);
    }
    for (action, z) in vitrine.zones() {
        println!("zone {action:?} : {:.2} {:.2} {:.2} {:.2}", z[0], z[1], z[2], z[3]);
    }
    // La lampe de cette trame : son intensite suit la hauteur du galet, comme
    // le conge (`galet::lampe::intensite`), et le journal la dit.
    let suit = m.h as f32 / (galet::arbitree::HAUTEUR_DU_CONGE * e);
    match opt.lampe {
        Some(l) => println!(
            "lampe : intensite {l:.4} (niveau {:.2}) imposee (instrument, hors chemin normal)",
            galet::lampe::niveau(l)
        ),
        None => println!(
            "lampe : niveaux {:.0} deploye / {:.0} replie de galet::lampe ; a cette trame (hauteur {} px, suit {suit:.4}) intensite {:.4}, niveau {:.2}",
            galet::lampe::NIVEAU_DEPLOYE,
            galet::lampe::NIVEAU_REPLIE,
            m.h,
            galet::lampe::intensite(suit),
            galet::lampe::niveau(galet::lampe::intensite(suit))
        ),
    }
    if verre_permis.is_ok() {
        poser_le_fond(opt, &gpu, &mut vitrine, &m);
    }
    // La transformation (GIVRE.md), figee a un etat ou a un instant ; sans
    // verre permis, il n'y en a pas.
    vitrine.matiere = if verre_permis.is_ok() { opt.matiere() } else { None };
    vitrine.origine_fonte = opt.origine;
    if let Some((g, f)) = vitrine.matiere {
        // Sans `--origine`, la trame lit le centre du galet TEL QU'IL EST : a
        // p = 1 c'est (160 ; 74), le centre du deploye, et plus bas c'est celui
        // du galet de ce p. Le journal dit ce que la trame fait, pas une
        // constante.
        let o = opt.origine.unwrap_or((m.l / 2.0 / e, m.h / 2.0 / e));
        println!(
            "transformation : givre {g:.4}  fonte {f:.4}  origine ({:.2}, {:.2}) logiques{}{}",
            o.0,
            o.1,
            opt.instant.map(|t| format!("  instant {t} ms")).unwrap_or_default(),
            if opt.retour { "  retour" } else { "" }
        );
    }
    println!(
        "p = {:.2}  galet {}x{} a ({}, {})  rayon {:.1}  exposant {:.2}  phase « {} »  reste {}",
        opt.p,
        m.l,
        m.h,
        m.x,
        m.y,
        opt.rayon.unwrap_or(m.rayon),
        opt.exposant.unwrap_or(m.n),
        chrono.phase_affichee(),
        chrono.temps_restant()
    );
    println!("largeurs des lignes : {:?}", vitrine.largeurs());
    // `--ordre=MS` : la premiere transformation n'est ordonnee qu'a MS ms de la
    // construction du rendu, pour que le releve de m-g1 se lise a un instant
    // choisi et non a celui, quelconque, ou la capture est prete.
    if let Some(ms) = opt.ordre {
        let cible = std::time::Duration::from_millis(ms);
        let age = vitrine.age();
        if age < cible {
            std::thread::sleep(cible - age);
        }
        println!(
            "ordre : la transformation est ordonnee a {:.1} ms de la construction du rendu (--ordre={ms})",
            vitrine.age().as_secs_f64() * 1e3
        );
    }
    // Sous « moins de mouvement » (m5), une mi-course est un fondu enchaine :
    // le replie en entier, puis le deploye par-dessus a l'opacite p. Le temoin
    // du pas 7 compare l'image a la moyenne lineaire des deux repos.
    let enchaine = crate::systeme::mouvement_reduit() && opt.p > 0.0 && opt.p < 1.0;
    if enchaine {
        println!(
            "mouvement reduit : fondu enchaine, le replie en entier puis le deploye a l'opacite {:.2} par-dessus",
            opt.p
        );
    }
    capture::capturer(&gpu, l, h, chemin, |gpu, vue| {
        if enchaine {
            let fondu = vitrine.opacite;
            vitrine.superposer = None;
            vitrine.composer(&mut typo, chrono, 0.0);
            vitrine.dessiner(gpu, &mut typo, vue);
            // Le deploye en entier dans l'image, posee a p fois le fondu.
            vitrine.opacite = 1.0;
            vitrine.composer(&mut typo, chrono, 1.0);
            vitrine.superposer = Some(opt.p * fondu);
            vitrine.dessiner(gpu, &mut typo, vue);
            vitrine.superposer = None;
            vitrine.opacite = fondu;
        } else {
            vitrine.dessiner(gpu, &mut typo, vue)
        }
    })
    .expect("ecriture du PNG impossible");
    // Le releve de m-g1 : ce que la premiere transformation a attendu de la
    // pousse des cristaux, et le g que la premiere image lirait, calcule par
    // `Matiere::a_l_instant` pour une trame de 1/144 s apres cette attente
    // (le temoin : g <= 0,05 ; l'epreuve, la pousse dans la trame : g > 0,9).
    if let Some(r) = vitrine.pousse_relevee() {
        let g = matiere::Matiere::a_l_instant(r.attente.as_secs_f32() + 1.0 / 144.0, true, None)
            .etat()
            .0;
        println!(
            "pousse des cristaux (m-g1) : {:.1} ms {} ; premiere transformation ordonnee a {:.1} ms de la construction ; attendue {:.1} ms ; g a la premiere image, calcule pour une trame de 6,94 ms apres l'attente : {g:.4}",
            r.duree.as_secs_f64() * 1e3,
            if r.dans_le_fil { "dans le fil du lancement" } else { "dans la trame (EPREUVE)" },
            r.ordre.as_secs_f64() * 1e3,
            r.attente.as_secs_f64() * 1e3
        );
    }
    println!("-> {}", chemin.display());
}

/// Pose le PNG de `--verre-fond` sous la vitrine, comme le bureau le serait.
fn poser_le_fond(opt: &Options, gpu: &Gpu, vitrine: &mut ilot::Vitrine, m: &ilot::Mesures) {
    let Some(chemin_fond) = &opt.verre_fond else { return };
    let e = opt.echelle * crate::systeme::taille_du_texte();
    let (pixels, fl, fh) = match lire_png_rgba(chemin_fond) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("fond illisible, {} : {e}", chemin_fond.display());
            std::process::exit(2);
        }
    };
    let marge = galet::marge(e);
    let (dx, dy) = opt
        .verre_decalage
        .unwrap_or((marge - m.x as i32, marge - m.y as i32));
    // La surface est posee a l'origine du bureau ; le PNG commence donc a
    // (-dx, -dy) pour que le pixel (0, 0) de la surface tombe en (dx, dy).
    vitrine.televerser_fond(
        gpu,
        &pixels,
        galet::Source {
            x: -dx,
            y: -dy,
            l: fl,
            h: fh,
        },
    );
    vitrine.origine_fenetre = Some((0, 0));
    println!(
        "verre : fond {} ({fl}x{fh}), surface a ({dx}, {dy}) dans le fond, marge {marge}",
        chemin_fond.display()
    );
}

/// Ecrit la texture des cristaux du galet deploye (`givre.rs`) en PNG : R
/// l'arrivee, G la modulation, B l'epaisseur.
fn capturer_cristaux(chemin: &std::path::Path, epreuve_tiges: bool) {
    // `--epreuve-tiges` : la texture d'avant m-g3, pour que le temoin des tiges
    // tardives puisse tomber sur ce binaire meme.
    if epreuve_tiges {
        println!("cristaux : EPREUVE, la texture d'avant m-g3 (tiges tardives non corrigees)");
    }
    let c = givre::pousser_avec(
        ilot::DEPLOYE_L,
        ilot::DEPLOYE_H,
        ilot::DEPLOYE_RAYON,
        ilot::DEPLOYE_N,
        givre::GRAINE,
        !epreuve_tiges,
    );
    println!(
        "cristaux : {}x{} texels ({} par pixel logique), {} sites, {} segments, duree {:.1} tics, largeur du front {:.4}",
        c.largeur,
        c.hauteur,
        givre::SURECHANTILLON,
        c.sites,
        c.segments,
        c.duree,
        c.largeur_front
    );
    // Le temoin de m-g3 : aucune tige n'arrive apres la mosaique qu'elle
    // touche. `captures/integration-apres-epreuve/pas3/tiges-tardives.py` le
    // recompte sur le PNG par un autre chemin.
    println!("tiges tardives (m-g3, doit valoir 0) : {}", c.tiges_tardives());
    if let Some(parent) = chemin.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("dossier de la capture");
        }
    }
    let fichier = std::fs::File::create(chemin).expect("ecriture du PNG impossible");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(fichier), c.largeur, c.hauteur);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut ecrivain = enc.write_header().expect("en-tete du PNG");
    ecrivain.write_image_data(&c.texels).expect("donnees du PNG");
    ecrivain.finish().expect("fin du PNG");
    println!("-> {}", chemin.display());
}

/// L'instrument du cout de la transformation (`cout_givre.rs`).
fn mesurer_givre(opt: &Options, chrono: &Chrono, journal: &std::path::Path) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let (gpu, horodate) = Gpu::ouvrir_nomme_horodate(instance, opt.adaptateur.as_deref());
    println!("adaptateur : {}  horodateur : {}", gpu.nom(), if horodate { "oui" } else { "non" });
    let mut typo = Typographie::nouvelle(&gpu);
    let e = opt.echelle;
    let (l, h) = (
        (ilot::DEPLOYE_L * e).round() as u32,
        (ilot::DEPLOYE_H * e).round() as u32,
    );
    let mut vitrine = ilot::Vitrine::nouvelle(&gpu, &mut typo, capture::FORMAT, (l, h), e, true);
    vitrine.composer(&mut typo, chrono, 1.0);
    let m = vitrine.mesures();
    if opt.verre_fond.is_none() {
        eprintln!("--mesure-givre demande --verre-fond");
        std::process::exit(2);
    }
    poser_le_fond(opt, &gpu, &mut vitrine, &m);
    let cible = gpu.peripherique.create_texture(&wgpu::TextureDescriptor {
        label: Some("cible de mesure"),
        size: wgpu::Extent3d {
            width: l,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: capture::FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let vue = cible.create_view(&Default::default());
    if let Err(e) = cout_givre::mesurer(&gpu, horodate, &mut typo, &mut vitrine, &vue, opt, journal) {
        eprintln!("mesure impossible : {e}");
        std::process::exit(1);
    }
    println!("-> {}", journal.display());
}

/// Un PNG a huit bits en octets RGBA.
fn lire_png_rgba(chemin: &std::path::Path) -> Result<(Vec<u8>, u32, u32), String> {
    let fichier = std::fs::File::open(chemin).map_err(|e| e.to_string())?;
    let mut lecteur = png::Decoder::new(std::io::BufReader::new(fichier))
        .read_info()
        .map_err(|e| e.to_string())?;
    let mut tampon = vec![0; lecteur.output_buffer_size()];
    let info = lecteur.next_frame(&mut tampon).map_err(|e| e.to_string())?;
    if info.bit_depth != png::BitDepth::Eight {
        return Err("seuls les PNG a huit bits sont lus".into());
    }
    let n = (info.width * info.height) as usize;
    let pixels = match info.color_type {
        png::ColorType::Rgba => tampon[..n * 4].to_vec(),
        png::ColorType::Rgb => tampon[..n * 3]
            .chunks(3)
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        autre => return Err(format!("type de couleur non lu : {autre:?}")),
    };
    Ok((pixels, info.width, info.height))
}

// --- L'application -----------------------------------------------------------

struct Horloge {
    fenetre: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    scene: Scene,
    /// Date de la trame precedente, pour mesurer le temps ecoule plutot que de
    /// le supposer : une trame perdue ne doit pas ralentir le volet.
    trame_precedente: std::time::Instant,
    /// Trames dessinees depuis le debut de la bascule en cours.
    trames: u32,
    /// Trames de la bascule PRECEDENTE, gardees a part. Sans ce champ, la trace
    /// afficherait toujours zero : le compteur en cours est remis a zero a la
    /// fin de la bascule, donc bien avant que new_events ne le lise.
    trames_precedentes: u32,
    debut_bascule: std::time::Instant,
}

struct Etat {
    gpu: Gpu,
    /// Une seule typographie pour les deux fenetres : les polices pesent plus
    /// d'un megaoctet et leurs caches de faconnage se partagent sans effort,
    /// puisque les deux surfaces viennent du meme peripherique.
    typo: Typographie,
    horloge: Horloge,
    /// L'ilot mesure lui-meme le temps entre ses trames : les siennes et celles
    /// de l'horloge ne tombent pas aux memes instants, et c'est son ressort qui
    /// sait quand son animation commence.
    ilot: Ilot,
}

struct Application {
    /// Le fil de la sonde du bureau et celui du relevé passent par lui.
    proxy: winit::event_loop::EventLoopProxy<Evenement>,
    opt: Options,
    chrono: Chrono,
    etat: Option<Etat>,
    plein_ecran: bool,
    /// Derniere avance du chrono, pour le faire courir sur le temps REEL.
    derniere_pendule: std::time::Instant,
    /// La conscience du DPI : acceptee ou non, et relue avant la boucle.
    #[cfg(windows)]
    conscience: bureau::Conscience,
    #[cfg(windows)]
    releve: Option<releve::Releve>,
    #[cfg(windows)]
    oeil: Option<oeil::Oeil>,
    /// Le relevé de la matiere (`--releve-matiere`) : le dossier de sortie et
    /// l'instant du depart.
    releve_matiere: Option<(std::path::PathBuf, std::time::Instant)>,
    /// Le relevé de la presence (`--releve-presence`) : le dossier de sortie,
    /// l'instant du depart, et le journal des decisions.
    releve_presence: Option<(std::path::PathBuf, std::time::Instant, Vec<String>)>,
    /// La seconde dont l'heure est posee : un reveil par un geste dans la meme
    /// seconde ne repose pas l'heure et ne relance pas la bascule.
    seconde_posee: Option<(u32, u32, u32)>,
    /// Maj enfoncee, pour Maj+Tab (winit ne la livre pas avec la touche).
    maj: bool,
    /// Le journal date de la boucle : le premier tic de la sonde et la premiere
    /// trame de l'ilot, une fois chacun.
    sonde_vue: bool,
    trame_ilot_vue: bool,
    trame_horloge_vue: bool,
    /// La premiere lecture du premier plan est faite.
    presence_lue: bool,
}

/// Les deux evenements venus d'un fil a part.
enum Evenement {
    /// Le tic du relevé de la capture.
    #[cfg_attr(not(windows), allow(dead_code))]
    Tic,
    /// Le tic de la sonde du bureau, qui fait avancer le verre sous un ilot
    /// immobile.
    #[cfg_attr(not(windows), allow(dead_code))]
    Sonde,
}

impl Application {
    /// Reporte au dessin ce que le chrono vient de dire.
    /// La regle de presence (DESIGN.md §6, « Presence »), a chaque changement
    /// de premier plan : l'ilot est la si et seulement si la fenetre passee
    /// devant, l'ilot mis a part, n'est pas l'horloge, et que le systeme ne dit
    /// ni plein ecran, ni jeu, ni presentation ; il parait sur l'ecran qui a la
    /// plus grande part de cette fenetre.
    fn reevaluer_presence(&mut self, devant: isize) {
        let Some(etat) = self.etat.as_mut() else { return };
        let horloge = hwnd_de(&etat.horloge.fenetre);
        #[cfg(windows)]
        let ilot = etat.ilot.hwnd_public().unwrap_or(0);
        #[cfg(not(windows))]
        let ilot = 0;
        if devant == ilot {
            // L'ilot mis a part : un clic sur lui ne change rien a sa presence.
            return;
        }
        let titre = crate::systeme::titre(devant);
        let decision = if devant == horloge {
            etat.ilot.appliquer_la_regle(false, None, "l'horloge est devant");
            "retire : l'horloge est devant".to_string()
        } else if let Some(pourquoi) = crate::systeme::occupation() {
            etat.ilot.appliquer_la_regle(false, None, pourquoi);
            format!("retire : {pourquoi}")
        } else {
            let ecran = crate::systeme::ecran_de(devant);
            let nom = ecran.as_ref().map(|e| e.nom.clone()).unwrap_or_else(|| "ecran inconnu".to_string());
            etat.ilot.appliquer_la_regle(true, ecran.as_ref(), &format!("« {titre} » est devant"));
            format!("parait sur {nom}")
        };
        let ligne = format!(
            "presence : premier plan {devant:#x} « {titre} » -> {decision} ({:.0} ms du depart)",
            depuis_le_depart()
        );
        println!("{ligne}");
        if let Some((_, _, journal)) = self.releve_presence.as_mut() {
            journal.push(ligne);
        }
    }

    /// Un geste de l'ilot, souris ou clavier (DECISION-APRES-EPREUVE.md §B).
    fn executer(&mut self, geste: ilot::Geste) {
        let Some(etat) = self.etat.as_mut() else { return };
        match geste {
            ilot::Geste::Action(chrono::Action::Matiere) => {
                // L'ilot execute lui-meme l'action de la matiere.
                etat.ilot.basculer_matiere(true);
            }
            ilot::Geste::Action(action) => {
                self.chrono.agir(action);
                self.accorder();
            }
            ilot::Geste::RendreLHorloge => {
                // L'horloge revient devant ; la regle de presence, qui lit le
                // premier plan, retire l'ilot. Le processus vient de recevoir
                // le clic : il a le droit de poser le premier plan.
                etat.horloge.fenetre.focus_window();
                println!("geste : l'horloge est appelee devant (focus_window)");
            }
        }
    }

    fn accorder(&mut self) {
        let Some(etat) = self.etat.as_mut() else { return };
        if !self.opt.legende_imposee {
            let change = etat
                .horloge
                .scene
                .poser_phase(self.chrono.phase_affichee(), self.chrono.lumiere_basse());
            if change {
                // Le variateur part d'ICI. Sans cette remise a l'heure, sa
                // premiere trame mesurerait le temps ecoule depuis la fin de la
                // derniere bascule : un clic a 800 ms dans la seconde donnerait
                // dt = 500 ms, donc 40 % de la course en une seule trame, la ou
                // DESIGN.md §7 veut « une lampe sur variateur » sur 1200 ms.
                etat.horloge.trame_precedente = std::time::Instant::now();
            }
        }
        etat.horloge.fenetre.request_redraw();
        etat.ilot.fenetre.request_redraw();
    }

    fn basculer_plein_ecran(&mut self) {
        let Some(etat) = self.etat.as_mut() else { return };
        self.plein_ecran = !self.plein_ecran;
        if self.plein_ecran {
            etat.horloge
                .fenetre
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
            // Le plein ecran repose la marque et reecrit le style : on refait.
            println!("horloge : {}", rendre_reductible_par_win_d(&etat.horloge.fenetre));
        } else {
            etat.horloge.fenetre.set_fullscreen(None);
            let _ = etat.horloge.fenetre.request_inner_size(
                winit::dpi::PhysicalSize::new(self.opt.taille.0, self.opt.taille.1),
            );
        }
        etat.horloge.fenetre.request_redraw();
    }
    /// Peint et presente UNE trame de l'horloge. Rend faux quand la chaine
    /// d'echange n'a rien donne : l'appelant n'a alors rien a rearmer.
    ///
    /// Elle est ecrite a part parce que deux chemins y mènent : la demande de
    /// redessin ordinaire, et — voie « invisible » de l'ouverture — la toute
    /// premiere trame, peinte dans `resumed` alors que la fenetre n'est pas
    /// encore montree. Une fenetre cachee ne recoit pas `WM_PAINT`, donc pas de
    /// `RedrawRequested` : sans ce chemin direct, l'horloge cachee resterait
    /// cachee pour toujours. Mesure faite, et c'est ce qui est arrive au
    /// premier essai.
    fn peindre_l_horloge(&mut self) -> bool {
        let Some(etat) = self.etat.as_mut() else {
            return false;
        };
        let maintenant = std::time::Instant::now();
        let dt = maintenant
            .duration_since(etat.horloge.trame_precedente)
            .as_secs_f32()
            * 1000.0;
        etat.horloge.trame_precedente = maintenant;
        etat.horloge.scene.avancer(dt);

        let trame = etat.horloge.surface.get_current_texture();
        use wgpu::CurrentSurfaceTexture as T;
        let trame = match trame {
            T::Success(t) | T::Suboptimal(t) => t,
            T::Outdated | T::Lost => {
                etat.horloge
                    .surface
                    .configure(&etat.gpu.peripherique, &etat.horloge.config);
                return false;
            }
            _ => return false,
        };
        let vue = trame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let taille = (etat.horloge.config.width, etat.horloge.config.height);
        let Etat { gpu, typo, horloge, .. } = etat;
        horloge.scene.dessiner(gpu, typo, &vue, taille);
        gpu.file.present(trame);
        if !self.trame_horloge_vue {
            self.trame_horloge_vue = true;
            println!(
                "boucle : premiere trame de l'horloge presentee a {:.0} ms du depart",
                depuis_le_depart()
            );
        }
        true
    }
}

impl ApplicationHandler<Evenement> for Application {
    #[cfg(windows)]
    fn user_event(&mut self, el: &ActiveEventLoop, evenement: Evenement) {
        let Some(etat) = self.etat.as_mut() else {
            return;
        };
        match evenement {
            Evenement::Tic => {
                if let Some(releve) = self.releve.as_mut() {
                    releve.pas(&mut etat.ilot, el);
                }
                if let Some(oeil) = self.oeil.as_mut() {
                    oeil.pas(&mut etat.ilot, el);
                }
                // Le relevé de la presence : huit secondes de fenetres reelles
                // livrees a un script, puis ce que l'ilot a decide, et la sortie.
                if let Some((dossier, depuis, journal)) = self.releve_presence.as_ref() {
                    if self.sonde_vue && depuis.elapsed() >= std::time::Duration::from_secs(self.opt.duree.unwrap_or(8)) {
                        let position = etat.ilot.fenetre.outer_position().ok().map(|p| (p.x, p.y));
                        let decisions = journal.iter().map(|l| format!("  {l}")).collect::<Vec<_>>().join("\n");
                        let texte = format!(
                            "Le relevé de la presence : les vraies fenetres pendant {:.2} s depuis le premier tic de la sonde, avec la vraie regle.\ndecisions :\n{}\na la fin : {} ; trames demandees retire et non presentees (apres la trame vide qui pose la region) : {} ; position de la fenetre {:?}\n",
                            depuis.elapsed().as_secs_f64(),
                            decisions,
                            if etat.ilot.est_presente() { "present" } else { "retire" },
                            etat.ilot.trames_retire,
                            position
                        );
                        let _ = std::fs::create_dir_all(dossier);
                        let chemin = dossier.join("releve-presence.txt");
                        std::fs::write(&chemin, &texte).expect("ecriture du relevé de la presence");
                        print!("{texte}");
                        println!("-> {}", chemin.display());
                        el.exit();
                    }
                }
                // Le relevé de la matiere : trois secondes de fenetres reelles,
                // puis les compteurs de la capture, et la sortie.
                if let Some((dossier, depuis)) = self.releve_matiere.as_ref() {
                    // Jamais avant un tic vivant de la sonde : sur une machine
                    // chargee la boucle peut rester morte plus de trois
                    // secondes apres `resumed`, et un relevé pris alors ne
                    // compterait que du vide.
                    if self.sonde_vue && depuis.elapsed() >= std::time::Duration::from_secs(3) {
                        let c = &etat.ilot.suivi.compteurs;
                        let v = &etat.ilot.verre_compteurs;
                        let texte = format!(
                            "Le relevé de la matiere : les vraies fenetres pendant {:.2} s depuis le premier tic de la sonde, puis les compteurs.\n\
                             matiere : {} ; verre possible : {} ; alpha par pixel : {}\n\
                             sondes {} ; trames {} ; televersements {} ; oublis {} ; identiques {} ; differentes {}\n\
                             tours {} ; neuves {} ; inchanges {} ; attentes {} ; perdus {} ; fermes {} ; ouvertures {} ; echecs d'ouverture {} ; forcees {}\n\
                             capteur ouvert a la fin : {}\n",
                            depuis.elapsed().as_secs_f64(),
                            if etat.ilot.verre_choisi() { "verre" } else { "galet opaque" },
                            etat.ilot.matiere_possible(),
                            etat.ilot.alpha_par_pixel(),
                            v.sondes, v.trames, v.televersements, v.oublis, v.identiques, v.differentes,
                            c.tours, c.neuves, c.inchanges, c.attentes, c.perdus, c.fermes, c.ouvertures, c.echecs_ouverture, c.forcees,
                            etat.ilot.suivi.capteur().is_some()
                        );
                        let _ = std::fs::create_dir_all(dossier);
                        let chemin = dossier.join("releve-matiere.txt");
                        std::fs::write(&chemin, &texte).expect("ecriture du relevé de la matiere");
                        print!("{texte}");
                        println!("-> {}", chemin.display());
                        el.exit();
                    }
                }
            }
            Evenement::Sonde => {
                if !self.sonde_vue {
                    self.sonde_vue = true;
                    println!("boucle : premier tic de la sonde a {:.0} ms du depart", depuis_le_depart());
                    // Le relevé de la matiere compte ses trois secondes depuis
                    // le premier tic vivant, pas depuis `resumed` : la boucle
                    // peut rester morte une a deux secondes entre les deux sur
                    // une machine chargee, et ce temps n'est celui de rien.
                    if let Some(r) = self.releve_matiere.as_mut() {
                        r.1 = std::time::Instant::now();
                    }
                    if let Some(r) = self.releve_presence.as_mut() {
                        r.1 = std::time::Instant::now();
                    }
                }
                // Le relevé fait ses propres tours : il ne laisse la sonde agir
                // que pendant sa phase « sonde seule », sans quoi ses compteurs
                // de phase compteraient les deux.
                if self.releve.as_ref().is_none_or(|r| r.laisse_sonder()) {
                    etat.ilot.sonder();
                }
                // Une duplication neuve ne rend que les changements du bureau :
                // tant que l'ilot attend sa premiere image, l'horloge presente
                // une trame pour lui en donner un.
                if etat.ilot.attend_une_premiere_image() {
                    etat.horloge.fenetre.request_redraw();
                }
            }
        }
    }

    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.etat.is_some() {
            return;
        }

        // --- L'icone de l'application ---------------------------------------
        // Un seul fichier `assets/veilleuse.ico`, quatorze entrees, embarque en
        // ressource sous l'identifiant 1 par `build.rs`. Il sert les TROIS
        // chemins : l'icone du fichier (l'explorateur, un raccourci, la
        // recherche, ET LA BARRE DES TACHES), l'icone petite (`ICON_SMALL`,
        // barre de titre) et l'icone grande (`ICON_BIG`, Alt+Tab). Les deux
        // dernieres sont chargees ici, depuis cette meme ressource, a la taille
        // EXACTE que le systeme emploie — jamais par `Icon::from_rgba`, qui ne
        // porte qu'une taille et laisse Windows redimensionner tout le reste.
        //
        // ~~`ICON_BIG`, Alt+Tab et barre des taches~~ : la barre des taches de
        // Windows 11 ne dessine NI `ICON_BIG` NI `ICON_SMALL`. Les changer de
        // l'exterieur ne change pas son bouton d'un pixel ; elle prend l'icone
        // du FICHIER par le cache du shell et la met a l'echelle de son bouton
        // (`INTEGRATION-ICONE.md`, « En plus des douze pas », mesure et
        // controle).
        #[cfg(windows)]
        let (icone_petite, icone_grande) = {
            use winit::platform::windows::IconExtWindows;
            let (p, g) = systeme::tailles_d_icone();
            let charger = |c: u32| match winit::window::Icon::from_resource(
                1,
                Some(winit::dpi::PhysicalSize::new(c, c)),
            ) {
                Ok(i) => Some(i),
                // Sans compilateur de ressources, `build.rs` n'embarque rien et
                // le chargement echoue. Le programme continue sans icone de
                // fenetre — mais il le DIT, au lieu de livrer une icone
                // generique en silence.
                Err(e) => {
                    eprintln!("icone de {c} px introuvable dans la ressource 1, {e}");
                    None
                }
            };
            (charger(p), charger(g))
        };

        // --- L'horloge -----------------------------------------------------
        // --- L'OUVERTURE : rien ne parait avant d'avoir une image ------------
        // Une fenetre de haut niveau superposee que Windows montre avant que
        // rien ne l'ait peinte est composee BLANC PUR : c'est sa surface de
        // redirection. `OUVERTURE-LE-BLANC-ETEINT.md` l'etablit par une table
        // de douze barreaux sur une fenetre Win32 nue — ni le cadre, ni le
        // style etendu, ni la taille, ni la retaille n'y changent rien ; seules
        // `WS_POPUP` et l'absence de monstration l'eteignent.
        //
        // Les deux fenetres naissent donc invisibles. Elles seront peintes a la
        // fin de `resumed`, puis montrees, et ce qui parait a l'ecran portera
        // deja une image. Ce n'est pas un blanc repeint : c'est un blanc qui
        // n'existe plus, faute de fenetre a l'ecran pour le porter.
        let paraitre_des_la_creation = self.opt.voie_d_ouverture.is_some();

        let mut attributs = Window::default_attributes()
            .with_title("Veilleuse")
            .with_decorations(false)
            .with_visible(paraitre_des_la_creation)
            .with_resizable(false);
        #[cfg(windows)]
        {
            use winit::platform::windows::WindowAttributesExtWindows;
            // L'ombre d'une fenetre sans decoration ajoute une ligne claire de
            // 1 px en haut ; DESIGN.md §8 interdit toute ombre.
            //
            // ~~`.with_skip_taskbar(true)`~~ : l'horloge REPREND son bouton
            // dans la barre des taches (DESIGN.md §6). C'est la que l'icone de
            // l'application se voit le plus souvent, a 30 px sur l'ecran de
            // l'auteur. L'ilot, lui, garde le sien : il reste hors barre.
            attributs = attributs
                .with_undecorated_shadow(false)
                .with_skip_taskbar(false)
                .with_window_icon(icone_petite.clone())
                .with_taskbar_icon(icone_grande.clone());
            // Une des voies ECARTEES de l'arbitrage, rejouee.
            if self.opt.voie_d_ouverture.as_deref() == Some("sans-redirection") {
                attributs = attributs.with_no_redirection_bitmap(true);
            }
        }
        if self.opt.fenetre {
            attributs = attributs.with_inner_size(winit::dpi::PhysicalSize::new(
                self.opt.taille.0,
                self.opt.taille.1,
            ));
        }
        let horloge_f =
            Arc::new(el.create_window(attributs).expect("creation de fenetre refusee"));
        // La voie ECARTEE « fond noir », rejouee : la surface de redirection
        // est noircie a l'instant meme ou la fenetre existe, avant tout le
        // reste. Elle n'a de sens que sur une fenetre deja montree.
        #[cfg(windows)]
        if self.opt.voie_d_ouverture.as_deref() == Some("fond-noir") {
            let px = noircir_le_fond(&horloge_f);
            println!(
                "ouverture : fond de l'horloge noirci, {px} px, a {:.0} ms du depart",
                depuis_le_depart()
            );
        }

        // --- L'ilot ---------------------------------------------------------
        let mut attributs = Window::default_attributes()
            .with_title("Veilleuse — ilot")
            .with_decorations(false)
            .with_resizable(false)
            // Toujours demande, meme si l'adaptateur ne saura peut-etre pas le
            // tenir : c'est cet appel, et lui seul, qui demande a DWM de tenir
            // compte du canal alpha (ARCHITECTURE.md §1.2). Quand la chaine
            // d'echange ne suit pas, la fenetre est simplement opaque et la
            // region GDI fait seule la decoupe.
            .with_transparent(true)
            // La meme raison que l'horloge, et elle vaut davantage encore : la
            // fenetre de l'ilot est AU-DESSUS de tout, si bien que son blanc de
            // 400 x 185 survivrait meme a une horloge deja noire. Mesure faite :
            // l'horloge corrigee seule, l'ilot en laissait 400 px.
            .with_visible(paraitre_des_la_creation)
            .with_window_level(WindowLevel::AlwaysOnTop)
            // La taille du texte du systeme (N7) : toutes les cotes logiques
            // de l'ilot la suivent, la fenetre aussi.
            .with_inner_size(winit::dpi::LogicalSize::new(
                ilot::DEPLOYE_L * systeme::taille_du_texte(),
                ilot::DEPLOYE_H * systeme::taille_du_texte(),
            ));
        #[cfg(windows)]
        {
            use winit::platform::windows::WindowAttributesExtWindows;
            attributs = attributs
                .with_undecorated_shadow(false)
                .with_skip_taskbar(true)
                // La MEME ressource que l'horloge. L'ilot reste hors de la barre
                // des taches, mais il porte `WS_EX_APPWINDOW` et parait dans
                // Alt+Tab. Ce qu'il y montrerait SANS cette ligne n'est pas
                // mesure ; ce qui l'est, c'est qu'AVEC elle il porte les entrees
                // 20 et 40 a l'octet (`INTEGRATION-ICONE.md`, pas 8).
                .with_window_icon(icone_petite)
                .with_taskbar_icon(icone_grande);
        }
        let ilot_f = Arc::new(el.create_window(attributs).expect("creation de l'ilot refusee"));

        // La position memorisee ne vaut que si elle retombe sur un moniteur
        // branche aujourd'hui.
        let echelle = ilot_f.scale_factor();
        // Le relevé pose l'ilot a sa place par defaut, au-dessus de la zone que
        // l'horloge laisse noire, et ne la memorise pas : ses images ne doivent
        // pas dependre d'une position laissee par l'auteur, ni la deplacer.
        let position = Ilot::position_memorisee()
            .filter(|p| !self.opt.scripte() && Ilot::position_visible(el, *p))
            .unwrap_or_else(|| {
                let largeur = el
                    .primary_monitor()
                    .or_else(|| el.available_monitors().next())
                    .map(|m| m.size().width)
                    .unwrap_or(1920);
                Ilot::position_defaut(largeur, echelle)
            });
        ilot_f.set_outer_position(winit::dpi::PhysicalPosition::new(position.0, position.1));

        // --- L'epreuve de l'ouverture, etage 1 : les fenetres seules --------
        // Les deux fenetres de l'application, creees par le meme code, a la
        // meme taille, au meme endroit — et rien de wgpu. La fin de la sequence
        // reelle (le clic qui traverse, le plein ecran) est jouee quand meme,
        // pour que cet etage et le suivant ne different QUE par wgpu.
        if self.opt.epreuve_ouverture.as_deref() == Some("fenetres") {
            let _ = horloge_f.set_cursor_hittest(false);
            if !self.opt.fenetre {
                horloge_f.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
            tenir_l_epreuve_de_l_ouverture("fenetres");
        }

        // --- Le peripherique, partage --------------------------------------
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(el.owned_display_handle()),
        ));
        let horloge_s = instance
            .create_surface(horloge_f.clone())
            .expect("surface de l'horloge impossible a creer");
        let ilot_s = instance
            .create_surface(ilot_f.clone())
            .expect("surface de l'ilot impossible a creer");
        let gpu = Gpu::ouvrir_pour_deux(instance, &horloge_s, &ilot_s);

        // L'horloge ne recoit rien et ne bloque rien : le drapeau est pose une
        // fois pour toutes (ARCHITECTURE.md §2). Le clavier continue d'arriver
        // tant que la fenetre garde le focus.
        let _ = horloge_f.set_cursor_hittest(false);
        if !self.opt.fenetre {
            horloge_f.set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            self.plein_ecran = false;
        }
        // ~~`println!("horloge : {}", rendre_reductible_par_win_d(&horloge_f));`~~
        // Le bouton de reduction ne se pose plus ICI. Depuis que la fenetre
        // nait invisible, `set_visible(true)` passe par `apply_diff` de winit,
        // qui reecrit GWL_STYLE en entier a partir de ses propres drapeaux :
        // tout bit pose a la main entre-temps est efface. Mesure faite — le
        // style de l'horloge passait de 0x16020000 a 0x16000000, c'est-a-dire
        // que WS_MINIMIZEBOX disparaissait et que Win+D cessait de la reduire.
        // L'appel a donc lieu APRES que les fenetres paraissent, a la fin de
        // `resumed`.

        // --- L'epreuve de l'ouverture, etage 2 : les surfaces, sans chaine --
        // L'instance wgpu, les deux surfaces (`VkSurfaceKHR` sur Vulkan) et
        // l'ouverture du peripherique — le gros des 576 ms de `resumed`. Aucune
        // chaine d'echange : `configure` n'a pas encore ete appele.
        if self.opt.epreuve_ouverture.as_deref() == Some("surface") {
            tenir_l_epreuve_de_l_ouverture("surface");
        }

        let caps = horloge_s.get_capabilities(&gpu.adaptateur);
        // Bgra8UnormSrgb est en tete sur les six couples dorsal/adaptateur de
        // cette machine (ARCHITECTURE.md §1.3) ; on le prend s'il est la.
        let format = if caps.formats.contains(&wgpu::TextureFormat::Bgra8UnormSrgb) {
            wgpu::TextureFormat::Bgra8UnormSrgb
        } else {
            caps.formats[0]
        };
        let taille = horloge_f.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: taille.width.max(1),
            height: taille.height.max(1),
            // Fifo cale sur l'ecran et existe partout ; Immediate tourne a
            // 834 Hz pour rien, Mailbox manque sur Vulkan/Intel.
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        horloge_s.configure(&gpu.peripherique, &config);

        // --- L'epreuve de l'ouverture, etage 3 : la chaine d'echange --------
        // La chaine d'echange de l'horloge existe, a la taille de la fenetre,
        // et elle ne sera jamais presentee. C'est l'etage que le §5 de
        // `OUVERTURE-LE-NOIR.md` soupconnait sans pouvoir le montrer.
        if self.opt.epreuve_ouverture.as_deref() == Some("chaine") {
            tenir_l_epreuve_de_l_ouverture("chaine");
        }

        let mut typo = Typographie::nouvelle(&gpu);
        let scene = Scene::nouvelle(&gpu, &mut typo, format, config.width, config.height, &self.opt);
        // Un mode scripte qui dit sa matiere (`--matiere=verre`, l'oeil) la
        // prend sans lire ni ecrire `matiere.txt` : c'est celle de l'auteur.
        if self.opt.scripte() {
            matiere::forcer_le_choix(self.opt.matiere_visee);
        }
        let mut ilot = Ilot::nouveau(
            &gpu,
            ilot_f,
            ilot_s,
            &mut typo,
            self.opt.exposant,
            self.opt.rayon,
            true,
        );
        ilot.epreuve_sans_comparaison = self.opt.epreuve_boucle;
        ilot.epreuve_sans_repli = self.opt.epreuve_repli;
        ilot.sans_memoire = self.opt.scripte();
        if let Some(dossier) = self.opt.releve_matiere.clone() {
            self.releve_matiere = Some((dossier, std::time::Instant::now()));
        }
        if let Some(dossier) = self.opt.releve_presence.clone() {
            self.releve_presence = Some((dossier, std::time::Instant::now(), Vec::new()));
        }
        // La presence (DESIGN.md §6) : la regle se lit sur les changements de
        // premier plan, jamais par interrogation. Les instruments scriptes qui
        // ont besoin de l'ilot sous les yeux la forcent.
        if self.opt.presence_reelle() {
            let ok = crate::systeme::ecouter_le_premier_plan();
            println!(
                "presence : la regle se lit sur les changements de premier plan (SetWinEventHook {}) ; une alerte dure {} s (SPI_GETMESSAGEDURATION)",
                if ok { "pose" } else { "REFUSE : l'ilot restera retire" },
                crate::systeme::duree_du_message().as_secs()
            );
        } else {
            ilot.forcer_presence();
            println!("presence : forcee, mode scripte");
        }
        if self.opt.ilot_deploye {
            // Ne deploye ; la premiere trame pose les ressorts sur la cible,
            // sans mouvement (A3, la mesure du cout deploye).
            ilot.viser(true);
            println!("ilot : ne deploye (--ilot-deploye, mesure du cout)");
        }

        println!("adaptateur : {}", gpu.nom());
        #[cfg(windows)]
        println!(
            "capture du bureau : conscience du DPI {} (relue {}, couche {:?}), affinite de l'ilot relue {:?} (17 = exclu)",
            if self.conscience.acceptee { "declaree" } else { "refusee" },
            self.conscience.lue,
            self.conscience.couche,
            ilot.affinite
        );
        #[cfg(windows)]
        if let Some(dossier) = self.opt.releve_bureau.clone() {
            self.releve = Some(releve::Releve::nouveau(
                dossier,
                self.opt.epreuve_dpi,
                self.conscience.clone(),
                gpu.nom().to_string(),
                echelle,
            ));
        }
        #[cfg(windows)]
        if let Some(dossier) = self.opt.oeil.clone() {
            self.oeil = Some(oeil::Oeil::nouveau(
                dossier,
                self.opt.epreuve_oeil.as_deref().unwrap_or(""),
                gpu.nom().to_string(),
                echelle,
            ));
        }
        // La sonde du bureau. Sa cadence est celle de l'ecran, et ce n'est pas
        // un reglage : hors du galet, le compositeur montre le bureau vivant a
        // chaque rafraichissement. Un verre sonde moins souvent se decalerait
        // de ce qui l'entoure au bord meme du galet ; plus souvent, il n'y a
        // rien de neuf a lire. Le fil tourne aussi pendant le relevé, qui ne
        // laisse passer ses tics que dans sa phase « sonde seule ».
        let millihertz = ilot
            .fenetre
            .current_monitor()
            .and_then(|m| m.refresh_rate_millihertz());
        ilot.hz = millihertz.filter(|&h| h > 0).unwrap_or(60_000) as f32 / 1000.0;
        {
            let (rl, rh) = crate::systeme::rectangle_du_double_clic();
            println!(
                "gestes : appui maintenu {} ms (SPI_GETMOUSEHOVERTIME), triple clic {} ms entre \
                 appuis (GetDoubleClickTime) dans {:.0} x {:.0} px physiques (SM_CXDOUBLECLK), \
                 traversant sous un demi-pixel d'avance a {:.0} Hz",
                crate::systeme::delai_de_survol().as_millis(),
                crate::systeme::delai_du_double_clic().as_millis(),
                rl,
                rh,
                ilot.hz
            );
        }
        #[cfg(windows)]
        if !self.opt.epreuve_sans_sonde {
            let periode = std::time::Duration::from_secs_f64(
                1000.0 / millihertz.filter(|&h| h > 0).unwrap_or(60_000) as f64,
            );
            println!(
                "sonde du bureau : toutes les {:.2} ms (rafraichissement annonce {:?} mHz)",
                periode.as_secs_f64() * 1000.0,
                millihertz
            );
            let proxy = self.proxy.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(periode);
                if proxy.send_event(Evenement::Sonde).is_err() {
                    break;
                }
            });
        }
        println!("horloge : {}x{} format {format:?}", config.width, config.height);
        println!(
            "ilot : {}x{} a ({}, {}), echelle {} (ecran {echelle} x texte {:.2}), alpha par pixel : {}",
            (ilot::DEPLOYE_L * ilot.echelle() as f32).round(),
            (ilot::DEPLOYE_H * ilot.echelle() as f32).round(),
            position.0,
            position.1,
            ilot.echelle(),
            ilot.facteur_texte(),
            if ilot.alpha_par_pixel() {
                "oui"
            } else {
                "non — decoupe GDI, coins creneles"
            }
        );
        println!("blocs : {}", Programme::chemin().display());
        println!("boucle : resumed fini a {:.0} ms du depart", depuis_le_depart());

        let maintenant = std::time::Instant::now();
        self.etat = Some(Etat {
            gpu,
            typo,
            horloge: Horloge {
                fenetre: horloge_f,
                surface: horloge_s,
                config,
                scene,
                trame_precedente: maintenant,
                trames: 0,
                trames_precedentes: 0,
                debut_bascule: maintenant,
            },
            ilot,
        });
        self.derniere_pendule = maintenant;
        self.accorder();

        // --- L'ouverture : peindre, puis paraitre ---------------------------
        // Les deux fenetres sont nees invisibles ; rien n'est encore a l'ecran.
        // Leur premiere image se peint ICI, directement, sans passer par une
        // demande de redessin : une fenetre cachee ne recoit pas `WM_PAINT`,
        // donc pas de `RedrawRequested`, et elle resterait cachee pour
        // toujours — c'est ce qui est arrive au premier essai, et c'est
        // mesure. Puis elles paraissent, l'horloge d'abord, l'ilot ensuite,
        // qui est au-dessus d'elle.
        //
        // Elles paraissent MEME si la peinture a echoue. Une fenetre qu'on ne
        // montre pas parce qu'un `get_current_texture` a rendu `Outdated`
        // serait une application invisible : le defaut qu'on corrige vaut
        // mieux que l'application qui ne s'ouvre pas.
        if !paraitre_des_la_creation {
            let peinte = self.peindre_l_horloge();
            if let Some(Etat { gpu, typo, ilot, .. }) = self.etat.as_mut() {
                if !self.trame_ilot_vue {
                    self.trame_ilot_vue = true;
                    println!(
                        "boucle : premiere trame de l'ilot a {:.0} ms du depart",
                        depuis_le_depart()
                    );
                }
                ilot.dessiner(gpu, typo, &self.chrono);
            }
            if let Some(etat) = self.etat.as_ref() {
                etat.horloge.fenetre.set_visible(true);
                etat.ilot.fenetre.set_visible(true);
            }
            println!(
                "ouverture : les deux fenetres paraissent a {:.0} ms du depart, horloge peinte : {}",
                depuis_le_depart(),
                if peinte { "oui" } else { "NON — montree quand meme" }
            );
        }

        // Le bouton de reduction et la demarque de plein ecran, APRES que la
        // fenetre a paru : `set_visible` reecrit le style entier, et les
        // poserait plus tot les effacerait (voir plus haut).
        if let Some(etat) = self.etat.as_ref() {
            println!("horloge : {}", rendre_reductible_par_win_d(&etat.horloge.fenetre));
        }

        armer(el, temps::maintenant(), None);
    }

    /// Le reveil sur la frontiere de seconde. Il est rearme a chaque tour a
    /// partir de l'horloge du systeme, jamais par un intervalle fixe : une
    /// cadence irreguliere est ce que l'oeil remarque.
    fn new_events(&mut self, el: &ActiveEventLoop, cause: winit::event::StartCause) {
        // Windows vient d'annoncer un changement de reglage ou d'alimentation
        // (le crochet de `construire_boucle`). La question est posee ici parce
        // que `new_events` passe a chaque tour de pompe, quelle que soit la
        // cause : le repli arrive donc dans le meme lot de messages que
        // l'annonce, pas une seconde plus tard. Elle repond faux presque
        // toujours, et ne coute alors qu'une lecture atomique.
        if let Some(devant) = crate::systeme::premier_plan_change() {
            self.reevaluer_presence(devant);
        } else if !self.presence_lue && self.etat.is_some() && self.opt.presence_reelle() {
            // La premiere lecture, une fois l'etat construit : qui est devant
            // au lancement ? D'ordinaire l'horloge, que `resumed` vient de
            // mettre en plein ecran.
            self.presence_lue = true;
            let devant = crate::systeme::premier_plan();
            self.reevaluer_presence(devant);
        }
        if crate::systeme::reglages_changes() {
            if let Some(etat) = self.etat.as_mut() {
                etat.ilot.relire_les_reglages();
                if self.opt.trace {
                    // Sans cette ligne, le crochet ne serait verifiable qu'en
                    // changeant un reglage de la machine — qui n'est pas a nous.
                    println!(
                        "reglages relus : mouvement reduit = {}",
                        crate::systeme::mouvement_reduit()
                    );
                }
            }
        }
        // Les gestes a echeance (§B) : l'appui maintenu qui deploie, le depart
        // du pointeur qui replie. Verifies a chaque tour, ils ne coutent qu'une
        // comparaison d'instants quand rien n'est en attente.
        if let Some(etat) = self.etat.as_mut() {
            if etat.ilot.verifier_les_gestes() {
                etat.ilot.fenetre.request_redraw();
            }
        }
        // Le geste qu'une fermeture de compte a mis en attente
        // (GESTE-TRIPLE-CLIC.md §5) : l'ilot le pose, la boucle le joue.
        if let Some(geste) = self.etat.as_mut().and_then(|e| e.ilot.geste_differe()) {
            self.executer(geste);
        }
        if !matches!(cause, winit::event::StartCause::ResumeTimeReached { .. }) {
            return;
        }
        if self.etat.is_none() {
            return;
        }
        let instant = temps::maintenant();
        // Reveille par un geste et non par la frontiere de seconde : l'heure
        // est deja posee, la bascule ne repart pas ; on rearme seulement.
        let seconde = (instant.heure as u32, instant.minute as u32, instant.seconde as u32);
        if self.seconde_posee == Some(seconde) {
            let geste = self.etat.as_ref().and_then(|e| e.ilot.echeance());
            armer(el, instant, geste);
            return;
        }
        self.seconde_posee = Some(seconde);

        // Le chrono avance sur le temps REEL mesure entre deux reveils, jamais
        // sur un compte de trames : une mise en veille de deux heures retombe au
        // bon endroit du programme.
        let maintenant = std::time::Instant::now();
        let ecoule = maintenant.duration_since(self.derniere_pendule).as_secs_f64() * 1000.0;
        self.derniere_pendule = maintenant;
        let phase_avant = self.chrono.phase();
        self.chrono.avancer(ecoule);
        let phase_apres = self.chrono.phase();
        if phase_apres != phase_avant {
            // Les alertes (DESIGN.md §6, « Phases ») : la fin d'une lecture et
            // la nuit terminee deploient l'ilot le temps d'un message du
            // systeme, meme en plein ecran ; il parait sur l'ecran de l'horloge.
            let quoi = match phase_apres {
                chrono::Phase::LivreFerme => Some("fin de la lecture, livre ferme"),
                chrono::Phase::Fini => Some("nuit terminee"),
                _ => None,
            };
            if let Some(quoi) = quoi {
                let duree = crate::systeme::duree_du_message();
                if let Some(etat) = self.etat.as_mut() {
                    let ecran = crate::systeme::ecran_de(hwnd_de(&etat.horloge.fenetre));
                    etat.ilot.alerter(quoi, duree, ecran.as_ref());
                }
            }
        }

        let trace = self.opt.trace;
        let etat = self.etat.as_mut().unwrap();
        if trace {
            // Le reveil est toujours en retard, jamais en avance : la derive
            // affichee est le nombre de millisecondes deja passees dans la
            // seconde au moment ou l'on se reveille.
            println!(
                "{:02}:{:02}:{:02}  reveil a +{} ms  ({} trames a la bascule precedente)  reste {}",
                instant.heure,
                instant.minute,
                instant.seconde,
                instant.milli,
                etat.horloge.trames_precedentes,
                self.chrono.temps_restant()
            );
        }
        etat.horloge.trames = 0;
        // La bascule part d'ici. Sans cette remise a zero, la premiere trame de
        // l'animation mesurerait le temps ecoule depuis la trame precedente —
        // une seconde entiere — et la bascule serait finie avant d'avoir ete vue.
        etat.horloge.trame_precedente = maintenant;
        etat.horloge.debut_bascule = maintenant;
        let Etat { typo, horloge, ilot, .. } = etat;
        horloge.scene.poser_heure(typo, instant);
        let geste = ilot.echeance();

        self.accorder();
        armer(el, instant, geste);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, id: WindowId, evenement: WindowEvent) {
        let Some(etat) = self.etat.as_mut() else {
            return;
        };
        let sur_ilot = id == etat.ilot.fenetre.id();

        match evenement {
            WindowEvent::CloseRequested => el.exit(),

            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::{Key, NamedKey};
                if !event.state.is_pressed() {
                    return;
                }
                match event.logical_key {
                    // Plein ecran de l'horloge, et le retour. La touche est lue
                    // sur les DEUX fenetres : l'horloge etant traversante, elle
                    // perd le focus des le premier clic sur l'ilot, et le
                    // raccourci doit continuer de repondre.
                    Key::Named(NamedKey::F11) => self.basculer_plein_ecran(),
                    Key::Named(NamedKey::Escape) => {
                        if sur_ilot {
                            // DESIGN.md §6 : Echap replie.
                            etat.ilot.replier();
                        } else {
                            el.exit();
                        }
                    }
                    Key::Character(ref c) if c == "q" || c == "Q" => el.exit(),
                    // Le clavier de l'ilot, une fois deploye et clique (le
                    // focus vient du clic) : Tab et Maj+Tab parcourent les
                    // actions, Entree ou Espace agit (DECISION-APRES-EPREUVE.md,
                    // §2 du juge). Il n'y a pas de chemin au clavier seul vers
                    // l'ilot : c'est un sacrifice declare.
                    Key::Named(NamedKey::Tab) if sur_ilot => {
                        if etat.ilot.clavier_tab(self.maj) {
                            etat.ilot.fenetre.request_redraw();
                        }
                    }
                    Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Space) if sur_ilot => {
                        if let Some(geste) = etat.ilot.clavier_agir() {
                            self.executer(geste);
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                self.maj = m.state().shift_key();
            }

            // --- L'ilot -----------------------------------------------------
            WindowEvent::CursorMoved { position, .. } if sur_ilot => {
                if etat.ilot.souris_bougee(position.x, position.y) {
                    etat.ilot.fenetre.request_redraw();
                }
                // Sortir du rectangle du double clic ferme le compte : son
                // horloge est rendue ici, sans attendre le tour suivant.
                let differe = etat.ilot.geste_differe();
                if let Some(geste) = differe {
                    self.executer(geste);
                }
            }
            WindowEvent::CursorLeft { .. } if sur_ilot => {
                if etat.ilot.souris_sortie() {
                    etat.ilot.fenetre.request_redraw();
                }
                // Le depart du pointeur reveille la boucle au bout du delai.
                let geste = etat.ilot.echeance();
                let differe = etat.ilot.geste_differe();
                armer(el, temps::maintenant(), geste);
                if let Some(geste) = differe {
                    self.executer(geste);
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } if sur_ilot => {
                // Le point d'appui n'est pas dans l'evenement : winit le livre
                // dans le dernier CursorMoved, que l'ilot a garde.
                match state {
                    ElementState::Pressed => {
                        etat.ilot.souris_appuyee_ici();
                        // Le delai de l'appui maintenu, et celui du compte des
                        // clics, reveillent la boucle.
                        let geste = etat.ilot.echeance();
                        // Un appui hors du delai ou du rectangle ferme le
                        // compte precedent : son horloge part d'ici.
                        let differe = etat.ilot.geste_differe();
                        armer(el, temps::maintenant(), geste);
                        if let Some(geste) = differe {
                            self.executer(geste);
                        }
                    }
                    ElementState::Released => {
                        let rendu = etat.ilot.souris_relachee_ici();
                        // Le compte des clics echoit apres le dernier
                        // relachement : la boucle doit se reveiller pour lui.
                        let geste = etat.ilot.echeance();
                        armer(el, temps::maintenant(), geste);
                        if let Some(geste) = rendu {
                            self.executer(geste);
                        }
                    }
                }
            }
            WindowEvent::Focused(false) if sur_ilot => {
                // Pendant le relevé, le focus appartient a l'environnement : une
                // autre fenetre de la machine peut le prendre a tout instant, et
                // le repli qui suivrait changerait la geometrie sous le temoin.
                // Le relevé le compte au lieu de le suivre.
                #[cfg(windows)]
                if let Some(releve) = self.releve.as_mut() {
                    releve.focus_perdu();
                    return;
                }
                #[cfg(windows)]
                if let Some(oeil) = self.oeil.as_mut() {
                    oeil.focus_perdu();
                    return;
                }
                // « Clic n'importe ou ailleurs » : la perte de focus est le seul
                // signal qu'une fenetre recoit d'un clic hors d'elle-meme.
                etat.ilot.replier();
            }
            WindowEvent::Moved(position) if sur_ilot => {
                if !self.opt.scripte() {
                    etat.ilot.memoriser_position(position.x, position.y);
                }
            }
            // Glisse sur un moniteur d'un autre facteur d'echelle, l'ilot garde
            // sa taille LOGIQUE et change de taille physique : les deux
            // evenements arrivent, et le second porte la taille.
            WindowEvent::ScaleFactorChanged { .. } if sur_ilot => {
                let taille = etat.ilot.fenetre.inner_size();
                let Etat { gpu, typo, ilot, .. } = etat;
                ilot.redimensionner(gpu, typo, taille.width, taille.height);
            }
            WindowEvent::Resized(taille) if sur_ilot => {
                let Etat { gpu, typo, ilot, .. } = etat;
                ilot.redimensionner(gpu, typo, taille.width, taille.height);
            }

            // --- L'horloge --------------------------------------------------
            WindowEvent::Resized(taille) if !sur_ilot => {
                // La chaine d'echange ne suit pas toute seule un plein ecran.
                let (l, h) = (taille.width.max(1), taille.height.max(1));
                if (l, h) == (etat.horloge.config.width, etat.horloge.config.height) {
                    return;
                }
                etat.horloge.config.width = l;
                etat.horloge.config.height = h;
                etat.horloge
                    .surface
                    .configure(&etat.gpu.peripherique, &etat.horloge.config);
                // Toute la geometrie decoule de F, donc de la taille : un
                // changement de moniteur refait la scene plutot que d'etirer une
                // disposition calculee pour une autre largeur.
                let Etat { gpu, typo, horloge, .. } = etat;
                horloge.scene =
                    Scene::nouvelle(gpu, typo, horloge.config.format, l, h, &self.opt);
                if !self.opt.legende_imposee {
                    horloge
                        .scene
                        .poser_phase(self.chrono.phase_affichee(), self.chrono.lumiere_basse());
                }
                horloge.fenetre.request_redraw();
            }

            WindowEvent::RedrawRequested if sur_ilot => {
                if !self.trame_ilot_vue {
                    self.trame_ilot_vue = true;
                    println!("boucle : premiere trame de l'ilot a {:.0} ms du depart", depuis_le_depart());
                }
                // Les deux ressorts avancent DANS `dessiner` : leurs cibles ne
                // sont connues qu'une fois le contenu faconne.
                let Etat { gpu, typo, ilot, .. } = etat;
                ilot.dessiner(gpu, typo, &self.chrono);
                // Tant que le ressort travaille, ou que la matiere change, on
                // redemande une trame ; Fifo en cale le rythme sur l'ecran.
                if ilot.en_mouvement() || ilot.matiere_en_cours() {
                    ilot.fenetre.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                if !self.trame_horloge_vue {
                    println!("boucle : premiere trame de l'horloge demandee a {:.0} ms du depart", depuis_le_depart());
                }
                if !self.peindre_l_horloge() {
                    return;
                }
                let Some(Etat { horloge, ilot, .. }) = self.etat.as_mut() else {
                    return;
                };

                // Tant qu'un volet tourne — ou que le variateur descend — on
                // redemande une trame. Sinon on ne dessine plus rien jusqu'a la
                // prochaine frontiere de seconde.
                if horloge.scene.en_mouvement() {
                    horloge.trames += 1;
                    horloge.fenetre.request_redraw();
                } else {
                    if horloge.trames > 0 && self.opt.trace {
                        let ms = horloge.debut_bascule.elapsed().as_secs_f32() * 1000.0;
                        println!(
                            "    bascule finie en {ms:.0} ms, {} trames -> {:.0} Hz",
                            horloge.trames,
                            horloge.trames as f32 * 1000.0 / ms
                        );
                    }
                    horloge.trames_precedentes = horloge.trames;
                    horloge.trames = 0;
                    let geste = ilot.echeance();
                    armer(el, temps::maintenant(), geste);
                }
            }
            _ => {}
        }
    }
}
