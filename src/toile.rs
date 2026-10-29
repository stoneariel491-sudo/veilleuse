//! La toile : un fond CONNU sous l'ilot, pour l'oeil (`oeil.rs`).
//!
//! Une fenetre Win32 nue, plein moniteur, sans cadre, sans barre des taches,
//! qui ne prend jamais le focus. Elle ne porte aucune surface wgpu : Windows
//! la peint lui-meme avec la brosse de sa classe, un aplat GDI de 24 bits dont
//! chaque octet est exactement celui qu'on a demande. Un effacement wgpu passe
//! par une couleur lineaire que la chaine d'echange reencode ; l'octet qui en
//! sort est juste a l'arrondi pres, et c'est l'arrondi qu'on ne veut pas avoir a
//! prouver sous un temoin a reponse connue.
//!
//! Elle est posee au-dessus de l'horloge et SOUS l'ilot, qui est toujours
//! au-dessus. Masquee, elle rend l'horloge.

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{CreateSolidBrush, DeleteObject, InvalidateRect, UpdateWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, SetClassLongPtrW, SetWindowPos,
    ShowWindow, GCLP_HBRBACKGROUND, HWND_TOP, MA_NOACTIVATE, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    SW_HIDE, WM_MOUSEACTIVATE, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
};

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if msg == WM_MOUSEACTIVATE {
        return MA_NOACTIVATE as LRESULT;
    }
    unsafe { DefWindowProcW(hwnd, msg, w, l) }
}

pub struct Toile {
    hwnd: HWND,
    rect: (i32, i32, i32, i32),
}

fn large(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

impl Toile {
    /// Cree la toile, masquee, sur le rectangle physique `(x, y, l, h)`.
    pub fn nouvelle(x: i32, y: i32, l: i32, h: i32) -> Option<Toile> {
        let classe = large("VeilleuseToile");
        let titre = large("Veilleuse — toile de l'oeil");
        unsafe {
            let instance = GetModuleHandleW(null_mut());
            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(procedure),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: null_mut(),
                hCursor: null_mut(),
                hbrBackground: CreateSolidBrush(0),
                lpszMenuName: std::ptr::null(),
                lpszClassName: classe.as_ptr(),
            };
            // Un second enregistrement echoue sans dommage : la classe existe.
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                classe.as_ptr(),
                titre.as_ptr(),
                WS_POPUP,
                x,
                y,
                l,
                h,
                null_mut(),
                null_mut(),
                instance,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                return None;
            }
            Some(Toile {
                hwnd,
                rect: (x, y, l, h),
            })
        }
    }

    /// Montre la toile peinte de `rgb`, au-dessus des fenetres ordinaires.
    pub fn montrer(&mut self, rgb: [u8; 3]) {
        let (x, y, l, h) = self.rect;
        unsafe {
            // COLORREF : 0x00BBGGRR.
            let brosse = CreateSolidBrush(rgb[0] as u32 | (rgb[1] as u32) << 8 | (rgb[2] as u32) << 16);
            let ancienne = SetClassLongPtrW(self.hwnd, GCLP_HBRBACKGROUND, brosse as isize);
            if ancienne != 0 {
                DeleteObject(ancienne as _);
            }
            SetWindowPos(self.hwnd, HWND_TOP, x, y, l, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            InvalidateRect(self.hwnd, std::ptr::null(), 1);
            UpdateWindow(self.hwnd);
        }
    }

    /// Masque la toile : l'horloge redevient le fond.
    pub fn masquer(&mut self) {
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

impl Drop for Toile {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}
