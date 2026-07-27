# Portal probe — file picker GTK dans un bureau agent (nested)

Slice exploratoire de la spec `--isolated` (§8), rejouée sur session vivante
le 2026-07-24. Critère binaire : un file picker GTK sous portal s'ouvre et
fonctionne dans le nested, avec la recette de la spec (§2).

Verdict: UNSUPPORTED

La recette gelée lance l'app dans le nested avec l'environnement hérité de
l'hôte : `WAYLAND_DISPLAY` pointe sur le nested mais `DBUS_SESSION_BUS_ADDRESS`
reste le bus de session de l'utilisateur. Sous cet environnement, le file
picker ne rend jamais la main et aucun dialogue n'apparaît, ni dans le bureau
agent ni sur le bureau utilisateur.

Limite à documenter pour le cycle : dans un bureau agent lancé avec le bus de
session hérité de l'hôte — la recette gelée —, toute app qui passe par le
portal `FileChooser` reste bloquée sur son dialogue. La piste du bus privé
plus bas lève ce blocage, hors contrat de ce cycle. Cela couvre les
apps GTK4, dont le picker passe par le portal quel que soit `GTK_USE_PORTAL`
(voir plus bas).

## Environnement

- Hyprland 0.56.0 (`36b2e0cf`), hôte + nested, même binaire.
- xdg-desktop-portal 1.22.1, xdg-desktop-portal-gtk 1.15.3,
  xdg-desktop-portal-hyprland 1.4.0, dbus 1.16.2.
- zenity 4.2.2 (GTK 4.22.4), grim 1.5.0.
- `XDG_CURRENT_DESKTOP=Hyprland` ; `/usr/share/xdg-desktop-portal/hyprland-portals.conf`
  = `default=hyprland;gtk`, donc `FileChooser` est servi par l'impl gtk.
- Env effectif du process lancé dans le nested (relevé par `env` dans le
  wrapper) : `WAYLAND_DISPLAY=wayland-2`,
  `HYPRLAND_INSTANCE_SIGNATURE=<sig nested>`,
  `DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus` (bus de l'HÔTE).

## Ce qui a été rejoué

Recette nested du PoC (spec §2), à l'identique : `hyprctl output create headless
hyprpilot-portalprobe` → `dispatch renameworkspace <id actif du headless>
agent-portalprobe` → `dispatch exec "[workspace name:agent-portalprobe silent;
noinitialfocus] Hyprland -c <conf minimale>"` → identification de la console
par PID + class `aquamarine` → `hyprctl -i <sig> dispatch exec <wrapper>`.
Les raffinements §4 non rejoués (mode-set `WxH@60` + `scale 1`, revérification
du snapshot hôte) ne touchent pas le chemin portal.

Wrapper : `GTK_USE_PORTAL=1 zenity --file-selection --title=hyprpilot-portal
--filename=/run/user/1000/hpprobe/sentinel.txt`, stdout/stderr/rc capturés dans
des fichiers. Fichier sentinelle créé avant le lancement (32 octets).

## Résultat

L'appel portal part bien et est routé (trace `dbus-monitor` sur le bus hôte) :

```
method call sender=:1.1106 -> destination=org.freedesktop.portal.Desktop
  interface=org.freedesktop.portal.FileChooser; member=OpenFile
  string ""                      # parent_window
  string "hyprpilot-portal"      # title
  ... current_folder = "/run/user/1000/hpprobe"
method call sender=:1.40 -> destination=:1.46   # xdg-desktop-portal -> impl gtk
  interface=org.freedesktop.impl.portal.FileChooser; member=OpenFile
  string "org.gnome.Zenity"
```

Ensuite, pendant toute la fenêtre d'observation bornée (poll 0,1 s sur
`hyprctl -i <sig> clients -j` ET `hyprctl clients -j`, 120 itérations ;
16,6 s séparent les deux appels `OpenFile` du run) :

