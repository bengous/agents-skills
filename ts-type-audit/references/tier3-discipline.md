# Tier 3 · Discipline

Question du tier : la sûreté de type est-elle effective, vérifiée, et
couvre-t-elle tout le projet ?

## Inventaire (statuer sur chaque ligne)

1. **tsconfig durci** : au-delà de `strict`, `noUncheckedIndexedAccess`,
   `exactOptionalPropertyTypes`, `verbatimModuleSyntax`, `isolatedModules`.
   Deux vérifications : le code est-il réellement conforme (pas de rustine),
   et l'`include` couvre-t-il tout le TS du dépôt (`scripts/`, `tools/`,
   hooks, plugins locaux) ? Du TS jamais vu par `tsc` est un constat mineur
   type.
2. **Parse, don't validate** : un schéma (zod, valibot, arktype ou équivalent)
   à CHAQUE frontière réelle : stdin, variables d'environnement, HTTP,
   fichiers de config, argv, stockage. Types dérivés du schéma, zéro
   interface dupliquée du format wire. Compter les frontières et vérifier
   chacune.
3. **Tests de types** : `@ts-expect-error`, `expectTypeOf`, `tsd` verrouillant
   marques, dérivations et options durcies ; couverts par un gate `tsc
   --noEmit` existant. Zéro test de type alors que le gate couvre déjà les
   tests est un constat mineur type.
4. **Gardes et assertions typées** : `x is T`, `asserts x is T` ciblés.
   Discipline de cast mécanisée (règle lint exigeant une justification) plutôt
   que par commentaire. Zéro `as any`, zéro assertion non-null en code
   applicatif.
5. **Gestion explicite des ressources** : `using` / `Symbol.dispose` quand un
   couple acquire/release est protégé par `try`/`finally` (terminal, fichiers,
   listeners, connexions). Vérifier la disponibilité (`target`/`lib`, version
   TS).

## Grep de départ

- `\.infer<|\.parse\(|safeParse|\.object\(`
- `@ts-expect-error|@ts-ignore|@ts-nocheck|expectTypeOf|tsd`
- `\bis [A-Z]|asserts `
- `\busing\b|Symbol\.(async)?[Dd]ispose`
- `\bas unknown\b|\bas any\b|!\.|!\)|!;`, `\bas [A-Z]` (hors `as const`)
- `process\.env|JSON\.parse|readFileSync|fetch\(|argv`
- Lecture directe obligatoire : `tsconfig*.json`, `package.json` (options,
  versions, scripts), config lint existante.

## Signaux typiques

- `mineur` : fichiers TS hors `include` du tsconfig, jamais typecheckés,
  seulement chargés à l'exécution. Fix : une ligne dans l'`include`.
- `mineur` : frontière lue sans schéma (`JSON.parse(...) as Config`,
  `Number(process.env.X)`) ; le type est affirmé, pas prouvé.
- `mineur` : narrowing par casts successifs commentés au lieu d'un prédicat
  `isX(value): value is X` ; la preuve reste un commentaire.
- `mineur` : zéro test de type alors que le gate `typecheck` couvre déjà les
  tests.
- `mineur` : acquire/release en `try`/`finally` alors que `using` est
  disponible ; si la libération est déjà idempotente, `[Symbol.dispose]()` est
  une ligne.
- `info` : deux régimes de tolérance aux frontières (échec bruyant vs retour
  silencieux de `undefined`) sans politique explicite.

## Pièges

- Vérifier la CONFORMITÉ réelle aux options durcies par lecture : rustines
  classiques de `noUncheckedIndexedAccess` (indexation non gardée puis `!`) et
  de `exactOptionalPropertyTypes` (`undefined` explicite là où il faut une
  assignation conditionnelle).
- Une règle lint qui impose des justifications de cast transforme une
  convention en mécanisme : `solide`, même si quelques casts subsistent.
- Les interfaces manuelles restantes sont acceptables quand elles décrivent
  des données possédées par le code, pas du texte externe.
- `using` demande `target`/`lib` esnext ou un polyfill : vérifier avant de le
  recommander.
