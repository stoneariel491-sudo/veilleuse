// La matiere de verre, resolue depuis l'optique et non posee au juge.
//
// Ce nuanceur remplace l'esquisse de `matiere.wgsl`, qu'`OPTIQUE.md` §12 juge
// fausse sur trois points : signe du deplacement, epaisseur constante, lisere
// nourri par une lumiere qui ne peut pas atteindre l'oeil. Rien ici n'est
// recopie d'ailleurs ; chaque formule renvoie a son paragraphe.
//
// TOUT se met a zero. Avec `couches` a zero et `biseau.x` a zero, ce nuanceur
// doit rendre EXACTEMENT `verre.wgsl` — c'est le temoin sans lequel aucun
// chiffre mesure ici ne prouverait quoi que ce soit.
//
// Le modele, en une phrase (OPTIQUE.md §1) : une dalle d'epaisseur `e` posee a
// meme l'ecran, dont le coin superieur est adouci par un conge d'arc de cercle
// de rayon `b`. L'oeil regarde de haut, en projection orthographique. La
// normale y balaie 0 -> 90 degres sur exactement `b` pixels d'empreinte.

struct Reglages {
    // xy = taille de la cible en pixels, zw = taille de la source
    taille: vec4<f32>,
    // xy = direction du flou (1,0) ou (0,1), z = sigma, w = rayon en taps
    flou: vec4<f32>,
    // xy = demi-taille du galet, z = rayon de coin, w = exposant n
    forme: vec4<f32>,
    // rgb = teinte, a = sa force. La teinte est NOIRE : c'est ce qui permet de
    // l'ecrire comme une transmission multiplicative (voir `transmission`).
    teinte: vec4<f32>,
    // x = saturation, y = marge, z = luminosite, w = teinte sur les octets
    reglages: vec4<f32>,
    // x = conge `b` en px, y = indice `n`, z = inutilise, w = epaisseur `e` en px
    biseau: vec4<f32>,
    // x = force de la reflexion (1 = la physique), y = profil de clarte
    // (0 = marche au bord du conge, 1 = rampe en (1-t)^2), z = force de la
    // clarte, w = inutilise
    bord: vec4<f32>,
    // x = refraction, y = Fresnel (reflexion ET (1-R) transmis), z =
    // Beer-Lambert sur le chemin reel, w = nombre de prises sur l'empreinte
    couches: vec4<f32>,
    // La lampe d'ecran (DECISION-APRES-EPREUVE.md E2) : x = son intensite L,
    // luminance lineaire, 0 = pas de lampe ; y = 1 si elle est un ciel (poids
    // (1 - n_y) / 2 sur la normale sortante du bord), 0 si elle est uniforme ;
    // zw = inutilises.
    lampe: vec4<f32>,
}

@group(0) @binding(0) var<uniform> u: Reglages;
@group(0) @binding(1) var ech: sampler;
@group(0) @binding(2) var source: texture_2d<f32>;
@group(0) @binding(3) var net: texture_2d<f32>;

// Les poids de luminance de Rec. 709, qui sont ceux de l'espace sRGB lineaire
// dans lequel tout ce nuanceur compose.
const LUMA = vec3<f32>(0.2126, 0.7152, 0.0722);

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
// Reprise telle quelle de l'esquisse : c'est la seule chose qu'elle avait
// entierement juste, et elle est deja eprouvee par le banc du metrologue.
struct Forme {
    d: f32,
    // Unitaire, vers l'EXTERIEUR.
    dir: vec2<f32>,
}

