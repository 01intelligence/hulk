use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CSVInput {
    /// <p>Specifies that CSV field values may contain quoted record delimiters and such records should be allowed. Default value is FALSE. Setting this value to TRUE may lower performance.</p>
    pub allow_quoted_record_delimiter: Option<bool>,
    /// <p>A single character used to indicate that a row should be ignored when the character is present at the start of that row. You can specify any character to indicate a comment line.</p>
    pub comments: Option<String>,
    /// <p>A single character used to separate individual fields in a record. You can specify an arbitrary delimiter.</p>
    pub field_delimiter: Option<String>,
    /// <p><p>Describes the first line of input. Valid values are:</p> <ul> <li> <p> <code>NONE</code>: First line is not a header.</p> </li> <li> <p> <code>IGNORE</code>: First line is a header, but you can&#39;t use the header values to indicate the column in an expression. You can use column position (such as _1, <em>2, …) to indicate the column (<code>SELECT s.</em>1 FROM OBJECT s</code>).</p> </li> <li> <p> <code>Use</code>: First line is a header, and you can use the header value to identify a column in an expression (<code>SELECT &quot;name&quot; FROM OBJECT</code>). </p> </li> </ul></p>
    pub file_header_info: Option<String>,
    /// <p>A single character used for escaping when the field delimiter is part of the value. For example, if the value is <code>a, b</code>, Amazon S3 wraps this field value in quotation marks, as follows: <code>" a , b "</code>.</p> <p>Type: String</p> <p>Default: <code>"</code> </p> <p>Ancestors: <code>CSV</code> </p>
    pub quote_character: Option<String>,
    /// <p>A single character used for escaping the quotation mark character inside an already escaped value. For example, the value """ a , b """ is parsed as " a , b ".</p>
    pub quote_escape_character: Option<String>,
    /// <p>A single character used to separate individual records in the input. Instead of the default value, you can specify an arbitrary delimiter.</p>
    pub record_delimiter: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct JSONInput {
    /// <p>The type of JSON. Valid values: Document, Lines.</p>
    #[serde(rename = "Type")]
    pub type_: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ParquetInput {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    None,
    Gzip,
    Bzip2,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct InputSerialization {
    /// <p>Specifies object's compression format. Valid values: NONE, GZIP, BZIP2. Default Value: NONE.</p>
    pub compression_type: Option<CompressionType>,
    /// <p>Describes the serialization of a CSV-encoded object.</p>
    #[serde(rename = "CSV")]
    pub csv: Option<CSVInput>,
    /// <p>Specifies JSON as object's input serialization format.</p>
    #[serde(rename = "JSON")]
    pub json: Option<JSONInput>,
    /// <p>Specifies Parquet as object's input serialization format.</p>
    pub parquet: Option<ParquetInput>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CSVOutput {
    /// <p>The value used to separate individual fields in a record. You can specify an arbitrary delimiter.</p>
    pub field_delimiter: Option<String>,
    /// <p>A single character used for escaping when the field delimiter is part of the value. For example, if the value is <code>a, b</code>, Amazon S3 wraps this field value in quotation marks, as follows: <code>" a , b "</code>.</p>
    pub quote_character: Option<String>,
    /// <p>The single character used for escaping the quote character inside an already escaped value.</p>
    pub quote_escape_character: Option<String>,
    /// <p><p>Indicates whether to use quotation marks around output fields. </p> <ul> <li> <p> <code>ALWAYS</code>: Always use quotation marks for output fields.</p> </li> <li> <p> <code>ASNEEDED</code>: Use quotation marks for output fields when needed.</p> </li> </ul></p>
    pub quote_fields: Option<String>,
    /// <p>A single character used to separate individual records in the output. Instead of the default value, you can specify an arbitrary delimiter.</p>
    pub record_delimiter: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct JSONOutput {
    /// <p>The value used to separate individual records in the output. If no value is specified, Amazon S3 uses a newline character ('\n').</p>
    pub record_delimiter: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OutputSerialization {
    /// <p>Describes the serialization of CSV-encoded Select results.</p>
    pub csv: Option<CSVOutput>,
    /// <p>Specifies JSON as request's output serialization format.</p>
    pub json: Option<JSONOutput>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RequestProgress {
    /// <p>Specifies whether periodic QueryProgress frames should be sent. Valid values: TRUE, FALSE. Default value: FALSE.</p>
    pub enabled: Option<bool>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SelectParameters {
    /// <p>The expression that is used to query the object.</p>
    pub expression: String,
    /// <p>The type of the provided expression (for example, SQL).</p>
    pub expression_type: String,
    /// <p>Describes the serialization format of the object.</p>
    pub input_serialization: InputSerialization,
    /// <p>Describes how the results of the Select job are serialized.</p>
    pub output_serialization: OutputSerialization,
    /// <p>Specifies if periodic request progress information should be enabled.</p>
    pub request_progress: Option<RequestProgress>,
}
