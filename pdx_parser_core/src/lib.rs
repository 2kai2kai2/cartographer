// pub mod bin_tokenizer;
pub mod bin_deserialize;
pub mod bin_lexer;
pub mod common_deserialize;
mod err_context;
pub mod eu4_date;
pub mod eu4_save_parser;
pub mod eu5;
pub mod helpers;
pub mod modern_header;
pub mod raw_parser;
pub mod stellaris_date;
pub mod stellaris_save_parser;
mod strings_resolver;
pub mod text_deserialize;
pub mod text_lexer;

pub use bin_deserialize::{BinDeserialize, BinDeserializer};
pub use err_context::Context;
pub use eu4_date::{EU4Date, Month};
pub use pdx_parser_macros::{BinDeserialize, TextDeserialize, eu5_token};
pub use stellaris_date::StellarisDate;
pub use strings_resolver::StringsResolver;
pub use text_deserialize::{TextDeserialize, TextDeserializer};

#[cfg(test)]
mod tests {
    use crate::{
        StringsResolver,
        bin_deserialize::{BinDeserialize, BinDeserializer},
        text_deserialize::{TextDeserialize, TextDeserializer},
    };

    use super::*;

    #[test]
    fn thingy_text() {
        #[derive(BinDeserialize, TextDeserialize)]
        struct Thingy {
            asdf: u32,
            true_false_maybe: Option<bool>,
        }

        let input1 = b"\x03\x00\
        \x17\x00\x04\x00asdf\x01\x00\x14\x00\x37\x13\x00\x00\
        \x04\x00";
        let (thingy, _rest) = Thingy::take(BinDeserializer::from_bytes(
            input1,
            &StringsResolver::default(),
        ))
        .unwrap();
        assert_eq!(thingy.asdf, 0x1337);
        assert_eq!(thingy.true_false_maybe, None);

        let input2 = b"\x03\x00\
        \x17\x00\x04\x00asdf\x01\x00\x14\x00\x37\x13\x00\x00\
        \x17\x00\x10\x00true_false_maybe\x01\x00\x0e\x00\x01\
        \x04\x00";
        let (thingy, _rest) = Thingy::take(BinDeserializer::from_bytes(
            input2,
            &StringsResolver::default(),
        ))
        .unwrap();
        assert_eq!(thingy.asdf, 0x1337);
        assert_eq!(thingy.true_false_maybe, Some(true));

        let input3 = format!("{{ asdf = {} }}", 0x1337);
        let (thingy, _rest) = Thingy::take_text(TextDeserializer::from_str(&input3)).unwrap();
        assert_eq!(thingy.asdf, 0x1337);
        assert_eq!(thingy.true_false_maybe, None);

        let input4 = format!("{{asdf = {} true_false_maybe = yes}}", 0x1337);
        let (thingy, _rest) = Thingy::take_text(TextDeserializer::from_str(&input4)).unwrap();
        assert_eq!(thingy.asdf, 0x1337);
        assert_eq!(thingy.true_false_maybe, Some(true));
    }
    #[test]
    fn thingy_bin() {
        #[derive(BinDeserialize)]
        struct Thingy {
            #[bin_token("test")]
            asdf: u32,
            #[bin_token("test")]
            true_false_maybe: Option<bool>,
        }

        let input = b"\x03\x00\
        \x17\x00\x04\x00\x01\x01\x01\x00\x14\x00\x37\x13\x00\x00\
        \x04\x00";
        let (thingy, _rest) = Thingy::take(BinDeserializer::from_bytes(
            input,
            &StringsResolver::default(),
        ))
        .unwrap();
        assert_eq!(thingy.asdf, 0x1337);

        let input2 = b"\x03\x00\
        \x01\x01\x01\x00\x14\x00\x37\x13\x00\x00\
        \x34\x12\x01\x00\x0e\x00\x01\
        \x04\x00";
        let (thingy, _rest) = Thingy::take(BinDeserializer::from_bytes(
            input2,
            &StringsResolver::default(),
        ))
        .unwrap();
        assert_eq!(thingy.asdf, 0x1337);
        assert_eq!(thingy.true_false_maybe, Some(true));
    }
}
