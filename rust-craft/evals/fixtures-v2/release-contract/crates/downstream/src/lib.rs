use wire_core::Encoder;

#[derive(Default)]
pub struct Identity;

impl Encoder for Identity {
    fn encode(&mut self, input: &[u8]) -> Vec<u8> {
        input.to_vec()
    }
}

pub fn configured() -> Box<dyn Encoder> {
    Box::new(Identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_implementor_and_trait_object_still_work() {
        let mut encoder = configured();
        assert_eq!(encoder.encode(b"data"), b"data");
        assert_eq!(encoder.flush(), Ok(()));
    }
}
