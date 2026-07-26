# Plan d'orchestration par slices — hyprpilot `--isolated`

Date : 2026-07-26

Découpage du cycle « bureaux agents isolés » sur la spec gelée
[`docs/superpowers/specs/2026-07-24-hyprpilot-isolated-design.md`](../../docs/superpowers/specs/2026-07-24-hyprpilot-isolated-design.md)
(sections 3 à 6 et 10). La spec reste la référence comportementale : chaque
slice cite sa section plutôt que de la paraphraser. Format hérité du plan du
cycle v2, `crates/hyprpilot/hyprpilot-slice-plan.md` (run terminé, fichier non
versionné à ce jour).

## Paramètres du run

- **Repo cible** : `/home/b3ngous/projects/agents-skills` (crate
  `crates/hyprpilot`).
- **Branche** : `feature/hyprpilot-isolated-foundation`, créée en S1 depuis la
  pointe de `feature/hyprpilot-session-v2`. Le cycle v2 n'est pas poussé : ce
  cycle s'empile dessus, rebase plutôt que merge.
- **Exécutant par défaut** : Codex `gpt-5.6-sol`, effort `xhigh`, via
  `/codex-orchestrate`. Claude = architecte, QA, committeur ; Codex ne commite
  ni ne conçoit. S1, les checkpoints et S12 sont tenus par l'orchestrateur.
- **Gates cargo (G1)**, lancés par l'orchestrateur après chaque slice, jamais
  sur la foi du vert annoncé par l'exécutant :

  ```bash
  cargo fmt --all --check
  cargo clippy -p hyprpilot --all-targets -- -D warnings
  cargo test -p hyprpilot --locked
  cargo build -p hyprpilot --locked
  ```

- **Gate Hyprland (G2)** : `scripts/hyprland-gate.sh <scénario>|all` sur session
  Hyprland réelle, **exécuté exclusivement par l'orchestrateur**. Chaque slice
  qui change un comportement observable ajoute son scénario au script ; le
  scénario tourne au checkpoint qui suit.
- **Interdits globaux** (à répéter dans chaque prompt de slice) : pas de commit
  git, aucune nouvelle dépendance, aucun fichier hors périmètre listé, ne pas
  exécuter `scripts/hyprland-gate.sh`, ne pas modifier `../../hyprpilot/SKILL.md`
  (S1 y pose le namespace et la limite portals, S12 consolide le reste), ne
  supprimer aucun test existant, ne jamais faire tomber une commande du mode
  isolé dans le chemin partagé.
- **Échec** : gate rouge → diagnostiquer code vs environnement ; échec de code →
  `exec resume` du thread avec le rapport exact ; deux échecs sur la même slice
  → stop et rapport.

## Colonne « session vivante »

`non` = validable hors session Hyprland, G1 suffit. `oui` = la slice mute un
compositeur réel, donc elle n'est validée qu'au checkpoint G2 suivant. Les
slices marquées `oui` peuvent malgré tout livrer des fonctions pures testées
sous G1 ; c'est la validation de bout en bout qui exige la session.

## Séquence

