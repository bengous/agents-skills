# Tier 1 · États impossibles

Question du tier : quels états ou séquences invalides reste-t-il possible
d'écrire dans cette codebase ?

## Inventaire des mécanismes (statuer sur chacun)

1. **Unions discriminées + épuisement `never`** : discriminateur unique
   (`kind:` / `mode:` / `type:`), narrowing systématique, et preuve
   d'épuisement (`assertNever`, `const exhausted: never`) dans chaque
   `switch` qui dispatche une union. Un `default` qui avale des cas futurs
   sans erreur de compilation est LE constat majeur type de ce tier.
2. **Branded / opaque types** : `unique symbol` en marque, constructeur
   gardien unique, invariant documenté par un commentaire `SAFETY`.
   Détecter aussi les candidats manqués : primitives porteuses d'un
   domaine (ids, paths, tokens) passées nues entre fonctions.
3. **Typestate / builder typé** : séquences invalides non compilables ;
   état intermédiaire portant des corrélations (champs booléens ou
   optionnels corrélés = suspect).
4. **Template literal types** : types construits sur gabarits (routes,
   clés d'événements, sélecteurs).
5. **Paramètres génériques `const`** : `<const T>` ; usage couvert par
   `as const` dérivé en `(typeof X)[number]` quand le besoin est simple.
6. **`satisfies`** : validation de forme sans élargissement.

## Patterns grep de départ

- `\bnever\b`, `switch \(` + champs discriminateurs `kind:|mode:|type:|state:`
- `z\.discriminatedUnion`
- `unique symbol|__brand|\bbrand`
- `<const `, `as const`
- backtick en position de type, `extends \``
- `satisfies`

## Calibration (constats réels d'une passe de référence)

- `majeur` — dispatch CLI : cast `head as Subcommand` après un test
  `includes()`, `default` qui route verbatim vers un fallback. Ajouter un
  nom à la liste sans ajouter de `case` compile et dévie silencieusement.
  Fix : `assertNever` dans le `default`, comportement actuel inchangé.
- `mineur` — interface `Draft` avec champs corrélés non représentés :
  l'invariant « stocker exige un token résolu » est vérifié deux fois à
  l'exécution avec la même expression triple, dans deux fichiers. Fix :
  champ discriminé construit seulement quand la clé est validée.
- `mineur` — `?? ""` fabrique une valeur impossible dont la preuve vit
  dans une fonction distante ; le type laisse `""` représentable.
- `info` — garde redondante doublant une discrimination déjà garantie par
  le schéma de validation.
- `info` — type exporté sans aucune référence : mort, ou occasion manquée
  de tracer une provenance.

## Pièges

- Un hit de grep ne prouve rien : lire le contexte avant tout constat.
- Les fixtures (contre-exemples de règles lint, chaînes de test)
  contiennent souvent du code type-level : hors analyse.
- L'absence de branded types peut être saine (petit CLI, aucun risque de
  confusion entre domaines) : statut `absent` + note « aucun site
  légitime », pas de recommandation.
- Le pattern `?? fail(...)` avec `fail(): never` est une technique
  valide de narrowing : ne pas le confondre avec un `?? ""` fautif.
