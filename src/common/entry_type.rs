use jubako as jbk;

#[repr(u8)]
pub enum EntryType {
    Content = 0,
    Redirect = 1,
}

impl TryFrom<jbk::VariantIdx> for EntryType {
    type Error = String;
    fn try_from(id: jbk::VariantIdx) -> Result<Self, Self::Error> {
        match id.into_u8() {
            0 => Ok(Self::Content),
            1 => Ok(Self::Redirect),
            _ => Err("Invalid variant id".into()),
        }
    }
}

impl From<EntryType> for jbk::VariantIdx {
    fn from(t: EntryType) -> Self {
        (t as u8).into()
    }
}
