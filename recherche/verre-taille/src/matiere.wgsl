// La matiere : le verre taille de `verre.wgsl`, plus ce qui fait qu'un oeil y
// lit du VERRE et non une tache floue — un biseau qui refracte, une dispersion,
// une clarte au bord, et un lisere qui est du Fresnel et non un produit
// scalaire avec une lampe imaginaire.
//
// TOUT ce qui suit est parametre de facon a pouvoir etre mis a ZERO. Avec les
// quatre parametres du biseau a zero, ce nuanceur doit rendre EXACTEMENT
// `verre.wgsl`. C'est le temoin du banc : sans lui, aucun chiffre mesure ici ne
// prouverait quoi que ce soit.

struct Reglages {
    // xy = taille de la cible en pixels, zw = taille de la source
    taille: vec4<f32>,
    // xy = direction du flou (1,0) ou (0,1), z = sigma, w = rayon en taps
    flou: vec4<f32>,
    // xy = demi-taille du galet, z = rayon de coin, w = exposant n
    forme: vec4<f32>,
    // rgb = teinte, a = sa force
    teinte: vec4<f32>,
    // x = saturation, y = marge, z = luminosite, w = teinte sur les octets
    reglages: vec4<f32>,
    // x = largeur du biseau en px, y = indice de refraction, z = dispersion
    // (ecart d'indice entre le rouge et le bleu), w = epaisseur de la dalle px
    biseau: vec4<f32>,
    // x = force du lisere (0 = aucun), y = galbe (hauteur du biseau / largeur),
    // z = clarte au bord (part de l'image NETTE ramenee a l'arete), w = inutilise
    bord: vec4<f32>,
}

@group(0) @binding(0) var<uniform> u: Reglages;
@group(0) @binding(1) var ech: sampler;
@group(0) @binding(2) var source: texture_2d<f32>;
@group(0) @binding(3) var net: texture_2d<f32>;

@vertex
fn sommet(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(i) / 2) * 4.0 - 1.0;
    let y = f32(i32(i) & 1) * 4.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

// Une passe de flou gaussien separable, identique a celle du verre taille.
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

// La distance signee a la superellipse ET la direction de son gradient.
//
// La direction est la nouveaute par rapport a `verre.wgsl` : sans elle on sait
// OU est le bord mais pas de quel cote il regarde, et aucune refraction n'est
// possible. Elle sort du meme calcul, donc elle ne coute presque rien.
struct Forme {
    d: f32,
    // Unitaire, vers l'exterieur.
    dir: vec2<f32>,
}

fn galet(p: vec2<f32>, demi: vec2<f32>, rayon: f32, n: f32) -> Forme {
    let s = sign(p);
    let q = abs(p) - demi + vec2<f32>(rayon);
    var d: f32;
    var g: vec2<f32>;
    if (q.x > 0.0 && q.y > 0.0) {
        // Dans le coin. La norme n n'est pas euclidienne : son gradient vaut
        // 2^(1/n - 1/2) vers 45 degres, soit 0,841 pour n = 4. On divise par sa
        // norme, ce qui rend la distance vraie au premier ordre — et la
        // direction, elle, est ce meme gradient normalise.
        let f = pow(pow(q.x, n) + pow(q.y, n), 1.0 / n);
        let brut = vec2<f32>(pow(q.x, n - 1.0), pow(q.y, n - 1.0))
            / max(pow(f, n - 1.0), 1e-6);
        let ln = max(length(brut), 1e-4);
        d = (f - rayon) / ln;
        g = brut / ln;
    } else {
        let m = max(q, vec2<f32>(0.0));
        d = length(m) + min(max(q.x, q.y), 0.0) - rayon;
        if (m.x + m.y > 1e-5) {
            g = normalize(m);
        } else if (q.x > q.y) {
            g = vec2<f32>(1.0, 0.0);
        } else {
            g = vec2<f32>(0.0, 1.0);
        }
    }
    return Forme(d, g * s);
}

// Le profil d'epaisseur du biseau : squircle convexe, h(t) = (1-(1-t)^4)^(1/4),
// t = 0 a l'arete et 1 au bout du biseau. On renvoie sa PENTE.
//
// Pourquoi ce profil et pas un quart de cercle : sa pente vaut zero exactement
// au raccord avec le plat (t = 1), donc le raccord ne se voit pas. Un quart de
// cercle raccorde aussi a pente nulle, mais sa courbure saute — c'est la meme
// raison qui a fait choisir la superellipse pour le coin, un etage plus haut.
//
// La pente est infinie a l'arete, d'ou le plancher sur t : a t = 0,004 elle
// vaut 22, soit 87,4 degres. Au-dela, la refraction ne change plus rien et la
// reflexion vaut deja 1.
fn pente_biseau(t: f32) -> f32 {
    let v = 1.0 - clamp(t, 0.004, 1.0);
    let v4 = v * v * v * v;
    return v * v * v / max(pow(max(1.0 - v4, 1e-6), 0.75), 1e-6);
}

