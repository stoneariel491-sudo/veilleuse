"""Extrait le raisonnement complet des agents d'un chantier.

Pourquoi cet outil existe. Les documents que rendent les agents sont des
conclusions : ils disent ce qu'il faut retenir, pas le chemin. Or le chemin est
precisement ce qui a de la valeur — les hypotheses ecartees, les mesures qui
ont contredit une attente, le moment ou quelqu'un se corrige. Tout cela vit
dans les transcriptions, et disparait avec le dossier temporaire.

Ce qui est conserve INTEGRALEMENT :
  — les blocs de reflexion, verbatim, sans coupe ;
  — le texte que l'agent adresse a l'exterieur ;
  — les commandes lancees, qui portent souvent le raisonnement a elles seules.

Ce qui est abrege, et pourquoi : le CONTENU des ecritures de fichier. Il existe
deja sur le disque, sous son vrai nom, et le recopier ici ferait un fichier
illisible ou l'on ne retrouverait plus la pensee. On garde le chemin ecrit et
la taille.

Usage :
    python outils/raisonnement.py                  # le dernier chantier
    python outils/raisonnement.py wf_e235df1d-5aa  # un chantier nomme
    python outils/raisonnement.py --tous           # tous les chantiers
    python outils/raisonnement.py --agents juges a60b96 a244ee
        # des agents lances un a un, hors chantier, archives sous le nom donne
    python outils/raisonnement.py --session epreuve-en-session 9ae49623
        # la session d'orchestration elle-meme, sans les rappels de l'environnement
    python outils/raisonnement.py --session l-origine f0d6f55e --depuis=2026-09-11T13:00:00Z
        # la meme, a partir d'un instant, quand la session portait un autre travail avant
"""

import io
import json
import os
import re
import sys
from datetime import datetime, timezone

PROJETS = os.path.join(os.path.expanduser("~"), ".claude", "projects")


def dossiers_de_chantier():
    """Tous les chantiers connus, toutes sessions confondues, par nom.

    Les transcriptions vivent sous le dossier de la SESSION qui a lance le
    chantier. Viser une session en dur, c'est cesser de voir tout chantier
    lance apres un redemarrage : l'erreur a ete faite une fois, et l'outil
    rendait alors un chantier perime sans rien signaler.
    """
    import glob
    trouves = {}
    motif = os.path.join(PROJETS, "*", "*", "subagents", "workflows", "wf_*")
    for chemin in glob.glob(motif):
        if not os.path.isdir(chemin):
            continue
        nom = os.path.basename(chemin)
        if nom not in trouves or os.path.getmtime(chemin) > os.path.getmtime(trouves[nom]):
            trouves[nom] = chemin
    return trouves
SORTIE = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "raisonnement")

# Au-dela, un resultat d'outil est coupe : ce sont des donnees, pas du
# raisonnement, et elles noieraient le reste.
COUPE_RESULTAT = 2000
# Les ecritures de fichier ne gardent que leur entete.
COUPE_ECRITURE = 400
# Ce que l'environnement glisse dans la transcription d'une session : des
# rappels adresses a l'orchestrateur, pas sa pensee.
RAPPELS = re.compile(
    r"<(system-reminder|local-command-caveat|local-command-stdout|command-name|command-message|command-args)>.*?</\1>",
    re.S,
)
# Toute ligne d'attribution : un co-auteur de commit, une mention « generated
# with » suivie de son lien, une adresse noreply. L'auteur n'en porte aucune et
# les refuse partout ; dans une archive de session elles ne survivent que citees,
# et la citation suffit a les porter. Elles sont remplacees. Le motif ne nomme
# aucun outil, pour ne pas faire entrer dans le depot ce qu'il retire.
ATTRIBUTION = re.compile(
    r"Co-Authored-By:[^\n`\"]*"
    r"|\S*\s*Generated with \\?\[[^\]\n]*(?:\\?\]\([^)\n]*\))?"
    r"|<?noreply@[\w.-]+>?",
    re.I,
)


