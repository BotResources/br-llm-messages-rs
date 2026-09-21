nonempty_string_newtype!(Signature, "signature");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MessageError;

    #[test]
    fn given_empty_when_new_then_blank() {
        assert!(matches!(
            Signature::new(""),
            Err(MessageError::Blank { field: "signature" })
        ));
    }

    #[test]
    fn given_padded_opaque_value_when_new_then_stored_byte_for_byte() {
        let raw = "  Ab+/=\n";
        let sig = Signature::new(raw).unwrap();
        assert_eq!(sig.as_str(), raw);
    }

    #[test]
    fn given_signature_when_round_tripped_then_identical() {
        let sig = Signature::new("EqoBCkYIB...==").unwrap();
        let json = serde_json::to_string(&sig).unwrap();
        assert_eq!(serde_json::from_str::<Signature>(&json).unwrap(), sig);
    }
}
