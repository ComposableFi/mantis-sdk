pub trait Credential: Sized {
    type Error;

    fn from_base58_file(filename: &str) -> Result<Self, Self::Error>;
    fn from_mnemonic(mnemonic: &str) -> Result<Self, Self::Error>;
    fn error_from_str(s: &str) -> Self::Error;
}

pub fn from_options<C: Credential>(
    keypair: &Option<String>,
    mnemonic: &Option<String>,
) -> Result<C, C::Error> {
    match (keypair, mnemonic) {
        (Some(keypair), None) => C::from_base58_file(keypair),
        (None, Some(mnemonic)) => C::from_mnemonic(mnemonic),
        _ => Err(C::error_from_str(
            "Either keypair or mnemonic must be provided",
        )),
    }
}
