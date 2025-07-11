use anyhow::Result;
use pdx_parser_core::raw_parser::{RawPDXObject, RawPDXObjectItem, RawPDXValue};

#[derive(clap::Args)]
#[command()]
pub struct ViewArgs {
    /// The location of the file to parse and search
    pub file: std::path::PathBuf,
    /// A series of keys
    ///
    /// - If path element is `$` it matches all value-type object items (not KV)
    /// - If path element is `*` it matches all KVs
    /// - If path element is `*some_text_here` it matches all KV with `some_text_here` as the key
    /// - Otherwise matches the first KV with the key.
    ///
    /// ## Examples
    /// - `country` `*` prints each country's data in Stellaris
    pub path: Vec<String>,
}

fn format_item(item: &RawPDXObjectItem<'_>, indent: usize) -> String {
    return match item {
        RawPDXObjectItem::KV(k, RawPDXValue::Scalar(scalar)) => {
            format!("{:indent$}{}: {}", "", k.0, scalar.0)
        }
        RawPDXObjectItem::KV(k, RawPDXValue::Object(object)) => {
            format!("{:indent$}{}: {{<{} items>}}", "", k.0, object.0.len())
        }
        RawPDXObjectItem::Value(RawPDXValue::Scalar(scalar)) => {
            format!("{:indent$}{}", "", scalar.0)
        }
        RawPDXObjectItem::Value(RawPDXValue::Object(obj)) => {
            format!("{:indent$}{{<{} items>}}", "", obj.0.len())
        }
    };
}

fn format_value(value: &RawPDXValue<'_>, indent: usize) -> String {
    return match value {
        RawPDXValue::Scalar(scalar) => format!("{:indent$}{}", "", scalar.0),
        RawPDXValue::Object(object) => {
            let inside: String = object
                .0
                .iter()
                .map(|item| format_item(item, indent + 4) + "\n")
                .collect();
            format!("{{\n{inside}{:indent$}}}", "")
        }
    };
}

/// - If path element is `$` it matches all value-type object items (not KV)
/// - If path element is `*` it matches all KVs
/// - If path element is `*some_text_here` it matches all KV with `some_text_here` as the key
/// - Otherwise matches the first KV with the key.
fn visit(value: &RawPDXValue<'_>, path: &[&str]) {
    match &path {
        &[] => println!("{}", format_value(value, 0)),
        &["$", rest @ ..] => match value {
            RawPDXValue::Scalar(_) => eprintln!("<ERROR: cannot use $ on scalar>"),
            RawPDXValue::Object(object) => {
                for value in object.iter_values() {
                    visit(value, rest);
                }
            }
        },
        &["*", rest @ ..] => match value {
            RawPDXValue::Scalar(_) => eprintln!("<ERROR: cannot use * on scalar>"),
            RawPDXValue::Object(object) => {
                for (_, value) in object.iter_all_KVs() {
                    visit(value, rest);
                }
            }
        },
        &[curr, rest @ ..] => match value {
            RawPDXValue::Scalar(_) => eprintln!("<ERROR: cannot index scalar>"),
            RawPDXValue::Object(object) => {
                if let Some(curr) = curr.strip_prefix('*') {
                    for (k, v) in object.iter_all_KVs() {
                        if k.as_string() == curr {
                            visit(v, rest);
                        }
                    }
                } else if let Some(v) = object.get_first(&curr) {
                    visit(v, rest);
                } else {
                    eprintln!("<ERROR: could not find key \"{curr}\"");
                }
            }
        },
    }
}

pub fn view_main(args: ViewArgs) -> Result<()> {
    // TODO: eventually might want to add an interactive mode
    let file = std::fs::File::open(args.file)?;
    let text = crate::utils::from_cp1252(file)?;
    let text: String = crate::utils::lines_without_comments(&text)
        .flat_map(|line| [line, "\n"])
        .collect();
    let (rest, obj) = RawPDXObject::parse_object_inner(&text).unwrap();
    assert!(rest.is_empty());

    let path: Vec<&str> = args.path.iter().map(String::as_str).collect();
    visit(&obj.into(), &path);
    return Ok(());
}
