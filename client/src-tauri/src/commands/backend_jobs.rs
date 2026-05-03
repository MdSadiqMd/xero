use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use crate::commands::{CommandError, CommandResult};

#[derive(Debug, Clone, Default)]
pub struct BackendJobRegistry {
    latest_jobs: Arc<Mutex<HashMap<String, LatestJobState>>>,
    operation_lanes: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    next_generation: Arc<AtomicU64>,
}

#[derive(Debug, Clone)]
struct LatestJobState {
    generation: u64,
    cancellation: BackendCancellationToken,
}

#[derive(Debug)]
pub(crate) struct BackendLatestJob {
    key: String,
    label: &'static str,
    generation: u64,
    cancellation: BackendCancellationToken,
}

#[derive(Debug, Clone, Default)]
pub struct BackendCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl BackendCancellationToken {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn check_cancelled(&self, label: &'static str) -> CommandResult<()> {
        if self.is_cancelled() {
            return Err(cancelled_error(label));
        }

        Ok(())
    }
}

impl BackendJobRegistry {
    pub(crate) fn start_latest(
        &self,
        key: impl Into<String>,
        label: &'static str,
    ) -> BackendLatestJob {
        let key = key.into();
        let generation = self.next_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let cancellation = BackendCancellationToken::default();

        let mut latest_jobs = self
            .latest_jobs
            .lock()
            .expect("backend latest job registry lock poisoned");
        if let Some(previous) = latest_jobs.insert(
            key.clone(),
            LatestJobState {
                generation,
                cancellation: cancellation.clone(),
            },
        ) {
            previous.cancellation.cancel();
        }

        BackendLatestJob {
            key,
            label,
            generation,
            cancellation,
        }
    }

    pub(crate) fn is_latest(&self, job: &BackendLatestJob) -> bool {
        let latest_jobs = self
            .latest_jobs
            .lock()
            .expect("backend latest job registry lock poisoned");
        latest_jobs
            .get(&job.key)
            .map(|state| state.generation == job.generation)
            .unwrap_or(false)
    }

    pub(crate) fn finish_latest(&self, job: &BackendLatestJob) {
        let mut latest_jobs = self
            .latest_jobs
            .lock()
            .expect("backend latest job registry lock poisoned");
        if latest_jobs
            .get(&job.key)
            .map(|state| state.generation == job.generation)
            .unwrap_or(false)
        {
            latest_jobs.remove(&job.key);
        }
    }

    pub fn cancel_latest(&self, key: &str) {
        let mut latest_jobs = self
            .latest_jobs
            .lock()
            .expect("backend latest job registry lock poisoned");
        if let Some(previous) = latest_jobs.remove(key) {
            previous.cancellation.cancel();
        }
    }

    pub async fn run_blocking_latest<T, F>(
        &self,
        key: impl Into<String>,
        label: &'static str,
        work: F,
    ) -> CommandResult<T>
    where
        T: Send + 'static,
        F: FnOnce(BackendCancellationToken) -> CommandResult<T> + Send + 'static,
    {
        let job = self.start_latest(key, label);
        let cancellation = job.cancellation.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            cancellation.check_cancelled(label)?;
            work(cancellation)
        })
        .await
        .map_err(|error| join_error(label, error))?;

        let latest = self.is_latest(&job);
        self.finish_latest(&job);

        if !latest {
            return Err(stale_error(job.label));
        }

        result
    }

    pub async fn run_blocking_project_lane<T, F>(
        &self,
        project_id: impl Into<String>,
        lane: &'static str,
        label: &'static str,
        work: F,
    ) -> CommandResult<T>
    where
        T: Send + 'static,
        F: FnOnce() -> CommandResult<T> + Send + 'static,
    {
        let lane_lock = self.project_lane(project_id.into(), lane);
        tauri::async_runtime::spawn_blocking(move || {
            let _guard = lane_lock.lock().map_err(|_| {
                CommandError::system_fault(
                    "backend_operation_lane_poisoned",
                    format!("Xero could not enter the {label} operation lane."),
                )
            })?;
            work()
        })
        .await
        .map_err(|error| join_error(label, error))?
    }

    fn project_lane(&self, project_id: String, lane: &'static str) -> Arc<Mutex<()>> {
        let key = format!("{project_id}\u{0}{lane}");
        let mut operation_lanes = self
            .operation_lanes
            .lock()
            .expect("backend operation lane registry lock poisoned");
        operation_lanes
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}

fn cancelled_error(label: &'static str) -> CommandError {
    CommandError::retryable(
        "backend_job_cancelled",
        format!("Xero cancelled stale {label} work because a newer request replaced it."),
    )
}

fn stale_error(label: &'static str) -> CommandError {
    CommandError::retryable(
        "backend_job_stale_result",
        format!("Xero ignored stale {label} work because a newer request completed first."),
    )
}

fn join_error(label: &'static str, error: impl std::fmt::Display) -> CommandError {
    CommandError::system_fault(
        "backend_job_failed",
        format!("Xero could not finish background {label} work: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{mpsc, Arc, Mutex},
        time::Duration,
    };

    use super::BackendJobRegistry;

    #[test]
    fn starting_latest_job_cancels_the_previous_job_for_the_same_key() {
        let registry = BackendJobRegistry::default();

        let first = registry.start_latest("project-search", "project search");
        assert!(!first.cancellation.is_cancelled());
        assert!(registry.is_latest(&first));

        let second = registry.start_latest("project-search", "project search");
        assert!(first.cancellation.is_cancelled());
        assert!(!registry.is_latest(&first));
        assert!(registry.is_latest(&second));
    }

    #[test]
    fn finishing_a_replaced_latest_job_does_not_remove_the_current_job() {
        let registry = BackendJobRegistry::default();

        let first = registry.start_latest("repository-diff", "repository diff");
        let second = registry.start_latest("repository-diff", "repository diff");

        registry.finish_latest(&first);

        assert!(registry.is_latest(&second));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn project_operation_lanes_serialize_work_per_project_and_lane() {
        let registry = BackendJobRegistry::default();
        let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let (started_tx, started_rx) = mpsc::channel::<()>();
        let (release_tx, release_rx) = mpsc::channel::<()>();

        let first_order = Arc::clone(&order);
        let first_registry = registry.clone();
        let first = tokio::spawn(async move {
            first_registry
                .run_blocking_project_lane("project-1", "git", "git mutation", move || {
                    first_order.lock().expect("order lock").push("first-start");
                    started_tx.send(()).expect("started");
                    release_rx.recv().expect("release");
                    first_order.lock().expect("order lock").push("first-end");
                    Ok(())
                })
                .await
        });

        started_rx.recv().expect("first started");

        let second_order = Arc::clone(&order);
        let second = tokio::spawn(async move {
            registry
                .run_blocking_project_lane("project-1", "git", "git mutation", move || {
                    second_order
                        .lock()
                        .expect("order lock")
                        .push("second-start");
                    second_order.lock().expect("order lock").push("second-end");
                    Ok(())
                })
                .await
        });

        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(*order.lock().expect("order lock"), vec!["first-start"]);

        release_tx.send(()).expect("release first");
        first.await.expect("first join").expect("first result");
        second.await.expect("second join").expect("second result");

        assert_eq!(
            *order.lock().expect("order lock"),
            vec!["first-start", "first-end", "second-start", "second-end"]
        );
    }
}
