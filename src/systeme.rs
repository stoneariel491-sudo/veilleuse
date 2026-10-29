//! Le peu qu'on demande au systeme d'exploitation — et le peu qu'il nous dit
//! quand il change d'avis en cours de route.
//!
//! Le repli « moins de mouvement » a longtemps ete lu UNE SEULE FOIS, a la
//! construction de l'ilot : un reglage change en cours de nuit n'aurait jamais
//! ete vu.
//!
//! Ce module porte donc trois choses : la lecture du reglage, une procedure de
//! fenetre qui leve un drapeau quand Windows annonce un changement, et un
//! forcage de ligne de commande sans lequel la branche de repli ne serait
//! exercable qu'en touchant aux reglages de la machine — ce qui n'est pas a
//! nous.
//!
//! ~~Le second repli, « pas de transparence », et l'ecoute de l'alimentation
//! qui le servait~~ sont retires avec l'ancien verre du systeme
//! (`VERRE-TAILLE.md` §11) : le verre taille ne depend pas des effets de
//! transparence de Windows, et quand sa capture manque l'ilot redevient le galet
//! noir sans rien demander au systeme.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
#[cfg(windows)]
use std::sync::atomic::AtomicUsize;

/// Leve par la procedure de fenetre sur `WM_SETTINGCHANGE`, baisse par celui
/// qui relit. C'est un message qu'on
/// ecoute, pas une interrogation periodique : entre deux changements, rien n'est
/// lu du tout.
static CHANGEMENT: AtomicBool = AtomicBool::new(false);

/// Forcage de la ligne de commande. Trois etats — 0 : le systeme decide ;
/// 1 : force a vrai ; 2 : force a faux.
static FORCE_MOUVEMENT_REDUIT: AtomicU8 = AtomicU8::new(0);

/// Windows vient d'annoncer un changement de reglage.
pub fn signaler_changement() {
    CHANGEMENT.store(true, Ordering::Relaxed);
}

/// Procedure de fenetre precedente, gardee pour lui rendre la main. Une seule
/// fenetre est ecoutee — l'ilot — donc un seul emplacement suffit.
#[cfg(windows)]
static PRECEDENTE: AtomicUsize = AtomicUsize::new(0);

/// Ecoute sur `hwnd` le message par lequel Windows annonce qu'il a change
/// d'avis : `WM_SETTINGCHANGE` (0x001A), dont « Afficher les animations dans
/// Windows ».
///
/// **Pourquoi une procedure de fenetre et non le crochet de messages de winit.**
/// `with_msg_hook` ne voit que ce que `PeekMessage` REND, c'est-a-dire les
/// messages POSTES. Or celui-la est ENVOYE : Windows les diffuse par
/// `SendMessageTimeout`, et un message envoye depuis un autre fil est remis
/// directement a la procedure de fenetre depuis l'interieur de `PeekMessage`,
/// sans jamais ressortir dans un `MSG`. Le crochet a ete essaye et mesure : zero
/// ligne de trace sur une diffusion de `WM_SETTINGCHANGE`, avant comme apres.
/// La procedure de fenetre, elle, le recoit.
///
/// Elle ne fait RIEN d'autre que lever un drapeau. Relire un reglage ici
/// tiendrait la file de messages pendant un acces au registre ; la relecture a
/// lieu au tour suivant de la boucle, dans `new_events`.
#[cfg(windows)]
pub fn ecouter_les_changements(hwnd: isize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowLongPtrW, GWLP_WNDPROC,
    };
    let notre = notre_procedure as *const () as isize;
    let precedente = unsafe { SetWindowLongPtrW(hwnd as _, GWLP_WNDPROC, notre) };
    PRECEDENTE.store(precedente as usize, Ordering::Relaxed);
}

