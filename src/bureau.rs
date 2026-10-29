//! Le bureau sous l'ilot, tel que la duplication le rend, et sans l'ilot dedans.
//!
//! Porte de `recherche/verre-taille/src/bureau.rs`, ou chaque regle ci-dessous a
//! ete payee a l'essai (`VERRE-TAILLE.md` §1, §2 et §6). Rien ne s'y voit a la
//! lecture du code :
//!
//! 1. **L'exclusion.** L'ilot se declare invisible a la capture
//!    (`WDA_EXCLUDEFROMCAPTURE`). Elle vaut pour la duplication : 115 200 pixels
//!    vus sans le drapeau, 0 avec. Sans elle, le verre boucle sur son reflet.
//! 2. **Le DPI.** Sans conscience du DPI declaree AVANT toute fenetre et tout
//!    DXGI, les coordonnees du bureau reviennent a l'echelle (1536 x 864) alors
//!    que la texture est en pixels physiques (1920 x 1080). `main` la declare ;
//!    `Capteur::ouvrir` la RELIT et compare les deux tailles, pour que l'ordre
//!    soit verifie a chaque ouverture au lieu d'etre suppose.
//! 3. **`LastPresentTime`.** `AcquireNextFrame` reussit aussi pour un simple
//!    mouvement de curseur, et la texture ne porte alors aucune image neuve.
//!    C'est le seul signal fiable, et il vaut aussi pour la copie forcee.
//! 4. **`ReleaseFrame`.** Il rend la texture indefinie : la copie a lieu AVANT,
//!    sinon on lit un noir parfait.
//!
//! Le regime est celui que le banc a mesure : acquisition NON BLOQUANTE (la
//! boucle de rendu ne doit jamais se caler sur l'activite des autres fenetres),
//! filtre par rectangles sales et de deplacement, copie forcee quand la region
//! suivie change, reouverture quand l'acces est perdu.
//!
//! L'adaptateur de la duplication n'est pas forcement celui de wgpu : cette
//! machine en a deux, et `DuplicateOutput` echoue sur celui qui ne pilote pas la
//! sortie. La region revient donc au processeur, en octets RGBA ; le verre la
//! televersera. Aucune interoperation entre peripheriques.

use std::time::{Duration, Instant};

use windows::core::Interface;
use windows::Win32::Foundation::{HMODULE, HWND, RECT};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::UI::HiDpi::{
    GetAwarenessFromDpiAwarenessContext, GetThreadDpiAwarenessContext,
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    DPI_AWARENESS_CONTEXT_UNAWARE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowDisplayAffinity, SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
};

// La marge capturee autour du galet n'est plus une constante d'ici : elle suit
// l'echelle de l'ecran avec le sigma du flou, et vit a cote des parametres du
// verre (`galet::marge`).

/// Attente minimale entre deux tentatives d'ouverture. Une ouverture coute un
/// peripherique D3D11 entier ; la retenter a chaque trame pendant un bureau
/// securise ferait travailler la carte pour rien.
const REOUVERTURE: Duration = Duration::from_secs(1);

// --- La conscience du DPI ----------------------------------------------------

/// Ce que la declaration de la conscience du DPI a donne.
#[derive(Clone, Debug)]
pub struct Conscience {
    pub acceptee: bool,
    /// Code de l'echec, 0 si acceptee. `0x80070005` (acces refuse) : la
    /// conscience etait deja fixee avant nous.
    pub erreur: i32,
    /// La conscience relue juste apres.
    pub lue: i32,
    /// La couche de compatibilite que Windows attache a CE chemin
    /// d'executable, s'il y en a une (`HKCU\...\AppCompatFlags\Layers`).
    /// `HIGHDPIAWARE` y fixe la conscience au lancement, avant `main` : la
    /// declaration est alors refusee alors que la conscience est la bonne.
    pub couche: Option<String>,
}

/// Declare la conscience du DPI par moniteur, v2. A appeler avant toute fenetre
/// et tout DXGI.
///
/// `inconscient` sert a l'epreuve du temoin DPI : le processus se declare alors
/// INCONSCIENT, pour verifier que `Capteur::ouvrir` sait voir un ordre casse.
pub fn declarer_conscience_dpi(inconscient: bool) -> Conscience {
    let contexte = if inconscient {
        DPI_AWARENESS_CONTEXT_UNAWARE
    } else {
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
    };
    let r = unsafe { SetProcessDpiAwarenessContext(contexte) };
    Conscience {
        acceptee: r.is_ok(),
        erreur: r.err().map_or(0, |e| e.code().0),
        lue: conscience_dpi(),
        couche: couche_de_compatibilite(),
    }
}

