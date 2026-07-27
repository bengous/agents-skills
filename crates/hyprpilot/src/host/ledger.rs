//! What a session did to the user's compositor, and how each of it comes back.
//!
//! Five of the crate's six durable host mutations used to be posed with no
//! record of how to retract them. One of them renamed a workspace slot the user
//! owned and left the dead name in waybar until waybar was restarted; another
//! leaked a workspace rule per start, thirty of them by the time it was found.
//! Both are the same defect: a mutation posed on the host without deciding its
//! undo.
//!
//! `HostMutation::undo` is an exhaustive match, so that decision is no longer
//! optional — a variant added without an arm does not compile, and an arm that
//! cannot undo anything has to say what it leaks and how the user clears it.

use serde::{Deserialize, Serialize};

use crate::error::{Error, RestoreFailure};
use crate::session;

/// What clears a `keyword` this crate posed. Hyprland has no runtime retraction
/// (hyprwm/Hyprland#5691), and reposting a rule stacks it instead of replacing
/// it, the first one keeping precedence (#2268) — so a keyword can be *not
/// posed*, or posed at most once, but never taken back.
const RELOAD: &str = "hyprctl reload";

/// One durable change this session made to the user's compositor. Persisted in
/// the session state before it is applied, so a state file always describes at
/// least as much as the host actually holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mutation", rename_all = "snake_case")]
pub enum HostMutation {
    /// A headless output this crate created. The one mutation that was already
    /// undone before this ledger existed.
    OutputCreated { output: String },
    /// A `monitor` mode-set rule. Unretractable, and unavoidable with it:
    /// `hyprctl output create headless <name>` takes no size, so the mode can
    /// only come from a keyword.
    MonitorRuleSet { rule: String },
    /// A `workspace` rule binding a workspace to an output. Unretractable, so
    /// the defence is to pose it at most once.
    WorkspaceRuleSet { rule: String },
    /// A workspace the host attached to a new output and this crate renamed.
    /// `from` is in the type: the original name is no longer recoverable only
    /// from the compositor, which is how it used to be lost for good.
    WorkspaceRenamed { id: i64, from: String, to: String },
    /// A workspace pushed off an output of ours onto one of the user's
    /// monitors. `from` is the monitor it sat on before the move, so the undo
    /// puts it back where `output remove` can rehome it as if nothing had
    /// happened.
    WorkspaceMovedToMonitor { workspace: String, from: String },
}

/// When an undo may run, relative to the removal of the output. The output goes
/// last and only once nothing is left on it, so anything that has to happen
/// *while* it still exists — a workspace getting its name back, a workspace
/// coming home to it — runs before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoPhase {
    BeforeTheOutput,
    TheOutput,
}

/// What undoing one host mutation produced.
#[derive(Debug)]
pub enum Undo {
    /// Retracted. `notes` say what changed on the host; `failure` is what the
    /// undo asked for and could not get.
    Done {
        notes: Vec<String>,
        failure: Option<RestoreFailure>,
    },
    /// Hyprland cannot retract this one while it runs, so the leak is named
    /// rather than left silent: `what` stays behind, `remedy` clears it.
    Impossible { what: String, remedy: &'static str },
}

/// The cursor-preserving output removal both modes' teardown already share
/// (fact §2.8), as a type so the seam below stays readable.
type RemoveOutput<'a> =
    &'a dyn Fn(&str, Option<(i32, i32)>) -> Result<session::OutputRemoval, Error>;

/// The compositor effects an undo needs, injected so every arm of the match
/// below can be driven without a live host — the same seam `Exit`, `Sweep` and
/// `TeardownEffects` already use.
pub struct UndoEffects<'a> {
    pub remove_output: RemoveOutput<'a>,
    pub dispatch: &'a dyn Fn(&[&str]) -> Result<(), Error>,
}

pub fn live_undo_effects() -> UndoEffects<'static> {
    UndoEffects {
        remove_output: &session::remove_output_restoring_cursor,
        dispatch: &super::hypr::dispatch,
    }
}

