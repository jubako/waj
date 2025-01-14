use super::entry::*;
use super::entry_type::EntryType;
use super::AllProperties;
use crate::error;
use jbk::reader::builder::PropertyBuilderTrait;
use jbk::reader::ByteSlice;

pub trait Builder {
    type Entry;

    fn new(properties: &AllProperties) -> Self;
    fn create_entry(&self, idx: jbk::EntryIdx, reader: &ByteSlice) -> jbk::Result<Self::Entry>;
}

impl Builder for () {
    type Entry = ();
    fn new(_properties: &AllProperties) -> Self {}
    fn create_entry(&self, _idx: jbk::EntryIdx, _reader: &ByteSlice) -> jbk::Result<Self::Entry> {
        Ok(())
    }
}

pub trait FullBuilderTrait {
    type Entry: EntryDef;

    fn new(properties: &AllProperties) -> Self;
    fn create_content(
        &self,
        idx: jbk::EntryIdx,
        reader: &ByteSlice,
    ) -> jbk::Result<<Self::Entry as EntryDef>::Content>;
    fn create_redirect(
        &self,
        idx: jbk::EntryIdx,
        reader: &ByteSlice,
    ) -> jbk::Result<<Self::Entry as EntryDef>::Redirect>;
}

impl<C, R> FullBuilderTrait for (C, R)
where
    C: Builder,
    R: Builder,
{
    type Entry = (C::Entry, R::Entry);

    fn new(properties: &AllProperties) -> Self {
        let content_builder = C::new(properties);
        let redirect_builder = R::new(properties);
        (content_builder, redirect_builder)
    }

    fn create_content(
        &self,
        idx: jbk::EntryIdx,
        reader: &ByteSlice,
    ) -> jbk::Result<<Self::Entry as EntryDef>::Content> {
        self.0.create_entry(idx, reader)
    }

    fn create_redirect(
        &self,
        idx: jbk::EntryIdx,
        reader: &ByteSlice,
    ) -> jbk::Result<<Self::Entry as EntryDef>::Redirect> {
        self.1.create_entry(idx, reader)
    }
}

pub(crate) struct RealBuilder<B: FullBuilderTrait> {
    store: jbk::reader::EntryStore,
    variant_id_property: jbk::reader::builder::VariantIdProperty,
    builder: B,
}

impl<B> RealBuilder<B>
where
    B: FullBuilderTrait,
{
    pub fn new(properties: &AllProperties) -> Self {
        let builder = B::new(properties);
        Self {
            store: properties.store.clone(),
            variant_id_property: properties.variant_id_property,
            builder,
        }
    }
}

impl<B> jbk::reader::builder::BuilderTrait for RealBuilder<B>
where
    B: FullBuilderTrait,
{
    type Entry = Entry<B::Entry>;
    type Error = error::BaseError;

    fn create_entry(&self, idx: jbk::EntryIdx) -> Result<Option<Self::Entry>, Self::Error> {
        self.store
            .get_entry_reader(idx)
            .map(|reader| {
                let entry_type = self.variant_id_property.create(&reader)?.try_into()?;
                Ok(match entry_type {
                    EntryType::Content => {
                        let entry = self.builder.create_content(idx, &reader)?;
                        Entry::Content(entry)
                    }
                    EntryType::Redirect => {
                        let entry = self.builder.create_redirect(idx, &reader)?;
                        Entry::Redirect(entry)
                    }
                })
            })
            .transpose()
    }
}
