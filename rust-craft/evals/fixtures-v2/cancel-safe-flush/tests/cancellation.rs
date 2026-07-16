use cancel_safe_flush::{BatchWriter, Sink};
use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

#[derive(Debug, PartialEq, Eq)]
enum SinkError {
    Rejected,
}

#[derive(Default)]
struct CommitThenYield {
    committed: Vec<Vec<u8>>,
    reject_next: bool,
}

struct Write<'a> {
    sink: &'a mut CommitThenYield,
    record: &'a [u8],
    committed: bool,
}

impl Future for Write<'_> {
    type Output = Result<(), SinkError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.sink.reject_next {
            self.sink.reject_next = false;
            return Poll::Ready(Err(SinkError::Rejected));
        }

        if !self.committed {
            let record = self.record.to_vec();
            self.sink.committed.push(record);
            self.committed = true;
            context.waker().wake_by_ref();
            return Poll::Pending;
        }

        Poll::Ready(Ok(()))
    }
}

impl Sink for CommitThenYield {
    type Error = SinkError;
    type Write<'a> = Write<'a>;

    fn write<'a>(&'a mut self, record: &'a [u8]) -> Self::Write<'a> {
        Write {
            sink: self,
            record,
            committed: false,
        }
    }
}

struct Noop;

impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

fn waker() -> Waker {
    Waker::from(Arc::new(Noop))
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let waker = waker();
    let mut context = Context::from_waker(&waker);

    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
    }
}

#[test]
fn cancellation_never_reoffers_a_committed_record() {
    let mut writer = BatchWriter::new(CommitThenYield::default());
    writer.submit(b"first".to_vec());
    writer.submit(b"second".to_vec());

    {
        let mut flush = pin!(writer.flush());
        let waker = waker();
        let mut context = Context::from_waker(&waker);
        assert_eq!(flush.as_mut().poll(&mut context), Poll::Pending);
    }

    assert_eq!(writer.sink().committed, [b"first".to_vec()]);
    assert_eq!(writer.pending(), 1);

    assert_eq!(block_on(writer.flush()), Ok(()));
    assert_eq!(
        writer.sink().committed,
        [b"first".to_vec(), b"second".to_vec()]
    );
    assert_eq!(writer.pending(), 0);
}

#[test]
fn rejected_record_remains_first_for_retry() {
    let sink = CommitThenYield {
        reject_next: true,
        ..CommitThenYield::default()
    };
    let mut writer = BatchWriter::new(sink);
    writer.submit(b"retry".to_vec());

    assert_eq!(block_on(writer.flush()), Err(SinkError::Rejected));
    assert_eq!(writer.pending(), 1);
    assert!(writer.sink().committed.is_empty());

    assert_eq!(block_on(writer.flush()), Ok(()));
    assert_eq!(writer.sink().committed, [b"retry".to_vec()]);
}
