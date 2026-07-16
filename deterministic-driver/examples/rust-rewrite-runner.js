export const meta = {
  name: 'rust-rewrite-runner',
  description: 'Driver déterministe d\'une réécriture multi-phases pilotée par spec gelée : implémentation via CLI externe (Codex), discovery review unique, fact-check, arbitrage fermé, commits par le driver seul. Tout le spécifique projet/machine vit dans la section CONFIG en tête du script.',
  whenToUse: 'NE PAS lancer sans validation humaine. Paramétrage = section CONFIG (repo, spec, gates, modèles). Après une pause : reprise = éditer les constantes en tête du fichier (RETRY_TOKENS/BACKUP_DIR/HUMAN_ARBITRATION/MUTATION_ACK) puis Workflow({scriptPath, resumeFromRunId}) — le paramètre args est ignoré à la reprise.',
  phases: [
    { title: 'Préflight', detail: 'invariants repo, toolchain, backup vérifié' },
    { title: 'Phase 0', detail: 'contrats et scaffold' },
    { title: 'Phase 1', detail: 'domaine pur' },
    { title: 'Phase 2', detail: 'lecture Chromium' },
    { title: 'Phase 3', detail: 'lecture Firefox' },
    { title: 'Phase 4', detail: 'CLI read-only' },
    { title: 'Phase 5', detail: 'plan et résolution' },
    { title: 'Phase 6', detail: 'backup, verrou, sécurité' },
    { title: 'Phase 7', detail: 'writer Chromium' },
    { title: 'Phase 8', detail: 'writer Firefox' },
    { title: 'Phase 9', detail: 'apply/recover complet' },
    { title: 'Phase 10', detail: 'documentation et audit final' },
    { title: 'Acceptance finale', detail: 'validation globale + audit sécurité' },
  ],
}

// ============================================================================
// EXAMPLE — real-world instance of the deterministic-driver skill.
// Frozen snapshot (2026-07-16) of the refactored rust-rewrite-runner: the
// driver that ran the bookmarker → Rust rewrite (10 phases, run
// wf_3b55d1fb-940, 2026-07-15), post-run parametrization applied. The LIVE
// copy lives in ~/.claude/workflows/rust-rewrite-runner.js and may drift.
// Read references/template.js for the bare skeleton; this file shows every
// mechanism assembled: CONFIG section, resume channel, fingerprint
// sandwich, detached CLI wrapper, findings pipeline, observed-state commits,
// browser-profile gate windows with MUTATION_ACK re-baseline.
// Prompts are in French (operator's language); the mechanics are generic.
// ============================================================================

// ============================================================================
// DRIVER DÉTERMINISTE — méthodologie générique : skill `deterministic-driver`
// (agents-skills). Version refactorée post-run wf_3b55d1fb-940 (2026-07-15) :
// paramétrage isolé en CONFIG, durcissements mid-run dégatés, FIXME 2-6
// appliqués. La version du run est archivée
// (~/.claude/workflows/archive/).
//
// Décisions de mapping (déviations documentées vis-à-vis de la spec runner) :
//
// 1. Driver déterministe : ce script JS est le driver. Les agents LLM sont des
//    exécuteurs mécaniques contraints par schéma ; le script juge exitCodes,
//    fingerprints et verdicts. exec() vérifie la correspondance 1:1 entre
//    commandes demandées et résultats rapportés (cardinalité + verbatim) ;
//    aucun gate ne peut passer « à vide ». Résidu de confiance : les valeurs
//    (exitCode) sont rapportées par un LLM — c'est pourquoi les décisions
//    critiques (commit) sont jugées sur l'ÉTAT OBSERVÉ (head, status) et pas
//    sur les exit codes rapportés. Aucune branche « si X disponible… sinon »
//    dans un prompt d'agent : la méthodologie s'impose (invocation
//    obligatoire, échec d'invocation signalé, jamais de bascule silencieuse).
//
// 2. CLI externe : agent(model) n'accepte que des modèles Claude.
//    L'implémenteur passe par un agent wrapper du CLI `codex exec`, lancé
//    DÉTACHÉ (setsid nohup + exit code écrit dans un fichier connu) et
//    attendu par appels Bash BLOQUANTS répétés — un agent à schéma ne peut
//    jamais « attendre » en terminant son tour (l'enforcement
//    StructuredOutput le tuerait ; a tué un run réel en phase 1).
//    FAITS VALIDÉS sur codex-cli 0.144.4 (probes réels, 2026-07-15) :
//    - `--json` n'expose NI le modèle NI l'effort => mode non-json + parsing
//      du header stderr (`model:`, `reasoning effort:`, `session id:`).
//    - le `session id` du header EST le thread id resumable.
//    - PIÈGE : `codex exec resume` N'HÉRITE PAS de l'effort — il retombe
//      silencieusement sur le défaut de ~/.codex/config.toml. L'effort est
//      re-passé explicitement à CHAQUE resume ; le modèle aussi
//      (ceinture-bretelles).
//    - id inconnu => exit 1 + `no rollout found for thread id ...` =>
//      sessionFound=false détectable.
//    Le driver pause si le modèle ou l'effort EFFECTIFS divergent (preuve par
//    header, pas confiance). Reprise par ID exact (`--last` interdit).
//    Re-prober ces faits à chaque changement de version du CLI, AVANT le
//    lancement — le seul mécanisme non probé du premier run l'a tué.
//
// 3. Gates humains et pauses : un workflow ne peut pas interroger
//    l'utilisateur. Pause = retour {status:'paused', reason, retryStep,
//    howToResume autosuffisant}. FAIT VALIDÉ (probe réel) : `resumeFromRunId`
//    IGNORE le paramètre `args`. Le canal de reprise est le FICHIER : les
//    constantes RETRY_TOKENS / BACKUP_DIR / HUMAN_ARBITRATION / MUTATION_ACK
//    sont éditées par l'opérateur entre deux reprises, puis
//    Workflow({scriptPath, resumeFromRunId}).
//    RETRY_TOKENS[stepId] est injecté dans le prompt de CE step seul (hdr()),
//    donc son bump n'invalide que ce step et sa suite. Les autres constantes
//    ne sont lues que par le JS : les éditer ne casse pas le cache antérieur.
//    TOUTE pause fournit un retryStep — il désigne l'OBSERVATION à rejouer
//    (paire before/after cohérente), et se bumpe TOUJOURS, même quand une
//    édition de prompt semble suffire à casser le cache.
//
// 4. État durable = git (un commit driver par phase) ; le cache par préfixe
//    est un accélérateur JETABLE. Le committed-check en début de phase rend
//    le cold restart bon marché (les phases committées sont sautées) : cold
//    restart par défaut aux pauses inter-phases, resume par préfixe réservé
//    à l'intra-phase. Les sessionIds Codex survivent dans les résultats
//    cachés ; si une session a disparu du disque, le wrapper la signale et
//    le driver pause (required_agent_session_unavailable) — contexte non
//    reconstructible, arbitrage humain requis.
//
// 5. Vérification compliance : la spec reprend la même session de review.
//    Les agents workflow sont one-shot ; la vérification est un agent frais
//    recevant le ledger complet (findings + dispositions + verdicts) dans
//    son prompt. Objectif fermé préservé par le ledger, pas par la session.
//
// 6. Reviewers read-only : pas de permissionMode imposable. Prévention par
//    prompt + détection par fingerprint avant/après chaque appel non
//    mécanique. Toute mutation => pause ; le retryStep re-exécute l'appel
//    fautif après remise en état humaine de l'arbre.
//
// 7. Modèles Claude : la spec interdit les modèles hors liste dans la loop.
//    Reviews/arbitrage : REVIEW (effort xhigh). Exécuteurs mécaniques et
//    wrappers : MECH (effort low). Coût : le socle fixe par agent (~27K
//    tokens) domine — fusionner les sondes avec l'exec adjacent quand la
//    cadence le permet reste la piste d'optimisation principale.
//
// 8. Gates de phase : les gates prose de la spec non réductibles à des
//    commandes (round-trips navigateur) sont des scripts à chemin FIXE
//    (scripts/gates/…) que l'implémenteur doit LIVRER pendant la phase.
//    Le driver exécute le chemin connu ; script manquant = gate rouge =
//    feedback normal. Les scripts vivent dans le repo (chemin non protégé),
//    sont commités avec la phase et reviewés par la discovery review.
//    Règles d'environnement des scripts formulées en OPÉRATIONS (cp autorisé,
//    ouverture interdite), jamais en intentions (« lecture seule ») — ouvrir
//    une base SQLite/WAL en LECTURE touche le -shm : une règle d'intention
//    est physiquement inviolable sur un répertoire surveillé par stat-hash.
//
// 9. Invariant worktree : le préflight exige un index vide PUIS fige la
//    baseline statusSha256 (changements protégés seuls). Après chaque commit
//    de phase, le status doit revenir exactement à cette baseline. Le commit
//    stage tout sauf les chemins protégés, avec un garde-fou anti-pollution
//    et `git check-ignore target` exigé à chaque gate.
//
// 10. Profils navigateur réels : denylist système inapplicable. Traduction :
//     (a) sandbox workspace-write du CLI ; (b) navigateurs FERMÉS exigés
//     (pgrep) aux DEUX extrémités de chaque fenêtre de gate navigateur — les
//     deux moitiés d'un gate (détection pgrep / interprétation stat-hash)
//     doivent avoir la même couverture temporelle, sinon l'interprétation
//     travaille sur une hypothèse que la détection ne garantit plus (TOCTOU) ;
//     (c) stat-hash des fichiers de profils réels capturé avant/après chaque
//     fenêtre — navigateurs fermés, toute variation = mutation par le gate
//     => pause, et la reprise exige une attestation MUTATION_ACK (re-baseline
//     arbitré). Le pgrep est un dispositif de QUALITÉ DE SIGNAL (imputabilité
//     du stat-hash) ; la défense anti-corruption réelle est la paire
//     stat-hash, fail-closed même quand le pgrep rate. Piège pgrep : -x
//     compare au comm noyau, tronqué à 15 caractères — motifs tronqués à 15.
//     Hors fenêtres, les profils ne sont pas surveillés (activité utilisateur
//     légitime). GATE HUMAIN DÉLÉGABLE (décidé au design) : la fermeture des
//     navigateurs peut être déléguée à l'orchestrateur si l'utilisateur l'a
//     autorisée explicitement — le driver, lui, ne ferme jamais rien.
//
// 11. Reprise : une garde live en tête de script est impossible (elle
//     invaliderait tout le cache par préfixe). Les re-vérifications sont
//     positionnées là où elles comptent : le backup est re-vérifié au début
//     des phases destructrices (cache si la phase est committée, live si
//     elle est le front de reprise).
//
// 12. Arbitrage humain : HUMAN_ARBITRATION[findingId] =
//     {verdict: 'REQUIRED_FIX'|'DISMISSED', requiredOutcome?} est appliqué
//     par le DRIVER aux verdicts ESCALATE (et aux findings rouverts) lors
//     d'une reprise — le canal de ré-injection de la décision humaine.
//
// 13. Correctifs mid-run : tout durcissement de prompt appliqué PENDANT un
//     run se gate par phase (p >= N) pour préserver le préfixe de cache des
//     étapes déjà jouées. Ce fichier est la version FRAÎCHE : tout est
//     dégaté ; les durcissements vivent dans le prompt de base de toutes les
//     phases applicables. L'opérateur n'édite le repo cible qu'ENTRE les
//     phases (le driver ne distingue pas une édition humaine d'une mutation
//     d'agent dans une fenêtre fingerprint).
//
// Le paramètre `args` du Workflow n'est PAS utilisé (ignoré à la reprise,
// cf. #3). Toute la configuration mutable est dans les constantes ci-dessous.
// ============================================================================

