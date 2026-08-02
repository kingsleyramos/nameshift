//! The two-phase, hierarchical move executor (§8.2–8.5) — the most
//! safety-critical code in the app.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::copy;
use crate::item::name_of;
use crate::state::CoreState;

/// Hidden prefix for the two-phase rename's intermediate files. Kept exactly
/// as the historical value so orphan recovery also rescues temps left by
/// earlier releases on the same machine (§24 Q13).
pub const TEMP_PREFIX: &str = ".fne-tmp-";

/// What a batch executor run produced.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MoveOutcome {
    /// Every `(from, to)` that now holds on disk, in commit order.
    pub succeeded: Vec<(PathBuf, PathBuf)>,
    /// User-presentable error messages (§A per-move errors).
    pub errors: Vec<String>,
    /// How many moves were attempted (succeeded or errored). Less than the
    /// requested count means the run was cut short by cancellation.
    pub attempted: usize,
}

/// Rename without ever clobbering: `std::fs::rename` overwrites existing
/// destinations on POSIX, so an explicit existence check guards every move.
/// Uses `symlink_metadata` so symlinks (even broken ones) count as occupied.
/// Long-path safe on Windows via `win_long_path` (§16.1).
fn move_no_clobber(from: &Path, to: &Path) -> std::io::Result<()> {
    let from = crate::platform::win_long_path(from);
    let to = crate::platform::win_long_path(to);
    if to.symlink_metadata().is_ok() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("“{}” already exists", name_of(&to)),
        ));
    }
    std::fs::rename(from, to)
}

/// Temp path for the two-phase rename. Encodes the original filename after
/// the UUID so a batch interrupted between its two moves (crash/power loss)
/// leaves a file [`recover_orphaned_temp_files`] can restore, not an opaque
/// orphan. Falls back to an opaque name when encoding would exceed the
/// 255-byte filename limit (unrecoverable-by-name, but still correct).
fn temp_path_for(source: &Path) -> PathBuf {
    let directory = source.parent().unwrap_or_else(|| Path::new(""));
    let id = Uuid::new_v4();
    let mut buffer = Uuid::encode_buffer();
    let id = id.hyphenated().encode_upper(&mut buffer);
    let candidate = format!("{TEMP_PREFIX}{id}-{}", name_of(source));
    let name = if candidate.len() <= 255 {
        candidate
    } else {
        format!("{TEMP_PREFIX}{id}")
    };
    directory.join(name)
}

/// macOS: the user-immutable (“Locked” / `uchg`) flag. NAS and torrent
/// clients commonly set it on completed files; renaming such a file fails
/// with a misleading “you don't have permission” error, so the rename path
/// unlocks on demand and restores the lock on the file's final name.
#[cfg(target_os = "macos")]
mod lock_flag {
    use std::ffi::CString;
    use std::os::macos::fs::MetadataExt;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    const UF_IMMUTABLE: u32 = 0x0000_0002;

    extern "C" {
        // libc doesn't bind lchflags; it has been in libSystem since 10.6.
        fn lchflags(path: *const libc::c_char, flags: libc::c_uint) -> libc::c_int;
    }

    pub fn is_locked(path: &Path) -> bool {
        path.symlink_metadata()
            .map(|meta| meta.st_flags() & UF_IMMUTABLE != 0)
            .unwrap_or(false)
    }

    pub fn set_locked(path: &Path, locked: bool) -> bool {
        let Ok(metadata) = path.symlink_metadata() else {
            return false;
        };
        let flags = if locked {
            metadata.st_flags() | UF_IMMUTABLE
        } else {
            metadata.st_flags() & !UF_IMMUTABLE
        };
        let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        // lchflags: never follow symlinks (§24 Q8).
        unsafe { lchflags(c_path.as_ptr(), flags) == 0 }
    }
}

#[cfg(not(target_os = "macos"))]
mod lock_flag {
    use std::path::Path;

    pub fn is_locked(_path: &Path) -> bool {
        false
    }
    pub fn set_locked(_path: &Path, _locked: bool) -> bool {
        false
    }
}

/// Windows: temps get the hidden attribute (a dot prefix doesn't hide files
/// there); it is cleared again on the final move (§8.3).
#[cfg(windows)]
fn set_hidden(path: &Path, hidden: bool) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, INVALID_FILE_ATTRIBUTES,
    };
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let attributes = GetFileAttributesW(wide.as_ptr());
        if attributes == INVALID_FILE_ATTRIBUTES {
            return;
        }
        let updated = if hidden {
            attributes | FILE_ATTRIBUTE_HIDDEN
        } else {
            attributes & !FILE_ATTRIBUTE_HIDDEN
        };
        SetFileAttributesW(wide.as_ptr(), updated);
    }
}

#[cfg(not(windows))]
fn set_hidden(_path: &Path, _hidden: bool) {}

struct Staged {
    temp: PathBuf,
    from: PathBuf,
    to: PathBuf,
    was_locked: bool,
}