/// Lit, sans rien ecrire, la couche de compatibilite de l'executable courant.
fn couche_de_compatibilite() -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
    let exe = std::env::current_exe().ok()?;
    let valeur: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
    let cle: Vec<u16> = "Software\\Microsoft\\Windows NT\\CurrentVersion\\AppCompatFlags\\Layers"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut tampon = vec![0u16; 1024];
    let mut octets = (tampon.len() * 2) as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            cle.as_ptr(),
            valeur.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            tampon.as_mut_ptr().cast(),
            &mut octets,
        )
    };
    if r != 0 {
        return None;
    }
    let n = (octets as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&tampon[..n]))
}

/// La conscience du DPI en vigueur sur ce fil : 0 inconscient, 1 systeme,
/// 2 par moniteur, -1 invalide.
pub fn conscience_dpi() -> i32 {
    unsafe { GetAwarenessFromDpiAwarenessContext(GetThreadDpiAwarenessContext()).0 }
}

// --- L'exclusion de la capture ---------------------------------------------

/// Pose ou retire l'exclusion de la capture sur une fenetre, puis RELIT
/// l'affinite effective. Rend cette affinite relue (17 = exclue, 0 = aucune),
/// ou `None` si la lecture echoue.
pub fn exclure_de_la_capture(hwnd: isize, exclure: bool) -> Option<u32> {
    let h = HWND(hwnd as *mut core::ffi::c_void);
    unsafe {
        let _ = SetWindowDisplayAffinity(
            h,
            if exclure { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE },
        );
        let mut lue = 0u32;
        GetWindowDisplayAffinity(h, &mut lue).ok()?;
        Some(lue)
    }
}

// --- La region -----------------------------------------------------------------

/// Un rectangle du bureau virtuel, en pixels physiques.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub l: i32,
    pub h: i32,
}

/// Une region du bureau en octets RGBA, avec l'endroit ou elle a ete prise.
///
/// L'origine est celle de ce qui a ete REELLEMENT copie : une region demandee a
/// cheval sur le bord de l'ecran est rognee, et le verre doit savoir de combien.
pub struct Region {
    pub pixels: Vec<u8>,
    /// Coin haut-gauche, dans le bureau virtuel.
    pub x: i32,
    pub y: i32,
    pub largeur: u32,
    pub hauteur: u32,
}

/// Ce qu'une acquisition a rapporte.
enum Acquise {
    Neuve(Region),
    Inchange,
    Perdu,
}

/// `DXGI_ERROR_WAIT_TIMEOUT` : rien de neuf, et ce n'est pas une erreur.
const EXPIRE: i32 = 0x887A0027u32 as i32;

// --- Le capteur ------------------------------------------------------------------

/// Garde le peripherique et la duplication ouverts : les recreer a chaque trame
/// couterait plusieurs millisecondes et reveillerait la carte.
pub struct Capteur {
    appareil: ID3D11Device,
    contexte: ID3D11DeviceContext,
    duplication: IDXGIOutputDuplication,
    /// Texture de relecture gardee d'une copie a l'autre, tant que la taille ne
    /// change pas.
    lecture: Option<(ID3D11Texture2D, u32, u32)>,
    /// Ce qui a fait juger la derniere image touchante, pour le relevé.
    contact: Option<Contact>,
    /// La sortie dans le bureau virtuel : les coordonnees de fenetre sont
    /// absolues, la texture commence a zero.
    pub sortie: Rect,
    /// La taille de la texture dupliquee, toujours en pixels physiques. C'est le
    /// temoin du DPI : elle doit valoir la taille de `sortie`.
    pub texture: (u32, u32),
    pub adaptateur: String,
    /// La conscience du DPI relue a l'ouverture.
    pub conscience: i32,
}

