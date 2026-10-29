//! Le bureau, tel que la duplication le rend — et sans nous dedans.
//!
//! Trois pieges, tous payes a l'essai et tous invisibles a la lecture :
//!
//! 1. **DPI.** Sans conscience du DPI, les coordonnees du bureau sont mises a
//!    l'echelle (1536x864) alors que la texture est en pixels physiques
//!    (1920x1080). On la demande AVANT toute fenetre et tout DXGI.
//! 2. **`LastPresentTime`.** `AcquireNextFrame` reussit aussi pour un simple
//!    mouvement de curseur, et la texture ne contient alors aucune image
//!    neuve. C'est le seul signal fiable.
//! 3. **`ReleaseFrame`.** Il rend le contenu de la texture indefini. La copie
//!    doit avoir lieu AVANT — sinon on lit un noir parfait et on croit que la
//!    duplication ne marche pas.

use windows::core::Interface;
use windows::Win32::Foundation::{HMODULE, RECT};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::*;

/// Une region du bureau, en octets RGBA, deja debarrassee de son alignement.
pub struct Region {
    pub pixels: Vec<u8>,
    pub largeur: u32,
    pub hauteur: u32,
}

/// Le capteur garde son peripherique et sa duplication ouverts : les recreer a
/// chaque trame couterait plusieurs millisecondes et reveillerait la carte.
pub struct Capteur {
    appareil: ID3D11Device,
    contexte: ID3D11DeviceContext,
    duplication: IDXGIOutputDuplication,
    /// Origine de la sortie dans le bureau virtuel : les coordonnees de fenetre
    /// sont absolues, la texture commence a zero.
    pub origine: (i32, i32),
    pub taille_ecran: (u32, u32),
    pub adaptateur: String,
}

impl Capteur {
    /// Ouvre la duplication de la sortie qui contient le point donne.
    ///
    /// L'adaptateur ne se devine pas : `DuplicateOutput` echoue s'il n'est pas
    /// celui qui pilote la sortie, et cette machine en a deux.
    pub fn ouvrir(x: i32, y: i32) -> windows::core::Result<Capteur> {
        unsafe {
            let fabrique: IDXGIFactory1 = CreateDXGIFactory1()?;
            let mut ia = 0u32;
            while let Ok(adaptateur) = fabrique.EnumAdapters1(ia) {
                let desc = adaptateur.GetDesc1()?;
                let nom: String = String::from_utf16_lossy(&desc.Description);
                let nom = nom.trim_end_matches(char::from(0)).to_string();
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
                        let appareil = appareil.unwrap();
                        let contexte = contexte.unwrap();
                        let duplication = s1.DuplicateOutput(&appareil)?;

                        return Ok(Capteur {
                            appareil,
                            contexte,
                            duplication,
                            origine: (r.left, r.top),
                            taille_ecran: (
                                (r.right - r.left) as u32,
                                (r.bottom - r.top) as u32,
                            ),
                            adaptateur: nom,
                        });
                    }
                    io += 1;
                }
                ia += 1;
            }
            Err(windows::core::Error::from_thread())
        }
    }

    /// Une region du bureau en RGBA. `x` et `y` sont absolus.
    ///
    /// `attentes` est le nombre de tours d'attente d'une image neuve : la
    /// duplication ne rend que les CHANGEMENTS, donc un bureau immobile ne
    /// produit rien et l'appel expire sans rien donner.
    pub fn region(
        &self,
        x: i32,
        y: i32,
        largeur: u32,
        hauteur: u32,
        attentes: u32,
        mut entre_deux: impl FnMut(),
    ) -> windows::core::Result<Region> {
        unsafe {
            // Le rectangle demande, ramene dans la texture et borne a l'ecran.
            let gx = (x - self.origine.0).max(0) as u32;
            let gy = (y - self.origine.1).max(0) as u32;
            let gl = largeur.min(self.taille_ecran.0.saturating_sub(gx));
            let gh = hauteur.min(self.taille_ecran.1.saturating_sub(gy));
            if gl == 0 || gh == 0 {
                return Err(windows::core::Error::from_thread());
            }

            let mut lecture = D3D11_TEXTURE2D_DESC {
                Width: gl,
                Height: gh,
                MipLevels: 1,
                ArraySize: 1,
                Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
                ..Default::default()
            };
            lecture.SampleDesc.Count = 1;
            let mut copie: Option<ID3D11Texture2D> = None;
            self.appareil
                .CreateTexture2D(&lecture, None, Some(&mut copie))?;
            let copie = copie.unwrap();

            let boite = D3D11_BOX {
                left: gx,
                top: gy,
                front: 0,
                right: gx + gl,
                bottom: gy + gh,
                back: 1,
            };

            for _ in 0..attentes {
                entre_deux();
                let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
                let mut ressource: Option<IDXGIResource> = None;
                if self
                    .duplication
                    .AcquireNextFrame(80, &mut info, &mut ressource)
                    .is_err()
                {
                    continue;
                }
                let mut pris = false;
                if info.LastPresentTime != 0 {
                    if let Some(r) = &ressource {
                        if let Ok(image) = r.cast::<ID3D11Texture2D>() {
                            // AVANT ReleaseFrame, imperativement.
                            self.contexte.CopySubresourceRegion(
                                &copie,
                                0,
                                0,
                                0,
                                0,
                                &image,
                                0,
                                Some(&boite),
                            );
                            pris = true;
                        }
                    }
                }
                let _ = self.duplication.ReleaseFrame();
                if !pris {
                    continue;
                }

                let mut m = D3D11_MAPPED_SUBRESOURCE::default();
                self.contexte
                    .Map(&copie, 0, D3D11_MAP_READ, 0, Some(&mut m))?;
                let mut pixels = vec![0u8; (gl * gh * 4) as usize];
                for ligne in 0..gh {
                    let src = (m.pData as *const u8).add((ligne * m.RowPitch) as usize);
                    for col in 0..gl {
                        let s = src.add((col * 4) as usize);
                        let d = ((ligne * gl + col) * 4) as usize;
                        // BGRA vers RGBA.
                        pixels[d] = *s.add(2);
                        pixels[d + 1] = *s.add(1);
                        pixels[d + 2] = *s;
                        pixels[d + 3] = 255;
                    }
                }
                self.contexte.Unmap(&copie, 0);
                return Ok(Region {
                    pixels,
                    largeur: gl,
                    hauteur: gh,
                });
            }
            Err(windows::core::Error::from_thread())
        }
    }
}