/// Rename files in two phases — everything to a hidden temporary name, then
/// to the final name — so sibling swaps (`a↔b`) and case-only renames
/// (`readme → README` on case-insensitive filesystems) can never collide.
/// Do not "optimize" this away for small batches (§8.3).
fn perform_moves(
    moves: &[(PathBuf, PathBuf)],
    is_cancelled: &dyn Fn() -> bool,
    on_progress: &mut dyn FnMut(usize),
) -> MoveOutcome {
    let mut staged: Vec<Staged> = Vec::with_capacity(moves.len());
    let mut errors: Vec<String> = Vec::new();

    // Phase 1 — stage everything to a hidden temp. Cancel here rolls the
    // staged files back, so nothing is renamed.
    for (from, to) in moves {
        if is_cancelled() {
            for stage in &staged {
                if move_no_clobber(&stage.temp, &stage.from).is_ok() && stage.was_locked {
                    lock_flag::set_locked(&stage.from, true);
                }
            }
            return MoveOutcome {
                succeeded: Vec::new(),
                errors,
                attempted: 0,
            };
        }
        let temp = temp_path_for(from);
        match move_no_clobber(from, &temp) {
            Ok(()) => {
                set_hidden(&temp, true);
                staged.push(Staged {
                    temp,
                    from: from.clone(),
                    to: to.clone(),
                    was_locked: false,
                });
            }
            Err(first_error) => {
                // A Locked (`uchg`) file fails rename with a misleading "you
                // don't have permission" error. Unlock, retry, and remember
                // to restore the lock on the file's final name.
                if lock_flag::is_locked(from) && lock_flag::set_locked(from, false) {
                    match move_no_clobber(from, &temp) {
                        Ok(()) => {
                            set_hidden(&temp, true);
                            staged.push(Staged {
                                temp,
                                from: from.clone(),
                                to: to.clone(),
                                was_locked: true,
                            });
                            continue;
                        }
                        Err(_) => {
                            lock_flag::set_locked(from, true);
                        }
                    }
                }
                errors.push(copy::couldnt_rename(
                    &name_of(from),
                    &first_error.to_string(),
                ));
            }
        }
    }

    // Phase 2 — commit temps to their final names. Cancel here keeps what's
    // already committed (recorded + revertible) and resolves the rest.
    let stride = (staged.len() / 100).max(1);
    let mut succeeded: Vec<(PathBuf, PathBuf)> = Vec::new();
    for index in 0..staged.len() {
        if is_cancelled() {
            // Roll the uncommitted temps back to their original names. If a
            // name is already occupied — an earlier committed move in a swap
            // (A→B done, B→A still pending) took it — rolling back would
            // strand this file in a hidden temp forever, so complete the move
            // to its target instead and record it, keeping disk and the
            // recorded snapshot in sync. Never strand a file at a temp name.
            for pending in &staged[index..] {
                if move_no_clobber(&pending.temp, &pending.from).is_ok() {
                    if pending.was_locked {
                        lock_flag::set_locked(&pending.from, true);
                    }
                    continue;
                }
                if move_no_clobber(&pending.temp, &pending.to).is_ok() {
                    if pending.was_locked {
                        lock_flag::set_locked(&pending.to, true);
                    }
                    set_hidden(&pending.to, false);
                    succeeded.push((pending.from.clone(), pending.to.clone()));
                } else {
                    errors.push(copy::couldnt_finish_stranded(
                        &name_of(&pending.from),
                        &name_of(&pending.temp),
                    ));
                }
            }
            break;
        }
        let stage = &staged[index];
        match move_no_clobber(&stage.temp, &stage.to) {
            Ok(()) => {
                set_hidden(&stage.to, false);
                if stage.was_locked {
                    lock_flag::set_locked(&stage.to, true);
                }
                succeeded.push((stage.from.clone(), stage.to.clone()));
                if succeeded.len().is_multiple_of(stride) || index == staged.len() - 1 {
                    on_progress(succeeded.len());
                }
            }
            Err(error) => {
                errors.push(copy::couldnt_rename_to(
                    &name_of(&stage.from),
                    &name_of(&stage.to),
                    &error.to_string(),
                ));
                // Roll this one temp back — guarded, so a swap partner's
                // committed file is never clobbered.
                if move_no_clobber(&stage.temp, &stage.from).is_ok() && stage.was_locked {
                    lock_flag::set_locked(&stage.from, true);
                }
            }
        }
    }
    let attempted = succeeded.len() + errors.len();
    MoveOutcome {
        succeeded,
        errors,
        attempted,
    }
}

