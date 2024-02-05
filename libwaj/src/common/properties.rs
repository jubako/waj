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

impl AllProperties {
    pub fn new(
        store: jbk::reader::EntryStore,
        value_storage: &jbk::reader::ValueStorage,
    ) -> jbk::Result<Self> {
        let layout = store.layout();
        let (variant_offset, variants, variants_map) = layout.variant_part.as_ref().unwrap();
        assert_eq!(variants.len(), 2);
        let path_property = (&layout.common["path"], value_storage).try_into()?;
        let variant_id_property = jbk::reader::builder::VariantIdProperty::new(*variant_offset);
        let content_mimetype_property = (
            &variants[variants_map["content"] as usize]["mimetype"],
            value_storage,
        )
            .try_into()?;
        let content_address_property =
            (&variants[variants_map["content"] as usize]["content"]).try_into()?;
        let redirect_target_property = (
            &variants[variants_map["redirect"] as usize]["target"],
            value_storage,
        )
            .try_into()?;
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