- aucune fenêtre dans le nested (`clients` de l'instance = `[]`) ;
- aucune nouvelle adresse côté hôte hormis la console `aquamarine` du nested
  (diff d'adresses seules contre le snapshot de départ) ;
- aucun signal `Response` sur le bus ;
- `zenity` ne rend jamais la main : ni stdout, ni code retour. Il a fallu le
  tuer (`pkill -f -- "--filename=<sentinelle>"`).

Le chemin sentinelle n'a donc jamais pu être sélectionné : il n'y a pas de
chemin retourné à vérifier. Non tranché, sans incidence sur le verdict : le
dialogue n'est jamais créé, ou il l'est au-delà de la fenêtre d'observation —
dans les deux cas l'agent ne dispose d'aucun picker.

## Erreur exacte

Seule trace émise, par l'impl gtk, à la milliseconde de l'appel
(`journalctl --user`) :

```
xdg-desktop-portal-gtk[4213]: Unhandled parent window type
xdg-desktop-portal-gtk[4213]: Failed to associate portal window with parent window
```

Ces deux lignes ne sont PAS la cause. Contrôle exécuté depuis un client de
l'instance HÔTE (même appel, même `parent_window` vide) : les deux mêmes lignes
sont émises ET le dialogue s'affiche normalement
(`{"class":"xdg-desktop-portal-gtk","title":"hyprpilot-hostctl","pid":4213,"monitor":0}`).
Le différenciateur est donc bien le nested, pas le handle de parent absent.

## Fait annexe : `GTK_USE_PORTAL=0` ne désactive rien

Le second passage du probe, avec `GTK_USE_PORTAL=0`, produit exactement le même
appel `org.freedesktop.portal.FileChooser.OpenFile` (trace `dbus-monitor`) et le
même blocage. Avec zenity 4.2.2 / GTK 4.22, le portal est le seul chemin des
file pickers. Conséquence pour le cycle : la limite ne se contourne pas par
l'environnement de l'app, et elle vaut pour toute app GTK4 lancée dans un
bureau agent, pas seulement pour zenity.

## Piste consignée : bus D-Bus de session privé par instance (validée)

La piste annoncée par la spec §8 a été testée dans la foulée, même recette
nested, un seul changement : un bus de session privé dont l'environnement
pointe sur le `WAYLAND_DISPLAY` du nested.

```bash
# 1. bus privé ; son env est hérité par tous les services qu'il active
WAYLAND_DISPLAY=<socket nested> HYPRLAND_INSTANCE_SIGNATURE=<sig nested> \
  dbus-daemon --session --address="unix:path=$XDG_RUNTIME_DIR/<court>.sock" \
  --print-address --nofork &
# 2. l'app, dans le nested, sur ce bus
hyprctl -i <sig nested> dispatch exec <wrapper>   # DBUS_SESSION_BUS_ADDRESS=<addr privée>
```

Résultat : le dialogue s'ouvre DANS le bureau agent
(`{"class":"xdg-desktop-portal-gtk","title":"hyprpilot-privbus"}`, client de
l'instance nested), `Down` + `Return` envoyés par
`hyprctl -i <sig> dispatch sendshortcut` sélectionnent la sentinelle, `zenity`
sort avec `rc=0` et stdout `/run/user/1000/hpprobe2/sentinel.txt`, identique au
chemin attendu. Bureau utilisateur intact sur ce run (aucune adresse perdue ou
gagnée, curseur, workspace actif et fenêtre active identiques avant/après).

Ce que cela coûterait, hors contrat gelé de ce cycle :

- une ressource de plus dans le cycle de vie de session (§4 et §6 ne
  connaissent pas de bus) : `dbus-daemon` + les impls activées à la demande —
  ici `xdg-desktop-portal`, `xdg-desktop-portal-gtk` ET
  `xdg-desktop-portal-hyprland`, les trois sur le display du nested ;
- un teardown qui les termine : identification sûre par
  `DBUS_SESSION_BUS_ADDRESS=<addr privée>` dans `/proc/<pid>/environ`, jamais
  par nom de binaire ;
- contrainte pratique : `sun_path` est limité à 108 octets, le chemin du socket
  doit être court. `$XDG_RUNTIME_DIR/hyprpilot/sessions/<name>/bus.sock` passe ;
  un chemin sous un dossier de scratch profond échoue avec
  `Failed to start message bus: Socket name too long`.

Bénéfice collatéral si cette piste est reprise : xdg-desktop-portal-hyprland
lancé sur ce bus se lie au nested (son log liste `zwlr_screencopy_manager_v1` et
`hyprland_toplevel_export_manager_v1` de l'instance agent), donc chaque bureau
agent aurait aussi ses portals screenshot/screencast, isolés de ceux de
l'utilisateur.

## Effet de bord observé sur l'hôte, à surveiller

Sur le premier des deux runs, DEUX fenêtres ghostty de l'utilisateur ont
disparu : les deux sont présentes au snapshot post-spawn
(`host-post-spawn-clients.json`) et absentes du snapshot after-portal
(`host-after-portal-clients.json`).

- `0x5608f1632e40` (pid 481801, titre « ⠐ Reprendre le projet moment ») :
  `hyprctl output create headless` a coïncidé à la milliseconde avec un SIGSEGV
  de ce process, dans `libgtk-4.so.1` sous `wl_display_dispatch_queue_pending`
  (kernel : `ghostty[481801]: segfault ... in libgtk-4.so.1.2200.4`, coredump
  systemd 23:37:53). Aucune commande du probe n'a visé ce processus : le seul
  événement concomitant est l'ajout d'un `wl_output`. Non systématique : le
  second run du probe et les runs du PoC de l'après-midi ont créé/retiré des
  outputs sans crash (aucun coredump entre 14:46 et 15:35, `coredumpctl`).
  À re-qualifier pendant l'implémentation (client GTK4 fragile à l'ajout
  d'output) ; sur ces seules traces, rien ne le rattache à un défaut de la
  recette.
- `0x5608f1693740` (pid 469832, titre `~/projects/agents-skills`) : AUCUN
  coredump (`coredumpctl list --since '2026-07-24 14:00'` ne rend que 481801).
  Sa cause n'est PAS établie et elle n'est pas imputable au probe : `run.log`
  ne contient aucun `closewindow` ni `focuswindow` visant l'hôte, et
  `host_addr` est vide dans les deux passages. Une fermeture par l'utilisateur
  est au moins aussi plausible.

L'invariant « hôte intact » n'a donc PAS été vérifié pour le run 1 : la
revérification du snapshot hôte n'a pas été rejouée (cf. plus haut, raffinements
§4 non rejoués).

Autre observation du même run, pour la question ouverte §9 : l'avertissement
nested « Hyprland was started without start-hyprland » est cosmétique mais il
s'affiche en bandeau dans le bureau agent, donc il apparaît dans les captures
`--full`.

## Reproduction

Scripts et artefacts (éphémères, scratchpad de la session
`f44dfcb2-1fa5-4b0f-9fa5-f20f0c87b269`) : `portal-probe/probe.sh` (recette
gelée, deux passages `GTK_USE_PORTAL=1` puis `0`, `dbus-monitor` sur le bus
hôte), `portal-probe2/probe2.sh` (bus privé, capture
`nested-picker-selected.png`), `host-control/control.sh` (contrôle hôte).
`probe.sh` et `probe2.sh` posent un trap EXIT/INT/TERM qui termine le nested,
retire l'output et restaure `cursorpos`. `control.sh` ne crée ni output headless
ni instance nested : son trap tue son propre zenity par le chemin de sa
sentinelle (`pkill -f -- "--filename=<sentinelle>"`), supprime le dossier
sentinelle et restaure le focus initial.
