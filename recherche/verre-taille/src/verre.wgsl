// Le verre taille : on floute nous-memes ce qui est derriere, et on le masque
// par NOTRE forme. Rien ici ne depend du compositeur de Windows.

struct Reglages {
    // xy = taille de la cible en pixels, zw = taille de la source
    taille: vec4<f32>,
    // xy = direction du flou (1,0) ou (0,1), z = sigma, w = rayon en taps
    flou: vec4<f32>,
    // xy = demi-taille du galet, z = rayon de coin, w = exposant n
    forme: vec4<f32>,
    // rgb = teinte, a = sa force
    teinte: vec4<f32>,
    // x = saturation, y = marge, z = luminosite, w = inutilise
    reglages: vec4<f32>,
}

@group(0) @binding(0) var<uniform> u: Reglages;
@group(0) @binding(1) var ech: sampler;
@group(0) @binding(2) var source: texture_2d<f32>;
@group(0) @binding(3) var net: texture_2d<f32>;

// Triangle plein ecran : trois sommets, aucun tampon.
@vertex
fn sommet(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(i) / 2) * 4.0 - 1.0;
    let y = f32(i32(i) & 1) * 4.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

// Une passe de flou gaussien separable. Appelee deux fois, horizontale puis
// verticale. L'echantillonnage se fait sur une vue sRGB, donc la somme a lieu
// en LINEAIRE — c'est physiquement juste, et c'est ce que DWM ne fait pas.
@fragment
fn flou(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let sigma = u.flou.z;
    let r = i32(u.flou.w);
    var somme = vec3<f32>(0.0);
    var poids = 0.0;
    for (var i = -r; i <= r; i = i + 1) {
        let d = f32(i);
        let w = exp(-0.5 * d * d / (sigma * sigma));
        let p = pos.xy + u.flou.xy * d;
        let uv = p / u.taille.zw;
        somme = somme + textureSampleLevel(source, ech, uv, 0.0).rgb * w;
        poids = poids + w;
    }
    return vec4<f32>(somme / poids, 1.0);
}


// Le flou est une moyenne PHYSIQUE : il se fait en lineaire, et la vue sRGB
// s'en charge. La teinte est un geste de PEINTRE : elle se pose la ou l'oeil
// juge, c'est-a-dire sur les octets encodes. Melanger les deux au meme endroit
// est l'erreur qui rend un verre soit delave, soit mort.
fn vers_srgb(c: vec3<f32>) -> vec3<f32> {
    let bas = c * 12.92;
    let haut = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(bas, haut, c > vec3<f32>(0.0031308));
}

fn depuis_srgb(c: vec3<f32>) -> vec3<f32> {
    let bas = c / 12.92;
    let haut = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(bas, haut, c > vec3<f32>(0.04045));
}

// La distance signee a la superellipse, corrigee de son gradient.
//
// La norme n n'est pas euclidienne : son gradient vaut 2^(1/n - 1/2) vers 45
// degres, soit 0,841 pour n = 4. Sans la division, le pixel de transition
// s'epaissit de 19 % a la pointe du coin et le bord y parait flou alors qu'il
// est net ailleurs.
fn distance_galet(p: vec2<f32>, demi: vec2<f32>, rayon: f32, n: f32) -> f32 {
    let q = abs(p) - demi + vec2<f32>(rayon);
    let m = max(q, vec2<f32>(0.0));
    if (m.x <= 0.0 || m.y <= 0.0) {
        // Sur les bords droits, la norme n se confond avec la distance.
        return length(m) + min(max(q.x, q.y), 0.0) - rayon;
    }
    let sn = pow(m.x, n) + pow(m.y, n);
    let norme = pow(sn, 1.0 / n);
    let d = norme - rayon;
    let grad = length(vec2<f32>(pow(m.x, n - 1.0), pow(m.y, n - 1.0)))
        / pow(sn, (n - 1.0) / n);
    return d / max(grad, 1e-4);
}

// La composition : dedans le verre, dehors ce qui etait la, intact.
@fragment
fn composer(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let marge = u.reglages.y;
    // Coordonnee dans la source, qui est plus grande de `marge` de chaque cote.
    let dans_source = (pos.xy + vec2<f32>(marge)) / u.taille.zw;
    let centre = pos.xy - u.taille.xy * 0.5;

    let d = distance_galet(centre, u.forme.xy, u.forme.z, u.forme.w);
    let masque = 1.0 - smoothstep(-0.5, 0.5, d);

    let derriere = textureSampleLevel(source, ech, dans_source, 0.0).rgb;
    let intact = textureSampleLevel(net, ech, dans_source, 0.0).rgb;

    // Vibrance : on rehausse la saturation avant de teinter, sinon le verre
    // rend une bouillie grise. C'est l'ingredient qu'Apple ajoute au flou.
    let gris = dot(derriere, vec3<f32>(0.2126, 0.7152, 0.0722));
    let sature = mix(vec3<f32>(gris), derriere, u.reglages.x);
    let eclaire = sature * u.reglages.z;

    // reglages.w : 0 = teinte en lineaire (physique), 1 = teinte sur les
    // octets encodes (perceptif, ce que fait DWM). A teinte egale, le second
    // est nettement plus sombre — donc plus lisible.
    var verre: vec3<f32>;
    if (u.reglages.w > 0.5) {
        let s = vers_srgb(eclaire);
        let st = vers_srgb(u.teinte.rgb);
        verre = depuis_srgb(mix(s, st, u.teinte.a));
    } else {
        verre = mix(eclaire, u.teinte.rgb, u.teinte.a);
    }

    return vec4<f32>(mix(intact, verre, masque), 1.0);
}
