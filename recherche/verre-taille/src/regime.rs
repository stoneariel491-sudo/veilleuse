//! Le regime continu — le trou que `VERRE-TAILLE.md` §7 declarait ouvert.
//!
//! Tout le reste du chantier est mesure sur une capture FIGEE. Or ce qui decide
//! si le verre est livrable n'est pas le cout d'une image, c'est le cout d'une
//! seconde. Deux regimes comptent, et ils n'ont rien a voir :
//!
//! - **le bureau immobile**, qui est le cas de Stone la nuit : un livre a cote,
//!   un ecran noir. La duplication ne presente alors RIEN, et la question est
//!   de savoir si l'attendre coute quelque chose ;
//! - **le bureau qui bouge**, le pire cas : une video, un defilement. La
//!   question devient le cout de la copie, et s'il tient dans une trame.
//!
//! La forme NON BLOQUANTE est obligatoire. Attendre une trame du bureau dans la
//! boucle de rendu reviendrait a caler notre affichage sur l'activite des
//! autres fenetres — l'horloge sauterait parce qu'une autre application dort.

use verre_taille::bureau::{Capteur, Etat, Region};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::*;

const PX: i32 = 300;
const PY: i32 = 240;
const PL: i32 = 900;
const PH: i32 = 560;
const IL: u32 = 320;
const IH: u32 = 148;
const MARGE: u32 = 64;

static TEINTE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

unsafe extern "system" fn proc_planche(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if m == WM_PAINT {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(h, &mut ps);
        let mut r = RECT::default();
        let _ = GetClientRect(h, &mut r);
        let t = TEINTE.load(std::sync::atomic::Ordering::Relaxed);
        let bandes: [u32; 6] = [
            0x00202020,
            0x00E8EFF2,
            0x004030C0 ^ (t * 7 & 0xFF),
            0x0030A0E0,
            0x00202020,
            0x00F0F0F0,
        ];
        let larg = (r.right - r.left) / 6;
        for (i, c) in bandes.iter().enumerate() {
            let p = CreateSolidBrush(COLORREF(*c));
            let b = RECT {
                left: r.left + larg * i as i32,
                top: r.top,
                right: r.left + larg * (i as i32 + 1),
                bottom: r.bottom,
            };
            FillRect(dc, &b, p);
            let _ = DeleteObject(p.into());
        }
        let _ = EndPaint(h, &ps);
        return LRESULT(0);
    }
    if m == WM_DESTROY {
        PostQuitMessage(0);
        return LRESULT(0);
    }
    DefWindowProcW(h, m, w, l)
}

