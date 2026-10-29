// Le galet : un rectangle a coins ronds, noir, dessine par sa distance signee.
//
// Pourquoi une distance et non une geometrie : le rayon change a chaque trame
// pendant le deploiement (18 -> 28 px). Une geometrie devrait etre reconstruite
// a chaque pas ; une distance ne coute qu'un nombre dans un uniforme, et donne
// en prime un bord adouci exact, ce qu'une decoupe GDI ne sait pas faire.
//
// La couleur sortie est (0, 0, 0, a). DWM compose en alpha PREMULTIPLIE, sur
// les octets stockes : l'encre etant noire, le premultiplie s'ecrit tel quel,
// sans aucun calcul (ARCHITECTURE.md §1.4).

struct Uniformes {
    // Taille de la fenetre, en pixels.
    taille: vec2<f32>,
    // Centre du galet dans la fenetre, en pixels.
    centre: vec2<f32>,
    // Demi-largeur et demi-hauteur.
    demi: vec2<f32>,
    rayon: f32,
    // Exposant de la superellipse. 2 redonne exactement l'arc de cercle.
    n: f32,
    // 1 : la chaine d'echange sait heriter de la composition, le nuanceur
    // decoupe lui-meme et adoucit le bord. 0 : c'est une region GDI qui decoupe
    // la fenetre, et le nuanceur n'a plus qu'a peindre du noir opaque.
    alpha_par_pixel: f32,
    // Largeur du ruban de transition du bord, en pixels. 1 au repos.
    adoucissement: f32,
    // L'opacite du fondu d'apparition ou de retrait (DESIGN-ILOT.md §1.5), 1 au
    // repos : elle multiplie la sortie premultipliee entiere, couleur et alpha.
    opacite: f32,
    bourrage: f32,
    // M3 : la vitesse d'un bord lateral et celle du bord bas, en pixels par
    // seconde ; l'etalon ; et le ruban le plus large moins un (3 x echelle - 1).
    vitesse: vec2<f32>,
    v_pointe: f32,
    ruban_max: f32,
    // Le theme de contraste du systeme (DESIGN-ILOT.md §8.1) : le fond du galet
    // (Window) et son drapeau, la bordure (WindowText) et sa largeur en pixels,
    // la surbrillance (Highlight) et son drapeau, la cible de l'action survolee.
    fond: vec4<f32>,
    bord: vec4<f32>,
    surbrillance: vec4<f32>,
    zone: vec4<f32>,
};

// Sous le theme de contraste, l'encre du galet a ce pixel : le fond Window, la
// bordure WindowText sur `bord.w` pixels en dedans du bord, et la surbrillance
// Highlight dans la cible de l'action survolee. Jamais un gris de la charte :
// les paires de couleurs du systeme, comme Microsoft l'ecrit [HC].
fn encre_du_contraste(pos: vec2<f32>, d: f32) -> vec3<f32> {
    if (d > -u.bord.w) {
        return u.bord.rgb;
    }
    if (u.surbrillance.w > 0.5
        && pos.x >= u.zone.x && pos.x < u.zone.x + u.zone.z
        && pos.y >= u.zone.y && pos.y < u.zone.y + u.zone.w) {
        return u.surbrillance.rgb;
    }
    return u.fond.rgb;
}

// M3 (DECISION-APRES-EPREUVE.md, la formule du designer approuvee par le juge
// en physicien) : le ruban de transition d'un bord suit la vitesse NORMALE de
// ce bord, `1 + (3 x echelle - 1) x min(1, |v.n| / v_pointe)` pixels. Le bord
// haut ne bouge pas et garde son pixel d'anticrenelage ; les cotes vont chacun
// a la moitie de la vitesse de la largeur ; un point du bord bas va a la
// vitesse de la hauteur, et un point a mi-hauteur a la moitie, puisque le galet
// grandit depuis son bord haut. `p` est relatif au centre, `dir` la normale.
fn ruban(p: vec2<f32>, dir: vec2<f32>) -> f32 {
    let f = clamp((p.y + u.demi.y) / max(2.0 * u.demi.y, 1e-3), 0.0, 1.0);
    let v = vec2<f32>(sign(p.x) * u.vitesse.x, f * u.vitesse.y);
    return 1.0 + u.ruban_max * min(1.0, abs(dot(v, dir)) / max(u.v_pointe, 1e-3));
}