#[cfg(windows)]
unsafe extern "system" fn notre_procedure(
    hwnd: windows_sys::Win32::Foundation::HWND,
    message: u32,
    w: windows_sys::Win32::Foundation::WPARAM,
    l: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, DefWindowProcW, WM_POWERBROADCAST, WM_SETTINGCHANGE,
        WM_SYSCOLORCHANGE, WM_THEMECHANGED,
    };
    // Les reglages du systeme (pas 7) : la transparence et la taille du texte
    // arrivent par WM_SETTINGCHANGE, le theme de contraste par WM_SETTINGCHANGE
    // et WM_SYSCOLORCHANGE (ses couleurs), le theme par WM_THEMECHANGED, et
    // l'economiseur de batterie par WM_POWERBROADCAST. Tous levent le meme
    // drapeau ; la relecture, au tour suivant, relit tout.
    if message == WM_SETTINGCHANGE
        || message == WM_POWERBROADCAST
        || message == WM_SYSCOLORCHANGE
        || message == WM_THEMECHANGED
    {
        signaler_changement();
    }
    let precedente = PRECEDENTE.load(Ordering::Relaxed);
    if precedente == 0 {
        return unsafe { DefWindowProcW(hwnd, message, w, l) };
    }
    // La procedure de winit garde la main sur tout le reste : on ne fait
    // qu'ecouter passer ce message.
    let precedente: windows_sys::Win32::UI::WindowsAndMessaging::WNDPROC =
        Some(unsafe { std::mem::transmute::<usize, unsafe extern "system" fn(_, _, _, _) -> _>(precedente) });
    unsafe { CallWindowProcW(precedente, hwnd, message, w, l) }
}

#[cfg(not(windows))]
pub fn ecouter_les_changements(_hwnd: isize) {}

/// Y a-t-il eu un changement depuis la derniere fois ? La question se pose une
/// fois par reveil de l'horloge, et elle repond faux presque toujours.
pub fn reglages_changes() -> bool {
    CHANGEMENT.swap(false, Ordering::Relaxed)
}

pub fn forcer_mouvement_reduit(force: Option<bool>) {
    FORCE_MOUVEMENT_REDUIT.store(etat(force), Ordering::Relaxed);
}

fn etat(force: Option<bool>) -> u8 {
    match force {
        None => 0,
        Some(true) => 1,
        Some(false) => 2,
    }
}

fn force(drapeau: &AtomicU8) -> Option<bool> {
    match drapeau.load(Ordering::Relaxed) {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
}

/// Le systeme demande-t-il moins de mouvement ?
///
/// DESIGN.md §5 point 6 : dans ce cas les chiffres changent d'un coup, sans
/// volet. Sous Windows, le reglage s'appelle « Afficher les animations dans
/// Windows » et se lit par `SPI_GETCLIENTAREAANIMATION` : il vaut faux quand
/// l'utilisateur a coupe les animations.
pub fn mouvement_reduit() -> bool {
    if let Some(f) = force(&FORCE_MOUVEMENT_REDUIT) {
        return f;
    }
    lire_mouvement_reduit()
}

#[cfg(windows)]
fn lire_mouvement_reduit() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION,
    };
    let mut animations: i32 = 1;
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            (&mut animations as *mut i32).cast(),
            0,
        )
    };
    // En cas d'echec on garde le mouvement : l'horloge doit tourner.
    ok != 0 && animations == 0
}

#[cfg(not(windows))]
fn lire_mouvement_reduit() -> bool {
    false
}

// --- Les reglages du systeme (DESIGN-ILOT.md §8.1, DECISION-APRES-EPREUVE.md §H, pas 7)
//
// Quatre reglages de plus, lus comme le mouvement reduit : une lecture au
// lancement et a chaque annonce de Windows, jamais par interrogation, et un
// forcage de ligne de commande pour chacun, sans quoi la branche ne serait
// exercable qu'en touchant aux reglages de la machine — qui n'est pas a nous.

/// Trois etats, comme `FORCE_MOUVEMENT_REDUIT`.
static FORCE_TRANSPARENCE_COUPEE: AtomicU8 = AtomicU8::new(0);
static FORCE_ECONOMISEUR: AtomicU8 = AtomicU8::new(0);
static FORCE_CONTRASTE: AtomicU8 = AtomicU8::new(0);
/// 0 : le systeme decide ; sinon le facteur force, en pour cent.
static FORCE_TAILLE_DU_TEXTE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn forcer_transparence_coupee(force: Option<bool>) {
    FORCE_TRANSPARENCE_COUPEE.store(etat(force), Ordering::Relaxed);
}

pub fn forcer_economiseur(force: Option<bool>) {
    FORCE_ECONOMISEUR.store(etat(force), Ordering::Relaxed);
}

pub fn forcer_contraste(force: Option<bool>) {
    FORCE_CONTRASTE.store(etat(force), Ordering::Relaxed);
}

pub fn forcer_taille_du_texte(pour_cent: Option<u32>) {
    FORCE_TAILLE_DU_TEXTE.store(pour_cent.unwrap_or(0), Ordering::Relaxed);
}

