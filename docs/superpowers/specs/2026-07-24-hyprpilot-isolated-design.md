# hyprpilot `--isolated` : bureaux agents isolés sur Hyprland

Date : 2026-07-24. Statut : design validé en brainstorming + PoC empirique
(7 runs Codex sur session vivante, thread `019f9428-8c4c-7633-8317-aa054f0d78d7`,
artefacts `scratchpad/poc/run3-run8` de la session 0d81af35). Base de code :
branche `feature/hyprpilot-session-v2` (schema_version 2, mode partagé).

## 1. Objectif

Permettre à un ou plusieurs agents de piloter des GUI chacun sur un bureau
complet qui lui appartient (compositeur, seat, curseur, clavier propres),
pendant qu'Augustin travaille sur son bureau sans aucune interférence :
fondation d'un computer use local pour agents sur Hyprland/Wayland.

Le mode partagé actuel reste le mode « piloter MES fenêtres réelles » ; il
est inchangé fonctionnellement.

## 2. Faits empiriques qui contraignent le design

Validés sur session vivante (PoC 2026-07-23 et 2026-07-24) :

1. Hyprland pur headless crashe (0.56, pas de DRM master) : le bureau agent
   est un Hyprland imbriqué (fenêtre sur l'hôte), obligatoirement.
2. Un nested dont la fenêtre-console est occluse (workspace hôte inactif)
   ne reçoit plus de frame callbacks : son rendu se fige et toute capture
   screencopy bloque indéfiniment. `AQ_NO_MODIFIERS` et le rendu logiciel
   ne changent rien ; ce n'est pas un problème dmabuf.
3. La solution : la console vit sur le workspace ACTIF d'un output headless
   hôte. L'hôte composite cet output en permanence, invisible pour
   l'utilisateur ; les captures marchent alors des deux côtés.
   `moveworkspacetomonitor` vers le headless ne suffit pas (le workspace y
   reste inactif) : il faut renommer le workspace actif du headless.
4. Spawn sans vol de focus : règles one-shot du dispatcher exec
   (`[workspace name:… silent; noinitialfocus]`). Prouvé sans aucun effet
   sur workspace actif, fenêtre active, curseur.
5. La fenêtre-console d'un nested 0.56 a la class `aquamarine` (titre
   `aquamarine - WAYLAND-<n>`). Identification fiable : PID exact du
   process Hyprland spawné + class ; le titre seulement en confirmation.
6. Une app se lance DANS une instance via `hyprctl -i <sig> dispatch exec`
   (un spawn shell meurt en SIGHUP).
7. L'output headless créé DANS un nested reste en 0x0 : la machinerie
   output/parking v2 ne doit jamais s'exécuter à l'intérieur d'un nested.
8. `hyprctl output remove` re-centre le curseur utilisateur ; sauvegarder
   `cursorpos` juste avant et le restaurer par `dispatch movecursor` juste
   après fonctionne au pixel près.
9. Le nested laisse `$XDG_RUNTIME_DIR/hypr/<sig>/` derrière lui après
   `dispatch exit` : à nettoyer explicitement.
10. L'output headless hérite d'un scale non trivial si on ne le fixe pas
    (scale 2 observé) : résolution ET scale doivent être imposés.
11. Les workspaces nommés apparaissent immédiatement dans
    `hyprctl workspaces -j` (ids négatifs) : visibles par waybar selon sa
    config (`all-outputs`).

## 3. Architecture retenue

Mode intégré (approche A du brainstorming) : `--isolated` est une variante
de session dans le binaire actuel. Pas de namespace CLI séparé, pas de
démon. Un éventuel serveur MCP computer use sera une couche future
au-dessus de ces primitives, hors scope.

### Sessions nommées

- Flag global `--session NAME`, défaut = env `HYPRPILOT_SESSION`, sinon
  `default`. Un agent = une session = une instance nested.
- État : `$XDG_RUNTIME_DIR/hyprpilot/sessions/<name>/` contenant
  `session.json` (`schema_version: 3`), la config nested générée et le log
  du Hyprland imbriqué. Claim atomique (`create_new`) par session.
- Sessions isolées simultanées : illimitées (une par agent). Session
  partagée : une seule à la fois, quel que soit son nom (l'output partagé
  `hyprpilot` est un singleton) ; vérifié au start en parcourant les
  sessions existantes.
- Nom de session : `[a-z0-9-]{1,32}`, validé au start.

### État v3 (nouveaux champs)