impl Capteur {
    /// Ouvre la duplication de la sortie qui contient le point donne.
    pub fn ouvrir(x: i32, y: i32) -> windows::core::Result<Capteur> {
        unsafe {
            let fabrique: IDXGIFactory1 = CreateDXGIFactory1()?;
            let mut ia = 0u32;
            while let Ok(adaptateur) = fabrique.EnumAdapters1(ia) {
                let desc = adaptateur.GetDesc1()?;
                let nom = String::from_utf16_lossy(&desc.Description)
                    .trim_end_matches(char::from(0))
                    .to_string();
                let mut io = 0u32;
                while let Ok(sortie) = adaptateur.EnumOutputs(io) {
                    let d = sortie.GetDesc()?;
                    let r: RECT = d.DesktopCoordinates;
                    if x >= r.left && x < r.right && y >= r.top && y < r.bottom {
                        let s1: IDXGIOutput1 = sortie.cast()?;
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
                        let appareil = appareil.ok_or_else(windows::core::Error::empty)?;
                        let contexte = contexte.ok_or_else(windows::core::Error::empty)?;
                        let duplication = s1.DuplicateOutput(&appareil)?;
                        let mode = duplication.GetDesc().ModeDesc;
                        return Ok(Capteur {
                            appareil,
                            contexte,
                            duplication,
                            lecture: None,
                            contact: None,
                            sortie: Rect {
                                x: r.left,
                                y: r.top,
                                l: r.right - r.left,
                                h: r.bottom - r.top,
                            },
                            texture: (mode.Width, mode.Height),
                            adaptateur: nom,
                            conscience: conscience_dpi(),
                        });
                    }
                    io += 1;
                }
                ia += 1;
            }
            Err(windows::core::Error::empty())
        }
    }

    /// Les coordonnees du bureau et la texture parlent-elles le meme pixel ?
    /// Faux quand la conscience du DPI a ete declaree trop tard (ou pas du tout).
    ///
    /// Une sortie tournee de 90 degres echangerait les deux dimensions de la
    /// texture ; ce cas n'est pas mesure.
    pub fn dpi_accorde(&self) -> bool {
        self.texture == (self.sortie.l as u32, self.sortie.h as u32)
    }

    fn contient(&self, x: i32, y: i32) -> bool {
        let s = self.sortie;
        x >= s.x && x < s.x + s.l && y >= s.y && y < s.y + s.h
    }

