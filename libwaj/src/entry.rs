use crate::common::{AllProperties, Builder};
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::ByteSlice;

pub struct CommonPart {
    idx: jbk::EntryIdx,
    path: jbk::SmallBytes,
}

pub trait CommonEntry {
    fn common(&self) -> &CommonPart;
    fn idx(&self) -> jbk::EntryIdx {
        self.common().idx
    }
    fn path(&self) -> &[u8] {
        &self.common().path
    }
}

pub struct Content {
    common: CommonPart,
    mimetype: jbk::SmallBytes,
    content: jbk::ContentAddress,
}

impl CommonEntry for Content {
    fn common(&self) -> &CommonPart {
        &self.common
    }
}

impl Content {
    pub fn content(&self) -> jbk::ContentAddress {
        self.content
    }

    pub fn mimetype(&self) -> &[u8] {
        &self.mimetype
    }
}

pub struct Redirect {
    common: CommonPart,
    target: jbk::SmallBytes,
}

impl CommonEntry for Redirect {
    fn common(&self) -> &CommonPart {
        &self.common
    }
}

impl Redirect {
    pub fn target(&self) -> &[u8] {
        &self.target
    }
}

mod private {
    use super::*;
    pub struct CommonBuilder {
        path_property: jbk::reader::builder::ArrayProperty,
    }

    impl CommonBuilder {
        fn new(properties: &AllProperties) -> Self {
            Self {
                path_property: properties.path_property.clone(),
            }
        }

        fn create_entry(&self, idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<CommonPart> {
            let path_prop = self.path_property.create(reader)?;
            let mut path = jbk::SmallBytes::new();
            path_prop.resolve_to_vec(&mut path)?;
            Ok(CommonPart { idx, path })
        }
    }

    pub struct ContentBuilder {
        common: CommonBuilder,
        mimetype_property: jbk::reader::builder::ArrayProperty,
        content_address_property: jbk::reader::builder::ContentProperty,
    }

    impl Builder for ContentBuilder {
        type Entry = Content;

        fn new(properties: &AllProperties) -> Self {
            Self {
                common: CommonBuilder::new(properties),
                mimetype_property: properties.content_mimetype_property.clone(),
                content_address_property: properties.content_address_property,
            }
        }

        fn create_entry(&self, idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
            let mimetype_prop = self.mimetype_property.create(reader)?;
            let mut mimetype = jbk::SmallBytes::new();
            mimetype_prop.resolve_to_vec(&mut mimetype)?;
            Ok(Content {
                common: self.common.create_entry(idx, reader)?,
                mimetype,
                content: self.content_address_property.create(reader)?,
            })
        }
    }

    pub struct RedirectBuilder {
        common: CommonBuilder,
        link_property: jbk::reader::builder::ArrayProperty,
    }

    impl Builder for RedirectBuilder {
        type Entry = Redirect;

        fn new(properties: &AllProperties) -> Self {
            Self {
                common: CommonBuilder::new(properties),
                link_property: properties.redirect_target_property.clone(),
            }
        }

        fn create_entry(&self, idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
            let common = self.common.create_entry(idx, reader)?;
            let target_prop = self.link_property.create(reader)?;
            let mut target = jbk::SmallBytes::new();
            target_prop.resolve_to_vec(&mut target)?;
            Ok(Redirect { common, target })
        }
    }
} // private mode

pub type FullBuilder = (private::ContentBuilder, private::RedirectBuilder);

pub type FullEntry = super::Entry<(Content, Redirect)>;
