mod private {
    pub(super) trait Sealed {}
    impl<T> Sealed for Result<T, crate::bin_deserialize::BinError> {}
    impl<T> Sealed for Result<T, crate::text_deserialize::TextError> {}
}

#[allow(private_bounds)]
pub trait Context: private::Sealed {
    fn context<S: AsRef<str>>(self, context: S) -> Self;
    fn with_context<S: AsRef<str>, F: Fn() -> S>(self, context: F) -> Self;
}
impl<T> Context for Result<T, crate::bin_deserialize::BinError> {
    fn context<S: AsRef<str>>(self, context: S) -> Self {
        return match self {
            Ok(value) => Ok(value),
            Err(err) => Err(err.context(context)),
        };
    }
    fn with_context<S: AsRef<str>, F: Fn() -> S>(self, context: F) -> Self {
        return match self {
            Ok(value) => Ok(value),
            Err(err) => Err(err.context(context())),
        };
    }
}
impl<T> Context for Result<T, crate::text_deserialize::TextError> {
    fn context<S: AsRef<str>>(self, context: S) -> Self {
        return match self {
            Ok(value) => Ok(value),
            Err(err) => Err(err.context(context)),
        };
    }
    fn with_context<S: AsRef<str>, F: Fn() -> S>(self, context: F) -> Self {
        return match self {
            Ok(value) => Ok(value),
            Err(err) => Err(err.context(context())),
        };
    }
}