    /// Une tentative NON BLOQUANTE sur la region `r`.
    ///
    /// `forcer` : la region a change depuis la derniere copie reussie, et aucun
    /// rectangle sale ne peut dire ce qu'il y a sous sa NOUVELLE position. On
    /// copie alors la prochaine image presentee, qu'elle touche la region ou
    /// non. Seulement une image presentee : une trame de curseur seul
    /// (`LastPresentTime` nul) ne porte aucune image, et le banc la copiait
    /// quand meme dans sa `region_forcee` — ce port ne le fait pas.
    fn acquerir(&mut self, r: Rect, forcer: bool) -> Acquise {
        unsafe {
            let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut ressource: Option<IDXGIResource> = None;
            if let Err(e) = self.duplication.AcquireNextFrame(0, &mut info, &mut ressource) {
                // Toute autre erreur qu'une expiration est traitee en perte :
                // la duplication se rouvre, ce qui est le bon remede a
                // `DXGI_ERROR_ACCESS_LOST` comme a ce qu'on ne connait pas.
                return if e.code().0 == EXPIRE {
                    Acquise::Inchange
                } else {
                    Acquise::Perdu
                };
            }
            let sortie = 'copie: {
                if info.LastPresentTime == 0 {
                    break 'copie Acquise::Inchange;
                }
                // Le bureau a bouge quelque part, peut-etre pas sous nous.
                self.contact = None;
                if !forcer {
                    let contact = self.touche(r, &info);
                    if contact == Contact::Aucun {
                        break 'copie Acquise::Inchange;
                    }
                    self.contact = Some(contact);
                }
                let Some(image) = ressource
                    .as_ref()
                    .and_then(|res| res.cast::<ID3D11Texture2D>().ok())
                else {
                    break 'copie Acquise::Inchange;
                };
                match self.copier(&image, r) {
                    Some(region) => Acquise::Neuve(region),
                    None => Acquise::Inchange,
                }
            };
            // APRES la copie, toujours : ensuite la texture est indefinie.
            let _ = self.duplication.ReleaseFrame();
            sortie
        }
    }

    /// La copie d'une region vers le processeur. C'est elle, et non l'attente,
    /// qui porte le cout reel.
    unsafe fn copier(&mut self, image: &ID3D11Texture2D, r: Rect) -> Option<Region> {
        // Intersection de la region demandee avec la sortie, dans la texture.
        let s = self.sortie;
        let gauche = r.x.max(s.x);
        let haut = r.y.max(s.y);
        let droite = (r.x + r.l).min(s.x + s.l);
        let bas = (r.y + r.h).min(s.y + s.h);
        if droite <= gauche || bas <= haut {
            return None;
        }
        let (gx, gy) = ((gauche - s.x) as u32, (haut - s.y) as u32);
        let (gl, gh) = ((droite - gauche) as u32, (bas - haut) as u32);
        if gx + gl > self.texture.0 || gy + gh > self.texture.1 {
            // Coordonnees et texture en desaccord : le DPI est casse, et copier
            // lirait hors de la texture.
            return None;
        }

        let lecture = match &self.lecture {
            Some((t, l, h)) if (*l, *h) == (gl, gh) => t.clone(),
            _ => {
                let mut d = D3D11_TEXTURE2D_DESC {
                    Width: gl,
                    Height: gh,
                    MipLevels: 1,
                    ArraySize: 1,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    Usage: D3D11_USAGE_STAGING,
                    BindFlags: 0,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                    MiscFlags: 0,
                    ..Default::default()
                };
                d.SampleDesc.Count = 1;
                let mut t: Option<ID3D11Texture2D> = None;
                self.appareil.CreateTexture2D(&d, None, Some(&mut t)).ok()?;
                let t = t?;
                self.lecture = Some((t.clone(), gl, gh));
                t
            }
        };
        let boite = D3D11_BOX {
            left: gx,
            top: gy,
            front: 0,
            right: gx + gl,
            bottom: gy + gh,
            back: 1,
        };
        self.contexte
            .CopySubresourceRegion(&lecture, 0, 0, 0, 0, image, 0, Some(&boite));
        let mut m = D3D11_MAPPED_SUBRESOURCE::default();
        self.contexte
            .Map(&lecture, 0, D3D11_MAP_READ, 0, Some(&mut m))
            .ok()?;
        let mut pixels = vec![0u8; (gl * gh * 4) as usize];
        for ligne in 0..gh {
            let src = (m.pData as *const u8).add((ligne * m.RowPitch) as usize);
            for col in 0..gl {
                let p = src.add((col * 4) as usize);
                let d = ((ligne * gl + col) * 4) as usize;
                // BGRA vers RGBA.
                pixels[d] = *p.add(2);
                pixels[d + 1] = *p.add(1);
                pixels[d + 2] = *p;
                pixels[d + 3] = 255;
            }
        }
        self.contexte.Unmap(&lecture, 0);
        Some(Region {
            pixels,
            x: gauche,
            y: haut,
            largeur: gl,
            hauteur: gh,
        })
    }

    /// Le changement touche-t-il notre region, et par quoi ?
    ///
    /// En cas de doute la reponse est OUI : un faux positif coute une copie, un
    /// faux negatif laisse un verre perime a l'ecran. Les deux erreurs n'ont pas
    /// le meme prix.
    unsafe fn touche(&self, r: Rect, info: &DXGI_OUTDUPL_FRAME_INFO) -> Contact {
        if info.TotalMetadataBufferSize == 0 {
            return Contact::SansMetadonnees;
        }
        let notre = RECT {
            left: r.x - self.sortie.x,
            top: r.y - self.sortie.y,
            right: r.x - self.sortie.x + r.l,
            bottom: r.y - self.sortie.y + r.h,
        };
        let croise = |q: &RECT| {
            q.left < notre.right && q.right > notre.left && q.top < notre.bottom && q.bottom > notre.top
        };

        let mut tampon = vec![0u8; info.TotalMetadataBufferSize as usize];
        let mut taille = 0u32;
        // Les deplacements d'abord : la source et la destination comptent
        // toutes les deux.
        if self
            .duplication
            .GetFrameMoveRects(
                tampon.len() as u32,
                tampon.as_mut_ptr() as *mut DXGI_OUTDUPL_MOVE_RECT,
                &mut taille,
            )
            .is_err()
        {
            return Contact::SansMetadonnees;
        }
        let n = taille as usize / std::mem::size_of::<DXGI_OUTDUPL_MOVE_RECT>();
        let mouvements =
            std::slice::from_raw_parts(tampon.as_ptr() as *const DXGI_OUTDUPL_MOVE_RECT, n);
        for m in mouvements {
            let source = RECT {
                left: m.SourcePoint.x,
                top: m.SourcePoint.y,
                right: m.SourcePoint.x + (m.DestinationRect.right - m.DestinationRect.left),
                bottom: m.SourcePoint.y + (m.DestinationRect.bottom - m.DestinationRect.top),
            };
            if croise(&m.DestinationRect) || croise(&source) {
                return Contact::Deplacement;
            }
        }
        // Puis les rectangles sales, dans ce qui reste du tampon.
        let reste = taille as usize;
        let mut taille2 = 0u32;
        if self
            .duplication
            .GetFrameDirtyRects(
                (tampon.len() - reste) as u32,
                tampon.as_mut_ptr().add(reste) as *mut RECT,
                &mut taille2,
            )
            .is_err()
        {
            return Contact::SansMetadonnees;
        }
        let n2 = taille2 as usize / std::mem::size_of::<RECT>();
        let sales = std::slice::from_raw_parts(tampon.as_ptr().add(reste) as *const RECT, n2);
        // Le plus grand rectangle sale qui nous croise : c'est lui qui dit si la
        // copie vient d'un petit changement voisin ou d'une presentation qui se
        // declare sale sur tout l'ecran.
        sales
            .iter()
            .filter(|q| croise(q))
            .max_by_key(|q| (q.right - q.left) as i64 * (q.bottom - q.top) as i64)
            .map_or(Contact::Aucun, |q| {
                Contact::Sale(Rect {
                    x: q.left + self.sortie.x,
                    y: q.top + self.sortie.y,
                    l: q.right - q.left,
                    h: q.bottom - q.top,
                })
            })
    }
}

