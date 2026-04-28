//! Phase 4 cleanup: only `load_provider_profiles_snapshot` remains here. The
//! Tauri commands `list_provider_profiles`, `upsert_provider_profile`,
//! `set_active_provider_profile`, and `logout_provider_profile` were deleted
//! when the credentials-driven UX shipped (see PROVIDER_REFACTOR_PLAN.md).
//!
//! The snapshot loader is kept because internal callers — `auth/store`,
//! `provider_models`, `runtime/provider`, `runtime/diagnostics`,
//! `commands/doctor_report`, `commands/provider_diagnostics`, and
//! `commands/get_runtime_settings` — still read the legacy snapshot to
//! compose runtime bindings. Those will migrate off the snapshot in a later
//! wave alongside the deletion of the underlying `provider_profiles` SQLite
//! tables.

use tauri::{AppHandle, Runtime};

use crate::{
    commands::CommandResult, global_db::open_global_database,
    provider_profiles::load_provider_profiles_or_default, provider_profiles::ProviderProfilesSnapshot,
    state::DesktopState,
};

pub(crate) fn load_provider_profiles_snapshot<R: Runtime>(
    app: &AppHandle<R>,
    state: &DesktopState,
) -> CommandResult<ProviderProfilesSnapshot> {
    let connection = open_global_database(&state.global_db_path(app)?)?;
    load_provider_profiles_or_default(&connection)
}
