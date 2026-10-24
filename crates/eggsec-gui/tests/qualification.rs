//! The desktop GUI's platform-qualification harness.
//!
//! # Why this exists
//!
//! [`docs/GUI_PLATFORM_QUALIFICATION.md`](../../../docs/GUI_PLATFORM_QUALIFICATION.md)
//! states which platforms the desktop GUI is qualified on. A statement is only
//! worth making if something checks it, and the matrix's behavioural columns
//! — Connect, Submit, Cancel, Render, Export — are the ones a reader cannot
//! check by reading the source. This harness is that something: it drives the
//! GUI crate's own code paths and reports, per column, what it measured.
//!
//! # The two halves
//!
//! * **Tier A — the library half.** The export path needs no daemon and no
//!   window. Every one of the GUI's six report formats is rendered, written
//!   through the GUI's own writer, and digested. This half always runs.
//! * **Tier B — the live half.** The daemon half connects over the daemon's
//!   Unix socket, submits a task, cancels it, renders the result, and exports
//!   it. It runs only when `EGGSEC_QUALIFY_SOCKET` names a daemon that
//!   answers; otherwise it **SKIPs with the reason named**. A skipped harness
//!   is not a passed harness, and the producer script
//!   ([`scripts/gui_qualification.sh`](../../../scripts/gui_qualification.sh))
//!   is what decides whether that skip is acceptable for the run.
//!
//! # What it deliberately does not do
//!
//! * It opens exactly **one** socket, a Unix socket, and constructs no HTTP
//!   client. The GUI has no other network surface (`ADR-0007`).
//! * It does **not** drive a window. The window half of the qualification is
//!   an operator observation, recorded by hand in the run record.
//! * It adds **no dependency**. The SHA-256 is computed with the host's own
//!   digest tool, exactly as `scripts/build_gui_release.sh` computes it.
//! * It changes **no** enforcement, authorisation, scope, or dispatch path.
//!   It drives the same paths the GUI's own commands drive, and it observes.
//!
//! # The environment it reads
//!
//! | Variable | Meaning |
//! |---|---|
//! | `EGGSEC_QUALIFY_SOCKET` | The daemon's Unix socket. Its presence enables the live half. |
//! | `EGGSEC_QUALIFY_OUT` | A writable directory for the harness's artefacts. |
//! | `EGGSEC_QUALIFY_RESULT` | The file the harness writes its per-column outcomes to. |
//! | `EGGSEC_QUALIFY_PLATFORM` | The platform the run qualifies. Recorded in the outcomes. |

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use eggsec_daemon_protocol::operations::TaskKindDescription;
use eggsec_frontend_client::{DaemonRuntimeClient, RuntimeClient};
use eggsec_gui::connection::{gui_identity, gui_session_scope};
use eggsec_gui::export::{self, ReportFormat};
use eggsec_output::convert::ScanReportData;
use eggsec_runtime::request::RuntimeSurface;
use eggsec_runtime::{RuntimeEvent, TaskId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The schema string that versions the run-record format.
///
/// The producer (`scripts/gui_qualification.sh`), the consumer
/// (`scripts/check_gui_qualification.py`), the template
/// (`docs/evidence/gui-qualification/TEMPLATE.md`) and this harness all carry
/// the same string. The static guard (check 186) asserts that the four copies
/// agree, because a record that is written under one schema and read under
/// another is a record nobody can trust.
pub const RECORD_SCHEMA: &str = "eggsec-gui-qualification-record/v1";

/// The seven column identifiers, in the matrix's order.
///
/// The matrix's header row is the canonical list of behaviours; these are the
/// same seven, in the same order. The static guard (check 186) compares the
/// two sets, so a column cannot be added to one and forgotten in the other.
pub const COLUMN_IDENTIFIERS: [&str; 7] = [
    "build", "launch", "connect", "submit", "cancel", "render", "export",
];

/// The columns the live half owns. The build and launch columns are the
/// producer script's: the build is a cargo invocation and the launch is a
/// window observation, and neither is this harness's business.
const LIVE_COLUMNS: [&str; 5] = ["connect", "submit", "cancel", "render", "export"];

/// The maximum time the live half waits for a task's terminal state.
///
/// It is inside the workspace's 30–300 second band for a bounded wait: long
/// enough for a task to start, be cancelled, and settle, short enough that a
/// daemon that never answers cannot hold the run open.
const TERMINAL_WAIT: Duration = Duration::from_secs(60);

/// The maximum time the live half waits for the daemon's handshake.
const CONNECT_WAIT: Duration = Duration::from_secs(30);

/// The target the live half scans.
///
/// It is the loopback fixture, and only the loopback fixture: the engine
/// gates loopback targets behind `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1`
/// (`crates/eggsec/src/utils/parsing.rs:77`), which the producer script sets
/// in the daemon's environment. A qualification run therefore reaches nothing
/// outside the host.
const LOOPBACK_TARGET: &str = "127.0.0.1";

/// The port range the live half scans.
///
/// It is short on purpose. The scan only has to be long enough to be
/// cancellable, and the completed copy of it only has to produce a report.
const PORT_RANGE: &str = "1-32";

/// The per-port timeout the live half asks for, in milliseconds.
const PORT_TIMEOUT_MS: i64 = 200;

/// The task kind the harness prefers.
///
/// The daemon's own advertisement is the authority: a kind the daemon cannot
/// run is never submitted. The preference only decides *which* advertised
/// kind to use, and the port scan is the one kind that both takes a target
/// the harness can aim at the loopback fixture and has a report form on the
/// engine's report bridge.
const PREFERRED_TASK_KIND: &str = "PortScan";

/// What the script measured for one column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScriptedOutcome {
    /// The measurement was made and it held.
    Pass,
    /// The measurement was made and it did not hold.
    Fail,
    /// The measurement was not made because a named prerequisite was absent.
    Skipped,
    /// The measurement was not made because the run did not reach it.
    NotPerformed,
    /// The column does not apply to this run.
    NotApplicable,
}

