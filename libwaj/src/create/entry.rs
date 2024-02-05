use crate::common::*;

pub struct Path1 {
    value_id: jbk::creator::ValueHandle,
    prefix: u8,
    size: u16,
}
static_assertions::assert_eq_size!(Path1, [u8; 24]);

impl Path1 {
    pub fn new(mut path: Vec<u8>, value_store: &jbk::creator::StoreHandle) -> Self {
        //        println!("Add path {path:?}");
        let size = path.len() as u16;
        let prefix = if size == 0 { 0 } else { path.remove(0) };
        let value_id = value_store.add_value(path);
        Self {
            prefix,
            size,
            value_id,
        }
    }
}

pub struct Content {
    mimetype: jbk::creator::ValueHandle,
    content_id: jbk::ContentIdx,
    pack_id: jbk::PackId,
}
static_assertions::assert_eq_size!(Content, [u8; 24]);

pub struct Entry {
    idx: jbk::Vow<jbk::EntryIdx>,
    // The three path_* are technically a Path1.
    // But extract the three fields from the Path1 allow compiler to
    // reorganise the fields and reduce the structure size.
    path_value_id: jbk::creator::ValueHandle,
    path_prefix: u8,
    path_size: u16,

    kind: EntryKind,
}

pub enum EntryKind {
    Content(Content),
    Redirect(Path1),
}
static_assertions::assert_eq_size!(Entry, [u8; 64]);

impl Entry {
    pub fn new_content(
        path: Path1,
        mimetype: jbk::creator::ValueHandle,
        content: jbk::ContentAddress,
    ) -> Self {
        Self {
            idx: jbk::Vow::new(0.into()),
            path_value_id: path.value_id,
            path_prefix: path.prefix,
            path_size: path.size,
            kind: EntryKind::Content(Content {
                mimetype,
                content_id: content.content_id,
                pack_id: content.pack_id,
            }),
        }
    }

    pub fn new_redirect(path: Path1, target: Path1) -> Self {
        Self {
            idx: jbk::Vow::new(0.into()),
            path_value_id: path.value_id,
            path_prefix: path.prefix,
            path_size: path.size,

            kind: EntryKind::Redirect(target),
        }
    }
}

impl jbk::creator::EntryTrait<Property, EntryType> for Entry {
    fn variant_name(&self) -> Option<jbk::MayRef<EntryType>> {
        Some(jbk::MayRef::Owned(match self.kind {
            EntryKind::Content(_) => EntryType::Content,
            EntryKind::Redirect(_) => EntryType::Redirect,
        }))
    }

    fn value_count(&self) -> jbk::PropertyCount {
        match self.kind {
            EntryKind::Content(_) => 3.into(),
            EntryKind::Redirect(_) => 2.into(),
        }
    }

    fn set_idx(&mut self, idx: jbk::EntryIdx) {
        self.idx.fulfil(idx)
    }

    fn get_idx(&self) -> jbk::Bound<jbk::EntryIdx> {
        self.idx.bind()
    }

    fn value(&self, name: &Property) -> jbk::MayRef<jbk::creator::Value> {
        jbk::MayRef::Owned(match name {
            Property::Path => jbk::creator::Value::Array1(Box::new(jbk::creator::ArrayS::<1> {
                data: [self.path_prefix],
                value_id: self.path_value_id.clone_get(),
                size: self.path_size as usize,
            })),
            Property::Mimetype => {
                if let EntryKind::Content(content) = &self.kind {
                    jbk::creator::Value::IndirectArray(Box::new(content.mimetype.clone_get()))
                } else {
                    unreachable!()
                }
            }
            Property::Content => {
                if let EntryKind::Content(content) = &self.kind {
                    jbk::creator::Value::Content(jbk::ContentAddress::new(
                        content.pack_id,
                        content.content_id,
                    ))
                } else {
                    unreachable!()
                }
            }
            Property::Target => {
                if let EntryKind::Redirect(target) = &self.kind {
                    jbk::creator::Value::Array1(Box::new(jbk::creator::ArrayS::<1> {
                        data: [target.prefix],
                        value_id: target.value_id.clone_get(),
                        size: target.size as usize,
                    }))
                } else {
                    unreachable!()
                }
            }
        })
    }
}

impl jbk::creator::FullEntryTrait<Property, EntryType> for Entry {
    fn compare<'i, I>(&self, _sort_keys: &'i I, other: &Self) -> std::cmp::Ordering
    where
        I: IntoIterator<Item = &'i Property> + Copy,
    {
        use std::cmp;
        //let mut iter = sort_keys.into_iter();
        //assert_eq!(iter.next(), Some(&Property::Path));
        //assert_eq!(iter.next(), None);
        match self.path_prefix.cmp(&other.path_prefix) {
            cmp::Ordering::Less => cmp::Ordering::Less,
            cmp::Ordering::Greater => cmp::Ordering::Greater,
            cmp::Ordering::Equal => {
                match self.path_value_id.get().cmp(&other.path_value_id.get()) {
                    cmp::Ordering::Less => cmp::Ordering::Less,
                    cmp::Ordering::Greater => cmp::Ordering::Greater,
                    cmp::Ordering::Equal => self.path_size.cmp(&other.path_size),
                }
            }
        }
    }
}
