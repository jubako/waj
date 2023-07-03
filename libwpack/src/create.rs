use jubako as jbk;

use crate::common::{EntryType, Property};
use jbk::creator::schema;
use std::collections::{hash_map::Entry as MapEntry, HashMap};
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use mime_guess::Mime;

const VENDOR_ID: u32 = 0x6a_69_6d_00;

type EntryStore = jbk::creator::EntryStore<
    Property,
    EntryType,
    Box<jbk::creator::BasicEntry<Property, EntryType>>,
>;

pub enum ConcatMode {
    OneFile,
    TwoFiles,
    NoConcat,
}

pub enum EntryKind {
    Content(jbk::Reader, Mime),
    Redirect(OsString),
}

pub trait EntryTrait {
    /// The kind of the entry
    fn kind(&self) -> jbk::Result<EntryKind>;

    /// Under which name the entry will be stored
    fn name(&self) -> &OsStr;
}

impl<T> EntryTrait for Box<T>
where
    T: EntryTrait + ?Sized,
{
    fn kind(&self) -> jbk::Result<EntryKind> {
        self.as_ref().kind()
    }
    fn name(&self) -> &OsStr {
        self.as_ref().name()
    }
}

type EntryIdx = jbk::Bound<jbk::EntryIdx>;
type Void = jbk::Result<()>;

pub struct CachedContentPack {
    content_pack: jbk::creator::ContentPackCreator,
    cache: HashMap<blake3::Hash, jbk::ContentIdx>,
    progress: Rc<dyn Progress>,
}

impl CachedContentPack {
    fn new(content_pack: jbk::creator::ContentPackCreator, progress: Rc<dyn Progress>) -> Self {
        Self {
            content_pack,
            cache: Default::default(),
            progress,
        }
    }

    fn add_content(&mut self, content: jbk::Reader) -> jbk::Result<jbk::ContentIdx> {
        let mut hasher = blake3::Hasher::new();
        std::io::copy(&mut content.create_flux_all(), &mut hasher)?;
        let hash = hasher.finalize();
        match self.cache.entry(hash) {
            MapEntry::Vacant(e) => {
                let content_idx = self.content_pack.add_content(content)?;
                e.insert(content_idx);
                Ok(content_idx)
            }
            MapEntry::Occupied(e) => {
                self.progress.cached_data(content.size());
                Ok(*e.get())
            }
        }
    }

    fn into_inner(self) -> jbk::creator::ContentPackCreator {
        self.content_pack
    }
}

pub trait Progress {
    fn cached_data(&self, _size: jbk::Size) {}
}

impl Progress for () {}

pub struct Creator {
    content_pack: CachedContentPack,
    directory_pack: jbk::creator::DirectoryPackCreator,
    entry_store: Box<EntryStore>,
    main_entry_path: PathBuf,
    main_entry_id: Option<EntryIdx>,
    concat_mode: ConcatMode,
    tmp_path_content_pack: tempfile::TempPath,
    tmp_path_directory_pack: tempfile::TempPath,
}

impl Creator {
    pub fn new<P: AsRef<Path>>(
        outfile: P,
        main_entry: PathBuf,
        concat_mode: ConcatMode,
        jbk_progress: Arc<dyn jbk::creator::Progress>,
        progress: Rc<dyn Progress>,
    ) -> jbk::Result<Self> {
        let outfile = outfile.as_ref();
        let out_dir = outfile.parent().unwrap();

        let (tmp_content_pack, tmp_path_content_pack) =
            tempfile::NamedTempFile::new_in(out_dir)?.into_parts();
        let content_pack = jbk::creator::ContentPackCreator::new_from_file_with_progress(
            tmp_content_pack,
            jbk::PackId::from(1),
            VENDOR_ID,
            jbk::FreeData40::clone_from_slice(&[0x00; 40]),
            jbk::CompressionType::Zstd,
            jbk_progress,
        )?;

        let (_, tmp_path_directory_pack) = tempfile::NamedTempFile::new_in(out_dir)?.into_parts();
        let mut directory_pack = jbk::creator::DirectoryPackCreator::new(
            &tmp_path_directory_pack,
            jbk::PackId::from(0),
            VENDOR_ID,
            jbk::FreeData31::clone_from_slice(&[0x00; 31]),
        );

        let path_store = directory_pack.create_value_store(jbk::creator::ValueStoreKind::Plain);
        let mime_store = directory_pack.create_value_store(jbk::creator::ValueStoreKind::Indexed);

        let schema = schema::Schema::new(
            // Common part
            schema::CommonProperties::new(vec![
                schema::Property::new_array(1, Rc::clone(&path_store), Property::Path), // the path
            ]),
            vec![
                // Content
                (
                    EntryType::Content,
                    schema::VariantProperties::new(vec![
                        schema::Property::new_array(0, Rc::clone(&mime_store), Property::Mimetype), // the mimetype
                        schema::Property::new_content_address(Property::Content),
                    ]),
                ),
                // Redirect
                (
                    EntryType::Redirect,
                    schema::VariantProperties::new(vec![
                        schema::Property::new_array(0, Rc::clone(&path_store), Property::Target), // Id of the linked entry
                    ]),
                ),
            ],
            Some(vec![Property::Path]),
        );

        let entry_store = Box::new(EntryStore::new(schema));

        Ok(Self {
            content_pack: CachedContentPack::new(content_pack, progress),
            directory_pack,
            entry_store,
            main_entry_path: main_entry,
            main_entry_id: Default::default(),
            concat_mode,
            tmp_path_content_pack,
            tmp_path_directory_pack,
        })
    }

