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
   après chaque ressource compositeur acquise (output créé, instance
   spawnée, app lancée) : un start qui échoue à mi-chemin reste toujours
   rattrapable par `teardown`, comme en v2. Snapshot hôte (workspace
   actif, fenêtre active) pris ici, avant toute mutation.
2. `hyprctl output create headless hyprpilot-<name>`, puis mode-set
   `WxH@60` (défaut 1920x1080) et `scale 1` imposés ; vérification bornée
   par `monitors -j`.
3. Lecture de l'`activeWorkspace.id` du headless, puis
   `dispatch renameworkspace <id> agent-<name>`.
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
4. `cursorpos` sauvé, `output remove hyprpilot-<name>`, `dispatch
   movecursor` de restauration, vérification. Ceci corrige aussi, pour le
   mode partagé, la limite documentée « teardown ne restaure pas le
   curseur » : même mécanique appliquée aux deux modes.
5. Suppression du dossier session. Toute étape sur un objet déjà absent est
   un succès idempotent.

## 7. Waybar et observation

Les workspaces `agent-<session>` sont visibles dans `hyprctl workspaces -j`
dès le spawn. Selon la config waybar (`all-outputs`), ils apparaissent dans
la barre : c'est un indicateur de présence des agents, PAS un bouton (un
clic waybar bascule le focus vers l'output headless invisible). Documenter
dans SKILL/README : observation = `session show`/`hide` ; le module waybar
custom éventuel est hors scope.

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