fn galet(p: vec2<f32>, demi: vec2<f32>, rayon: f32, n: f32) -> Forme {
    let s = sign(p);
    let q = abs(p) - demi + vec2<f32>(rayon);
    var d: f32;
    var g: vec2<f32>;
    if (q.x > 0.0 && q.y > 0.0) {
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

// Tout ce que la position dans le conge determine, en un seul calcul.
//
// C'est le coeur, et il tient en huit operations sans une seule trigonometrie
// inverse (OPTIQUE.md §3). Le parametrage vient du modele :
//
//   sin phi = 1 + d/b        phi = 90 degres a l'arete, 0 au bout du conge
//   z(phi)  = e - b + b cos phi       hauteur du point de surface sur l'ecran
//
// Et deux identites font tout le travail : en posant
//   num = sin phi cos tht - cos phi sin tht = sin(phi - tht) = sin delta
//   den = cos phi cos tht + sin phi sin tht = cos(phi - tht) = cos delta
// on obtient la deviation ET le cosinus du chemin d'absorption sans un seul
// appel transcendant. `den` est `cos delta`, que Beer-Lambert reclame.
struct Bord {
    // Deplacement lateral, en px, vers l'INTERIEUR du galet.
    // OPTIQUE.md §12.1 : l'esquisse le prenait vers l'exterieur, et dessinait
    // donc une gorge concave la ou il faut une arete convexe.
    deplacement: f32,
    // Reflectance de Fresnel EXACTE, non polarisee. Pas Schlick : 23 % d'erreur
    // relative a 56 degres, en plein dans la bande qui porte la reflexion
    // derivable (OPTIQUE.md §4).
    reflectance: f32,
    // Distance, vers l'EXTERIEUR, du point d'ecran que la face renvoie a
    // l'oeil. Negative quand le rayon reflechi monte vers la chambre.
    portee: f32,
    // Longueur du trajet dans la matiere, rapportee a l'epaisseur.
    chemin: f32,
}

fn bord_en(d: f32, b: f32, n: f32, e: f32) -> Bord {
    let sinp = clamp(1.0 + d / b, 0.0, 1.0);
    let cosp = sqrt(max(1.0 - sinp * sinp, 0.0));
    // La hauteur du point de surface au-dessus de l'ecran. Elle ne peut pas
    // etre negative, et rien ne l'en empechait : si l'epaisseur descend sous le
    // conge, `e - b + b cos phi` passe sous zero pres de l'arete, et DEUX
    // choses partent ensemble — `chemin` devient negatif, donc
    // `pow(1 - teinte, chemin)` depasse 1, donc `tau` devient NEGATIF et le
    // `mix` extrapole de l'autre cote : le verre eclaircirait le fond au lieu
    // de le teinter ; et `deplacement` change de signe, donc le biseau
    // echantillonne vers l'EXTERIEUR, ce qui est exactement le defaut de
    // l'esquisse qu'OPTIQUE.md §12.1 a corrige.
    //
    // Le reglage retenu est b = e = 12, donc z = 12 cos phi >= 0 : le piege
    // n'est pas atteint aujourd'hui et rien de ce qui est publie n'en depend —
    // le temoin est que les planches sont identiques AU BIT PRES avant et apres
    // ce garde-fou. Mais l'espace de reglage y va deja (la direction B de
    // l'esquisse porte largeur 28 pour epaisseur 26), et le conge qui suit la
    // hauteur du galet fait varier b et e ensemble.
    let z = max(e - b + b * cosp, 0.0);

    let sint = sinp / n;
    let cost = sqrt(max(1.0 - sint * sint, 0.0));

    let num = sinp * cost - cosp * sint;
    let den = max(cosp * cost + sinp * sint, 1e-6);

    // Fresnel exact, air -> verre. A incidence normale il vaut r0 ; a l'arete
    // la face est vue par la tranche et il vaut 1. Aucune lampe n'intervient.
    let rs = (cosp - n * cost) / max(cosp + n * cost, 1e-6);
    let rp = (n * cosp - cost) / max(n * cosp + cost, 1e-6);

    // Le rayon vertical reflechi repart en direction (sin 2phi, cos 2phi). Tant
    // que phi < 45 degres il MONTE vers la chambre, qui est noire. Des que
    // phi > 45 degres il REDESCEND sur l'ecran, et cette image, nous l'avons
    // (OPTIQUE.md §5). Au bord exact la portee tombe a zero : le verre reflechit
    // le pixel de bureau qui le touche. C'est une identite, pas un reglage.
    let c2 = 1.0 - 2.0 * sinp * sinp;
    let s2 = 2.0 * sinp * cosp;
    var portee = -1.0;
    if (c2 < 0.0) {
        portee = z * s2 / (-c2);
    }

    return Bord(z * num / den, 0.5 * (rs * rs + rp * rp), portee, (z / e) / den);
}

@fragment
fn composer(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let marge = u.reglages.y;
    let dans_source = (pos.xy + vec2<f32>(marge)) / u.taille.zw;
    let centre = pos.xy - u.taille.xy * 0.5;

    let f = galet(centre, u.forme.xy, u.forme.z, u.forme.w);
    let masque = 1.0 - smoothstep(-0.5, 0.5, f.d);

    let b = u.biseau.x;
    let n = max(u.biseau.y, 1.0);
    let e = max(u.biseau.w, 1e-3);
    let r0 = pow((n - 1.0) / (n + 1.0), 2.0);
    let pas = f.dir / u.taille.zw;

    // --- Ce que le conge determine, moyenne sur l'empreinte du pixel ---------
    //
    // La montee de Fresnel est SOUS-PIXELLAIRE : phi = 80 degres tombe dans le
    // dernier 1,5 % de la largeur du conge, soit 0,18 px a b = 12. Prise au
    // centre du pixel elle manque tout l'effet, et la valeur affichee depend du
    // hasard de la grille — 13 % d'ecart entre un bord droit et un coin
    // (MATIERE.md §5.2). Ce qu'on affiche n'est donc pas R(theta) mais sa
    // MOYENNE sur l'empreinte (OPTIQUE.md §4), et c'est ce qui rend le lisere
    // derive au lieu d'etre peint.
    var transmis = 1.0 - r0;
    var reflet = vec3<f32>(0.0);
    var chemin = 1.0;

    let dans_conge = b > 0.0 && f.d > -b - 1.0;
    if (dans_conge) {
        let prises = max(i32(u.couches.w), 1);
        var s_t = 0.0;
        var s_chemin = 0.0;
        var s_reflet = vec3<f32>(0.0);
        for (var k = 0; k < prises; k = k + 1) {
            let dd = f.d - 0.5 + (f32(k) + 0.5) / f32(prises);
            let g = bord_en(min(dd, 0.0), b, n, e);
            s_chemin = s_chemin + g.chemin;
            // La transmission porte les DEUX pertes, et c'est le point que ni
            // OPTIQUE.md ni l'esquisse n'ecrivaient : ce qui traverse vaut
            // (1 - R) fois ce que Beer-Lambert laisse passer. A l'arete les deux
            // se compensent exactement — le chemin tombe a zero (T -> 1) mais la
            // reflectance monte a 1 ((1-R) -> 0). Sans ce lien, un conge a
            // e = b rendrait le fond INTACT sur son dernier pixel.
            s_t = s_t + (1.0 - g.reflectance);
            if (g.portee >= 0.0) {
                // La capture ne va que jusqu'a la marge : au-dela, la reflexion
                // n'est pas derivable faute de pixels. On l'ecrete, et ce qui
                // est ecrete vaut au plus R = 0,058 (OPTIQUE.md §5).
                let p = min(g.portee, marge);
                let l = textureSampleLevel(net, ech, dans_source + pas * p, 0.0).rgb;
                s_reflet = s_reflet + l * g.reflectance;
            }
        }
        let inv = 1.0 / f32(prises);
        chemin = s_chemin * inv;
        transmis = s_t * inv;
        reflet = s_reflet * inv;
    }

    // --- La refraction ------------------------------------------------------
    //
    // Le biseau refracte la capture NETTE ; la face transmet la floutee. Ce
    // n'est pas un arrangement : c'est une vitre depolie a chants polis, et sans
    // elle la refraction serait INVISIBLE — le flou a sigma 18 a deja detruit la
    // structure que le biseau aurait du tordre (OPTIQUE.md §8, MATIERE.md §1).
    var clarte = 0.0;
    if (b > 0.0) {
        if (u.bord.y > 0.5) {
            let t = clamp(-f.d / b, 0.0, 1.0);
            clarte = (1.0 - t) * (1.0 - t);
        } else {
            // La marche est a la frontiere du conge, la ou le poli s'arrete
            // vraiment, anticrenelee sur un pixel — la meme convention que le
            // ruban du galet (DESIGN.md §6).
            clarte = smoothstep(-b - 0.5, -b + 0.5, f.d);
        }
    }
    clarte = clarte * u.bord.z;

    // Autant de prises que pour Fresnel, chacune avec SON deplacement.
    //
    // `OPTIQUE.md` §13 laissait le trou ouvert : « trois prises devraient
    // suffire ; personne ne l'a verifie ». Mesure faite — trois prises ne
    // couvrent que 1,6 px de la tranche de source de 3,69 px du pixel du bord,
    // parce que le deplacement varie tres vite sur l'empreinte. On prend donc le
    // meme nombre de prises que la reflectance, et le banc dit ce que ca change.
    var derriere = vec3<f32>(0.0);
    var poids = 0.0;
    let taps = select(1, max(i32(u.couches.w), 1), dans_conge && u.couches.x > 0.0);
    for (var k = 0; k < taps; k = k + 1) {
        var dep = 0.0;
        if (u.couches.x > 0.0 && dans_conge) {
            let dd = f.d - 0.5 + (f32(k) + 0.5) / f32(taps);
            dep = bord_en(min(dd, 0.0), b, n, e).deplacement;
        }
        // Vers l'INTERIEUR : le bord montre ce qui est plus dedans.
        let uv = dans_source - pas * dep;
        var c = textureSampleLevel(source, ech, uv, 0.0).rgb;
        if (clarte > 0.0) {
            let cn = textureSampleLevel(net, ech, uv, 0.0).rgb;
            if (u.bord.w > 1.5) {
                // LA COUCHE NETTE N'APPORTE QUE DE LA LUMINANCE.
                //
                // La raison n'est pas un gout : c'est que cette couche
                // n'apporte RIEN de chromatique. La dispersion a ete mesuree a
                // 0,000 px sur toutes les directions du verrier (instrument 3),
                // donc la refraction ne separe ici aucune couleur — ce que la
                // couche nette ramene est une STRUCTURE SPATIALE, et une
                // structure spatiale est une grandeur de luminance.
                //
                // Le prendre en RVB ramenait donc, avec la structure, la
                // chroma haute frequence que le flou venait de dissoudre : sur
                // un bureau raye colore, saturation 131 dans la bande de bord
                // contre 35 au coeur de la meme image. DESIGN.md §8 l'interdit.
                //
                // On garde donc la chroma de la couche FLOUE et on lui donne la
                // luminance de la couche nette. C'est aussi ce que l'oeil fait
                // : il resout la chroma a une frequence spatiale bien plus
                // basse que la luminance — le fait meme sur lequel repose le
                // sous-echantillonnage de chroma.
                let y0 = dot(c, LUMA);
                let y1 = dot(cn, LUMA);
                let y = mix(y0, y1, clarte);
                // L'apport est un DECALAGE NEUTRE, pas une mise a l'echelle.
                //
                // La mise a l'echelle a ete essayee la premiere, et le banc
                // l'a refusee : elle garde le rapport des canaux, donc la
                // saturation RELATIVE, mais la couche nette du bord est trois
                // fois plus claire que la floue, et un ecart de canal multiplie
                // par trois est une couleur trois fois plus forte a l'oeil. La
                // bande de bord restait a 130 de saturation pour un coeur a 35.
                //
                // Ajouter `y - y0` a chaque canal donne exactement la meme
                // luminance — dot(c + k, LUMA) = y0 + k — et laisse le vecteur
                // de chroma `c - gris` INCHANGE en absolu. C'est la definition
                // meme d'un apport achromatique.
                c = max(c + vec3<f32>(y - y0), vec3<f32>(0.0));
            } else {
                c = mix(c, cn, clarte);
            }
        }
        derriere = derriere + c;
        poids = poids + 1.0;
    }
    derriere = derriere / poids;

    let intact = textureSampleLevel(net, ech, dans_source, 0.0).rgb;

    // --- La vibrance, et ce qu'elle compense ---------------------------------
    //
    // Le rehaussement de saturation de `verre.wgsl` existe pour une raison
    // ecrite dans son propre commentaire : « sinon le verre rend une bouillie
    // grise ». C'est une COMPENSATION DU FLOU — un flou a sigma 18 moyenne les
    // couleurs voisines, donc les tire vers le gris, et le facteur 1,35 rend ce
    // que le flou a pris.
    //
    // La couche NETTE n'a rien perdu. Lui appliquer la meme compensation,
    // c'est rehausser une saturation qui n'a pas ete abaissee : mesure sur
    // bureau raye colore, la bande des cinq premiers pixels du bord passe a une
    // saturation de 131 quand le coeur reste a 35. La matiere remettait au bord
    // exactement la couleur que le verre venait de dissoudre, et
    // `DESIGN.md` §8 l'interdit.
    //
    // La vibrance suit donc la clarte : pleine la ou tout est floute, absente
    // la ou la couche nette domine. Ce n'est pas un reglage de gout, c'est la
    // compensation ramenee a ce qu'elle compense.
    //
    // C'est le PREMIER remede essaye (`u.bord.w` = 1). Le banc l'a juge
    // insuffisant — saturation 131 -> 105 pour un coeur a 35 — parce qu'il
    // s'attaque au rehaussement et non a la chroma rehaussee. Le second remede
    // (`u.bord.w` = 2, plus haut dans ce fichier) coupe la chroma a sa source,
    // et laisse alors la vibrance intacte : elle compense a nouveau exactement
    // ce qu'elle compensait.
    let vibrance = select(
        u.reglages.x,
        mix(u.reglages.x, 1.0, clarte),
        u.bord.w > 0.5 && u.bord.w < 1.5,
    );
    let gris = dot(derriere, LUMA);
    let sature = mix(vec3<f32>(gris), derriere, vibrance);
    let eclaire = sature * u.reglages.z;

    // --- La teinte, et le chemin qui la module ------------------------------
    //
    // La teinte se pose sur les OCTETS ENCODES (VERRE-TAILLE.md §3) : c'est ce
    // qui decide de la lisibilite, 45 contre 109 sur fond 226. Beer-Lambert, lui,
    // dit que la transmission suit le CHEMIN : T(L) = T0^(L/e). La teinte etant
    // NOIRE, mix(s, 0, tau) vaut (1 - tau) s, une transmission multiplicative
    // exacte : elever ce facteur a la puissance L/e garde la decision ratifiee
    // et y ajoute la physique du chemin. L'espace reste celui des octets, et cet
    // ecart-la est assume, herite, et il achete la lisibilite (OPTIQUE.md §7).
    var tau = u.teinte.a;
    if (u.couches.z > 0.0) {
        tau = 1.0 - pow(max(1.0 - u.teinte.a, 1e-6), chemin);
    }

    var verre: vec3<f32>;
    if (u.reglages.w > 0.5) {
        let s = vers_srgb(eclaire);
        let st = vers_srgb(u.teinte.rgb);
        verre = depuis_srgb(mix(s, st, tau));
    } else {
        verre = mix(eclaire, u.teinte.rgb, tau);
    }

    // --- Le partage de l'energie -------------------------------------------
    //
    // En LINEAIRE, parce que c'est la que la lumiere s'ajoute. Ce qui traverse
    // est attenue de (1 - R), ce qui est renvoye vaut R fois ce que la face
    // regarde. Sous 45 degres elle regarde la chambre, qui est noire, et le
    // terme vaut zero — la seule convention de tout ce nuanceur, et elle est
    // bornee : R <= 0,0526 sous 45 degres, soit 13,4 octets meme pour une
    // chambre d'un blanc parfait (OPTIQUE.md §5).
    if (u.couches.y > 0.0) {
        verre = verre * transmis + reflet * u.bord.x;
    }

    // --- La lampe d'ecran (DECISION-APRES-EPREUVE.md E2) ---------------------
    //
    // Une lumiere blanche au-dessus de l'ecran, dont le chant porte le reflet a
    // toute heure : ce n'est pas une source de la piece, c'est ce que fait
    // l'ecran, comme chez Apple. Son intensite L est la luminance que le bord
    // droit lit sur du blanc le jour ; son poids suit la normale sortante du
    // bord (1 au bord haut, 1/2 aux cotes, 0 au bord bas) ; son profil sur le
    // chant est celui de la reflectance de Fresnel moyennee sur l'empreinte,
    // normalisee par celle du pixel du bord et ramenee a zero au pied du conge
    // (l'exces sur r0 : sur la face plate, une lampe a l'horizon ne renvoie
    // rien vers l'oeil, et un profil qui s'arreterait a r0 / R(bord) = 0,15
    // laisserait une marche au pied du conge, celle que l'essai 8.A a chassee).
    // Elle se combine au reflet de l'ecran par le plus grand des deux en
    // luminance, jamais par leur somme : la borne du bord tient alors par
    // construction, chacun des deux termes la tenant seul.
    if (u.lampe.x > 0.0 && dans_conge) {
        // La reflectance moyenne de CE pixel : 1 - transmis, et celle du pixel
        // du bord, dont l'empreinte va de d = -1 a d = 0, avec les memes
        // prises : c'est la reflectance de plus par pixel du conge.
        let prises_l = max(i32(u.couches.w), 1);
        var s_bord = 0.0;
        for (var k = 0; k < prises_l; k = k + 1) {
            let dd = -1.0 + (f32(k) + 0.5) / f32(prises_l);
            s_bord = s_bord + bord_en(min(dd, 0.0), b, n, e).reflectance;
        }
        let r_bord = s_bord / f32(prises_l);
        let r_ici = 1.0 - transmis;
        let profil_lampe = clamp((r_ici - r0) / max(r_bord - r0, 1e-6), 0.0, 1.0);
        var poids = 1.0;
        if (u.lampe.y > 0.5) {
            poids = (1.0 - f.dir.y) * 0.5;
        }
        let lueur = u.lampe.x * poids * profil_lampe;
        if (lueur > dot(verre, LUMA)) {
            verre = vec3<f32>(lueur);
        }
    }

    return vec4<f32>(mix(intact, verre, masque), 1.0);
}