`mode` (`shared` | `isolated`), `name`, et pour l'isolé : signature
d'instance, `WAYLAND_DISPLAY` du nested, PID du Hyprland nested, adresse de
la fenêtre-console côté hôte, nom de l'output headless hôte, état
show/hide. Pas de compat : toute commande refuse un état v2/legacy avec
erreur claire, sauf `teardown` qui sait encore nettoyer l'ancien
emplacement `hyprpilot/session.json` (même politique que la v2 envers le
format legacy).

### Namespace réservé

Outputs `hyprpilot*` (existant) et `hyprpilot-<session>`, workspaces
`agent-<session>` et `special:hyprpilot-parked`. Documenté dans README et
SKILL.

## 4. Séquence `session start --isolated` (recette PoC industrialisée)

```
session start --isolated --app CMD --match-title T [--match-class C]
              [--size WxH] [--session NAME]
```

1. Claim atomique du dossier session ; refus si session partagée exigée en
   parallèle (voir singleton) ou nom invalide. L'état est ensuite réécrit
   **avant** chaque mutation d'hôte et après chaque ressource acquise
   (output créé, instance spawnée, app lancée) : un start qui échoue à
   mi-chemin, ou tué net, reste toujours rattrapable par `teardown`.
   Snapshot hôte (workspace actif, fenêtre active) pris ici, avant toute
   mutation.

   Chaque mutation durable de l'hôte entre au **journal** (`host` dans la
   charge utile) avec de quoi la défaire, et y entre avant d'être posée.
   Un crash entre les deux laisse un état qui promet plus que le
   compositeur ne tient, ce que le teardown absorbe puisque chaque undo est
   idempotent ; l'ordre inverse laisserait une mutation que rien
   n'enregistre. C'est ce défaut qui a produit les deux régressions du
   2026-07-27, la barre waybar dégradée et les trente règles orphelines.
