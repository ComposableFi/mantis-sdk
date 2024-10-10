pub enum Credential<'a> {
    Keypair(&'a str),
    Mnemonic(&'a str),
}

impl<'a> Credential<'a> {
    pub fn from_options(keypair: &'a Option<String>, mnemonic: &'a Option<String>) -> Option<Self> {
        match (keypair, mnemonic) {
            (Some(keypair), None) => Some(Credential::Keypair(&keypair)),
            (None, Some(mnemonic)) => Some(Credential::Mnemonic(&mnemonic)),
            _ => None,
        }
    }
}