// --- CANAL DE REPRISE : constantes éditées par l'opérateur entre reprises ---
//
// EMPLACEMENT DU SCRIPT : ce fichier doit vivre HORS du repo cible (invocation
// par scriptPath). S'il vivait dans <repo>/.claude/, chaque édition de ces
// constantes mid-run modifierait un chemin protégé => protectedSha256
// divergerait entre fingerprints cachés et live => pauses spurieuses. Hors
// repo, les éditions ne touchent ni la baseline worktree ni le hash protégé,
// et l'implémenteur ne peut pas altérer le runner.

// Chemin du backup vérifié des profils réels. null => pause au préflight.
// Éditer puis Workflow({scriptPath, resumeFromRunId}).
const BACKUP_DIR = null

// Bump du token d'une étape => ré-exécution LIVE de cette étape et de tout ce
// qui suit ; le reste rejoue depuis le cache. La pause indique le stepId exact.
// Ex: { 'p3.val.1': 1 }
const RETRY_TOKENS = {}

// Décisions humaines sur les escalades d'arbitrage et les blocages compliance.
// Ex: { 'F-12': { verdict: 'DISMISSED' } }
//     { 'F-13': { verdict: 'REQUIRED_FIX', requiredOutcome: '...' } }
const HUMAN_ARBITRATION = {}

// Attestations post-mutation (re-baseline arbitré) : après une pause
// live_profile_mutation_suspected au step S, la reprise EXIGE une entrée
// MUTATION_ACK[S] décrivant la vérification effectuée (profils comparés au
// backup, integrity_check, wal) — au rejeu, la fenêtre capture une paire
// stat-hash fraîche (nouvelle référence), acceptée au titre de l'attestation
// et tracée dans le log. Ex: { 'p8.val.1': 'Bookmarks == backup, integrity_check ok, wal vide' }
const MUTATION_ACK = {}

// ============================================================================
// CONFIG — tout le spécifique projet/machine. Le corps du script ne référence
// que cette section (et les constantes de reprise ci-dessus).
// ============================================================================

const NAME = 'rust-rewrite-runner'
// Désignation de la tâche, utilisée dans les prompts.
const TASK = 'la réécriture Rust de bookmarker'
const REPO = '/home/b3ngous/projects/bookmarker-rust'
const BRANCH = 'feature/rust-rewrite'
// Contrat gelé (sha256 vérifié à chaque phase) + sections citées aux agents.
const SPEC = 'rust_rewrite.md'
const SPEC_SECTIONS = {
  contracts: '§1-12',      // contrats
  phase: '§13',            // définition des phases et gates prose
  tests: '§14',            // exigences de tests
  gates: '§15',            // gate déterministe (fmt/clippy/test/build)
  humanValidation: '§17',  // validation produit humaine finale
  safetyAudit: '§§14-17',  // référentiel de l'audit final de sécurité
}
const PHASES = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
const MAX_VALIDATION_ATTEMPTS = 3
const MAX_COMPLIANCE_RETRIES = 1
const CODEX_MODEL = 'gpt-5.6-sol'
const CODEX_EFFORT = 'high'
// Hors dépôt : les fichiers de travail Codex ne doivent pas polluer le
// worktree (la baseline statusSha256 les verrait comme des résidus).
const CODEX_WORK = '/tmp/codex-runner-bookmarker-rust'
const REVIEW = { model: 'opus', effort: 'xhigh' }
const MECH = { model: 'opus', effort: 'low' }

// Toolchain exigée au préflight (le driver n'installe RIEN lui-même).
const TOOLCHAIN_VERSION = '1.97.0'
const TOOLCHAIN_CMDS = [
  'rustc --version',
  'cargo --version',
  'cargo fmt --version',
  'cargo clippy --version',
]

const CARGO_GATE = [
  'cargo fmt --all --check',
  'cargo clippy --all-targets --all-features --locked -- -D warnings',
  'cargo test --all-targets --all-features --locked',
  'cargo build --release --locked',
]

// Hygiène exigée à chaque gate : target/ doit être gitignoré (sinon le commit
// driver l'embarquerait) — le gate échoue tant que .gitignore ne le couvre pas.
const HYGIENE_GATE = ['git check-ignore -q target']

// Chemins utilisateur protégés : jamais stagés, jamais modifiés par un agent.
const PROTECTED = ['plans', '.claude', '.agents', '.issue-queue.json']
const PROTECT_EXCLUDES = PROTECTED.map(p => `":(exclude)${p}"`).join(' ')
// Garde anti-pollution du commit driver (chemins protégés + target/).
const GUARD_PATTERN = '^(target/|plans/|\\.claude/|\\.agents/|\\.issue-queue\\.json)'

// Fenêtres navigateur : navigateurs fermés + profils réels sous surveillance.
// Comm noyau tronqué à 15 caractères : motifs coupés via cut -c1-15 (un motif
// de 16+ caractères ne peut JAMAIS matcher avec pgrep -x). Liste = comm réels
// observés sur cette machine (brave, chromium, firefox) + variantes usuelles.
const BROWSER_GATE_PHASES = [2, 3, 7, 8]
const BROWSER_COMMS = ['brave', 'brave-browser', 'chromium', 'chrome', 'firefox']
const BROWSERS_CLOSED_CMD =
  `bash -c 'for p in ${BROWSER_COMMS.join(' ')}; do pgrep -x "$(cut -c1-15 <<< "$p")" >/dev/null && { echo "ACTIF: $p"; exit 1; }; done; echo OK'`
const PROFILE_STAT_CMD =
  `bash -c 'stat -c "%n %s %Y" ~/.config/BraveSoftware/Brave-Browser/*/Bookmarks ~/.config/chromium/*/Bookmarks ~/.mozilla/firefox/*/places.sqlite* 2>/dev/null | LC_ALL=C sort | sha256sum'`
// Noms lisibles pour les hints humains.
const BROWSER_NAMES = 'Brave, Chromium et Firefox'

// Phases destructrices : backup re-vérifié en entrée de phase (live au front
// de reprise, cache si la phase est déjà committée).
const BACKUP_RECHECK_PHASES = [7, 8, 9]
// Hint du gate humain de création du backup (préflight).
const BACKUP_CREATE_HINT = [
  `GATE HUMAIN : 1) fermer ${BROWSER_NAMES} ;`,
  '2) créer le backup ciblé sous ~/.local/share/bookmarker/pre-rewrite-backup/<timestamp>/ (Bookmarks + snapshot places.sqlite + MANIFEST.sha256) ;',
  '3) éditer la constante BACKUP_DIR en tête de ce script puis Workflow({scriptPath, resumeFromRunId}). Le paramètre args est ignoré à la reprise : le fichier est le seul canal.',
].join(' ')

// Scripts de gate à chemin fixe : LIVRABLES obligatoires de leur phase
// (créés par l'implémenteur, commités avec la phase, reviewés par la
// discovery review). Le driver exécute le chemin connu ; absent ou rouge =
// échec de validation, feedback normal à l'implémenteur.
const GATE_SCRIPTS = {
  2: { path: 'scripts/gates/phase2-chromium-read.sh', deliverable: 'extraction déterministe sur copie scratch du profil Brave/Chromium : 2 runs, sorties identiques, exit 0' },
  3: { path: 'scripts/gates/phase3-firefox-read.sh', deliverable: 'integrity_check + extraction sur copie scratch Firefox, zéro écriture sur le profil source' },
  7: { path: 'scripts/gates/phase7-chromium-roundtrip.sh', deliverable: 'round-trip writer Chromium sur copie scratch : Brave/Chromium ouvre le profil écrit, réextraction exactement égale à l\'état attendu' },
  8: { path: 'scripts/gates/phase8-firefox-roundtrip.sh', deliverable: 'round-trip writer Firefox sur copie scratch : integrity_check + invariants Places verts, réextraction exacte' },
}
// Règle de lancement navigateur des scripts de gate (opérations, pas intentions).
const SCRATCH_LAUNCH_RULES = '--user-data-dir=<scratch> (Chromium), -no-remote -profile <scratch> (Firefox) — jamais un profil réel'