unsafe fn pomper() {
    let mut msg = MSG::default();
    while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

fn quantile(v: &mut Vec<f64>, q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[(((v.len() - 1) as f64) * q).round() as usize]
}

/// Un tour de boucle complet, tel que l'application le ferait.
struct Tour {
    neuve: bool,
    cout_capture: f64,
}

fn main() -> windows::core::Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let module = GetModuleHandleW(None)?;

        let c = windows::core::w!("RegimePlanche");
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc_planche),
            hInstance: module.into(),
            lpszClassName: c,
            ..Default::default()
        });
        let ci = windows::core::w!("RegimeIlot");
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc_planche),
            hInstance: module.into(),
            lpszClassName: ci,
            hbrBackground: CreateSolidBrush(COLORREF(0x00101010)),
            ..Default::default()
        });

        let planche = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            c,
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
        let ix = PX + (PL - IL as i32) / 2;
        let iy = PY + (PH - IH as i32) / 2;
        let ilot = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            ci,
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
        SetWindowDisplayAffinity(ilot, WDA_EXCLUDEFROMCAPTURE)?;
        let _ = ShowWindow(planche, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(ilot, SW_SHOWNOACTIVATE);
        for _ in 0..60 {
            pomper();
            std::thread::sleep(std::time::Duration::from_millis(8));
        }

        let capteur = Capteur::ouvrir(ix + IL as i32 / 2, iy + IH as i32 / 2)?;
        println!("adaptateur : {}", capteur.adaptateur);
        let sl = IL + 2 * MARGE;
        let sh = IH + 2 * MARGE;
        println!("region suivie : {}x{} ({} Ko par copie)", sl, sh, sl * sh * 4 / 1024);

        let mut perdus = 0u32;
        // Une seule boucle, deux regimes : on la lance deux fois.
        let mut essai = |nom: &str, anime: bool, deplace: bool, secondes: f64| {
            let mut tours: Vec<Tour> = Vec::new();
            let debut = std::time::Instant::now();
            let mut derniere: Option<Region> = None;
            let mut i = 0u32;
            // La region suivie au tour precedent : si elle a change, les
            // rectangles sales ne peuvent rien nous dire d'elle.
            let mut region_precedente: Option<(i32, i32)> = None;
            while debut.elapsed().as_secs_f64() < secondes {
                i += 1;
                if anime {
                    // Le pire cas : le fond change a chaque trame.
                    TEINTE.store(i, std::sync::atomic::Ordering::Relaxed);
                    let _ = InvalidateRect(Some(planche), None, false);
                }
                // L'ilot se promene : la region suivie bouge avec lui.
                let (cx, cy) = if deplace {
                    let d = ((i % 120) as i32) - 60;
                    let _ = SetWindowPos(ilot, None, ix + d, iy, IL as i32, IH as i32,
                        SWP_NOACTIVATE | SWP_NOZORDER);
                    (ix + d, iy)
                } else {
                    (ix, iy)
                };
                pomper();

                // Quand la region a bouge, aucun rectangle sale ne peut dire
                // ce qu'il y a sous la NOUVELLE position : il faut recopier.
                let a_bouge = region_precedente.map_or(true, |p| p != (cx, cy));
                region_precedente = Some((cx, cy));

                let t0 = std::time::Instant::now();
                let etat = if a_bouge {
                    match capteur.region_forcee(cx - MARGE as i32, cy - MARGE as i32, sl, sh) {
                        Some(r) => Etat::Neuve(r),
                        None => Etat::Inchange,
                    }
                } else {
                    capteur.region_maintenant(cx - MARGE as i32, cy - MARGE as i32, sl, sh)
                };
                let cout = t0.elapsed().as_secs_f64() * 1000.0;
                let neuve = match etat {
                    Etat::Neuve(r) => {
                        derniere = Some(r);
                        true
                    }
                    Etat::Inchange => false,
                    Etat::Perdu => {
                        perdus += 1;
                        false
                    }
                };
                tours.push(Tour {
                    neuve,
                    cout_capture: cout,
                });
                // On vise 60 Hz, comme le ferait la boucle de rendu.
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
            let n = tours.len();
            let nb_neuves = tours.iter().filter(|t| t.neuve).count();
            let mut couts_neuves: Vec<f64> = tours
                .iter()
                .filter(|t| t.neuve)
                .map(|t| t.cout_capture)
                .collect();
            let mut couts_vides: Vec<f64> = tours
                .iter()
                .filter(|t| !t.neuve)
                .map(|t| t.cout_capture)
                .collect();
            println!();
            println!("--- {} ---", nom);
            println!("  tours : {}, dont {} avec image neuve ({:.0} %)",
                n, nb_neuves, 100.0 * nb_neuves as f64 / n as f64);
            if !couts_neuves.is_empty() {
                println!(
                    "  copie quand il y a du neuf : median {:.2} ms, p95 {:.2} ms",
                    quantile(&mut couts_neuves, 0.5),
                    quantile(&mut couts_neuves, 0.95)
                );
            }
            if !couts_vides.is_empty() {
                println!(
                    "  tour a vide (rien n'a change) : median {:.3} ms, p95 {:.3} ms",
                    quantile(&mut couts_vides, 0.5),
                    quantile(&mut couts_vides, 0.95)
                );
            }
            let _ = derniere;
        };

        essai("bureau IMMOBILE (la nuit, un livre a cote)", false, false, 3.0);
        essai("bureau ANIME a chaque trame (le pire cas)", true, false, 3.0);
        essai("ILOT QUI SE DEPLACE, bureau calme", false, true, 3.0);

        println!();
        println!("acces perdus pendant l'essai : {}", perdus);
        println!("budget d'une trame a 144 Hz : 6,94 ms ; a 60 Hz : 16,7 ms");

        let _ = DestroyWindow(ilot);
        let _ = DestroyWindow(planche);
        Ok(())
    }
}
