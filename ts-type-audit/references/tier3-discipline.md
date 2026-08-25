# Tier 3 · Discipline

Question du tier : la sûreté de type est-elle effective, vérifiée, et
couvre-t-elle tout le projet ?

## Inventaire des mécanismes (statuer sur chacun)

1. **tsconfig durci** : au-delà de `strict` — `noUncheckedIndexedAccess`,
   `exactOptionalPropertyTypes`, `verbatimModuleSyntax`, `isolatedModules`.
   Deux vérifications : le code est-il réellement conforme (pas de rustine)
   et l'`include` couvre-t-il tout le TS du dépôt (tools/, scripts/, hooks/,
   plugin locaux) ? Du TS jamais vu par `tsc` est un constat mineur type.
2. **Parse, don't validate** : un schéma (zod ou équivalent) à CHAQUE
   frontière réelle — stdin, variables d'environnement, HTTP, fichiers de
   config, argv — avec types dérivés (`z.infer`/`z.output`), zéro interface
   dupliquée du format wire. Compter les frontières et vérifier chacune.
3. **Tests de types** : `@ts-expect-error`, `expect-type`, `tsd`
   verrouillant marques, dérivations et options durcies ; couverts par un
   gate `tsc --noEmit` existant si possible. Zéro test de type alors que le
   gate couvre déjà les tests est un constat mineur type.
4. **Gardes et assertions typées** : `x is T`, `asserts x is T` ciblés ;
   discipline de cast MÉCANISÉE (règle lint maison exigeant une
   justification type `SAFETY:`) plutôt que commentataire. Zéro `as any`,
   zéro assertion non-null en code applicatif.
5. **Gestion explicite des ressources** : `using` / `Symbol.dispose` quand
   un couple acquire/release est protégé par `try`/`finally` (terminal,
   fichiers, listeners). Vérifier la disponibilité (`target: esnext`,
   version TS).

## Patterns grep de départ

- `z\.infer|z\.object|safeParse`
- `@ts-expect-error|@ts-ignore|expect-type|tsd`
- `\bis [A-Z]|asserts `
- `\busing\b|Symbol\.(async)?Dispose`
- `\bas unknown|\bas any|!\.|as [A-Za-z]`, `satisfies`
- Lecture directe obligatoire : `tsconfig.json`, `package.json` (options,
  versions), config lint existante.

## Calibration (constats réels d'une passe de référence)

- `mineur` — ~30 fichiers TS hors `include` du tsconfig, jamais typecheckés,
  seulement chargés à l'exécution. Fix : une ligne dans l'`include`.
- `mineur` — narrowing par casts successifs commentés `SAFETY:` au lieu
  d'un prédicat `isSubcommand(value): value is Subcommand` : la preuve
  reste commentataire, le compilateur ne voit pas la divergence possible.
- `mineur` — zéro test de type dans tout le dépôt alors que le gate
  `typecheck` couvre déjà le répertoire de tests.
- `mineur` — pairing terminal (raw mode + handlers + état) géré par
  `try`/`finally` alors que `using`/`Symbol.dispose` est disponible ;
  l'idempotence existe déjà, le `[Symbol.dispose]()` est une ligne.
- `info` — deux régimes de tolérance aux frontières fichier (échec bruyant
  vs retour silencieux de `undefined`) sans politique explicite.
- `info` — échafaudage identique (pipeline de parse stdin) copié entre
  plusieurs fichiers : divergence future à trois modifications synchrones.

## Pièges

- Vérifier la CONFORMITÉ réelle aux options durcies par lecture : rustines
  classiques de `noUncheckedIndexedAccess` (indexation non gardée) et de
  `exactOptionalPropertyTypes` (`undefined` explicite là où il faut une
  assignation conditionnelle).
- Une règle lint maison qui impose des justifications de cast transforme
  une convention en mécanisme : c'est `solide`, même si quelques casts
  subsistent.
- Les interfaces manuelles restantes sont acceptables quand elles décrivent
  des données possédées par le code, pas du texte externe.
