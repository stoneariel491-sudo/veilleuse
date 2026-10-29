//! La question qui decide : notre propre fenetre se voit-elle dans la
//! duplication de bureau ?
//!
//! Si elle s'y voit, tout verre taille par nous-memes boucle sur lui-meme et
//! l'idee est morte. Le test n'a de valeur qu'avec son temoin : on capture
//! DEUX fois, sans le drapeau puis avec, et on compte les pixels magenta.
//!
//! Trois pieges, tous payes au premier essai :
//!
//! 1. sans conscience du DPI, les coordonnees du bureau sont mises a l'echelle
//!    (1536x864) alors que la texture de duplication est en pixels physiques
//!    (1920x1080). On la demande donc AVANT toute fenetre ;
//! 2. `AcquireNextFrame` reussit aussi pour un simple mouvement de curseur, et
//!    la texture rendue ne contient alors AUCUNE image neuve. Le seul signal
//!    fiable est `LastPresentTime != 0` ;
//! 3. la duplication ne rend que les CHANGEMENTS. Un bureau immobile n'en
//!    produit aucun, d'ou l'agitateur : une petite fenetre qu'on deplace pour
//!    forcer une presentation.

use windows::core::Interface;
use windows::Win32::Foundation::{COLORREF, HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Gdi::CreateSolidBrush;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::*;

/// Position et taille de la fenetre temoin, en pixels physiques.
const X: i32 = 400;
const Y: i32 = 300;
const LARGEUR: i32 = 480;
const HAUTEUR: i32 = 240;

unsafe extern "system" fn proc_fenetre(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
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
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

unsafe fn creer_fenetre(classe: windows::core::PCWSTR, x: i32, y: i32, l: i32, h: i32) -> HWND {
    let module = GetModuleHandleW(None).unwrap();
    let f = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        classe,
        windows::core::w!(""),
        WS_POPUP | WS_VISIBLE,
        x,
        y,
        l,
        h,
        None,
        None,
        Some(module.into()),
        None,
    )
    .unwrap();
    let _ = ShowWindow(f, SW_SHOWNOACTIVATE);
    f
}

/// L'adaptateur qui PILOTE la sortie contenant le point donne. Avec deux cartes
/// il n'y a qu'un seul bon choix, et le deviner ne marche pas : DuplicateOutput
/// echoue si l'adaptateur n'est pas celui de la sortie.
unsafe fn sortie_pour(
    x: i32,
    y: i32,
) -> windows::core::Result<(IDXGIAdapter1, IDXGIOutput1, RECT, String)> {
    let fabrique: IDXGIFactory1 = CreateDXGIFactory1()?;
    let mut ia = 0u32;
    while let Ok(adaptateur) = fabrique.EnumAdapters1(ia) {
        let desc = adaptateur.GetDesc1()?;
        let nom: String = String::from_utf16_lossy(&desc.Description);
        let nom = nom.trim_end_matches(char::from(0)).to_string();
        let mut io = 0u32;
        while let Ok(sortie) = adaptateur.EnumOutputs(io) {
            let d = sortie.GetDesc()?;
            let r = d.DesktopCoordinates;
            if x >= r.left && x < r.right && y >= r.top && y < r.bottom {
                let s1: IDXGIOutput1 = sortie.cast()?;
                return Ok((adaptateur.clone(), s1, r, nom));
            }
            io += 1;
        }
        ia += 1;
    }
    Err(windows::core::Error::from_thread())
}

/// Une image du bureau, en BGRA. `agitateur` est deplace a chaque tour pour
/// forcer une presentation : sans cela un bureau immobile ne rend jamais rien.
unsafe fn capturer_bureau(agitateur: HWND) -> windows::core::Result<(Vec<u8>, u32, u32)> {
    let (adaptateur, sortie, rect, nom) = sortie_pour(X + LARGEUR / 2, Y + HAUTEUR / 2)?;
    eprintln!(
        "  sortie {}x{} pilotee par {}",
        rect.right - rect.left,
        rect.bottom - rect.top,
        nom
    );

    let mut appareil: Option<ID3D11Device> = None;
    let mut contexte: Option<ID3D11DeviceContext> = None;
    D3D11CreateDevice(
        &adaptateur,
        D3D_DRIVER_TYPE_UNKNOWN,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        None,
        D3D11_SDK_VERSION,
        Some(&mut appareil),
        None,
        Some(&mut contexte),
    )?;
    let appareil = appareil.unwrap();
    let contexte = contexte.unwrap();

    let duplication = sortie.DuplicateOutput(&appareil)?;

    // LA faute a ne pas refaire : `ReleaseFrame` rend le contenu de la
    // texture indefini. La copie doit avoir lieu AVANT, pas apres — sinon on
    // lit un noir parfait et on croit que la duplication ne marche pas.
    let mut resultat: Option<(Vec<u8>, u32, u32)> = None;
    let mut trames_vues = 0u32;
    for tour in 0..90 {
        // On agite : une vraie presentation du bureau, provoquee.
        let dx = if tour % 2 == 0 { 0 } else { 7 };
        let _ = SetWindowPos(
            agitateur,
            None,
            8 + dx,
            8,
            24,
            24,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
        pomper(12);

        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut ressource: Option<IDXGIResource> = None;
        if duplication
            .AcquireNextFrame(120, &mut info, &mut ressource)
            .is_err()
        {
            continue;
        }
        trames_vues += 1;

        // LastPresentTime a zero = seul le curseur a bouge. La texture ne
        // contient alors aucune image neuve.
        let mut pris = None;
        if info.LastPresentTime != 0 {
            if let Some(r) = &ressource {
                if let Ok(image) = r.cast::<ID3D11Texture2D>() {
                    let mut d = D3D11_TEXTURE2D_DESC::default();
                    image.GetDesc(&mut d);

                    let mut lecture = d;
                    lecture.Usage = D3D11_USAGE_STAGING;
                    lecture.BindFlags = 0;
                    lecture.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                    lecture.MiscFlags = 0;
                    lecture.SampleDesc.Count = 1;

                    let mut copie: Option<ID3D11Texture2D> = None;
                    appareil.CreateTexture2D(&lecture, None, Some(&mut copie))?;
                    let copie = copie.unwrap();
                    contexte.CopyResource(&copie, &image);
                    pris = Some((copie, d.Width, d.Height));
                }
            }
        }
        let _ = duplication.ReleaseFrame();

        if let Some((copie, w, h)) = pris {
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            contexte.Map(&copie, 0, D3D11_MAP_READ, 0, Some(&mut m))?;
            let mut octets = vec![0u8; (w * h * 4) as usize];
            for ligne in 0..h {
                let src = (m.pData as *const u8).add((ligne * m.RowPitch) as usize);
                let dst = octets.as_mut_ptr().add((ligne * w * 4) as usize);
                std::ptr::copy_nonoverlapping(src, dst, (w * 4) as usize);
            }
            contexte.Unmap(&copie, 0);
            eprintln!("  trame utile au tour {} ({} acquises), {}x{}", tour, trames_vues, w, h);
            resultat = Some((octets, w, h));
            break;
        }
    }

    match resultat {
        Some(r) => Ok(r),
        None => {
            eprintln!("  aucune trame avec image neuve ({} acquises)", trames_vues);
            Err(windows::core::Error::from_thread())
        }
    }
}

/// Combien de pixels portent la couleur temoin, et ou se trouve le premier.
fn compter_temoin(octets: &[u8], w: u32, h: u32) -> (usize, Option<(u32, u32)>) {
    let mut n = 0usize;
    let mut ou = None;
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            // La duplication rend du BGRA.
            let b = octets[i];
            let g = octets[i + 1];
            let r = octets[i + 2];
            if r > 200 && g < 60 && b > 200 {
                n += 1;
                if ou.is_none() {
                    ou = Some((x, y));
                }
            }
        }
    }
    (n, ou)
}

