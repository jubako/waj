use jubako as jbk;
use std::rc::Rc;

pub struct AllProperties {
    pub store: Rc<jbk::reader::EntryStore>,
    pub path_property: jbk::reader::builder::ArrayProperty,
    pub variant_id_property: jbk::reader::builder::VariantIdProperty,
    pub content_mimetype_property: jbk::reader::builder::ArrayProperty,
    pub content_address_property: jbk::reader::builder::ContentProperty,
    pub redirect_target_property: jbk::reader::builder::ArrayProperty,
}

impl AllProperties {
    pub fn new(
        store: Rc<jbk::reader::EntryStore>,
        value_storage: &jbk::reader::ValueStorage,
    ) -> jbk::Result<Self> {
        let layout = store.layout();
        let (variant_offset, variants) = layout.variant_part.as_ref().unwrap();
        assert_eq!(variants.len(), 2);
        let path_property = (&layout.common[0], value_storage).try_into()?;
        let variant_id_property = jbk::reader::builder::VariantIdProperty::new(*variant_offset);
        let content_mimetype_property = (&variants[0][0], value_storage).try_into()?;
        let content_address_property = (&variants[0][1]).try_into()?;
        let redirect_target_property = (&variants[1][0], value_storage).try_into()?;
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