def chantiers():
    d = dossiers_de_chantier()
    return sorted(d, key=lambda n: os.path.getmtime(d[n]))


def etiquettes(dossier):
    """Le nom de role de chaque agent, lu dans le journal du chantier."""
    noms = {}
    j = os.path.join(dossier, "journal.jsonl")
    if not os.path.isfile(j):
        return noms
    for ligne in io.open(j, encoding="utf-8", errors="replace"):
        try:
            e = json.loads(ligne)
        except ValueError:
            continue
        if e.get("agentId") and e.get("label"):
            noms[e["agentId"]] = (e["label"], e.get("phase", ""))
    return noms


def heure(ts):
    if not ts:
        return ""
    try:
        d = datetime.fromisoformat(ts.replace("Z", "+00:00"))
        return d.astimezone().strftime("%H:%M:%S")
    except ValueError:
        return ts[:19]


def blocs(entree):
    """Les morceaux d'une entree, deja mis en forme."""
    msg = entree.get("message") or {}
    contenu = msg.get("content")
    if isinstance(contenu, str):
        return [("texte", contenu)]
    if not isinstance(contenu, list):
        return []
    sortie = []
    for b in contenu:
        if not isinstance(b, dict):
            continue
        genre = b.get("type")
        if genre == "thinking":
            pensee = b.get("thinking") or ""
            if pensee.strip():
                sortie.append(("reflexion", pensee))
        elif genre == "text":
            txt = b.get("text") or ""
            if txt.strip():
                sortie.append(("texte", txt))
        elif genre == "tool_use":
            sortie.append(("outil", (b.get("name") or "?", b.get("input") or {})))
        elif genre == "tool_result":
            c = b.get("content")
            if isinstance(c, list):
                c = "\n".join(
                    x.get("text", "") for x in c if isinstance(x, dict)
                )
            sortie.append(("resultat", str(c or "")))
    return sortie


def rendre_outil(nom, entree):
    """Une commande se garde entiere ; une ecriture garde son entete."""
    if nom in ("Bash", "PowerShell"):
        cmd = entree.get("command", "")
        return "```\n" + cmd.strip() + "\n```"
    if nom in ("Write", "Edit", "NotebookEdit"):
        chemin = entree.get("file_path", "?")
        corps = entree.get("content") or entree.get("new_string") or ""
        n = len(corps)
        tete = corps[:COUPE_ECRITURE]
        suite = f"\n… (+{n - COUPE_ECRITURE} caracteres, le fichier est sur le disque)" if n > COUPE_ECRITURE else ""
        return f"**ecrit** `{chemin}` ({n} caracteres)\n\n```\n{tete}{suite}\n```"
    if nom in ("Read", "Glob", "Grep"):
        bref = {k: v for k, v in entree.items() if k in
                ("file_path", "pattern", "path", "glob", "offset", "limit")}
        return f"**{nom}** `{json.dumps(bref, ensure_ascii=False)}`"
    if nom in ("WebSearch", "WebFetch"):
        return f"**{nom}** `{entree.get('query') or entree.get('url', '')}`"
    bref = json.dumps(entree, ensure_ascii=False)
    if len(bref) > 600:
        bref = bref[:600] + " …"
    return f"**{nom}** `{bref}`"


def agents_seuls():
    """Les agents lances un a un, hors chantier, toutes sessions confondues.

    Ils n'ont pas de journal : leur transcription est posee a cote de celles de
    la session, et leur role est la description ecrite dans leur fichier meta.
    """
    import glob
    trouves = {}
    for chemin in glob.glob(os.path.join(PROJETS, "*", "*", "subagents", "agent-*.jsonl")):
        trouves[os.path.basename(chemin)[len("agent-"):-len(".jsonl")]] = chemin
    return trouves