/// La moyenne dit si l'image est reelle ou si on a lu une texture vide.
fn moyenne(octets: &[u8]) -> (f64, f64, f64) {
    let n = (octets.len() / 4) as f64;
    let mut s = (0f64, 0f64, 0f64);
    for i in (0..octets.len()).step_by(4) {
        s.0 += octets[i + 2] as f64;
        s.1 += octets[i + 1] as f64;
        s.2 += octets[i] as f64;
    }
    (s.0 / n, s.1 / n, s.2 / n)
}

fn ecrire_png(chemin: &str, octets: &[u8], w: u32, h: u32) {
    let mut rgba = vec![0u8; octets.len()];
    for i in (0..octets.len()).step_by(4) {
        rgba[i] = octets[i + 2];
        rgba[i + 1] = octets[i + 1];
        rgba[i + 2] = octets[i];
        rgba[i + 3] = 255;
    }
    let f = std::fs::File::create(chemin).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(&rgba).unwrap();
}

fn main() -> windows::core::Result<()> {
    unsafe {
        // AVANT toute fenetre et tout DXGI, sinon les coordonnees du bureau
        // sont mises a l'echelle et ne correspondent plus a la texture.
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        std::fs::create_dir_all("captures").ok();

        let module = GetModuleHandleW(None)?;

        // Deux classes : le temoin magenta, et l'agitateur noir.
        let c_temoin = windows::core::w!("SondeTemoin");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(proc_fenetre),
            hInstance: module.into(),
            lpszClassName: c_temoin,
            // COLORREF est 0x00BBGGRR : magenta plein.
            hbrBackground: CreateSolidBrush(COLORREF(0x00FF00FF)),
            ..Default::default()
        };
        RegisterClassW(&wc);

        let c_agit = windows::core::w!("SondeAgitateur");
        let wc2 = WNDCLASSW {
            lpfnWndProc: Some(proc_fenetre),
            hInstance: module.into(),
            lpszClassName: c_agit,
            hbrBackground: CreateSolidBrush(COLORREF(0x00203040)),
            ..Default::default()
        };
        RegisterClassW(&wc2);

        let temoin = creer_fenetre(c_temoin, X, Y, LARGEUR, HAUTEUR);
        let agitateur = creer_fenetre(c_agit, 8, 8, 24, 24);
        pomper(700);

        // --- Temoin : SANS le drapeau. La fenetre DOIT se voir. ---
        eprintln!("temoin, sans exclusion :");
        let (a, w, h) = capturer_bureau(agitateur)?;
        let (n_sans, ou) = compter_temoin(&a, w, h);
        let m = moyenne(&a);
        eprintln!(
            "  moyenne ({:.1} ; {:.1} ; {:.1})",
            m.0, m.1, m.2
        );
        eprintln!("  pixels magenta = {} (attendu ~{})", n_sans, LARGEUR * HAUTEUR);
        if let Some((px, py)) = ou {
            eprintln!("  premier a ({}, {}), fenetre posee en ({}, {})", px, py, X, Y);
        }
        ecrire_png("captures/sonde-sans-exclusion.png", &a, w, h);

        // --- Essai : AVEC le drapeau. ---
        eprintln!("essai, avec exclusion :");
        SetWindowDisplayAffinity(temoin, WDA_EXCLUDEFROMCAPTURE)?;
        pomper(500);
        let (b, w, h) = capturer_bureau(agitateur)?;
        let (n_avec, _) = compter_temoin(&b, w, h);
        let m2 = moyenne(&b);
        eprintln!("  moyenne ({:.1} ; {:.1} ; {:.1})", m2.0, m2.1, m2.2);
        eprintln!("  pixels magenta = {}", n_avec);
        ecrire_png("captures/sonde-avec-exclusion.png", &b, w, h);

        println!();
        println!("sans exclusion : {} px temoin", n_sans);
        println!("avec exclusion : {} px temoin", n_avec);
        if n_sans > 1000 && n_avec == 0 {
            println!("VERDICT : l'exclusion fonctionne. Le verre taille est possible.");
        } else if n_sans <= 1000 {
            println!("VERDICT : test INVALIDE, le temoin ne s'est pas vu lui-meme.");
        } else {
            println!("VERDICT : l'exclusion NE marche PAS, il y aurait bouclage.");
        }

        let _ = DestroyWindow(temoin);
        let _ = DestroyWindow(agitateur);
        Ok(())
    }
}
