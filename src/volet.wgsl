// Le volet : un quad texture, tourne autour du pli par une vraie perspective.
//
// Aucun raccourci en 2D. Le point du quad entre dans un repere dont l'origine
// est SUR le pli, tourne autour de l'axe X, puis est projete par une
// perspective a un point propre a sa carte.

struct Uniformes {
    // Taille de la cible en pixels : le nuanceur travaille en pixels et ne
    // convertit en coordonnees d'horloge qu'au dernier moment.
    taille: vec2<f32>,
    remplissage: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniformes;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var echantillonneur: sampler;

struct Entree {
    // x, y, largeur, hauteur du quad a plat, en pixels d'ecran.
    @location(0) rect: vec4<f32>,
    // u0, v0, u1, v1 dans l'atlas des chiffres.
    @location(1) uv: vec4<f32>,
    // angle (radians), y du pli en pixels, distance de l'oeil, lumiere 0/1.
    @location(2) par: vec4<f32>,
    // Multiplicateur de l'encre, en lineaire. Il vaut (1, 1, 1) en pleine
    // lumiere et descend vers le gris quand le livre se ferme.
    @location(3) teinte: vec4<f32>,
};

struct Sortie {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) lum: f32,
    @location(2) teinte: vec3<f32>,
};

// Plancher de la lumiere, en LINEAIRE. A 0,30 la face rasante descendait a
// #A19E9B, et surtout : a mi-bascule le volet bas qui arrive etait a #C5C3BE
// contre une moitie haute a #F2EFE9. Pendant environ 80 ms le chiffre se lisait
// en deux encres differentes, blanc en haut, gris en bas — l'image exacte contre
// laquelle DESIGN.md §1 met en garde.
//
// 0,65 en lineaire vaut 0,826 en sRGB : la face la plus rasante tombe a
// #C8C5C0 et jamais plus bas. La carte prend encore la lumiere, le mouvement
// reste lisible, et le chiffre ne se lit jamais comme deux blancs differents.
const PLANCHER: f32 = 0.65;

@vertex
fn vs(@builtin(vertex_index) iv: u32, e: Entree) -> Sortie {
    // Deux triangles, six sommets, aucun tampon de sommets : le coin se deduit
    // de l'indice.
    var coins = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    let coin = coins[iv];

    let px = e.rect.x + coin.x * e.rect.z;
    let py = e.rect.y + coin.y * e.rect.w;

    let angle = e.par.x;
    let pli = e.par.y;
    let d = e.par.z;

    // Repere local : l'origine est SUR le pli, c'est l'axe de la charniere.
    let ly = py - pli;
    let c = cos(angle);
    let s = sin(angle);
    let y3 = ly * c;
    let z3 = ly * s;            // positif = vers l'observateur

    // Perspective a un point, propre a CETTE carte et jamais partagee : une
    // perspective commune ferait basculer de biais les cartes des bords.
    // Le point de fuite est le centre horizontal de la carte, a la hauteur du
    // pli. max(..., 1.0) garde-fou : un volet qui passerait le plan de l'oeil
    // diviserait par zero.
    let cx = e.rect.x + e.rect.z * 0.5;
    let profondeur = max(d - z3, 1.0);
    let k = d / profondeur;
    let xp = (px - cx) * k + cx;
    let yp = y3 * k + pli;

    // On NE divise PAS soi-meme. Le sommet part avec w = 1/k et c'est le
    // materiel qui divise : sans cela, les coordonnees de texture seraient
    // interpolees lineairement en coordonnees d'ecran, donc la face du volet
    // serait deformee — la vieille torsion des textures sans correction de
    // perspective, visible comme un pli le long de la diagonale des triangles.
    let w = profondeur / d;
    let xn = (xp / u.taille.x) * 2.0 - 1.0;
    let yn = 1.0 - (yp / u.taille.y) * 2.0;

    var o: Sortie;
    o.pos = vec4<f32>(xn * w, yn * w, 0.0, w);
    o.uv = mix(e.uv.xy, e.uv.zw, coin);
    // La lumiere glisse sur la face pendant la rotation : la carte etant noire
    // sur noir, seule l'encre peut prendre la lumiere.
    o.lum = mix(1.0, PLANCHER + (1.0 - PLANCHER) * abs(c), e.par.w);
    o.teinte = e.teinte.rgb;
    return o;
}

// Nombre maximal d'echantillons verticaux. Un volet a 88 deg ecrase 180 lignes
// de source dans une dizaine de pixels : un echantillon unique y prend une
// ligne sur dix-huit, et l'encre se met a moirer. Seize taquets suffisent a
// eteindre le moire sans couter ailleurs, puisque seuls les fragments a grande
// empreinte les paient.
const TAQUETS_MAX: i32 = 16;

@fragment
fn fs(s: Sortie) -> @location(0) vec4<f32> {
    let hauteur_atlas = f32(textureDimensions(atlas).y);
    // Empreinte verticale du pixel dans l'atlas, en texels. Les derivees se
    // prennent ici, en flux uniforme, jamais dans la boucle.
    let pente = dpdy(s.uv.y);
    let empreinte = abs(pente) * hauteur_atlas;
    let taquets = i32(clamp(ceil(empreinte), 1.0, f32(TAQUETS_MAX)));

    let pas = pente / f32(taquets);
    let depart = s.uv.y - pas * (f32(taquets) - 1.0) * 0.5;
    var somme = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0; i < taquets; i = i + 1) {
        // textureSampleLevel, et non textureSample : l'echantillonnage a lieu
        // dans une boucle, donc hors flux uniforme, ou les derivees implicites
        // sont interdites.
        let uv = vec2<f32>(s.uv.x, depart + pas * f32(i));
        somme = somme + textureSampleLevel(atlas, echantillonneur, uv, 0.0).rgb;
    }
    let encre = somme / f32(taquets);

    // L'atlas est sRGB : l'echantillon arrive en lineaire, la multiplication
    // par la lumiere ET par la teinte s'y fait donc physiquement, et la cible
    // sRGB reencode.
    // Opaque : les volets se cachent les uns les autres, c'est cette occlusion
    // qui fait la mecanique. Pas de melange, pas d'ombre.
    return vec4<f32>(encre * s.lum * s.teinte, 1.0);
}
