---
name: ts-type-audit
description: |
  Audit de maturité du typage d'une codebase TypeScript, en trois tiers de
  mécanismes avancés : états impossibles (unions discriminées, branded types,
  épuisement never, typestate), précision de l'inférence (tuples variadiques,
  types conditionnels, variance), discipline (tsconfig durci,
  parse-don't-validate, tests de types). Lance 3 sous-agents parallèles en
  lecture seule, un par tier ; chacun produit un rapport normalisé ; puis
  synthèse croisée classée en chat. Utiliser dès que l'utilisateur veut auditer
  ou évaluer le typage TypeScript d'un projet, savoir si une base exploite les
  mécanismes avancés ("premium") de TS, détecter candidats branded types,
  primitive obsession, exhaustivité manquante, ou mettre à jour un audit
  précédent. Audit statique de typage uniquement : pas une chasse aux bugs, pas
  une revue de perf ni de style. Diffère de state-machine/unrepresentable
  (conception de code neuf) et de code-review (revue d'un diff).
---

# ts-type-audit

Audit statique en lecture seule du typage d'une codebase TypeScript, organisé en
trois tiers. Un sous-agent par tier, rapports normalisés identiques en
structure, synthèse croisée par l'orchestrateur. La valeur vient de la
normalisation : des rapports comparables permettent la fusion des constats et
le suivi d'une passe à l'autre.

## Étapes

1. **Cadrer** : racine du projet, sous-périmètre éventuel, répertoire de sortie
   des rapports (proposer `reports/` par défaut). La langue des rapports est la
   langue de la conversation.
2. **Lancer les 3 sous-agents en parallèle**, dans le même tour, un par tier
   (voir `references/tier{1,2,3}.md` pour l'inventaire et le prompt de chacun).
   Agents en lecture seule, raisonnement solide (le tier 1 juge des intentions).
3. **Vérifier** : 3 rapports présents, 8 sections complètes, inventaire
   exhaustif, chaque constat cité `fichier:ligne`.
4. **Synthèse croisée** en chat (protocole plus bas).

Pendant toute la passe : ne modifier aucun fichier du projet audité. Les
rapports citent des numéros de ligne ; une édition pendant la passe les rend
faux.

## Prompt squelette pour chaque agent

Adapter à chaque tier, garder la structure :

```
Rôle : auditeur lecture seule du tier N (« <question du tier> ») de la
codebase à <racine>. Tu ne modifies rien ; tu ne lances ni tsc ni tests.

Méthode :
1. Liste les fichiers TS du périmètre (hors node_modules, dist, fixtures).
   Lis intégralement le code applicatif ; les gros répertoires secondaires
   peuvent être balayés par grep ciblé.
2. Exécute les patterns grep de départ de ta référence. Un hit de grep n'est
   ni un constat ni une preuve : confirme chaque candidat par lecture avant
   de conclure.
3. Statue sur CHAQUE mécanisme de l'inventaire de ta référence (statut
   absent / partiel / solide / n/a). Une absence peut être saine : voir
   « Anti-biais ».
4. Écris ton rapport toi-même à <chemin imposé>, structure exacte du
   template ci-dessous, en <langue>.

Interdictions : aucune édition, aucune installation, aucune commande qui
muterait le dépôt ou son état (pas de --fix, pas de rm). Exclus de
l'analyse le code contenu en chaînes (fixtures, contre-exemples de règles
lint). Signale tes limites en section Méthode.
```

## Template de rapport (structure exacte, 8 sections)

```markdown
# Audit TypeScript — Tier N · <nom du tier>

> Rapport normalisé, rédigé par le sous-agent Tier N seul. Date de la passe :
> <date>. Périmètre : <question du tier>.

## 1. Méthode
Fichiers balayés (intégralement vs grep), patterns exécutés, limites de la
passe (audit statique, ni tsc ni tests sauf demande explicite).

## 2. Verdict
3 à 5 phrases : niveau général du tier, les 1 à 2 absences réelles, le ton
de la suite du rapport.

## 3. Inventaire des mécanismes
Table : # | Mécanisme | Statut (`absent`/`partiel`/`solide`/`n/a`) |
Localisations | Note courte. Toutes les lignes de la référence du tier,
sans exception.

## 4. Constats détaillés
Un bloc par constat, id `T<N>-F<xx>` (numérotation continue) :
- Sévérité : `majeur` / `mineur` / `info`
- Localisation : `fichier:lignes`
- Constat : ce que le code permet aujourd'hui, preuve à l'appui
- Recommandation : le changement minimal, et le comportement inchangé

## 5. Déjà bien fait
Minimum 3 points positifs concrets, cités `fichier:lignes`. Obligatoire :
il calibre le rapport et évite le biais de ne chercher que des défauts.

## 6. Priorités
Numérotées, ordonnées par ratio bénéfice/coût, une ligne chacune.

## 7. Hors périmètre
Observations libres touchant un autre tier, sans verdict.

## 8. Sources consultées
Chemins lus, greps exécutés.
```

## Taxonomies

Statuts d'inventaire : `solide` (mécanisme présent et correctement employé),
`partiel` (présent mais inégal entre zones du dépôt), `absent` (aucun usage),
`n/a` (aucun site concevable dans ce projet).

Sévérités :

- `majeur` : une déviation silencieuse est compilable aujourd'hui. Exemple
  réel : un switch de dispatch dont le `default` route tout cas nouveau vers
  un fallback, parce que l'épuisement `never` n'est pas prouvé.
- `mineur` : un invariant est porté par convention, duplication ou
  commentaire au lieu du type. Exemples réels : la même expression de garde
  dupliquée en deux fichiers ; un tuple non vide reconstruit à la main à
  deux endroits ; ~30 fichiers TS hors `include` du tsconfig.
- `info` : redondance bénigne, type mort, tolérance inégale non documentée,
  ou absence SANS site légitime. Une absence saine se constate, elle ne
  recommande rien.

## Anti-biais

La règle la plus importante du skill : distinguer l'absence injustifiée de
l'absence justifiée. Variance explicite, `NoInfer`, remapping de clés sont
absents de la plupart des bases applicatives sans qu'aucun site ne les
requière : statut `absent` + note « aucun site légitime », et AUCUNE
recommandation d'introduction. Ne jamais recommander un mécanisme pour
lui-même ; un mécanisme entre quand un invariant réel existe et que le
compilateur peut le porter mieux qu'un commentaire ou une duplication.

Autres règles :

- Chaque constat cite `fichier:ligne` vérifié par lecture, jamais déduit
  d'un grep.
- Les fixtures et chaînes de test ne sont pas du code applicatif.
- Deux agents qui signalent le même endroit sont un signal, pas un doublon
  à supprimer prématurément (la fusion a lieu en synthèse).

## Synthèse croisée (orchestrateur)

Après vérification des 3 rapports :

1. Relis les 3 rapports en entier.
2. Fusionne les chevauchements : même fichier ou même mécanisme signalé par
   deux tiers = un seul constat consolidé, citant les deux IDs, classé au
   rang renforcé.
3. Livre en chat : verdict global en une phrase ; classement des priorités
   (majeurs d'abord, maximum 5 items, au-delà grouper les mineurs) ;
   mention de l'emplacement des rapports.
4. Propose les fixes applicables, un par ligne, sans en appliquer aucun
   sans accord explicite.
