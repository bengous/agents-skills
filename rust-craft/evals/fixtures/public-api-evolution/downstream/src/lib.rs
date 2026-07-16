use store_api::{Store, StoreError};

#[derive(Default)]
pub struct DownstreamStore {
    writes: Vec<(String, Vec<u8>)>,
}

impl Store for DownstreamStore {
    fn write(&mut self, key: &str, value: &[u8]) -> Result<(), StoreError> {
        self.writes.push((key.to_owned(), value.to_owned()));
        Ok(())
    }
}

pub fn existing_consumer() -> Result<Box<dyn Store>, StoreError> {
    let mut store: Box<dyn Store> = Box::new(DownstreamStore::default());
    store.write("key", b"value")?;
    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_downstream_implementation_and_trait_object_still_work() {
        assert!(existing_consumer().is_ok());
    }
}
