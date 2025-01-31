use crate::{
    common::EntryType,
    error::{BaseError, WajFormatError},
};

use jbk::{layout_builder, properties};

properties! {
    Property {
        Path:"array" => "path",
        Mimetype:"array" => "mimetype",
        Content:"content" => "content",
        Target:"array" => "target"
    }
}

pub struct AllProperties {
    pub store: jbk::reader::EntryStore,
    pub path_property: jbk::reader::builder::ArrayProperty,
    pub variant_id_property: jbk::reader::builder::VariantIdBuilder<EntryType>,
    pub content_mimetype_property: jbk::reader::builder::ArrayProperty,
    pub content_address_property: jbk::reader::builder::ContentProperty,
    pub redirect_target_property: jbk::reader::builder::ArrayProperty,
}

impl AllProperties {
    pub fn new(
        store: jbk::reader::EntryStore,
        value_storage: &jbk::reader::ValueStorage,
    ) -> Result<Self, BaseError> {
        let layout = store.layout();
        if layout.variant_len() != 2 {
            return Err(WajFormatError("Layout must contain 3 variants").into());
        }
        let path_property = layout_builder!(
            layout[common][Property::Path],
            value_storage,
            WajFormatError
        );
        let variant_id_property = layout.variant_id_builder().expect("We have variants");
        let content_mimetype_property = layout_builder!(
            layout[EntryType::Content][Property::Mimetype],
            value_storage,
            WajFormatError
        );
        let content_address_property = layout_builder!(
            layout[EntryType::Content][Property::Content],
            value_storage,
            WajFormatError
        );
        let redirect_target_property = layout_builder!(
            layout[EntryType::Redirect][Property::Target],
            value_storage,
            WajFormatError
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