/// Execute moves grouped by path depth (§8.2). `parents_first` renames
/// shallow paths first and prefix-rewrites deeper pending moves under the
/// new parent names (Apply); the opposite order undoes them (Revert) —
/// which is what makes recorded history entries always match the final
/// on-disk layout. Within one depth group the two-phase rename still makes
/// sibling swaps safe.
pub fn perform_moves_hierarchical(
    moves: &[(PathBuf, PathBuf)],
    parents_first: bool,
    is_cancelled: &dyn Fn() -> bool,
    on_progress: &mut dyn FnMut(usize),
) -> MoveOutcome {
    if moves.is_empty() {
        return MoveOutcome::default();
    }
    let mut grouped: BTreeMap<usize, Vec<(PathBuf, PathBuf)>> = BTreeMap::new();
    for (from, to) in moves {
        grouped
            .entry(from.components().count())
            .or_default()
            .push((from.clone(), to.clone()));
    }
    let depths: Vec<usize> = if parents_first {
        grouped.keys().copied().collect()
    } else {
        grouped.keys().rev().copied().collect()
    };

    let mut outcome = MoveOutcome::default();
    let mut rewrites: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut completed_base = 0usize;
    for depth in depths {
        if is_cancelled() {
            break;
        }
        let group: Vec<(PathBuf, PathBuf)> = grouped[&depth]
            .iter()
            .map(|(from, to)| {
                let mut from = from.clone();
                let mut to = to.clone();
                for (old, new) in &rewrites {
                    if let Some(rewritten) = rewrite_path_prefix(&from, old, new) {
                        from = rewritten;
                    }
                    if let Some(rewritten) = rewrite_path_prefix(&to, old, new) {
                        to = rewritten;
                    }
                }
                (from, to)
            })
            .collect();
        let base = completed_base;
        let result = perform_moves(&group, is_cancelled, &mut |done| on_progress(base + done));
        completed_base += result.succeeded.len();
        if parents_first {
            rewrites.extend(result.succeeded.iter().cloned());
        }
        outcome.succeeded.extend(result.succeeded);
        outcome.errors.extend(result.errors);
        outcome.attempted += result.attempted;
    }
    outcome
}

/// If `path` equals `old` or lives under it, return it re-based onto `new`.
/// Comparison is component-wise, so `/a/bc` is never under `/a/b`.
pub fn rewrite_path_prefix(path: &Path, old: &Path, new: &Path) -> Option<PathBuf> {
    let rest = path.strip_prefix(old).ok()?;
    // Joining an empty rest would append a trailing separator, producing a
    // path that later fails rename with ENOTDIR — exact matches return `new`
    // untouched.
    if rest.as_os_str().is_empty() {
        Some(new.to_path_buf())
    } else {
        Some(new.join(rest))
    }
}

/// Prefix-rewrite every tracked path after a batch of successful moves
/// (§8.5 steps 1–2): item paths, excluded paths, and watched-folder roots.
/// Returns the ids of watched folders whose root changed — the shell must
/// re-arm each one's watcher on the new path (same folder id).
///
/// There are deliberately NO "does the old path still exist" guards here —
/// batch ordering makes existence checks wrong (hard-learned; a CLAUDE.md
/// invariant). A file's path can never prefix another tracked path, so
/// unconditional rewriting is safe.
pub fn rewrite_live_paths(state: &mut CoreState, moves: &[(PathBuf, PathBuf)]) -> Vec<Uuid> {
    let mut rearmed: Vec<Uuid> = Vec::new();
    for (from, to) in moves {
        for item in &mut state.files {
            if let Some(rewritten) = rewrite_path_prefix(&item.path, from, to) {
                item.path = rewritten;
            }
        }
        state.excluded_paths = state
            .excluded_paths
            .drain()
            .map(|path| rewrite_path_prefix(&path, from, to).unwrap_or(path))
            .collect();
        for folder in &mut state.watched_folders {
            if let Some(rewritten) = rewrite_path_prefix(&folder.path, from, to) {
                folder.path = rewritten;
                if !rearmed.contains(&folder.id) {
                    rearmed.push(folder.id);
                }
            }
        }
    }
    rearmed
}

/// Restore files stranded by a rename batch interrupted between its two
/// phases (§8.4). A stranded temp is named `<TEMP_PREFIX><uuid>-<original>`;
/// if `<original>` is non-empty and free, move it back. Never clobbers an
/// existing file (an orphan whose original name is taken is left in place
/// for manual handling). Returns how many were recovered.
///
/// Callers must SKIP this entirely while a batch is processing — it would
/// grab the live batch's phase-1 temps (historical bug).
pub fn recover_orphaned_temp_files(directory: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(crate::platform::win_long_path(directory)) else {
        return 0;
    };
    let mut recovered = 0;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(rest) = name.strip_prefix(TEMP_PREFIX) else {
            continue;
        };
        // "<36-char-UUID>-<original>", with the UUID validated.
        if rest.len() <= 37 || rest.as_bytes().get(36) != Some(&b'-') {
            continue;
        }
        if Uuid::parse_str(&rest[..36]).is_err() {
            continue;
        }
        let original = &rest[37..];
        if original.is_empty() {
            continue;
        }
        let target = directory.join(original);
        if move_no_clobber(&entry.path(), &target).is_ok() {
            set_hidden(&target, false);
            recovered += 1;
        }
    }
    recovered
}