// Gates spécifiques de phase, en plus de HYGIENE_GATE + CARGO_GATE exécutés à
// chaque validation. Le gate doc de la phase 10 (README suffisant pour un
// nouvel utilisateur) est un jugement, pas une commande : il est porté par la
// discovery review cumulative de la phase 10.
const PHASE_EXTRA_GATES = {
  0: [],
  1: [],
  2: [`bash ${GATE_SCRIPTS[2].path}`],
  3: [`bash ${GATE_SCRIPTS[3].path}`],
  4: [`bash -c '[ -z "$(git ls-files "*.ts" "*.tsx")" ]'`],
  5: [],
  6: [],
  7: [`bash ${GATE_SCRIPTS[7].path}`],
  8: [`bash ${GATE_SCRIPTS[8].path}`],
  9: [],
  10: [],
}

// Scripts d'acceptance finale : livrables de la phase 9 (orchestration
// complète), exécutés une dernière fois après la dernière phase.
const FINAL_SCRIPTS_PHASE = 9
const FINAL_ACCEPTANCE_SCRIPTS = [
  'scripts/gates/final-brave.sh',
  'scripts/gates/final-chromium.sh',
  'scripts/gates/final-firefox.sh',
]
const FINAL_ACCEPTANCE_DESC = 'valident respectivement Brave, Chromium et Firefox 152.0.5/schema 86 de bout en bout sur copies'
const FINAL_BROWSER_GATES = FINAL_ACCEPTANCE_SCRIPTS.map(s => `bash ${s}`)

// Messages de commit du driver (sujet exact : sert aussi au committed-check).
const COMMIT_SUBJECT = p => `feat(rust): complete phase ${p}`
const FINAL_FIX_COMMIT_SUBJECT = 'fix(rust): final safety audit corrections'

// ============================================================================
// FIN CONFIG — plus aucune valeur projet/machine sous cette ligne.
// ============================================================================

// --- Schémas -----------------------------------------------------------------
// Règle : tout champ requis doit nourrir un jugement du driver. Un champ que
// le driver ne lit jamais est celui qu'un exécuteur peut inventer gratuitement.

const EXEC_SCHEMA = {
  type: 'object',
  required: ['results'],
  properties: {
    results: {
      type: 'array',
      items: {
        type: 'object',
        required: ['command', 'exitCode', 'outputTail'],
        properties: {
          command: { type: 'string', description: 'Commande exécutée, VERBATIM' },
          exitCode: { type: 'integer' },
          outputTail: { type: 'string', description: '60 dernières lignes max de stdout+stderr, verbatim' },
        },
      },
    },
  },
}

const FP_SCHEMA = {
  type: 'object',
  required: ['branch', 'head', 'specSha256', 'indexSha256', 'statusSha256', 'diffSha256', 'protectedSha256'],
  properties: {
    branch: { type: 'string' },
    head: { type: 'string' },
    specSha256: { type: 'string' },
    indexSha256: { type: 'string' },
    statusSha256: { type: 'string' },
    diffSha256: { type: 'string' },
    protectedSha256: { type: 'string' },
  },
}

const CODEX_SCHEMA = {
  type: 'object',
  required: ['sessionId', 'reportedModel', 'reportedEffort', 'completed', 'sessionFound', 'summary', 'finalOutput'],
  properties: {
    sessionId: { type: 'string', description: 'ID exact de la session Codex (jamais --last)' },
    reportedModel: { type: 'string', description: 'Modèle effectif rapporté par le runtime Codex' },
    reportedEffort: { type: 'string', description: 'Reasoning effort effectif rapporté par le runtime Codex' },
    completed: { type: 'boolean' },
    sessionFound: { type: 'boolean', description: 'false si une reprise par ID a échoué car la session n\'existe plus' },
    summary: { type: 'string', description: '5 lignes max' },
    finalOutput: { type: 'string', description: 'Message final de Codex VERBATIM, non résumé, non tronqué' },
  },
}

const FINDINGS_SCHEMA = {
  type: 'object',
  required: ['findings'],
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        required: ['id', 'file', 'summary', 'contractViolated', 'failureScenario', 'evidence', 'minimalRequiredOutcome'],
        properties: {
          id: { type: 'string' },
          file: { type: 'string' },
          line: { type: 'integer' },
          summary: { type: 'string' },
          contractViolated: { type: 'string', description: 'Section/contrat de la spec violé' },
          failureScenario: { type: 'string' },
          evidence: { type: 'string', description: 'Preuve, chemin de code ou reproducteur' },
          minimalRequiredOutcome: { type: 'string' },
        },
      },
    },
  },
}

const DISPOSITIONS_SCHEMA = {
  type: 'object',
  required: ['dispositions'],
  properties: {
    dispositions: {
      type: 'array',
      items: {
        type: 'object',
        required: ['findingId', 'rootCauseId', 'disposition', 'evidence'],
        properties: {
          findingId: { type: 'string' },
          rootCauseId: { type: 'string', description: 'Regroupe les formulations dupliquées du même bug' },
          disposition: { type: 'string', enum: ['ACCEPT', 'REBUT', 'PREEXISTING', 'OUT_OF_PHASE', 'UNCERTAIN'] },
          evidence: { type: 'string' },
          minimalRequiredOutcome: { type: 'string' },
        },
      },
    },
  },
}

const ARBITRATION_SCHEMA = {
  type: 'object',
  required: ['verdicts'],
  properties: {
    verdicts: {
      type: 'array',
      items: {
        type: 'object',
        required: ['findingId', 'verdict', 'reason'],
        properties: {
          findingId: { type: 'string' },
          verdict: { type: 'string', enum: ['REQUIRED_FIX', 'DISMISSED', 'ESCALATE'] },
          reason: { type: 'string' },
          requiredOutcome: { type: 'string' },
        },
      },
    },
  },
}

const VERIFY_SCHEMA = {
  type: 'object',
  required: ['verdict', 'reopened'],
  properties: {
    verdict: { type: 'string', enum: ['PASS', 'REOPEN', 'PAUSE'] },
    reopened: {
      type: 'array',
      items: {
        type: 'object',
        required: ['findingId', 'reason', 'requiredOutcome'],
        properties: {
          findingId: { type: 'string' },
          reason: { type: 'string' },
          requiredOutcome: { type: 'string' },
        },
      },
    },
  },
}

// --- Helpers ------------------------------------------------------------------

class Pause {
  constructor(reason, detail, retryStep) {
    this.reason = reason
    this.detail = detail
    this.retryStep = retryStep
  }
}
// retryStep OBLIGATOIRE : sans lui, le résultat d'échec serait rejoué depuis le
// cache à la reprise et la pause bouclerait indéfiniment. retryStep désigne
// l'étape à re-exécuter en live (bump RETRY_TOKENS[retryStep]) — en général
// l'OBSERVATION qui a échoué (fingerprint, validation), pas le travail caché.
function pause(reason, detail, retryStep) {
  throw new Pause(reason, detail, retryStep)
}

function rt(stepId) {
  return RETRY_TOKENS[stepId] || 0
}
function hdr(stepId) {
  return `[${NAME} step:${stepId} retry:${rt(stepId)}]`
}
function humanDecision(findingId) {
  return HUMAN_ARBITRATION[findingId] || null
}

const MECH_RULES = [
  'Règles strictes :',
  "- Tu es un exécuteur mécanique du driver. AUCUNE interprétation, AUCUNE initiative.",
  "- N'exécute rien d'autre que ce qui est listé. Ne corrige rien. Ne retente rien.",
  '- Une commande qui échoue : consigne son résultat et passe à la suivante.',
  '- Ne modifie aucun fichier. Interdits : git add/commit/push/reset/stash, rm, écriture hors repo.',
  '- Copie les valeurs VERBATIM depuis les sorties réelles. Aucune valeur inventée.',
  '- Rapporte UN résultat par commande listée, dans le même ordre, avec la commande recopiée verbatim.',
].join('\n')

async function exec(stepId, phaseName, commands, context) {
  const prompt = [
    hdr(stepId),
    `Contexte : ${context}`,
    `Depuis ${REPO}, exécute via Bash, dans l'ordre, exactement ces ${commands.length} commandes :`,
    ...commands.map(c => `- ${c}`),
    MECH_RULES,
    // Soupape : un timeout backgroundé devient un échec propre, jamais une
    // attente en fin de tour (l'enforcement StructuredOutput tuerait l'agent).
    'Timeouts : passe timeout=600000 au tool Bash pour chaque commande. Si une commande atteint ce timeout et est déplacée en arrière-plan par le harness, NE l\'attends PAS et ne termine pas ton tour : rapporte pour elle exitCode=-1 et outputTail="TIMEOUT_BACKGROUNDED", puis continue la liste.',
    'Pour chaque commande retourne : command (verbatim), exitCode, outputTail (60 dernières lignes max).',
  ].join('\n')
  const out = await agent(prompt, { schema: EXEC_SCHEMA, label: stepId, phase: phaseName, ...MECH })
  if (!out) pause('executor_agent_failed', { stepId }, stepId)
  // Un gate ne passe jamais « à vide » : correspondance 1:1 commandes/résultats.
  if (out.results.length !== commands.length) {
    pause('executor_results_mismatch', { stepId, expected: commands.length, got: out.results.length }, stepId)
  }
  // Comparaison modulo enveloppe `bash -c '...'` : les exécuteurs strippent
  // systématiquement le wrapper en recopiant la commande (observé plusieurs
  // fois sur le run initial). La commande INTERNE reste verbatim et la
  // cardinalité 1:1 reste exigée — la garantie anti « gate à vide » tient.
  const normCmd = c => {
    const t = c.trim()
    const m = t.match(/^bash -c '([\s\S]*)'$/)
    return (m ? m[1] : t).trim()
  }
  const mismatched = out.results
    .map((r, i) => (normCmd(r.command) === normCmd(commands[i]) ? null : { index: i, expected: commands[i], got: r.command }))
    .filter(Boolean)
  if (mismatched.length) pause('executor_results_mismatch', { stepId, mismatched }, stepId)
  return out.results
}