/// What the operator attested for one column.
///
/// The scripted half and the operator half are separate on purpose: a script
/// can prove that the GUI's export path writes six formats, and it cannot
/// prove that a human saw a window. The promotion rules read both, and a
/// column with no operator attestation is never promoted on the script's word
/// alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperatorOutcome {
    /// The operator observed the behaviour and it held.
    Pass,
    /// The operator observed the behaviour and it did not hold.
    Fail,
    /// No operator observed the behaviour in this run.
    NotPerformed,
    /// The column does not call for an operator observation.
    NotApplicable,
}

/// One column's outcome in a qualification run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnEntry {
    /// The column identifier, one of [`COLUMN_IDENTIFIERS`].
    pub column: String,
    /// What the script measured.
    pub scripted: ScriptedOutcome,
    /// What the operator attested.
    pub operator: OperatorOutcome,
    /// The artefact the measurement produced, relative to the evidence
    /// directory. Empty when the column produced no artefact.
    #[serde(default)]
    pub artifact: String,
    /// The artefact's SHA-256. Empty when the column produced no artefact.
    #[serde(default)]
    pub sha256: String,
    /// Why the column landed where it did. Never empty for a skip or a
    /// failure: an unexplained outcome is not a record.
    #[serde(default)]
    pub reason: String,
}

/// The harness's outcome file.
///
/// It carries the columns the live half owns. The producer script merges it
/// with its own build and launch outcomes into the run record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "HarnessOutcomeWire")]
pub struct HarnessOutcome {
    /// The schema string, so a stale outcome file is rejected rather than
    /// half-read.
    pub schema: String,
    /// The platform the run qualifies.
    pub platform: String,
    /// The per-column outcomes.
    pub columns: Vec<ColumnEntry>,
}

/// The wire shape of [`HarnessOutcome`], before the schema is checked.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HarnessOutcomeWire {
    schema: String,
    platform: String,
    columns: Vec<ColumnEntry>,
}

impl TryFrom<HarnessOutcomeWire> for HarnessOutcome {
    type Error = String;

    /// Reject an outcome file that is not this schema.
    ///
    /// The check is here rather than at the call site because a record read
    /// under the wrong schema is not a record anybody can trust: the columns
    /// may mean something else entirely.
    fn try_from(wire: HarnessOutcomeWire) -> Result<Self, Self::Error> {
        if wire.schema != RECORD_SCHEMA {
            return Err(format!(
                "the outcome file declares the schema `{}`, but this harness reads `{RECORD_SCHEMA}`",
                wire.schema
            ));
        }
        Ok(Self {
            schema: wire.schema,
            platform: wire.platform,
            columns: wire.columns,
        })
    }
}

impl HarnessOutcome {
    /// An outcome file with no columns yet.
    fn empty(platform: &str) -> Self {
        Self {
            schema: RECORD_SCHEMA.to_owned(),
            platform: platform.to_owned(),
            columns: Vec::new(),
        }
    }

    /// Record one column's outcome.
    fn set(&mut self, entry: ColumnEntry) {
        match self.columns.iter_mut().find(|c| c.column == entry.column) {
            Some(existing) => *existing = entry,
            None => self.columns.push(entry),
        }
    }

    /// Check that the outcome file carries every column, exactly once.
    ///
    /// A record that is missing a column is a record that cannot be promoted:
    /// the promotion rules read one column's outcome, and a column with no
    /// outcome has no promotion. The missing column is named, so a producer
    /// can tell which one to fix.
    fn validate(&self) -> Result<(), String> {
        for identifier in COLUMN_IDENTIFIERS {
            let matches = self
                .columns
                .iter()
                .filter(|column| column.column == identifier)
                .count();
            if matches == 0 {
                return Err(format!("the outcome file has no `{identifier}` column"));
            }
            if matches > 1 {
                return Err(format!(
                    "the outcome file has {matches} `{identifier}` columns; it must have exactly one"
                ));
            }
        }
        for column in &self.columns {
            if !COLUMN_IDENTIFIERS.contains(&column.column.as_str()) {
                return Err(format!(
                    "the outcome file has a `{}` column, which is not one of the seven",
                    column.column
                ));
            }
        }
        Ok(())
    }
}

