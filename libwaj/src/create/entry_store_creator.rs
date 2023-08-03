use jubako as jbk;

use crate::common::{EntryType, Property};
use jbk::creator::schema;
use std::collections::HashMap;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use super::{EntryKind, EntryTrait, Void};

type EntryStore = jbk::creator::EntryStore<
    Property,
    EntryType,
    Box<jbk::creator::BasicEntry<Property, EntryType>>,
>;

type EntryIdx = jbk::Bound<jbk::EntryIdx>;

pub struct EntryStoreCreator {
    entry_store: Box<EntryStore>,
    path_store: jbk::creator::StoreHandle,
    mime_store: jbk::creator::StoreHandle,
    main_entry_path: PathBuf,
    main_entry_id: Option<EntryIdx>,
}

impl EntryStoreCreator {
    pub fn new(main_entry: PathBuf) -> Self {
        let path_store = jbk::creator::ValueStore::new_plain(None);
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
                        schema::Property::new_array(0, path_store.clone(), Property::Target), // Id of the linked entry
                    ]),
                ),
            ],
            Some(vec![Property::Path]),
        );

        let entry_store = Box::new(EntryStore::new(schema, None));

        Self {
            entry_store,
            path_store,
            mime_store,
            main_entry_path: main_entry,
            main_entry_id: Default::default(),
        }
    }

    pub fn finalize(self, directory_pack: &mut jbk::creator::DirectoryPackCreator) -> Void {
        let main_entry_id = match self.main_entry_id {
            Some(id) => id,
            None => {
                return Err(format!(
                    "No entry found for the main entry ({})",
                    self.main_entry_path.display()
                )
                .into())
            }
        };
        let entry_count = self.entry_store.len();
        directory_pack.add_value_store(self.path_store);
        directory_pack.add_value_store(self.mime_store);
        let entry_store_id = directory_pack.add_entry_store(self.entry_store);
        directory_pack.create_index(
            "waj_entries",
            Default::default(),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            jbk::EntryCount::from(entry_count as u32),
            jubako::EntryIdx::from(0).into(),
        );
        directory_pack.create_index(
            "waj_main",
            Default::default(),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            jubako::EntryCount::from(1),
            main_entry_id.into(),
        );
        Ok(())
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
        let mut values = HashMap::from([(
            Property::Path,
            jbk::Value::Array(entry.name().to_os_string().into_vec()),
        )]);
        let is_main_entry = entry.name() == self.main_entry_path;
        let entry = Box::new(match entry_kind {
            EntryKind::Content(content_address, mimetype) => {
                values.insert(Property::Content, jbk::Value::Content(content_address));
                values.insert(
                    Property::Mimetype,
                    jbk::Value::Array(mimetype.to_string().into()),
                );
                jbk::creator::BasicEntry::new_from_schema(
                    &self.entry_store.schema,
                    Some(EntryType::Content),
                    values,
                )
            }
            EntryKind::Redirect(target) => {
                values.insert(Property::Target, jbk::Value::Array(target.into_vec()));
                jbk::creator::BasicEntry::new_from_schema(
                    &self.entry_store.schema,
                    Some(EntryType::Redirect),
                    values,
                )
            }
        });
        let entry_idx = self.entry_store.add_entry(entry);
        if is_main_entry {
            self.main_entry_id = Some(entry_idx);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use jubako as jbk;
    use mime_guess::mime;

    #[test]
    fn test_empty() -> jbk::Result<()> {
        let mut creator =
            jbk::creator::DirectoryPackCreator::new(jbk::PackId::from(0), 0, Default::default());

        let entry_store_creator = EntryStoreCreator::new("".into());
        assert!(entry_store_creator.finalize(&mut creator).is_err());
        Ok(())
    }

    struct SimpleEntry(OsString);

    impl EntryTrait for SimpleEntry {
        fn name(&self) -> &OsStr {
            &self.0
        }

        fn kind(&self) -> jbk::Result<Option<EntryKind>> {
            Ok(Some(EntryKind::Content(
                jbk::ContentAddress::new(1.into(), 0.into()),
                mime::APPLICATION_OCTET_STREAM,
            )))
        }
    }

    #[test]
    fn test_one_content() -> jbk::Result<()> {
        let waj_file = tempfile::NamedTempFile::new_in(&std::env::temp_dir())?;
        let (mut waj_file, waj_name) = waj_file.into_parts();
        let mut creator =
            jbk::creator::DirectoryPackCreator::new(jbk::PackId::from(0), 0, Default::default());

        let mut entry_store_creator = EntryStoreCreator::new("foo.txt".into());
        let entry = SimpleEntry("foo.txt".into());
        entry_store_creator.add_entry(&entry)?;
        entry_store_creator.finalize(&mut creator)?;
        creator.finalize(&mut waj_file)?;
        assert!(waj_name.is_file());

        let directory_pack =
            jbk::reader::DirectoryPack::new(jbk::creator::FileSource::open(waj_name)?.into())?;
        let index = directory_pack.get_index_from_name("waj_entries")?;
        assert!(!index.is_empty());
        Ok(())
    }
}
