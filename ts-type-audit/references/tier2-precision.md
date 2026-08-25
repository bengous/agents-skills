# Tier 2 · Précision de l'inférence

Question du tier : que sait prouver le compilateur grâce aux mécanismes
qui contrôlent ou affinent l'inférence ?

## Inventaire des mécanismes (statuer sur chacun)

1. **Tuples variadiques** : tuples typés pour « au moins un élément »
   (`readonly [T, ...T[]]`), membres nommés (`[program: string, ...args:
   string[]]`), composition `[...A, ...B]`.
2. **Types conditionnels + `infer`** : distinguer les utilitaires standard
   (`Extract`, `Parameters`, `ReturnType`, `Awaited`, dérivation zod
   `z.infer`/`z.output`) des conditionnels artisanaux (`= T extends U ? …`).
   Les premiers sont sains et courants ; les seconds demandent une
   justification.
3. **Variance explicite `in` / `out`** : utile seulement sur des génériques
   multi-positions ou muables.
4. **`NoInfer`** : utile seulement quand un paramètre générique a deux
   sources d'inférence concurrentes.
5. **Mapped types avec remapping de clés** : `[K in keyof T as …]` ;
   distinguer du `Record<string, string>` standard.

## Patterns grep de départ

- `\binfer\b`, `\bNoInfer\b`, `<\s*(in|out)\s+[A-Za-z_]`
- `\.\.\..*\[\]`, `^\s*(export )?type \w+ = \[`
- `\[K in|\[P in|\bkeyof\b`
- `Record<|Pick<|Readonly<|Partial<|Omit<`
- `ReturnType|Awaited<|Parameters<|InstanceType`
- `z\.infer|z\.output`, `Extract<`, `= .* extends`

## Calibration (constats réels d'une passe de référence)

- `mineur` — reconstruction d'un tuple non vide dupliquée en deux sites
  (déstructurer la tête, garde `undefined`, reconstruire) : l'invariant du
  type repose sur deux copies manuelles synchrones. Fix : helper unique
  `nonEmptyOf(files): ExistingTouchedFiles | null`.
- `info` × 3 — variance, `NoInfer`, remapping de clés absents sans aucun
  site légitime : absences saines, aucune action, ne rien recommander.
- Point positif type — invariant « au moins un élément » porté par
  `readonly [string, ...string[]]` et construit uniquement après garde ;
  membre nommé exploitant l'index 0 ; prédicat adossé à
  `Extract<Outcome, { kind: "fail" }>`.

## Pièges

- Ce tier est volontairement mince dans la plupart des bases applicatives :
  un inventaire majoritairement `absent` est normal. Le jugement porte sur
  deux choses : les invariants existants sont-ils portés par le système de
  types là où c'est rentable, et les preuves dupliquées sont-elles
  factorisées ?
- Exclure les usages de `infer`/mapped types présents uniquement en chaînes
  (fixtures de tests de règles lint).
- Ne pas recommander un mapped type parce qu'un renommage de clés existe en
  runtime : si les données viennent du JSON distant, la transformation
  runtime à la frontière est le bon endroit.