async function fingerprint(stepId, phaseName) {
  const prompt = [
    hdr(stepId),
    `Sonde read-only du driver. Depuis ${REPO}, exécute exactement ces commandes et copie les valeurs verbatim :`,
    '- branch : git rev-parse --abbrev-ref HEAD',
    '- head : git rev-parse HEAD',
    `- specSha256 : sha256sum ${SPEC} (premier champ)`,
    '- indexSha256 : git diff --cached | sha256sum (premier champ)',
    '- statusSha256 : git status --porcelain=v2 --untracked-files=all | LC_ALL=C sort | sha256sum (premier champ)',
    '- diffSha256 : git diff | sha256sum (premier champ)',
    `- protectedSha256 : find ${PROTECTED.join(' ')} -type f -print0 2>/dev/null | LC_ALL=C sort -z | xargs -0 -r sha256sum | sha256sum (premier champ)`,
    MECH_RULES,
  ].join('\n')
  const f = await agent(prompt, { schema: FP_SCHEMA, label: stepId, phase: phaseName, ...MECH })
  if (!f) pause('fingerprint_agent_failed', { stepId }, stepId)
  if (f.branch !== BRANCH) pause('wrong_branch', { expected: BRANCH, got: f.branch }, stepId)
  return f
}

// afterStepId = étape à re-sonder/re-exécuter après remise en état humaine.
function assertUnchanged(before, after, reason, phaseNum, afterStepId) {
  const same =
    before.head === after.head &&
    before.indexSha256 === after.indexSha256 &&
    before.statusSha256 === after.statusSha256 &&
    before.diffSha256 === after.diffSha256 &&
    before.protectedSha256 === after.protectedSha256
  if (!same) pause(reason, { phase: phaseNum, before, after }, afterStepId)
}

// Le wrapper n'improvise rien : le driver fournit la commande exacte, le shell
// fait le parsing (header stderr -> model/effort/session id, -o -> message
// final), l'agent recopie. Lancement DÉTACHÉ : un run Codex peut dépasser le
// timeout Bash max (600 s) et un agent à schéma ne peut pas attendre en
// terminant son tour — setsid nohup + exit code fichier + attentes bloquantes.
function codexWrapperPrompt(stepId, resumeSessionId, instruction) {
  const work = `${CODEX_WORK}/${stepId.replace(/[^A-Za-z0-9._-]/g, '_')}`
  const cmd = resumeSessionId
    // resume : PAS de -s (hérité) ; effort re-passé car NON hérité (piège validé).
    ? `codex exec resume "${resumeSessionId}" -m ${CODEX_MODEL} -c model_reasoning_effort=${CODEX_EFFORT} -o "${work}/final.txt" - < "${work}/prompt.md" > "${work}/run.log" 2> "${work}/header.err"`
    : `codex exec -m ${CODEX_MODEL} -c model_reasoning_effort=${CODEX_EFFORT} -s workspace-write -o "${work}/final.txt" - < "${work}/prompt.md" > "${work}/run.log" 2> "${work}/header.err"`
  const launch = `mkdir -p "${work}" && rm -f "${work}/exit.code" && cd "${REPO}" && setsid nohup bash -c '${cmd}; echo "EXIT=$?" > "${work}/exit.code"' >/dev/null 2>&1 < /dev/null & echo LAUNCHED`
  const waitCmd = `timeout 580 tail -F "${work}/exit.code" 2>/dev/null | grep -m1 '^EXIT=' || cat "${work}/exit.code" 2>/dev/null || echo PENDING`
  return [
    hdr(stepId),
    'Wrapper mécanique du CLI Codex. Tu exécutes des commandes fournies, tu recopies le résultat. Aucune initiative.',
    `ÉTAPE 1 — avec l'outil Write (JAMAIS via le shell : le prompt contient des backticks et des $() qui seraient interprétés), crée le dossier et écris le fichier ${work}/prompt.md dont le contenu est EXACTEMENT le bloc délimité ci-dessous, sans les délimiteurs, sans rien ajouter, sans reformuler, sans résumer :`,
    '--- DÉBUT CONTENU prompt.md ---',
    instruction,
    '--- FIN CONTENU prompt.md ---',
    `ÉTAPE 2 — lance Codex détaché via Bash, exactement :`,
    launch,
    resumeSessionId
      ? `Reprise par ID exact uniquement (--last INTERDIT). Si ${work}/header.err ou ${work}/run.log contient « no rollout found for thread id », la session n'existe plus : NE CRÉE PAS de nouvelle session, retourne sessionFound=false et completed=false.`
      : 'Nouvelle session.',
    `ÉTAPE 2b — ATTENTE : Codex peut travailler plus d'une heure. INTERDIT : Monitor, run_in_background, ou terminer ton tour pour attendre (tu serais tué avant d'avoir rendu le résultat). Répète cet appel Bash BLOQUANT (paramètre timeout du tool = 600000) jusqu'à ce que sa sortie contienne une ligne EXIT= :`,
    waitCmd,
    `Maximum 24 répétitions (≈ 4 h). Si toujours PENDING après 24 : exécute pkill -f "${work}" puis retourne completed=false (ne fabrique aucune valeur).`,
    `ÉTAPE 3 — extrais les valeurs EFFECTIVES depuis le header, via Bash :`,
    `grep -oP '^(model|reasoning effort|session id): .*' "${work}/header.err"`,
    `ÉTAPE 4 — lis le message final de Codex : ${work}/final.txt, et le code de sortie : ${work}/exit.code`,
    'RETOUR — recopie VERBATIM depuis les sorties réelles, sans rien inventer ni corriger :',
    '- sessionId = la valeur après « session id: » du header (ou, en cas de session introuvable, l\'ID demandé) ;',
    '- reportedModel = la valeur après « model: » ; reportedEffort = la valeur après « reasoning effort: » (valeurs EFFECTIVES du header, jamais celles demandées) ;',
    `- completed = true seulement si ${work}/exit.code contient EXIT=0 ; sessionFound = false uniquement sur « no rollout found » ;`,
    `- finalOutput = le contenu INTÉGRAL et VERBATIM de ${work}/final.txt (canal de données : ne résume pas, ne tronque pas, ne reformate pas) ;`,
    '- summary = 5 lignes max pour le journal humain.',
    'Interdits : modifier des fichiers du dépôt, git add/commit/push, exécuter autre chose que les commandes ci-dessus, relancer Codex en cas d\'échec.',
  ].join('\n')
}

async function codex(stepId, phaseName, resumeSessionId, instruction) {
  const res = await agent(codexWrapperPrompt(stepId, resumeSessionId, instruction), {
    schema: CODEX_SCHEMA, label: stepId, phase: phaseName, ...MECH,
  })
  if (!res) pause('codex_wrapper_failed', { stepId }, stepId)
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(res.sessionId)) {
    pause('codex_session_id_invalid', { stepId, got: res.sessionId }, stepId)
  }
  if (resumeSessionId && !res.sessionFound) {
    pause('required_agent_session_unavailable', {
      stepId,
      sessionId: resumeSessionId,
      hint: 'La session Codex requise a disparu (élagage, autre machine). Contexte non reconstructible — arbitrage humain requis sur la suite.',
    }, stepId)
  }
  if (res.reportedModel !== CODEX_MODEL || res.reportedEffort !== CODEX_EFFORT) {
    pause('unexpected_effective_model', {
      stepId, expected: { model: CODEX_MODEL, effort: CODEX_EFFORT },
      got: { model: res.reportedModel, effort: res.reportedEffort },
    }, stepId)
  }
  if (resumeSessionId && res.sessionId !== resumeSessionId) {
    pause('codex_session_mismatch', { stepId, expected: resumeSessionId, got: res.sessionId }, stepId)
  }
  return res
}

