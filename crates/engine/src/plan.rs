//! Apply-plan construction (§8.1).

use std::collections::HashMap;
use std::path::PathBuf;

use uuid::Uuid;

use crate::copy;
use crate::item::ListMode;
use crate::preview::Preview;
use crate::state::CoreState;

/// Everything an Apply needs: the moves and the snapshot summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyPlan {
    /// `(from, to)` for every changed entry, in view order.
    pub moves: Vec<(PathBuf, PathBuf)>,
    /// ` · `-joined description recorded on the snapshot (§8.1).
    pub summary: String,
    /// Whether the rule stack was non-empty — drives the banner's
    /// cleared/kept flags (§8.5 step 4).
    pub had_rules: bool,
    /// Whether any changed entry came from a manual override.
    pub had_overrides: bool,
    /// Whether the plan was built in Folders mode.
    pub is_folders: bool,
}

/// Build the plan from the current preview. Returns `None` unless
/// `can_apply` (§7.7) — the caller must not bypass gating.
pub fn build_plan(state: &CoreState, preview: &Preview) -> Option<ApplyPlan> {
    if !preview.counts.can_apply {
        return None;
    }
    let items: HashMap<Uuid, &crate::item::FileItem> =
        state.files.iter().map(|item| (item.id, item)).collect();
    let mut moves = Vec::new();
    let mut had_overrides = false;
    for entry in preview.entries.iter().filter(|entry| entry.is_changed) {
        let item = items.get(&entry.id)?;
        moves.push((item.path.clone(), item.directory().join(&entry.new_name)));
        had_overrides |= entry.has_override;
    }
    let is_folders = state.list_mode == ListMode::Folders;

    let mut parts: Vec<String> = Vec::new();
    if is_folders {
        parts.push(copy::SUMMARY_FOLDERS_PREFIX.to_string());
    }
    for rule in state
        .rules
        .iter()
        .filter(|rule| rule.is_enabled && rule.is_effective())
    {
        parts.push(rule.summary());
    }
    if state.trims_whitespace {
        parts.push(copy::SUMMARY_TRIM_SPACES.to_string());
    }
    if had_overrides {
        parts.push(copy::SUMMARY_MANUAL_EDITS.to_string());
    }
    Some(ApplyPlan {
        moves,
        summary: parts.join(" · "),
        had_rules: !state.rules.is_empty(),
        had_overrides,
        is_folders,
    })
}

/// What a completed Apply produced (§8.5 step 4's payload, plus what the
/// shell needs to finish up).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ApplyOutcome {
    /// How many renames succeeded.
    pub renamed: u32,
    /// The recorded snapshot, when anything succeeded.
    pub snapshot_id: Option<Uuid>,
    /// User-presentable per-move errors.
    pub errors: Vec<String>,
    /// Watched folders whose root moved — the shell re-arms their watchers.
    pub rearmed_folder_ids: Vec<Uuid>,
    /// Banner flag: rules existed and keep-rules is off, so the SHELL must
    /// now clear the stack through the undoable funnel (§8.5 step 5).
    pub cleared_rules: bool,
    /// Banner flag: rules existed and keep-rules kept them.
    pub kept_rules: bool,
    /// Whether this was a Folders-mode apply.
    pub is_folders: bool,
}

/// The synchronous apply path — the tested reference implementation (§8.6);
/// the shell's worker adds only threading, progress, and cancel around it.
/// Performs §8.5 steps 1–3, 5 (deselect half) and 6 in the normative order;
/// the shell finishes step 4 (events), the rules-clearing half of step 5
/// (undoable funnel), 7 (alerts), and 8 (session save).
pub fn apply_plan(
    state: &mut CoreState,
    plan: &ApplyPlan,
    is_cancelled: &dyn Fn() -> bool,
    on_progress: &mut dyn FnMut(usize),
) -> ApplyOutcome {
    use crate::execute::{perform_moves_hierarchical, rewrite_live_paths};
    use crate::item::standardized;
    use crate::revert::record_snapshot;

    let result = perform_moves_hierarchical(&plan.moves, true, is_cancelled, on_progress);

    // 1–2. Update tracked paths (exact + descendant prefix rewrites).
    let rearmed_folder_ids = rewrite_live_paths(state, &result.succeeded);

    // 3. Record the snapshot, newest first, capped.
    let snapshot_id = if result.succeeded.is_empty() {
        None
    } else {
        Some(record_snapshot(
            state,
            plan.summary.clone(),
            &result.succeeded,
        ))
    };

    // 5 (deselect half). Keep-rules on → deselect exactly the items whose
    // standardized path is a `to` of a success, preventing a double
    // transform on re-Apply.
    if state.keep_rules_after_apply {
        let renamed_targets: std::collections::HashSet<PathBuf> = result
            .succeeded
            .iter()
            .map(|(_, to)| standardized(to))
            .collect();
        for item in &mut state.files {
            if renamed_targets.contains(&standardized(&item.path)) {
                item.is_selected = false;
            }
        }
    }

    // 6. Clear overrides on the applied mode only — the other mode's pending
    // edits survive.
    let applied_directories = plan.is_folders;
    for item in &mut state.files {
        if item.is_directory == applied_directories {
            item.override_name = None;
        }
    }

    ApplyOutcome {
        renamed: result.succeeded.len() as u32,
        snapshot_id,
        errors: result.errors,
        rearmed_folder_ids,
        cleared_rules: plan.had_rules && !state.keep_rules_after_apply,
        kept_rules: plan.had_rules && state.keep_rules_after_apply,
        is_folders: plan.is_folders,
    }
}
