#[cfg(test)]
mod scaffold {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Probe {
        ok: bool,
    }

    #[test]
    fn serde_round_trips() {
        let encoded = serde_json::to_string(&Probe { ok: true }).unwrap();
        assert_eq!(
            serde_json::from_str::<Probe>(&encoded).unwrap(),
            Probe { ok: true }
        );
    }
}
