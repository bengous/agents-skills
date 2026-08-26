# Tier 1 · États impossibles

Question du tier : quels états ou séquences invalides reste-t-il possible
d'écrire dans cette codebase ?

## Inventaire (statuer sur chaque ligne)

1. **Unions discriminées + épuisement `never`** : discriminateur unique
   (`kind` / `type` / `status`), narrowing systématique, preuve d'épuisement
   (`assertNever`, `const exhausted: never`) dans chaque `switch` ou chaîne
   `if` qui dispatche une union. Un `default` qui avale des membres futurs
   sans erreur de compilation est LE constat majeur type de ce tier.
2. **Branded / opaque types** : marque par `unique symbol` ou propriété
   fantôme, constructeur gardien unique, invariant documenté. Détecter aussi
   les candidats manqués : primitives porteuses d'un domaine (ids, chemins,
   tokens, montants, unités) passées nues entre fonctions, surtout deux
   paramètres adjacents de même primitive et de domaines différents.
3. **Typestate / builder typé** : séquences invalides non compilables ; état
   intermédiaire portant des corrélations. Champs booléens ou optionnels
   corrélés (`done: boolean` + `result?: T`) = suspect.
4. **Template literal types** : types construits sur gabarits (routes, clés
   d'événements, sélecteurs, préfixes d'ids).
5. **`as const` et paramètres génériques `const`** : littéraux préservés,
   dérivation `(typeof X)[number]`, `<const T>` quand l'appelant fournit le
   littéral.
6. **`satisfies`** : validation de forme sans élargissement.

## Grep de départ

- `\bnever\b`, `switch \(`, `default:`, discriminateurs `kind:|type:|status:|mode:`
- `discriminatedUnion`
- `unique symbol|__brand|\bbrand|Branded<|Opaque<`
- `as const`, `<const `
- `` extends ` ``, backtick en position de type
- `satisfies`
- `\?\?\s*(""|0|\[\]|\{\})` (valeurs par défaut fabriquées)

## Signaux typiques

- `majeur` : switch de dispatch sur une union, `default` qui route vers un
  fallback, aucun `never`. Ajouter un membre compile sans erreur et dévie
  silencieusement. Fix : `assertNever(value)` dans le `default`, comportement
  inchangé.
- `majeur` : cast vers un type union après un test runtime partiel
  (`includes`, `in`, regex), sans prédicat `value is T`.
- `mineur` : interface à champs corrélés ; l'invariant est revérifié à
  l'exécution par la même expression à plusieurs endroits. Fix : membre
  d'union construit seulement quand l'invariant est établi.
- `mineur` : `?? ""` ou `?? 0` fabrique une valeur que le domaine interdit ;
  le type la laisse représentable.
- `info` : garde redondante doublant une discrimination déjà garantie par un
  schéma de validation ; type exporté sans référence.

## Pièges

- Un hit de grep ne prouve rien : lire le contexte avant tout constat.
- Les fixtures et chaînes de test contiennent souvent du code type-level :
  hors analyse.
- L'absence de branded types peut être saine (petit outil, aucun risque de
  confusion entre domaines) : statut `absent`, note « aucun site légitime »,
  pas de recommandation.
- `?? fail()` avec `fail(): never` est un narrowing valide, pas un `?? ""`
  fautif.
- Le typestate complet est rare et coûteux ; le recommander seulement quand
  une séquence invalide est atteignable et a un coût réel.
