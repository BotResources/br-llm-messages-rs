#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageMime {
    #[serde(rename = "image/jpeg")]
    Jpeg,
    #[serde(rename = "image/png")]
    Png,
    #[serde(rename = "image/gif")]
    Gif,
    #[serde(rename = "image/webp")]
    Webp,
}

impl ImageMime {
    pub fn as_str(self) -> &'static str {
        match self {
            ImageMime::Jpeg => "image/jpeg",
            ImageMime::Png => "image/png",
            ImageMime::Gif => "image/gif",
            ImageMime::Webp => "image/webp",
        }
    }
}

impl std::fmt::Display for ImageMime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_mime_when_serialized_then_uses_media_type_string() {
        assert_eq!(
            serde_json::to_string(&ImageMime::Png).unwrap(),
            "\"image/png\""
        );
        assert_eq!(
            serde_json::to_string(&ImageMime::Jpeg).unwrap(),
            "\"image/jpeg\""
        );
    }

    #[test]
    fn given_every_variant_when_round_tripped_then_identical() {
        for mime in [
            ImageMime::Jpeg,
            ImageMime::Png,
            ImageMime::Gif,
            ImageMime::Webp,
        ] {
            let json = serde_json::to_string(&mime).unwrap();
            assert_eq!(serde_json::from_str::<ImageMime>(&json).unwrap(), mime);
            assert_eq!(mime.to_string(), mime.as_str());
        }
    }

    #[test]
    fn given_unknown_media_type_when_deserialized_then_refused() {
        assert!(serde_json::from_str::<ImageMime>("\"image/svg+xml\"").is_err());
    }
}