async function validatePhase(p, phaseName, stepId) {
  const extras = PHASE_EXTRA_GATES[p] || []
  const isBrowser = BROWSER_GATE_PHASES.includes(p)
  // Fenêtre navigateur : navigateurs fermés + profils réels stat-hashés
  // avant/après. Fermés, les profils ne peuvent pas changer légitimement :
  // toute variation = mutation par le gate. Le check navigateurs est fait aux
  // DEUX extrémités de la fenêtre : les deux moitiés du gate (détection
  // pgrep / interprétation stat-hash) doivent avoir la même couverture
  // temporelle, sinon un navigateur lancé mid-fenêtre rend le stat-hash
  // ininterprétable (TOCTOU).
  const commands = isBrowser
    ? [BROWSERS_CLOSED_CMD, PROFILE_STAT_CMD, ...HYGIENE_GATE, ...extras, ...CARGO_GATE, PROFILE_STAT_CMD, BROWSERS_CLOSED_CMD]
    : [...HYGIENE_GATE, ...extras, ...CARGO_GATE]
  if (isBrowser && MUTATION_ACK[stepId]) {
    // Re-baseline arbitré (post-incident) : l'opérateur a attesté la
    // vérification ; la fenêtre rejouée capture sa propre paire before/after
    // (nouvelle référence), acceptée au titre de l'attestation et tracée ici.
    // Une mutation qui se REPRODUIT malgré l'attestation pause encore
    // (fail-closed, voir plus bas).
    log(`${phaseName} : re-baseline stat-hash arbitré (${stepId}) — attestation : ${MUTATION_ACK[stepId]}`)
  }
  const results = await exec(stepId, phaseName, commands, `gate déterministe phase ${p} (${SPEC_SECTIONS.phase} + ${SPEC_SECTIONS.gates})`)
  if (isBrowser) {
    if (results[0].exitCode !== 0) {
      pause('browsers_running_during_gate', {
        phase: p, observed: results[0].outputTail,
        hint: 'GATE HUMAIN (délégable à l\'orchestrateur si autorisé) : fermer les navigateurs (le runner ne ferme jamais rien lui-même), puis relancer.',
      }, stepId)
    }
    // Vérifié AVANT le stat-hash : un navigateur lancé mid-fenêtre rend le
    // stat-hash ininterprétable — le diagnostic correct est celui-ci, pas
    // une accusation du script de gate.
    if (results[results.length - 1].exitCode !== 0) {
      pause('browsers_running_during_gate', {
        phase: p, observed: results[results.length - 1].outputTail,
        hint: 'Navigateur détecté en FIN de fenêtre de gate (lancé mid-fenêtre). Fermer le navigateur, vérifier les profils réels contre le backup, puis relancer.',
      }, stepId)
    }
    const profBefore = results[1].outputTail.trim()
    const profAfter = results[results.length - 2].outputTail.trim()
    if (profBefore !== profAfter) {
      if (!MUTATION_ACK[stepId]) {
        pause('live_profile_mutation_suspected', {
          phase: p, before: profBefore, after: profAfter,
          hint: `STOP : un fichier de profil réel a changé pendant la fenêtre de gate navigateurs-fermés. Vérifier le script de gate ET les profils réels contre le backup (fichiers comparés, integrity_check, wal). La reprise EXIGE : MUTATION_ACK["${stepId}"] = attestation de cette vérification, ET bump de RETRY_TOKENS["${stepId}"].`,
        }, stepId)
      }
      // Mutation encore observée au rejeu malgré l'attestation : la cause
      // persiste (ex : script de gate qui ouvre la base réelle). Fail-closed —
      // jamais d'acceptation silencieuse d'une mutation récurrente.
      pause('live_profile_mutation_recurred', {
        phase: p, before: profBefore, after: profAfter, ack: MUTATION_ACK[stepId],
        hint: 'La mutation se reproduit dans une fenêtre POSTÉRIEURE à l\'attestation : la cause racine persiste (script de gate ? processus externe ?). Corriger la cause, re-vérifier contre le backup, mettre à jour MUTATION_ACK et re-bumper le retryStep.',
      }, stepId)
    }
    const gateResults = results.slice(2, results.length - 2)
    const failures = gateResults.filter(r => r.exitCode !== 0)
    return { ok: failures.length === 0, failures, results }
  }
  const failures = results.filter(r => r.exitCode !== 0)
  return { ok: failures.length === 0, failures, results }
}

// --- Prompts agents non mécaniques ---------------------------------------------

// Règle durcie SQLite/WAL, formulée en OPÉRATIONS (post-incident phase 8 du
// run initial : ouvrir une base WAL en LECTURE crée/touche le -shm =>
// « lecture seule » est physiquement inviolable sur un profil surveillé par
// stat-hash). Présente dans le prompt de base de TOUTE phase à script de
// gate, de la phase des scripts d'acceptance finale, et des rounds de
// correction de ces phases.
const CP_ONLY_RULE =
  "RÈGLE DURCIE — scripts de gate et tests : ne JAMAIS ouvrir un fichier de profil navigateur réel avec SQLite ou tout parseur, même en lecture seule (l'ouverture d'une base WAL crée/touche places.sqlite-shm : mutation détectée par le driver => arrêt du chantier). La SEULE interaction permise avec un chemin de profil réel est une copie fichier (cp) vers un scratch, navigateurs fermés ; toute ouverture SQLite/parsing se fait UNIQUEMENT sur la copie. Cette règle s'applique aussi aux scripts d'acceptance finale."
function cpOnlyRuleApplies(p) {
  return Boolean(GATE_SCRIPTS[p]) || p === FINAL_SCRIPTS_PHASE || p === 'final'
}

// Embarque les chemins de scripts de gate (fixes, décidés d'avance) mais pas
// le détail des commandes PHASE_EXTRA_GATES : un ajustement de commande de
// gate n'invalide pas le cache des appels d'implémentation — seule la
// validation se ré-exécute.
function codexImplInstruction(p, attempt, lastValidation) {
  const lines = [
    `Tu implémentes la Phase ${p} de ${TASK} (${REPO}).`,
    `Contrat : ${SPEC}, section ${SPEC_SECTIONS.phase} « Phase ${p} », plus les sections ${SPEC_SECTIONS.contracts} pertinentes. Lis le fichier ; son contenu est gelé (le driver vérifie son sha256).`,
    'Règles non négociables :',
    '- Aucun commit, aucun push, aucune commande git mutante.',
    `- Aucune écriture hors de ${REPO}. Aucune écriture dans : ${PROTECTED.join(', ')}.`,
    '- Aucun profil navigateur réel touché — jamais.',
    '- Aucun refactor non demandé, aucune abstraction ou dépendance spéculative.',
    `Le gate à rendre vert : le gate spécifique de la phase (${SPEC_SECTIONS.phase}), target/ gitignoré, et ${JSON.stringify(CARGO_GATE)}.`,
  ]
  if (GATE_SCRIPTS[p]) {
    lines.push(
      `LIVRABLE OBLIGATOIRE de cette phase : le script de gate ${GATE_SCRIPTS[p].path} (exécuté par le driver via \`bash ${GATE_SCRIPTS[p].path}\`, exit 0 = vert).`,
      `Il doit prouver : ${GATE_SCRIPTS[p].deliverable}.`,
      `Il opère EXCLUSIVEMENT sur des copies scratch — lancements navigateur : ${SCRATCH_LAUNCH_RULES}, même en lecture-écriture accidentelle. Le driver vérifie les profils réels par stat-hash avant/après : toute mutation = arrêt du chantier.`,
    )
  }
  if (cpOnlyRuleApplies(p)) {
    lines.push(CP_ONLY_RULE)
  }
  if (p === FINAL_SCRIPTS_PHASE) {
    lines.push(
      `LIVRABLES OBLIGATOIRES supplémentaires : les scripts d'acceptance finale ${JSON.stringify(FINAL_ACCEPTANCE_SCRIPTS)} (mêmes règles scratch-only), qui ${FINAL_ACCEPTANCE_DESC}.`,
    )
  }
  if (attempt > 1 && lastValidation) {
    lines.push(
      `Tentative ${attempt}/${MAX_VALIDATION_ATTEMPTS}. Le gate a échoué. Échecs (commande, exitCode, fin de sortie) :`,
      JSON.stringify(lastValidation.failures, null, 2),
      'Corrige uniquement ce qui rend le gate vert. Pas de contournement des checks.',
    )
  }
  return lines.join('\n')
}

function discoveryPrompt(stepId, p, reviewBaseSha) {
  return [
    hdr(stepId),
    `Tu es la discovery review UNIQUE de la Phase ${p} de ${TASK} (${REPO}). Tu es STRICTEMENT read-only : aucune écriture de fichier, aucune commande git mutante, aucun sous-agent.`,
    `Cible : le diff \`git diff ${reviewBaseSha}\` (changements du worktree inclus — le driver n'a pas encore commité la phase).`,
    p === PHASES[PHASES.length - 1]
      ? `Phase ${p} : la review couvre le diff CUMULATIF de tout le rewrite depuis la base, pas seulement la documentation. Elle porte AUSSI le gate doc de ${SPEC_SECTIONS.phase} : vérifie qu'un nouvel utilisateur peut comparer, planifier, résoudre et appliquer sur des copies uniquement avec le README — sinon, finding avec le manque précis.`
      : `Contrat : ${SPEC} ${SPEC_SECTIONS.phase} Phase ${p} et les sections de contrat associées.`,
    // Méthodologie imposée, pas de branche conditionnelle : sur le run initial,
    // un « si disponible » a produit 4 reviews outillées sur 9 — seule
    // inconsistance méthodologique du run.
    "Invoque OBLIGATOIREMENT la skill code-review via l'outil Skill, au niveau xhigh. Seule exception : échec technique de l'invocation — effectue alors une review équivalente en lisant le diff et les fichiers, et signale cet échec dans le summary d'un finding.",
    "Un finding admissible DOIT nommer : emplacement exact, contrat/invariant violé, scénario d'échec concret, preuve (chemin de code ou reproducteur), résultat minimal requis.",
    "Une préférence de style, une amélioration hypothétique ou un problème préexistant n'est pas automatiquement un blocker de phase — signale-le comme tel dans summary si tu le rapportes.",
    'Retourne la liste (éventuellement vide) via le schéma. Ne modifie RIEN.',
  ].join('\n')
}

function factCheckInstruction(findings) {
  return [
    "MODE ANALYSE SEULE — fact-check des findings de la review. Tu ne modifies AUCUN fichier, tu n'anticipes AUCUNE correction.",
    'Findings :',
    JSON.stringify(findings, null, 2),
    'Pour CHAQUE finding :',
    '- regroupe les formulations dupliquées par cause racine (rootCauseId partagé) ;',
    `- vérifie chaque affirmation dans le code, ${SPEC} et les tests ;`,
    '- disposition : ACCEPT, REBUT, PREEXISTING, OUT_OF_PHASE ou UNCERTAIN ;',
    '- fournis des preuves (fichier:ligne, sortie de commande, citation du contrat) ;',
    '- propose uniquement le résultat minimal requis.',
    "Tu as naturellement intérêt à défendre ton code : un arbitre indépendant tranchera. Sois factuel.",
    'Ton MESSAGE FINAL doit être UNIQUEMENT un objet JSON de la forme {"dispositions": [{"findingId", "rootCauseId", "disposition", "evidence", "minimalRequiredOutcome"}]} — intégral, sans texte autour.',
  ].join('\n')
}