/// Ce qu'une tentative non bloquante rapporte.
pub enum Etat {
    /// Le bureau a presente une image neuve, la voici.
    Neuve(Region),
    /// Rien n'a change. C'est le cas NORMAL et il ne coute presque rien :
    /// on regarde le verre de la trame precedente, qui est toujours juste.
    Inchange,
    /// La duplication a perdu son acces : changement de mode, bureau securise,
    /// plein ecran exclusif. Il faut la rouvrir.
    Perdu,
}

/// `DXGI_ERROR_ACCESS_LOST`, qu'il faut distinguer d'une simple expiration.
const ACCES_PERDU: i32 = -2005270490; // 0x887A0026
/// `DXGI_ERROR_WAIT_TIMEOUT` : rien de neuf, et ce n'est pas une erreur.
const EXPIRE: i32 = -2005270489; // 0x887A0027

impl Capteur {
    /// Une tentative NON BLOQUANTE. C'est la forme qu'il faut en regime :
    /// attendre une trame du bureau dans la boucle de rendu reviendrait a
    /// caler l'affichage sur l'activite des autres fenetres.
    pub fn region_maintenant(&self, x: i32, y: i32, largeur: u32, hauteur: u32) -> Etat {
        unsafe {
            let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut ressource: Option<IDXGIResource> = None;
            match self.duplication.AcquireNextFrame(0, &mut info, &mut ressource) {
                Err(e) => {
                    let c = e.code().0;
                    if c == ACCES_PERDU {
                        return Etat::Perdu;
                    }
                    if c == EXPIRE {
                        return Etat::Inchange;
                    }
                    return Etat::Perdu;
                }
                Ok(()) => {}
            }
            // Une trame acquise sans image neuve : seul le curseur a bouge.
            if info.LastPresentTime == 0 {
                let _ = self.duplication.ReleaseFrame();
                return Etat::Inchange;
            }

            // Le bureau a bouge quelque part — mais peut-etre pas SOUS NOUS.
            // `AcquireNextFrame` se reveille pour tout l'ecran, donc sans ce
            // filtre on recopie 483 Ko parce qu'une horloge a change de minute
            // dans un coin. Les rectangles sales disent ou, exactement.
            if !self.touche(x, y, largeur, hauteur, &info) {
                let _ = self.duplication.ReleaseFrame();
                return Etat::Inchange;
            }
            let r = match &ressource {
                Some(r) => r.clone(),
                None => {
                    let _ = self.duplication.ReleaseFrame();
                    return Etat::Inchange;
                }
            };
            let image: ID3D11Texture2D = match r.cast() {
                Ok(t) => t,
                Err(_) => {
                    let _ = self.duplication.ReleaseFrame();
                    return Etat::Inchange;
                }
            };
            let sortie = self.copier(&image, x, y, largeur, hauteur);
            // AVANT de relacher, toujours : apres, la texture est indefinie.
            let _ = self.duplication.ReleaseFrame();
            match sortie {
                Some(r) => Etat::Neuve(r),
                None => Etat::Inchange,
            }
        }
    }

