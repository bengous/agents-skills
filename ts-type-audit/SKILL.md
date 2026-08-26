---
name: ts-type-audit
description: Audit statique en lecture seule de la maturité du typage TypeScript d'un projet. Trois workers parallèles (états impossibles, inférence, discipline), rapports normalisés, synthèse croisée. Ne corrige rien.
disable-model-invocation: true
argument-hint: "[racine du projet] [sous-périmètre]"
---

# ts-type-audit

Trois auditeurs en lecture seule, un par tier, rendent trois rapports de
structure identique. L'orchestrateur les vérifie, les fusionne, les classe. La
normalisation rend les rapports comparables entre tiers et entre passes.

| Tier | Question | Référence |
|---|---|---|
| 1 · États impossibles | Quels états invalides restent écrivables ? | `references/tier1-etats.md` |
| 2 · Précision de l'inférence | Que sait prouver le compilateur ? | `references/tier2-precision.md` |
| 3 · Discipline | La sûreté est-elle effective et vérifiée partout ? | `references/tier3-discipline.md` |

Le template de rapport, les taxonomies et les règles anti-biais sont dans
`references/rapport.md`. Ce fichier est collé dans chaque brief.

## Cadrage

Fixer avant de lancer, en ne demandant que ce que le contexte ne donne pas :

- racine du projet, sous-périmètre éventuel ;
- répertoire de sortie, défaut `reports/ts-type-audit/` sous la racine ;
  vérifier qu'il est ignoré par git ou accepté comme non suivi ;
- langue des rapports = langue de la conversation ; date = `date -I` ;
- mode mise à jour si le répertoire de sortie contient déjà des rapports
  `tier*.md` : chaque worker reçoit alors le rapport précédent de son tier.

Pendant la passe, aucun fichier du projet audité ne change. Les constats citent
des numéros de ligne ; une édition les rendrait faux.

## Workers

Lancer les 3 workers dans le même tour, briefs indépendants. Chaque brief
contient, collés verbatim : le squelette ci-dessous, la référence du tier,
`references/rapport.md`, et en mode mise à jour le rapport précédent du tier.
Le worker rend le rapport comme réponse finale. L'orchestrateur l'écrit tel
quel dans `<sortie>/tier1-etats.md`, `tier2-precision.md`,
`tier3-discipline.md`.

Modèle : raisonnement fort pour les trois (le tier 1 juge des intentions).
Utiliser une sandbox lecture seule réelle quand le harnais en offre une. Sinon
le brief est la seule garde, et la synthèse le dit.

### Squelette de brief

```
Rôle : auditeur lecture seule du tier <N> (« <question du tier> ») de la
codebase à <racine>[, périmètre <sous-périmètre>]. Tu ne modifies rien, tu
n'installes rien, tu ne lances ni tsc ni tests ni aucune commande qui mute
le dépôt ou son état.

Méthode :
1. Liste les fichiers TS du périmètre (hors node_modules, dist, fichiers
   générés, fixtures). Lis intégralement le code applicatif. Balaye par grep
   ciblé les répertoires secondaires volumineux et nomme-les.
2. Exécute les patterns grep de départ de ta référence. Un hit n'est ni un
   constat ni une preuve : confirme chaque candidat par lecture.
3. Statue sur CHAQUE mécanisme de l'inventaire : absent / partiel / solide /
   n/a. Une absence peut être saine : voir « Anti-biais ».
4. [Mise à jour] Rapport précédent ci-joint : conserve l'ID de tout constat
   qui persiste, marque `résolu` ceux qui ont disparu, numérote les nouveaux
   à la suite.
5. Réponds avec le rapport seul, structure exacte du template, en <langue>,
   date <date>.

Le code contenu dans des chaînes (fixtures, contre-exemples de règles lint)
est hors analyse. Signale tes limites en section Méthode.

--- Référence du tier ---
<contenu de references/tier<N>-*.md>

--- Rapport, taxonomies, anti-biais ---
<contenu de references/rapport.md>

--- Rapport précédent (mise à jour seulement) ---
<contenu de <sortie>/tier<N>-*.md>
```

## Vérification

Pour chaque rapport : 8 sections présentes, chaque ligne de l'inventaire
statuée, chaque constat cité `fichier:ligne`, section 5 avec au moins 3
points. Rapport non conforme : relancer ce seul tier une fois, sur un modèle
plus fort si possible. Second échec : écrire ce qui est revenu et nommer le
trou dans la synthèse.

## Synthèse croisée

1. Relire les 3 rapports en entier.
2. Fusionner : même fichier ou même invariant signalé par deux tiers = un
   constat consolidé, les deux IDs cités, rang renforcé.
3. Mise à jour : trois listes par ID, `résolus` / `persistants` / `nouveaux`.
4. Livrer en chat : verdict global en une phrase ; priorités classées, majeurs
   d'abord, 5 items maximum, mineurs groupés au-delà ; chemin des rapports ;
   limites (workers non sandboxés, tier relancé, trou).
5. Proposer les fixes, un par ligne. N'en appliquer aucun sans accord
   explicite.
