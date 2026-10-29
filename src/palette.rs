//! Cinq valeurs de gris, et rien d'autre (DESIGN.md §2).

use glyphon::Color;

/// Blanc legerement chaud. Le blanc froid fatigue a 3 h du matin.
pub const ENCRE: Color = Color::rgb(0xF2, 0xEF, 0xE9);

/// Gris chaud, meme temperature que le blanc : visible sans appeler l'oeil.
pub const SECONDAIRE: Color = Color::rgb(0x8E, 0x8B, 0x86);

/// Presque eteint : une legende se devine, ne se lit pas.
pub const LEGENDE: Color = Color::rgb(0x5E, 0x5B, 0x57);

/// Le gris du verre (DESIGN.md §2 ; DECISION-APRES-EPREUVE.md G, M2) : la
/// teinte de `SECONDAIRE` la plus sombre qui tient 4,5:1 contre le pixel le
/// plus clair que le verre montre sous la phase, la sous-ligne et l'action au
/// repos, sur les sept fonds de travail et le blanc pur (4,52 au pire ;
/// `#9B9893`, un octet plus sombre, ne tient pas). Il ne vaut que sur le verre
/// de l'ilot, et l'encre suit la matiere visee des le clic qui la change.
pub const GRIS_DU_VERRE: Color = Color::rgb(0x9C, 0x99, 0x94);

/// Passe un octet sRGB en lineaire. Sert a calculer la teinte des chiffres :
/// le nuanceur multiplie une valeur deja lineaire, donc le rapport de deux
/// couleurs doit se prendre la aussi, pas sur les octets.
pub fn lineaire(octet: u8) -> f32 {
    let c = octet as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Le facteur a appliquer a l'encre gravee dans l'atlas pour obtenir `cible`.
///
/// L'atlas est grave une fois en `ENCRE` ; l'echantillon vaut donc
/// `ENCRE_lineaire × couverture`. Multiplier par ce rapport donne exactement
/// `cible_lineaire × couverture`, sans regraver un seul glyphe.
pub fn teinte(cible: Color) -> [f32; 4] {
    let [r, v, b] = [
        lineaire(cible.r()) / lineaire(ENCRE.r()),
        lineaire(cible.g()) / lineaire(ENCRE.g()),
        lineaire(cible.b()) / lineaire(ENCRE.b()),
    ];
    [r, v, b, 0.0]
}

/// Melange deux couleurs, `t = 0` donnant la premiere.
///
/// Le melange se fait sur les OCTETS et non en lineaire : DESIGN.md decrit une
/// lampe sur variateur entre deux gris voisins, et c'est la progression
/// perceptuelle — donc celle des octets — qui doit etre reguliere. En lineaire,
/// la moitie du chemin paraitrait deja presque arrivee.
pub fn melange(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::rgba(
        m(a.r(), b.r()),
        m(a.g(), b.g()),
        m(a.b(), b.b()),
        m(a.a(), b.a()),
    )
}
