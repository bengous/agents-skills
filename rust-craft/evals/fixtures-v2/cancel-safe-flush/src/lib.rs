use std::collections::VecDeque;
use std::future::Future;

pub trait Sink {
    type Error;
    type Write<'a>: Future<Output = Result<(), Self::Error>>
    where
        Self: 'a;

    fn write<'a>(&'a mut self, record: &'a [u8]) -> Self::Write<'a>;
}

pub struct BatchWriter<S> {
    sink: S,
    queued: VecDeque<Vec<u8>>,
}

impl<S> BatchWriter<S>
where
    S: Sink,
{
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            queued: VecDeque::new(),
        }
    }

    pub fn submit(&mut self, record: impl Into<Vec<u8>>) {
        self.queued.push_back(record.into());
    }

    pub fn pending(&self) -> usize {
        self.queued.len()
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    pub async fn flush(&mut self) -> Result<(), S::Error> {
        while let Some(record) = self.queued.front() {
            self.sink.write(record).await?;
            self.queued.pop_front();
        }

        Ok(())
    }
}
