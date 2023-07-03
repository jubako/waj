use super::common::*;
use super::Wpack;
use jbk::reader::Range;
use jubako as jbk;

pub trait Operator<Context, Builder: FullBuilderTrait> {
    fn on_start(&self, context: &mut Context) -> jbk::Result<()>;
    fn on_stop(&self, context: &mut Context) -> jbk::Result<()>;
    fn on_content(
        &self,
        context: &mut Context,
        entry: &<Builder::Entry as EntryDef>::Content,
    ) -> jbk::Result<()>;
    fn on_redirect(
        &self,
        context: &mut Context,
        entry: &<Builder::Entry as EntryDef>::Redirect,
    ) -> jbk::Result<()>;
}

pub struct Walker<'a, Context> {
    pack: &'a Wpack,
    context: Context,
}

impl<'a, Context> Walker<'a, Context> {
    pub fn new(pack: &'a Wpack, context: Context) -> Self {
        Self { pack, context }
    }

    pub fn run<B>(&mut self, op: &dyn Operator<Context, B>) -> jbk::Result<()>
    where
        B: FullBuilderTrait,
    {
        let builder = RealBuilder::<B>::new(&self.pack.properties);

        op.on_start(&mut self.context)?;
        self._run(&self.pack.root_index, &builder, op)?;
        op.on_stop(&mut self.context)
    }

    fn _run<R: Range, B>(
        &mut self,
        range: &R,
        builder: &RealBuilder<B>,
        op: &dyn Operator<Context, B>,
    ) -> jbk::Result<()>
    where
        B: FullBuilderTrait,
    {
        let read_entry = ReadEntry::new(range, builder);
        for entry in read_entry {
            match entry? {
                Entry::Content(e) => op.on_content(&mut self.context, &e)?,
                Entry::Redirect(e) => op.on_redirect(&mut self.context, &e)?,
            }
        }
        Ok(())
    }
}