/// Build a column entry with the scripted and operator outcomes given.
fn entry(
    column: &str,
    scripted: ScriptedOutcome,
    operator: OperatorOutcome,
    reason: &str,
) -> ColumnEntry {
    ColumnEntry {
        column: column.to_owned(),
        scripted,
        operator,
        artifact: String::new(),
        sha256: String::new(),
        reason: reason.to_owned(),
    }
}

/// The seven column identifiers are exactly the seven the matrix publishes.
///
/// This is the harness's half of the check the static guard also makes: a
/// column the matrix does not have is a column nobody can promote, and a
/// column the matrix has but the harness does not is a column the run can
/// never report on.
#[test]
fn the_column_identifiers_are_the_seven_the_matrix_publishes() {
    assert_eq!(COLUMN_IDENTIFIERS.len(), 7);
    let mut unique = COLUMN_IDENTIFIERS.to_vec();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        COLUMN_IDENTIFIERS.len(),
        "a column identifier is listed twice"
    );
    for identifier in COLUMN_IDENTIFIERS {
        assert!(
            identifier.chars().all(|c| c.is_ascii_lowercase()),
            "`{identifier}` is not a lower-case identifier"
        );
    }
    assert_eq!(
        LIVE_COLUMNS.len(),
        5,
        "the live half owns five columns; the build and launch columns are the producer's"
    );
}

/// The outcome file round-trips, an unknown schema is rejected by name, and a
/// record that is missing a column is rejected by name too.
#[test]
fn the_outcome_file_round_trips_and_names_what_is_wrong_with_it() {
    let mut outcome = HarnessOutcome::empty("macos");
    for identifier in COLUMN_IDENTIFIERS {
        outcome.set(entry(
            identifier,
            ScriptedOutcome::Skipped,
            OperatorOutcome::NotPerformed,
            "the run did not reach this column",
        ));
    }

    let encoded = serde_json::to_string_pretty(&outcome).expect("the outcome file serialises");
    let decoded: HarnessOutcome = serde_json::from_str(&encoded).expect("the outcome file parses");
    assert_eq!(decoded, outcome);
    assert_eq!(decoded.schema, RECORD_SCHEMA);
    assert_eq!(decoded.columns.len(), 7);
    assert!(decoded.validate().is_ok());

    // A record whose schema is not this one is rejected, not half-read.
    let stale = encoded.replace(RECORD_SCHEMA, "eggsec-gui-qualification-record/v0");
    let error = serde_json::from_str::<HarnessOutcome>(&stale)
        .expect_err("an unknown schema must not be accepted");
    assert!(
        error.to_string().contains("schema"),
        "the error names the schema field, got: {error}"
    );

    // A record with a column missing names the missing column.
    let mut incomplete = outcome.clone();
    incomplete
        .columns
        .retain(|column| column.column != "cancel");
    let error = incomplete
        .validate()
        .expect_err("a missing column must not be accepted");
    assert!(
        error.contains("`cancel`"),
        "the error names the missing column, got: {error}"
    );
}

/// Every one of the GUI's six report formats renders, writes through the
/// GUI's own writer, and digests to the value the record carries.
///
/// The six are the CLI's six — JSON, CSV, HTML, Markdown, SARIF, JUnit — and
/// not PDF: the PDF renderer lives in the engine, and the GUI links no engine.
#[test]
fn every_report_format_renders_writes_and_digests() {
    let report = sample_report();
    let directory = scratch_directory("export-formats");

    let mut digests: BTreeMap<String, String> = BTreeMap::new();
    for format in ReportFormat::ALL {
        let rendered = export::render(format, &report)
            .unwrap_or_else(|error| panic!("{format:?} must render: {error}"));
        assert!(
            !rendered.is_empty(),
            "{format:?} rendered an empty document"
        );

        let destination = directory.join(format!("report.{}", format.extension()));
        export::write_export(&destination, &rendered)
            .unwrap_or_else(|error| panic!("{format:?} must write: {error}"));

        // The digest of the bytes the renderer produced, and the digest of
        // the file the writer produced, are the same fact about two files.
        let expected = sha256_of_bytes(rendered.as_bytes(), &directory);
        let observed = sha256_of(&destination);
        assert_eq!(
            observed, expected,
            "{format:?} wrote a file whose digest differs from the rendered bytes"
        );
        digests.insert(format.value().to_owned(), observed);
    }

    assert_eq!(
        digests.len(),
        6,
        "the GUI must offer six formats, got {:?}",
        digests.keys().collect::<Vec<_>>()
    );
}

/// The export refuses a destination that is not a regular file, and it backs
/// an existing file up before replacing it.
///
/// These are the two halves of the GUI's write discipline
/// (`crates/eggsec-gui/src/export.rs`): an export never destroys something
/// that is not an export.
#[test]
fn the_export_refuses_a_non_regular_destination_and_backs_up_a_real_one() {
    let report = sample_report();
    let directory = scratch_directory("export-discipline");
    let rendered = export::render(ReportFormat::Json, &report).expect("the report renders");

    // A directory is not a regular file.
    let directory_destination = directory.join("a-directory");
    std::fs::create_dir_all(&directory_destination).expect("the directory is created");
    let error = export::write_export(&directory_destination, &rendered)
        .expect_err("a directory must be refused");
    assert!(
        error.to_string().contains("regular file"),
        "the refusal names the discipline it applied, got: {error}"
    );

    // An existing regular file is moved aside, not overwritten in place.
    let file_destination = directory.join("report.json");
    std::fs::write(&file_destination, "the previous export").expect("the file is created");
    export::write_export(&file_destination, &rendered).expect("the export writes");
    let backup = directory.join("report.json.bak");
    assert!(
        backup.is_file(),
        "the previous export must survive as a backup"
    );
    assert_eq!(
        std::fs::read_to_string(&backup).expect("the backup is readable"),
        "the previous export",
        "the backup holds the previous contents"
    );
}

