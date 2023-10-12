use crate::common::*;
use jubako::*;

pub struct Path1 {
    value_id: creator::ValueHandle,
    prefix: u8,
    size: u16,
}

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
    idx: Vow<EntryIdx>,
    // The three path_* are technically a Path1.
    // But extract the three fields from the Path1 allow compiler to
    // reorganise the fields and reduce the structure size.
    path_value_id: creator::ValueHandle,
    path_prefix: u8,
    path_size: u16,
    mimetype: creator::ValueHandle,
    content_id: ContentIdx,
    pack_id: PackId,
}
static_assertions::assert_eq_size!(Entry, [u8; 56]);

pub struct Redirect {
    idx: Vow<EntryIdx>,
    // The three path_* are technically a Path1.
    // But extract the three fields from the Path1 allow compiler to
    // reorganise the fields and reduce the structure size.
    path_value_id: creator::ValueHandle,
    path_prefix: u8,
    path_size: u16,
    // The three target_* are technically a Path1.
    // But extract the three fields from the Path1 allow compiler to
    // reorganise the fields and reduce the structure size.
    target_value_id: creator::ValueHandle,
    target_prefix: u8,
    target_size: u16,
}

static_assertions::assert_eq_size!(Redirect, [u8; 48]);

pub enum Entry {
    Content(Content),
    Redirect(Redirect),
}
static_assertions::assert_eq_size!(Entry, [u8; 56]);

impl Entry {
    pub fn new_content(
        path: Path1,
        mimetype: creator::ValueHandle,
        content: ContentAddress,
    ) -> Self {
        Self::Content(Content {
            idx: Vow::new(0.into()),
            path_value_id: path.value_id,
            path_prefix: path.prefix,
            path_size: path.size,
            mimetype,
            content_id: content.content_id,
            pack_id: content.pack_id,
        })
    }

    pub fn new_redirect(path: Path1, target: Path1) -> Self {
        Self::Redirect(Redirect {
            idx: Vow::new(0.into()),
            path_value_id: path.value_id,
            path_prefix: path.prefix,
            path_size: path.size,
            target_value_id: target.value_id,
            target_prefix: target.prefix,
            target_size: target.size,
        })
    }
}

impl jubako::creator::EntryTrait<Property, EntryType> for Entry {
    fn variant_name(&self) -> Option<jubako::MayRef<EntryType>> {
        Some(jubako::MayRef::Owned(match &self {
            Self::Content(_) => EntryType::Content,
            Self::Redirect(_) => EntryType::Redirect,
        }))
    }

    fn value_count(&self) -> PropertyCount {
        match &self {
            Self::Content(_) => 3.into(),
            Self::Redirect(_) => 2.into(),
        }
    }

    fn set_idx(&mut self, idx: EntryIdx) {
        match &self {
            Self::Content(content) => content.idx.fulfil(idx),
            Self::Redirect(redirect) => redirect.idx.fulfil(idx),
        }
    }

    fn get_idx(&self) -> Bound<EntryIdx> {
        match &self {
            Self::Content(content) => content.idx.bind(),
            Self::Redirect(redirect) => redirect.idx.bind(),
        }
    }

    fn value(&self, name: &Property) -> jubako::MayRef<creator::Value> {
        jubako::MayRef::Owned(match &self {
            Self::Content(content) => match name {
                Property::Path => creator::Value::Array1(Box::new(creator::ArrayS::<1> {
                    data: [content.path_prefix],
                    value_id: content.path_value_id.clone_get(),
                    size: content.path_size as usize,
                })),
                Property::Mimetype => {
                    creator::Value::IndirectArray(Box::new(content.mimetype.clone_get()))
                }
                Property::Content => creator::Value::Content(ContentAddress::new(
                    content.pack_id,
                    content.content_id,
                )),
                _ => unreachable!(),
            },
            Self::Redirect(redirect) => match name {
                Property::Path => creator::Value::Array1(Box::new(creator::ArrayS::<1> {
                    data: [redirect.path_prefix],
                    value_id: redirect.path_value_id.clone_get(),
                    size: redirect.path_size as usize,
                })),
                Property::Target => creator::Value::Array1(Box::new(creator::ArrayS::<1> {
                    data: [redirect.target_prefix],
                    value_id: redirect.target_value_id.clone_get(),
                    size: redirect.target_size as usize,
                })),
                _ => unreachable!(),
            },
        })
    }
}

impl jubako::creator::FullEntryTrait<Property, EntryType> for Entry {
    fn compare<'i, I>(&self, sort_keys: &'i I, other: &Self) -> std::cmp::Ordering
    where
        I: IntoIterator<Item = &'i Property> + Copy,
    {
        use std::cmp;
        let mut iter = sort_keys.into_iter();
        assert_eq!(iter.next(), Some(&Property::Path));
        assert_eq!(iter.next(), None);
        let (entry_prefix, entry_size, entry_value_id) = match self {
            Self::Content(e) => (&e.path_prefix, &e.path_size, &e.path_value_id),
            Self::Redirect(e) => (&e.path_prefix, &e.path_size, &e.path_value_id),
        };
        let (other_prefix, other_size, other_value_id) = match &other {
            Self::Content(e) => (&e.path_prefix, &e.path_size, &e.path_value_id),
            Self::Redirect(e) => (&e.path_prefix, &e.path_size, &e.path_value_id),
        };
        match entry_prefix.cmp(other_prefix) {
            cmp::Ordering::Less => cmp::Ordering::Less,
            cmp::Ordering::Greater => cmp::Ordering::Greater,
            cmp::Ordering::Equal => match entry_value_id.get().cmp(&other_value_id.get()) {
                cmp::Ordering::Less => cmp::Ordering::Less,
                cmp::Ordering::Greater => cmp::Ordering::Greater,
                cmp::Ordering::Equal => entry_size.cmp(other_size),
            },
        }
    }
}
