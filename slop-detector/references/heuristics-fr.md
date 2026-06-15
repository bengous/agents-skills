# Heuristiques d'écriture IA — Français

Catalogue des tics de langage de l'IA en français. Charger ce fichier quand le
texte audité est en français (ou lancer `score.py --lang fr`). Les heuristiques
anglaises de `heuristics.md` ne s'appliquent pas telles quelles : la langue
change les marqueurs, et surtout les anglicismes deviennent un signal majeur.

## Principe de gradation (rappel)

Comme en anglais : un tic isolé n'est pas une preuve. **Les signaux DURS**
(fuite d'assistant, artefacts de modèle, calques très spécifiques) comptent dès
une occurrence ; **les signaux DOUX** (anglicismes courants, tirets, intensifs,
connecteurs) ne comptent qu'en grappe — répétition, ou co-occurrence avec un
signal dur ou un pivot. Voir `references/strictness-modes.md`.

## Catégories

- **assistant-leakage** (DUR, sev 10) — « en tant que modèle de langage », « en tant qu'assistant IA », divulgation d'identité ou de date de connaissance.
- **negation-affirmation-pivot** (PRIORITÉ 1, doux-gated) — la famille antithèse : « ce n'est pas X, c'est Y ». C'est le tic n°1 signalé par l'utilisateur.
- **fr-anglicism** (PRIORITÉ 2) — calques et emprunts (voir le barème par force ci-dessous).
- **typography** (PRIORITÉ 3) — tiret cadratin, guillemets, espace insécable, AVEC garde anti-faux-positif (la typo française correcte ne doit JAMAIS être signalée).
- **opener-cliche / hedge-scaffolding / lexical-cliche / copula-avoidance / structural-connector / significance-inflation** — équivalents français des catégories anglaises.

## Anglicismes — barème par force

- **FORT, FP faible (peu/pas gated) :** *adresser* un problème/une question ; *délivrer* de la valeur/des résultats ; *introduire* quelqu'un (= présenter) ; *accommoder* X personnes ; (des/les) *évidences* (= preuves) ; *couvrir* les bases/un sujet (= traiter).
- **MOYEN (gated) :** *définitivement* (= certainement) ; *réaliser que* (= se rendre compte) ; *en termes de* ; *en charge de* ; *initier* (un projet) ; *implémenter* ; *impacter* (verbe) ; *investiguer* ; *sur base de* / *sur la base de*.
- **FAIBLE, FP élevé (toujours gated, jamais seul) :** *digital* (= numérique) ; *challenge* (= défi) ; *au final* ; *baser/basé sur* (= fonder sur) ; *opportunité* (surusage) ; *futur* (= avenir).

Un anglicisme courant employé une fois par un humain n'est pas du slop. Le signal,
c'est l'**accumulation** d'anglicismes + d'autres tics dans un même texte.

## Garde typographique — NE JAMAIS signaler

- Espace insécable (U+00A0 / U+202F) **avant** `:` `;` `!` `?` et **dans** « … » = typographie française CORRECTE. Supprimer toute alerte NBSP en contexte français.
- Le tiret cadratin (—) : signaler seulement la **densité** anormale. Le tiret collé (`mot—mot`, sans espaces) est un import anglais → signal plus fort. Le français bien composé préfère souvent les parenthèses, la virgule ou le deux-points.
- Guillemets : le français correct utilise « » avec espaces insécables. Les guillemets droits `"` ou courbes `"` en contexte français peuvent indiquer un copier-coller machine — mais c'est FP élevé (clavier), donc à grouper.

## Phrases signatures (haute valeur)

- **Pivots :** « ce n'est pas X, c'est Y » / « il ne s'agit pas (tant) de X mais de Y » / « non (pas) seulement X, mais (aussi) Y » / « loin d'être X, c'est Y » / « ce n'est pas tant X que Y » / cascade « Pas de X. Pas de Y. Juste Z. ».
- **Ouvertures :** « Dans un monde en constante évolution » / « Dans le monde trépidant d'aujourd'hui » / « À l'ère du… » / « Dans cet article, nous allons explorer » / « plongeons dans les complexités » / « Que vous soyez X ou Y ».
- **Échafaudage :** « il convient de noter que » / « il est important de noter que » / « force est de constater que ».
- **Inflation de portée :** « jouer un rôle crucial / clé » / « témoigne de » / « ouvrir la voie à ».
- **Clichés lexicaux :** « naviguer dans le paysage » / « pierre angulaire » / « libérer le potentiel » / « propulser vers de nouveaux sommets » / « un savant mélange » / « tisser des liens » / « à double tranchant » ; intensifs : *crucial, essentiel, primordial, incontournable, véritable, robuste, holistique*.
- **Périphrases corporate (copula-avoidance) :** « tirer parti de » / « mettre en œuvre » / « optimiser » / « être en mesure de » / « en capacité de » / « répondre aux besoins ».
- **Connecteurs (densité) :** par ailleurs, en outre, de surcroît, qui plus est, d'une part… d'autre part, en somme, en définitive, en conclusion.

## Quoi faire

Mêmes principes qu'en anglais : citer la phrase exacte, expliquer le problème,
proposer un remplacement qui préserve la voix. Pour un anglicisme, proposer le
mot français juste (« adresser un problème » → « traiter / régler un problème »).
Ne pas sur-corriger : une écriture française vivante garde des aspérités.
