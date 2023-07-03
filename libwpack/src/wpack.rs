use super::common::{AllProperties, Comparator, Entry, FullBuilderTrait, RealBuilder};
use jubako as jbk;
use jubako::reader::Range;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub use jbk::SubReader as Reader;

pub struct Wpack {
    container: jbk::reader::Container,
    pub(crate) root_index: jbk::reader::Index,
    pub(crate) properties: AllProperties,
}

impl std::ops::Deref for Wpack {
    type Target = jbk::reader::Container;
    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

fn create_properties(
    container: &jbk::reader::Container,
    index: &jbk::reader::Index,
) -> jbk::Result<AllProperties> {
    AllProperties::new(
        index.get_store(container.get_entry_storage())?,
        container.get_value_storage(),
    )
}

impl Wpack {
    pub fn new<P: AsRef<Path>>(file: P) -> jbk::Result<Self> {
        let container = jbk::reader::Container::new(&file)?;
        let root_index = container
            .get_directory_pack()
            .get_index_from_name("wpack_entries")?;
        let properties = create_properties(&container, &root_index)?;
        Ok(Self {
            container,
            root_index,
            properties,
        })
    }

    pub fn create_properties(&self, index: &jbk::reader::Index) -> jbk::Result<AllProperties> {
        create_properties(&self.container, index)
    }

    pub fn get_entry<B, P>(&self, path: P) -> jbk::Result<Entry<B::Entry>>
    where
        P: AsRef<Path>,
        B: FullBuilderTrait,
    {
        let comparator = Comparator::new(&self.properties);
        let builder = RealBuilder::<B>::new(&self.properties);
        let current_range: jbk::EntryRange = (&self.root_index).into();
        let comparator = comparator.compare_with(path.as_ref().as_os_str().as_bytes());
        match current_range.find(&comparator)? {
            None => Err("Cannot found entry".to_string().into()),
            Some(idx) => {
                let entry = current_range.get_entry(&builder, idx)?;
                Ok(entry)
            }
        }
    }
}