/// Une valeur DWORD sous HKEY_CURRENT_USER, ou rien.
#[cfg(windows)]
fn dword_utilisateur(cle: &str, valeur: &str) -> Option<u32> {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let cle: Vec<u16> = cle.encode_utf16().chain(Some(0)).collect();
    let valeur: Vec<u16> = valeur.encode_utf16().chain(Some(0)).collect();
    let mut donnee: u32 = 0;
    let mut octets: u32 = 4;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            cle.as_ptr(),
            valeur.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut donnee as *mut u32).cast(),
            &mut octets,
        )
    };
    (r == 0).then_some(donnee)
}

/// « Effets de transparence » coupes dans les reglages de Windows
/// (Personnalisation > Couleurs). Le reglage vit dans le registre,
/// `Themes\Personalize\EnableTransparency` (0 = coupe), et son changement est
/// diffuse par `WM_SETTINGCHANGE` (« ImmersiveColorSet »). Microsoft l'expose
/// aussi par `UISettings.AdvancedEffectsEnabled`, qui lit la meme valeur.
pub fn transparence_coupee() -> bool {
    if let Some(f) = force(&FORCE_TRANSPARENCE_COUPEE) {
        return f;
    }
    lire_transparence_coupee()
}

#[cfg(windows)]
fn lire_transparence_coupee() -> bool {
    dword_utilisateur(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
        "EnableTransparency",
    ) == Some(0)
}

#[cfg(not(windows))]
fn lire_transparence_coupee() -> bool {
    false
}

/// L'economiseur de batterie est en marche (`SYSTEM_POWER_STATUS.SystemStatusFlag`,
/// 1 = economiseur). Son changement arrive par `WM_POWERBROADCAST`.
pub fn economiseur_de_batterie() -> bool {
    if let Some(f) = force(&FORCE_ECONOMISEUR) {
        return f;
    }
    lire_economiseur()
}

#[cfg(windows)]
fn lire_economiseur() -> bool {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut etat: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    unsafe { GetSystemPowerStatus(&mut etat) != 0 && etat.SystemStatusFlag == 1 }
}

#[cfg(not(windows))]
fn lire_economiseur() -> bool {
    false
}

/// Les couleurs du theme de contraste, en RVB 0-255 : le fond (`Window`),
/// le texte (`WindowText`), la surbrillance (`Highlight`) et son texte
/// (`HighlightText`) — les paires du systeme, jamais une couleur codee en dur
/// (DESIGN-ILOT.md §8.1, [HC]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contraste {
    pub fond: [u8; 3],
    pub texte: [u8; 3],
    pub surbrillance: [u8; 3],
    pub texte_surbrillance: [u8; 3],
}

/// Le theme de contraste du systeme, s'il est actif (`SPI_GETHIGHCONTRAST`,
/// `HCF_HIGHCONTRASTON`), avec ses couleurs. Force, il prend les couleurs que
/// `GetSysColor` rend a cet instant : celles du theme courant.
pub fn theme_de_contraste() -> Option<Contraste> {
    match force(&FORCE_CONTRASTE) {
        Some(false) => None,
        Some(true) => Some(couleurs_du_systeme()),
        None => lire_contraste_actif().then(couleurs_du_systeme),
    }
}

#[cfg(windows)]
fn lire_contraste_actif() -> bool {
    use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETHIGHCONTRAST};
    let mut hc: HIGHCONTRASTW = unsafe { std::mem::zeroed() };
    hc.cbSize = std::mem::size_of::<HIGHCONTRASTW>() as u32;
    let ok = unsafe {
        SystemParametersInfoW(SPI_GETHIGHCONTRAST, hc.cbSize, (&mut hc as *mut HIGHCONTRASTW).cast(), 0)
    };
    ok != 0 && hc.dwFlags & HCF_HIGHCONTRASTON != 0
}

#[cfg(not(windows))]
fn lire_contraste_actif() -> bool {
    false
}

