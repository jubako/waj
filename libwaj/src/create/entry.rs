use crate::common::*;
use jubako::*;

pub struct Path1 {
    value_id: creator::ValueHandle,
    prefix: u8,
    size: u16,
}
static_assertions::assert_eq_size!(Path1, [u8; 24]);

impl Path1 {
    pub fn new(mut path: Vec<u8>, value_store: &creator::StoreHandle) -> Self {
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
    mimetype: creator::ValueHandle,
    content_id: ContentIdx,
    pack_id: PackId,
}
static_assertions::assert_eq_size!(Content, [u8; 24]);

pub struct Entry {
    idx: Vow<EntryIdx>,
    // The three path_* are technically a Path1.
    // But extract the three fields from the Path1 allow compiler to
    // reorganise the fields and reduce the structure size.
    path_value_id: creator::ValueHandle,
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
        mimetype: creator::ValueHandle,
        content: ContentAddress,
    ) -> Self {
        Self {
            idx: Vow::new(0.into()),
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
            idx: Vow::new(0.into()),
            path_value_id: path.value_id,
            path_prefix: path.prefix,
            path_size: path.size,

            kind: EntryKind::Redirect(target),
        }
    }
}

impl jubako::creator::EntryTrait<Property, EntryType> for Entry {
    fn variant_name(&self) -> Option<jubako::MayRef<EntryType>> {
        Some(jubako::MayRef::Owned(match self.kind {
            EntryKind::Content(_) => EntryType::Content,
            EntryKind::Redirect(_) => EntryType::Redirect,
        }))
    }

    fn value_count(&self) -> PropertyCount {
        match self.kind {
            EntryKind::Content(_) => 3.into(),
            EntryKind::Redirect(_) => 2.into(),
        }
    }

    fn set_idx(&mut self, idx: EntryIdx) {
        self.idx.fulfil(idx)
    }

    fn get_idx(&self) -> Bound<EntryIdx> {
        self.idx.bind()
    }

    fn value(&self, name: &Property) -> jubako::MayRef<creator::Value> {
        jubako::MayRef::Owned(match name {
            Property::Path => creator::Value::Array1(Box::new(creator::ArrayS::<1> {
                data: [self.path_prefix],
                value_id: self.path_value_id.clone_get(),
                size: self.path_size as usize,
            })),
            Property::Mimetype => {
                if let EntryKind::Content(content) = &self.kind {
                    creator::Value::IndirectArray(Box::new(content.mimetype.clone_get()))
                } else {
                    unreachable!()
                }
            }
            Property::Content => {
                if let EntryKind::Content(content) = &self.kind {
                    creator::Value::Content(ContentAddress::new(
                        content.pack_id,
                        content.content_id,
                    ))
                } else {
                    unreachable!()
                }
            }
            Property::Target => {
                if let EntryKind::Redirect(target) = &self.kind {
                    creator::Value::Array1(Box::new(creator::ArrayS::<1> {
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

impl jubako::creator::FullEntryTrait<Property, EntryType> for Entry {
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
