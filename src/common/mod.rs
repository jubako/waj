mod builder;
mod entry;
mod entry_type;
mod properties;

pub(crate) use builder::RealBuilder;
pub use builder::{Builder, FullBuilderTrait};
pub use entry::{Entry, EntryDef};
pub use entry_type::EntryType;
use jbk::reader::builder::PropertyBuilderTrait;
pub use jbk::SubReader as Reader;
use jubako as jbk;
pub use properties::{AllProperties, Property};

pub struct Comparator {
    store: jbk::reader::EntryStore,
    path_property: jbk::reader::builder::ArrayProperty,
}

impl Comparator {
    pub fn new(properties: &AllProperties) -> Self {
        Self {
            store: properties.store.clone(),
            path_property: properties.path_property.clone(),
        }
    }

    pub fn compare_with<'a>(&'a self, component: &'a [u8]) -> EntryCompare {
        EntryCompare {
            comparator: self,
            path_value: component,
        }
    }
}

pub struct EntryCompare<'a> {
    comparator: &'a Comparator,
    path_value: &'a [u8],
}

impl jbk::reader::CompareTrait for EntryCompare<'_> {
    fn compare_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<std::cmp::Ordering> {
        let reader = self.comparator.store.get_entry_reader(idx);
        //let mut path = vec![];
        let entry_path = self.comparator.path_property.create(&reader)?;
        //        entry_path.resolve_to_vec(&mut path)?;
        //        println!("Compare {:?}\n   with {:?}", path, self.path_value);
        //        println!("Compare {:?}\n   with {:?}", String::from_utf8(path), String::from_utf8(self.path_value.to_vec()));
        match entry_path.partial_cmp(self.path_value) {
            Some(c) => Ok(c),
            None => Err("Cannot compare".into()),
        }
    }
    fn ordered(&self) -> bool {
        true
    }
}