    pub fn finalize(mut self, outfile: &Path) -> Void {
        let entry_count = self.entry_store.len();
        let entry_store_id = self.directory_pack.add_entry_store(self.entry_store);
        self.directory_pack.create_index(
            "wpack_entries",
            jubako::ContentAddress::new(0.into(), 0.into()),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            jbk::EntryCount::from(entry_count as u32),
            jubako::EntryIdx::from(0).into(),
        );
        self.directory_pack.create_index(
            "wpack_main",
            jubako::ContentAddress::new(0.into(), 0.into()),
            jbk::PropertyIdx::from(0),
            entry_store_id,
            jubako::EntryCount::from(1),
            self.main_entry_id.unwrap().into(),
        );

        let directory_pack_info = match self.concat_mode {
            ConcatMode::NoConcat => {
                let mut outfilename = outfile.file_name().unwrap().to_os_string();
                outfilename.push(".wpackd");
                let mut directory_pack_path = PathBuf::new();
                directory_pack_path.push(outfile);
                directory_pack_path.set_file_name(outfilename);
                let directory_pack_info = self
                    .directory_pack
                    .finalize(Some(directory_pack_path.clone()))?;
                if let Err(e) = self.tmp_path_directory_pack.persist(&directory_pack_path) {
                    return Err(e.error.into());
                };
                directory_pack_info
            }
            _ => self.directory_pack.finalize(None)?,
        };

        let content_pack_info = match self.concat_mode {
            ConcatMode::OneFile => self.content_pack.into_inner().finalize(None)?,
            _ => {
                let mut outfilename = outfile.file_name().unwrap().to_os_string();
                outfilename.push(".wpackc");
                let mut content_pack_path = PathBuf::new();
                content_pack_path.push(outfile);
                content_pack_path.set_file_name(outfilename);
                let content_pack_info = self
                    .content_pack
                    .into_inner()
                    .finalize(Some(content_pack_path.clone()))?;
                if let Err(e) = self.tmp_path_content_pack.persist(&content_pack_path) {
                    return Err(e.error.into());
                }
                content_pack_info
            }
        };

        let mut manifest_creator = jbk::creator::ManifestPackCreator::new(
            outfile,
            VENDOR_ID,
            jbk::FreeData63::clone_from_slice(&[0x00; 63]),
        );

        manifest_creator.add_pack(directory_pack_info);
        manifest_creator.add_pack(content_pack_info);
        manifest_creator.finalize()?;
        Ok(())
    }

    pub fn add_entry<E>(&mut self, entry: E) -> Void
    where
        E: EntryTrait,
    {
        let mut values = HashMap::from([(
            Property::Path,
            jbk::Value::Array(entry.name().to_os_string().into_vec()),
        )]);
        let is_main_entry = entry.name() == self.main_entry_path;
        match entry.kind()? {
            EntryKind::Content(reader, mimetype) => {
                let content_id = self.content_pack.add_content(reader)?;
                values.insert(
                    Property::Content,
                    jbk::Value::Content(jbk::ContentAddress::new(jbk::PackId::from(1), content_id)),
                );
                values.insert(
                    Property::Mimetype,
                    jbk::Value::Array(mimetype.to_string().into())
                );
                let entry = Box::new(jbk::creator::BasicEntry::new_from_schema(
                    &self.entry_store.schema,
                    Some(EntryType::Content),
                    values,
                ));
                let current_idx = self.entry_store.add_entry(entry);
                if is_main_entry {
                    self.main_entry_id = Some(current_idx);
                }
                Ok(())
            }
            EntryKind::Redirect(target) => {
                values.insert(Property::Target, jbk::Value::Array(target.into_vec()));
                let entry = Box::new(jbk::creator::BasicEntry::new_from_schema(
                    &self.entry_store.schema,
                    Some(EntryType::Redirect),
                    values,
                ));
                let current_idx = self.entry_store.add_entry(entry);
                if is_main_entry {
                    self.main_entry_id = Some(current_idx);
                }
                Ok(())
            }
        }
    }
}