/// Pourquoi une image presentee a ete jugee nous toucher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    Aucun,
    /// Pas de metadonnees, ou lecture en echec : la prudence dit oui.
    SansMetadonnees,
    Deplacement,
    /// Un rectangle sale croise la region ; le plus grand, dans le bureau.
    Sale(Rect),
}

// --- Le suivi ------------------------------------------------------------------

/// Ce qu'un tour de suivi a donne.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tour {
    /// Une image neuve de la region est la, dans `Suivi::region`.
    Neuve,
    /// Rien n'a change sous nous. C'est le cas NORMAL : la region precedente
    /// est toujours juste.
    Inchange,
    /// La region a change et aucune image n'a encore ete presentee depuis :
    /// `Suivi::region` est celle de l'ancienne position. Elle sera servie a la
    /// prochaine presentation.
    Attente,
    /// L'acces vient d'etre perdu ; la duplication se rouvrira.
    Perdu,
    /// Pas de duplication ouverte, et la reouverture attend son delai ou vient
    /// d'echouer.
    Ferme,
}

/// Les compteurs du relevé.
#[derive(Clone, Debug, Default)]
pub struct Compteurs {
    pub tours: u64,
    pub neuves: u64,
    /// Neuves servies parce que la region avait change (copie forcee).
    pub forcees: u64,
    pub inchanges: u64,
    pub attentes: u64,
    pub perdus: u64,
    pub fermes: u64,
    pub ouvertures: u64,
    pub echecs_ouverture: u64,
    /// Neuves non forcees, par ce qui les a declenchees.
    pub par_sans_metadonnees: u64,
    pub par_deplacement: u64,
    pub par_sale: u64,
    /// Le plus grand rectangle sale qui ait declenche une copie, depuis la
    /// derniere remise a zero par le relevé.
    pub plus_grand_sale: Option<Rect>,
}

pub struct Suivi {
    capteur: Option<Capteur>,
    derniere_tentative: Option<Instant>,
    /// La derniere region copiee, et le rectangle DEMANDE qui l'a produite. Les
    /// deux different au bord de l'ecran, et c'est le second qui dit si la
    /// region suivie a change.
    pub region: Option<Region>,
    demande: Option<Rect>,
    /// La prochaine acquisition doit copier quoi qu'il arrive : la duplication
    /// vient d'etre rouverte, ou le relevé l'exige.
    forcer: bool,
    pub compteurs: Compteurs,
    /// Duree de chaque tour, en millisecondes, gardee seulement si on la
    /// demande : l'application ne l'accumule pas toute la nuit.
    pub durees: Option<Vec<(Tour, f64)>>,
    /// La derniere plainte d'ouverture, pour le relevé.
    pub plainte: Option<String>,
    /// Aucune ouverture n'est permise : le relevé simule ainsi une capture
    /// perdue qui dure, comme un bureau securise, sans toucher a l'ecran.
    ouverture_empechee: bool,
}

impl Suivi {
    pub fn nouveau() -> Suivi {
        Suivi {
            capteur: None,
            derniere_tentative: None,
            region: None,
            demande: None,
            forcer: true,
            compteurs: Compteurs::default(),
            durees: None,
            plainte: None,
            ouverture_empechee: false,
        }
    }

