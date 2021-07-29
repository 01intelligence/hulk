use std::io::Write;

use quick_xml::DeError;
use serde::ser::Serialize;

const HEADER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";

pub fn to_writer<W: Write, S: Serialize>(mut writer: W, value: &S) -> Result<(), DeError> {
    writer.write(HEADER.as_bytes());
    quick_xml::se::to_writer(writer, value)
}

pub fn to_writer_with_indent<W: Write, S: Serialize>(
    mut writer: W,
    value: &S,
    indent_char: u8,
    indent_size: usize,
) -> Result<(), DeError> {
    writer.write(HEADER.as_bytes());
    quick_xml::se::to_writer_with_indent(writer, value, indent_char, indent_size)
}

pub fn to_string<S: Serialize>(value: &S) -> Result<String, DeError> {
    let mut writer = Vec::new();
    to_writer(&mut writer, value)?;
    let s = String::from_utf8(writer).map_err(|e| quick_xml::Error::Utf8(e.utf8_error()))?;
    Ok(s)
}

pub fn to_string_with_indent<S: Serialize>(
    value: &S,
    indent_char: u8,
    indent_size: usize,
) -> Result<String, DeError> {
    let mut writer = Vec::new();
    to_writer_with_indent(&mut writer, value, indent_char, indent_size)?;
    let s = String::from_utf8(writer).map_err(|e| quick_xml::Error::Utf8(e.utf8_error()))?;
    Ok(s)
}