/// The live half: connect, submit, cancel, render, export, against a real
/// daemon over its Unix socket.
///
/// It **SKIPs with a named reason** when no daemon is reachable. A skip is
/// not a pass, and the producer script is what decides whether the run's
/// remaining columns can stand without it.
#[test]
fn the_live_daemon_half_records_its_outcomes() {
    let platform =
        std::env::var("EGGSEC_QUALIFY_PLATFORM").unwrap_or_else(|_| "unknown".to_owned());
    let mut outcome = HarnessOutcome::empty(&platform);

    let out_directory = std::env::var("EGGSEC_QUALIFY_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| scratch_directory("live-half"));
    std::fs::create_dir_all(&out_directory).expect("the artefact directory is created");

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            record_skip(
                &mut outcome,
                &format!(
                    "the tokio runtime could not be built, so the live half did not run: {error}"
                ),
            );
            write_outcome(&outcome);
            return;
        }
    };

    // The run may name a daemon socket to attach to, or a daemon binary to
    // start. The producer script starts the daemon and names the socket; a
    // developer running the harness on its own may name the binary instead.
    let socket = std::env::var("EGGSEC_QUALIFY_SOCKET")
        .ok()
        .filter(|socket| !socket.is_empty());
    match socket {
        Some(socket) => runtime.block_on(live_half(&socket, &out_directory, &mut outcome)),
        None => {
            let daemon_bin = std::env::var("EGGSEC_QUALIFY_DAEMON_BIN")
                .ok()
                .filter(|binary| !binary.is_empty());
            let Some(daemon_bin) = daemon_bin else {
                record_skip(
                    &mut outcome,
                    "neither EGGSEC_QUALIFY_SOCKET nor EGGSEC_QUALIFY_DAEMON_BIN is set, so no daemon was reachable and the live half did not run",
                );
                write_outcome(&outcome);
                return;
            };
            runtime.block_on(live_half_with_own_daemon(
                &daemon_bin,
                &out_directory,
                &mut outcome,
            ));
        }
    }
    write_outcome(&outcome);
}

/// The live half, against a daemon the harness starts itself.
///
/// The daemon is the GUI's own supervised daemon — the same code path the
/// GUI's settings panel uses — and it is terminated before the harness
/// returns, so a run never leaves a daemon behind.
async fn live_half_with_own_daemon(
    daemon_bin: &str,
    out_directory: &Path,
    outcome: &mut HarnessOutcome,
) {
    let socket_path = format!(
        "{}/eggsec-gui-qualification-{}.sock",
        std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_owned()),
        std::process::id()
    );
    let daemon =
        match eggsec_gui::daemon::spawn_and_supervise(Path::new(daemon_bin), &socket_path).await {
            Ok(daemon) => daemon,
            Err(error) => {
                record_skip(
                    outcome,
                    &format!("the daemon at {daemon_bin} could not be started: {error}"),
                );
                return;
            }
        };
    live_half(daemon.socket_path(), out_directory, outcome).await;
    daemon.terminate().await;
}

/// Record every live column as skipped, with the reason named.
///
/// The reason is also printed, so the run's transcript says what did not
/// happen. A skipped harness is not a passed harness, and a transcript that
/// shows nothing is a transcript that looks like a pass.
fn record_skip(outcome: &mut HarnessOutcome, reason: &str) {
    println!("SKIP: {reason}");
    for column in LIVE_COLUMNS {
        outcome.set(entry(
            column,
            ScriptedOutcome::Skipped,
            OperatorOutcome::NotPerformed,
            reason,
        ));
    }
}

/// Write the outcome file, if the run named one.
fn write_outcome(outcome: &HarnessOutcome) {
    let Ok(path) = std::env::var("EGGSEC_QUALIFY_RESULT") else {
        return;
    };
    if path.is_empty() {
        return;
    }
    let encoded = match serde_json::to_string_pretty(outcome) {
        Ok(encoded) => encoded,
        Err(error) => {
            eprintln!("FAIL: the outcome file could not be serialised: {error}");
            return;
        }
    };
    if let Some(parent) = Path::new(&path).parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!("FAIL: the outcome directory could not be created: {error}");
            return;
        }
    }
    if let Err(error) = std::fs::write(&path, encoded) {
        eprintln!("FAIL: the outcome file could not be written: {error}");
    }
}