    pub fn capteur(&self) -> Option<&Capteur> {
        self.capteur.as_ref()
    }

    /// Ferme la duplication, comme le ferait une perte d'acces. Ne sert qu'au
    /// relevé, pour exercer le chemin de reouverture sans toucher au mode de
    /// l'ecran.
    pub fn fermer(&mut self) {
        self.capteur = None;
        self.derniere_tentative = None;
    }

    /// Ferme la duplication et empeche de la rouvrir, ou leve l'empechement.
    /// Ne sert qu'au relevé : c'est le seul moyen d'exercer le repli du verre
    /// sur une perte qui dure, sans provoquer un vrai changement de mode.
    pub fn empecher_ouverture(&mut self, empecher: bool) {
        self.ouverture_empechee = empecher;
        if empecher {
            self.capteur = None;
        }
        self.derniere_tentative = None;
    }

    /// Exige une copie a la prochaine image presentee, meme si la region n'a
    /// pas change.
    pub fn exiger_copie(&mut self) {
        self.forcer = true;
    }

    /// Un tour de suivi sur la region `r` (le galet courant, marge comprise).
    pub fn tour(&mut self, r: Rect) -> Tour {
        let debut = Instant::now();
        let tour = self.tour_sans_mesure(r);
        let c = &mut self.compteurs;
        c.tours += 1;
        match tour {
            Tour::Neuve => c.neuves += 1,
            Tour::Inchange => c.inchanges += 1,
            Tour::Attente => c.attentes += 1,
            Tour::Perdu => c.perdus += 1,
            Tour::Ferme => c.fermes += 1,
        }
        if let Some(d) = self.durees.as_mut() {
            d.push((tour, debut.elapsed().as_secs_f64() * 1000.0));
        }
        tour
    }

    fn tour_sans_mesure(&mut self, r: Rect) -> Tour {
        let (cx, cy) = (r.x + r.l / 2, r.y + r.h / 2);
        // Le galet est passe sur un autre moniteur : la duplication ouverte ne
        // le voit plus. Ce cas n'est pas mesure, cette machine n'a qu'un ecran.
        if self.capteur.as_ref().is_some_and(|c| !c.contient(cx, cy)) {
            self.capteur = None;
        }
        if self.capteur.is_none() {
            if self.ouverture_empechee {
                return Tour::Ferme;
            }
            if self
                .derniere_tentative
                .is_some_and(|t| t.elapsed() < REOUVERTURE)
            {
                return Tour::Ferme;
            }
            self.derniere_tentative = Some(Instant::now());
            match Capteur::ouvrir(cx, cy) {
                Ok(c) => {
                    self.capteur = Some(c);
                    self.compteurs.ouvertures += 1;
                    self.plainte = None;
                    // Une duplication neuve ne rend que les changements a
                    // venir : la region d'avant ne se rattrape pas d'elle-meme.
                    self.forcer = true;
                }
                Err(e) => {
                    self.compteurs.echecs_ouverture += 1;
                    self.plainte = Some(format!("{e}"));
                    return Tour::Ferme;
                }
            }
        }
        let forcer = self.forcer || self.demande != Some(r);
        let capteur = self.capteur.as_mut().expect("ouvert juste au-dessus");
        let acquise = capteur.acquerir(r, forcer);
        let contact = capteur.contact;
        match acquise {
            Acquise::Neuve(region) => {
                let c = &mut self.compteurs;
                match contact {
                    _ if forcer => c.forcees += 1,
                    Some(Contact::SansMetadonnees) => c.par_sans_metadonnees += 1,
                    Some(Contact::Deplacement) => c.par_deplacement += 1,
                    Some(Contact::Sale(s)) => {
                        c.par_sale += 1;
                        let aire = |q: Rect| q.l as i64 * q.h as i64;
                        if c.plus_grand_sale.is_none_or(|p| aire(s) > aire(p)) {
                            c.plus_grand_sale = Some(s);
                        }
                    }
                    _ => {}
                }
                self.region = Some(region);
                self.demande = Some(r);
                self.forcer = false;
                Tour::Neuve
            }
            Acquise::Inchange => {
                if forcer {
                    Tour::Attente
                } else {
                    Tour::Inchange
                }
            }
            Acquise::Perdu => {
                self.capteur = None;
                // La reouverture se tente des le tour suivant.
                self.derniere_tentative = None;
                self.forcer = true;
                Tour::Perdu
            }
        }
    }
}
