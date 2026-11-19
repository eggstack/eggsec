"use strict";

/**
 * Eggsec desktop GUI — webview frontend.
 *
 * This file is a pure data renderer. It holds no per-operation knowledge:
 * every operation, task kind, form control, field name, status label and
 * result kind it displays arrives as data from the connected daemon through
 * the Tauri IPC commands of the GUI's IPC contract. A future operation
 * therefore appears here with no frontend change.
 *
 * Rules this file obeys:
 *   - no build step, no dependency, no network access of its own;
 *   - every daemon-supplied value is rendered as text (`textContent` /
 *     `createTextNode`), never as markup;
 *   - a blank control is sent as `null`, which is the wire meaning of
 *     "the operator did not provide this field" — never `""`, `0`, `false`
 *     or `[]`, so server-side defaults are never clobbered;
 *   - retained state is bounded (log lines per task, tasks tracked).
 */

(function () {
  const TASK_EVENT = "eggsec://task-event";

  /* Retention caps. The daemon caps its own rings; the window caps its copy
     so a long-lived or chatty task cannot grow the webview without bound. */
  const MAX_LOG_LINES_PER_TASK = 500;
  const MAX_TRACKED_TASKS = 200;

  const tauri = typeof window !== "undefined" ? window.__TAURI__ : undefined;
  const invoke =
    tauri && tauri.core && typeof tauri.core.invoke === "function"
      ? tauri.core.invoke.bind(tauri.core)
      : null;
  const listen =
    tauri && tauri.event && typeof tauri.event.listen === "function"
      ? tauri.event.listen.bind(tauri.event)
      : null;

  /* ----------------------------------------------------------------- dom */

  const dom = {
    connectForm: document.getElementById("connect-form"),
    socketInput: document.getElementById("socket-input"),
    connectButton: document.getElementById("connect-button"),
    catalogFacts: document.getElementById("catalog-facts"),
    bannerArea: document.getElementById("banner-area"),
    noOpBadge: document.getElementById("no-op-badge"),
    connectionBadge: document.getElementById("connection-badge"),
    taskKindList: document.getElementById("task-kind-list"),
    formBody: document.getElementById("form-body"),
    formSubheading: document.getElementById("form-subheading"),
    taskList: document.getElementById("task-list"),
    dashboardCounters: document.getElementById("dashboard-counters"),
    cancelActiveButton: document.getElementById("cancel-active-button"),
    daemonFacts: document.getElementById("daemon-facts"),
    coverageBody: document.getElementById("coverage-body"),
    exportForm: document.getElementById("export-form"),
    exportTask: document.getElementById("export-task"),
    exportFormat: document.getElementById("export-format"),
    exportPath: document.getElementById("export-path"),
    exportButton: document.getElementById("export-button"),
    exportHint: document.getElementById("export-hint"),
    reportButton: document.getElementById("report-button"),
    reportBody: document.getElementById("report-body"),
  };

  /* --------------------------------------------------------------- state */

  const state = {
    connected: false,
    connecting: false,
    catalog: null,
    operations: [],
    operationsById: new Map(),
    taskKinds: [],
    selectedVariant: null,
    selectedKind: null,
    form: null,
    formReaders: new Map(),
    fieldErrorNodes: new Map(),
    formSubmit: null,
    formErrorNode: null,
    tasks: new Map(),
    taskNodes: new Map(),
    selectedTaskId: null,
    droppedLogLines: 0,
    unknownTaskEvents: 0,
    submitting: false,
    coverage: [],
    daemon: null,
  };

  const banners = new Map();

  /* -------------------------------------------------------------- helpers */

  function el(tag, attrs, children) {
    const node = document.createElement(tag);
    if (attrs) {
      for (const key of Object.keys(attrs)) {
        const value = attrs[key];
        if (value === null || value === undefined || value === false) continue;
        if (key === "class") node.className = value;
        else if (key === "text") node.textContent = String(value);
        else if (key.startsWith("on") && typeof value === "function") {
          node.addEventListener(key.slice(2), value);
        } else if (value === true) node.setAttribute(key, "");
        else node.setAttribute(key, String(value));
      }
    }
    if (children) {
      for (const child of [].concat(children)) {
        if (child === null || child === undefined || child === false) continue;
        node.appendChild(
          typeof child === "string" || typeof child === "number"
            ? document.createTextNode(String(child))
            : child
        );
      }
    }
    return node;
  }

  function clear(node) {
    if (!node) return;
    while (node.firstChild) node.removeChild(node.firstChild);
  }

  function escapeRegExp(value) {
    return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  function errorMessage(error) {
    if (typeof error === "string") return error;
    if (error && typeof error.message === "string") return error.message;
    return String(error);
  }

  function safeStringify(value) {
    try {
      const rendered = JSON.stringify(value, null, 2);
      return rendered === undefined ? String(value) : rendered;
    } catch (error) {
      return "The result payload could not be rendered as JSON.";
    }
  }

  function domId(value) {
    return String(value === null || value === undefined ? "" : value).replace(
      /[^A-Za-z0-9_-]/g,
      "_"
    );
  }

  /**
   * Build the argument object for an IPC command.
   *
   * The GUI's IPC contract names multi-word arguments in snake_case, while
   * Tauri's command macro renames them to camelCase by default. Both
   * spellings are sent so the frontend works against either convention; the
   * side that receives the payload reads the key it knows and ignores the
   * other. Single-word arguments are sent once.
   */
  /**
   * Build a command's argument object.
   *
   * The names are the wire names, verbatim. Every Rust command declares
   * `rename_all = "snake_case"`, so the argument names are snake_case on
   * both sides of the IPC boundary. Do not "helpfully" also send a camelCase
   * alias: two spellings of one argument is an ambiguity the receiver has to
   * resolve, and it hides the fact that the contract is snake_case.
   */
  function commandArgs(pairs) {
    const args = {};
    for (const pair of pairs) {
      args[pair[0]] = pair[1];
    }
    return args;
  }

  /* ---------------------------------------------------------------- ipc */

  function requireIpc() {
    if (invoke) return true;
    setBanner("ipc", {
      level: "error",
      title: "IPC bridge unavailable",
      message:
        "The Tauri IPC bridge is not present in this window. The GUI must run inside the Eggsec desktop shell.",
    });
    return false;
  }

  /* ------------------------------------------------------------- banners */

  function setBanner(id, banner) {
    if (banner) banners.set(id, banner);
    else banners.delete(id);
    renderBanners();
  }

  function renderBanners() {
    clear(dom.bannerArea);
    for (const banner of banners.values()) {
      dom.bannerArea.appendChild(
        el("div", { class: "banner banner--" + banner.level }, [
          el("span", { class: "banner__title", text: banner.title }),
          el("span", { class: "banner__message", text: banner.message }),
        ])
      );
    }
  }

  /* ------------------------------------------------------------- catalog */

  function renderCatalog() {
    const catalog = state.catalog;
    dom.noOpBadge.hidden = !(catalog && catalog.no_op_executor);
    dom.connectionBadge.textContent = catalog ? "Connected" : "Not connected";
    dom.connectionBadge.className = "badge " + (catalog ? "badge--ok" : "badge--muted");

    clear(dom.catalogFacts);
    if (!catalog) return;

    const facts = [
      ["Socket", catalog.socket_path],
      ["Session", catalog.session_id],
      ["Protocol", "v" + catalog.protocol_version],
      ["Operations", catalog.operation_count],
      ["Task kinds", catalog.task_kind_count],
    ];
    for (const fact of facts) {
      dom.catalogFacts.appendChild(el("dt", { class: "catalog-facts__name", text: fact[0] }));
      dom.catalogFacts.appendChild(
        el("dd", { class: "catalog-facts__value", text: fact[1] })
      );
    }
  }

  /* -------------------------------------------------------------- daemon */

  /**
   * Render what the GUI knows about the daemon it is attached to.
   *
   * The facts arrive from the Rust side: the attachment state, the socket,
   * the published enabled-feature set and the advertised capability count.
   * An absent feature set is *unknown*, not empty, and it is shown that way.
   */
  function renderDaemonStatus() {
    clear(dom.daemonFacts);
    const daemon = state.daemon;
    if (!daemon) {
      dom.daemonFacts.appendChild(el("dt", { class: "catalog-facts__name", text: "Daemon" }));
      dom.daemonFacts.appendChild(
        el("dd", { class: "catalog-facts__value", text: "Not connected to a daemon yet." })
      );
      return;
    }

    const features = Array.isArray(daemon.enabled_features) ? daemon.enabled_features : null;
    const facts = [
      ["Attachment", labelize(daemon.attachment)],
      ["Socket", daemon.socket_path ? String(daemon.socket_path) : "none"],
      [
        "Engine features",
        features === null ? "unknown" : features.length ? features.join(", ") : "none enabled",
      ],
      ["Capabilities", String(daemon.advertised_capability_count)],
    ];
    for (const fact of facts) {
      dom.daemonFacts.appendChild(el("dt", { class: "catalog-facts__name", text: fact[0] }));
      dom.daemonFacts.appendChild(
        el("dd", { class: "catalog-facts__value", text: fact[1] })
      );
    }
    if (daemon.notice) {
      dom.daemonFacts.appendChild(el("dt", { class: "catalog-facts__name", text: "Notice" }));
      dom.daemonFacts.appendChild(
        el("dd", { class: "catalog-facts__value", text: String(daemon.notice) })
      );
    }
  }

  /** Capitalise a wire value for display. */
  function labelize(value) {
    const text = String(value === undefined || value === null ? "" : value);
    if (!text) return "";
    return text.charAt(0).toUpperCase() + text.slice(1);
  }

  /* ------------------------------------------------------------ coverage */

  /**
   * Render the coverage table: every published task kind, and why it is or
   * is not reachable from this GUI.
   *
   * The rows arrive from the Rust side, which derives them from what the
   * daemon published. This file renders them and holds no opinion of its
   * own about which operations exist or which of them work.
   */
  function renderCoverage() {
    clear(dom.coverageBody);
    const rows = Array.isArray(state.coverage) ? state.coverage : [];

    if (rows.length === 0) {
      dom.coverageBody.appendChild(
        el("p", {
          class: "empty",
          text: state.connected
            ? "The connected daemon published no task kinds."
            : "Not connected to a daemon yet.",
        })
      );
      return;
    }

    const head = el("tr", null, [
      el("th", { scope: "col", text: "Task kind" }),
      el("th", { scope: "col", text: "Operation" }),
      el("th", { scope: "col", text: "Capability" }),
      el("th", { scope: "col", text: "Verdict" }),
      el("th", { scope: "col", text: "Reason" }),
      el("th", { scope: "col", text: "Report" }),
    ]);

    const body = el("tbody", null, []);
    for (const row of rows) {
      const verdict = String((row && row.reachability) || "");
      const reportForm = String((row && row.report_form) || "");
      const reportReason = String((row && row.report_reason) || "");
      body.appendChild(
        el("tr", { class: "coverage-row" }, [
          el("td", { class: "coverage-cell coverage-cell--name", text: String(row.variant || "") }),
          el("td", { class: "coverage-cell", text: String(row.operation_id || "") }),
          el("td", { class: "coverage-cell", text: String(row.capability_name || "") }),
          el("td", { class: "coverage-cell coverage-cell--verdict" }, [
            el("span", { class: "coverage-verdict coverage-verdict--" + verdict, text: verdict }),
          ]),
          el("td", { class: "coverage-cell coverage-cell--reason", text: String(row.reason || "") }),
          el("td", { class: "coverage-cell coverage-cell--report" }, [
            el("span", {
              class: "coverage-verdict coverage-verdict--" + reportForm,
              text: reportForm,
            }),
            reportReason
              ? el("span", {
                  class: "coverage-cell--reason",
                  text: reportReason,
                })
              : null,
          ]),
        ])
      );
    }

    dom.coverageBody.appendChild(
      el("table", { class: "coverage-table" }, [el("thead", null, [head]), body])
    );
  }

  /* -------------------------------------------------------------- export */

  /** Fill the format list from the formats the Rust side offers. */
  function renderExportFormats() {
    if (!dom.exportFormat) return;
    const formats =
      state.daemon && Array.isArray(state.daemon.report_formats) ? state.daemon.report_formats : [];
    const selected = dom.exportFormat.value;
    clear(dom.exportFormat);
    for (const format of formats) {
      const option = el("option", {
        value: String(format.value),
        text: String(format.label) + " (." + String(format.extension) + ")",
      });
      dom.exportFormat.appendChild(option);
    }
    if (selected) dom.exportFormat.value = selected;
    renderExportState();
  }

  /**
   * Enable the export only when a report can exist for the chosen task.
   *
   * The daemon retains a report for a completed task, so the export is
   * offered for a task this window saw complete. Anything else says so
   * instead of offering an action that cannot succeed.
   */
  function renderExportState() {
    if (!dom.exportButton) return;
    const taskId = dom.exportTask ? String(dom.exportTask.value || "").trim() : "";
    const format = dom.exportFormat ? String(dom.exportFormat.value || "") : "";
    const path = dom.exportPath ? String(dom.exportPath.value || "").trim() : "";
    const task = taskId ? state.tasks.get(taskId) : null;
    const completed = task && String(task.status || "") === "completed";

    dom.exportButton.disabled = !(taskId && format && path && completed);
    if (dom.reportButton) dom.reportButton.disabled = !(taskId && format && completed);

    if (!dom.exportHint) return;
    if (!taskId) {
      dom.exportHint.textContent = "Choose a completed task to show or export its report.";
    } else if (!task) {
      dom.exportHint.textContent =
        "This window has not seen that task. The daemon keeps a bounded number of reports for completed tasks.";
    } else if (!completed) {
      dom.exportHint.textContent = "A report exists only for a task that completed.";
    } else {
      dom.exportHint.textContent = "The report is written to the destination you choose.";
    }
  }

  /* -------------------------------------------------------------- report */

  /**
   * Render a report from the **data** the Rust side sent.
   *
   * Nothing from the scanned target becomes markup: every value is written
   * as a text node, so a banner or a finding title that contains markup is
   * shown as the characters it is. The GUI never injects the engine's HTML
   * document into this window — its inline style and inline script are
   * blocked by the window's Content Security Policy, and the policy stays
   * as it is.
   */
  function renderReport(payload) {
    clear(dom.reportBody);
    if (!payload || typeof payload !== "object") {
      dom.reportBody.appendChild(
        el("p", { class: "empty", text: "No report is loaded." })
      );
      return;
    }

    const report = payload.report || {};
    const facts = [
      ["Target", String(report.target || "")],
      ["Scan", String(report.scan_type || "")],
      ["Finished", String(report.timestamp || "")],
      ["Duration", String(report.duration_ms || 0) + " ms"],
    ];

    const summary = el("dl", { class: "report-facts" });
    for (const fact of facts) {
      summary.appendChild(el("dt", { class: "report-facts__name", text: fact[0] }));
      summary.appendChild(el("dd", { class: "report-facts__value", text: fact[1] }));
    }
    dom.reportBody.appendChild(summary);

    const findings = Array.isArray(report.findings) ? report.findings : [];
    dom.reportBody.appendChild(
      el("h3", { class: "report-heading", text: "Findings (" + findings.length + ")" })
    );
    if (findings.length === 0) {
      dom.reportBody.appendChild(el("p", { class: "empty", text: "No findings." }));
    }
    for (const finding of findings) {
      const children = [
        el("p", { class: "report-finding__title", text: String(finding.title || "") }),
        el("p", { class: "report-finding__meta" }, [
          el("span", {
            class: "report-finding__severity report-finding__severity--" + String(finding.severity || ""),
            text: String(finding.severity || ""),
          }),
          el("span", { class: "report-finding__category", text: String(finding.category || "") }),
          el("span", { class: "report-finding__location", text: String(finding.location || "") }),
        ]),
      ];
      if (finding.description) {
        children.push(
          el("p", { class: "report-finding__description", text: String(finding.description) })
        );
      }
      if (finding.evidence) {
        children.push(el("p", { class: "report-finding__evidence", text: String(finding.evidence) }));
      }
      if (finding.remediation) {
        children.push(
          el("p", { class: "report-finding__remediation", text: String(finding.remediation) })
        );
      }
      dom.reportBody.appendChild(el("div", { class: "report-finding" }, children));
    }

    const ports = Array.isArray(report.open_ports) ? report.open_ports : [];
    dom.reportBody.appendChild(
      el("h3", { class: "report-heading", text: "Open ports (" + ports.length + ")" })
    );
    if (ports.length === 0) {
      dom.reportBody.appendChild(el("p", { class: "empty", text: "No open ports." }));
    } else {
      const rows = el("tbody", null, []);
      for (const port of ports) {
        rows.appendChild(
          el("tr", null, [
            el("td", { text: String(port.port) }),
            el("td", { text: String(port.protocol || "") }),
            el("td", { text: String(port.status || "") }),
            el("td", { text: String(port.service || "") }),
            el("td", { text: String(port.version || "") }),
            el("td", { text: String(port.banner || "") }),
          ])
        );
      }
      dom.reportBody.appendChild(
        el("table", { class: "report-table" }, [
          el("thead", null, [
            el("tr", null, [
              el("th", { scope: "col", text: "Port" }),
              el("th", { scope: "col", text: "Protocol" }),
              el("th", { scope: "col", text: "Status" }),
              el("th", { scope: "col", text: "Service" }),
              el("th", { scope: "col", text: "Version" }),
              el("th", { scope: "col", text: "Banner" }),
            ]),
          ]),
          rows,
        ])
      );
    }

    const services = Array.isArray(report.services) ? report.services : [];
    dom.reportBody.appendChild(
      el("h3", { class: "report-heading", text: "Services (" + services.length + ")" })
    );
    if (services.length === 0) {
      dom.reportBody.appendChild(el("p", { class: "empty", text: "No services." }));
    } else {
      const rows = el("tbody", null, []);
      for (const service of services) {
        rows.appendChild(
          el("tr", null, [
            el("td", { text: String(service.service || "") }),
            el("td", { text: String(service.version || "") }),
            el("td", { text: String(service.banner || "") }),
          ])
        );
      }
      dom.reportBody.appendChild(
        el("table", { class: "report-table" }, [
          el("thead", null, [
            el("tr", null, [
              el("th", { scope: "col", text: "Service" }),
              el("th", { scope: "col", text: "Version" }),
              el("th", { scope: "col", text: "Banner" }),
            ]),
          ]),
          rows,
        ])
      );
    }

    if (payload.document) {
      dom.reportBody.appendChild(el("h3", { class: "report-heading", text: "Document" }));
      dom.reportBody.appendChild(
        el("pre", { class: "report-document", text: String(payload.document) })
      );
    }
  }

  async function showReport() {
    if (!invoke || !dom.reportButton || dom.reportButton.disabled) return;
    const taskId = dom.exportTask ? String(dom.exportTask.value || "").trim() : "";
    const format = dom.exportFormat ? String(dom.exportFormat.value || "") : "";
    if (!taskId) return;

    dom.reportButton.disabled = true;
    setBanner("report", null);
    try {
      const payload = await invoke(
        "report_for",
        commandArgs([
          ["task_id", taskId],
          ["format", format],
        ])
      );
      renderReport(payload);
    } catch (error) {
      clear(dom.reportBody);
      dom.reportBody.appendChild(
        el("p", { class: "empty", text: "The report could not be read: " + errorMessage(error) })
      );
      setBanner("report", {
        level: "error",
        title: "The report could not be shown",
        message: errorMessage(error),
      });
    } finally {
      renderExportState();
    }
  }

  /* ---------------------------------------------------------- task kinds */
  function isKindAvailable(kind) {
    if (!kind) return false;
    if (!kind.supported) return false;
    if (!kind.manual_exposable) return false;
    const operation = state.operationsById.get(kind.operation_id);
    if (operation && !operation.manual_exposable) return false;
    return true;
  }

  function kindUnavailableReason(kind) {
    if (!kind) return "";
    if (kind.unavailable_reason) return String(kind.unavailable_reason);
    if (!kind.supported) {
      return "The connected daemon does not advertise this capability.";
    }
    if (!kind.manual_exposable) {
      return "This operation is not exposed on the manual surface.";
    }
    return "";
  }

  function renderTaskKinds() {
    clear(dom.taskKindList);

    if (state.taskKinds.length === 0) {
      dom.taskKindList.appendChild(
        el("li", {
          class: "empty",
          text: state.connected
            ? "The connected daemon published no task kinds."
            : "Not connected to a daemon yet.",
        })
      );
      return;
    }

    for (const kind of state.taskKinds) {
      const operation = state.operationsById.get(kind.operation_id) || null;
      const displayName = operation ? operation.display_name : String(kind.variant || "");
      const risk = operation ? String(operation.risk || "") : "";
      const available = isKindAvailable(kind);
      const reason = kindUnavailableReason(kind);
      const selected = kind.variant === state.selectedVariant;

      const classes = ["task-kind"];
      if (!available) classes.push("task-kind--unavailable");
      if (selected) classes.push("task-kind--selected");

      const children = [
        el("span", { class: "task-kind__head" }, [
          el("span", { class: "task-kind__name", text: displayName }),
          el("span", { class: "task-kind__risk", text: risk }),
        ]),
      ];
      if (!available) {
        children.push(
          el("span", { class: "task-kind__risk", text: "unavailable" })
        );
        if (reason) {
          children.push(el("span", { class: "task-kind__reason", text: reason }));
        }
      }

      const entry = el(
        "button",
        {
          type: "button",
          class: classes.join(" "),
          title: String(kind.variant || "") + (kind.capability_name ? " — " + kind.capability_name : ""),
          "aria-pressed": selected ? "true" : "false",
          onclick: function () {
            selectTaskKind(kind);
          },
        },
        children
      );

      dom.taskKindList.appendChild(el("li", { class: "task-kind__item" }, [entry]));
    }
  }

  /* ---------------------------------------------------------------- form */

  function setFormError(message) {
    if (!state.formErrorNode) return;
    state.formErrorNode.hidden = !message;
    state.formErrorNode.textContent = message ? String(message) : "";
  }

  function clearFieldErrors() {
    for (const node of state.fieldErrorNodes.values()) {
      node.hidden = true;
      node.textContent = "";
    }
    for (const node of dom.formBody.querySelectorAll(".field--invalid")) {
      node.classList.remove("field--invalid");
    }
  }

  function setFieldError(name, message) {
    const node = state.fieldErrorNodes.get(name);
    if (!node) return;
    node.hidden = false;
    node.textContent = String(message);
    const field = node.closest(".field");
    if (field) field.classList.add("field--invalid");
  }

  /**
   * Build one control for one published control descriptor.
   *
   * The returned `reader` maps the operator's current input to the wire
   * value: a filled control sends its value, a blank control sends `null`.
   */
  function buildControl(control, fieldId, labelId) {
    const kind = String(control.kind || "text");
    const name = String(control.name || "");

    if (kind === "number") {
      const input = el("input", {
        type: "number",
        id: fieldId,
        name: name,
        class: "field__input",
        autocomplete: "off",
        spellcheck: "false",
      });
      if (control.minimum !== null && control.minimum !== undefined) {
        input.setAttribute("min", String(control.minimum));
      }
      if (control.maximum !== null && control.maximum !== undefined) {
        input.setAttribute("max", String(control.maximum));
      }
      if (control.placeholder) input.setAttribute("placeholder", String(control.placeholder));
      if (control.required) {
        input.setAttribute("required", "");
        input.setAttribute("aria-required", "true");
      }
      const reader = function () {
        const raw = input.value.trim();
        if (raw === "") return { value: null, error: null };
        if (input.validity && input.validity.badInput) {
          return { value: null, error: "Enter a number." };
        }
        const parsed = Number(raw);
        if (!Number.isFinite(parsed)) return { value: null, error: "Enter a number." };
        return { value: parsed, error: null };
      };
      return { node: input, reader: reader };
    }

    if (kind === "checkbox") {
      /* Tri-state: "not set" is a distinct value from false, because a null
         leaves the engine's own default in place. */
      const select = el("select", {
        id: fieldId,
        name: name,
        class: "field__input",
      });
      select.appendChild(el("option", { value: "", text: "Not set" }));
      select.appendChild(el("option", { value: "true", text: "Yes" }));
      select.appendChild(el("option", { value: "false", text: "No" }));
      if (control.required) select.setAttribute("aria-required", "true");
      const reader = function () {
        if (select.value === "true") return { value: true, error: null };
        if (select.value === "false") return { value: false, error: null };
        return { value: null, error: null };
      };
      return { node: select, reader: reader };
    }

    if (kind === "list") {
      /* One row is present from the start, so the operator can type straight
         into a text input. "Add" appends another row, "Remove" drops one. */
      const entries = [""];
      const list = el("ul", {
        class: "list-control",
        id: fieldId,
        role: "group",
        "aria-labelledby": labelId,
      });

      const renderRows = function () {
        clear(list);
        for (let index = 0; index < entries.length; index += 1) {
          const row = el("li", { class: "list-control__row" });
          const input = el("input", {
            type: "text",
            class: "field__input",
            value: entries[index],
            autocomplete: "off",
            spellcheck: "false",
            "aria-label": "Entry " + String(index + 1),
          });
          input.addEventListener("input", function () {
            entries[index] = input.value;
          });
          const remove = el("button", {
            type: "button",
            class: "button button--small",
            text: "Remove",
          });
          remove.addEventListener("click", function () {
            entries.splice(index, 1);
            renderRows();
          });
          row.appendChild(input);
          row.appendChild(remove);
          list.appendChild(row);
        }
        if (entries.length === 0) {
          list.appendChild(el("li", { class: "empty", text: "No entries." }));
        }
      };

      const add = el("button", { type: "button", class: "button button--small", text: "Add" });
      add.addEventListener("click", function () {
        entries.push("");
        renderRows();
        const inputs = list.querySelectorAll("input");
        if (inputs.length) inputs[inputs.length - 1].focus();
      });

      renderRows();

      const wrapper = el("div", { class: "field__listwrap" }, [list]);
      wrapper.appendChild(el("div", { class: "list-control__actions" }, [add]));

      const reader = function () {
        const values = [];
        for (const entry of entries) {
          const trimmed = String(entry).trim();
          if (trimmed !== "") values.push(trimmed);
        }
        /* No entries means the operator supplied nothing: send null, not []. */
        return { value: values.length ? values : null, error: null };
      };
      return { node: wrapper, reader: reader };
    }

    if (kind === "json") {
      const area = el("textarea", {
        id: fieldId,
        name: name,
        class: "field__input field__textarea",
        rows: 6,
        spellcheck: "false",
      });
      if (control.placeholder) area.setAttribute("placeholder", String(control.placeholder));
      if (control.required) area.setAttribute("aria-required", "true");
      const reader = function () {
        const raw = area.value.trim();
        if (raw === "") return { value: null, error: null };
        try {
          return { value: JSON.parse(raw), error: null };
        } catch (error) {
          return { value: null, error: "Enter valid JSON: " + errorMessage(error) };
        }
      };
      return { node: area, reader: reader };
    }

    /* "text", and the documented fallback for any unrecognized kind. */
    const input = el("input", {
      type: "text",
      id: fieldId,
      name: name,
      class: "field__input",
      autocomplete: "off",
      spellcheck: "false",
    });
    if (control.placeholder) input.setAttribute("placeholder", String(control.placeholder));
    if (control.required) {
      input.setAttribute("required", "");
      input.setAttribute("aria-required", "true");
    }
    const reader = function () {
      const raw = input.value;
      return { value: raw === "" ? null : raw, error: null };
    };
    return { node: input, reader: reader };
  }

  function renderControl(control, index) {
    const name = String(control.name || "");
    const kind = String(control.kind || "text");
    const fieldId = "ctl-" + String(index) + "-" + domId(name);
    const labelId = fieldId + "-label";
    const isGroup = kind === "list";

    const field = el("div", { class: "field field--" + domId(kind) });

    const nameSpan = el("span", { class: "field__name", text: name });
    const labelChildren = [nameSpan];
    if (control.required) {
      labelChildren.push(el("span", { class: "field__required", "aria-hidden": "true", text: "*" }));
    }

    if (isGroup) {
      field.appendChild(el("span", { class: "field__label", id: labelId }, labelChildren));
    } else {
      field.appendChild(el("label", { class: "field__label", for: fieldId }, labelChildren));
    }

    const built = buildControl(control, fieldId, labelId);
    field.appendChild(built.node);
    state.formReaders.set(name, built.reader);

    const meta = [];
    if (control.format) meta.push("format: " + String(control.format));
    if (control.minimum !== null && control.minimum !== undefined) {
      meta.push("min: " + String(control.minimum));
    }
    if (control.maximum !== null && control.maximum !== undefined) {
      meta.push("max: " + String(control.maximum));
    }
    if (meta.length) {
      field.appendChild(el("p", { class: "field__meta", text: meta.join(" · ") }));
    }
    if (control.description) {
      field.appendChild(el("p", { class: "field__help", text: String(control.description) }));
    }

    const errorNode = el("p", { class: "field__error", role: "alert", hidden: true });
    state.fieldErrorNodes.set(name, errorNode);
    field.appendChild(errorNode);

    return field;
  }

  function renderForm(form) {
    state.form = form;
    state.formReaders = new Map();
    state.fieldErrorNodes = new Map();
    clear(dom.formBody);

    const displayName = form.display_name ? String(form.display_name) : "";
    dom.formSubheading.textContent = displayName
      ? displayName + " — " + String(form.variant || "")
      : String(form.variant || "");

    state.formErrorNode = el("p", { class: "form-error", role: "alert", hidden: true });
    dom.formBody.appendChild(state.formErrorNode);

    const unavailableReason = state.selectedKind ? kindUnavailableReason(state.selectedKind) : "";
    if (form.disabled_reason || unavailableReason) {
      dom.formBody.appendChild(
        el("p", {
          class: "notice",
          role: "status",
          text: String(form.disabled_reason || unavailableReason),
        })
      );
    }

    const formElement = el("form", {
      class: "op-form",
      id: "operation-form",
      /* The daemon is the validator: a blank required field is sent as null
         and rejected with a field-level error there, so the webview must not
         block the submission. */
      novalidate: true,
    });
    formElement.addEventListener("submit", onSubmitForm);

    const fields = el("div", { class: "op-form__fields" });
    const controls = Array.isArray(form.controls) ? form.controls : [];
    controls.forEach(function (control, index) {
      fields.appendChild(renderControl(control, index));
    });
    formElement.appendChild(fields);

    const submit = el("button", {
      type: "submit",
      class: "button button--primary",
      text: "Run",
    });
    submit.disabled = !form.submit_enabled || !isKindAvailable(state.selectedKind);
    state.formSubmit = submit;

    const actions = el("div", { class: "op-form__actions" }, [submit]);
    if (submit.disabled) {
      actions.appendChild(
        el("span", {
          class: "op-form__hint",
          text: String(form.disabled_reason || unavailableReason || "This operation cannot be run right now."),
        })
      );
    }
    formElement.appendChild(actions);

    dom.formBody.appendChild(formElement);
  }

  function onSubmitForm(event) {
    event.preventDefault();
    if (!state.form || state.submitting) return;

    setFormError(null);
    clearFieldErrors();

    const values = {};
    let failed = false;
    const controls = Array.isArray(state.form.controls) ? state.form.controls : [];
    for (const control of controls) {
      const name = String(control.name || "");
      const reader = state.formReaders.get(name);
      if (!reader) continue;
      const result = reader();
      if (result.error) {
        setFieldError(name, result.error);
        failed = true;
        values[name] = null;
      } else {
        values[name] = result.value;
      }
    }

    if (failed) {
      setFormError("Fix the highlighted field(s) before running this operation.");
      return;
    }

    state.submitting = true;
    if (state.formSubmit) state.formSubmit.disabled = true;

    const variant = String(state.form.variant || "");
    invoke("submit_task", commandArgs([["variant", variant], ["values", values]]))
      .then(function (response) {
        const taskId = response && response.task_id ? String(response.task_id) : "";
        if (taskId) {
          state.selectedTaskId = taskId;
        }
        return refreshSnapshot().then(function () {
          highlightSelectedTask();
        });
      })
      .catch(function (error) {
        const message = errorMessage(error);
        setFormError(message);
        applyServerFieldErrors(message);
      })
      .then(function () {
        state.submitting = false;
        if (state.formSubmit) {
          state.formSubmit.disabled =
            !state.form.submit_enabled || !isKindAvailable(state.selectedKind);
        }
      });
  }

  /**
   * Attach a daemon-side rejection to the field it names, when the message
   * mentions one. The message is shown verbatim either way.
   */
  function applyServerFieldErrors(message) {
    if (!message || !state.form) return;
    const controls = Array.isArray(state.form.controls) ? state.form.controls : [];
    for (const control of controls) {
      const name = String(control.name || "");
      if (!name) continue;
      let pattern;
      try {
        pattern = new RegExp("\\b" + escapeRegExp(name) + "\\b");
      } catch (error) {
        continue;
      }
      if (pattern.test(message)) setFieldError(name, message);
    }
  }

  async function selectTaskKind(kind) {
    if (!kind) return;
    state.selectedVariant = String(kind.variant || "");
    state.selectedKind = kind;
    renderTaskKinds();

    clear(dom.formBody);
    state.form = null;
    state.formReaders = new Map();
    state.fieldErrorNodes = new Map();
    state.formSubmit = null;
    state.formErrorNode = null;
    dom.formSubheading.textContent = "";
    dom.formBody.appendChild(el("p", { class: "empty", text: "Loading the form…" }));

    if (!requireIpc()) return;

    try {
      const form = await invoke("form_for", { variant: state.selectedVariant });
      if (state.selectedVariant !== String(kind.variant || "")) return;
      renderForm(form);
    } catch (error) {
      if (state.selectedVariant !== String(kind.variant || "")) return;
      clear(dom.formBody);
      dom.formBody.appendChild(
        el("p", { class: "form-error", role: "alert", text: errorMessage(error) })
      );
    }
  }

  /* --------------------------------------------------------------- tasks */

  function statusClass(status) {
    switch (String(status)) {
      case "queued":
        return "queued";
      case "running":
        return "running";
      case "completed":
        return "completed";
      case "failed":
        return "failed";
      case "cancelled":
        return "cancelled";
      case "awaiting_policy":
        return "awaiting";
      default:
        return "unknown";
    }
  }

  function isTerminalStatus(status) {
    const className = statusClass(status);
    return className === "completed" || className === "failed" || className === "cancelled";
  }

  function levelClass(level) {
    const cleaned = String(level === null || level === undefined ? "" : level)
      .toLowerCase()
      .replace(/[^a-z0-9_-]/g, "");
    return cleaned === "" ? "plain" : cleaned;
  }

  function renderResult(result) {
    const box = el("section", { class: "result" });
    const kindLabel = result.kind_label ? String(result.kind_label) : String(result.kind || "");
    box.appendChild(el("h4", { class: "result__kind", text: kindLabel }));

    if (result.summary) {
      box.appendChild(el("p", { class: "result__summary", text: String(result.summary) }));
    }

    const fields = Array.isArray(result.summary_fields) ? result.summary_fields : [];
    if (fields.length) {
      const list = el("dl", { class: "result__fields" });
      for (const pair of fields) {
        const name = Array.isArray(pair) ? pair[0] : "";
        const value = Array.isArray(pair) ? pair[1] : "";
        list.appendChild(el("dt", { text: String(name === null || name === undefined ? "" : name) }));
        list.appendChild(
          el("dd", { text: String(value === null || value === undefined ? "" : value) })
        );
      }
      box.appendChild(list);
    }

    const artifacts = Array.isArray(result.artifacts) ? result.artifacts : [];
    if (artifacts.length) {
      const list = el("ul", { class: "result__artifacts" });
      for (const artifact of artifacts) {
        const id = artifact && artifact.artifact_id ? String(artifact.artifact_id) : "";
        const summary = artifact && artifact.summary ? String(artifact.summary) : "";
        list.appendChild(
          el("li", { class: "artifact" }, [
            el("span", { class: "artifact__id", text: id }),
            summary ? el("span", { class: "artifact__summary", text: summary }) : null,
          ])
        );
      }
      box.appendChild(list);
    }

    /* An unknown kind degrades to a generic JSON render rather than an
       empty panel. */
    const generic = Boolean(result.generic);
    if (generic || result.supports_json_detail !== false) {
      const details = el("details", { class: "result__json" });
      if (generic) details.open = true;
      details.appendChild(
        el("summary", {
          class: "result__json-summary",
          text: generic ? "Raw JSON (unrecognized result kind)" : "Raw JSON",
        })
      );
      const payload = el("pre", { class: "result__payload" });
      payload.textContent = safeStringify(result.payload);
      details.appendChild(payload);
      box.appendChild(details);
    }

    return box;
  }

  function renderTaskCard(task) {
    const taskId = String(task.task_id || "");
    const status = String(task.status || "unknown");
    const card = el("article", { class: "task task--" + statusClass(status) });

    const header = el("header", { class: "task__header" }, [
      el("h3", { class: "task__variant", text: String(task.variant || "") }),
      el("span", { class: "badge badge--" + statusClass(status), text: status }),
    ]);
    if (taskId && taskId === state.selectedTaskId) {
      header.appendChild(el("span", { class: "badge badge--accent", text: "selected" }));
    }
    card.appendChild(header);
    card.appendChild(el("code", { class: "task__id", text: taskId, title: taskId }));

    const total = task.total;
    const completed = Number(task.completed || 0);
    if (typeof total === "number" && isFinite(total) && total > 0) {
      const percentage = Math.max(0, Math.min(100, (completed / total) * 100));
      const bar = el("div", {
        class: "progress",
        role: "progressbar",
        "aria-valuemin": 0,
        "aria-valuemax": total,
        "aria-valuenow": completed,
        "aria-label": "Task progress",
      });
      const fill = el("div", { class: "progress__fill" });
      fill.style.width = percentage.toFixed(2) + "%";
      bar.appendChild(fill);
      card.appendChild(bar);
    }

    if (task.message) {
      card.appendChild(el("p", { class: "task__message", text: String(task.message) }));
    }

    const logs = Array.isArray(task.logs) ? task.logs : [];
    if (logs.length) {
      const list = el("ul", { class: "logs" });
      for (const line of logs) {
        const level = line && line.level ? String(line.level) : "";
        const message = line && line.message ? String(line.message) : "";
        list.appendChild(
          el("li", { class: "log log--" + levelClass(level) }, [
            el("span", { class: "log__level", text: level }),
            el("span", { class: "log__message", text: message }),
          ])
        );
      }
      card.appendChild(list);
    }

    const dropped = Number(task.dropped_log_lines || 0);
    if (dropped > 0) {
      card.appendChild(
        el("p", {
          class: "task__dropped",
          text: "… " + String(dropped) + " earlier lines dropped",
        })
      );
    }

    if (task.error) {
      card.appendChild(
        el("p", { class: "task__error", role: "status", text: String(task.error) })
      );
    }

    // A task waiting on a policy decision shows the policy's own question,
    // and offers the operator the two answers. The prompt's wording is the
    // policy's, not the GUI's.
    const prompt = task.policy_prompt;
    if (status === "awaiting_policy" && prompt) {
      const box = el("div", { class: "task__prompt", role: "group", "aria-label": "Policy decision" });
      box.appendChild(
        el("p", {
          class: "task__prompt-message",
          role: "status",
          text: String(prompt.message || "This operation needs an explicit approval."),
        })
      );
      const confirmationClass = prompt.confirmation_class
        ? String(prompt.confirmation_class)
        : "";
      if (confirmationClass) {
        box.appendChild(
          el("p", { class: "task__prompt-class", text: confirmationClass })
        );
      }
      const decision = el("div", { class: "task__prompt-actions" });
      const approve = el("button", {
        type: "button",
        class: "button button--primary button--small",
        text: "Approve",
      });
      approve.addEventListener("click", function () {
        answerPolicyDecision(taskId, true);
      });
      const reject = el("button", {
        type: "button",
        class: "button button--danger button--small",
        text: "Reject",
      });
      reject.addEventListener("click", function () {
        answerPolicyDecision(taskId, false);
      });
      decision.appendChild(approve);
      decision.appendChild(reject);
      box.appendChild(decision);
      card.appendChild(box);
    }

    const actions = el("div", { class: "task__actions" });
    const cancel = el("button", {
      type: "button",
      class: "button button--danger button--small",
      text: "Cancel",
    });
    if (isTerminalStatus(status)) cancel.disabled = true;
    cancel.addEventListener("click", function () {
      cancelTask(taskId);
    });
    actions.appendChild(cancel);
    card.appendChild(actions);

    if (status === "completed" && task.result) {
      card.appendChild(renderResult(task.result));
    }

    return card;
  }

  function preserveDetailsState(previous, next) {
    const before = previous.querySelectorAll("details");
    const after = next.querySelectorAll("details");
    for (let index = 0; index < before.length && index < after.length; index += 1) {
      after[index].open = before[index].open;
    }
  }

  function renderTaskList() {
    clear(dom.taskList);
    state.taskNodes.clear();

    if (state.tasks.size === 0) {
      dom.taskList.appendChild(
        el("p", {
          class: "empty",
          text: state.connected
            ? "No tasks yet. Run an operation to see it here."
            : "Not connected to a daemon yet.",
        })
      );
      renderCancelActiveState();
      return;
    }

    for (const task of state.tasks.values()) {
      const node = renderTaskCard(task);
      state.taskNodes.set(String(task.task_id || ""), node);
      dom.taskList.appendChild(node);
    }
    renderCancelActiveState();
  }

  function updateTaskNode(task) {
    const taskId = String(task.task_id || "");
    const previous = state.taskNodes.get(taskId);
    const node = renderTaskCard(task);
    if (previous && previous.parentNode) {
      preserveDetailsState(previous, node);
      previous.parentNode.replaceChild(node, previous);
      state.taskNodes.set(taskId, node);
    } else {
      renderTaskList();
    }
    renderCancelActiveState();
  }

  function renderCancelActiveState() {
    if (!dom.cancelActiveButton) return;
    let active = false;
    for (const task of state.tasks.values()) {
      if (!isTerminalStatus(String(task.status || ""))) {
        active = true;
        break;
      }
    }
    dom.cancelActiveButton.disabled = !active;
  }

  function renderCounters() {
    if (!dom.dashboardCounters) return;
    clear(dom.dashboardCounters);
    const counters = [];
    if (state.droppedLogLines > 0) {
      counters.push("Dropped log lines: " + String(state.droppedLogLines));
    }
    if (state.unknownTaskEvents > 0) {
      counters.push("Unknown task events: " + String(state.unknownTaskEvents));
    }
    for (const counter of counters) {
      dom.dashboardCounters.appendChild(el("span", { class: "counter", text: counter }));
    }
    dom.dashboardCounters.hidden = counters.length === 0;
  }

  function enforceTaskCap() {
    if (state.tasks.size <= MAX_TRACKED_TASKS) return false;
    let excess = state.tasks.size - MAX_TRACKED_TASKS;
    const ids = Array.from(state.tasks.keys());

    /* Prefer evicting finished tasks, oldest first. */
    for (const id of ids) {
      if (excess <= 0) break;
      const task = state.tasks.get(id);
      if (task && isTerminalStatus(String(task.status || ""))) {
        state.tasks.delete(id);
        state.taskNodes.delete(id);
        excess -= 1;
      }
    }
    for (const id of ids) {
      if (excess <= 0) break;
      if (state.tasks.has(id)) {
        state.tasks.delete(id);
        state.taskNodes.delete(id);
        excess -= 1;
      }
    }
    return true;
  }

  function upsertTask(task) {
    if (!task || typeof task !== "object") return;
    const taskId = String(task.task_id || "");
    if (!taskId) return;

    const existing = state.tasks.get(taskId);
    const merged = existing ? Object.assign({}, existing, task) : Object.assign({}, task);
    if (Array.isArray(task.logs)) {
      merged.logs = task.logs.slice(-MAX_LOG_LINES_PER_TASK);
    } else if (!Array.isArray(merged.logs)) {
      merged.logs = [];
    }

    const isNew = !state.tasks.has(taskId);
    state.tasks.set(taskId, merged);
    const evicted = enforceTaskCap();
    if (isNew || evicted) renderTaskList();
    else updateTaskNode(merged);
    offerTaskForExport(merged);
    renderExportState();
  }

  /**
   * Point the export panel at a task that just completed.
   *
   * The operator's choice is never overwritten: the panel is only filled in
   * when it is still empty.
   */
  function offerTaskForExport(task) {
    if (!dom.exportTask) return;
    if (String(dom.exportTask.value || "").trim()) return;
    if (!task || String(task.status || "") !== "completed") return;
    dom.exportTask.value = String(task.task_id || "");
  }

  function removeTask(taskId) {
    const id = String(taskId || "");
    if (!id) return;
    state.tasks.delete(id);
    state.taskNodes.delete(id);
    if (state.selectedTaskId === id) state.selectedTaskId = null;
    renderTaskList();
  }

  function appendLogs(taskId, lines, dropped) {
    const id = String(taskId || "");
    const task = state.tasks.get(id);
    if (!task) {
      /* A log line for a task this window does not know about: the daemon
         counts these and reports them through the counters event. */
      return;
    }
    const existing = Array.isArray(task.logs) ? task.logs.slice() : [];
    const incoming = Array.isArray(lines) ? lines : [];
    let merged = existing.concat(incoming);
    let droppedCount = Number(task.dropped_log_lines || 0) + Number(dropped || 0);
    if (merged.length > MAX_LOG_LINES_PER_TASK) {
      const overflow = merged.length - MAX_LOG_LINES_PER_TASK;
      merged = merged.slice(overflow);
      droppedCount += overflow;
    }
    task.logs = merged;
    task.dropped_log_lines = droppedCount;
    updateTaskNode(task);
  }

  function applySnapshot(snapshot) {
    if (!snapshot || typeof snapshot !== "object") return;
    const tasks = Array.isArray(snapshot.tasks) ? snapshot.tasks : [];
    state.tasks.clear();
    for (const task of tasks) {
      if (!task || typeof task !== "object") continue;
      const taskId = String(task.task_id || "");
      if (!taskId) continue;
      const copy = Object.assign({}, task);
      copy.logs = Array.isArray(task.logs) ? task.logs.slice(-MAX_LOG_LINES_PER_TASK) : [];
      state.tasks.set(taskId, copy);
    }
    state.droppedLogLines = Math.max(
      state.droppedLogLines,
      Number(snapshot.dropped_log_lines || 0)
    );
    state.unknownTaskEvents = Math.max(
      state.unknownTaskEvents,
      Number(snapshot.unknown_task_events || 0)
    );
    renderTaskList();
    renderCounters();
    renderExportState();
  }

  function highlightSelectedTask() {
    if (!state.selectedTaskId) return;
    const node = state.taskNodes.get(state.selectedTaskId);
    if (node && typeof node.scrollIntoView === "function") {
      node.scrollIntoView({ block: "nearest" });
    }
  }

  /* ------------------------------------------------------------ commands */

  async function refreshSnapshot() {
    if (!invoke) return;
    try {
      const snapshot = await invoke("task_snapshot");
      applySnapshot(snapshot);
    } catch (error) {
      setBanner("snapshot", {
        level: "error",
        title: "Could not read the task list",
        message: errorMessage(error),
      });
    }
  }

  async function cancelTask(taskId) {
    if (!invoke) return;
    try {
      await invoke("cancel_task", commandArgs([["task_id", String(taskId || "")]]));
    } catch (error) {
      setBanner("cancel", {
        level: "error",
        title: "Cancellation failed",
        message: errorMessage(error),
      });
    }
  }

  async function cancelActive() {
    if (!invoke) return;
    try {
      await invoke("cancel_active");
    } catch (error) {
      setBanner("cancel", {
        level: "error",
        title: "Cancellation failed",
        message: errorMessage(error),
      });
    }
  }

  // Answer a pending policy decision.
  //
  // The answer names yes or no; it never names the authorisation. What the
  // answer permits is computed by the policy kernel on the daemon's side.
  async function answerPolicyDecision(taskId, approved) {
    if (!invoke) return;
    try {
      await invoke(
        "approve_task",
        commandArgs([
          ["task_id", String(taskId || "")],
          ["approved", Boolean(approved)],
          [
            "reason",
            approved ? "approved in the desktop GUI" : "declined in the desktop GUI",
          ],
        ])
      );
      await refreshSnapshot();
    } catch (error) {
      setBanner("policy", {
        level: "error",
        title: approved ? "Approval failed" : "Refusal failed",
        message: errorMessage(error),
      });
    }
  }

  async function refreshCoverage() {
    if (!invoke) return;
    try {
      const rows = await invoke("coverage");
      state.coverage = Array.isArray(rows) ? rows : [];
      renderCoverage();
    } catch (error) {
      state.coverage = [];
      renderCoverage();
      setBanner("coverage", {
        level: "error",
        title: "Could not read the coverage table",
        message: errorMessage(error),
      });
    }
  }

  async function refreshDaemonStatus() {
    if (!invoke) return;
    try {
      const status = await invoke("daemon_status");
      state.daemon = status || null;
      applyDefaultSocketPath(state.daemon);
      renderDaemonStatus();
      renderExportFormats();
    } catch (error) {
      state.daemon = null;
      renderDaemonStatus();
      renderExportFormats();
      setBanner("daemon", {
        level: "error",
        title: "Could not read the daemon status",
        message: errorMessage(error),
      });
    }
  }

  /* The daemon's default socket path, as the Rust side resolved it. The
     webview holds no copy of it: it learns the path from `daemon_status` and
     shows it in an empty field. A path the operator typed is never
     overwritten. */
  function applyDefaultSocketPath(status) {
    if (!status || !dom.socketInput) return;
    const fallback = String(status.default_socket_path || "").trim();
    if (!fallback) return;
    if (String(dom.socketInput.value || "").trim()) return;
    dom.socketInput.value = fallback;
  }

  async function exportReport(event) {
    if (event) event.preventDefault();
    if (!invoke || !dom.exportButton || dom.exportButton.disabled) return;
    const taskId = dom.exportTask ? String(dom.exportTask.value || "").trim() : "";
    const format = dom.exportFormat ? String(dom.exportFormat.value || "") : "";
    const path = dom.exportPath ? String(dom.exportPath.value || "").trim() : "";
    if (!taskId || !format || !path) return;

    dom.exportButton.disabled = true;
    setBanner("export", null);
    try {
      const written = await invoke(
        "export_report",
        commandArgs([
          ["task_id", taskId],
          ["format", format],
          ["path", path],
        ])
      );
      setBanner("export", {
        level: "info",
        title: "Report exported",
        message: written && written.path ? String(written.path) : path,
      });
    } catch (error) {
      setBanner("export", {
        level: "error",
        title: "The export failed",
        message: errorMessage(error),
      });
    } finally {
      renderExportState();
    }
  }

  async function refreshCatalogLists() {
    if (!invoke) return;
    try {
      const operations = await invoke("list_operations");
      const taskKinds = await invoke("list_task_kinds");
      state.operations = Array.isArray(operations) ? operations : [];
      state.operationsById = new Map();
      for (const operation of state.operations) {
        if (operation && operation.id !== undefined) {
          state.operationsById.set(String(operation.id), operation);
        }
      }
      state.taskKinds = Array.isArray(taskKinds) ? taskKinds : [];
      renderTaskKinds();
    } catch (error) {
      setBanner("catalog", {
        level: "error",
        title: "Could not read the operation catalog",
        message: errorMessage(error),
      });
    }
  }

  async function connect(socketPath) {
    if (!requireIpc()) return;
    /* The socket may be empty. The webview holds no copy of the daemon's
       default path, so an empty socket is not a blank field to be clobbered:
       it is the request for "the GUI's default", and the Rust side resolves
       it to the path the GUI was started with. */
    const socket = String(socketPath || "").trim();

    state.connecting = true;
    if (dom.connectButton) dom.connectButton.disabled = true;
    setBanner("connect", null);
    setBanner("notice", null);
    setBanner("catalog", null);
    setBanner("snapshot", null);

    try {
      const catalog = await invoke("connect_daemon", { socket: socket });
      state.catalog = catalog;
      state.connected = true;
      if (catalog && catalog.socket_path && dom.socketInput) {
        dom.socketInput.value = String(catalog.socket_path);
      }
      renderCatalog();
      if (catalog && catalog.notice) {
        setBanner("notice", {
          level: "warning",
          title: "Daemon notice",
          message: String(catalog.notice),
        });
      }
      await refreshCatalogLists();
      await refreshSnapshot();
      await refreshCoverage();
      await refreshDaemonStatus();
    } catch (error) {
      state.catalog = null;
      state.connected = false;
      state.operations = [];
      state.operationsById = new Map();
      state.taskKinds = [];
      state.selectedKind = null;
      state.coverage = [];
      state.daemon = null;
      renderCatalog();
      renderTaskKinds();
      renderCoverage();
      renderDaemonStatus();
      renderExportFormats();
      renderReport(null);
      setBanner("connect", {
        level: "error",
        title: "Could not connect to the daemon",
        message: errorMessage(error),
      });
    } finally {
      state.connecting = false;
      if (dom.connectButton) dom.connectButton.disabled = false;
    }
  }

  /* -------------------------------------------------------------- events */

  function handleTaskEvent(payload) {
    if (!payload || typeof payload !== "object") {
      state.unknownTaskEvents += 1;
      renderCounters();
      return;
    }
    switch (String(payload.type)) {
      case "task_upsert":
        upsertTask(payload.task);
        break;
      case "task_removed":
        removeTask(payload.task_id);
        break;
      case "logs":
        appendLogs(payload.task_id, payload.lines, payload.dropped);
        break;
      case "counters":
        state.droppedLogLines = Math.max(
          state.droppedLogLines,
          Number(payload.dropped_log_lines || 0)
        );
        state.unknownTaskEvents = Math.max(
          state.unknownTaskEvents,
          Number(payload.unknown_task_events || 0)
        );
        renderCounters();
        break;
      default:
        state.unknownTaskEvents += 1;
        renderCounters();
        break;
    }
  }

  function subscribeToTaskEvents() {
    if (!listen) {
      setBanner("ipc", {
        level: "error",
        title: "Event bridge unavailable",
        message:
          "The Tauri event bridge is not present in this window, so live task updates cannot be received.",
      });
      return;
    }
    listen(TASK_EVENT, function (event) {
      try {
        handleTaskEvent(event && event.payload);
      } catch (error) {
        state.unknownTaskEvents += 1;
        renderCounters();
      }
    }).catch(function (error) {
      setBanner("ipc", {
        level: "error",
        title: "Could not subscribe to task events",
        message: errorMessage(error),
      });
    });
  }

  /* ---------------------------------------------------------------- boot */

  async function boot() {
    renderCatalog();
    renderBanners();
    renderTaskKinds();
    renderTaskList();
    renderCounters();
    renderCoverage();
    renderDaemonStatus();
    renderExportFormats();
    renderReport(null);

    if (dom.connectForm) {
      dom.connectForm.addEventListener("submit", function (event) {
        event.preventDefault();
        const socket = dom.socketInput ? dom.socketInput.value : "";
        connect(socket);
      });
    }

    if (dom.exportForm) {
      dom.exportForm.addEventListener("submit", exportReport);
    }
    if (dom.exportTask) {
      dom.exportTask.addEventListener("input", renderExportState);
    }
    if (dom.exportPath) {
      dom.exportPath.addEventListener("input", renderExportState);
    }
    if (dom.exportFormat) {
      dom.exportFormat.addEventListener("change", renderExportState);
    }
    if (dom.reportButton) {
      dom.reportButton.addEventListener("click", showReport);
    }

    if (dom.cancelActiveButton) {
      dom.cancelActiveButton.addEventListener("click", function () {
        cancelActive();
      });
    }

    subscribeToTaskEvents();

    /* The daemon's default socket path is learned from Rust — the webview
       holds no copy of it — so the status is read before the automatic
       connect, and the field is filled from it. */
    await refreshDaemonStatus();

    /* One automatic connection attempt against the default socket, so the
       window shows the catalog when a daemon is already running. There is no
       retry loop: further attempts are operator-driven. */
    const socket = dom.socketInput ? dom.socketInput.value : "";
    connect(socket);
  }

  boot().catch(function (error) {
    setBanner("boot", {
      level: "error",
      title: "The window failed to start",
      message: errorMessage(error),
    });
  });
})();
