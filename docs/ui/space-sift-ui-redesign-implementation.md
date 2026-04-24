# Space Sift tabbed UI redesign

## Goal

Create a task-focused Windows desktop interface that separates Space Sift's major workflows into clear tabs while preserving the existing safety model: local-first history, read-only exploration, duplicate verification before cleanup, Recycle Bin-first execution, and high-friction permanent delete.

## Recommended information architecture

Use seven top-level tabs:

1. **Overview** — current status, loaded scan, next safe action, high-level metrics.
2. **Scan** — scan root input, start/cancel controls, live progress, current path, scan-rate/skip summaries.
3. **History** — completed scans and interrupted runs, with completed scans reopening into Explorer and interrupted runs showing continuity fields.
4. **Explorer** — read-only browseable result tree, breadcrumbs, sort controls, open-in-Explorer handoff, current-level space map.
5. **Duplicates** — duplicate-analysis status, verified groups, keep-selection helpers, delete-candidate summary.
6. **Cleanup** — cleanup source selection, preview generation, validation issues, Recycle Bin execution, permanent-delete confirmation.
7. **Safety** — durable explanation of unprivileged mode, protected-path behavior, local history, and destructive-action safeguards.

## Why tabs are the right interface here

A single long page makes the app feel like a form. This product is actually a workflow: scan → inspect → verify → preview → execute. Tabs make state and intent explicit without hiding global progress or history. They also let the UI preserve context when a scan is running and a user is reviewing stored results.

## Implementation pattern

### Add tab state

```tsx
type WorkspaceTab =
  | "overview"
  | "scan"
  | "history"
  | "explorer"
  | "duplicates"
  | "cleanup"
  | "safety";

const workspaceTabs: Array<{
  id: WorkspaceTab;
  label: string;
  description: string;
}> = [
  { id: "overview", label: "Overview", description: "Status and next action" },
  { id: "scan", label: "Scan", description: "Start and monitor" },
  { id: "history", label: "History", description: "Completed and interrupted" },
  { id: "explorer", label: "Explorer", description: "Browse result tree" },
  { id: "duplicates", label: "Duplicates", description: "Verify before marking" },
  { id: "cleanup", label: "Cleanup", description: "Preview, then execute" },
  { id: "safety", label: "Safety", description: "Rules and guardrails" },
];

const [activeTab, setActiveTab] = useState<WorkspaceTab>("overview");
```

### Use accessible tab semantics

```tsx
<nav className="workspace-sidebar" aria-label="Space Sift workspace">
  <div role="tablist" aria-orientation="vertical" aria-label="Workspace sections">
    {workspaceTabs.map((tab) => (
      <button
        key={tab.id}
        id={`tab-${tab.id}`}
        type="button"
        role="tab"
        aria-selected={activeTab === tab.id}
        aria-controls={`panel-${tab.id}`}
        tabIndex={activeTab === tab.id ? 0 : -1}
        className="workspace-tab"
        onClick={() => setActiveTab(tab.id)}
      >
        <span className="workspace-tab__label">{tab.label}</span>
        <span className="workspace-tab__description">{tab.description}</span>
      </button>
    ))}
  </div>
</nav>
```

### Render panels as separate sections

```tsx
<section
  id="panel-scan"
  role="tabpanel"
  aria-labelledby="tab-scan"
  hidden={activeTab !== "scan"}
>
  <ScanWorkspace />
</section>
```

Use the existing state and handlers first, then progressively extract each panel into components. Avoid changing backend command contracts for a UI refactor.

## Component extraction

Suggested files:

```text
src/components/AppShell.tsx
src/components/WorkspaceTabs.tsx
src/components/MetricCard.tsx
src/components/StatusPill.tsx
src/features/scan/ScanTab.tsx
src/features/history/HistoryTab.tsx
src/features/history/InterruptedRunsTable.tsx
src/features/explorer/ExplorerTab.tsx
src/features/duplicates/DuplicatesTab.tsx
src/features/cleanup/CleanupTab.tsx
src/features/safety/SafetyTab.tsx
```

Keep business logic in `App.tsx` initially and pass props down. After the UI is stable, extract reducer-style state by feature.

## Interaction rules

- When a scan completes, auto-open the result and switch to **Explorer**.
- When a user clicks **Reopen** in History, load the scan and switch to **Explorer**.
- When duplicate analysis starts, switch to **Duplicates**.
- When duplicate selections create delete candidates, offer a CTA to switch to **Cleanup**.
- Keep live scan status visible in the sidebar or top utility rail even when another tab is active.
- Do not enable Resume unless `run.canResume === true`.
- Permanent delete stays disabled until a confirmation checkbox is checked and a cleanup preview exists.

## Required UI details by tab

### Overview

- Current loaded scan summary.
- Live scan summary when running.
- Top four metrics: total bytes, total files, duplicate reclaimable bytes, cleanup candidate count.
- Next-step CTA based on current state.

### Scan

- Path input with example Windows path.
- Primary Start Scan button.
- Secondary Cancel Scan button visible/enabled only while running.
- Progress metrics: state, files, directories, bytes processed, current path, last update, rate if available.
- Error/notice area close to scan controls.

### History

- Completed scans table with root path, completed time, total bytes, scan id, reopen action.
- Interrupted runs table with `run_id`, `seq`, `status`, `created_at`, `items_scanned`, `errors_count`, progress percent, rate, cancel action, and disabled resume when `can_resume` is false.
- Filters should not hide active warnings without a clear empty-state message.

### Explorer

- Breadcrumbs.
- Current location.
- Sort by size/name.
- Current-level table: name, type, size, usage share, actions.
- Read-only messaging for old scans without entries.

### Duplicates

- Analyze/cancel controls.
- Analysis stage and processed count.
- Verified groups only.
- Keep newest, keep oldest, and explicit keep selection.
- Summary of files marked for later deletion and reclaimable bytes.

### Cleanup

- Source selection: duplicate candidates and cleanup rules.
- Refresh preview CTA.
- Candidate table with source labels, size, and validation status.
- Issues table for excluded/invalid candidates.
- Default action: Move to Recycle Bin.
- Advanced permanent delete section with explicit confirmation.

### Safety

- Unprivileged by default.
- Recycle Bin first.
- Local-only history.
- Protected-path behavior.
- Resume availability explanation: `can_resume` is the actionability source of truth.

## Styling principles

- Use a two-column desktop shell: left tab rail, right content panels.
- Collapse tabs into a two-column or one-column grid below tablet width.
- Use cards for metrics and panels for workflows.
- Keep destructive actions visually distinct and physically separated from safe actions.
- Use plain button elements for tabs and actions.
- Support focus-visible styles and keyboard navigation for tabs.
- Do not rely on color alone for state; include text labels such as `Running`, `Resume unavailable`, `Recycle Bin`.

## Test coverage to add

- Renders all workspace tabs.
- Clicking a tab hides other panels and shows the selected panel.
- Reopen scan switches to Explorer.
- Start duplicate analysis switches to Duplicates.
- Cleanup permanent-delete button remains disabled until preview exists and confirmation is checked.
- Interrupted run Resume button is disabled when `canResume` is false.
- Keyboard navigation moves through tabs with arrow keys, Home, and End.

## Validation commands

```bash
npm run lint
npm run test -- scan
npm run test -- history
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```
