// Format config file carries backend format specific details.
const FORMAT_CONFIG_FILE: &str = "format.json";

// Version of the FormatMetaV1
const FORMAT_META_VERSION_V1: &str = "1";

// format.json currently has the format:
// {
//   "version": "1",
//   "format": "XXXXX",
//   "XXXXX": {
//
//   }
// }
// Here "XXXXX" depends on the backend, currently we have "fs" and "xl" implementations.
// FormatMetaV1 should be inherited by backend format structs. Please look at format-fs.go
// and format-xl.go for details.

// Ideally we will never have a situation where we will have to change the
// fields of this struct and deal with related migration.
struct FormatMetaV1 {
    version: String, // Version of the format config.
    format: String,  // The backend format type, supports two values 'xl' and 'fs'.
    id: String,      // The identifier for the deployment.
}