#[cfg(windows)]
fn couleurs_du_systeme() -> Contraste {
    use windows_sys::Win32::Graphics::Gdi::{
        GetSysColor, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW, COLOR_WINDOWTEXT,
    };
    // COLORREF : 0x00BBGGRR.
    let rvb = |c: u32| [(c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8];
    unsafe {
        Contraste {
            fond: rvb(GetSysColor(COLOR_WINDOW)),
            texte: rvb(GetSysColor(COLOR_WINDOWTEXT)),
            surbrillance: rvb(GetSysColor(COLOR_HIGHLIGHT)),
            texte_surbrillance: rvb(GetSysColor(COLOR_HIGHLIGHTTEXT)),
        }
    }
}

#[cfg(not(windows))]
fn couleurs_du_systeme() -> Contraste {
    Contraste { fond: [255; 3], texte: [0; 3], surbrillance: [0, 120, 215], texte_surbrillance: [255; 3] }
}

/// Le facteur de la taille du texte du systeme (Accessibilite > Taille du
/// texte, `UISettings.TextScaleFactor`), 1 a 2,25. Il vit dans le registre,
/// `Accessibility\TextScaleFactor` en pour cent, absent a 100 %, et son
/// changement est diffuse par `WM_SETTINGCHANGE`.
pub fn taille_du_texte() -> f32 {
    let force = FORCE_TAILLE_DU_TEXTE.load(Ordering::Relaxed);
    let pour_cent = if force > 0 { force } else { lire_taille_du_texte() };
    (pour_cent.clamp(100, 225) as f32) / 100.0
}

#[cfg(windows)]
fn lire_taille_du_texte() -> u32 {
    dword_utilisateur("Software\\Microsoft\\Accessibility", "TextScaleFactor").unwrap_or(100)
}

#[cfg(not(windows))]
fn lire_taille_du_texte() -> u32 {
    100
}

/// Le verre est-il permis par le systeme ? Sinon la raison, dans les mots de
/// DESIGN-ILOT.md §8.1 : le galet est opaque, sans action de matiere, le
/// raccourci inerte, le choix restant retenu.
pub fn verre_permis() -> Result<(), &'static str> {
    if transparence_coupee() {
        Err("effets de transparence coupes")
    } else if economiseur_de_batterie() {
        Err("economiseur de batterie")
    } else if theme_de_contraste().is_some() {
        Err("theme de contraste")
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod essais {
    use super::*;

    #[test]
    fn le_forcage_prend_le_pas_sur_le_systeme_puis_le_rend() {
        forcer_mouvement_reduit(Some(true));
        assert!(mouvement_reduit(), "le forcage a vrai doit s'imposer");
        forcer_mouvement_reduit(Some(false));
        assert!(!mouvement_reduit(), "le forcage a faux doit s'imposer aussi");
        forcer_mouvement_reduit(None);
        // Sans forcage, c'est le systeme qui repond — on ne teste pas SA
        // reponse, seulement que le forcage a bien rendu la main.
        assert_eq!(mouvement_reduit(), lire_mouvement_reduit());
    }

    #[test]
    fn le_drapeau_de_changement_se_prend_une_seule_fois() {
        // Sans le swap, deux lecteurs relireraient les reglages pour un seul
        // message, ou pire : le drapeau resterait leve pour toujours.
        assert!(!reglages_changes(), "il doit partir baisse");
        signaler_changement();
        assert!(reglages_changes());
        assert!(!reglages_changes(), "il devait se baisser en se prenant");
    }
}

// --- Le premier plan, la presence de l'ilot ---------------------------------

/// La derniere fenetre passee au premier plan, et le drapeau de son changement.
/// La regle de presence se lit sur ces changements, jamais par interrogation
/// (DESIGN.md §6, « Presence »).
static AVANT_PLAN: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
static AVANT_PLAN_CHANGE: AtomicBool = AtomicBool::new(false);

/// Ecoute les changements de premier plan pour tout le bureau
/// (`EVENT_SYSTEM_FOREGROUND`) et les reductions (`MINIMIZESTART` a
/// `MINIMIZEEND`, Win+D et Win+M), hors contexte : Windows appelle la
/// procedure sur le fil de la boucle, entre deux messages, et elle ne fait que
/// lever un drapeau, relu au tour suivant comme celui des reglages. Rend faux
/// si le systeme refuse l'un des deux crochets.
#[cfg(windows)]
pub fn ecouter_le_premier_plan() -> bool {
    use windows_sys::Win32::UI::Accessibility::SetWinEventHook;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART,
        WINEVENT_OUTOFCONTEXT,
    };
    let premier = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            std::ptr::null_mut(),
            Some(sur_premier_plan),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    let reduction = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_MINIMIZESTART,
            EVENT_SYSTEM_MINIMIZEEND,
            std::ptr::null_mut(),
            Some(sur_premier_plan),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    !premier.is_null() && !reduction.is_null()
}

#[cfg(not(windows))]
pub fn ecouter_le_premier_plan() -> bool {
    false
}

#[cfg(windows)]
unsafe extern "system" fn sur_premier_plan(
    _crochet: windows_sys::Win32::UI::Accessibility::HWINEVENTHOOK,
    _evenement: u32,
    _hwnd: windows_sys::Win32::Foundation::HWND,
    _objet: i32,
    _enfant: i32,
    _fil: u32,
    _temps: u32,
) {
    // Quel que soit l'evenement, la question est la meme : qui est devant
    // maintenant ? La reponse est relue ici, pas deduite de l'evenement.
    AVANT_PLAN.store(premier_plan(), Ordering::Relaxed);
    AVANT_PLAN_CHANGE.store(true, Ordering::Relaxed);
}

/// Le premier plan a-t-il change depuis la derniere fois ? Rend la fenetre
/// passee devant.
pub fn premier_plan_change() -> Option<isize> {
    if AVANT_PLAN_CHANGE.swap(false, Ordering::Relaxed) {
        Some(AVANT_PLAN.load(Ordering::Relaxed))
    } else {
        None
    }
}

/// La fenetre au premier plan, maintenant. Ne sert qu'a la premiere lecture,
/// au lancement, et a la procedure du crochet.
#[cfg(windows)]
pub fn premier_plan() -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe { GetForegroundWindow() as isize }
}