| # | Contenu | Spec | Exécutant | Gate de sortie | Session vivante |
|---|---------|------|-----------|----------------|-----------------|
| S0 | Exploration portals, timeboxée | §8 | orchestrateur | verdict binaire écrit | oui (**joué**, `d5369d4`) |
| S1 | Fondation : état v3, sessions nommées, routage par mode | §3 | orchestrateur | G1 | non |
| S2 | Génération de la config nested | §4.4 | Codex sol | G1 | non |
| S3 | Output headless agent + workspace `agent-<name>` | §4.2, §4.3 | Codex sol | G1 | oui |
| CP-A | G2 : output créé/retiré, workspace renommé, hôte intact | §10.2 | orchestrateur | G2 | oui |
| S4 | Spawn de l'instance nested, découverte, snapshot hôte | §4.1, §4.5, §4.6 | Codex sol | G1 | oui |
| S5 | Teardown isolé + curseur restauré dans les deux modes | §6 | Codex sol | G1 | oui |
| CP-B | G2 : spawn puis teardown, zéro résidu | §10.1, §10.6 | orchestrateur | G2 | oui |
| S6 | Lancement de l'app dans l'instance, contrat `ready` | §4.7 | Codex sol | G1 | oui |
| S7 | Input isolé : `key`/`type`/`click`/`scroll`, `--focus` no-op | §5 | Codex sol | G1 | oui |
| S8 | Captures isolées + timeout borné de toute capture | §5 | Codex sol | G1 | oui |
| CP-C | G2 : isolé nominal de bout en bout | §10.1 | orchestrateur | G2 | oui |
| S9 | `target` et `windows` isolés | §5 | Codex sol | G1 | oui |
| S10 | `session show` / `session hide` | §5 | Codex sol | G1 | oui |
| S11 | `status` et `doctor` isolés | §5 | Codex sol | G1 | oui |
| CP-D | G2 `all` : nominal, hôte intact, deux sessions parallèles, régression partagée | §10 | orchestrateur | G2 complet | oui |
| S12 | Docs README + SKILL, sweep final | §3, §7 | orchestrateur | G1 + G2 `all` | non |

Un blocage transversal découvert en route donne une slice insérée, jamais
fusionnée dans la slice courante.

---

## S0 — Exploration portals (§8) — JOUÉ

