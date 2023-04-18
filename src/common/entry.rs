pub trait EntryDef {
    type Content;
    type Redirect;
}

impl<C, R> EntryDef for (C, R) {
    type Content = C;
    type Redirect = R;
}

pub enum Entry<E: EntryDef> {
    Content(E::Content),
    Redirect(E::Redirect),
}