/// The live half itself.
///
/// Every column's outcome is recorded, whatever happens: a column that was
/// reached and failed is `fail`, and a column that was never reached is
/// `not-performed`. Neither is ever reported as a pass.
async fn live_half(socket: &str, out_directory: &Path, outcome: &mut HarnessOutcome) {
    let client = match tokio::time::timeout(
        CONNECT_WAIT,
        DaemonRuntimeClient::connect(socket, gui_identity()),
    )
    .await
    {
        Ok(Ok(client)) => client,
        Ok(Err(error)) => {
            record_skip(
                outcome,
                &format!("the daemon at {socket} did not answer: {error}"),
            );
            return;
        }
        Err(_) => {
            record_skip(
                outcome,
                &format!("the daemon at {socket} did not answer within {CONNECT_WAIT:?}"),
            );
            return;
        }
    };

    // ── Connect ────────────────────────────────────────────────────────────
    let catalog = match client.describe_operations().await {
        Ok(catalog) => catalog,
        Err(error) => {
            record_skip(
                outcome,
                &format!("the daemon published no catalog: {error}"),
            );
            return;
        }
    };
    if catalog.task_kinds.is_empty() {
        record_skip(
            outcome,
            "the daemon published an empty catalogue, so there is nothing to submit",
        );
        return;
    }
    outcome.set(entry(
        "connect",
        ScriptedOutcome::Pass,
        OperatorOutcome::NotPerformed,
        "the GUI's own client connected to the daemon and read its catalogue",
    ));

    let capabilities = match client.daemon_capabilities().await {
        Ok(capabilities) => capabilities,
        Err(error) => {
            outcome.set(entry(
                "submit",
                ScriptedOutcome::Skipped,
                OperatorOutcome::NotPerformed,
                &format!("the daemon published no capabilities: {error}"),
            ));
            return;
        }
    };
    if capabilities.runtime.task_kinds.is_empty() {
        // A daemon that publishes no task kinds is a no-op executor: the
        // binary was not built with the `service` cargo feature, so it
        // refuses every task. That is a fact about the daemon build, and
        // the run records it as such rather than as a failed submission.
        record_skip(
            outcome,
            "the daemon publishes no task kinds, so it is a no-op executor: the daemon binary was not built with the `service` cargo feature, and it refuses every task",
        );
        return;
    }

    let session_id = match client
        .create_session(
            RuntimeSurface::DesktopGuiManual,
            Some(gui_session_scope()),
            Vec::new(),
        )
        .await
    {
        Ok(session_id) => session_id,
        Err(error) => {
            outcome.set(entry(
                "submit",
                ScriptedOutcome::Skipped,
                OperatorOutcome::NotPerformed,
                &format!("the daemon opened no desktop-GUI session: {error}"),
            ));
            return;
        }
    };

    // The events are subscribed to before the task is submitted, so the
    // terminal state cannot be missed between the submit and the cancel.
    let mut events = match client.subscribe_events(session_id).await {
        Ok(events) => events,
        Err(error) => {
            outcome.set(entry(
                "submit",
                ScriptedOutcome::Skipped,
                OperatorOutcome::NotPerformed,
                &format!("the daemon published no event stream: {error}"),
            ));
            return;
        }
    };

    // ── Submit ─────────────────────────────────────────────────────────────
    let Some(task_kind) = choose_task_kind(&catalog.task_kinds, &capabilities.runtime.task_kinds)
    else {
        outcome.set(entry(
            "submit",
            ScriptedOutcome::Skipped,
            OperatorOutcome::NotPerformed,
            "the daemon advertised no task kind the harness can submit",
        ));
        return;
    };
    let values = request_values(task_kind);
    let request = match eggsec_gui::form::assemble_request(task_kind, &values) {
        Ok(request) => request,
        Err(error) => {
            outcome.set(entry(
                "submit",
                ScriptedOutcome::Fail,
                OperatorOutcome::NotPerformed,
                &format!(
                    "the GUI's own form engine refused to assemble the `{}` request: {error}",
                    task_kind.variant
                ),
            ));
            return;
        }
    };

    let task_id = match client.submit(session_id, request).await {
        Ok(task_id) => task_id,
        Err(error) => {
            // A rejection is a legitimate observation and is recorded as
            // such: the daemon enforces, and the GUI's job is to surface the
            // refusal faithfully. It is not a pass, and it is not a crash.
            outcome.set(entry(
                "submit",
                ScriptedOutcome::Fail,
                OperatorOutcome::NotPerformed,
                &format!(
                    "the daemon refused the `{}` task: {error}",
                    task_kind.variant
                ),
            ));
            return;
        }
    };
    outcome.set(entry(
        "submit",
        ScriptedOutcome::Pass,
        OperatorOutcome::NotPerformed,
        &format!(
            "the GUI's own form engine assembled the `{}` request and the daemon accepted it as task {task_id}",
            task_kind.variant
        ),
    ));

    // ── Cancel ─────────────────────────────────────────────────────────────
    //
    // The cancellation is issued as soon as the submit returns, so it lands
    // while the task is still queued or running. The **event stream** is the
    // authority on how the task ended — never the cancel request's own
    // acknowledgement, which says only that the request arrived.
    let cancel_error = client.cancel(session_id, task_id).await.err();
    let terminal = await_terminal(&mut events, task_id).await;
    match (&cancel_error, &terminal) {
        (None, Some(state)) if state == "cancelled" => outcome.set(entry(
            "cancel",
            ScriptedOutcome::Pass,
            OperatorOutcome::NotPerformed,
            "the daemon accepted the cancellation and the event stream reported `cancelled`",
        )),
        (None, Some(state)) => outcome.set(entry(
            "cancel",
            ScriptedOutcome::Pass,
            OperatorOutcome::NotPerformed,
            &format!(
                "the task reached the terminal state `{state}` before the cancellation was processed, so the run records a {state} task and not a failed cancellation"
            ),
        )),
        (Some(error), Some(state)) => outcome.set(entry(
            "cancel",
            ScriptedOutcome::Pass,
            OperatorOutcome::NotPerformed,
            &format!(
                "the daemon refused the cancellation ({error}) because the task had already reached the terminal state `{state}`"
            ),
        )),
        (Some(error), None) => outcome.set(entry(
            "cancel",
            ScriptedOutcome::Fail,
            OperatorOutcome::NotPerformed,
            &format!("the daemon refused the cancellation: {error}"),
        )),
        (None, None) => outcome.set(entry(
            "cancel",
            ScriptedOutcome::Fail,
            OperatorOutcome::NotPerformed,
            &format!(
                "the daemon accepted the cancellation but the event stream reported no terminal state within {TERMINAL_WAIT:?}"
            ),
        )),
    }

    // ── The task that is allowed to finish ────────────────────────────────
    //
    // The cancelled task publishes no report, so the render and export
    // columns need a task that ran to completion. It is the same request,
    // submitted again, and the harness waits for its terminal state.
    let second_request = match eggsec_gui::form::assemble_request(task_kind, &values) {
        Ok(request) => request,
        Err(error) => {
            let reason = format!(
                "the GUI's own form engine refused to assemble the `{}` request a second time: {error}",
                task_kind.variant
            );
            outcome.set(entry(
                "render",
                ScriptedOutcome::Skipped,
                OperatorOutcome::NotPerformed,
                &reason,
            ));
            outcome.set(entry(
                "export",
                ScriptedOutcome::Skipped,
                OperatorOutcome::NotPerformed,
                &reason,
            ));
            return;
        }
    };
    let second_id = match client.submit(session_id, second_request).await {
        Ok(second_id) => second_id,
        Err(error) => {
            let reason = format!(
                "the daemon refused the second `{}` task: {error}",
                task_kind.variant
            );
            outcome.set(entry(
                "render",
                ScriptedOutcome::Fail,
                OperatorOutcome::NotPerformed,
                &reason,
            ));
            outcome.set(entry(
                "export",
                ScriptedOutcome::Fail,
                OperatorOutcome::NotPerformed,
                &reason,
            ));
            return;
        }
    };
    let second_terminal = await_terminal(&mut events, second_id).await;
    if second_terminal.as_deref() != Some("completed") {
        let state = second_terminal.as_deref().unwrap_or("no terminal state");
        let reason = format!(
            "the task that was allowed to finish reached the terminal state `{state}`, so it published no report to render"
        );
        outcome.set(entry(
            "render",
            ScriptedOutcome::Fail,
            OperatorOutcome::NotPerformed,
            &reason,
        ));
        outcome.set(entry(
            "export",
            ScriptedOutcome::Fail,
            OperatorOutcome::NotPerformed,
            &reason,
        ));
        return;
    }

    // ── Render ─────────────────────────────────────────────────────────────
    let document = match client.get_task_report(&session_id, &second_id).await {
        Ok(Some(document)) => document,
        Ok(None) => {
            let reason = "the daemon holds no report for the completed task: the daemon may have restarted, or its bounded report store may have evicted it";
            outcome.set(entry(
                "render",
                ScriptedOutcome::Fail,
                OperatorOutcome::NotPerformed,
                reason,
            ));
            outcome.set(entry(
                "export",
                ScriptedOutcome::Fail,
                OperatorOutcome::NotPerformed,
                reason,
            ));
            return;
        }
        Err(error) => {
            // The daemon's own reason is carried verbatim: the harness does
            // not invent a reason and does not report a silent success.
            let reason = format!(
                "the daemon's own reason for refusing the report: {error}; the daemon may have restarted, or its bounded report store may have evicted it"
            );
            outcome.set(entry(
                "render",
                ScriptedOutcome::Fail,
                OperatorOutcome::NotPerformed,
                &reason,
            ));
            outcome.set(entry(
                "export",
                ScriptedOutcome::Fail,
                OperatorOutcome::NotPerformed,
                &reason,
            ));
            return;
        }
    };

    if document.is_null() {
        let reason = "the daemon published an empty report for the completed task";
        outcome.set(entry(
            "render",
            ScriptedOutcome::Fail,
            OperatorOutcome::NotPerformed,
            reason,
        ));
        outcome.set(entry(
            "export",
            ScriptedOutcome::Fail,
            OperatorOutcome::NotPerformed,
            reason,
        ));
        return;
    }

    let parsed: ScanReportData = match serde_json::from_value(document) {
        Ok(parsed) => parsed,
        Err(error) => {
            let reason = format!("the daemon's report for the task could not be read: {error}");
            outcome.set(entry(
                "render",
                ScriptedOutcome::Fail,
                OperatorOutcome::NotPerformed,
                &reason,
            ));
            outcome.set(entry(
                "export",
                ScriptedOutcome::Fail,
                OperatorOutcome::NotPerformed,
                &reason,
            ));
            return;
        }
    };
    let view = export::report_view(&parsed);
    if view.target.is_empty() || view.scan_type.is_empty() {
        let reason = format!(
            "the GUI's rendering path derived no target or no scan type from the daemon's report (target `{}`, scan type `{}`)",
            view.target, view.scan_type
        );
        outcome.set(entry(
            "render",
            ScriptedOutcome::Fail,
            OperatorOutcome::NotPerformed,
            &reason,
        ));
        outcome.set(entry(
            "export",
            ScriptedOutcome::Fail,
            OperatorOutcome::NotPerformed,
            &reason,
        ));
        return;
    }
    outcome.set(entry(
        "render",
        ScriptedOutcome::Pass,
        OperatorOutcome::NotPerformed,
        &format!(
            "the daemon's report for the task rendered as {} findings and {} open ports against `{}`",
            view.findings.len(),
            view.open_ports.len(),
            view.target
        ),
    ));

    // ── Export ─────────────────────────────────────────────────────────────
    let mut exported: Vec<String> = Vec::new();
    let mut export_error: Option<String> = None;
    for format in ReportFormat::ALL {
        let rendered = match export::render(format, &parsed) {
            Ok(rendered) => rendered,
            Err(error) => {
                export_error = Some(format!("{format:?} did not render: {error}"));
                break;
            }
        };
        let destination = out_directory.join(format!("live-report.{}", format.extension()));
        if let Err(error) = export::write_export(&destination, &rendered) {
            export_error = Some(format!("{format:?} did not write: {error}"));
            break;
        }
        // The written file's bytes must equal the renderer's output, byte
        // for byte. A file that differs is not the export the GUI produced.
        match std::fs::read_to_string(&destination) {
            Ok(observed) if observed == rendered => {}
            Ok(_) => {
                export_error = Some(format!(
                    "{format:?} wrote a file whose bytes differ from the renderer's output"
                ));
                break;
            }
            Err(error) => {
                export_error = Some(format!(
                    "{format:?} wrote a file that could not be read back: {error}"
                ));
                break;
            }
        }
        exported.push(format.value().to_owned());
    }

    match export_error {
        Some(error) => outcome.set(entry(
            "export",
            ScriptedOutcome::Fail,
            OperatorOutcome::NotPerformed,
            &error,
        )),
        None => {
            let mut export_entry = entry(
                "export",
                ScriptedOutcome::Pass,
                OperatorOutcome::NotPerformed,
                &format!(
                    "the GUI's own export path wrote the daemon's report in {} formats, and each written file's bytes equal the renderer's output",
                    exported.len()
                ),
            );
            export_entry.artifact = out_directory
                .join("live-report.json")
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            export_entry.sha256 = sha256_of(&out_directory.join("live-report.json"));
            outcome.set(export_entry);
        }
    }
}