2. `hyprctl output create headless hyprpilot-<name>`, puis mode-set
   `WxH@60` (défaut 1920x1080) et `scale 1` imposés ; vérification bornée
   par `monitors -j`. La règle de mode-set est indélébile
   (hyprwm/Hyprland#5691) et inévitable : `output create headless` ne prend
   pas de taille. Elle est journalisée comme telle, avec son remède
   (`hyprctl reload`).
3. Lecture de l'`activeWorkspace.id` du headless, puis
   `dispatch renameworkspace <id> agent-<name>`, journalisé avec le nom
   d'origine — sans lui, le nom est irrécupérable et le bouton de la barre
   garde un label mort. Le workspace que Hyprland attache à un output créé
   à l'exécution est le plus petit identifiant libre, donc un slot de
   l'utilisateur : le renommage est refusé si ce workspace tenait une
   fenêtre, était visible quelque part, ou existait déjà avant la création
   de l'output.

   Recette écartée, mesurée le 2026-07-27 sur 0.56 : poser
   `agent-<name>, monitor:hyprpilot-<name>, default:true` avant la création
   de l'output ne supprime pas le renommage. La règle est bien enregistrée
   dans `workspacerules` mais n'est pas appliquée à un output créé à
   l'exécution — l'output reçoit un workspace numérique. Le renommage reste
   donc la seule recette, et son coût est assumé : pendant toute la vie de
   la session, un numéro de la barre porte un nom `agent-*`.
4. Génération de la config nested minimale dans le dossier session
   (animations off, wallpaper uni, gaps/bordures zéro, keymap hérité, pas
   d'exec-once).
5. `dispatch exec "[workspace name:agent-<name> silent; noinitialfocus;
   fullscreen] Hyprland -c <conf>"`. Découverte de la signature par diff
   borné de `$XDG_RUNTIME_DIR/hypr/` ; identification de la console par
   PID + class `aquamarine` ; vérification qu'elle est bien sur
   `agent-<name>` (sinon `movetoworkspacesilent` de rattrapage, puis échec
   si toujours ailleurs). Si la règle one-shot `fullscreen` est sans effet,
   fallback : enveloppe d'emprunt/restauration v2 pour un
   `fullscreen` ciblé, une seule fois au start.
6. Revérification du snapshot hôte de l'étape 1 : toute déviation
   (workspace actif ou fenêtre active modifiés) = échec du start avec
   cleanup complet.
7. Lancement de l'app : `hyprctl -i <sig> dispatch exec <CMD>` ; attente
   bornée du match (matching exact non ambigu existant, contre les clients
   du nested). `ready` = la fenêtre est capturable (contrat v2 conservé).

## 5. Commandes en mode isolé

Toutes les commandes lisent le mode dans l'état de leur session et routent.

- **Input, sémantique « utilisateur normal »** : le seat du nested
  appartient à l'agent. `key`/`type` : `sendshortcut` de l'instance vers la
  fenêtre cible ; `click`/`scroll` : virtual pointer créé sur le
  `WAYLAND_DISPLAY` du nested, coordonnées du layout nested (un seul
  output), vérification `cursorpos` de l'instance conservée. Aucune
  enveloppe de sauvegarde/restauration curseur/focus (aucun humain sur ce
  seat). `--focus` : accepté, no-op documenté.
- **`target`** : `focuswindow` dans le nested, sans parking ni disposition.
  `windows` : clients de l'instance, mêmes annotations.
- **`shot`** : grim sur le display du nested, framé sur la fenêtre active ;
  `--full` = bureau agent entier. TOUTE capture (isolé ET partagé) passe
  sous timeout process borné (5 s, kill après 1 s de grâce) : en isolé un
  timeout signale l'invariant cassé « workspace agent non actif sur son
  headless » et le message d'erreur le dit. Fallback documenté :
  `grim -o hyprpilot-<name>` côté hôte (contient la waybar du headless,
  acceptable en secours).
- **`wait`** : inchangé, au-dessus de ces captures.
- **`session show`** : déplace la console sur le workspace courant de
  l'utilisateur, flottante et focusable ; **`session hide`** : la renvoie
  sur `agent-<name>` (le rendu et la capture survivent au show : fenêtre
  visible = frames). En mode partagé : erreur explicite.
- **`session resize`** : non supporté en isolé dans ce cycle, erreur
  explicite (extension possible : mode-set du headless + resize console).
- **`status`** : ajoute mode, nom de session, signature, display nested,
  état show/hide. **`doctor`** : checks isolé (binaire Hyprland, version
  testée, grim, sockets, droits).
- Instance morte (crash nested) : toute commande échoue avec « instance
  morte, lancer teardown » ; `teardown` reste idempotent.

## 6. Teardown isolé

1. Fermeture de l'app par politesse si encore vivante (`dispatch exit` de
   l'instance tue tout de toute façon ; pas de dispositions restore/close :
   elles n'ont pas de sens quand le bureau entier disparaît).
2. `hyprctl -i <sig> dispatch exit`, attente bornée de la mort du PID,
   SIGTERM puis SIGKILL en derniers recours.
3. Copie du log nested dans le dossier session (ou un chemin --out), puis
   suppression de `$XDG_RUNTIME_DIR/hypr/<sig>/`.
4. Déroulé du journal d'hôte, en ordre inverse de sa pose : un workspace
   récupère son nom, un workspace évacué revient sur son moniteur, tant que
   l'output qui les a déplacés existe encore. Rendre un nom après
   `output remove` le rendrait à un workspace déjà détruit — c'est
   exactement le défaut waybar. Ce que Hyprland ne sait pas retirer (les
   `keyword`) est nommé dans les notes avec son remède au lieu d'être tu.
5. `cursorpos` sauvé, `output remove hyprpilot-<name>`, `dispatch
   movecursor` de restauration, vérification. Le retrait est décidé par la
   présence d'une entrée `output_created` au journal, jamais par un
   drapeau : un drapeau peut valoir vrai sans qu'aucune création soit
   derrière. Ceci corrige aussi, pour le mode partagé, la limite documentée
   « teardown ne restaure pas le curseur » : même mécanique appliquée aux
   deux modes.
6. Suppression du dossier session. Toute étape sur un objet déjà absent est
   un succès idempotent.

## 7. Waybar et observation

Le workspace `agent-<session>` n'ajoute pas un bouton à la barre : il en
**renomme un**. Hyprland attache au nouvel output headless le plus petit
identifiant libre, donc un slot que l'utilisateur occupe déjà dans sa
config `persistent-workspaces` ; le renommage lui donne un nom pour lequel
`format-icons` n'a pas de clé, et la barre tombe sur `default`. Le symptôme
observé le 2026-07-27 est donc la **disparition du numéro confisqué**, pas
l'apparition d'un workspace agent.

Le teardown rend le nom avant de retirer l'output, donc la barre est
entière dès la fin de la session. Entre les deux, la dégradation est le
coût assumé de la recette (§4.3). Un clic sur ce workspace bascule le focus
vers l'output headless invisible : observation = `session show`/`hide` ; le
module waybar custom éventuel est hors scope.

## 8. Portals : slice exploratoire timeboxée

Risque connu : xdg-desktop-portal-hyprland se lie à une instance via l'env
D-Bus ; les file pickers dans le nested peuvent viser le mauvais
compositeur. Slice d'exploration EN TÊTE de plan, timeboxée, critère
binaire : un file picker GTK s'ouvre et fonctionne dans le nested. Si oui,
la recette entre dans la config générée ; si non, limite documentée +
piste consignée (portal dédié par instance via bus de session privé). Ne
bloque aucune autre slice.

## 9. Erreurs et garde-fous

- Fail fast partout ; settle borné après chaque mutation compositeur,
  jamais de sleep aveugle ; échecs avec contexte (les deux erreurs si une
  action ET son rattrapage échouent, comme en v2).
- Le warning nested « started without start-hyprland » observé au PoC est
  consigné : à qualifier pendant l'implémentation (bloquant ou cosmétique).
- Concurrence intra-session toujours hors contrat (le fichier d'état n'est
  pas un verrou) ; la concurrence INTER-sessions est, elle, garantie par
  l'isolation des dossiers et des instances.

## 10. Validation

- **G1** : tests unitaires (état v3, claim par nom, refus v2, génération
  config nested, parsing CLI, routage par mode).
- **G2** (`scripts/hyprland-gate.sh`, session vivante) :
  1. Isolé nominal : start → app → type/key/click/scroll/shot/wait →
     show/hide → teardown.
  2. Hôte intact : snapshots hôte (workspace actif, fenêtre active,
     curseur, liste des workspaces, monitors) identiques avant/après le
     scénario complet.
  3. Deux sessions isolées parallèles : actions croisées, teardowns
     indépendants, zéro fuite d'état.
  4. Portal : rejouer le critère binaire de la slice d'exploration.
  5. Régression : les 10 scénarios partagés existants restent verts après
     la migration d'état.
  6. Curseur restauré après chaque teardown (les deux modes).
- Les gates sont toujours lancés par l'orchestrateur lui-même.

## 11. Hors scope de ce cycle

- Serveur/démon/MCP computer use (couche future au-dessus du CLI).
- Audio dans le nested.
- `hyprland-toplevel-export-v1` (piste captures lock-proof du mode
  partagé ; indépendante).
- `session resize` en isolé.
- Module waybar custom pour les bureaux agents.
- Multi-fenêtres AVANCÉ dans le nested au-delà du matching existant
  (le matching non ambigu et `target` suffisent pour dialogues/popups).

## 12. Critères de succès du cycle

1. Un agent muni de `HYPRPILOT_SESSION=<x>` pilote un bureau complet
   (app, saisie, clics, captures fiables) sans qu'aucun pixel, focus ou
   curseur du bureau utilisateur ne bouge.
2. Deux agents simultanés sans interférence mutuelle ni avec l'utilisateur.
3. Le mode partagé reste intact (gate de régression vert) et le README +
   SKILL documentent le routage : partagé = piloter MES fenêtres, isolé =
   bureau agent.

## 13. Écarts entre ce design et ce qui a été livré

Ajouté après la livraison du cycle, pour qu'une lecture de ce document
n'attribue pas au code des propriétés qu'il n'a pas.

- **Log nested (§6.3)** : le log est bien copié dans `sessions/<name>/`, mais
  l'étape 5 supprime ce dossier — après un `teardown` réussi il ne reste donc
  rien. `--out` n'a pas été implémenté. Le log survit là où il sert vraiment :
  le rollback d'un start raté le conserve et en imprime la fin.
- **Portails (§8)** : la slice a été jouée, verdict dans
  `crates/hyprpilot/references/portal-probe.md`. Avec l'environnement gelé
  (bus D-Bus hérité de l'hôte) : bloqué, donc limite documentée. La piste du
  bus de session privé a été validée en laboratoire dans le même probe et
  reste hors périmètre : le cycle de vie §4/§6 ne connaît pas de bus.
- **Namespace (§7)** : `hyprpilot*` comme préfixe réservé est arrivé avec ce
  cycle. La v2 ne réservait que le nom exact `hyprpilot`, seul output que son
  sweep connaissait.
- **Identité de fenêtre (schema v4)** : une fenêtre suivie est enregistrée avec
  son `stableId` en plus de son adresse, parce que Hyprland réutilise les
  adresses. Le design ne parlait que d'adresses ; le mode isolé n'en bénéficie
  pas encore (TODO dans `isolated.rs`).