// Le decalage lateral, en pixels, que subit le fond vu a travers le biseau.
//
// Modele, et il est dit en entier : le rayon de l'oeil est perpendiculaire a
// l'ecran ; il rencontre la face avant inclinee de theta ; Snell le devie vers
// la normale ; il traverse l'epaisseur de la dalle et retombe sur le fond,
// decale de epaisseur x tan(theta - theta_t). Une seule refraction, pas de
// sortie par la face arriere. L'approximation est celle que tout le monde
// utilise pour du verre d'interface, et elle se voit surtout aux tres fortes
// pentes ou le vrai verre devierait un peu moins.
fn decalage(pente: f32, indice: f32, epaisseur: f32) -> f32 {
    let theta = atan(pente);
    let sin_t = clamp(sin(theta) / max(indice, 1.0), -1.0, 1.0);
    return epaisseur * tan(theta - asin(sin_t));
}

// Fresnel par Schlick a l'interface air -> verre.
//
// r0 = ((n-1)/(n+1))^2, soit 0,040 pour n = 1,5. Au centre du galet la face est
// de front, l'incidence est nulle, et le verre ne renvoie que 4 % : invisible.
// A l'arete la face est vue par la tranche, l'incidence tend vers 90 degres, et
// le terme tend vers 1 : la tranche est un MIROIR. C'est la seule raison pour
// laquelle le bord d'un verre brille, et elle ne demande aucune lampe.
fn fresnel(cos_i: f32, indice: f32) -> f32 {
    let r0 = pow((indice - 1.0) / (indice + 1.0), 2.0);
    return r0 + (1.0 - r0) * pow(1.0 - clamp(cos_i, 0.0, 1.0), 5.0);
}