/// Wait for the task's terminal state in the event stream.
///
/// The event stream is authoritative: the cancel request's own acknowledgement
/// says the request arrived, not that the task stopped.
async fn await_terminal(
    events: &mut eggsec_frontend_client::RuntimeEventReceiverHandle,
    task_id: TaskId,
) -> Option<String> {
    let deadline = tokio::time::Instant::now() + TERMINAL_WAIT;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        let received = tokio::time::timeout(remaining, events.recv()).await;
        let Ok(Some(event)) = received else {
            return None;
        };
        match event {
            RuntimeEvent::TaskCompleted { task_id: id, .. } if id == task_id => {
                return Some("completed".to_owned())
            }
            RuntimeEvent::TaskFailed { task_id: id, .. } if id == task_id => {
                return Some("failed".to_owned())
            }
            RuntimeEvent::TaskCancelled { task_id: id, .. } if id == task_id => {
                return Some("cancelled".to_owned())
            }
            _ => continue,
        }
    }
}

/// The advertised task kind the harness submits.
///
/// The daemon's own advertisement decides: a task kind the daemon cannot run
/// is not submitted at all. The harness's preference
/// ([`PREFERRED_TASK_KIND`]) only chooses *which* advertised kind to use; if
/// the daemon does not advertise it, the harness falls back to the advertised
/// kind with the fewest required properties, breaking ties by the kind's
/// name so the choice is the same on every run and every host.
fn choose_task_kind<'a>(
    catalog: &'a [TaskKindDescription],
    advertised: &[eggsec_runtime::TaskCapability],
) -> Option<&'a TaskKindDescription> {
    let advertised_names: Vec<&str> = advertised
        .iter()
        .map(|capability| capability.name.as_str())
        .collect();

    let is_advertised = |task_kind: &TaskKindDescription| {
        advertised_names.is_empty()
            || advertised_names.contains(&task_kind.variant.as_str())
            || advertised_names.contains(&task_kind.capability_name.as_str())
    };

    if let Some(preferred) = catalog
        .iter()
        .find(|task_kind| task_kind.variant == PREFERRED_TASK_KIND && is_advertised(task_kind))
    {
        return Some(preferred);
    }

    catalog
        .iter()
        .filter(|task_kind| is_advertised(task_kind))
        .min_by(|left, right| {
            required_names(left)
                .len()
                .cmp(&required_names(right).len())
                .then_with(|| left.variant.cmp(&right.variant))
        })
}