function arbitrationPrompt(stepId, p, findings, dispositions) {
  return [
    hdr(stepId),
    `Tu es l'arbitre compliance FERMÉ de la Phase ${p} de ${TASK} (${REPO}). Tu es STRICTEMENT read-only (lecture de fichiers et commandes git en lecture seule autorisées pour vérifier les preuves).`,
    "Entrées — findings de la discovery review, puis dispositions du fact-check de l'implémenteur :",
    JSON.stringify({ findings, dispositions }, null, 2),
    'Ton mandat est FERMÉ. Pour chaque finding, tu peux seulement :',
    "- REQUIRED_FIX : exiger une correction (avec requiredOutcome minimal) ;",
    '- DISMISSED : valider une réfutation ou déclasser (préexistant, hors phase, style) ;',
    "- ESCALATE : demander un arbitrage humain faute de preuve suffisante.",
    "INTERDIT : recommencer une recherche générale de problèmes, ajouter des findings, émettre un verdict sur un findingId absent des entrées. Zéro REQUIRED_FIX est un résultat valide.",
    `Référentiel : ${SPEC} (contrats ${SPEC_SECTIONS.contracts}, phase ${SPEC_SECTIONS.phase}, tests ${SPEC_SECTIONS.tests}).`,
  ].join('\n')
}

function correctionInstruction(p, fixes, gateFailures, round) {
  const lines = [
    `Corrections arbitrées (round ${round}/${MAX_COMPLIANCE_RETRIES}). Applique UNIQUEMENT ces exigences :`,
    JSON.stringify(fixes, null, 2),
    'Règles :',
    '- Corriger uniquement les causes arbitrées.',
    '- Faire le plus petit changement correct.',
    '- Aucun cleanup ou refactor non lié.',
    '- Aucune abstraction ou dépendance spéculative.',
    '- Ajouter uniquement les tests nécessaires à la preuve.',
    '- Ne pas commit, ne pas push.',
  ]
  // La règle durcie couvre aussi les rounds de correction : un fix.N des
  // phases à script de gate travaille sous les mêmes contraintes que l'impl.
  if (cpOnlyRuleApplies(p)) {
    lines.push(CP_ONLY_RULE)
  }
  if (gateFailures && gateFailures.length) {
    lines.push(
      'De plus, le gate déterministe a échoué après la correction précédente. Échecs à résoudre sans élargir le scope :',
      JSON.stringify(gateFailures, null, 2),
    )
  }
  return lines.join('\n')
}

function verificationPrompt(stepId, p, ledger, reviewBaseSha) {
  return [
    hdr(stepId),
    `Vérification compliance FERMÉE, Phase ${p}, ${TASK} (${REPO}). STRICTEMENT read-only.`,
    "Ledger arbitré (findings, dispositions du fact-check, verdicts d'arbitrage) :",
    JSON.stringify(ledger, null, 2),
    `Examine le diff de correction : \`git diff ${reviewBaseSha}\` (worktree inclus).`,
    'Règles :',
    '- Vérifier seulement que chaque REQUIRED_FIX du ledger est satisfait (requiredOutcome atteint).',
    '- Ne pas refaire une code review ouverte. Ne pas introduire de finding sans lien.',
    '- Un nouveau blocker exige une régression directement causée par la correction, ou un risque prouvé de perte/corruption de données — et doit référencer un findingId du ledger ou être signalé via PAUSE.',
    'Verdict : PASS (tout satisfait), REOPEN (liste reopened), ou PAUSE (arbitrage humain requis).',
  ].join('\n')
}

function finalSafetyAuditPrompt(stepId, rewriteBaseSha) {
  return [
    hdr(stepId),
    `Audit final de sécurité des données, ${TASK} (${REPO}). STRICTEMENT read-only.`,
    `Diff complet du rewrite : \`git diff ${rewriteBaseSha}..HEAD\`. Référentiel : ${SPEC} ${SPEC_SECTIONS.safetyAudit}.`,
    "Focus exclusif — risques de données, rien d'autre :",
    '- frontières des chemins de profils réels ;',
    '- backup vérifié avant toute écriture ;',
    '- détection des navigateurs actifs ;',
    "- ordre des écritures ;",
    '- rollback et crash recovery ;',
    '- compensation two-way ;',
    "- absence d'opération Delete ;",
    '- absence de TypeScript productif.',
    'Ne rapporte que des findings BLOQUANTS prouvés sur ces axes. Zéro finding est un résultat valide (PASS).',
  ].join('\n')
}

// --- Résolution des findings (fact-check -> arbitrage -> corrections fermées) ---

async function resolveFindings(tag, phaseName, p, findings, sessionId, reviewBaseSha, baseHead) {
  const findingIds = new Set(findings.map(f => f.id))

  // Fact-check : même session Codex, analyse seule ; dispositions transmises
  // via finalOutput (verbatim), jamais via summary.
  const fcBefore = await fingerprint(`${tag}.factcheck.before`, phaseName)
  const fcRes = await codex(`${tag}.factcheck`, phaseName, sessionId, factCheckInstruction(findings))
  const fcAfter = await fingerprint(`${tag}.factcheck.after`, phaseName)
  assertUnchanged(fcBefore, fcAfter, 'factcheck_mutated_repo', p, `${tag}.factcheck`)

  const parsed = await agent([
    hdr(`${tag}.factcheck.parse`),
    'Parseur mécanique. Voici la sortie finale brute du fact-check ; extrais les dispositions telles quelles, sans interprétation ni complétion :',
    fcRes.finalOutput,
    'Si la sortie ne contient pas une disposition claire pour chacun de ces findingIds, retourne une liste vide :',
    JSON.stringify(findings.map(f => f.id)),
  ].join('\n'), { schema: DISPOSITIONS_SCHEMA, label: `${tag}.factcheck.parse`, phase: phaseName, ...MECH })
  if (!parsed) pause('factcheck_parse_failed', { tag }, `${tag}.factcheck.parse`)
  const dispositions = parsed.dispositions
  const missing = findings.filter(f => !dispositions.some(d => d.findingId === f.id))
  if (missing.length) {
    pause('factcheck_incomplete', { phase: p, missing: missing.map(f => f.id) }, `${tag}.factcheck`)
  }

  // Arbitrage fermé — agent frais, mandat clos
  const arbBefore = await fingerprint(`${tag}.arbitration.before`, phaseName)
  const arb = await agent(arbitrationPrompt(`${tag}.arbitration`, p, findings, dispositions), {
    schema: ARBITRATION_SCHEMA, label: `${tag}.arbitration`, phase: phaseName, ...REVIEW,
  })
  if (!arb) pause('arbitration_failed', { tag }, `${tag}.arbitration`)
  const arbAfter = await fingerprint(`${tag}.arbitration.after`, phaseName)
  assertUnchanged(arbBefore, arbAfter, 'arbitration_mutated_repo', p, `${tag}.arbitration`)

  // Containment du mandat fermé : le driver, pas le prompt, garantit le périmètre.
  const foreign = arb.verdicts.filter(v => !findingIds.has(v.findingId))
  if (foreign.length) {
    pause('arbitration_out_of_mandate', { phase: p, foreign }, `${tag}.arbitration`)
  }

  // Escalades : canal de décision humaine HUMAN_ARBITRATION (lu par le
  // driver seul — n'invalide pas le cache).
  const escalations = arb.verdicts.filter(v => v.verdict === 'ESCALATE')
  const unresolved = escalations.filter(v => !humanDecision(v.findingId))
  if (unresolved.length) {
    pause('finding_requires_human_arbitration', {
      phase: p,
      escalations: unresolved,
      hint: 'GATE HUMAIN : éditer HUMAN_ARBITRATION en tête du script — [findingId] = {verdict:"REQUIRED_FIX"|"DISMISSED", requiredOutcome?} pour chaque escalade — puis Workflow({scriptPath, resumeFromRunId}). HUMAN_ARBITRATION n\'est lu que par le driver : le cache antérieur est préservé.',
    })
  }
  const humanRequired = escalations
    .filter(v => humanDecision(v.findingId).verdict === 'REQUIRED_FIX')
    .map(v => ({
      findingId: v.findingId,
      verdict: 'REQUIRED_FIX',
      reason: `arbitrage humain (escalade : ${v.reason})`,
      requiredOutcome: humanDecision(v.findingId).requiredOutcome || v.reason,
    }))
  const required = [...arb.verdicts.filter(v => v.verdict === 'REQUIRED_FIX'), ...humanRequired]
  log(`${phaseName} : ${findings.length} findings, ${required.length} REQUIRED_FIX, ${escalations.length} escalade(s) (${unresolved.length} non résolue(s))`)
  if (!required.length) return

  const ledger = { findings, dispositions, verdicts: arb.verdicts, humanArbitration: humanRequired }
  let fixes = required
  let gateFailures = null

  for (let round = 0; round <= MAX_COMPLIANCE_RETRIES; round++) {
    await codex(`${tag}.fix.${round}`, phaseName, sessionId, correctionInstruction(p, fixes, gateFailures, round))

    const fpFix = await fingerprint(`${tag}.fix.${round}.fp`, phaseName)
    if (fpFix.head !== baseHead) {
      pause('unexpected_commit_during_correction', { phase: p, round }, `${tag}.fix.${round}.fp`)
    }
    if (fpFix.protectedSha256 !== fcBefore.protectedSha256) {
      pause('protected_paths_modified', { phase: p, round }, `${tag}.fix.${round}.fp`)
    }

    const validation = await validatePhase(p, phaseName, `${tag}.fix.${round}.val`)
    if (!validation.ok) {
      if (round === MAX_COMPLIANCE_RETRIES) {
        pause('correction_validation_failed', { phase: p, failures: validation.failures }, `${tag}.fix.${round}.val`)
      }
      // Les échecs de gate ne remplacent PAS les fixes arbitrés : ils s'y
      // ajoutent au round suivant (déviation assumée vs pseudo-code spec).
      gateFailures = validation.failures
      continue
    }
    gateFailures = null

    const vBefore = await fingerprint(`${tag}.verify.${round}.before`, phaseName)
    const verif = await agent(verificationPrompt(`${tag}.verify.${round}`, p, ledger, reviewBaseSha), {
      schema: VERIFY_SCHEMA, label: `${tag}.verify.${round}`, phase: phaseName, ...REVIEW,
    })
    if (!verif) pause('verification_failed', { tag, round }, `${tag}.verify.${round}`)
    const vAfter = await fingerprint(`${tag}.verify.${round}.after`, phaseName)
    assertUnchanged(vBefore, vAfter, 'verification_mutated_repo', p, `${tag}.verify.${round}`)

    if (verif.verdict === 'PASS') return
    if (verif.verdict === 'PAUSE') {
      pause('compliance_requested_pause', { phase: p, verif }, `${tag}.verify.${round}`)
    }
    // Containment + décision humaine sur les rouverts.
    const foreignReopened = verif.reopened.filter(r => !findingIds.has(r.findingId))
    if (foreignReopened.length) {
      pause('verification_out_of_mandate', { phase: p, foreignReopened }, `${tag}.verify.${round}`)
    }
    const reopenedEffective = verif.reopened.filter(r => {
      const d = humanDecision(r.findingId)
      return !(d && d.verdict === 'DISMISSED')
    })
    if (!reopenedEffective.length) {
      log(`${phaseName} : tous les findings rouverts sont DISMISSED par arbitrage humain — PASS`)
      return
    }
    if (round === MAX_COMPLIANCE_RETRIES) {
      pause('compliance_did_not_converge', {
        phase: p,
        reopened: reopenedEffective,
        hint: 'GATE HUMAIN : éditer HUMAN_ARBITRATION[findingId]={verdict:"DISMISSED"} pour clore, ou bump de RETRY_TOKENS[retryStep] pour un round de vérification live.',
      }, `${tag}.verify.${round}`)
    }
    fixes = reopenedEffective
  }
}

