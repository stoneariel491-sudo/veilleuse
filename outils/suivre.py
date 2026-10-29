"""Suivre en direct le raisonnement de l'agent qui travaille.

Il n'y a rien a configurer : le suiveur trouve tout seul le chantier le plus
recent, s'attache a l'agent qui ecrit, et bascule de lui-meme quand un autre
prend la main. On le lance dans un onglet de terminal et on le laisse tourner.

    python outils/suivre.py              # l'agent vivant du dernier chantier
    python outils/suivre.py --tout       # avec les commandes et leurs resultats
    python outils/suivre.py --depuis 40  # en repartant 40 entrees en arriere

Ce qu'il montre :
  — la REFLEXION de l'agent, verbatim, c'est le coeur ;
  — ce qu'il DIT, c'est-a-dire ce qui sortira dans son rapport ;
  — ce qu'il FAIT, en une ligne.

Un agent tue et relance apparait comme un nouvel agent : le suiveur le signale
et le suit, sans qu'on ait rien a faire.
"""

import io
import json
import os
import sys
import time

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

# Le terminal de Windows comprend ces sequences depuis longtemps.
GRIS = "\033[90m"
BLANC = "\033[97m"
JAUNE = "\033[93m"
CYAN = "\033[96m"
FIN = "\033[0m"


def dernier_chantier():
    d = dossiers_de_chantier()
    if not d:
        return None
    return max(d.values(), key=os.path.getmtime)


def etiquettes(dossier):
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
            noms[e["agentId"]] = e["label"]
    return noms


def agent_vivant(dossier):
    """Celui dont la transcription a bouge le plus recemment."""
    fichiers = [
        os.path.join(dossier, f)
        for f in os.listdir(dossier)
        if f.startswith("agent-") and f.endswith(".jsonl")
    ]
    if not fichiers:
        return None
    return max(fichiers, key=os.path.getmtime)


def envelopper(texte, largeur, retrait):
    """Un retour a la ligne qui respecte les mots et les paragraphes."""
    sortie = []
    for para in texte.split("\n"):
        if not para.strip():
            sortie.append("")
            continue
        ligne = retrait
        for mot in para.split():
            if len(ligne) + len(mot) + 1 > largeur and ligne.strip():
                sortie.append(ligne)
                ligne = retrait + mot
            else:
                ligne = (ligne + " " + mot) if ligne.strip() else retrait + mot
        sortie.append(ligne)
    return "\n".join(sortie)


def heure(e):
    ts = e.get("timestamp") or ""
    try:
        from datetime import datetime
        return datetime.fromisoformat(ts.replace("Z", "+00:00")).astimezone().strftime("%H:%M:%S")
    except ValueError:
        return "  --  "


def resume_outil(nom, entree):
    if nom in ("Bash", "PowerShell"):
        c = " ".join((entree.get("command") or "").split())
        return c[:150]
    if nom in ("Write", "Edit"):
        return os.path.basename(entree.get("file_path", "?"))
    if nom == "Read":
        return os.path.basename(entree.get("file_path", "?"))
    if nom in ("Grep", "Glob"):
        return entree.get("pattern", "")
    return ""


def afficher(e, tout):
    msg = e.get("message") or {}
    contenu = msg.get("content")
    if not isinstance(contenu, list):
        return
    h = heure(e)
    for b in contenu:
        if not isinstance(b, dict):
            continue
        t = b.get("type")
        if t == "thinking":
            p = (b.get("thinking") or "").strip()
            if p:
                print(f"\n{GRIS}{h}{FIN}  {CYAN}pense{FIN}")
                print(f"{GRIS}{envelopper(p, 96, '    ')}{FIN}")
        elif t == "text" and e.get("type") == "assistant":
            p = (b.get("text") or "").strip()
            if p:
                print(f"\n{GRIS}{h}{FIN}  {BLANC}dit{FIN}")
                print(f"{BLANC}{envelopper(p, 96, '    ')}{FIN}")
        elif t == "tool_use":
            nom = b.get("name") or "?"
            print(f"{GRIS}{h}{FIN}  {JAUNE}{nom}{FIN} {GRIS}{resume_outil(nom, b.get('input') or {})}{FIN}")
        elif t == "tool_result" and tout:
            c = b.get("content")
            if isinstance(c, list):
                c = "\n".join(x.get("text", "") for x in c if isinstance(x, dict))
            c = (str(c or "")).strip()
            if c:
                print(f"{GRIS}{envelopper(c[:600], 96, '      | ')}{FIN}")
    sys.stdout.flush()


def main():
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    tout = "--tout" in sys.argv
    depuis = 0
    if "--depuis" in sys.argv:
        i = sys.argv.index("--depuis")
        if i + 1 < len(sys.argv):
            depuis = int(sys.argv[i + 1])

    dossier = dernier_chantier()
    if not dossier:
        print("aucun chantier en cours")
        return
    print(f"{BLANC}chantier {os.path.basename(dossier)}{FIN}")
    print(f"{GRIS}Ctrl+C pour arreter. Le suiveur change d'agent tout seul.{FIN}")

    suivi = None
    place = 0
    noms = etiquettes(dossier)

    try:
        while True:
            dossier = dernier_chantier()
            f = agent_vivant(dossier)
            if f is None:
                time.sleep(1.0)
                continue

            if f != suivi:
                suivi = f
                noms = etiquettes(dossier)
                aid = os.path.basename(f)[len("agent-"):-len(".jsonl")]
                role = noms.get(aid, "?")
                lignes = io.open(f, encoding="utf-8", errors="replace").readlines()
                # On reprend un peu en arriere pour avoir le contexte immediat,
                # pas depuis le debut : la consigne fait des milliers de lignes.
                place = max(0, len(lignes) - depuis) if depuis else len(lignes)
                print(f"\n{BLANC}{'─' * 100}{FIN}")
                print(f"{BLANC}  {role}{FIN}  {GRIS}{aid[:10]}{FIN}")
                print(f"{BLANC}{'─' * 100}{FIN}")
                for ligne in lignes[place:]:
                    try:
                        afficher(json.loads(ligne), tout)
                    except ValueError:
                        pass
                place = len(lignes)

            lignes = io.open(f, encoding="utf-8", errors="replace").readlines()
            if len(lignes) > place:
                for ligne in lignes[place:]:
                    try:
                        afficher(json.loads(ligne), tout)
                    except ValueError:
                        pass
                place = len(lignes)
            time.sleep(0.7)
    except KeyboardInterrupt:
        print(f"\n{GRIS}arrete{FIN}")


if __name__ == "__main__":
    main()