@group(0) @binding(0) var<uniform> u: Uniformes;

@vertex
fn vs(@builtin(vertex_index) iv: u32) -> @builtin(position) vec4<f32> {
    // Deux triangles qui couvrent la fenetre entiere, sans tampon de sommets.
    var coins = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
    );
    return vec4<f32>(coins[iv], 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    // Distance signee au galet. Elle est exprimee en PIXELS, ce qui permet de
    // prendre la couverture du bord directement, sans derivee ni
    // echantillonnage multiple.
    //
    // Le coin n'est PAS un arc de cercle mais une SUPERELLIPSE : la norme
    // euclidienne y devient la norme n. La raison tient en un nombre — la
    // courbure le long du quart de coin, t = 0 a la jonction avec le bord droit
    // et t = 1 a la pointe. Les valeurs sont donnees pour r = 28 ; la courbure
    // se lit en 1/r, donc le rapport entre les lignes ne depend pas du rayon :
    //
    //     n     t=0.00   t=0.05   t=0.20   t=1.00
    //    2.0    0.0357   0.0357   0.0357   0.0357
    //    3.0    0.0000   0.0028   0.0114   0.0636
    //    4.0    0.0000   0.0002   0.0030   0.0901
    //
    // A n = 2 la courbure vaut 1/r DES le premier point de l'arc : elle saute
    // de zero a 0,0357 exactement la ou le droit rencontre le courbe, et c'est
    // cette cassure de la derivee qui se voit. Pour tout n > 2 elle part de
    // zero et entre progressivement dans le virage.
    let p = pos.xy - u.centre;
    // La meme forme que le verre et le givre (`forme`, plus bas) : la distance
    // signee, vraie au premier ordre dans le coin (la norme n divisee par son
    // gradient, sans quoi le pixel de transition s'epaissirait de 19 % a la
    // pointe pour n = 4), et la normale, dont le ruban de chaque bord a besoin.
    let f = forme(p, u.demi, u.rayon, u.n);
    let d = f.d;

    if (u.alpha_par_pixel < 0.5) {
        // La region GDI a deja decoupe la fenetre : tout ce qui reste visible
        // est l'interieur du galet, et il est noir opaque — ou dans les
        // couleurs du systeme sous le theme de contraste.
        if (u.fond.w > 0.5) {
            return vec4<f32>(encre_du_contraste(pos.xy, d), 1.0);
        }
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    // Un ruban de transition centre sur le bord : d = 0 donne une demi
    // couverture, ce qui est exactement la part du pixel que la forme occupe.
    //
    // Sa largeur vaut UN pixel au repos — c'est l'anticrenelage exact, et rien
    // de plus. Elle s'elargit avec la VITESSE du deploiement : une matiere qui
    // arrive n'a pas encore de bord net, et elle se condense en se posant. Le
    // flou et l'echelle bougent donc ensemble, puisque tous deux sortent du
    // meme ressort ; ce n'est pas un fondu d'opacite, l'encre reste noire
    // opaque du debut a la fin.
    let couverture = clamp(0.5 - d / max(ruban(p, f.dir), 1.0), 0.0, 1.0);
    if (u.fond.w > 0.5) {
        let a = couverture * u.opacite;
        return vec4<f32>(encre_du_contraste(pos.xy, d) * a, a);
    }
    return vec4<f32>(0.0, 0.0, 0.0, couverture * u.opacite);
}

// ============================================================================
// Le verre taille : la direction ARBITREE de `DECISION.md`, portee du banc.
// ============================================================================
//
// Tout ce qui suit est `recherche/verre-taille/src/verrier.wgsl` reduit aux
// branches que l'ARBITREE allume : Snell sur la couche nette, Fresnel exact
// moyenne sur l'empreinte, rampe (1 - t)^2, couche nette en luminance seule,
// teinte sur les octets, ni Beer-Lambert ni dispersion. Les branches que la
// decision eteint (marche, vibrance relative, teinte lineaire, chemin) ne sont
// pas portees : un code qui ne tourne jamais ne se verifie jamais. Chaque
// formule renvoie a son paragraphe d'`OPTIQUE.md` ; les longues justifications
// sont restees au banc, la ou elles ont ete mesurees.
//
// Deux choses changent, et seulement deux, parce que l'application n'est pas le
// banc :
//
// 1. LES COORDONNEES. Au banc, la source mesure exactement l'ilot plus deux
//    marges et commence a la marge. Ici, la region capturee suit le galet
//    courant, elle est rognee au bord de l'ecran, et elle vit dans une texture
//    plus grande qu'elle, reutilisee d'une taille a l'autre. Toute lecture
//    passe donc par `dans`, qui borne le point au centre des pixels extremes de
//    la REGION : c'est ce que `ClampToEdge` fait au banc sur une texture de la
//    taille de la source, filtrage lineaire compris.
// 2. LE BORD. Le banc melange en lineaire le fond intact et le verre, sur un
//    `smoothstep`. Ici, ce qui est hors du galet n'est pas peint : c'est le
//    compositeur qui montre le vrai bureau, en alpha PREMULTIPLIE sur les
//    octets stockes (ARCHITECTURE.md §1.4). La couverture est donc celle du
//    galet noir, ruban de DESIGN.md §6 que la decoupe GDI suit, et le verre
//    sort premultiplie sur les octets. L'ecart au banc se limite a ce ruban ;
//    `INTEGRATION.md` le chiffre.

struct Verre {
    // xy = direction du flou, z = sigma en pixels, w = rayon du noyau en pixels.
    flou: vec4<f32>,
    // xy = position du pixel (0, 0) de la fenetre dans la region capturee,
    // zw = taille de la region capturee.
    source: vec4<f32>,
    // xy = taille des textures, z = portee maximale de la reflexion (la marge),
    // w = nombre de prises sur l'empreinte du pixel.
    textures: vec4<f32>,
    // x = force de la teinte noire posee sur les octets, y = saturation,
    // z = force de la reflexion, w = clarte de la couche nette au biseau.
    matiere: vec4<f32>,
    // x = conge b, y = indice, z = epaisseur e, w = refraction (0 ou 1).
    biseau: vec4<f32>,
    // --- La transformation (GIVRE.md). Le tuyau du verre ne lit rien de ce
    // qui suit : ces champs ne servent qu'au tuyau `givre` et aux passes
    // `flou_givre`.
    // x = croissance g, y = fonte f, zw = origine de la fonte, en pixels de
    // fenetre.
    givre: vec4<f32>,
    // x = portee maximale depuis l'origine (px), y = largeur du front de
    // croissance (en unites d'arrivee), z = pas de lecture des passes de flou du
    // givre (2 : lire un texel sur deux), w = largeur du menisque (px).
    givre2: vec4<f32>,
    // xy = taille de la texture givree (texels), zw = taille de la region qu'elle
    // porte (texels) : la moitie de la region nette, arrondie au-dessus.
    givre3: vec4<f32>,
    // La lampe d'ecran (DECISION-APRES-EPREUVE.md E2) : x = son intensite L en
    // luminance lineaire (0 = pas de lampe), y = 1 si son poids suit la normale
    // sortante du bord, (1 - n_y) / 2 ; zw = inutilises.
    lampe: vec4<f32>,
};

@group(1) @binding(0) var<uniform> v: Verre;
@group(1) @binding(1) var ech: sampler;
@group(1) @binding(2) var floutee: texture_2d<f32>;
@group(1) @binding(3) var nette: texture_2d<f32>;
// Les deux couches de la transformation, liees au seul tuyau `givre` : la
// couche givree (le flou large, a demi-resolution) et la texture des cristaux
// (`givre.rs`), en coordonnees normalisees du galet.
@group(1) @binding(4) var givree: texture_2d<f32>;
@group(1) @binding(5) var cristaux: texture_2d<f32>;

// Rec. 709 : les poids de luminance de l'espace sRGB lineaire.
const LUMA = vec3<f32>(0.2126, 0.7152, 0.0722);

// Un point de la region, en pixels, vers la texture : borne au centre des
// pixels extremes de la region, puis rapporte a la taille de la texture.
fn dans(p: vec2<f32>) -> vec2<f32> {
    return clamp(p, vec2<f32>(0.5), v.source.zw - vec2<f32>(0.5)) / v.textures.xy;
}

// Une passe de flou gaussien separable, en LINEAIRE : les textures sont vues en
// sRGB, l'echantillonnage decode et l'ecriture reencode (VERRE-TAILLE.md §3).
// Chaque passe lie sa propre source a la place de `nette`.
// L'image entiere d'un etat, posee par-dessus la vue a l'opacite `u.opacite`
// (le fondu enchaine de « moins de mouvement », m5) : la texture est la trame
// entiere, prise pixel a pixel ; en melange premultiplie, la vue en dessous
// reste a `1 - opacite x alpha`.
@fragment
fn copie(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    return textureLoad(floutee, vec2<i32>(pos.xy), 0) * u.opacite;
}

@fragment
fn flou(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let sigma = v.flou.z;
    let r = i32(v.flou.w);
    var somme = vec3<f32>(0.0);
    var poids = 0.0;
    for (var i = -r; i <= r; i = i + 1) {
        let d = f32(i);
        let w = exp(-0.5 * d * d / (sigma * sigma));
        somme = somme + textureSampleLevel(nette, ech, dans(pos.xy + v.flou.xy * d), 0.0).rgb * w;
        poids = poids + w;
    }
    return vec4<f32>(somme / poids, 1.0);
}

// La meme passe, pour la couche givree : elle lit sa source avec un pas
// `givre2.z` (deux pour tirer une demi-resolution de la couche floutee, un pour
// la seconde passe qui lit la demi-resolution). Le flou du givre est bien plus
// large que celui du verre, et il est lu par un pixel sur quatre : le faire a
// demi-resolution divise son cout par huit sans rien changer de visible, la
// source etant deja floutee a sigma = 18 (RECETTES.md). Une passe a part, et non
// un drapeau dans `flou`, pour que le tuyau du verre ne change pas d'un octet.
@fragment
fn flou_givre(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let sigma = v.flou.z;
    let r = i32(v.flou.w);
    let pas = v.givre2.z;
    var somme = vec3<f32>(0.0);
    var poids = 0.0;
    for (var i = -r; i <= r; i = i + 1) {
        let d = f32(i);
        let w = exp(-0.5 * d * d / (sigma * sigma));
        somme = somme + textureSampleLevel(nette, ech, dans(pos.xy * pas + v.flou.xy * d * pas), 0.0).rgb * w;
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

// La distance signee a la superellipse et la direction de son gradient, vers
// l'exterieur. Celle du banc, a la lettre.
struct Forme {
    d: f32,
    dir: vec2<f32>,
};

fn forme(p: vec2<f32>, demi: vec2<f32>, rayon: f32, n: f32) -> Forme {
    let s = sign(p);
    let q = abs(p) - demi + vec2<f32>(rayon);
    var d: f32;
    var g: vec2<f32>;
    if (q.x > 0.0 && q.y > 0.0) {
        let f = pow(pow(q.x, n) + pow(q.y, n), 1.0 / n);
        let brut = vec2<f32>(pow(q.x, n - 1.0), pow(q.y, n - 1.0)) / max(pow(f, n - 1.0), 1e-6);
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

// Ce que la position dans le conge determine (OPTIQUE.md §3) : le deplacement
// de Snell vers l'interieur, la reflectance de Fresnel exacte non polarisee, et
// la portee du rayon reflechi qui redescend sur l'ecran (OPTIQUE.md §5).
struct Bord {
    deplacement: f32,
    reflectance: f32,
    portee: f32,
};

fn bord_en(d: f32, b: f32, n: f32, e: f32) -> Bord {
    let sinp = clamp(1.0 + d / b, 0.0, 1.0);
    let cosp = sqrt(max(1.0 - sinp * sinp, 0.0));
    // La hauteur du point de surface ne descend jamais sous l'ecran : le
    // garde-fou et sa raison sont commentes au banc.
    let z = max(e - b + b * cosp, 0.0);
    let sint = sinp / n;
    let cost = sqrt(max(1.0 - sint * sint, 0.0));
    let num = sinp * cost - cosp * sint;
    let den = max(cosp * cost + sinp * sint, 1e-6);
    let rs = (cosp - n * cost) / max(cosp + n * cost, 1e-6);
    let rp = (n * cosp - cost) / max(n * cosp + cost, 1e-6);
    let c2 = 1.0 - 2.0 * sinp * sinp;
    let s2 = 2.0 * sinp * cosp;
    var portee = -1.0;
    if (c2 < 0.0) {
        portee = z * s2 / (-c2);
    }
    return Bord(z * num / den, 0.5 * (rs * rs + rp * rp), portee);
}

// La couleur du verre ARBITREE en un pixel, en lineaire, avant la couverture.
// `px` est le point lu dans la region capturee, `f` la forme au pixel peint :
// les deux sont confondus dans `verre`, et le menisque de la fonte les separe
// de quelques pixels. Le corps de cette fonction est celui de `verre` d'avant la
// transformation, extrait tel quel ; la preuve que rien n'a bouge est
// `GIVRE.md` §5 (le verre d'avant et d'apres, identiques a l'octet).
fn couleur_verre(px: vec2<f32>, f: Forme) -> vec3<f32> {
    let b = v.biseau.x;
    let n = max(v.biseau.y, 1.0);
    let e = max(v.biseau.z, 1e-3);
    let r0 = pow((n - 1.0) / (n + 1.0), 2.0);
    let marge = v.textures.z;
    let prises = max(i32(v.textures.w), 1);

    // --- Fresnel, moyenne sur l'empreinte du pixel (OPTIQUE.md §4) ---------
    var transmis = 1.0 - r0;
    var reflet = vec3<f32>(0.0);
    let dans_conge = b > 0.0 && f.d > -b - 1.0;
    if (dans_conge) {
        var s_t = 0.0;
        var s_reflet = vec3<f32>(0.0);
        for (var k = 0; k < prises; k = k + 1) {
            let dd = f.d - 0.5 + (f32(k) + 0.5) / f32(prises);
            let g = bord_en(min(dd, 0.0), b, n, e);
            s_t = s_t + (1.0 - g.reflectance);
            if (g.portee >= 0.0) {
                let p = min(g.portee, marge);
                let l = textureSampleLevel(nette, ech, dans(px + f.dir * p), 0.0).rgb;
                s_reflet = s_reflet + l * g.reflectance;
            }
        }
        let inv = 1.0 / f32(prises);
        transmis = s_t * inv;
        reflet = s_reflet * inv;
    }

    // --- La clarte : rampe (1 - t)^2 sur le conge ----------------------------
    var clarte = 0.0;
    if (b > 0.0) {
        let t = clamp(-f.d / b, 0.0, 1.0);
        clarte = (1.0 - t) * (1.0 - t);
    }
    clarte = clarte * v.matiere.w;

    // --- La refraction : le biseau refracte la NETTE, la face transmet la
    // floutee (OPTIQUE.md §8), avec autant de prises que Fresnel. -------------
    var derriere = vec3<f32>(0.0);
    let refracte = dans_conge && v.biseau.w > 0.0;
    let taps = select(1, prises, refracte);
    for (var k = 0; k < taps; k = k + 1) {
        var dep = 0.0;
        if (refracte) {
            let dd = f.d - 0.5 + (f32(k) + 0.5) / f32(taps);
            dep = bord_en(min(dd, 0.0), b, n, e).deplacement;
        }
        let q = dans(px - f.dir * dep);
        var c = textureSampleLevel(floutee, ech, q, 0.0).rgb;
        if (clarte > 0.0) {
            // La couche nette n'apporte que de la luminance, en decalage neutre.
            let cn = textureSampleLevel(nette, ech, q, 0.0).rgb;
            let y0 = dot(c, LUMA);
            let y = mix(y0, dot(cn, LUMA), clarte);
            c = max(c + vec3<f32>(y - y0), vec3<f32>(0.0));
        }
        derriere = derriere + c;
    }
    derriere = derriere / f32(taps);

    // --- La vibrance, compensation du flou, puis la teinte sur les octets ----
    let gris = dot(derriere, LUMA);
    let sature = mix(vec3<f32>(gris), derriere, v.matiere.y);
    // La teinte est noire : mix(s, 0, tau) sur les octets encodes.
    var verre = depuis_srgb(vers_srgb(sature) * (1.0 - v.matiere.x));

    // --- Le partage de l'energie, en lineaire --------------------------------
    verre = verre * transmis + reflet * v.matiere.z;

    // --- La lampe d'ecran (DECISION-APRES-EPREUVE.md E2) ---------------------
    //
    // Une lumiere blanche au-dessus de l'ecran, dont le chant porte le reflet a
    // toute heure : ce n'est pas une source de la piece, c'est ce que fait
    // l'ecran, comme chez Apple. Son intensite L est le niveau que le bord droit
    // lit sur du blanc le jour, au plus bas des echelles ; son poids suit la
    // normale sortante du bord (1 en haut, 1/2 aux cotes, 0 en bas) ; son
    // profil sur le chant est celui de la reflectance de Fresnel moyennee sur
    // l'empreinte, normalisee par celle du pixel du bord et ramenee a zero au
    // pied du conge (l'exces sur r0 : la face plate ne renvoie rien d'une lampe
    // a l'horizon, et un profil arrete a r0 laisserait une marche au pied du
    // conge). Elle se combine au reflet de l'ecran par le plus grand des deux
    // en luminance, jamais par la somme : chacun tient la borne seul, le
    // maximum la tient donc. Quand elle perd, le pixel est celui d'avant, bit
    // pour bit : c'est le temoin du blanc pur.
    if (v.lampe.x > 0.0 && dans_conge) {
        // La reflectance du pixel du bord, calculee une fois par trame sur le
        // processeur (`galet::lampe::reflectance_du_bord`, la meme formule) :
        // dans le nuanceur, huit `bord_en` de plus par pixel du conge coutaient
        // 11 % d'une passe du verre, mesures (INTEGRATION-APRES-EPREUVE.md).
        let r_bord = v.lampe.z;
        let r_ici = 1.0 - transmis;
        let profil_lampe = clamp((r_ici - r0) / max(r_bord - r0, 1e-6), 0.0, 1.0);
        var poids = 1.0;
        if (v.lampe.y > 0.5) {
            poids = (1.0 - f.dir.y) * 0.5;
        }
        let lueur = v.lampe.x * poids * profil_lampe;
        if (lueur > dot(verre, LUMA)) {
            verre = vec3<f32>(lueur);
        }
    }
    return verre;
}

@fragment
fn verre(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let f = forme(pos.xy - u.centre, u.demi, u.rayon, u.n);

    // La couverture du galet noir, a l'identique : c'est elle que la decoupe
    // GDI suit, et c'est elle qui decide de ce que le compositeur montre.
    var couverture = 1.0;
    if (u.alpha_par_pixel > 0.5) {
        couverture = clamp(0.5 - f.d / max(ruban(pos.xy - u.centre, f.dir), 1.0), 0.0, 1.0);
        if (couverture <= 0.0) {
            return vec4<f32>(0.0);
        }
    }

    // Ce pixel de fenetre, en pixels de la region capturee.
    let px = pos.xy + v.source.xy;
    let verre = couleur_verre(px, f);

    // Premultiplie sur les OCTETS : c'est ce que le compositeur lit.
    return vec4<f32>(depuis_srgb(vers_srgb(verre) * couverture), couverture) * u.opacite;
}

// ============================================================================
// La transformation : galet opaque, givre, puis verre (GIVRE.md).
// ============================================================================
//
// Trois matieres se partagent chaque pixel selon deux fronts lus dans la
// texture des cristaux :
//
// - le NOIR, la ou la croissance `g` n'a pas encore atteint l'arrivee `tau`
//   du texel : le galet opaque, qui ne transmet rien ;
// - le GIVRE, la ou `g` est passe et la fonte `f` non : la lumiere de l'ecran
//   traverse en transmission DIFFUSE — la couche givree (flou large), avec une
//   part rétrodiffusee perdue, modulee par la structure des cristaux qui
//   guident et devient la lumiere, sans chant, sans reflexion, sans vibrance :
//   une surface mate. Le givre ne blanchit pas : la seule lumiere est celle de
//   l'ecran derriere lui (GIVRE.md §1.1) ;
// - le VERRE, la ou la fonte est passee : la couleur ARBITREE, chant compris,
//   lue a travers un menisque qui deplace l'image sur la bande du front. Comme
//   la matiere d'Apple, la transition ne fond pas en opacite : c'est la
//   refraction qui revient.
//
// L'ordre de fonte n'est pas celui de la croissance : il part de l'origine (la
// ou la main a agi) et le mince fond avant l'epais, si bien que les squelettes
// des fougeres restent un instant sur le verre avant de se dissoudre.

// Transmission diffuse du givre : la part de lumiere qui traverse, une fois la
// retrodiffusion vers l'ecran retranchee. GIVRE.md §3 en donne la raison.
const GIVRE_TRANSMISSION: f32 = 0.85;
// Eclat des cristaux qui viennent de pousser, dans la bande du front.
const GIVRE_ECLAT_DU_FRONT: f32 = 0.5;
// Plafond du gain de la structure sur la lumiere transmise, eclat compris.
// Derive de la borne du bord de DESIGN.md §8 (91 niveaux sur le blanc pur) :
// a la teinte 0,80 et a la transmission 0,85, un gain de 3 rend 76 niveaux
// sur le blanc, et 3,57 en rendrait 81 ; 3 garde quinze niveaux de marge.
const GIVRE_GAIN_MAXIMUM: f32 = 3.0;
// Eclat du menisque : l'arete d'un film d'eau concentre la lumiere qui la
// traverse, et la bande du front s'eclaire de cette part.
const FONTE_ECLAT_DU_MENISQUE: f32 = 0.35;
// Largeur du front de fonte, en unites de l'ordre de fonte, et part de
// l'epaisseur dans cet ordre.
const FONTE_LARGEUR: f32 = 0.03;
const FONTE_PART_EPAISSEUR: f32 = 0.25;

// Un point de la region nette, en pixels, vers la texture givree (a
// demi-resolution) : la meme borne que `dans`, dans l'espace de la couche.
fn dans_givree(p: vec2<f32>) -> vec2<f32> {
    return clamp(p / v.givre2.z, vec2<f32>(0.5), v.givre3.zw - vec2<f32>(0.5)) / v.givre3.xy;
}

@fragment
fn givre(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let f = forme(pos.xy - u.centre, u.demi, u.rayon, u.n);
    var couverture = 1.0;
    if (u.alpha_par_pixel > 0.5) {
        couverture = clamp(0.5 - f.d / max(ruban(pos.xy - u.centre, f.dir), 1.0), 0.0, 1.0);
        if (couverture <= 0.0) {
            return vec4<f32>(0.0);
        }
    }
    let px = pos.xy + v.source.xy;

    // Les cristaux, en coordonnees normalisees du galet.
    let uv = (pos.xy - (u.centre - u.demi)) / (2.0 * u.demi);
    let c = textureSampleLevel(cristaux, ech, uv, 0.0);
    let tau = c.r;
    let mu = 4.0 * c.g;
    let rho = c.b;

    // Le front de croissance : `g` balaie l'arrivee de 0 a 1, elargi de la
    // largeur du front pour que le dernier cristal s'allume entierement.
    let g = v.givre.x;
    let w = max(v.givre2.y, 1e-4);
    let gp = g * (1.0 + w);
    let mg = smoothstep(tau - w, tau, gp);

    // Le front de fonte : depuis l'origine, le mince avant l'epais.
    let fo = v.givre.y;
    let r = length(pos.xy - v.givre.zw) / max(v.givre2.x, 1.0);
    let tauf = clamp(mix(r, rho, FONTE_PART_EPAISSEUR), 0.0, 1.0);
    let fp = fo * (1.0 + FONTE_LARGEUR);
    let mf = smoothstep(tauf - FONTE_LARGEUR, tauf, fp);

    let part_givre = mg * (1.0 - mf);
    let part_verre = mg * mf;
    var couleur = vec3<f32>(0.0);

    if (part_givre > 0.0) {
        let lumiere = textureSampleLevel(givree, ech, dans_givree(px), 0.0).rgb;
        // Les cristaux qui viennent de pousser sont les plus minces et les plus
        // diffusants : la bande juste derriere le front s'eclaire, puis se fond
        // dans le voile.
        // `x * x` et non `pow(x, 2.0)` : la base est negative avant le front,
        // et `pow` d'une base negative n'est pas definie en WGSL (m-g7).
        let x_front = (gp - tau - w) / w;
        let eclat = exp(-(x_front * x_front)) * rho;
        let gain = min(mu * (1.0 + GIVRE_ECLAT_DU_FRONT * eclat), GIVRE_GAIN_MAXIMUM);
        let transmise = lumiere * GIVRE_TRANSMISSION * gain;
        // La meme teinte noire que le verre, sur les octets, et la meme
        // transmission d'entree (1 - R0) ; ni vibrance, ni chant, ni reflet.
        let n = max(v.biseau.y, 1.0);
        let r0 = pow((n - 1.0) / (n + 1.0), 2.0);
        let teintee = depuis_srgb(vers_srgb(transmise) * (1.0 - v.matiere.x));
        // L'expression d'avant, telle quelle : quand la lampe perd, l'octet est
        // celui des planches 01 de GIVRE.md, et c'est un temoin.
        var apport = part_givre * teintee * (1.0 - r0);
        // La lumiere propre du givre (DECISION-APRES-EPREUVE.md E2) : la lampe
        // d'ecran que la face du dessus renvoie, non teintee, la part que le
        // givre ne transmet pas (1 - GIVRE_TRANSMISSION) vue sur tout
        // l'hemisphere (0,5), modulee par la structure et l'eclat du front
        // comme la lumiere transmise, plafond compris ; combinee par le plus
        // grand des deux en luminance, jamais par la somme. Sur un ecran
        // allume, la transmise l'emporte ; sur du noir, les fougeres naissent
        // eclairees par la lampe.
        if (v.lampe.x > 0.0) {
            let propre = v.lampe.x * 0.5 * (1.0 - GIVRE_TRANSMISSION) * gain;
            if (propre > dot(teintee * (1.0 - r0), LUMA)) {
                apport = part_givre * vec3<f32>(propre);
            }
        }
        couleur = couleur + apport;
    }
    if (part_verre > 0.0) {
        // Le menisque : sur la bande du front, l'eau tire l'image vers
        // l'origine de quelques pixels. C'est la lentille de la fonte.
        // Meme raison qu'au front de croissance : `x * x`, pas `pow` (m-g7).
        let x_fonte = (fp - tauf - FONTE_LARGEUR * 0.5) / (FONTE_LARGEUR * 0.5);
        let bande = exp(-(x_fonte * x_fonte));
        let vers = pos.xy - v.givre.zw;
        var dir = vec2<f32>(0.0);
        if (length(vers) > 1e-3) {
            dir = normalize(vers);
        }
        // Le menisque s'arrete au conge : sur le chant, le verre de la fonte
        // est exactement le verre au repos, deplacement et eclat compris, pour
        // que la borne du bord de DESIGN.md §8 ne lise jamais plus pendant la
        // transformation qu'au repos. Mesure avant cette garde : 101 a 110
        // niveaux sur les fonds clairs a 330 ms, contre 98 au repos.
        let hors_du_conge = clamp((-f.d - v.biseau.x) / max(v.biseau.x, 1.0), 0.0, 1.0);
        let deplace = px - dir * v.givre2.w * bande * hors_du_conge;
        couleur = couleur + part_verre * couleur_verre(deplace, f) * (1.0 + FONTE_ECLAT_DU_MENISQUE * bande * hors_du_conge);
    }

    return vec4<f32>(depuis_srgb(vers_srgb(couleur) * couverture), couverture) * u.opacite;
}
