//! L'icone de l'application entre dans l'executable par ici, et nulle part
//! ailleurs.
//!
//! C'est le premier des trois chemins qu'une icone peut prendre dans un
//! programme Rust (`ICONE-FONDATIONS.md` §0.3) : la ressource de l'executable.
//! Elle n'est pas posee par le code mais par l'editeur de liens, depuis le
//! script `assets/veilleuse.rc` compile ici. C'est elle que voient
//! l'explorateur, le bureau, le menu Demarrer, un raccourci, la recherche,
//! « Ouvrir avec » et le gestionnaire des taches — et elle existe programme
//! eteint.
//!
//! Les deux autres chemins, l'icone de fenetre et l'icone grande, sont poses
//! par `src/main.rs` et chargent CETTE MEME ressource par `LoadImageW` : un
//! seul fichier sert les trois, et aucune taille n'est laissee a la mise a
//! l'echelle de Windows.
//!
//! LA CAISSE. `embed-resource` 3.0, la plus utilisee et la plus active des
//! trois (`ICONE-FONDATIONS.md` §1.3, versions relevees sur crates.io le
//! 18 septembre 2026). `winres` est abandonnee depuis cinq ans et n'a pas ete
//! envisagee. `manifest_optional` parce que l'icone est cosmetique : sur une
//! machine sans compilateur de ressources, la compilation doit aboutir sans
//! icone plutot que d'echouer.

fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/veilleuse.rc");
        println!("cargo:rerun-if-changed=assets/veilleuse.ico");
        embed_resource::compile("assets/veilleuse.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("compilation de la ressource d'icone");
    }
}
