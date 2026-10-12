use crate::value::{Base64Data, ImageMime, Text};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UserBlock {
    Text { text: Text },
    Image(Image),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Image {
    pub mime: ImageMime,
    pub data: Base64Data,
}

impl UserBlock {
    pub fn text(text: Text) -> Self {
        UserBlock::Text { text }
    }

    pub fn image(mime: ImageMime, data: Base64Data) -> Self {
        UserBlock::Image(Image { mime, data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_text_block_when_serialized_then_type_tagged() {
        let block = UserBlock::text(Text::new("hi").unwrap());
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json, serde_json::json!({ "type": "text", "text": "hi" }));
        assert_eq!(serde_json::from_value::<UserBlock>(json).unwrap(), block);
    }

    #[test]
    fn given_image_block_when_serialized_then_fields_flattened() {
        let block = UserBlock::image(ImageMime::Png, Base64Data::new("aGVsbG8=").unwrap());
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "image", "mime": "image/png", "data": "aGVsbG8=" })
        );
        assert_eq!(serde_json::from_value::<UserBlock>(json).unwrap(), block);
    }

    #[test]
    fn given_blank_text_in_json_when_deserialized_then_refused() {
        let json = serde_json::json!({ "type": "text", "text": "  " });
        assert!(serde_json::from_value::<UserBlock>(json).is_err());
    }

    #[test]
    fn given_unknown_type_tag_when_deserialized_then_refused() {
        let json = serde_json::json!({ "type": "nonsense", "text": "x" });
        assert!(serde_json::from_value::<UserBlock>(json).is_err());
    }
}
