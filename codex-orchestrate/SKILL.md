---
name: codex-orchestrate
description: Claude Code only. Use when the user explicitly invokes /codex-orchestrate in Claude Code or asks Claude to orchestrate a large multi-slice plan by delegating implementation to the Codex CLI through agents-bridge. In native Codex, use $slice-runner instead.
---

# Orchestration de plans par slices via Codex

Claude Code only. Dans Codex natif, utiliser `$slice-runner`.

## Principe

Claude est **architecte, QA et committeur** ; Codex (gpt-5.5, effort xhigh) est **l'exécutant**. Claude découpe le plan en slices séquentielles, impose les interfaces, lance un run Codex par slice, vérifie les gates lui-même, commite. Codex ne committe jamais et ne designe jamais : si Claude laisse Codex inventer une API, N slices produiront N styles.

**Règle de contexte (non négociable)** : ne jamais lire l'output complet d'un run Codex ni son diff complet. Lecture = résumé final (`tail` court) + `git diff --stat` + gates. Inspection ciblée (grep, Read partiel) uniquement si un gate échoue ou si la slice comporte un point à risque identifié d'avance dans le prompt.

## Pré-vol

1. Plan validé par l'utilisateur, découpé en slices : petites, séquentielles, chacune avec un gate de sortie explicite (ex. `validate` seul pour le câblage interne ; + e2e/visual pour ce qui touche le DOM).
2. **Slice 0 = baseline** : tous les gates verts avant la première slice. Sinon, stop et rapport.
3. Branche : suivre la consigne utilisateur ; par défaut une branche dédiée si un push sur main déclenche quelque chose. **Jamais de push** — l'utilisateur pousse.
4. Première slice = la plus petite et la plus autonome (smoke test du pipeline : env, conventions, sandbox).
5. `TaskCreate` une tâche par slice ; `TaskUpdate` au fil de l'eau.
6. Insérer une slice prérequise dès qu'un blocage transversal est découvert (ex. outillage de test manquant pour le nouveau pattern) — ne pas la fusionner dans la slice en cours.

## Boucle par slice

1. Rédiger le prompt (template ci-dessous) — interface imposée, sémantique à préserver point par point.
2. Lancer Codex en arrière-plan (mécanique ci-dessous).
3. À la notification : `tail` du résumé + `git diff --stat`. Inspection ciblée seulement sur les points à risque annoncés.
4. Lancer les gates **soi-même** (jamais sur la foi du « vert » annoncé par Codex).
5. Commit (message impératif, conventions du repo), tâche complétée, slice suivante.

## Template de prompt Codex

Toujours inclure, dans cet ordre :
- **Contexte** : stack + « Lis CLAUDE.md d'abord » + fichiers exemplaires à imiter (« suis le style de X »).
- **Objectif** : une phrase, avec « zéro changement de comportement » si refactor.
- **Interface imposée** : signatures exactes (types, noms, valeurs initiales), pas une intention.
- **Points délicats** : chaque subtilité sémantique nommée explicitement, avec le comportement attendu et l'implémentation suggérée.
- **Tests** : fichiers, cas, conventions de nommage ; « ne supprime aucun test existant ».
- **Interdits** : commit git, nouvelles dépendances, fichiers hors périmètre, + interdits du repo.
- **Documentation** : wiki/commentaires selon les conventions du repo, « reste minimal ».
- **Definition of done** : la commande de gate exacte, « corrige jusqu'au vert », « ne lance pas [suites lentes] (je m'en charge) », « résumé final court : fichiers + choix non triviaux ».

## Mécanique d'invocation

```bash
CODEX_MODEL=gpt-5.5 CODEX_REASONING=xhigh CODEX_SANDBOX=workspace-write \
  "$HOME/projects/claude-plugins/agents-bridge/scripts/codex" exec "PROMPT" < /dev/null
```

- `< /dev/null` **obligatoire** : sans lui, `codex exec` peut rester bloqué sur « Reading additional input from stdin... » indéfiniment.
- Dans le prompt entre guillemets doubles, **échapper tout `$`** (`\$state`, `\$bindable`…) : sinon le shell les avale silencieusement et Codex reçoit un prompt mutilé.
- `run_in_background: true`, timeout ≥ 900 s. Vérifier ~20 s après le lancement que l'output progresse (détecte les blocages immédiats).
- Chaque run retourne un session ID : le capturer pour `exec resume <ID>` lors des corrections (moins cher qu'un contexte neuf).

## Protocole d'échec

- Gate rouge → diagnostiquer d'abord : **échec de code** (reprendre la session Codex avec le rapport d'erreur exact) ou **échec d'environnement** (fichiers hors périmètre, config, outillage — le corriger soi-même, c'est le travail de l'orchestrateur, pas de l'exécutant).
- Deux échecs sur la même slice → stop, rapport à l'utilisateur. Ne pas insister.
- Déviation de Codex par rapport à la spec → juger sur pièces : si le contrat est préservé et le design défendable, accepter et le noter ; sinon resume avec correction.

## Pièges connus (vécus)

| Symptôme | Cause | Fix |
|---|---|---|
| Run figé, output = « Reading additional input from stdin... » | stdin pipe resté ouvert | `< /dev/null`, kill + relance |
| Codex reçoit `()` au lieu de `($bindable)` | expansion shell des `$` | échapper `\$` |
| Gate rouge sur des fichiers jamais touchés | pollution externe (skills installés, artefacts) | fix d'hygiène soi-même (ignore files), pas par Codex |
| Codex annonce vert, gate local rouge | environnements différents | toujours re-lancer les gates soi-même |
| Slice N dépend d'un outillage absent | prérequis transversal découvert tard | slice insérée, jamais fusionnée |
