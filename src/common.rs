use jbk::reader::builder::PropertyBuilderTrait;
use jubako as jbk;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::rc::Rc;

pub enum Entry {
    Content(ContentEntry),
    Redirect(RedirectEntry),
}

pub struct ContentEntry {
    path: jbk::reader::Array,
    mimetype: jbk::reader::Array,
    content_address: jbk::reader::ContentAddress,
    resolver: jbk::reader::Resolver,
}

impl ContentEntry {
    pub fn get_mimetype(&self) -> jbk::Result<String> {
        let mut mimetype = Vec::with_capacity(125);
        self.resolver
            .resolve_array_to_vec(&self.mimetype, &mut mimetype)?;
        Ok(String::from_utf8(mimetype)?)
    }

    pub fn get_content_address(&self) -> jbk::reader::ContentAddress {
        self.content_address
    }
}

pub struct RedirectEntry {
    path: jbk::reader::Array,
    target: jbk::reader::Array,
    resolver: jbk::reader::Resolver,
}

impl RedirectEntry {
    pub fn get_target_link(&self) -> jbk::Result<String> {
        let mut path = Vec::with_capacity(125);
        self.resolver
            .resolve_array_to_vec(&self.target, &mut path)?;
        Ok(String::from_utf8(path)?)
    }
}

pub struct Builder {
    value_storage: Rc<jbk::reader::ValueStorage>,
    store: Rc<jbk::reader::EntryStore>,
    path_property: jbk::reader::builder::ArrayProperty,
    variant_id_property: jbk::reader::builder::Property<u8>,
    file_content_address_property: jbk::reader::builder::ContentProperty,
    file_mimetype_property: jbk::reader::builder::ArrayProperty,
    link_target_property: jbk::reader::builder::ArrayProperty,
}

impl jbk::reader::builder::BuilderTrait for Builder {
    type Entry = Entry;

    fn create_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<Self::Entry> {
        let resolver = jbk::reader::Resolver::new(Rc::clone(&self.value_storage));
        let reader = self.store.get_entry_reader(idx);
        let path = self.path_property.create(&reader)?;
        Ok(match self.variant_id_property.create(&reader)? {
            0 => {
                let content_address = self.file_content_address_property.create(&reader)?;
                let mimetype = self.file_mimetype_property.create(&reader)?;
                Entry::Content(ContentEntry {
                    path,
                    mimetype,
                    content_address,
                    resolver,
                })
            }
            1 => {
                let target = self.link_target_property.create(&reader)?;
                Entry::Redirect(RedirectEntry {
                    path,
                    target,
                    resolver,
                })
            }
            _ => unreachable!(),
        })
    }
}

pub struct Schema {
    value_storage: Rc<jbk::reader::ValueStorage>,
}

impl Schema {
    pub fn new(container: &jbk::reader::Container) -> Self {
        Self {
            value_storage: Rc::clone(container.get_value_storage()),
        }
    }
}

impl jbk::reader::schema::SchemaTrait for Schema {
    type Builder = Builder;
    fn create_builder(&self, store: Rc<jbk::reader::EntryStore>) -> jbk::Result<Self::Builder> {
        let layout = store.layout();
        let (variant_offset, variants) = layout.variant_part.as_ref().unwrap();
        assert_eq!(variants.len(), 2);
        let path_property = (&layout.common[0]).try_into()?;
        let variant_id_property = jbk::reader::builder::Property::new(*variant_offset);
        let file_mimetype_property = (&variants[0][0]).try_into()?;
        let file_content_address_property = (&variants[0][1]).try_into()?;
        let link_target_property = (&variants[1][0]).try_into()?;
        Ok(Builder {
            value_storage: Rc::clone(&self.value_storage),
            store,
            path_property,
            variant_id_property,
            file_mimetype_property,
            file_content_address_property,
            link_target_property,
        })
    }
}

pub struct EntryCompare<'resolver, 'builder> {
    resolver: &'resolver jbk::reader::Resolver,
    builder: &'builder Builder,
    path_value: Vec<u8>,
}

impl<'resolver, 'builder> EntryCompare<'resolver, 'builder> {
    pub fn new(
        resolver: &'resolver jbk::reader::Resolver,
        builder: &'builder Builder,
        component: &OsStr,
    ) -> Self {
        let path_value = component.to_os_string().into_vec();
        Self {
            resolver,
            builder,
            path_value,
        }
    }
}

impl jbk::reader::CompareTrait<Schema> for EntryCompare<'_, '_> {
    fn compare_entry(&self, idx: jbk::EntryIdx) -> jbk::Result<std::cmp::Ordering> {
        let reader = self.builder.store.get_entry_reader(idx);
        let entry_path = self.builder.path_property.create(&reader)?;
        self.resolver.compare_array(&entry_path, &self.path_value)
    }
}

pub struct Jim {
    container: jbk::reader::Container,
    pub schema: Schema,
}

impl std::ops::Deref for Jim {
    type Target = jbk::reader::Container;
    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

impl Jim {
    pub fn new<P: AsRef<Path>>(file: P) -> jbk::Result<Self> {
        let container = jbk::reader::Container::new(&file)?;
        let schema = Schema::new(&container);
        Ok(Self { container, schema })
    }
}
