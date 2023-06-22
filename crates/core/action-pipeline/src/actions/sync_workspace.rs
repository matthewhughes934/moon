use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_actions::{sync_codeowners, sync_vcs_hooks};
use moon_logger::debug;
use moon_project_graph::ProjectGraph;
use moon_workspace::Workspace;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:sync-workspace";

pub async fn sync_workspace(
    _action: &mut Action,
    _context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    project_graph: Arc<RwLock<ProjectGraph>>,
) -> miette::Result<ActionStatus> {
    env::set_var("MOON_RUNNING_ACTION", "sync-workspace");

    let workspace = workspace.read().await;
    let project_graph = project_graph.read().await;

    debug!(target: LOG_TARGET, "Syncing workspace");

    if workspace.config.codeowners.sync_on_run {
        debug!(target: LOG_TARGET, "Syncing codeowners (syncOnRun enabled)");

        sync_codeowners(&workspace, &project_graph).await?;
    }

    if workspace.config.vcs.sync_hooks {
        debug!(
            target: LOG_TARGET,
            "Syncing {} hooks (syncHooks enabled)", workspace.config.vcs.manager
        );

        sync_vcs_hooks(&workspace).await?;
    }

    Ok(ActionStatus::Passed)
}