#[cfg(not(windows))]
pub fn premier_plan() -> isize {
    0
}

/// Le titre d'une fenetre, pour le journal.
#[cfg(windows)]
pub fn titre(hwnd: isize) -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowTextW;
    let mut tampon = [0u16; 256];
    let n = unsafe { GetWindowTextW(hwnd as _, tampon.as_mut_ptr(), tampon.len() as i32) };
    String::from_utf16_lossy(&tampon[..n.max(0) as usize])
}

#[cfg(not(windows))]
pub fn titre(_hwnd: isize) -> String {
    String::new()
}

/// L'etat des notifications du systeme (`SHQueryUserNotificationState`) : 1
/// absent, 2 occupe (une application en plein ecran), 3 jeu Direct3D en plein
/// ecran, 4 mode presentation, 5 accepte les notifications, 6 heures calmes,
/// 7 une application du Store en plein ecran ; 0 si l'appel echoue.
#[cfg(windows)]
pub fn etat_des_notifications() -> i32 {
    use windows_sys::Win32::UI::Shell::SHQueryUserNotificationState;
    let mut etat = 0i32;
    let r = unsafe { SHQueryUserNotificationState(&mut etat) };
    if r < 0 {
        0
    } else {
        etat
    }
}

#[cfg(not(windows))]
pub fn etat_des_notifications() -> i32 {
    0
}

/// Pourquoi l'ilot se retire devant ce que le systeme dit de l'ecran, s'il le
/// doit (DESIGN-ILOT.md §1.2 ; DECISION-APRES-EPREUVE.md, §H « Presence ») :
/// une application en plein ecran, un jeu en plein ecran, le mode presentation.
/// `None` sinon. Microsoft ecrit qu'aucune notification n'est envoyee quand une
/// application entre en plein ecran : l'etat se relit a chaque changement de
/// premier plan.
pub fn occupation() -> Option<&'static str> {
    match etat_des_notifications() {
        // La session verrouillee, l'ecran de veille : personne devant l'ecran,
        // rien ne se dessine (DESIGN-ILOT.md §1.2, la session se verrouille).
        1 => Some("personne devant l'ecran, session verrouillee (QUNS_NOT_PRESENT)"),
        2 => Some("une application en plein ecran (QUNS_BUSY)"),
        3 => Some("un jeu Direct3D en plein ecran (QUNS_RUNNING_D3D_FULL_SCREEN)"),
        4 => Some("le mode presentation (QUNS_PRESENTATION_MODE)"),
        7 => Some("une application en plein ecran (QUNS_APP)"),
        _ => None,
    }
}

/// La duree d'une notification du systeme (`SPI_GETMESSAGEDURATION`, en
/// secondes, 5 par defaut ; reglee dans les options d'accessibilite de
/// Windows) : c'est le temps que l'ilot reste deploye pour une alerte
/// (DESIGN-ILOT.md §1.4, m8).
#[cfg(windows)]
pub fn duree_du_message() -> std::time::Duration {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETMESSAGEDURATION};
    let mut secondes: u32 = 0;
    let ok = unsafe {
        SystemParametersInfoW(SPI_GETMESSAGEDURATION, 0, &mut secondes as *mut u32 as *mut _, 0)
    };
    if ok == 0 || secondes == 0 {
        std::time::Duration::from_secs(5)
    } else {
        std::time::Duration::from_secs(secondes as u64)
    }
}

#[cfg(not(windows))]
pub fn duree_du_message() -> std::time::Duration {
    std::time::Duration::from_secs(5)
}

