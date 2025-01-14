use crate::error::WajFormatError;

#[repr(u8)]
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum EntryType {
    Content = 0,
    Redirect = 1,
}

impl TryFrom<jbk::VariantIdx> for EntryType {
    type Error = WajFormatError;
    fn try_from(id: jbk::VariantIdx) -> Result<Self, Self::Error> {
        match id.into_u8() {
            0 => Ok(Self::Content),
            1 => Ok(Self::Redirect),
            _ => Err(WajFormatError("Invalid variant id")),
        }
    }
}

impl ToString for EntryType {
    fn to_string(&self) -> String {
        String::from(match self {
            EntryType::Content => "content",
            EntryType::Redirect => "redirect",
        })
    }
}

impl jbk::creator::VariantName for EntryType {}

impl From<EntryType> for jbk::VariantIdx {
    fn from(t: EntryType) -> Self {
        (t as u8).into()
    }
}
