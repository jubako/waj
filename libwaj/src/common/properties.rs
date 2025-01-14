use crate::error::{BaseError, WajFormatError};
use jbk::reader::VariantPart;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum Property {
    Path,
    Mimetype,
    Content,
    Target,
}

impl ToString for Property {
    fn to_string(&self) -> String {
        use Property::*;
        String::from(match self {
            Path => "path",
            Mimetype => "mimetype",
            Content => "content",
            Target => "target",
        })
    }
}

impl jbk::creator::PropertyName for Property {}

pub struct AllProperties {
    pub store: jbk::reader::EntryStore,
    pub path_property: jbk::reader::builder::ArrayProperty,
    pub variant_id_property: jbk::reader::builder::VariantIdProperty,
    pub content_mimetype_property: jbk::reader::builder::ArrayProperty,
    pub content_address_property: jbk::reader::builder::ContentProperty,
    pub redirect_target_property: jbk::reader::builder::ArrayProperty,
}

macro_rules! prop_as_builder {
    ($container:expr, $key: literal, $value_storage: expr, $kind:literal) => {
        $container
            .get($key)
            .ok_or(WajFormatError(concat!(
                "Property `",
                $key,
                "` is not present."
            )))?
            .as_builder($value_storage)?
            .ok_or(WajFormatError(concat!(
                "Property `",
                $key,
                "` is not a ",
                $kind,
                " proerty."
            )))?
    };
}

impl AllProperties {
    pub fn new(
        store: jbk::reader::EntryStore,
        value_storage: &jbk::reader::ValueStorage,
    ) -> Result<Self, BaseError> {
        let layout = store.layout();
        let VariantPart {
            variant_id_offset,
            variants,
            names,
        } = layout.variant_part.as_ref().unwrap();
        assert_eq!(variants.len(), 2);
        let path_property = prop_as_builder!(layout.common, "path", value_storage, "array");
        let variant_id_property = jbk::reader::builder::VariantIdProperty::new(*variant_id_offset);
        let content_mimetype_property = prop_as_builder!(
            variants[names["content"] as usize],
            "mimetype",
            value_storage,
            "array"
        );
        let content_address_property = prop_as_builder!(
            variants[names["content"] as usize],
            "content",
            value_storage,
            "content"
        );
        let redirect_target_property = prop_as_builder!(
            variants[names["redirect"] as usize],
            "target",
            value_storage,
            "arry"
        );
        Ok(Self {
            store,
            path_property,
            variant_id_property,
            content_mimetype_property,
            content_address_property,
            redirect_target_property,
        })
    }
}