@fragment
fn composer(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let marge = u.reglages.y;
    let dans_source = (pos.xy + vec2<f32>(marge)) / u.taille.zw;
    let centre = pos.xy - u.taille.xy * 0.5;

    let f = galet(centre, u.forme.xy, u.forme.z, u.forme.w);
    let masque = 1.0 - smoothstep(-0.5, 0.5, f.d);

    let largeur = u.biseau.x;
    let indice = u.biseau.y;
    let dispersion = u.biseau.z;
    let epaisseur = u.biseau.w;
    let galbe = u.bord.y;

    // t = 0 a l'arete, 1 au bout du biseau. Hors biseau, t = 1 : pente nulle,
    // decalage nul, Fresnel a 4 % — le plat du galet.
    var t = 1.0;
    if (largeur > 0.0) {
        t = clamp(-f.d / largeur, 0.0, 1.0);
    }
    let pente = pente_biseau(t) * galbe;

    // Trois indices : le bleu refracte plus que le rouge. C'est la dispersion,
    // et c'est elle qui met la frange coloree a l'arete d'un vrai verre.
    let d_r = decalage(pente, indice - dispersion * 0.5, epaisseur);
    let d_v = decalage(pente, indice, epaisseur);
    let d_b = decalage(pente, indice + dispersion * 0.5, epaisseur);

    let pas = f.dir / u.taille.zw;
    let sr = textureSampleLevel(source, ech, dans_source + pas * d_r, 0.0).r;
    let sv = textureSampleLevel(source, ech, dans_source + pas * d_v, 0.0).g;
    let sb = textureSampleLevel(source, ech, dans_source + pas * d_b, 0.0).b;
    var derriere = vec3<f32>(sr, sv, sb);

    // La clarte au bord : pres de l'arete le trajet dans la matiere est court,
    // donc le fond y est moins diffuse. On ramene donc une part de l'image
    // NETTE, decroissante en (1-t)^2. Mise a zero, rien ne se passe.
    let clarte = u.bord.z * (1.0 - t) * (1.0 - t);
    if (clarte > 0.0) {
        let nr = textureSampleLevel(net, ech, dans_source + pas * d_r, 0.0).r;
        let nv = textureSampleLevel(net, ech, dans_source + pas * d_v, 0.0).g;
        let nb = textureSampleLevel(net, ech, dans_source + pas * d_b, 0.0).b;
        derriere = mix(derriere, vec3<f32>(nr, nv, nb), clarte);
    }

    let intact = textureSampleLevel(net, ech, dans_source, 0.0).rgb;

    let gris = dot(derriere, vec3<f32>(0.2126, 0.7152, 0.0722));
    let sature = mix(vec3<f32>(gris), derriere, u.reglages.x);
    let eclaire = sature * u.reglages.z;

    // La teinte est une ABSORPTION : elle multiplie, et elle se pose la ou
    // l'oeil juge — sur les octets encodes (VERRE-TAILLE.md §3).
    var verre: vec3<f32>;
    if (u.reglages.w > 0.5) {
        let s = vers_srgb(eclaire);
        let st = vers_srgb(u.teinte.rgb);
        verre = depuis_srgb(mix(s, st, u.teinte.a));
    } else {
        verre = mix(eclaire, u.teinte.rgb, u.teinte.a);
    }

    // Le lisere. Il s'AJOUTE, en lineaire, APRES la teinte : une reflexion ne
    // traverse pas la matiere, donc elle ne subit pas son absorption. Un produit
    // qui multiplierait serait faux, et un degrade peint serait de la
    // decoration.
    //
    // Ce qu'il reflechit : dans une chambre sombre, rien n'eclaire l'avant de
    // l'ilot. La seule lumiere de la scene est l'ecran lui-meme. Le lisere
    // prend donc le fond, echantillonne a une largeur de biseau vers
    // l'INTERIEUR — d'ou vient la lumiere qui se reflechit sur la face avant.
    // Sur une horloge noire il vaut noir ; sous un chiffre blanc il s'allume.
    // C'est sa seule source, et c'est voulu : l'ilot ne peut pas briller dans
    // le noir.
    if (u.bord.x > 0.0 && largeur > 0.0) {
        let r0 = pow((indice - 1.0) / (indice + 1.0), 2.0);
        // Le lisere est une COUCHE D'AUTEUR, et c'est la mesure qui l'impose.
        //
        // Fresnel donne la raison de sa presence : a l'arete la face est vue par
        // la tranche, l'incidence tend vers 90 degres, et elle renvoie tout.
        // Mais Fresnel ne peut pas donner sa LARGEUR. Avec un profil en quart de
        // rond, la face est tangente au regard exactement a l'arete, et la zone
        // ou l'exces depasse la moitie de son maximum mesure 0,017 x largeur,
        // soit 0,14 px pour un biseau de 8. C'est sous le pixel, et l'elargir
        // n'y change rien : la zone s'elargit dans la meme proportion.
        //
        // Echantillonne au centre des pixels, un tel lisere vaut ce que le
        // hasard de la grille lui donne : 46 sur un bord droit, 71 dans un coin,
        // sur une planche ou le verre vaut 45 — 96 % d'ecart. En integrant sur
        // 4 px on descend a 37 %, et passer de 9 a 33 echantillons ne change
        // rien : ce n'est pas un probleme de quadrature, c'est que la chose est
        // sous-pixellique.
        //
        // On separe donc les deux questions. L'AMPLITUDE reste du Fresnel. La
        // LARGEUR est une decision d'auteur, portee par bord.w en pixels, avec
        // un profil lineaire en distance — le meme genre de ruban que
        // DESIGN.md §6 borne deja a 1 px au repos pour l'anticrenelage.
        var exces: f32;
        if (u.bord.w > 0.01) {
            exces = clamp(1.0 + f.d / u.bord.w, 0.0, 1.0);
        } else {
            let cp = inverseSqrt(1.0 + pente * pente);
            exces = max(fresnel(cp, indice) - r0, 0.0) / (1.0 - r0);
        }
        // On RETRANCHE r0, et la raison n'est pas cosmetique. Les 4 % d'une
        // face de front reflechissent ce qui est DEVANT l'ilot, c'est-a-dire la
        // chambre — noire. Seul l'exces au-dessus de r0, celui qui monte quand
        // la face se couche, est alimente par la lumiere venue de DERRIERE,
        // piegee sous la face avant. Nourrir les 4 % du plat avec le fond
        // serait une faute de modele, et elle coute cher : mesuree ici, elle
        // remonte le centre de 45 a 67 et fait tomber le contraste du blanc
        // chaud de 12,00 a 8,62.
        let dedans = dans_source - pas * (largeur * 2.0);
        let lumiere = textureSampleLevel(source, ech, dedans, 0.0).rgb;
        verre = verre + lumiere * exces * u.bord.x;
    }

    return vec4<f32>(mix(intact, verre, masque), 1.0);
}
