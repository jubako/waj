use internment::Intern;
use jbk::{SmallBytes, Value};

use crate::common::*;

#[derive(Debug)]
pub struct Content {
    mimetype: Intern<SmallBytes>,
    content: jbk::ContentAddress,
}
static_assertions::assert_eq_size!(Content, [u8; 16]);

#[derive(Debug)]
pub struct Entry {
    // The three path_* are technically a Path1.
    // But extract the three fields from the Path1 allow compiler to
    // reorganise the fields and reduce the structure size.
    pub(crate) path: SmallBytes,
    kind: EntryKind,
}

#[derive(Debug)]
pub enum EntryKind {
    Content(Content),
    Redirect(SmallBytes),
}
static_assertions::assert_eq_size!(Entry, [u8; 56]);

impl Entry {
    pub fn new_content(
        path: SmallBytes,
        mimetype: SmallBytes,
        content: jbk::ContentAddress,
    ) -> Self {
        Self {
            path,
            kind: EntryKind::Content(Content {
                mimetype: Intern::new(mimetype),
                content,
            }),
        }
    }

    pub fn new_redirect(path: SmallBytes, target: SmallBytes) -> Self {
        Self {
            path,
            kind: EntryKind::Redirect(target),
        }
    }
}

impl jbk::creator::EntryTrait<Property, EntryType> for Entry {
    fn variant_name(&self) -> Option<EntryType> {
        Some(match self.kind {
            EntryKind::Content(_) => EntryType::Content,
            EntryKind::Redirect(_) => EntryType::Redirect,
        })
    }

    fn value_count(&self) -> jbk::PropertyCount {
        match self.kind {
            EntryKind::Content(_) => 3.into(),
            EntryKind::Redirect(_) => 2.into(),
        }
    }

    fn value(&self, name: &Property) -> Value {
        match name {
            Property::Path => Value::Array(self.path.clone()),
            Property::Mimetype => {
                if let EntryKind::Content(content) = &self.kind {
                    Value::Array((*content.mimetype).clone())
                } else {
                    unreachable!()
                }
            }
            Property::Content => {
                if let EntryKind::Content(content) = &self.kind {
                    Value::Content(content.content)
                } else {
                    unreachable!()
                }
            }
            Property::Target => {
                if let EntryKind::Redirect(target) = &self.kind {
                    Value::Array(target.clone())
                } else {
                    unreachable!()
                }
            }
        }
    }
}
