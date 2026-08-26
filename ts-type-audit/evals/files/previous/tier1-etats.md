# Audit TypeScript — Tier 1 · États impossibles

> Passe du 2026-06-01. Périmètre : quels états ou séquences invalides
> reste-t-il possible d'écrire ? Auteur : worker tier 1 seul.

## 1. Méthode
Lecture intégrale de `src/`. Greps : `\bnever\b`, `switch \(`, `__brand`,
`as const`, `satisfies`, `\?\?\s*""`. Audit statique, ni tsc ni tests.

## 2. Verdict
Union discriminée présente mais sans preuve d'épuisement. Aucun branded type.
Une valeur par défaut fabriquée dans le chargement de config.

## 3. Inventaire des mécanismes
| # | Mécanisme | Statut | Localisations | Note |
|---|---|---|---|---|
| 1 | Unions discriminées + `never` | partiel | `src/commands.ts` | dispatch sans `never` |
| 2 | Branded types | absent | | candidats `userId`/`orderId` |
| 3 | Typestate | absent | | aucun site légitime |
| 4 | Template literal types | absent | | aucun site légitime |
| 5 | `as const` / `<const T>` | solide | `src/commands.ts:6` | |
| 6 | `satisfies` | solide | `src/files.ts:8` | |

## 4. Constats détaillés

### T1-F01
- Sévérité : `majeur`
- Localisation : `src/commands.ts:20-29`
- Constat : `run` dispatche `Command` par `switch` ; le `default` route `lint`
  et tout membre futur vers le même retour. Ajouter un membre compile.
- Recommandation : `case "lint"` explicite et `assertNever(command)` dans le
  `default`.

### T1-F02
- Sévérité : `mineur`
- Localisation : `src/config.ts:11`
- Constat : `dataDir: raw.dataDir ?? ""` fabrique un chemin vide que le
  domaine interdit ; le type `string` le laisse représentable.
- Recommandation : échouer quand `dataDir` est absent.

## 5. Déjà bien fait
- `src/commands.ts:6-7` : noms de commandes en `as const`, type dérivé.
- `src/files.ts:1-6` : tuple non vide construit seulement après garde.
- `src/files.ts:8` : `satisfies` sans élargissement.

## 6. Priorités
1. T1-F01 : `assertNever` dans le dispatch.
2. T1-F02 : supprimer le `?? ""`.

## 7. Hors périmètre
`src/commands.ts:11-14` : cast `head as CommandName` après `includes`, sujet
tier 3.

## 8. Sources consultées
`src/commands.ts`, `src/orders.ts`, `src/store.ts`, `src/config.ts`,
`src/terminal.ts`, `src/files.ts`.
