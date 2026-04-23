export type WorkspaceShellLogEvent =
  | "workspace_restore_context_load_failed"
  | "workspace_restore_context_save_failed"
  | "workspace_restore_context_validation_failed"
  | "workspace_auto_switch_applied"
  | "workspace_auto_switch_skipped_duplicate"
  | "workspace_next_safe_action_selected"
  | "workspace_status_notice_rendered";

export const workspaceShellLogger = {
  log(event: WorkspaceShellLogEvent, payload: Record<string, unknown> = {}) {
    console.info("[workspace-shell]", event, payload);
  },
};
