# Rapport normalisé

## Template (structure exacte, 8 sections)

```markdown
# Audit TypeScript — Tier <N> · <nom du tier>

> Passe du <date>. Périmètre : <question du tier>. Auteur : worker tier <N>
> seul.

## 1. Méthode
Fichiers lus intégralement vs balayés par grep, patterns exécutés, limites
(audit statique, ni tsc ni tests).

## 2. Verdict
3 à 5 phrases : niveau général du tier, les 1 à 2 absences réelles, le ton de
la suite.

## 3. Inventaire des mécanismes
Table : # | Mécanisme | Statut | Localisations | Note. Toutes les lignes de
l'inventaire du tier, sans exception.

## 4. Constats détaillés
Un bloc par constat, id `T<N>-F<xx>`, numérotation continue, IDs stables d'une
passe à l'autre :
- Sévérité : `majeur` / `mineur` / `info`
- Localisation : `fichier:lignes`
- Constat : ce que le code permet aujourd'hui, preuve à l'appui
- Recommandation : le changement minimal, comportement inchangé
- État (mise à jour seulement) : `nouveau` / `persistant` / `résolu`

## 5. Déjà bien fait
Au moins 3 points positifs concrets, cités `fichier:lignes`. Obligatoire : ils
calibrent le rapport.

## 6. Priorités
Numérotées, ordonnées par ratio bénéfice/coût, une ligne chacune.

## 7. Hors périmètre
Observations touchant un autre tier, sans verdict.

## 8. Sources consultées
Chemins lus, greps exécutés.
```

## Statuts d'inventaire

- `solide` : mécanisme présent et correctement employé.
- `partiel` : présent mais inégal entre zones du dépôt.
- `absent` : aucun usage. Préciser en note si un site légitime existe.
- `n/a` : aucun site concevable dans ce projet.

## Sévérités

- `majeur` : une déviation silencieuse est compilable aujourd'hui. Type : un
  `switch` sur une union dont le `default` absorbe tout membre ajouté plus
  tard.
- `mineur` : un invariant est porté par convention, duplication ou commentaire
  au lieu du type. Types : la même garde dupliquée dans deux fichiers ; du TS
  hors de l'`include` du tsconfig.
- `info` : redondance bénigne, type mort, tolérance inégale non documentée, ou
  absence sans site légitime. Une absence saine se constate, elle ne
  recommande rien.

## Anti-biais

Règle première : distinguer l'absence injustifiée de l'absence justifiée.
Variance explicite, `NoInfer`, remapping de clés, typestate sont absents de la
plupart des bases applicatives sans qu'aucun site ne les requière : statut
`absent`, note « aucun site légitime », AUCUNE recommandation d'introduction.
Un mécanisme entre quand un invariant réel existe et que le compilateur peut
le porter mieux qu'un commentaire ou une duplication. Jamais pour lui-même.

- Chaque constat cite `fichier:ligne` vérifié par lecture, jamais déduit d'un
  grep.
- Les fixtures et les chaînes de test ne sont pas du code applicatif.
- Deux tiers qui signalent le même endroit forment un signal ; la fusion a
  lieu en synthèse, pas dans le rapport.
- La section 5 est obligatoire : un rapport sans point positif est
  non conforme.