impl HostMutation {
    /// Exhaustive like `undo`: a mutation added without deciding whether its
    /// undo still needs the output does not compile. Getting this wrong is the
    /// waybar defect itself — a workspace renamed back *after* its output was
    /// removed is a workspace that never gets its name back.
    pub fn phase(&self) -> UndoPhase {
        match self {
            Self::OutputCreated { .. } => UndoPhase::TheOutput,
            Self::MonitorRuleSet { .. }
            | Self::WorkspaceRuleSet { .. }
            | Self::WorkspaceRenamed { .. }
            | Self::WorkspaceMovedToMonitor { .. } => UndoPhase::BeforeTheOutput,
        }
    }

    /// What this mutation leaves on the host that nothing can retract, or
    /// `None` when `undo` takes it back. Exhaustive like the two matches around
    /// it, and the same answer `undo` gives — asserted, since two matches can
    /// drift where one cannot.
    pub fn leak(&self) -> Option<String> {
        match self {
            Self::MonitorRuleSet { rule } => Some(format!("monitor rule `{rule}`")),
            Self::WorkspaceRuleSet { rule } => Some(format!("workspace rule `{rule}`")),
            Self::OutputCreated { .. }
            | Self::WorkspaceRenamed { .. }
            | Self::WorkspaceMovedToMonitor { .. } => None,
        }
    }

    /// How this mutation comes back. `cursor` is the position to warp back to
    /// when `output remove` re-centres it and `cursorpos` cannot be read at the
    /// last moment (fact §2.8): a rolled-back start passes the snapshot it took
    /// before its first mutation, a teardown has none to offer.
    ///
    /// Only the output removal propagates an `Err`, because it is the one undo
    /// whose failure must stop the caller — the session state has to survive for
    /// `teardown` to retry. A workspace that will not come back is reported
    /// alongside the rest instead: holding up the output removal over it would
    /// leave the user with a compositing headless output.
    pub fn undo(
        &self,
        effects: &UndoEffects<'_>,
        cursor: Option<(i32, i32)>,
    ) -> Result<Undo, Error> {
        match self {
            Self::OutputCreated { output } => {
                let removal = (effects.remove_output)(output, cursor)?;
                Ok(Undo::Done {
                    notes: removal.notes,
                    failure: removal.failure,
                })
            }
            Self::MonitorRuleSet { rule } => Ok(Undo::Impossible {
                what: format!("monitor rule `{rule}`"),
                remedy: RELOAD,
            }),
            // Posed at most once (`Ledger::bind_workspace`), because that is the
            // only defence: reposting stacks a second one (#2268) and nothing
            // retracts either (#5691).
            Self::WorkspaceRuleSet { rule } => Ok(Undo::Impossible {
                what: format!("workspace rule `{rule}`"),
                remedy: RELOAD,
            }),
            Self::WorkspaceRenamed { id, from, to } => Ok(dispatched(
                (effects.dispatch)(&["renameworkspace", &id.to_string(), from]),
                format!("workspace {id} (`{to}`) is named `{from}` again"),
                "renamed workspace",
                format!("workspace {id} named `{from}` again"),
            )),
            Self::WorkspaceMovedToMonitor { workspace, from } => Ok(dispatched(
                (effects.dispatch)(&[
                    "moveworkspacetomonitor",
                    &session::workspace_selector(workspace),
                    from,
                ]),
                format!("workspace {workspace} is back on {from}"),
                "evacuated workspace",
                format!("workspace {workspace} back on {from}"),
            )),
        }
    }
}

/// What unwinding a ledger did, and what it could not.
#[derive(Debug, Default)]
pub struct Unwound {
    /// What actually changed on the host, for the teardown message.
    pub notes: Vec<String>,
    /// What Hyprland cannot retract while it runs, each with the command that
    /// clears it. Not failures: they are the documented cost of a `keyword`
    /// (#5691), and the point of listing them is that the leak is visible
    /// instead of silent.
    pub leaked: Vec<String>,
    pub failures: Vec<RestoreFailure>,
    /// Set when an undo could not run at all. Everything still ahead of it in
    /// the ledger is untouched, so the state has to stay on disk for a later
    /// `teardown` to resume from.
    pub stopped: Option<RestoreFailure>,
}