    /// La copie d'une region vers le CPU. Separee parce que c'est elle, et non
    /// l'attente, qui porte le cout reel.
    unsafe fn copier(
        &self,
        image: &ID3D11Texture2D,
        x: i32,
        y: i32,
        largeur: u32,
        hauteur: u32,
    ) -> Option<Region> {
        let gx = (x - self.origine.0).max(0) as u32;
        let gy = (y - self.origine.1).max(0) as u32;
        let gl = largeur.min(self.taille_ecran.0.saturating_sub(gx));
        let gh = hauteur.min(self.taille_ecran.1.saturating_sub(gy));
        if gl == 0 || gh == 0 {
            return None;
        }
        let mut d = D3D11_TEXTURE2D_DESC {
            Width: gl,
            Height: gh,
            MipLevels: 1,
            ArraySize: 1,
            Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
            ..Default::default()
        };
        d.SampleDesc.Count = 1;
        let mut copie: Option<ID3D11Texture2D> = None;
        self.appareil.CreateTexture2D(&d, None, Some(&mut copie)).ok()?;
        let copie = copie?;
        let boite = D3D11_BOX {
            left: gx,
            top: gy,
            front: 0,
            right: gx + gl,
            bottom: gy + gh,
            back: 1,
        };
        self.contexte
            .CopySubresourceRegion(&copie, 0, 0, 0, 0, image, 0, Some(&boite));
        let mut m = D3D11_MAPPED_SUBRESOURCE::default();
        self.contexte
            .Map(&copie, 0, D3D11_MAP_READ, 0, Some(&mut m))
            .ok()?;
        let mut pixels = vec![0u8; (gl * gh * 4) as usize];
        for ligne in 0..gh {
            let src = (m.pData as *const u8).add((ligne * m.RowPitch) as usize);
            for col in 0..gl {
                let s = src.add((col * 4) as usize);
                let dst = ((ligne * gl + col) * 4) as usize;
                pixels[dst] = *s.add(2);
                pixels[dst + 1] = *s.add(1);
                pixels[dst + 2] = *s;
                pixels[dst + 3] = 255;
            }
        }
        self.contexte.Unmap(&copie, 0);
        Some(Region {
            pixels,
            largeur: gl,
            hauteur: gh,
        })
    }
}

impl Capteur {
    /// Le changement touche-t-il notre region ?
    ///
    /// En cas de doute la reponse est OUI : un faux positif coute une copie,
    /// un faux negatif laisse un verre perime a l'ecran. Les deux erreurs
    /// n'ont pas le meme prix.
    unsafe fn touche(
        &self,
        x: i32,
        y: i32,
        largeur: u32,
        hauteur: u32,
        info: &DXGI_OUTDUPL_FRAME_INFO,
    ) -> bool {
        if info.TotalMetadataBufferSize == 0 {
            return true;
        }
        let notre = RECT {
            left: x - self.origine.0,
            top: y - self.origine.1,
            right: x - self.origine.0 + largeur as i32,
            bottom: y - self.origine.1 + hauteur as i32,
        };
        let croise = |r: &RECT| {
            r.left < notre.right
                && r.right > notre.left
                && r.top < notre.bottom
                && r.bottom > notre.top
        };

        let mut tampon = vec![0u8; info.TotalMetadataBufferSize as usize];
        let mut taille = 0u32;

        // Les deplacements d'abord : ils portent deux rectangles chacun, la
        // source et la destination, et les deux comptent.
        if self
            .duplication
            .GetFrameMoveRects(
                tampon.len() as u32,
                tampon.as_mut_ptr() as *mut DXGI_OUTDUPL_MOVE_RECT,
                &mut taille,
            )
            .is_err()
        {
            return true;
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
                return true;
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
            return true;
        }
        let n2 = taille2 as usize / std::mem::size_of::<RECT>();
        let sales = std::slice::from_raw_parts(tampon.as_ptr().add(reste) as *const RECT, n2);
        sales.iter().any(croise)
    }
}

impl Capteur {
    /// Une copie SANS attendre d'image neuve, pour le cas ou la region suivie
    /// a change : la fenetre s'est deplacee, et aucun rectangle sale ne peut
    /// dire ce qui se trouve sous sa NOUVELLE position.
    ///
    /// Elle prend la derniere trame presentee, qui est celle affichee, et la
    /// recopie a la position demandee.
    pub fn region_forcee(&self, x: i32, y: i32, largeur: u32, hauteur: u32) -> Option<Region> {
        unsafe {
            let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut ressource: Option<IDXGIResource> = None;
            // Zero d'attente : s'il n'y a rien de neuf, on garde la precedente
            // et le verre se rattrapera a la trame suivante. Mieux vaut un
            // retard d'une trame qu'un a-coup dans le deplacement.
            if self
                .duplication
                .AcquireNextFrame(0, &mut info, &mut ressource)
                .is_err()
            {
                return None;
            }
            let sortie = match &ressource {
                Some(r) => match r.cast::<ID3D11Texture2D>() {
                    Ok(image) => self.copier(&image, x, y, largeur, hauteur),
                    Err(_) => None,
                },
                None => None,
            };
            let _ = self.duplication.ReleaseFrame();
            sortie
        }
    }
}
