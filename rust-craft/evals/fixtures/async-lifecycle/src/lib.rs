use std::{
    future::Future,
    pin::Pin,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Running,
    Stopped,
}

#[derive(Default)]
pub struct Runtime;

pub struct Task {
    future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
}

impl Runtime {
    pub fn spawn<F>(&self, future: F) -> Task
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Task {
            future: Box::pin(future),
        }
    }

    pub fn join(&self, mut task: Task) {
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);

        while task.future.as_mut().poll(&mut context).is_pending() {}
    }
}

pub struct Service {
    state: Mutex<State>,
    stop_requested: AtomicBool,
}

struct State {
    phase: Phase,
    completed_operations: usize,
}

impl Service {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                phase: Phase::Idle,
                completed_operations: 0,
            }),
            stop_requested: AtomicBool::new(false),
        }
    }

    pub fn start(&self, runtime: &Runtime) -> Task {
        self.state.lock().unwrap().phase = Phase::Running;
        runtime.spawn(self.run())
    }

    pub fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::Release);
    }

    pub fn phase(&self) -> Phase {
        self.state.lock().unwrap().phase
    }

    pub fn completed_operations(&self) -> usize {
        self.state.lock().unwrap().completed_operations
    }

    async fn run(&self) {
        let mut state = self.state.lock().unwrap();
        StopRequested {
            requested: &self.stop_requested,
        }
        .await;
        state.phase = Phase::Stopped;
        state.completed_operations += 1;
    }
}

struct StopRequested<'a> {
    requested: &'a AtomicBool,
}

impl Future for StopRequested<'_> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
        if self.requested.load(Ordering::Acquire) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