// --- Cycle d'une phase ----------------------------------------------------------

async function runPhase(p, phaseName, baseline, rewriteBaseSha) {
  // Backup re-vérifié en entrée des phases destructrices (live au front de
  // reprise — porte assertBackupStillVerified de la spec).
  if (BACKUP_RECHECK_PHASES.includes(p)) {
    const bkp = await exec(`p${p}.backupcheck`, phaseName, [
      `test -d "${BACKUP_DIR}"`,
      `cd "${BACKUP_DIR}" && sha256sum -c MANIFEST.sha256`,
    ], `re-vérification du backup avant la phase destructrice ${p}`)
    if (!bkp.every(r => r.exitCode === 0)) {
      pause('backup_invalid', { phase: p, bkp }, `p${p}.backupcheck`)
    }
  }

  const base = await fingerprint(`p${p}.base`, phaseName)
  if (base.specSha256 !== baseline.specSha256) {
    pause('spec_changed_mid_run', { phase: p, expected: baseline.specSha256, got: base.specSha256 }, `p${p}.base`)
  }

  // Implémentation / validation — passes bornées, session Codex fraîche par phase
  let sessionId = null
  let validation = null
  for (let attempt = 1; attempt <= MAX_VALIDATION_ATTEMPTS; attempt++) {
    const res = await codex(
      `p${p}.impl.${attempt}`, phaseName,
      attempt === 1 ? null : sessionId,
      codexImplInstruction(p, attempt, validation),
    )
    sessionId = res.sessionId

    const fpNow = await fingerprint(`p${p}.impl.${attempt}.fp`, phaseName)
    if (fpNow.head !== base.head) pause('agent_commit_detected', { phase: p, attempt }, `p${p}.impl.${attempt}.fp`)
    if (fpNow.indexSha256 !== base.indexSha256) pause('agent_staged_changes', { phase: p, attempt }, `p${p}.impl.${attempt}.fp`)
    if (fpNow.protectedSha256 !== base.protectedSha256) {
      pause('protected_paths_modified', { phase: p, attempt }, `p${p}.impl.${attempt}.fp`)
    }

    validation = await validatePhase(p, phaseName, `p${p}.val.${attempt}`)
    log(`${phaseName} : validation tentative ${attempt} → ${validation.ok ? 'VERT' : `${validation.failures.length} échec(s)`}`)
    if (validation.ok) break
    if (attempt === MAX_VALIDATION_ATTEMPTS) {
      pause('phase_validation_failed', { phase: p, failures: validation.failures }, `p${p}.val.${attempt}`)
    }
  }

  // Discovery review unique — jamais relancée après correction
  const reviewBaseSha = p === PHASES[PHASES.length - 1] ? rewriteBaseSha : base.head
  const rBefore = await fingerprint(`p${p}.review.before`, phaseName)
  const review = await agent(discoveryPrompt(`p${p}.review`, p, reviewBaseSha), {
    schema: FINDINGS_SCHEMA, label: `p${p}.review`, phase: phaseName, ...REVIEW,
  })
  if (!review) pause('discovery_review_failed', { phase: p }, `p${p}.review`)
  const rAfter = await fingerprint(`p${p}.review.after`, phaseName)
  assertUnchanged(rBefore, rAfter, 'discovery_review_mutated_repo', p, `p${p}.review`)
  log(`${phaseName} : discovery review → ${review.findings.length} finding(s)`)

  if (review.findings.length > 0) {
    await resolveFindings(`p${p}`, phaseName, p, review.findings, sessionId, reviewBaseSha, base.head)
  }

  // Commit par le driver seul. Garde-fou anti-pollution AVANT le commit, en
  // chaîne court-circuitée (une seule commande) : si un chemin protégé ou
  // target/ se retrouve stagé, le commit n'a pas lieu.
  await exec(`p${p}.commit`, phaseName, [
    `bash -c 'git add -A -- . ${PROTECT_EXCLUDES} && ! git diff --cached --name-only | grep -E "${GUARD_PATTERN}" && git commit -m "${COMMIT_SUBJECT(p)}"'`,
  ], `commit driver phase ${p} (add + garde anti-pollution + commit)`)

  // Le succès du commit est jugé sur l'ÉTAT OBSERVÉ, pas sur l'exit code
  // rapporté (idempotence : une reprise après mésrapport ne re-commite pas).
  const post = await fingerprint(`p${p}.post`, phaseName)
  const committed = post.head !== base.head && post.statusSha256 === baseline.statusSha256
  if (!committed) {
    // Le test protectedSha256 distingue une édition utilisateur mid-run d'un
    // résidu d'agent : les chemins protégés ne sont jamais touchés par un
    // agent conforme, mais l'utilisateur peut y avoir écrit.
    pause('driver_commit_failed', {
      phase: p,
      headMoved: post.head !== base.head,
      worktreeClean: post.statusSha256 === baseline.statusSha256,
      protectedChanged: post.protectedSha256 !== baseline.protectedSha256,
      hint: post.head === base.head
        ? 'Le commit n\'a pas eu lieu (garde anti-pollution ? rien à committer ?). Inspecter l\'index, corriger, bumper le retryStep.'
        : post.protectedSha256 !== baseline.protectedSha256
          ? 'Commit effectué mais un chemin PROTÉGÉ a changé depuis la baseline — vraisemblablement une édition utilisateur mid-run, pas un résidu d\'agent. Vérifier, restaurer ou accepter (la baseline protégée ne sera pas re-figée), puis bumper le retryStep.'
          : 'Commit effectué mais résidu hors chemins protégés dans le worktree (fichier non stagé ? script de gate mal placé ?). Corriger puis bumper le retryStep.',
    }, `p${p}.post`)
  }
  return { commitSha: post.head, sessionId }
}

// --- Main -----------------------------------------------------------------------