/// Undoes a ledger in reverse, which is the only order that holds: a workspace
/// gets its name back, and a pushed-off workspace comes home, *before* the
/// output that caused either is removed.
pub fn unwind(
    effects: &UndoEffects<'_>,
    ledger: &[HostMutation],
    cursor: Option<(i32, i32)>,
) -> Unwound {
    let mut unwound = Unwound::default();
    for mutation in ledger.iter().rev() {
        match mutation.undo(effects, cursor) {
            Ok(Undo::Done { notes, failure }) => {
                unwound.notes.extend(notes);
                unwound.failures.extend(failure);
            }
            Ok(Undo::Impossible { what, remedy }) => unwound
                .leaked
                .push(format!("{what} stays until `{remedy}`")),
            Err(error) => {
                unwound.stopped = Some(RestoreFailure {
                    what: "host mutation",
                    expected: format!("{mutation:?} undone"),
                    actual: error.to_string(),
                });
                return unwound;
            }
        }
    }
    unwound
}

/// The entries whose undo still needs the output. The output removal itself is
/// left to the caller, because it has to come last and behind a check that
/// nothing of the user's is on the output any more.
pub fn before_the_output(ledger: &[HostMutation]) -> Vec<HostMutation> {
    ledger
        .iter()
        .filter(|mutation| mutation.phase() == UndoPhase::BeforeTheOutput)
        .cloned()
        .collect()
}