/// Rend la fenetre reductible par Win+D et Win+M : `WS_MINIMIZEBOX` dans son
/// style. Winit l'ote a toute fenetre plein ecran sans bord (`to_window_styles`
/// retire `WS_OVERLAPPEDWINDOW` entier), et le bureau ne passe alors jamais
/// devant l'horloge : Win+D lui prend le focus sans la reduire, la fenetre
/// reste sur l'ecran, et `SHQueryUserNotificationState` repond « occupe »
/// (mesure `captures/integration-apres-epreuve/pas5/bureau-quns.txt`). Sans
/// `WS_CAPTION`, le bit ne dessine rien. Winit ne reecrit le style qu'au
/// changement d'un de ses drapeaux (visibilite, decorations, curseur), et
/// l'horloge n'en change plus apres sa naissance.
#[cfg(windows)]
pub fn rendre_reductible(hwnd: isize) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE, WS_MINIMIZEBOX,
    };
    unsafe {
        let style = GetWindowLongPtrW(hwnd as _, GWL_STYLE);
        SetWindowLongPtrW(hwnd as _, GWL_STYLE, style | WS_MINIMIZEBOX as isize);
        GetWindowLongPtrW(hwnd as _, GWL_STYLE) & WS_MINIMIZEBOX as isize != 0
    }
}

#[cfg(not(windows))]
pub fn rendre_reductible(_hwnd: isize) -> bool {
    false
}

/// Ote a la fenetre la marque « application plein ecran » de la barre des
/// taches (`ITaskbarList2::MarkFullscreenWindow`, faux). Winit la pose a toute
/// fenetre qu'il met en plein ecran, et le bureau la prend au mot : Win+D
/// n'en reduit pas une (mesure `pas5/reduction.txt` et `style-winD` : ni le
/// style, ni le style etendu, ni la taille n'y changent rien, quand une
/// fenetre nue de la meme taille se reduit), et `SHQueryUserNotificationState`
/// repond « occupe » tant qu'elle est sur l'ecran. La sequence est celle de
/// winit (`window.rs`, `taskbar_mark_fullscreen`), COM initialise en
/// appartement sur ce fil comme lui.
#[cfg(windows)]
pub fn demarquer_plein_ecran(hwnd: isize) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{ITaskbarList2, TaskbarList};
    unsafe {
        // Deja fait par winit sur ce fil ; un second appel compte, sans nuire.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let liste: ITaskbarList2 =
            CoCreateInstance(&TaskbarList, None, CLSCTX_ALL).map_err(|e| format!("CoCreateInstance : {e}"))?;
        liste.HrInit().map_err(|e| format!("HrInit : {e}"))?;
        liste
            .MarkFullscreenWindow(HWND(hwnd as *mut _), false)
            .map_err(|e| format!("MarkFullscreenWindow : {e}"))
    }
}

#[cfg(not(windows))]
pub fn demarquer_plein_ecran(_hwnd: isize) -> Result<(), String> {
    Err("pas Windows".to_string())
}

/// Le delai de l'appui maintenu (`SPI_GETMOUSEHOVERTIME`, en millisecondes,
/// 400 sur cette machine) : la seule duree « pointeur immobile » que Windows
/// publie, et celle que la decision prend pour le geste (DECISION-APRES-
/// EPREUVE.md §B). Relu a chaque `WM_SETTINGCHANGE`.
#[cfg(windows)]
pub fn delai_de_survol() -> std::time::Duration {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETMOUSEHOVERTIME};
    let mut ms: u32 = 0;
    let ok = unsafe { SystemParametersInfoW(SPI_GETMOUSEHOVERTIME, 0, &mut ms as *mut u32 as *mut _, 0) };
    if ok == 0 || ms == 0 {
        std::time::Duration::from_millis(400)
    } else {
        std::time::Duration::from_millis(ms as u64)
    }
}

#[cfg(not(windows))]
pub fn delai_de_survol() -> std::time::Duration {
    std::time::Duration::from_millis(400)
}

/// Le delai entre deux clics successifs (`GetDoubleClickTime`, en millisecondes,
/// 500 sur cette machine) : la seule duree « clics successifs » que Windows
/// publie, et celle que le systeme applique lui-meme pour decider d'un double
/// clic. C'est elle que le triple clic du galet replie prend
/// (`GESTE-TRIPLE-CLIC.md` §6). Relue a chaque `WM_SETTINGCHANGE`, comme le
/// delai de survol ; l'utilisateur la regle entre 200 et 900 ms dans le panneau
/// de la souris.
#[cfg(windows)]
pub fn delai_du_double_clic() -> std::time::Duration {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
    let ms = unsafe { GetDoubleClickTime() };
    if ms == 0 {
        std::time::Duration::from_millis(500)
    } else {
        std::time::Duration::from_millis(ms as u64)
    }
}