def role_d_agent(chemin):
    try:
        meta = json.load(io.open(chemin[:-len(".jsonl")] + ".meta.json", encoding="utf-8"))
    except (OSError, ValueError):
        return "inconnu"
    return meta.get("description") or meta.get("agentType") or "inconnu"


def extraire(nom_chantier):
    dossier = dossiers_de_chantier().get(nom_chantier)
    if dossier is None:
        return None
    noms = etiquettes(dossier)
    fichiers = sorted(
        f for f in os.listdir(dossier)
        if f.startswith("agent-") and f.endswith(".jsonl")
    )

    # Un agent relance porte un nouvel identifiant mais la meme etiquette :
    # on les regroupe, en gardant l'ordre chronologique.
    par_role = {}
    for f in fichiers:
        aid = f[len("agent-"):-len(".jsonl")]
        role, phase = noms.get(aid, ("inconnu", ""))
        par_role.setdefault((phase, role), []).append((aid, os.path.join(dossier, f)))
    return ecrire(nom_chantier, f"Raisonnement du chantier `{nom_chantier}`", par_role)


def extraire_agents(nom, prefixes):
    """Des agents lances hors chantier, reunis sous un meme nom d'archive.

    Un identifiant peut etre abrege tant qu'il ne designe qu'un agent.
    """
    connus = agents_seuls()
    par_role = {}
    for prefixe in prefixes:
        elus = [aid for aid in connus if aid.startswith(prefixe)]
        if len(elus) != 1:
            print(f"{prefixe} : {len(elus)} agent(s) correspondant(s), ignore")
            continue
        chemin = connus[elus[0]]
        par_role.setdefault(("", role_d_agent(chemin)), []).append((elus[0], chemin))
    return ecrire(nom, f"Raisonnement de `{nom}`", par_role)


def extraire_session(nom, prefixe, depuis=None):
    """La session qui orchestre, quand elle a fait elle-meme un travail d'agent.

    Sa transcription recoit aussi les rappels de l'environnement et, apres un
    compactage, le resume du contexte perdu : ni l'un ni l'autre n'est la pensee
    de la session, et ils sont retires. La reflexion reste entiere.

    Une session peut avoir porte un autre travail avant celui qu'on archive :
    `depuis`, un horodatage ISO, en ecarte tout ce qui le precede.
    """
    import glob
    elus = glob.glob(os.path.join(PROJETS, "*", prefixe + "*.jsonl"))
    if len(elus) != 1:
        print(f"{prefixe} : {len(elus)} session(s) correspondante(s), ignore")
        return None
    sid = os.path.basename(elus[0])[:-len(".jsonl")]
    par_role = {("", "session d'orchestration"): [(sid, elus[0])]}
    titre = f"Raisonnement de la session `{nom}`" + (f", depuis {depuis}" if depuis else "")
    return ecrire(nom, titre, par_role, nettoyer=True, depuis=depuis)


