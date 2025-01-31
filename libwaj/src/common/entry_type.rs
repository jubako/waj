jbk::variants! {
    EntryType {
        Content => "content",
        Redirect => "redirect"
    }
}

impl From<EntryType> for jbk::VariantIdx {
    fn from(t: EntryType) -> Self {
        (t as u8).into()
    }
}