#[cfg(not(windows))]
pub fn delai_du_double_clic() -> std::time::Duration {
    std::time::Duration::from_millis(500)
}

/// Le rectangle du double clic (`SM_CXDOUBLECLK` x `SM_CYDOUBLECLK`, 4 x 4 sur
/// cette machine) : la boite, centree sur le PREMIER appui, hors de laquelle
/// Windows cesse de compter les clics comme successifs.
///
/// **Ces deux metriques sont en pixels PHYSIQUES, et ne suivent pas le DPI.**
/// Mesure du 18 septembre 2026 : `GetSystemMetricsForDpi(SM_CXDOUBLECLK, dpi)`
/// rend 4 a 96, 120, 144 et 192 ppp, et `GetSystemMetrics` rend 4 dans un
/// processus conscient par moniteur dont le DPI systeme vaut 120. Elles ne se
/// multiplient donc jamais par l'echelle de l'ilot, a la difference des 4 px
/// LOGIQUES du glissement, qui sont un chiffre de la charte (DESIGN.md §6).
#[cfg(windows)]
pub fn rectangle_du_double_clic() -> (f64, f64) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXDOUBLECLK, SM_CYDOUBLECLK,
    };
    let l = unsafe { GetSystemMetrics(SM_CXDOUBLECLK) };
    let h = unsafe { GetSystemMetrics(SM_CYDOUBLECLK) };
    (
        if l > 0 { l as f64 } else { 4.0 },
        if h > 0 { h as f64 } else { 4.0 },
    )
}

#[cfg(not(windows))]
pub fn rectangle_du_double_clic() -> (f64, f64) {
    (4.0, 4.0)
}

/// L'identifiant de modele d'application, pose explicitement.
///
/// Un element epingle a la barre des taches est un `.lnk`, pas l'executable ;
/// c'est l'AppUserModelID qui apparie la fenetre en cours et le raccourci
/// epingle. Sans identifiant explicite, Windows en calcule un par heuristiques,
/// et Microsoft ecrit qu'un identifiant explicite est le seul moyen de garantir
/// l'experience exacte (https://learn.microsoft.com/en-us/windows/win32/shell/appids,
/// `ms.date` 2018-05-31, mise a jour 2025-04-15).
///
/// La forme est celle que la page impose : `Societe.Produit.SousProduit.Version`,
/// 128 caracteres au plus, SANS ESPACE, chaque section en PascalCase. Et elle
/// se pose « pendant l'initialisation, avant que l'application presente la
/// moindre interface » — d'ou l'appel au debut de `main`, avant la boucle
/// d'evenements et donc avant toute fenetre.
#[cfg(windows)]
pub const IDENTIFIANT_APPLICATION: &str = "Stone.Veilleuse.Horloge.1";

/// Ce que Windows dit de l'identifiant du processus, relu chez lui.
///
/// `GetCurrentProcessExplicitAppUserModelID` rend une chaine allouee par COM
/// qu'il faut rendre. Avant tout appel a `Set...`, il echoue : c'est ce qui
/// permet a la declaration ci-dessous de se verifier elle-meme au lieu de
/// repeter ce qu'on lui a donne.
#[cfg(windows)]
pub fn identifiant_relu() -> Option<String> {
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::GetCurrentProcessExplicitAppUserModelID;
    let mut p: *mut u16 = std::ptr::null_mut();
    let hr = unsafe { GetCurrentProcessExplicitAppUserModelID(&mut p) };
    if hr != 0 || p.is_null() {
        return None;
    }
    let mut n = 0usize;
    while unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    let s = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, n) });
    unsafe { CoTaskMemFree(p as *mut core::ffi::c_void) };
    Some(s)
}