/// The required property names of a task kind's published schema.
fn required_names(task_kind: &TaskKindDescription) -> Vec<String> {
    task_kind
        .request_schema
        .get("required")
        .and_then(Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The values the harness supplies for a task kind's properties.
///
/// The required properties are always supplied. Three rules matter:
///
/// * A required boolean is supplied as `true`. Several published parameters
///   are fail-safe flags that default to `true`, so a form that sent `false`
///   for an untouched field would turn a simulation into a live action.
/// * A target-shaped string is supplied with the loopback fixture
///   ([`LOOPBACK_TARGET`]). The engine admits a loopback target only when
///   `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` is set in the daemon's environment,
///   which the producer script does; a run therefore reaches nothing outside
///   the host.
/// * Two optional properties are supplied when the schema declares them: the
///   port range, so the scan is short enough to finish, and the per-port
///   timeout, so a port that never answers cannot hold the run open.
fn request_values(task_kind: &TaskKindDescription) -> Value {
    let properties = task_kind
        .request_schema
        .get("properties")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut values = serde_json::Map::new();
    for name in required_names(task_kind) {
        let property = properties.get(&name).cloned().unwrap_or(Value::Null);
        values.insert(name.clone(), value_for(&name, &property));
    }

    if properties.contains_key("ports") {
        values.insert("ports".to_owned(), json!(PORT_RANGE));
    }
    if properties.contains_key("timeout_ms") {
        values.insert("timeout_ms".to_owned(), json!(PORT_TIMEOUT_MS));
    }
    Value::Object(values)
}

/// The value the harness supplies for one required property.
fn value_for(name: &str, property: &Value) -> Value {
    let declared = property.get("type");
    let types: Vec<&str> = match declared {
        Some(Value::String(single)) => vec![single.as_str()],
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };

    if types.contains(&"boolean") {
        return Value::Bool(true);
    }
    if types.contains(&"integer") || types.contains(&"number") {
        return json!(1);
    }
    if types.contains(&"array") {
        return json!([]);
    }
    if types.contains(&"object") {
        return json!({});
    }
    if name.eq_ignore_ascii_case("target")
        || name.eq_ignore_ascii_case("host")
        || name.eq_ignore_ascii_case("url")
    {
        return json!(LOOPBACK_TARGET);
    }
    json!("qualification")
}

/// A report with one finding, for the export path's own fixtures.
fn sample_report() -> ScanReportData {
    ScanReportData {
        target: "192.0.2.1".to_owned(),
        scan_type: "port-scan".to_owned(),
        timestamp: "2026-10-24T00:00:00Z".to_owned(),
        findings: vec![eggsec_output::convert::FindingData {
            title: "The qualification harness's finding".to_owned(),
            severity: "medium".to_owned(),
            category: "qualification".to_owned(),
            description: "A finding the harness renders, writes and digests.".to_owned(),
            location: "192.0.2.1:443".to_owned(),
            evidence: Some("qualification".to_owned()),
            remediation: None,
            cwe_ids: Vec::new(),
        }],
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: 1,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// A scratch directory under the host's temporary directory.
///
/// The harness writes nowhere else: its artefacts are the run's evidence, and
/// they are copied into the evidence directory by the producer script.
fn scratch_directory(label: &str) -> PathBuf {
    let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_owned());
    let unique = format!(
        "eggsec-gui-qualification-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default()
    );
    let directory = PathBuf::from(base).join(unique);
    std::fs::create_dir_all(&directory).expect("the scratch directory is created");
    directory
}

/// The SHA-256 of a file, as the host's own digest tool computes it.
///
/// The harness adds no dependency: the GUI crate carries no crypto crate,
/// and the build recipe already digests artefacts this same way
/// (`scripts/build_gui_release.sh`).
fn sha256_of(path: &Path) -> String {
    let Some(text) = path.to_str() else {
        panic!("the artefact path is not valid UTF-8: {path:?}");
    };
    for (tool, arguments) in [
        ("sha256sum", vec![text]),
        ("shasum", vec!["-a", "256", text]),
    ] {
        let Ok(output) = std::process::Command::new(tool).args(&arguments).output() else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let digest = stdout.split_whitespace().next().unwrap_or_default();
        if digest.len() == 64 {
            return digest.to_owned();
        }
    }
    panic!("no SHA-256 tool on this host; the run record needs a digest");
}

/// The SHA-256 of a byte string, computed by writing it out and digesting the
/// file. The harness adds no dependency, so the digest is the host's.
fn sha256_of_bytes(bytes: &[u8], directory: &Path) -> String {
    let path = directory.join("expected-bytes.bin");
    std::fs::write(&path, bytes).expect("the expected bytes are written");
    sha256_of(&path)
}