try {
  // Préflight ------------------------------------------------------------------
  phase('Préflight')
  log('Préflight : index, baseline, toolchain, backup')

  // 1. Index vide EXIGÉ AVANT la capture de baseline : un commit driver
  // embarque tout l'index, et la baseline doit refléter l'arbre remédié.
  // (Si ce gate pause, la remédiation humaine précède la 1re capture de
  // baseline — pas de baseline empoisonnée au rejeu.)
  const idx = await exec('preflight.index', 'Préflight', ['git diff --cached --name-only'], 'inventaire index')
  if ((idx[0].outputTail || '').trim() !== '') {
    pause('index_not_clean', {
      staged: idx[0].outputTail,
      hint: 'GATE HUMAIN : committer ou dé-stager ces changements utilisateur, puis relancer en bumpant RETRY_TOKENS["preflight.index"] — la baseline sera capturée après.',
    }, 'preflight.index')
  }

  // 2. Baseline figée du run : head de base du rewrite, status de référence
  // (changements protégés seuls), sha du spec, contenu protégé.
  const baseline = await fingerprint('preflight.fp', 'Préflight')

  const tool = await exec('preflight.toolchain', 'Préflight', TOOLCHAIN_CMDS,
    `vérification toolchain Rust ${TOOLCHAIN_VERSION} + rustfmt + clippy`)
  const toolOk = tool.every(r => r.exitCode === 0) && tool.slice(0, 2).every(r => r.outputTail.includes(TOOLCHAIN_VERSION))
  if (!toolOk) {
    pause('rust_toolchain_missing', {
      observed: tool,
      hint: `GATE HUMAIN : installer Rust ${TOOLCHAIN_VERSION} (rustup) avec rustfmt et clippy, puis relancer en bumpant RETRY_TOKENS["preflight.toolchain"]. Le workflow n'installe RIEN lui-même.`,
    }, 'preflight.toolchain')
  }

  if (!BACKUP_DIR) {
    pause('backup_not_declared', { hint: BACKUP_CREATE_HINT }, 'preflight.backup')
  }
  const bkp = await exec('preflight.backup', 'Préflight', [
    `test -d "${BACKUP_DIR}"`,
    `cd "${BACKUP_DIR}" && sha256sum -c MANIFEST.sha256`,
  ], 'vérification du backup pré-rewrite')
  if (!bkp.every(r => r.exitCode === 0)) {
    pause('backup_invalid', { bkp }, 'preflight.backup')
  }

  log(`Préflight OK — base rewrite : ${baseline.head}, baseline worktree figée`)

  // Phases -----------------------------------------------------------------------
  const committed = {}
  const sessions = {}
  // Base du diff cumulatif (dernière phase + audit final). Corrigée ci-dessous
  // si un redémarrage à froid détecte des phases déjà committées.
  let rewriteBaseSha = baseline.head
  for (const p of PHASES) {
    const phaseName = `Phase ${p}`
    phase(phaseName)

    // Robustesse au redémarrage à froid (nouvelle session, cache perdu) :
    // l'état durable vit dans git — une phase dont le commit driver existe
    // n'est JAMAIS rejouée. Comparaison exacte du sujet, faite par le JS.
    const marks = await exec(`p${p}.committed-check`, phaseName, [
      'git log --format="%H %P %s" -n 50',
    ], `phase ${p} déjà committée ? (reprise à froid)`)
    const markLine = (marks[0].outputTail || '')
      .split('\n')
      .map(l => l.trim())
      .find(l => {
        const parts = l.split(' ')
        return parts.slice(2).join(' ') === COMMIT_SUBJECT(p)
      })
    if (markLine) {
      const parts = markLine.split(' ')
      committed[p] = parts[0]
      sessions[p] = null // session Codex de l'ancien run : reprise non garantie
      if (p === PHASES[0]) rewriteBaseSha = parts[1] // parent du commit de la 1re phase = base du rewrite
      log(`${phaseName} déjà committée (${committed[p]}) — sautée`)
      continue
    }

    const r = await runPhase(p, phaseName, baseline, rewriteBaseSha)
    committed[p] = r.commitSha
    sessions[p] = r.sessionId
    log(`${phaseName} committée : ${r.commitSha}`)
  }

  // Acceptance finale --------------------------------------------------------------
  phase('Acceptance finale')
  const finalBackup = await exec('final.backupcheck', 'Acceptance finale', [
    `cd "${BACKUP_DIR}" && sha256sum -c MANIFEST.sha256`,
  ], 'backup toujours vérifié avant l\'acceptance finale')
  if (!finalBackup.every(r => r.exitCode === 0)) pause('backup_invalid', { finalBackup }, 'final.backupcheck')

  const finalCargo = await exec('final.cargo', 'Acceptance finale', [...HYGIENE_GATE, ...CARGO_GATE], 'validation Cargo globale finale')
  // Fenêtre navigateur finale : mêmes protections que les gates de phase
  // (check navigateurs aux deux extrémités, paire stat-hash, MUTATION_ACK).
  if (MUTATION_ACK['final.browsers']) {
    log(`Acceptance finale : re-baseline stat-hash arbitré (final.browsers) — attestation : ${MUTATION_ACK['final.browsers']}`)
  }
  const finalBrowserCmds = [BROWSERS_CLOSED_CMD, PROFILE_STAT_CMD, ...FINAL_BROWSER_GATES, PROFILE_STAT_CMD, BROWSERS_CLOSED_CMD]
  const finalBrowsers = await exec('final.browsers', 'Acceptance finale', finalBrowserCmds, 'validations navigateur finales sur copies scratch')
  if (finalBrowsers[0].exitCode !== 0) {
    pause('browsers_running_during_gate', { observed: finalBrowsers[0].outputTail }, 'final.browsers')
  }
  if (finalBrowsers[finalBrowsers.length - 1].exitCode !== 0) {
    pause('browsers_running_during_gate', {
      observed: finalBrowsers[finalBrowsers.length - 1].outputTail,
      hint: 'Navigateur détecté en FIN de fenêtre finale (lancé mid-fenêtre). Stat-hash ininterprétable — fermer, vérifier contre le backup, relancer.',
    }, 'final.browsers')
  }
  if (finalBrowsers[1].outputTail.trim() !== finalBrowsers[finalBrowsers.length - 2].outputTail.trim()) {
    if (!MUTATION_ACK['final.browsers']) {
      pause('live_profile_mutation_suspected', {
        before: finalBrowsers[1].outputTail.trim(), after: finalBrowsers[finalBrowsers.length - 2].outputTail.trim(),
        hint: 'STOP : un fichier de profil réel a changé pendant la fenêtre finale navigateurs-fermés. Vérifier les scripts ET les profils contre le backup (fichiers comparés, integrity_check, wal). La reprise EXIGE : MUTATION_ACK["final.browsers"] = attestation de cette vérification, ET bump de RETRY_TOKENS["final.browsers"].',
      }, 'final.browsers')
    }
    pause('live_profile_mutation_recurred', {
      ack: MUTATION_ACK['final.browsers'],
      hint: 'La mutation se reproduit malgré l\'attestation : la cause racine persiste. Corriger la cause, re-vérifier contre le backup, mettre à jour MUTATION_ACK et re-bumper le retryStep.',
    }, 'final.browsers')
  }
  const finalFailures = [
    ...finalCargo.filter(r => r.exitCode !== 0),
    ...finalBrowsers.slice(2, finalBrowsers.length - 2).filter(r => r.exitCode !== 0),
  ]
  if (finalFailures.length) pause('final_validation_failed', { failures: finalFailures }, 'final.cargo')

  const beforeAudit = await fingerprint('final.fp', 'Acceptance finale')
  const audit = await agent(finalSafetyAuditPrompt('final.audit', rewriteBaseSha), {
    schema: FINDINGS_SCHEMA, label: 'final.audit', phase: 'Acceptance finale', ...REVIEW,
  })
  if (!audit) pause('final_audit_failed', {}, 'final.audit')
  const afterAudit = await fingerprint('final.audit.fp', 'Acceptance finale')
  assertUnchanged(beforeAudit, afterAudit, 'final_audit_mutated_repo', 'final', 'final.audit')

  if (audit.findings.length > 0) {
    // État séparé de la dernière phase (tag 'final') : pas d'écrasement du ledger.
    await resolveFindings('final', 'Acceptance finale', 'final', audit.findings, sessions[PHASES[PHASES.length - 1]], rewriteBaseSha, beforeAudit.head)
    await exec('final.commit', 'Acceptance finale', [
      `bash -c 'git add -A -- . ${PROTECT_EXCLUDES} && ! git diff --cached --name-only | grep -E "${GUARD_PATTERN}" && git commit -m "${FINAL_FIX_COMMIT_SUBJECT}"'`,
    ], 'commit des corrections de l\'audit final')
    const postFix = await fingerprint('final.commit.fp', 'Acceptance finale')
    if (postFix.head === beforeAudit.head || postFix.statusSha256 !== baseline.statusSha256) {
      pause('final_fix_commit_failed', { postFix }, 'final.commit.fp')
    }
    const recheck = await exec('final.recheck', 'Acceptance finale', [...HYGIENE_GATE, ...CARGO_GATE], 're-validation Cargo après corrections d\'audit')
    if (!recheck.every(r => r.exitCode === 0)) {
      pause('final_revalidation_failed', { recheck }, 'final.recheck')
    }
  }

  const done = await fingerprint('final.done', 'Acceptance finale')
  if (done.statusSha256 !== baseline.statusSha256) {
    pause('worktree_not_clean_at_end', { hint: 'Résidu non commité hors chemins protégés.' }, 'final.done')
  }

  log('ready_for_human_validation — aucun push effectué, validation produit humaine requise')
  return {
    status: 'ready_for_human_validation',
    rewriteBaseSha,
    finalHead: done.head,
    phases: committed,
    codexSessions: sessions,
    humanGateRemaining: `Validation produit humaine finale (${SPEC} ${SPEC_SECTIONS.humanValidation}) puis publication manuelle.`,
  }
} catch (e) {
  if (e instanceof Pause) {
    log(`PAUSE : ${e.reason}${e.retryStep ? ` (retryStep: ${e.retryStep})` : ''}`)
    return {
      status: 'paused',
      reason: e.reason,
      detail: e.detail,
      retryStep: e.retryStep || null,
      howToResume: e.retryStep
        ? `1) Traiter la cause. 2) Éditer ce script : RETRY_TOKENS["${e.retryStep}"] = <valeur actuelle + 1>. 3) Workflow({scriptPath, resumeFromRunId}). Ne JAMAIS reprendre sans bump : le résultat d'échec serait rejoué depuis le cache et la pause bouclerait. Le paramètre args est ignoré à la reprise — le fichier est le seul canal.`
        : "1) Traiter la cause. 2) Éditer la constante indiquée par le hint (BACKUP_DIR / HUMAN_ARBITRATION / MUTATION_ACK) en tête de ce script. 3) Workflow({scriptPath, resumeFromRunId}). Ces constantes ne sont lues que par le driver : le cache antérieur est préservé.",
    }
  }
  throw e
}