Verdict **UNSUPPORTED**, consigné dans
[`references/portal-probe.md`](references/portal-probe.md) (commit `d5369d4`).
L'app lancée dans le nested hérite du `DBUS_SESSION_BUS_ADDRESS` de l'hôte :
tout `FileChooser` portal reste bloqué, sans dialogue nulle part.
`GTK_USE_PORTAL=0` ne contourne rien (GTK4 n'a plus que ce chemin). Aucune
recette portal n'entre donc dans la config générée en S2 ; la limite est
documentée en S12. La piste « bus de session privé par instance » a été validée
empiriquement mais reste hors contrat gelé (voir Décisions).

## S1 — Fondation d'état et de routage (orchestrateur)

- **Objectif** : spec §3. Tout ce qui se teste hors session vivante et dont
  dépendent S2 à S11.
- **Interface imposée** :

  ```rust
  pub struct Session { pub schema_version: u32, pub name: String, pub state: ModeState }
  pub enum ModeState { Shared(Shared), Isolated(Isolated) }   // serde tag "mode", aplati
  pub struct Shared { /* champs v2 sans schema_version */ }
  pub struct Isolated { output, workspace, size, shown, instance: Instance }
  pub enum Instance { Pending, Live { signature, wayland_display, pid, console_address } }
  ```

  - État en `$XDG_RUNTIME_DIR/hyprpilot/sessions/<name>/session.json`,
    `schema_version: 3`, claim atomique `create_new` par nom.
  - Nom de session validé `[a-z0-9-]{1,32}` à **chaque** commande, pas
    seulement au start : le nom entre dans un chemin.
  - Drapeau global `--session NAME`, repli `HYPRPILOT_SESSION` puis `default`.
  - Singleton partagé : une seconde session `mode: shared` est refusée quel que
    soit son nom, par parcours de `sessions/*/session.json` au start.
  - Refus explicite de tout état v2 ou non versionné (version trouvée, version
    attendue, commande de sortie), y compris quand il traîne à l'ancien
    emplacement `hyprpilot/session.json` ; `teardown` seul sait encore le lire
    et le nettoyer.
  - `--isolated` sur `session start` ; toute commande en mode isolé échoue en
    nommant sa slice, avant toute mutation du compositeur.
  - `session resize` en isolé : erreur « non supporté dans ce cycle » (§11),
    pas un renvoi vers une slice.
- **Fichiers** : `src/session.rs`, `src/cli.rs`, `src/error.rs`,
  `src/capture.rs`, `src/keys.rs`, `src/pointer.rs`, `README.md`,
  `../../hyprpilot/SKILL.md`.
- **Tests exigés (G1)** : round-trip v3 partagé et isolé (les deux stades
  d'instance) ; claim par nom ; refus v2 et non versionné ; validation et
  résolution du nom ; singleton partagé ; parsing CLI (`--session` avant et
  après la sous-commande, `--isolated`) ; routage par mode nommant la slice.
- **DoD** : G1 ; les 71 tests v2 restent verts (adaptés là où la version 3 les
  rend faux, jamais supprimés).
- **Commit** : `feat(hyprpilot): état v3, sessions nommées et routage par mode`

## S2 — Config nested générée (Codex, sol)

- **Objectif** : spec §4.4. Écrire la config minimale du Hyprland imbriqué dans
  le dossier de session, en fonction pure testable hors session.
- **Interface imposée** :
  - `fn nested_config(name: &str, size: [u32; 2]) -> String` puis écriture dans
    `sessions/<name>/hyprland.conf`.
  - Contenu : animations off, wallpaper uni, gaps et bordures à zéro, keymap
    hérité de l'hôte (lu par `hyprctl devices`, pas deviné), aucun `exec-once`,
    aucune règle portal (S0 = UNSUPPORTED).
  - Chemin du log nested dans le même dossier ; la config ne référence aucun
    chemin absolu de la machine hors `$XDG_RUNTIME_DIR`.
- **Points délicats** : le keymap hérité est le seul champ dynamique ; une
  lecture qui échoue est une erreur, pas un repli sur `us`.
- **Fichiers** : `src/session.rs` (ou `src/nested.rs` si la taille le justifie),
  `src/hypr.rs` si `devices` doit exposer un champ de plus.
- **Tests exigés (G1)** : contenu généré (assertions par ligne attendue, pas
  comparaison d'un blob) ; écriture dans un `tempdir` ; erreur si le keymap est
  introuvable.
- **Commit** : `feat(hyprpilot): config nested générée par session`

## S3 — Output headless agent et workspace `agent-<name>` (Codex, sol)

- **Objectif** : spec §4.2 et §4.3. Session vivante : oui.
- **Interface imposée** :
  - `hyprctl output create headless hyprpilot-<name>`, puis mode-set
    `WxH@60` (défaut 1920x1080) **et** `scale 1` imposés (fait §2.10 : le scale
    est hérité sinon), vérification bornée par `monitors -j`.
  - Lecture de l'`activeWorkspace.id` du headless puis
    `dispatch renameworkspace <id> agent-<name>`. Jamais
    `moveworkspacetomonitor` (fait §2.3 : le workspace y resterait inactif).
  - Refus d'exécuter quoi que ce soit de cette machinerie **à l'intérieur** d'un
    nested (fait §2.7 : l'output y reste en 0x0). Mécanisme retenu : marqueur
    d'environnement posé au spawn en S4 et vérifié ici.
  - Output `hyprpilot-<name>` préexistant = échec du start, avec la commande de
    sortie (`hyprpilot --session <name> teardown`) : c'est un résidu, pas une
    ressource à réutiliser.
  - État réécrit dès l'output acquis (§4.1) : un start interrompu ici reste
    rattrapable par `teardown`.
- **Fichiers** : `src/session.rs`, `src/hypr.rs`, `src/error.rs`.
- **Tests exigés (G1)** : construction de la règle de moniteur (résolution,
  taux, scale) ; détection du marqueur nested ; refus de l'output préexistant
  sur fixture `monitors.json`.
- **Ajout gate** : `scenario_isolated_output` (créer, asserter nom, taille,
  scale 1 et workspace `agent-<name>` actif, retirer, asserter l'absence).
  Nettoyage par trap `hyprctl` brut, sans dépendre de S5.
- **Commit** : `feat(hyprpilot): output headless et workspace par bureau agent`

## CP-A — Checkpoint : output et workspace (orchestrateur)

`scenario_isolated_output` vert. Snapshots hôte identiques avant/après
(workspace actif, fenêtre active, curseur, liste des workspaces, monitors).

## S4 — Spawn de l'instance nested et découverte (Codex, sol)

- **Objectif** : spec §4.1, §4.5, §4.6. Session vivante : oui. Grosse slice :
  délégation interne autorisée pour les sous-parties mécaniques.
- **Interface imposée** :
  - Snapshot hôte (workspace actif, fenêtre active) pris **avant** toute
    mutation, revérifié après le spawn ; toute déviation = échec du start avec
    cleanup complet (§4.6).
  - `dispatch exec "[workspace name:agent-<name> silent; noinitialfocus;
    fullscreen] Hyprland -c <conf>"` (fait §2.4 : les règles one-shot ne volent
    pas le focus). Si la règle `fullscreen` reste sans effet, fallback documenté
    de la spec §4.5, une seule fois au start.
  - Découverte de la signature par diff borné de `$XDG_RUNTIME_DIR/hypr/`.
    Identification de la console par **PID exact + class `aquamarine`**, titre
    en confirmation seulement (fait §2.5).
  - Vérification que la console est sur `agent-<name>` ; sinon
    `movetoworkspacesilent` de rattrapage, puis échec si toujours ailleurs.
  - Marqueur d'environnement du bureau agent posé ici, consommé par S3.
  - État réécrit après chaque ressource acquise ; `Instance::Pending` devient
    `Instance::Live` en une seule écriture.
- **Points délicats** : le diff de signatures doit être borné et tolérer des
  instances tierces ; un spawn shell meurt en SIGHUP (fait §2.6), donc l'app de
  S6 passe obligatoirement par `hyprctl -i <sig> dispatch exec`.
- **Fichiers** : `src/session.rs`, `src/hypr.rs`, `src/error.rs`.
- **Tests exigés (G1)** : diff de signatures sur répertoires temporaires
  (aucune, une, plusieurs nouvelles) ; sélection de la console par PID + class
  sur fixture de clients (dont un leurre `aquamarine` d'un autre PID) ;
  comparaison de snapshots hôte.
- **Ajout gate** : `scenario_isolated_spawn` (spawn, asserter instance vivante,
  console sur `agent-<name>`, hôte intact ; trap `hyprctl` brut).
- **Question ouverte à clore ici** : le warning nested « started without
  start-hyprland » est déjà qualifié cosmétique par le probe S0, mais il
  s'affiche en bandeau, donc dans les captures `--full`. Consigner s'il peut
  être supprimé par la config générée (S2) ou s'il reste une limite documentée.
- **Commit** : `feat(hyprpilot): spawn et découverte de l'instance nested`

## S5 — Teardown isolé et restauration du curseur (Codex, sol)

- **Objectif** : spec §6. Session vivante : oui. Placée avant l'app pour que
  les scénarios suivants disposent d'un vrai nettoyage.
- **Interface imposée** :
  - Fermeture de l'app par politesse si vivante, puis
    `hyprctl -i <sig> dispatch exit`, attente bornée de la mort du PID, SIGTERM
    puis SIGKILL en derniers recours.
  - Copie du log nested dans le dossier de session (ou `--out`), puis
    suppression de `$XDG_RUNTIME_DIR/hypr/<sig>/` (fait §2.9).
  - `cursorpos` sauvé, `output remove hyprpilot-<name>`, `dispatch movecursor`
    de restauration, vérification (fait §2.8). **La même mécanique s'applique au
    mode partagé** : elle lève la limite documentée « teardown ne restaure pas
    le curseur ».
  - Suppression du dossier de session. Toute étape sur un objet déjà absent est
    un succès idempotent. Un état `Instance::Pending` se nettoie aussi.
  - Aucune disposition restore/close en isolé : le bureau entier disparaît.
- **Fichiers** : `src/session.rs`, `src/error.rs`, `src/cli.rs`.
- **Tests exigés (G1)** : plan de teardown isolé par stade d'instance
  (`Pending`, `Live`, PID déjà mort) ; idempotence sur objets absents ;
  tolérance ±1 px de la vérification curseur (constante existante de
  `pointer.rs`, jamais « exact »).
- **Ajout gate** : `scenario_isolated_teardown` (start S4 puis teardown :
  output absent, instance morte, `hypr/<sig>/` supprimé, dossier de session
  supprimé, curseur restauré) ; `scenario_shared_teardown_cursor` (même
  assertion curseur sur le mode partagé).
- **Commit** : `feat(hyprpilot): teardown isolé et curseur restauré`

## CP-B — Checkpoint : spawn puis teardown (orchestrateur)

`scenario_isolated_output`, `scenario_isolated_spawn`,
`scenario_isolated_teardown`, `scenario_shared_teardown_cursor` verts. Zéro
résidu : ni output, ni instance, ni dossier de session, ni `hypr/<sig>/`.

## S6 — App dans l'instance et contrat `ready` (Codex, sol)

- **Objectif** : spec §4.7. Session vivante : oui.
- **Interface imposée** :
  - `hyprctl -i <sig> dispatch exec <CMD>` (fait §2.6), attente bornée du match
    contre les **clients de l'instance**, avec le matcher exact non ambigu
    existant (`session::resolve`).
  - `ready` = la fenêtre est capturable, contrat v2 conservé. En isolé cela
    suppose l'invariant « workspace agent actif sur son headless ».
  - Échec du lancement = cleanup complet du start, comme en S4.
- **Fichiers** : `src/session.rs`, `src/hypr.rs`.
- **Tests exigés (G1)** : matching sur fixture de clients d'instance (0, 1, n) ;
  message d'ambiguïté avec le tableau JSON des candidats en dernière ligne.
- **Ajout gate** : `scenario_isolated_app` (start complet avec `--app zenity`,
  asserter la fenêtre dans l'instance et l'hôte intact).
- **Commit** : `feat(hyprpilot): lancement de l'app dans le bureau agent`

## S7 — Input isolé (Codex, sol)

- **Objectif** : spec §5, volet input. Session vivante : oui.
- **Interface imposée** :
  - `key`/`type` : `sendshortcut` de l'instance vers la fenêtre cible.
  - `click`/`scroll` : virtual pointer créé sur le `WAYLAND_DISPLAY` du nested,
    coordonnées du layout nested (un seul output), vérification `cursorpos` de
    l'instance conservée.
  - **Aucune enveloppe guard** (sauvegarde/restauration curseur ou focus) :
    aucun humain sur ce seat. `guard.rs` n'est pas touché.
  - `--focus` accepté et no-op documenté.
- **Fichiers** : `src/keys.rs`, `src/pointer.rs`, `src/session.rs`.
- **Tests exigés (G1)** : construction des commandes `hyprctl -i <sig>` ;
  sélection du socket Wayland du nested ; `--focus` sans effet sur le plan
  d'action.
- **Ajout gate** : `scenario_isolated_input` (type + key + click + scroll dans
  le nested, valeur relue dans l'app, hôte intact et curseur hôte immobile).
- **Commit** : `feat(hyprpilot): input isolé sur le seat du bureau agent`

## S8 — Captures isolées et timeout borné (Codex, sol)

- **Objectif** : spec §5, volets `shot` et `wait`. Session vivante : oui.
- **Interface imposée** :
  - `shot` : grim sur le display du nested, framé sur la fenêtre active ;
    `--full` = bureau agent entier.
  - **Toute** capture, isolé et partagé, passe sous timeout process borné (5 s,
    kill après 1 s de grâce). En isolé, un timeout signale l'invariant cassé
    « workspace agent non actif sur son headless » (fait §2.2) et le message le
    dit. Cela lève la limite v2 « le timeout borne la boucle, pas un grim
    bloqué ».
  - Fallback documenté : `grim -o hyprpilot-<name>` côté hôte, qui contient la
    waybar du headless.
  - `wait` inchangé au-dessus de ces captures ; scratch et dossier de shots
    déplacés sous `sessions/<name>/` (voir Décisions).
- **Fichiers** : `src/capture.rs`, `src/session.rs`, `src/error.rs`.
- **Tests exigés (G1)** : le wrapper de timeout (process qui sort, qui traîne,
  qui ignore SIGTERM) ; résolution du dossier de sortie par session ; message
  d'erreur nommant l'invariant.
- **Ajout gate** : `scenario_isolated_shot` (capture non vide de la fenêtre
  puis `--full`) ; `scenario_capture_timeout` si un blocage se provoque
  proprement, sinon limite consignée.
- **Commit** : `feat(hyprpilot): captures isolées sous timeout borné`

## CP-C — Checkpoint : isolé nominal (orchestrateur)

Chaîne complète start → app → type/key/click/scroll → shot → wait → teardown
(spec §10.1), plus snapshots hôte identiques avant/après (§10.2).

## S9 — `target` et `windows` isolés (Codex, sol)

- **Objectif** : spec §5. Session vivante : oui.
- **Interface imposée** : `target` = `focuswindow` dans le nested, **sans
  parking ni disposition** (le special workspace et les dispositions restent au
  mode partagé) ; `windows` = clients de l'instance, mêmes annotations que
  partagé. Le drapeau `--on-teardown` est refusé en isolé.
- **Fichiers** : `src/session.rs`, `src/cli.rs`.
- **Tests exigés (G1)** : résolution de cible sur fixture de clients
  d'instance ; refus de `--on-teardown` ; sérialisation de `windows` en mode
  isolé.
- **Ajout gate** : `scenario_isolated_target` (app ouvrant un second toplevel,
  bascule, captures distinctes).
- **Commit** : `feat(hyprpilot): target et windows dans le bureau agent`

## S10 — `session show` / `session hide` (Codex, sol)

- **Objectif** : spec §5. Session vivante : oui.
- **Interface imposée** : `show` déplace la console sur le workspace courant de
  l'utilisateur, flottante et focusable ; `hide` la renvoie sur `agent-<name>`.
  Rendu et captures survivent au `show` (fenêtre visible = frames). L'état
  `shown` est persisté. En mode partagé : erreur explicite.
- **Fichiers** : `src/cli.rs`, `src/session.rs`.
- **Tests exigés (G1)** : parsing CLI ; refus en mode partagé ; transition de
  l'état `shown`.
- **Ajout gate** : `scenario_isolated_show_hide` (show, capture toujours
  valide, hide, hôte revenu à son état).
- **Commit** : `feat(hyprpilot): session show et hide du bureau agent`

## S11 — `status` et `doctor` isolés (Codex, sol)

- **Objectif** : spec §5. Session vivante : oui pour `doctor`, non pour la
  sérialisation de `status`.
- **Interface imposée** : `status` ajoute mode, nom de session, signature,
  display nested, état show/hide, et lit la géométrie dans l'instance et non
  sur l'hôte. `doctor` ajoute les checks isolé (binaire Hyprland, version
  testée, grim, sockets, droits). Instance morte : toute commande échoue avec
  « instance morte, lancer teardown ».
- **Fichiers** : `src/cli.rs`, `src/session.rs`.
- **Tests exigés (G1)** : sérialisation de `status` isolé (clés exactes) ;
  détection d'instance morte ; rapport `doctor` sans session.
- **Ajout gate** : `scenario_isolated_status` (status pendant une session,
  puis après un kill brutal du nested : erreur « instance morte »).
- **Commit** : `feat(hyprpilot): status et doctor du mode isolé`

## CP-D — Checkpoint : G2 complet (orchestrateur)

`scripts/hyprland-gate.sh all` tout vert, dont les six volets de la spec §10 :

1. Isolé nominal (couvert CP-C).
2. Hôte intact sur le scénario complet.
3. **Deux sessions isolées parallèles** : actions croisées, teardowns
   indépendants, zéro fuite d'état. Scénario ajouté ici, pas plus tôt : il
   suppose input, captures et teardown livrés.
4. Portal : pas de scénario. S0 a tranché UNSUPPORTED et le mode d'échec est un
   blocage indéfini du picker, donc un scénario l'asserterait par un timeout de
   plusieurs secondes sans rien apprendre. Limite documentée en S12.
5. Régression : les 10 scénarios partagés existants verts après la migration
   d'état.
6. Curseur restauré après chaque teardown, dans les deux modes (couvert CP-B,
   rejoué ici).

## S12 — Docs et clôture (orchestrateur)

- `README.md` : mode isolé, sessions nommées, namespace réservé, contrat
  `ready` en isolé, teardown isolé, limite portals, limites levées (curseur
  restauré, capture sous timeout).
- `../../hyprpilot/SKILL.md` : routage « partagé = piloter MES fenêtres, isolé =
  bureau agent », boucle canonique isolée, `HYPRPILOT_SESSION`, observation par
  `session show`/`hide` et non par clic waybar (spec §7).
- Sweep final : G1, G2 `all`, relecture du `git log` de la branche, rapport.

## Décisions en cours de run

- **Namespace réservé** (spec §3) : outputs `hyprpilot` (partagé, singleton) et
  `hyprpilot-<session>` (isolé) ; workspaces `agent-<session>` et
  `special:hyprpilot-parked` (partagé seulement). Documenté en S1 dans le
  README et le SKILL, complété en S12.
- **Scratch et shots par session** (S8) : `wait` écrit aujourd'hui
  `wait-a.png` / `wait-b.png` dans `$XDG_RUNTIME_DIR/hyprpilot/`, et `shot`
  numérote dans `hyprpilot/shots/`. Deux sessions isolées parallèles se
  marcheraient dessus. Non corrigé en S1 : le singleton partagé et le routage
  « non implémenté » rendent la collision impossible avant S8, et le
  déplacement sous `sessions/<name>/` appartient à la slice qui touche les
  captures.
- **Piste bus D-Bus privé par instance** : validée empiriquement par S0 (le
  picker s'ouvre dans le bureau agent, sélection et code retour corrects) mais
  **hors contrat gelé** : §4 et §6 ne connaissent pas de ressource « bus », il
  faudrait l'ajouter au cycle de vie et au teardown (identification des
  process par `DBUS_SESSION_BUS_ADDRESS` dans `/proc/<pid>/environ`, jamais par
  nom de binaire). Contrainte pratique relevée : `sun_path` est limité à 108
  octets et `$XDG_RUNTIME_DIR/hyprpilot/sessions/<name>/bus.sock` passe. À
  proposer comme cycle suivant, pas à glisser dans celui-ci.
- **Triage du premier passage du gate** (écrit par l'auteur des scénarios, qui
  n'a pas pu les exécuter). À lire AVANT de conclure qu'un rouge est un défaut
  du crate :
  1. `isolated_start_match_failure` est le rouge le plus probable. Sa preuve
     d'ordre lit le socket d'évènements de Hyprland, dont les noms exacts
     (`monitorremoved`, `closewindow>>ADDR`, adresses sans `0x`) n'ont pas pu
     être vérifiés à froid. Discriminant : un FAIL « monitorremoved sans
     closewindow » accuse d'abord le format d'évènement ; un FAIL qui cite deux
     numéros de ligne accuse le rollback, et c'est alors le vrai défaut.
     La trace vit dans `$XDG_RUNTIME_DIR/hyprpilot-e2e-iso-nomatch.*/events.log`,
     que le trap supprime : jouer le scénario seul pour la garder.
  2. `isolated_show_occluded` : sensible à `input:follow_mouse=1` (la console
     flottante peut couvrir le point où le curseur a été posé, donc voler le
     focus hôte) et au GC des workspaces nommés vides. FAIL sur `elapsed` =
     crate ; FAIL sur le message ou le focus = scénario ou config.
  3. `isolated_start_concurrent` : compare strictement les listes de fenêtres et
     de workspaces hôte sur ~40 s, donc exposé à toute fenêtre qui s'ouvre
     seule pendant le run. FAIL « fenêtres hôte apparues » = parasite, rejouer.
  4. `isolated_start_bad_size` suppose que Hyprland refuse `99999x99999` au
     mode-set. S'il l'accepte, le scénario ne teste plus le défaut visé et il
     faut un autre déclencheur.
  5. Fragilité transverse la plus probable de toutes : la comparaison stricte
     des snapshots hôte, sur les runs longs.
  Enfin, six scénarios isolés (`spawn`, `app`, `shot`, `show_hide`,
  `host_intact`, `parallel`) n'avaient jamais pu atteindre leurs assertions
  avant le durcissement (`read_host_snapshot` et `assert_png_dimensions`
  avortaient le sous-shell sur `unbound variable`) : un rouge chez eux est
  probablement un défaut du crate observé pour la première fois.
- **Trou trouvé par ce triage et corrigé** : libwayland ne délie son socket
  qu'à une sortie propre, donc un nested tué par SIGKILL laissait `wayland-<n>`
  et son `.lock` derrière lui, là où le nettoyage ne traitait que
  `hypr/<sig>/`. Le teardown les délie désormais, en refusant de délier un
  socket sur lequel quelque chose écoute encore : le nom enregistré est le
  nôtre, mais il a pu être repris entre le kill et le nettoyage, et délier
  celui-là couperait ses clients.
- **Résidu connu, documenté plutôt que corrigé** : si le process hyprpilot est
  tué brutalement entre le spawn du nested et la persistance de son stade
  `Live`, l'état reste `Pending` sans signature. Le nested lui-même est
  rattrapable (le nonce d'instance vit dans `Isolated`, donc le balayage du
  teardown le trouve), mais `$XDG_RUNTIME_DIR/hypr/<sig>/` ne peut plus être
  nommé : litière inerte en tmpfs, effacée au reboot. Le fermer proprement
  demanderait un troisième stade d'`Instance`, donc une forme de schéma de plus
  pour un cas qui n'arrive que sur mort brutale de l'outil : disproportionné.
- **Effet de bord à surveiller** (S0) : sur un run du probe, un client GTK4 de
  l'hôte a SIGSEGV à la milliseconde du `output create headless`, dans
  `libgtk-4.so.1`. Non systématique, rien ne le rattache à un défaut de la
  recette. CP-A et CP-D asserteront la liste des adresses hôte avant/après ;
  une récidive devient une slice d'enquête, pas un correctif improvisé.
- **Exécution réelle du run** : l'orchestrateur est un agent Opus 5 ; les
  slices sont déléguées à des sous-agents Opus (décision Augustin du
  2026-07-24 : plus de wrapper CLI Codex, Opus 5 partout), l'orchestrateur
  garde les gates et les commits. La plomberie `hypr::Ctl::{Host, Instance}`
  (un seul `Command::new("hyprctl")` dans la crate, préfixe `-i <sig>` pour
  une instance) a été posée par l'orchestrateur avant S2, puisque toutes les
  slices isolées en dépendent.
- **Suivi des slices** : S0 ✅ (`d5369d4`, UNSUPPORTED) · S1 ✅ (`f2b6f7d` +
  `12bbf53`, G1 81 tests) · S2 S3 S4 S6 ✅ (`3593d9d`, 96 tests) · S5 S8 ✅
  (`5f55739`) · S7 ✅ (`1b721cd`) · S9 S10 S11 ✅ (`2793fcb`, 142 tests) ·
  S12 ✅ (`8449c81`) · scénarios G2 ✅ (`40a1bda`) · **CP-A à CP-D non joués**.
- **Écart d'exécution assumé** : les slices ont été livrées en quatre vagues
  d'agents en parallèle sur fichiers disjoints, pas une par une avec son
  checkpoint. Les gates G1 ont tourné après chaque vague, mais les checkpoints
  G2 n'ont pas pu s'intercaler : le classifieur de permissions de
  l'orchestrateur refuse l'exécution de `scripts/hyprland-gate.sh`, qui pilote
  le compositeur vivant. Conséquence à retenir avant toute confiance dans ce
  code : **aucune ligne du mode isolé n'a jamais tourné contre un compositeur
  réel**. Les 29 scénarios existent, aucun n'a été joué.
- **Round de revue adversariale** (en remplacement partiel du G2 manquant) :
  16 défauts confirmés sur le diff des quatre vagues, dont 5 faisant atterrir
  la console d'un bureau agent sur le bureau utilisateur (rollback et teardown
  retirant l'output avant le reap de la console, échec de mode-set laissant un
  output orphelin sans état, balayage par variable d'environnement héritable
  tuant le shell appelant, capture faisant confiance au drapeau `shown` au lieu
  d'observer la composition). Corrigés dans un commit dédié, avec les scénarios
  d'échec de start qui les rendent observables — le gate n'en avait aucun.
