# Tier 2 · Précision de l'inférence

Question du tier : que sait prouver le compilateur grâce aux mécanismes qui
contrôlent ou affinent l'inférence ?

## Inventaire (statuer sur chaque ligne)

1. **Tuples typés et variadiques** : « au moins un élément »
   (`readonly [T, ...T[]]`), membres nommés (`[program: string, ...args:
   string[]]`), composition `[...A, ...B]`.
2. **Types conditionnels + `infer`** : distinguer les utilitaires standard
   (`Extract`, `Exclude`, `Parameters`, `ReturnType`, `Awaited`, dérivation de
   schéma type `z.infer`) des conditionnels artisanaux (`T extends U ? … :
   …`). Les premiers sont sains et courants ; les seconds demandent une
   justification.
3. **Variance explicite `in` / `out`** : utile seulement sur des génériques
   multi-positions ou muables.
4. **`NoInfer`** : utile seulement quand un paramètre générique a deux sources
   d'inférence concurrentes.
5. **Mapped types avec remapping de clés** : `[K in keyof T as …]` ;
   distinguer du `Record<string, T>` standard.
6. **Génériques contraints** : `<T extends …>` qui portent une relation entre
   paramètres et retour, contre `any`/`unknown` élargis à l'appel.

## Grep de départ

- `\binfer\b`, `\bNoInfer\b`, `<\s*(in|out)\s+[A-Za-z_]`
- `\.\.\.[A-Za-z_]+\[\]`, `^\s*(export )?type \w+ = (readonly )?\[`
- `\[K in|\[P in|\bkeyof\b`
- `Record<|Pick<|Readonly<|Partial<|Omit<`
- `ReturnType<|Awaited<|Parameters<|InstanceType<`
- `Extract<|Exclude<`, `= .* extends .* \?`
- `\.infer<|\.output<|\.input<`

## Signaux typiques

- `mineur` : reconstruction d'un tuple non vide dupliquée en plusieurs sites
  (déstructurer la tête, garder `undefined`, reconstruire) ; l'invariant
  repose sur des copies manuelles synchrones. Fix : un helper unique qui
  retourne `NonEmpty<T> | null`.
- `mineur` : fonction générique dont le retour est `any` ou `unknown` alors
  que la relation avec l'entrée est connue (`ReturnType`, `Extract`).
- `info` : conditionnel artisanal qui réimplémente un utilitaire standard.
- `info` × 3 : variance, `NoInfer`, remapping absents sans aucun site
  légitime. Absences saines, rien à recommander.
- Point positif type : invariant « au moins un élément » porté par
  `readonly [T, ...T[]]` et construit seulement après garde ; prédicat adossé
  à `Extract<Union, { kind: "x" }>`.

## Pièges

- Ce tier est volontairement mince dans la plupart des bases applicatives : un
  inventaire majoritairement `absent` est normal. Le jugement porte sur deux
  choses : les invariants existants sont-ils portés par le système de types là
  où c'est rentable, et les preuves dupliquées sont-elles factorisées ?
- Exclure les usages de `infer` et de mapped types présents seulement en
  chaînes (fixtures de tests).
- Ne pas recommander un mapped type parce qu'un renommage de clés existe à
  l'exécution : si les données viennent d'une frontière (JSON distant), la
  transformation à la frontière est le bon endroit.