def ecrire(nom, titre, par_role, nettoyer=False, depuis=None):
    if not par_role:
        return None
    os.makedirs(SORTIE, exist_ok=True)
    lignes = []
    lignes.append(f"# {titre}")
    lignes.append("")
    lignes.append(
        "Extrait des transcriptions, sans coupe sur la reflexion. "
        "Genere par `outils/raisonnement.py`."
    )
    lignes.append("")
    total_reflexions = 0

    for (phase, role) in sorted(par_role):
        chemins = par_role[(phase, role)]
        lignes.append("")
        lignes.append(f"## {role}" + (f" — phase {phase}" if phase else ""))
        if len(chemins) > 1:
            lignes.append("")
            lignes.append(
                f"*{len(chemins)} tentatives : l'agent a ete relance. "
                "Les tentatives interrompues sont conservees, car leur "
                "raisonnement compte autant que celui qui a abouti.*"
            )
        for i, (aid, chemin) in enumerate(chemins, 1):
            if len(chemins) > 1:
                lignes.append("")
                lignes.append(f"### Tentative {i} (`{aid[:10]}`)")
            for ligne in io.open(chemin, encoding="utf-8", errors="replace"):
                try:
                    e = json.loads(ligne)
                except ValueError:
                    continue
                if e.get("type") == "attachment":
                    continue
                if nettoyer and (e.get("isCompactSummary") or e.get("isMeta")
                                 or e.get("type") not in ("user", "assistant")):
                    continue
                if depuis and (e.get("timestamp") or "") < depuis:
                    continue
                h = heure(e.get("timestamp"))
                # Une entree « user » porte l'enonce ou un retour d'outil, pas
                # la parole de l'agent. Les confondre ferait croire qu'il a
                # ecrit sa propre consigne.
                venant = e.get("type")
                for genre, corps in blocs(e):
                    if genre == "texte" and venant == "user":
                        if nettoyer:
                            corps = RAPPELS.sub("", corps).strip()
                            # Une session reprise s'ouvre sur le resume de la
                            # precedente : c'est un contexte, pas une consigne.
                            if not corps or corps.startswith("This session is being continued from a previous conversation"):
                                continue
                        lignes.append("")
                        lignes.append(f"**{h} — consigne**")
                        lignes.append("")
                        lignes.append("> " + corps.strip().replace("\n", "\n> "))
                        continue
                    if genre == "reflexion":
                        total_reflexions += 1
                        lignes.append("")
                        lignes.append(f"**{h} — reflexion**")
                        lignes.append("")
                        lignes.append("> " + corps.strip().replace("\n", "\n> "))
                    elif genre == "texte":
                        lignes.append("")
                        lignes.append(f"**{h} — dit**")
                        lignes.append("")
                        lignes.append(corps.strip())
                    elif genre == "outil":
                        outil, entree = corps
                        lignes.append("")
                        lignes.append(f"**{h} — fait** · {rendre_outil(outil, entree)}")
                    elif genre == "resultat":
                        txt = (RAPPELS.sub("", corps) if nettoyer else corps).strip()
                        if not txt:
                            continue
                        if len(txt) > COUPE_RESULTAT:
                            txt = txt[:COUPE_RESULTAT] + f"\n… (+{len(corps) - COUPE_RESULTAT} caracteres)"
                        lignes.append("")
                        lignes.append("<details><summary>resultat</summary>")
                        lignes.append("")
                        lignes.append("```")
                        lignes.append(txt)
                        lignes.append("```")
                        lignes.append("</details>")

    cible = os.path.join(SORTIE, nom + ".md")
    texte = "\n".join(lignes) + "\n"
    if nettoyer:
        texte = ATTRIBUTION.sub("[ligne d'attribution retiree]", texte)
    io.open(cible, "w", encoding="utf-8", newline="\n").write(texte)
    return cible, total_reflexions, len(par_role)


def dire(nom, r):
    if r is None:
        print(f"{nom} : aucune transcription")
        return
    cible, reflexions, roles = r
    ko = os.path.getsize(cible) / 1024
    print(f"{nom} : {roles} role(s), {reflexions} reflexions, {ko:.0f} Ko -> {cible}")


def main():
    args = [a for a in sys.argv[1:]]
    tous = "--tous" in args
    agents = "--agents" in args
    session = "--session" in args
    depuis = next((a.split("=", 1)[1] for a in args if a.startswith("--depuis=")), None)
    args = [a for a in args if not a.startswith("--")]

    if agents or session:
        if len(args) < 2 or (session and len(args) != 2):
            print(__doc__)
            return
        if session:
            dire(args[0], extraire_session(args[0], args[1], depuis))
        else:
            dire(args[0], extraire_agents(args[0], args[1:]))
        return

    liste = chantiers()
    if not liste:
        print("aucun chantier trouve sous", PROJETS)
        return
    if args:
        liste = [a for a in args]
    elif not tous:
        liste = liste[-1:]

    for nom in liste:
        dire(nom, extraire(nom))


if __name__ == "__main__":
    main()