#[cfg(windows)]
pub fn declarer_identifiant_application() -> Result<String, String> {
    use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    // Avant : Windows n'en a aucun. C'est l'epreuve de la relecture, et elle
    // vit dans la fonction meme, pour qu'un jour ou l'appel echouerait, la
    // ligne annoncee ne puisse pas le cacher.
    let avant = identifiant_relu();
    let large: Vec<u16> = IDENTIFIANT_APPLICATION
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let hr = unsafe { SetCurrentProcessExplicitAppUserModelID(large.as_ptr()) };
    if hr != 0 {
        return Err(format!("SetCurrentProcessExplicitAppUserModelID : 0x{hr:08X}"));
    }
    match identifiant_relu() {
        Some(apres) if apres == IDENTIFIANT_APPLICATION => Ok(format!(
            "{apres} (avant l'appel : {})",
            avant.unwrap_or_else(|| "aucun".into())
        )),
        Some(autre) => Err(format!("relu {autre} au lieu de {IDENTIFIANT_APPLICATION}")),
        None => Err("pose sans erreur, mais Windows n'en rend aucun".into()),
    }
}

/// Les deux tailles d'icone de fenetre que Windows emploie sur cette machine,
/// en pixels PHYSIQUES : la petite (`SM_CXSMICON`, celle d'`ICON_SMALL` — barre
/// de titre, coin de la fenetre) et la grande (`SM_CXICON`, celle d'`ICON_BIG`
/// — Alt+Tab).
///
/// ~~`ICON_BIG` — Alt+Tab, barre des taches~~ : la barre des taches de
/// Windows 11 ne dessine aucune des deux. Elle prend l'icone du FICHIER par le
/// cache du shell (`INTEGRATION-ICONE.md`, « En plus des douze pas »).
///
/// **Pourquoi on les demande au lieu de laisser le systeme choisir.**
/// `Icon::from_resource(1, None)` passe `0, 0` et `LR_DEFAULTSIZE` a
/// `LoadImageW` (`winit 0.30.13`, `platform_impl/windows/icon.rs`), et
/// `LR_DEFAULTSIZE` rend la GRANDE taille — la meme des deux cotes. La petite
/// icone recevrait alors une image de 40 px que Windows reduirait a 20 pour la
/// barre de titre : exactement la mise a l'echelle que le fichier a quatorze
/// entrees existe pour eviter. On demande donc a `LoadImageW` la taille exacte
/// que le systeme va employer, et le fichier la porte.
///
/// Relevé du 18 septembre 2026 dans un processus conscient du DPI, ecran a
/// 125 % : petite 20, grande 40. Le repli 16 et 32 est celui des metriques a
/// 100 %, et il ne sert que si l'appel echoue.
#[cfg(windows)]
pub fn tailles_d_icone() -> (u32, u32) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXICON, SM_CXSMICON,
    };
    let petite = unsafe { GetSystemMetrics(SM_CXSMICON) };
    let grande = unsafe { GetSystemMetrics(SM_CXICON) };
    (
        if petite > 0 { petite as u32 } else { 16 },
        if grande > 0 { grande as u32 } else { 32 },
    )
}

/// Un ecran, tel que Windows le decrit : sa poignee, son nom de peripherique,
/// son rectangle entier et sa zone de travail, en pixels physiques du bureau
/// virtuel (x, y, largeur, hauteur).
#[derive(Clone, Debug, PartialEq)]
pub struct Ecran {
    pub hmonitor: isize,
    pub nom: String,
    pub entier: (i32, i32, i32, i32),
    pub travail: (i32, i32, i32, i32),
}

/// L'ecran qui a la plus grande part d'une fenetre (`MonitorFromWindow`,
/// `MONITOR_DEFAULTTONEAREST`), avec sa zone de travail (`rcWork` : une barre
/// des taches en haut ou masquee est ainsi comptee, DESIGN-ILOT.md §8.2).
#[cfg(windows)]
pub fn ecran_de(hwnd: isize) -> Option<Ecran> {
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
    };
    let moniteur = unsafe { MonitorFromWindow(hwnd as _, MONITOR_DEFAULTTONEAREST) };
    if moniteur.is_null() {
        return None;
    }
    let mut info: MONITORINFOEXW = unsafe { std::mem::zeroed() };
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if unsafe { GetMonitorInfoW(moniteur, &mut info as *mut MONITORINFOEXW as *mut _) } == 0 {
        return None;
    }
    let fin = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
    let nom = String::from_utf16_lossy(&info.szDevice[..fin]);
    let m = info.monitorInfo.rcMonitor;
    let t = info.monitorInfo.rcWork;
    Some(Ecran {
        hmonitor: moniteur as isize,
        nom,
        entier: (m.left, m.top, m.right - m.left, m.bottom - m.top),
        travail: (t.left, t.top, t.right - t.left, t.bottom - t.top),
    })
}

#[cfg(not(windows))]
pub fn ecran_de(_hwnd: isize) -> Option<Ecran> {
    None
}
