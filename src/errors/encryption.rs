// AWS errors for invalid SSE-C requests.
pub const ERR_ENCRYPTED_OBJECT: &str = "The object was stored using a form of SSE";
pub const ERR_INVALID_SSE_PARAMETERS: &str = "The SSE-C key for key-rotation is not correct"; // special access denied
pub const ERR_KMS_NOT_CONFIGURED: &str = "KMS not configured for a server side encrypted object";
// Additional MinIO errors for SSE-C requests.
pub const ERR_OBJECT_TAMPERED: &str = "The requested object was modified and may be compromised";
// error returned when invalid encryption parameters are specified
pub const ERR_INVALID_ENCRYPTION_PARAMETERS: &str =
    "The encryption parameters are not applicable to this object";
