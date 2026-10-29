//! Le banc d'optique : ce qu'un bord de verre fait, calcule et non supppose.
//!
//! Ce programme ne dessine rien. Il resout, pour NOTRE geometrie — galet
//! 320 x 148, superellipse n = 4 de rayon 48, fond capture a plat sous la
//! dalle — les quantites que l'opticienne doit trancher avant qu'une seule
//! ligne de nuanceur soit ecrite :
//!
//!   1. la deviation par la loi de Snell, en PIXELS, et l'agrandissement local
//!      qui en resulte ;
//!   2. la reflectance de Fresnel exacte, l'erreur de Schlick, et la moyenne
//!      de Fresnel sur l'empreinte d'un pixel — car c'est cela qu'on affiche ;
//!   3. la part de la reflexion qui est DERIVABLE du bureau capture ;
//!   4. l'ecart chromatique entre le rouge et le bleu, en pixels ;
//!   5. le chemin optique dans le biseau, et ce qu'il fait a la teinte ;
//!   6. la place disponible : jusqu'ou un biseau peut s'etendre sans toucher
//!      l'encre.
//!
//! Chaque bloc s'ouvre par son TEMOIN : un cas dont la reponse est connue
//! d'avance. Sans lui un resultat ne prouve rien.

use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Les constantes de la matiere et de la piece
// ---------------------------------------------------------------------------

/// N-BK7, verre crown ordinaire. Indices aux trois raies de Fraunhofer.
/// Source : catalogue Schott / refractiveindex.info. Ce sont des valeurs de
/// litterature, PAS des mesures de cette machine, et c'est dit comme tel.
const ND: f64 = 1.516_80; // raie d, 587,6 nm — le jaune de reference
const NF: f64 = 1.522_38; // raie F, 486,1 nm — le bleu
const NC: f64 = 1.514_32; // raie C, 656,3 nm — le rouge

/// Le galet deploye de DESIGN.md §6.
const IL: f64 = 320.0;
const IH: f64 = 148.0;
const RAYON: f64 = 48.0;
const EXPOSANT: f64 = 4.0;
/// Marges interieures du contenu deploye : 20 px en haut/bas, 24 px a gauche/droite.
const MARGE_X: f64 = 24.0;
const MARGE_Y: f64 = 20.0;

/// Le fond clair de la table de decision de VERRE-TAILLE.md §4.
const FOND_CLAIR: f64 = 226.0;
/// La teinte retenue : 0,80 de noir pose sur les OCTETS encodes.
const TEINTE: f64 = 0.80;

// ---------------------------------------------------------------------------
// Outils
// ---------------------------------------------------------------------------

fn deg(r: f64) -> f64 { r * 180.0 / PI }
fn rad(d: f64) -> f64 { d * PI / 180.0 }

/// Fresnel exact, non polarise : la moyenne des deux polarisations.
/// n1 = milieu d'incidence, n2 = milieu de transmission.
/// Rend 1,0 en reflexion totale interne.
fn fresnel_exact(cos_i: f64, n1: f64, n2: f64) -> f64 {
    let sin_i = (1.0 - cos_i * cos_i).max(0.0).sqrt();
    let sin_t = n1 * sin_i / n2;
    if sin_t >= 1.0 {
        return 1.0; // reflexion totale interne
    }
    let cos_t = (1.0 - sin_t * sin_t).sqrt();
    let rs = ((n1 * cos_i - n2 * cos_t) / (n1 * cos_i + n2 * cos_t)).powi(2);
    let rp = ((n1 * cos_t - n2 * cos_i) / (n1 * cos_t + n2 * cos_i)).powi(2);
    0.5 * (rs + rp)
}

/// Schlick 1994. R0 + (1 - R0)(1 - cos)^5.
fn fresnel_schlick(cos_i: f64, r0: f64) -> f64 {
    r0 + (1.0 - r0) * (1.0 - cos_i).powi(5)
}

fn r0_de(n1: f64, n2: f64) -> f64 {
    ((n1 - n2) / (n1 + n2)).powi(2)
}

