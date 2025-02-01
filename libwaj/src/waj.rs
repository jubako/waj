use crate::error::{BaseError, WajError, WajFormatError};

use super::common::{AllProperties, Builder, Comparator, Entry, FullBuilderTrait, RealBuilder};
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::{ByteSlice, Range};
use std::path::Path;

pub struct Waj {
    container: jbk::reader::Container,
    pub(crate) root_index: jbk::reader::Index,
    pub(crate) properties: AllProperties,
}

impl std::ops::Deref for Waj {
    type Target = jbk::reader::Container;
    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

fn create_properties(
    container: &jbk::reader::Container,
    index: &jbk::reader::Index,
) -> Result<AllProperties, BaseError> {
    AllProperties::new(
        index.get_store(container.get_entry_storage())?,
        container.get_value_storage(),
    )
}

struct PathBuilder {
    path_property: jbk::reader::builder::ArrayProperty,
}

impl Builder for PathBuilder {
    type Entry = jbk::SmallBytes;

    fn new(properties: &AllProperties) -> Self {
        Self {
            path_property: properties.path_property.clone(),
        }
    }

    fn create_entry(&self, _idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        let path_prop = self.path_property.create(reader)?;
        let mut path = jbk::SmallBytes::new();
        path_prop.resolve_to_vec(&mut path)?;
        Ok(path)
    }
}

impl Waj {
    pub fn new<P: AsRef<Path>>(file: P) -> Result<Self, WajError> {
        let container = jbk::reader::Container::new(&file)?;
        let root_index = container
            .get_directory_pack()
            .get_index_from_name("waj_entries")?
            .ok_or(WajFormatError("No `waj_entries` in the archive"))?;
        let properties = create_properties(&container, &root_index)?;

        Ok(Self {
            container,
            root_index,
            properties,
        })
    }

    pub fn create_properties(
        &self,
        index: &jbk::reader::Index,
    ) -> Result<AllProperties, BaseError> {
        create_properties(&self.container, index)
    }

    pub fn get_entry<B>(&self, path: &str) -> Result<Entry<B::Entry>, WajError>
    where
        B: FullBuilderTrait,
    {
        let comparator = Comparator::new(&self.properties);
        let builder = RealBuilder::<B>::new(&self.properties);
        let current_range: jbk::EntryRange = (&self.root_index).into();
        let comparator = comparator.compare_with(path.as_bytes());
        match current_range.find(&comparator)? {
            None => Err(WajError::PathNotFound(format!("Cannot found entry {path}"))),
            Some(idx) => {
                let entry = current_range
                    .get_entry(&builder, idx)?
                    .expect("idx should be valid");
                Ok(entry)
            }
        }
    }
}