/// A dispatch-based undo: it either happened, or it is a failure to report next
/// to the others.
fn dispatched(
    result: Result<(), Error>,
    note: String,
    what: &'static str,
    expected: String,
) -> Undo {
    match result {
        Ok(()) => Undo::Done {
            notes: vec![note],
            failure: None,
        },
        Err(error) => Undo::Done {
            notes: Vec::new(),
            failure: Some(RestoreFailure {
                what,
                expected,
                actual: error.to_string(),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::error::Error as StdError;

    use super::{HostMutation, RELOAD, Undo, UndoEffects, UndoPhase, before_the_output};
    use crate::error::Error;
    use crate::session::OutputRemoval;

    /// One sample of every variant. The match in `kind` is what keeps this list
    /// honest: `undo` being exhaustive says every variant has an arm, and this
    /// says every variant is actually driven through one.
    fn every_mutation() -> Vec<HostMutation> {
        vec![
            HostMutation::OutputCreated {
                output: "hyprpilot-alpha".to_owned(),
            },
            HostMutation::MonitorRuleSet {
                rule: "hyprpilot-alpha,1600x1000@60,auto,1".to_owned(),
            },
            HostMutation::WorkspaceRuleSet {
                rule: "agent-alpha, monitor:hyprpilot-alpha".to_owned(),
            },
            HostMutation::WorkspaceRenamed {
                id: 3,
                from: "3".to_owned(),
                to: "agent-alpha".to_owned(),
            },
            HostMutation::WorkspaceMovedToMonitor {
                workspace: "4".to_owned(),
                from: "hyprpilot".to_owned(),
            },
        ]
    }

    /// Adding a variant breaks this match, so `every_mutation` above cannot
    /// silently stop covering one.
    fn kind(mutation: &HostMutation) -> &'static str {
        match mutation {
            HostMutation::OutputCreated { .. } => "output_created",
            HostMutation::MonitorRuleSet { .. } => "monitor_rule_set",
            HostMutation::WorkspaceRuleSet { .. } => "workspace_rule_set",
            HostMutation::WorkspaceRenamed { .. } => "workspace_renamed",
            HostMutation::WorkspaceMovedToMonitor { .. } => "workspace_moved_to_monitor",
        }
    }

    /// Records what an undo asked the compositor to do, and answers `ok`.
    #[derive(Default)]
    struct Recorder {
        dispatched: RefCell<Vec<Vec<String>>>,
        removed: RefCell<Vec<String>>,
    }

    impl Recorder {
        fn acted(&self) -> bool {
            !self.dispatched.borrow().is_empty() || !self.removed.borrow().is_empty()
        }

        fn only_dispatch(&self) -> Vec<Vec<String>> {
            assert!(
                self.removed.borrow().is_empty(),
                "this undo has no output to remove"
            );
            self.dispatched.borrow().clone()
        }
    }

    /// The closures capture the recorder, so they cannot be handed back from a
    /// method — they have to outlive the `UndoEffects` that borrow them. Built
    /// here and lent to the body instead.
    fn with_recorder<T>(body: impl FnOnce(&UndoEffects<'_>, &Recorder) -> T) -> T {
        let recorder = Recorder::default();
        let remove_output = |output: &str, _cursor: Option<(i32, i32)>| {
            recorder.removed.borrow_mut().push(output.to_owned());
            Ok(OutputRemoval {
                notes: vec![format!("removed output {output}")],
                failure: None,
            })
        };
        let dispatch = |args: &[&str]| {
            recorder
                .dispatched
                .borrow_mut()
                .push(args.iter().map(|arg| (*arg).to_owned()).collect());
            Ok(())
        };
        body(
            &UndoEffects {
                remove_output: &remove_output,
                dispatch: &dispatch,
            },
            &recorder,
        )
    }

    #[test]
    fn every_host_mutation_names_its_undo_or_its_remedy() -> Result<(), Box<dyn StdError>> {
        let mutations = every_mutation();
        let covered: Vec<&str> = mutations.iter().map(kind).collect();
        for expected in [
            "output_created",
            "monitor_rule_set",
            "workspace_rule_set",
            "workspace_renamed",
            "workspace_moved_to_monitor",
        ] {
            assert!(
                covered.contains(&expected),
                "{expected} is a host mutation no sample drives through `undo`"
            );
        }

        for mutation in &mutations {
            with_recorder(|effects, recorder| -> Result<(), Box<dyn StdError>> {
                match mutation.undo(effects, None)? {
                    Undo::Done { notes, failure } => {
                        assert!(
                            recorder.acted(),
                            "{} reports itself undone without asking the compositor for anything",
                            kind(mutation)
                        );
                        assert!(
                            !notes.is_empty(),
                            "{} is undone silently, so a teardown cannot report it",
                            kind(mutation)
                        );
                        assert!(failure.is_none(), "the recorder answered every request");
                    }
                    Undo::Impossible { what, remedy } => {
                        assert!(
                            !recorder.acted(),
                            "{} calls itself impossible after mutating the host",
                            kind(mutation)
                        );
                        assert!(
                            !what.is_empty(),
                            "{} leaks something it does not name",
                            kind(mutation)
                        );
                        assert_eq!(
                            remedy,
                            RELOAD,
                            "{} names a remedy the user cannot run",
                            kind(mutation)
                        );
                    }
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    #[test]
    fn a_renamed_workspace_is_given_its_name_back() -> Result<(), Box<dyn StdError>> {
        with_recorder(|effects, recorder| {
            HostMutation::WorkspaceRenamed {
                id: 3,
                from: "3".to_owned(),
                to: "agent-alpha".to_owned(),
            }
            .undo(effects, None)?;
            assert_eq!(
                recorder.only_dispatch(),
                [vec![
                    "renameworkspace".to_owned(),
                    "3".to_owned(),
                    "3".to_owned()
                ]],
                "the undo names the workspace by the id it renamed, and gives back `from`"
            );
            Ok(())
        })
    }

    #[test]
    fn an_evacuated_workspace_goes_back_to_its_monitor() -> Result<(), Box<dyn StdError>> {
        with_recorder(|effects, recorder| {
            HostMutation::WorkspaceMovedToMonitor {
                workspace: "4".to_owned(),
                from: "hyprpilot".to_owned(),
            }
            .undo(effects, None)?;
            assert_eq!(
                recorder.only_dispatch(),
                [vec![
                    "moveworkspacetomonitor".to_owned(),
                    "4".to_owned(),
                    "hyprpilot".to_owned()
                ]],
                "a numeric workspace passes through the selector unchanged"
            );
            Ok(())
        })
    }

    /// A workspace that will not come back must not stop the output removal:
    /// the user would be left with a compositing headless output instead.
    #[test]
    fn a_workspace_that_will_not_come_back_is_a_failure_not_an_abort()
    -> Result<(), Box<dyn StdError>> {
        let effects = UndoEffects {
            remove_output: &|_, _| unreachable(),
            dispatch: &|_| {
                Err(Error::Tool {
                    command: "hyprctl dispatch renameworkspace".to_owned(),
                    message: "no such workspace".to_owned(),
                })
            },
        };
        let renamed = HostMutation::WorkspaceRenamed {
            id: 3,
            from: "3".to_owned(),
            to: "agent-alpha".to_owned(),
        };
        let Undo::Done { failure, .. } = renamed.undo(&effects, None)? else {
            return Err("a rename that failed is not impossible to undo, it failed".into());
        };
        let failure = failure.ok_or("a refused rename has to be reported")?;
        assert_eq!(failure.what, "renamed workspace");
        assert!(failure.actual.contains("no such workspace"));
        Ok(())
    }

    fn unreachable<T>() -> Result<T, Error> {
        Err(Error::Tool {
            command: "test".to_owned(),
            message: "this undo must not remove an output".to_owned(),
        })
    }

    /// The waybar defect, as a unit. `hyprctl output create headless` makes
    /// Hyprland attach the lowest free workspace id to the new output, and this
    /// crate renames it. Giving the name back *after* `output remove` hands it
    /// to a workspace the compositor has already destroyed: the user's bar keeps
    /// the dead `agent-*` label on the confiscated slot until waybar restarts.
    #[test]
    fn a_renamed_workspace_is_given_its_name_back_before_its_output_goes() {
        // The ledger exactly as an isolated start writes it.
        let ledger = every_mutation();
        let before = before_the_output(&ledger);

        assert!(
            before.iter().any(
                |mutation| matches!(mutation, HostMutation::WorkspaceRenamed { from, .. } if from == "3")
            ),
            "the rename has to be undone while its output still exists"
        );
        assert!(
            !before
                .iter()
                .any(|mutation| matches!(mutation, HostMutation::OutputCreated { .. })),
            "the output removal is gated on the output being empty, so it is not \
             part of this phase"
        );
        assert_eq!(
            ledger.len() - before.len(),
            1,
            "exactly one mutation waits for the output to be empty"
        );
    }

    /// Exhaustive like `undo`: adding a mutation forces a decision about whether
    /// its undo still needs the output, instead of defaulting to the order that
    /// caused the defect above.
    #[test]
    fn every_mutation_says_whether_its_undo_still_needs_the_output() {
        for mutation in every_mutation() {
            let expected = match kind(&mutation) {
                "output_created" => UndoPhase::TheOutput,
                _ => UndoPhase::BeforeTheOutput,
            };
            assert_eq!(
                mutation.phase(),
                expected,
                "{} is undone in the wrong phase",
                kind(&mutation)
            );
        }
    }

    /// `undo` and `leak` are two exhaustive matches over the same enum, and two
    /// matches can drift where one cannot: what `doctor` lists as unretractable
    /// has to be exactly what `undo` refuses to take back.
    #[test]
    fn what_doctor_lists_is_what_the_undo_cannot_take_back() -> Result<(), Box<dyn StdError>> {
        for mutation in every_mutation() {
            let listed = mutation.leak().is_some();
            let impossible = with_recorder(|effects, _| {
                Ok::<_, Error>(matches!(
                    mutation.undo(effects, None)?,
                    Undo::Impossible { .. }
                ))
            })?;
            assert_eq!(
                listed,
                impossible,
                "{} is classified one way by `leak` and the other by `undo`",
                kind(&mutation)
            );
        }
        Ok(())
    }

    #[test]
    fn a_host_mutation_round_trips_through_its_serde_tag() -> Result<(), Box<dyn StdError>> {
        for mutation in every_mutation() {
            let raw = serde_json::to_string(&mutation)?;
            let tag: serde_json::Value = serde_json::from_str(&raw)?;
            assert_eq!(
                tag.get("mutation").and_then(serde_json::Value::as_str),
                Some(kind(&mutation)),
                "the tag on disk is what a later build matches on"
            );
            let parsed: HostMutation = serde_json::from_str(&raw)?;
            assert_eq!(parsed, mutation);
        }
        Ok(())
    }
}
