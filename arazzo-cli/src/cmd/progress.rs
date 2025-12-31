use arazzo_exec::executor::{Event, EventSink};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct ProgressEventSink {
    total_steps: usize,
    completed: Arc<AtomicUsize>,
    failed: Arc<AtomicUsize>,
    running: Arc<AtomicUsize>,
}

impl ProgressEventSink {
    pub fn new(total_steps: usize) -> Self {
        Self {
            total_steps,
            completed: Arc::new(AtomicUsize::new(0)),
            failed: Arc::new(AtomicUsize::new(0)),
            running: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn update_progress(&self) {
        let completed = self.completed.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let running = self.running.load(Ordering::Relaxed);
        let total = self.total_steps;
        let done = completed + failed;
        let percent = if total > 0 { (done * 100) / total } else { 0 };
        eprint!(
            "\rProgress: [{}/{}] {}% (✓{} ✗{} →{})",
            done, total, percent, completed, failed, running
        );
        if done == total {
            eprintln!();
        }
    }
}

#[async_trait]
impl EventSink for ProgressEventSink {
    async fn emit(&self, event: Event) {
        match event {
            Event::StepStarted { .. } => {
                self.running.fetch_add(1, Ordering::Relaxed);
                self.update_progress();
            }
            Event::StepSucceeded { .. } => {
                self.completed.fetch_add(1, Ordering::Relaxed);
                self.running
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                        if v > 0 {
                            Some(v - 1)
                        } else {
                            Some(0)
                        }
                    })
                    .ok();
                self.update_progress();
            }
            Event::StepFailed { .. } => {
                self.failed.fetch_add(1, Ordering::Relaxed);
                self.running
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                        if v > 0 {
                            Some(v - 1)
                        } else {
                            Some(0)
                        }
                    })
                    .ok();
                self.update_progress();
            }
            _ => {}
        }
    }
}

pub struct CompositeProgressSink {
    progress: Arc<ProgressEventSink>,
    base: Arc<dyn EventSink>,
}

impl CompositeProgressSink {
    pub fn new(progress: Arc<ProgressEventSink>, base: Arc<dyn EventSink>) -> Self {
        Self { progress, base }
    }
}

#[async_trait]
impl EventSink for CompositeProgressSink {
    async fn emit(&self, event: Event) {
        self.progress.emit(event.clone()).await;
        self.base.emit(event).await;
    }
}