/// Luminance relative WCAG d'un octet sRGB gris.
fn lum_grise(octet: f64) -> f64 {
    let c = octet / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// Luminance relative WCAG d'un triplet d'octets.
fn lum_rvb(r: f64, v: f64, b: f64) -> f64 {
    0.2126 * lum_grise(r) + 0.7152 * lum_grise(v) + 0.0722 * lum_grise(b)
}

fn contraste(l1: f64, l2: f64) -> f64 {
    let (a, b) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (a + 0.05) / (b + 0.05)
}

/// La distance signee au galet, exactement celle de `galet.wgsl` — norme n,
/// divisee par la norme du gradient. Reproduite ici pour que la place
/// disponible soit mesuree sur LA forme, pas sur une approximation.
fn distance_galet(px: f64, py: f64, demi_x: f64, demi_y: f64, rayon: f64, n: f64) -> f64 {
    let qx = px.abs() - demi_x + rayon;
    let qy = py.abs() - demi_y + rayon;
    if qx > 0.0 && qy > 0.0 {
        let sn = qx.powf(n) + qy.powf(n);
        let f = sn.powf(1.0 / n);
        let grad = (qx.powf(n - 1.0).powi(2) + qy.powf(n - 1.0).powi(2)).sqrt()
            / sn.powf((n - 1.0) / n);
        (f - rayon) / grad.max(1e-6)
    } else {
        let mx = qx.max(0.0);
        let my = qy.max(0.0);
        (mx * mx + my * my).sqrt() + qx.max(qy).min(0.0) - rayon
    }
}

// ---------------------------------------------------------------------------
// Le profil du biseau
// ---------------------------------------------------------------------------
//
// Modele : une dalle d'epaisseur `e`, posee A MEME l'ecran — le bureau capture
// EST sa face inferieure. Le coin superieur est adouci par un conge d'arc de
// cercle de rayon `b` ; sous le conge, une face verticale descend jusqu'a
// l'ecran. Vue de dessus en projection orthographique (ce qu'est un survol
// d'ecran), ce profil a une propriete qui decide de tout :
//
//   la normale balaie 0 -> 90 degres, et l'empreinte du biseau a l'ecran vaut
//   EXACTEMENT b pixels, quelle que soit l'epaisseur.
//
// Parametrage par l'angle de la normale phi :
//   position horizontale   s(phi) = -b * (1 - sin phi)   (s = 0 au bord)
//   hauteur au-dessus de l'ecran   z(phi) = e - b + b cos phi
//   angle d'incidence du rayon de l'oeil (vertical)  theta_i = phi

struct PointBiseau {
    phi: f64,      // angle de la normale, radians
    s: f64,        // abscisse a l'ecran, negative vers l'interieur
    z: f64,        // hauteur au-dessus du bureau
    theta_t: f64,  // angle refracte dans le verre
    deviation: f64,// phi - theta_t
    depl: f64,     // deplacement du point vu, en pixels, vers l'INTERIEUR
    chemin: f64,   // longueur du trajet dans le verre
    reflectance: f64,
}

fn point_biseau(phi: f64, b: f64, e: f64, n: f64) -> PointBiseau {
    let sin_t = phi.sin() / n;
    let theta_t = sin_t.asin();
    let deviation = phi - theta_t;
    let z = e - b + b * phi.cos();
    PointBiseau {
        phi,
        s: -b * (1.0 - phi.sin()),
        z,
        theta_t,
        deviation,
        depl: z * deviation.tan(),
        chemin: z / deviation.cos(),
        reflectance: fresnel_exact(phi.cos(), 1.0, n),
    }
}

/// Le phi qui place le point du biseau a l'abscisse s (s dans [-b, 0]).
fn phi_de_s(s: f64, b: f64) -> f64 {
    ((1.0 + s / b).clamp(-1.0, 1.0)).asin()
}

// ---------------------------------------------------------------------------

fn main() {
    temoins();
    bloc1_snell();
    bloc2_fresnel();
    bloc3_reflexion();
    bloc4_dispersion();
    bloc5_absorption();
    bloc6_place();
    bloc7_visibilite();
}

// ---------------------------------------------------------------------------
// 7. Le deplacement se voit-il ? Ce que le flou a deja detruit
// ---------------------------------------------------------------------------

fn bloc7_visibilite() {
    println!("\n== 7. CE QUE LE FLOU A DEJA DETRUIT ==");
    // Un flou gaussien de sigma attenue une sinusoide de periode p d'un facteur
    // exp(-2 pi^2 sigma^2 / p^2). Un decalage de d pixels change alors la valeur
    // d'au plus  A x exp(-2 pi^2 sigma^2 / p^2) x 2 |sin(pi d / p)|.
    // A = 127 est le pire cas possible sur un octet : une sinusoide pleine
    // echelle. Aucun bureau reel ne l'atteint ; c'est une BORNE, pas une mesure.
    let sigma = 18.0f64;
    println!("  Flou en service : sigma = {} px (verre.rs).", sigma);
    let seuil = |part: f64| -> f64 {
        // exp(-2 pi^2 sigma^2 / p^2) = part
        (2.0 * PI * PI * sigma * sigma / (-part.ln())).sqrt()
    };
    println!("  Le flou laisse 50 % d'amplitude a partir de p = {:.1} px,", seuil(0.5));
    println!("                 10 % a partir de p = {:.1} px,", seuil(0.1));
    println!("                  1 % a partir de p = {:.1} px.\n", seuil(0.01));

    println!("  Borne du changement d'octet provoque par un deplacement d,");
    println!("  sur une image deja floutee a sigma = {} (amplitude d'entree 127) :", sigma);
    println!("    p (px)   survie du flou   d = 2 px   d = 6 px   d = 13,7 px");
    for &p in &[20.0f64, 40.0, 60.0, 80.0, 120.0, 200.0, 400.0] {
        let survie = (-2.0 * PI * PI * sigma * sigma / (p * p)).exp();
        let ch = |d: f64| 127.0 * survie * 2.0 * (PI * d / p).sin().abs();
        println!("    {:6.0}      {:8.4}      {:8.2}   {:8.2}   {:10.2}",
            p, survie, ch(2.0), ch(6.0), ch(13.7));
    }
    println!("\n  Et sur la texture NETTE, que le nuanceur a deja sous la main :");
    println!("    survie = 1, donc la meme borne vaut 127 x 2 |sin(pi d / p)| —");
    println!("    {:.0} octets pour d = 2 px et p = 20 px.", 127.0 * 2.0 * (PI * 2.0 / 20.0).sin().abs());
}

// ---------------------------------------------------------------------------
// 0. Les temoins — sans eux, aucun chiffre plus bas ne vaut rien
// ---------------------------------------------------------------------------

fn temoins() {
    println!("== 0. TEMOINS ==");

    // T1 : a incidence normale, Fresnel exact DOIT valoir exactement R0.
    let r0 = r0_de(1.0, ND);
    let exact0 = fresnel_exact(1.0, 1.0, ND);
    println!("T1  Fresnel(0 deg) = {:.9}   R0 = {:.9}   ecart = {:.2e}",
        exact0, r0, (exact0 - r0).abs());

    // T2 : n = 1 (pas de verre du tout) DOIT donner 0 de reflectance a tout
    // angle, et 0 de deviation. On balaie jusqu'a 89,5 deg : a 90 pile,
    // l'interface n1 = n2 est une singularite retirable (0/0) et le code rend
    // 1 par sa garde de reflexion totale. C'est un artefact de codage, pas de
    // physique, et il vaut mieux le dire que le cacher.
    let mut pire_r = 0.0f64;
    let mut pire_d = 0.0f64;
    for i in 0..=895 {
        let a = rad(i as f64 / 10.0);
        pire_r = pire_r.max(fresnel_exact(a.cos(), 1.0, 1.0));
        let p = point_biseau(a, 12.0, 24.0, 1.0);
        pire_d = pire_d.max(p.depl.abs());
    }
    println!("T2  n = 1, de 0 a 89,5 deg : reflectance max = {:.2e}   deplacement max = {:.2e} px",
        pire_r, pire_d);

    // T3 : a l'angle critique vu de l'INTERIEUR, le rayon sort a 90 degres et
    // la reflectance atteint 1. Un demi-degre au-dela, elle reste a 1.
    let tc = (1.0 / ND).asin();
    println!("T3  angle critique verre->air = {:.3} deg   R(tc-0,5) = {:.4}   R(tc+0,5) = {:.4}",
        deg(tc),
        fresnel_exact(rad(deg(tc) - 0.5).cos(), ND, 1.0),
        fresnel_exact(rad(deg(tc) + 0.5).cos(), ND, 1.0));

    // T4 : le calcul de contraste doit retrouver les nombres DEJA PUBLIES dans
    // VERRE-TAILLE.md §4 — encre #F2EFE9 sur un verre a 45, et sur du noir.
    let l_blanc = lum_rvb(242.0, 239.0, 233.0);
    let l_gris = lum_rvb(142.0, 139.0, 134.0);
    let l_eteint = lum_rvb(94.0, 91.0, 87.0);
    println!("T4  #F2EFE9 sur noir = {:.2}  (publie 18,30)", contraste(l_blanc, 0.0));
    println!("    #F2EFE9 sur 45   = {:.2}  (publie 12,00)", contraste(l_blanc, lum_grise(45.0)));
    println!("    #8E8B86 sur 45   = {:.2}  (publie  4,06)", contraste(l_gris, lum_grise(45.0)));
    println!("    #5E5B57 sur 45   = {:.2}  (publie  2,04)", contraste(l_eteint, lum_grise(45.0)));
    println!();
}

// ---------------------------------------------------------------------------
// 1. Le biseau comme lentille
// ---------------------------------------------------------------------------

fn bloc1_snell() {
    println!("== 1. SNELL : le biseau comme lentille ==");

    // Le deplacement maximal, et ou il tombe, pour plusieurs couples
    // (largeur de conge b, epaisseur e).
    println!("  b    e     depl.max   a s=      phi      depl au bord   verdict de l'image");
    for &b in &[4.0f64, 6.0, 8.0, 12.0, 16.0] {
        for &r in &[1.0f64, 2.0, 3.0] {
            let e = b * r;
            let mut max_d = 0.0f64;
            let mut s_max = 0.0f64;
            let mut phi_max = 0.0f64;
            let pas = 0.002;
            let mut s = -b;
            while s <= 0.0 {
                let p = point_biseau(phi_de_s(s, b), b, e, ND);
                if p.depl > max_d { max_d = p.depl; s_max = s; phi_max = deg(p.phi); }
                s += pas;
            }
            let bord = point_biseau(rad(90.0), b, e, ND);
            // L'image se replie si l'abscisse SOURCE cesse d'etre croissante.
            let mut repli = false;
            let mut prec = f64::NEG_INFINITY;
            let mut s = -b;
            while s <= 0.0 {
                let vu = s - point_biseau(phi_de_s(s, b), b, e, ND).depl;
                if vu < prec - 1e-9 { repli = true; }
                prec = vu;
                s += pas;
            }
            println!(" {:4.0} {:5.0}   {:8.3} px  {:7.2}  {:6.2} deg   {:9.3} px   {}",
                b, e, max_d, s_max, phi_max, bord.depl,
                if repli { "REPLIEE (miroir au bord)" } else { "monotone, etiree au bord" });
        }
    }

    // Ce qui decide vraiment : quelle TRANCHE de source tombe dans chaque
    // pixel de sortie. C'est la largeur du filtre qu'il faudrait, et c'est
    // sans rapport avec un pas d'echantillonnage.
    println!("\n  Tranche de source vue par chaque pixel du biseau (conge 12 px) :");
    println!("    pixel   e = 12 px                 e = 24 px");
    println!("             source     largeur        source     largeur");
    for k in 0..6 {
        let s1 = -(k as f64);
        let s0 = -(k as f64 + 1.0);
        let mut ligne = format!("    {:5}", -(k as i32) - 1);
        for &e in &[12.0f64, 24.0] {
            let v0 = s0 - point_biseau(phi_de_s(s0, 12.0), 12.0, e, ND).depl;
            let v1 = s1 - point_biseau(phi_de_s(s1, 12.0), 12.0, e, ND).depl;
            ligne += &format!("   [{:7.2},{:7.2}] {:7.2}", v0.min(v1), v0.max(v1), (v1 - v0).abs());
        }
        println!("{}", ligne);
    }

    // Le profil detaille du cas de travail : b = 12, e = 24.
    println!("\n  Profil detaille, conge 12 px, epaisseur 24 px, n = {:.5} :", ND);
    println!("   s (px)   phi      theta_t   deviation   depl (px)   chemin/e   R exacte");
    for k in 0..=12 {
        let s = -12.0 + k as f64;
        let p = point_biseau(phi_de_s(s, 12.0), 12.0, 24.0, ND);
        println!("  {:6.1}  {:6.2}   {:6.2}    {:6.2}     {:8.3}    {:6.3}    {:7.4}",
            s, deg(p.phi), deg(p.theta_t), deg(p.deviation), p.depl, p.chemin / 24.0, p.reflectance);
    }

    // La borne structurelle : le cone transmis dans le verre est plafonne par
    // l'angle critique, DONC le deplacement l'est aussi.
    let tc = (1.0 / ND).asin();
    println!("\n  Borne : quelle que soit l'incidence, theta_t <= {:.2} deg (angle critique).",
        deg(tc));
    println!("  Donc la deviation ne depasse jamais {:.2} deg, et le deplacement",
        90.0 - deg(tc));
    println!("  jamais {:.4} x la hauteur du point au-dessus du bureau.", (rad(90.0) - tc).tan());
    println!("  Et un rayon venant de l'AIR ne peut PAS subir de reflexion totale :");
    println!("  elle n'existe qu'en sortant du milieu dense. Le biseau vu de dessus");
    println!("  n'en montre donc jamais. Ce qu'on voit briller est du Fresnel.\n");
}

// ---------------------------------------------------------------------------
// 2. Fresnel
// ---------------------------------------------------------------------------

fn bloc2_fresnel() {
    println!("== 2. FRESNEL ==");
    let r0 = r0_de(1.0, ND);
    println!("  R0 pour n = {:.5} : {:.5}  ({:.2} %)", ND, r0, r0 * 100.0);

    println!("\n  angle   exacte    Schlick   ecart abs   ecart rel");
    let mut pire_abs = (0.0f64, 0.0f64);
    let mut pire_rel = (0.0f64, 0.0f64);
    for i in 0..=900 {
        let a = i as f64 / 10.0;
        let c = rad(a).cos();
        let ex = fresnel_exact(c, 1.0, ND);
        let sc = fresnel_schlick(c, r0);
        let ea = (sc - ex).abs();
        let er = ea / ex;
        if ea > pire_abs.1 { pire_abs = (a, ea); }
        if er > pire_rel.1 { pire_rel = (a, er); }
        if (a * 10.0) as i64 % 100 == 0 || a == 85.0 || a == 87.5 {
            println!("  {:5.1}   {:7.5}   {:7.5}   {:8.5}   {:7.2} %", a, ex, sc, sc - ex, (sc - ex) / ex * 100.0);
        }
    }
    println!("\n  Pire ecart ABSOLU  : {:.5} a {:.1} deg", pire_abs.1, pire_abs.0);
    println!("  Pire ecart RELATIF : {:.2} % a {:.1} deg", pire_rel.1 * 100.0, pire_rel.0);

    // Ce qui compte n'est pas Fresnel a un angle, mais Fresnel MOYENNE sur
    // l'empreinte d'un pixel : c'est cela qu'on affiche.
    println!("\n  Fresnel moyennee sur l'empreinte d'un pixel, conge b :");
    println!("   b (px)   pixel du bord   pixel -1   pixel -2   pixel -3   moyenne du biseau");
    for &b in &[4.0f64, 6.0, 8.0, 12.0, 16.0] {
        let moy = |s0: f64, s1: f64| -> f64 {
            if s1 <= -b { return f64::NAN; }
            let a = s0.max(-b);
            let n = 4000;
            let mut acc = 0.0;
            for k in 0..n {
                let s = a + (s1 - a) * (k as f64 + 0.5) / n as f64;
                acc += fresnel_exact(phi_de_s(s, b).cos(), 1.0, ND);
            }
            acc / n as f64
        };
        println!("   {:4.0}      {:8.4}      {:8.4}   {:8.4}   {:8.4}      {:8.4}",
            b, moy(-1.0, 0.0), moy(-2.0, -1.0), moy(-3.0, -2.0), moy(-4.0, -3.0), moy(-b, 0.0));
    }
    println!();
}

// ---------------------------------------------------------------------------
// 3. La reflexion, et ce qui en est derivable du bureau capture
// ---------------------------------------------------------------------------

fn bloc3_reflexion() {
    println!("== 3. REFLEXION ==");
    // Le rayon de l'oeil, vertical, se reflechit en direction (sin 2phi, cos 2phi).
    // Pour phi > 45 degres, cos 2phi < 0 : le rayon REDESCEND vers l'ecran.
    // Il touche alors le bureau a une distance horizontale z * |tan 2phi| vers
    // l'exterieur — et cette image, NOUS L'AVONS.
    println!("  La reflexion speculaire d'un rayon vertical part en direction");
    println!("  (sin 2phi, cos 2phi). Elle redescend vers l'ecran des que phi > 45 deg.");
    println!("  Le point du bureau atteint est a  z(phi) x |tan 2phi|  vers l'exterieur.\n");

    for &b in &[8.0f64, 12.0, 16.0] {
        for &r in &[1.0f64, 2.0] {
            let e = b * r;
            println!("  conge {:.0} px, epaisseur {:.0} px :", b, e);
            println!("    phi     s (px)   R exacte   cible sur le bureau (px vers l'exterieur)");
            for &a in &[45.0f64, 55.0, 65.0, 75.0, 82.0, 86.0, 89.0, 90.0] {
                let p = point_biseau(rad(a), b, e, ND);
                let t2 = rad(2.0 * a);
                let cible = if t2.cos() < -1e-9 { p.z * t2.sin() / (-t2.cos()) } else { f64::INFINITY };
                println!("   {:5.1}   {:7.3}   {:7.4}    {:9.3}", a, p.s, p.reflectance, cible);
            }
            // Largeur en pixels de la bande derivable.
            let s45 = -b * (1.0 - rad(45.0).sin());
            println!("    bande derivable (phi > 45 deg) : de s = {:.3} au bord, soit {:.3} px = {:.1} % du conge",
                s45, -s45, -s45 / b * 100.0);
            // Part de l'energie reflechie qui tombe dans cette bande.
            let integre = |a0: f64, a1: f64| -> f64 {
                let n = 20000;
                let mut acc = 0.0;
                for k in 0..n {
                    let phi = a0 + (a1 - a0) * (k as f64 + 0.5) / n as f64;
                    // mesure a l'ecran : ds = b cos phi dphi
                    acc += fresnel_exact(phi.cos(), 1.0, ND) * b * phi.cos() * (a1 - a0) / n as f64;
                }
                acc
            };
            let bas = integre(0.0, rad(45.0));
            let haut = integre(rad(45.0), rad(90.0));
            println!("    energie reflechie : {:.4} px sous 45 deg (chambre, inconnue),", bas);
            println!("                        {:.4} px au-dessus (bureau, connue) -> {:.1} % derivable\n",
                haut, haut / (bas + haut) * 100.0);
        }
    }

    // La seconde borne, et elle est a nous : le bureau n'est capture que sur
    // MARGE = 64 px autour de l'ilot. Une reflexion qui vise plus loin n'est
    // pas derivable non plus, faute de pixels.
    println!("  Ce que la marge de capture (64 px, verre.rs) permet reellement :");
    println!("   conge  epaisseur   phi mini utilisable   bande utilisable (px)   part du conge");
    for &b in &[8.0f64, 12.0, 16.0] {
        for &r in &[1.0f64, 2.0] {
            let e = b * r;
            // On cherche le plus petit phi > 45 deg dont la cible tombe a moins de 64 px.
            let mut phi_min = 90.0f64;
            let mut a = 45.01;
            while a < 90.0 {
                let p = point_biseau(rad(a), b, e, ND);
                let t2 = rad(2.0 * a);
                let cible = p.z * t2.sin() / (-t2.cos());
                if cible <= 64.0 { phi_min = a; break; }
                a += 0.01;
            }
            let s0 = -b * (1.0 - rad(phi_min).sin());
            println!("   {:5.0}  {:9.0}   {:17.2} deg   {:19.3}   {:9.1} %",
                b, e, phi_min, -s0, -s0 / b * 100.0);
        }
    }

    // Ce que coute l'ignorance de la chambre : la part inconnue est plafonnee
    // par sa reflectance, qui est faible.
    println!("\n  Plafond de l'erreur commise en ignorant la chambre :");
    println!("    sous 45 deg, R ne depasse jamais {:.4} ({:.2} %).",
        fresnel_exact(rad(45.0).cos(), 1.0, ND), fresnel_exact(rad(45.0).cos(), 1.0, ND) * 100.0);
    println!("    Sur un fond clair a {} octets, cela vaut au plus {:.1} octets ajoutes.",
        FOND_CLAIR, fresnel_exact(rad(45.0).cos(), 1.0, ND) * 255.0);
    println!();
}

// ---------------------------------------------------------------------------
// 4. La dispersion
// ---------------------------------------------------------------------------

fn bloc4_dispersion() {
    println!("== 4. DISPERSION ==");
    let abbe = (ND - 1.0) / (NF - NC);
    println!("  N-BK7 : nd = {:.5}, nF = {:.5}, nC = {:.5}", ND, NF, NC);
    println!("  Nombre d'Abbe vd = (nd-1)/(nF-nC) = {:.2}   (catalogue : 64,17)", abbe);
    println!("  Ecart d'indice bleu-rouge : {:.5}, soit {:.3} % de n.\n", NF - NC, (NF - NC) / ND * 100.0);

    println!("  conge 12 px, epaisseur 24 px :");
    println!("    s (px)   depl rouge   depl bleu   ecart (px)   en % du depl");
    let (b, e) = (12.0f64, 24.0);
    let mut pire = (0.0f64, 0.0f64, 0.0f64);
    let mut s = -b;
    while s <= 0.0 {
        let phi = phi_de_s(s, b);
        let dr = point_biseau(phi, b, e, NC).depl;
        let db = point_biseau(phi, b, e, NF).depl;
        let ec = (db - dr).abs();
        if ec > pire.1 { pire = (s, ec, if dr != 0.0 { ec / dr * 100.0 } else { 0.0 }); }
        if (s.fract().abs() < 1e-9) && (s as i64) % 2 == 0 {
            println!("   {:6.1}    {:9.4}   {:9.4}   {:9.5}    {:8.3} %",
                s, dr, db, ec, if dr > 1e-9 { ec / dr * 100.0 } else { 0.0 });
        }
        s += 0.01;
    }
    println!("\n  Ecart chromatique maximal : {:.5} px (a s = {:.2}), soit {:.3} % du deplacement.",
        pire.1, pire.0, pire.2);

    // Quel nombre d'Abbe faudrait-il pour que l'ecart atteigne un pixel ?
    // L'ecart est proportionnel a (nF - nC), donc a 1/vd a nd fixe.
    let facteur = 1.0 / pire.1;
    println!("  Pour atteindre 1 px d'ecart il faudrait multiplier (nF-nC) par {:.1},", facteur);
    println!("  c'est-a-dire un nombre d'Abbe de {:.2}.", abbe / facteur);

    // Le contre-essai : le verre le PLUS dispersif du catalogue Schott,
    // N-SF66 (nd = 1,92286, vd = 20,88 ; source : shop.schott.com).
    // Meme lui ne franchit pas le pixel.
    let nd2 = 1.922_86f64;
    let vd2 = 20.88f64;
    let dn2 = (nd2 - 1.0) / vd2;
    let (nf2, nc2) = (nd2 + dn2 / 2.0, nd2 - dn2 / 2.0);
    let mut pire2 = 0.0f64;
    let mut s = -b;
    while s <= 1e-9 {
        let phi = phi_de_s(s.min(0.0), b);
        let ec = (point_biseau(phi, b, e, nf2).depl - point_biseau(phi, b, e, nc2).depl).abs();
        pire2 = pire2.max(ec);
        s += 0.01;
    }
    println!("\n  Contre-essai, N-SF66 (nd = {:.5}, vd = {:.2}, le plus dispersif du", nd2, vd2);
    println!("  catalogue Schott) sur la meme geometrie : ecart maximal {:.4} px.", pire2);
    println!("  Meme le pire verre du monde reste sous le pixel.");
    println!();
}

// ---------------------------------------------------------------------------
// 5. L'absorption
// ---------------------------------------------------------------------------

fn bloc5_absorption() {
    println!("== 5. ABSORPTION (Beer-Lambert) ==");
    // Notre teinte a 0,80 sur les octets vaut une transmission de 0,20 sur le
    // trajet normal, d'epaisseur e. Beer-Lambert etend cela a tout trajet :
    //   T(L) = T0 ^ (L/e)
    let t0: f64 = 1.0 - TEINTE;
    println!("  Teinte retenue {:.2} -> transmission normale T0 = {:.2}", TEINTE, t0);
    println!("  Coefficient equivalent : alpha x e = -ln(T0) = {:.4}\n", -t0.ln());

    println!("  Le chemin est-il plus long ou plus court dans le biseau ? Cela depend");
    println!("  du rapport epaisseur/conge, et il existe un basculement.\n");
    println!("   e/b    chemin/e au bord   chemin/e max   chemin/e min   verdict");
    for &r in &[1.0f64, 1.5, 2.0, 3.0, 4.0, 6.0] {
        let b = 12.0;
        let e = b * r;
        let mut mx = f64::NEG_INFINITY;
        let mut mn = f64::INFINITY;
        for k in 0..=2400 {
            let s = -b + b * k as f64 / 2400.0;
            let p = point_biseau(phi_de_s(s, b), b, e, ND);
            let c = p.chemin / e;
            mx = mx.max(c); mn = mn.min(c);
        }
        let bord = point_biseau(rad(90.0), b, e, ND).chemin / e;
        let verdict = if mx > 1.0001 && mn < 0.9999 { "les deux" }
            else if mx > 1.0001 { "plus long" } else { "plus court" };
        println!("   {:4.1}     {:10.4}       {:8.4}       {:8.4}    {}", r, bord, mx, mn, verdict);
    }

    println!("\n  Ce que cela fait a la teinte, conge 12 px, epaisseur 24 px, fond {} :", FOND_CLAIR);
    println!("    s (px)   chemin/e   T        octet sortant   #F2EFE9   #8E8B86   #5E5B57");
    let l_blanc = lum_rvb(242.0, 239.0, 233.0);
    let l_gris = lum_rvb(142.0, 139.0, 134.0);
    let l_eteint = lum_rvb(94.0, 91.0, 87.0);
    for k in 0..=12 {
        let s = -12.0 + k as f64;
        let p = point_biseau(phi_de_s(s, 12.0), 12.0, 24.0, ND);
        let rap = p.chemin / 24.0;
        let t = t0.powf(rap);
        let octet = FOND_CLAIR * t;
        let lf = lum_grise(octet);
        println!("   {:6.1}   {:7.4}   {:6.4}   {:11.1}   {:7.2}   {:7.2}   {:7.2}",
            s, rap, t, octet, contraste(l_blanc, lf), contraste(l_gris, lf), contraste(l_eteint, lf));
    }
    // Le repere : le verre plat, celui de la table publiee.
    let lf = lum_grise(FOND_CLAIR * t0);
    println!("   plat      1.0000   {:6.4}   {:11.1}   {:7.2}   {:7.2}   {:7.2}",
        t0, FOND_CLAIR * t0, contraste(l_blanc, lf), contraste(l_gris, lf), contraste(l_eteint, lf));
    println!();
}

// ---------------------------------------------------------------------------
// 6. La place disponible
// ---------------------------------------------------------------------------

fn bloc6_place() {
    println!("== 6. LA PLACE : jusqu'ou un biseau peut aller sans toucher l'encre ==");
    let (dx, dy) = (IL / 2.0, IH / 2.0);

    // Le contenu deploye vit dans la boite [MARGE_X, IL-MARGE_X] x [MARGE_Y, IH-MARGE_Y].
    // On cherche le point de cette boite le plus PROCHE du bord du galet.
    let mut mini = f64::INFINITY;
    let mut ou = (0.0f64, 0.0f64);
    let pas = 0.25;
    let mut x = MARGE_X;
    while x <= IL - MARGE_X + 1e-9 {
        let mut y = MARGE_Y;
        while y <= IH - MARGE_Y + 1e-9 {
            // Seul le contour de la boite peut etre le plus proche du bord.
            let sur_le_bord = (x - MARGE_X).abs() < pas || (x - (IL - MARGE_X)).abs() < pas
                || (y - MARGE_Y).abs() < pas || (y - (IH - MARGE_Y)).abs() < pas;
            if sur_le_bord {
                let d = distance_galet(x - dx, y - dy, dx, dy, RAYON, EXPOSANT);
                if d.abs() < mini { mini = d.abs(); ou = (x, y); }
            }
            y += pas;
        }
        x += pas;
    }
    println!("  Boite de contenu : x de {} a {}, y de {} a {}.", MARGE_X, IL - MARGE_X, MARGE_Y, IH - MARGE_Y);
    println!("  Point le plus proche du bord : ({:.2}, {:.2}), a {:.2} px du galet.", ou.0, ou.1, mini);
    println!("  Donc un biseau de conge b <= {:.0} px ne touche aucune encre.", mini.floor());

    // Combien de pixels du galet tombent dans la bande du biseau ?
    println!("\n  Part des pixels de l'ilot occupee par le biseau :");
    println!("   b (px)   pixels du galet   pixels du biseau   part");
    for &b in &[4.0f64, 6.0, 8.0, 12.0, 16.0, 19.0] {
        let mut dedans = 0u32;
        let mut bande = 0u32;
        for iy in 0..IH as i32 {
            for ix in 0..IL as i32 {
                let d = distance_galet(ix as f64 + 0.5 - dx, iy as f64 + 0.5 - dy, dx, dy, RAYON, EXPOSANT);
                if d < 0.0 {
                    dedans += 1;
                    if d > -b { bande += 1; }
                }
            }
        }
        println!("   {:4.0}      {:10}        {:10}       {:5.1} %",
            b, dedans, bande, bande as f64 / dedans as f64 * 100.0);
    }

    // Le cout, rapporte a ce qui est deja mesure : 218 prises de texture par
    // pixel pour les deux flous (sigma = 18, rayon = ceil(3 sigma) = 54).
    // Le repere de cout. On ne peut pas convertir des prises de texture en
    // millisecondes sans mesurer — et ce banc ne mesure rien sur le GPU. Mais
    // on peut dire le RAPPORT, qui est ce qui decide.
    let taps_flou = 2.0 * (2.0 * (3.0f64 * 18.0).ceil() + 1.0);
    // La part des pixels du GALET occupee par un conge de 12 px, relue ci-dessus.
    let part_12 = 10184.0 / 46696.0;
    println!("\n  Repere de cout. Les deux flous font {:.0} prises de texture par pixel", taps_flou);
    println!("  (sigma = 18, rayon = ceil(3 sigma) = 54) : c'est le travail qui pese");
    println!("  les 0,77 ms deja mesurees. Trois prises de plus dans la bande d'un");
    println!("  conge de 12 px, qui couvre {:.1} % des pixels du galet, ajoutent", part_12 * 100.0);
    println!("  3 x {:.3} / {:.0} = {:.2} % de prises de texture.", part_12, taps_flou, 3.0 * part_12 / taps_flou * 100.0);
    println!("  Ce rapport N'EST PAS une mesure de temps : c'est un comptage de prises.");
    println!("  Le budget de 2 ms se verifie sur le GPU, pas ici.");
}
