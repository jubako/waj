use super::entry::Entry;
use crate::common::{EntryType, Property};
use jbk::creator::{schema, EntryStore};

use super::{EntryKind, EntryTrait, Void};

pub struct EntryStoreCreator {
    schema: schema::Schema<Property, EntryType>,
    entry_store: Vec<Entry>,
    path_store: jbk::creator::StoreHandle,
    mime_store: jbk::creator::StoreHandle,
}

impl EntryStoreCreator {
    pub fn new(size_hint: Option<usize>) -> Self {
        let path_store = jbk::creator::ValueStore::new_plain(size_hint.map(|s| s * 2));
        let mime_store = jbk::creator::ValueStore::new_indexed();

        let schema = schema::Schema::new(
            // Common part
            schema::CommonProperties::new(vec![
                schema::Property::new_array(1, path_store.clone(), Property::Path), // the path
            ]),
            vec![
                // Content
                (
                    EntryType::Content,
                    schema::VariantProperties::new(vec![
                        schema::Property::new_array(0, mime_store.clone(), Property::Mimetype), // the mimetype
                        schema::Property::new_content_address(Property::Content),
                    ]),
                ),
                // Redirect
                (
                    EntryType::Redirect,
                    schema::VariantProperties::new(vec![
                        schema::Property::new_array(1, path_store.clone(), Property::Target), // Id of the linked entry
                    ]),
                ),
            ],
            Some(vec![Property::Path]),
        );

        let entry_store = Vec::new();

        Self {
            entry_store,
            schema,
            path_store,
            mime_store,
        }
    }

    pub fn add_entry<E>(&mut self, entry: &E) -> Void
    where
        E: EntryTrait,
    {
        let entry_kind = match entry.kind()? {
            Some(k) => k,
            None => {
                return Ok(());
            }
        };
        let path = entry.name().as_bytes().into();
        let entry = match entry_kind {
            EntryKind::Content(content_address, mimetype) => {
                Entry::new_content(path, mimetype.as_ref().as_bytes().into(), content_address)
            }
            EntryKind::Redirect(target) => {
                let target = target.as_bytes().into();
                Entry::new_redirect(path, target)
            }
        };
        self.entry_store.push(entry);
        Ok(())
    }
}

impl jbk::creator::EntryStoreCreatorTrait for EntryStoreCreator {
    fn finalize(mut self: Box<Self>, directory_pack: &mut jbk::creator::DirectoryPackCreator) {
        let entry_count = self.entry_store.len();
        directory_pack.add_value_store(self.path_store);
        directory_pack.add_value_store(self.mime_store);
        self.entry_store
            .sort_unstable_by(|a, b| a.path.cmp(&b.path));
        let jbk_entry_store = EntryStore::new(self.schema, self.entry_store.into_iter());
        let entry_store_id = directory_pack.add_entry_store(jbk_entry_store);
        directory_pack.create_index(
            "waj_entries",
            Default::default(),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            jbk::EntryCount::from(entry_count as u32),
            jbk::EntryIdx::from(0),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use jbk::creator::EntryStoreCreatorTrait;
    use mime_guess::mime;
    use rustest::{test, Result};

    #[test]
    fn test_empty() -> Result {
        let mut creator = jbk::creator::DirectoryPackCreator::new(
            jbk::PackId::from(0),
            crate::VENDOR_ID,
            Default::default(),
        );

        let entry_store_creator = Box::new(EntryStoreCreator::new(None));
        entry_store_creator.finalize(&mut creator);
        Ok(())
    }

    struct SimpleEntry(String);

    impl EntryTrait for SimpleEntry {
        fn name(&self) -> Cow<'_, str> {
            Cow::Borrowed(&self.0)
        }

        fn kind(&self) -> std::result::Result<Option<EntryKind>, CreatorError> {
            Ok(Some(EntryKind::Content(
                jbk::ContentAddress::new(1.into(), 0.into()),
                mime::APPLICATION_OCTET_STREAM,
            )))
        }
    }

    #[test]
    fn test_one_content(waj_file: rustest_fixtures::TempFile) -> Result {
        let waj_name = waj_file.path();
        let mut creator = jbk::creator::DirectoryPackCreator::new(
            jbk::PackId::from(0),
            crate::VENDOR_ID,
            Default::default(),
        );

        let mut entry_store_creator = Box::new(EntryStoreCreator::new(None));
        let entry = SimpleEntry("foo.txt".into());
        entry_store_creator.add_entry(&entry)?;
        entry_store_creator.finalize(&mut creator);
        {
            let mut waj_file = waj_file.reopen()?;
            creator.finalize()?.write(&mut waj_file)?;
        }
        assert!(waj_name.is_file());

        let directory_pack =
            jbk::reader::DirectoryPack::new(jbk::creator::FileSource::open(waj_name)?.into())?;
        let index = directory_pack.get_index_from_name("waj_entries")?;
        assert!(index.is_some());
        assert!(!index.unwrap().is_empty());
        Ok(())
    }
}
