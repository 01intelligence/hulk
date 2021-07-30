use actix_web::http::StatusCode;
use const_format::formatcp;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::globals::Guard;

#[derive(Debug)]
pub struct GenericApiError {
    pub code: &'static str,
    pub description: String,
    pub http_status_code: StatusCode,
}

#[derive(Debug)]
pub struct GenericApiErrorConst {
    pub code: &'static str,
    pub description: &'static str,
    pub http_status_code: StatusCode,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ApiErrorResponse {
    pub code: &'static str,
    pub message: String,
    pub key: String,
    pub bucket_name: String,
    pub resource: String,
    pub region: String,
    pub request_id: String,
    pub host_id: String,
}

impl ApiErrorResponse {
    pub fn from(
        err: GenericApiError,
        resource: String,
        request_id: String,
        host_id: String,
    ) -> Self {
        ApiErrorResponse {
            code: err.code,
            message: err.description,
            key: "".to_string(),         // TODO
            bucket_name: "".to_string(), // TODO
            resource,
            region: crate::globals::GLOBALS.server_region.guard().to_owned(),
            request_id,
            host_id,
        }
    }
}

/// S3 Error codes, non exhaustive list.
/// Refer: http://docs.aws.amazon.com/AmazonS3/latest/API/ErrorResponses.htm
#[non_exhaustive]
#[derive(Debug, Copy, Clone)]
pub enum ApiError {
    None,
    AccessDenied,
    BadDigest,
    EntityTooSmall,
    EntityTooLarge,
    PolicyTooLarge,
    IncompleteBody,
    InternalError,
    InvalidAccessKeyID,
    InvalidBucketName,
    InvalidDigest,
    InvalidRange,
    InvalidRangePartNumber,
    InvalidCopyPartRange,
    InvalidCopyPartRangeSource,
    InvalidMaxKeys,
    InvalidEncodingMethod,
    InvalidMaxUploads,
    InvalidMaxParts,
    InvalidPartNumberMarker,
    InvalidPartNumber,
    InvalidRequestBody,
    InvalidCopySource,
    InvalidMetadataDirective,
    InvalidCopyDest,
    InvalidPolicyDocument,
    InvalidObjectState,
    MalformedXML,
    MissingContentLength,
    MissingContentMD5,
    MissingRequestBodyError,
    MissingSecurityHeader,
    NoSuchBucket,
    NoSuchBucketPolicy,
    NoSuchBucketLifecycle,
    NoSuchLifecycleConfiguration,
    NoSuchBucketSSEConfig,
    NoSuchCORSConfiguration,
    NoSuchWebsiteConfiguration,
    ReplicationConfigurationNotFoundError,
    RemoteDestinationNotFoundError,
    ReplicationDestinationMissingLock,
    RemoteTargetNotFoundError,
    ReplicationRemoteConnectionError,
    BucketRemoteIdenticalToSource,
    BucketRemoteAlreadyExists,
    BucketRemoteLabelInUse,
    BucketRemoteArnTypeInvalid,
    BucketRemoteArnInvalid,
    BucketRemoteRemoveDisallowed,
    RemoteTargetNotVersionedError,
    ReplicationSourceNotVersionedError,
    ReplicationNeedsVersioningError,
    ReplicationBucketNeedsVersioningError,
    ReplicationNoMatchingRuleError,
    ObjectRestoreAlreadyInProgress,
    NoSuchKey,
    NoSuchUpload,
    InvalidVersionID,
    NoSuchVersion,
    NotImplemented,
    PreconditionFailed,
    RequestTimeTooSkewed,
    SignatureDoesNotMatch,
    MethodNotAllowed,
    InvalidPart,
    InvalidPartOrder,
    AuthorizationHeaderMalformed,
    MalformedPOSTRequest,
    POSTFileRequired,
    SignatureVersionNotSupported,
    BucketNotEmpty,
    AllAccessDisabled,
    MalformedPolicy,
    MissingFields,
    MissingCredTag,
    CredMalformed,
    InvalidRegion,
    InvalidServiceS3,
    InvalidServiceSTS,
    InvalidRequestVersion,
    MissingSignTag,
    MissingSignHeadersTag,
    MalformedDate,
    MalformedPresignedDate,
    MalformedCredentialDate,
    MalformedCredentialRegion,
    MalformedExpires,
    NegativeExpires,
    AuthHeaderEmpty,
    ExpiredPresignRequest,
    RequestNotReadyYet,
    UnsignedHeaders,
    MissingDateHeader,
    InvalidQuerySignatureAlgo,
    InvalidQueryParams,
    BucketAlreadyOwnedByYou,
    InvalidDuration,
    BucketAlreadyExists,
    MetadataTooLarge,
    UnsupportedMetadata,
    MaximumExpires,
    SlowDown,
    InvalidPrefixMarker,
    BadRequest,
    KeyTooLongError,
    InvalidBucketObjectLockConfiguration,
    ObjectLockConfigurationNotFound,
    ObjectLockConfigurationNotAllowed,
    NoSuchObjectLockConfiguration,
    ObjectLocked,
    InvalidRetentionDate,
    PastObjectLockRetainDate,
    UnknownWORMModeDirective,
    BucketTaggingNotFound,
    ObjectLockInvalidHeaders,
    InvalidTagDirective,
    // Add new error codes here.

    // SSE-S3 related API errors
    InvalidEncryptionMethod,

    // Server-Side-Encryption (with Customer provided key) related API errors.
    InsecureSSECustomerRequest,
    SSEMultipartEncrypted,
    SSEEncryptedObject,
    InvalidEncryptionParameters,
    InvalidSSECustomerAlgorithm,
    InvalidSSECustomerKey,
    MissingSSECustomerKey,
    MissingSSECustomerKeyMD5,
    SSECustomerKeyMD5Mismatch,
    InvalidSSECustomerParameters,
    IncompatibleEncryptionMethod,
    KMSNotConfigured,

    NoAccessKey,
    InvalidToken,

    // Bucket notification related errors.
    EventNotification,
    ARNNotification,
    RegionNotification,
    OverlappingFilterNotification,
    FilterNameInvalid,
    FilterNamePrefix,
    FilterNameSuffix,
    FilterValueInvalid,
    OverlappingConfigs,
    UnsupportedNotification,

    // S3 extended errors.
    ContentSHA256Mismatch,

    // Add new extended error codes here.

    // Hulk extended errors.
    ReadQuorum,
    WriteQuorum,
    ParentIsObject,
    StorageFull,
    RequestBodyParse,
    ObjectExistsAsDirectory,
    InvalidObjectName,
    InvalidObjectNamePrefixSlash,
    InvalidResourceName,
    ServerNotInitialized,
    OperationTimedOut,
    ClientDisconnected,
    OperationMaxedOut,
    InvalidRequest,
    TransitionStorageClassNotFoundError,
    // Storage class error codes
    InvalidStorageClass,
    BackendDown,
    // Add new extended error codes here.
    MalformedJSON,
    AdminNoSuchUser,
    AdminNoSuchGroup,
    AdminGroupNotEmpty,
    AdminNoSuchPolicy,
    AdminInvalidArgument,
    AdminInvalidAccessKey,
    AdminInvalidSecretKey,
    AdminConfigNoQuorum,
    AdminConfigTooLarge,
    AdminConfigBadJSON,
    AdminConfigDuplicateKeys,
    AdminCredentialsMismatch,
    InsecureClientRequest,
    ObjectTampered,
    // Bucket Quota error codes
    AdminBucketQuotaExceeded,
    AdminNoSuchQuotaConfiguration,

    HealNotImplemented,
    HealNoSuchProcess,
    HealInvalidClientToken,
    HealMissingBucket,
    HealAlreadyRunning,
    HealOverlappingPaths,
    IncorrectContinuationToken,

    // S3 Select ors,
    EmptyRequestBody,
    UnsupportedFunction,
    InvalidExpressionType,
    Busy,
    UnauthorizedAccess,
    ExpressionTooLong,
    IllegalSQLFunctionArgument,
    InvalidKeyPath,
    InvalidCompressionFormat,
    InvalidFileHeaderInfo,
    InvalidJSONType,
    InvalidQuoteFields,
    InvalidRequestParameter,
    InvalidDataType,
    InvalidTextEncoding,
    InvalidDataSource,
    InvalidTableAlias,
    MissingRequiredParameter,
    ObjectSerializationConflict,
    UnsupportedSQLOperation,
    UnsupportedSQLStructure,
    UnsupportedSyntax,
    UnsupportedRangeHeader,
    LexerInvalidChar,
    LexerInvalidOperator,
    LexerInvalidLiteral,
    LexerInvalidIONLiteral,
    ParseExpectedDatePart,
    ParseExpectedKeyword,
    ParseExpectedTokenType,
    ParseExpected2TokenTypes,
    ParseExpectedNumber,
    ParseExpectedRightParenBuiltinFunctionCall,
    ParseExpectedTypeName,
    ParseExpectedWhenClause,
    ParseUnsupportedToken,
    ParseUnsupportedLiteralsGroupBy,
    ParseExpectedMember,
    ParseUnsupportedSelect,
    ParseUnsupportedCase,
    ParseUnsupportedCaseClause,
    ParseUnsupportedAlias,
    ParseUnsupportedSyntax,
    ParseUnknownOperator,
    ParseMissingIdentAfterAt,
    ParseUnexpectedOperator,
    ParseUnexpectedTerm,
    ParseUnexpectedToken,
    ParseUnexpectedKeyword,
    ParseExpectedExpression,
    ParseExpectedLeftParenAfterCast,
    ParseExpectedLeftParenValueConstructor,
    ParseExpectedLeftParenBuiltinFunctionCall,
    ParseExpectedArgumentDelimiter,
    ParseCastArity,
    ParseInvalidTypeParam,
    ParseEmptySelect,
    ParseSelectMissingFrom,
    ParseExpectedIdentForGroupName,
    ParseExpectedIdentForAlias,
    ParseUnsupportedCallWithStar,
    ParseNonUnaryAgregateFunctionCall,
    ParseMalformedJoin,
    ParseExpectedIdentForAt,
    ParseAsteriskIsNotAloneInSelectList,
    ParseCannotMixSqbAndWildcardInSelectList,
    ParseInvalidContextForWildcardInSelectList,
    IncorrectSQLFunctionArgumentType,
    ValueParseFailure,
    EvaluatorInvalidArguments,
    IntegerOverflow,
    LikeInvalidInputs,
    CastFailed,
    InvalidCast,
    EvaluatorInvalidTimestampFormatPattern,
    EvaluatorInvalidTimestampFormatPatternSymbolForParsing,
    EvaluatorTimestampFormatPatternDuplicateFields,
    EvaluatorTimestampFormatPatternHourClockAmPmMismatch,
    EvaluatorUnterminatedTimestampFormatPatternToken,
    EvaluatorInvalidTimestampFormatPatternToken,
    EvaluatorInvalidTimestampFormatPatternSymbol,
    EvaluatorBindingDoesNotExist,
    MissingHeaders,
    InvalidColumnIndex,

    AdminConfigNotificationTargetsFailed,
    AdminProfilerNotEnabled,
    InvalidDecompressedSize,
    AddUserInvalidArgument,
    AdminAccountNotEligible,
    AccountNotEligible,
    AdminServiceAccountNotFound,
    PostPolicyConditionInvalidFormat,
}

impl ApiError {
    pub fn to_with_err(&self, err: &str) -> GenericApiError {
        self.value().to(self, Some(err))
    }

    pub fn to(&self) -> GenericApiError {
        self.value().to(self, None)
    }
}

impl GenericApiErrorConst {
    const fn new(
        code: &'static str,
        description: &'static str,
        http_status_code: StatusCode,
    ) -> Self {
        GenericApiErrorConst {
            code,
            description,
            http_status_code,
        }
    }

    fn to(&self, ae: &ApiError, err: Option<&str>) -> GenericApiError {
        let mut desc = None;
        let region = crate::globals::GLOBALS.server_region.guard();
        if !region.is_empty() {
            if let &ApiError::AuthorizationHeaderMalformed = ae {
                desc = Some(format!(
                    "The authorization header is malformed; the region is wrong; expecting '{}'.",
                    *region
                ));
            }
        }
        let desc = desc.unwrap_or({
            let mut desc = self.description.to_owned();
            if let Some(err) = err {
                desc = format!("{} ({})", desc, err);
            }
            desc
        });
        GenericApiError {
            code: self.code,
            description: desc,
            http_status_code: self.http_status_code,
        }
    }
}

impl ApiError {
    pub fn value(&self) -> &'static GenericApiErrorConst {
        match *self {
            ApiError::None => &INTERNAL_ERROR,
            ApiError::AccessDenied => &ACCESS_DENIED,
            ApiError::BadDigest => &BAD_DIGEST,
            ApiError::EntityTooSmall => &ENTITY_TOO_SMALL,
            ApiError::EntityTooLarge => &ENTITY_TOO_LARGE,
            ApiError::PolicyTooLarge => &POLICY_TOO_LARGE,
            ApiError::IncompleteBody => &INCOMPLETE_BODY,
            ApiError::InternalError => &INTERNAL_ERROR,
            ApiError::InvalidAccessKeyID => &INVALID_ACCESS_KEY_ID,
            ApiError::InvalidBucketName => &INVALID_BUCKET_NAME,
            ApiError::InvalidDigest => &INVALID_DIGEST,
            ApiError::InvalidRange => &INVALID_RANGE,
            ApiError::InvalidRangePartNumber => &INVALID_RANGE_PART_NUMBER,
            ApiError::InvalidCopyPartRange => &INVALID_COPY_PART_RANGE,
            ApiError::InvalidCopyPartRangeSource => &INVALID_COPY_PART_RANGE_SOURCE,
            ApiError::InvalidMaxKeys => &INVALID_MAX_KEYS,
            ApiError::InvalidEncodingMethod => &INVALID_ENCODING_METHOD,
            ApiError::InvalidMaxUploads => &INVALID_MAX_UPLOADS,
            ApiError::InvalidMaxParts => &INVALID_MAX_PARTS,
            ApiError::InvalidPartNumberMarker => &INVALID_PART_NUMBER_MARKER,
            ApiError::InvalidPartNumber => &INVALID_PART_NUMBER,
            ApiError::InvalidRequestBody => &INVALID_REQUEST_BODY,
            ApiError::InvalidCopySource => &INVALID_COPY_SOURCE,
            ApiError::InvalidMetadataDirective => &INVALID_METADATA_DIRECTIVE,
            ApiError::InvalidCopyDest => &INVALID_COPY_DEST,
            ApiError::InvalidPolicyDocument => &INVALID_POLICY_DOCUMENT,
            ApiError::InvalidObjectState => &INVALID_OBJECT_STATE,
            ApiError::MalformedXML => &MALFORMED_XML,
            ApiError::MissingContentLength => &MISSING_CONTENT_LENGTH,
            ApiError::MissingContentMD5 => &MISSING_CONTENT_MD5,
            ApiError::MissingRequestBodyError => &MISSING_REQUEST_BODY_ERROR,
            ApiError::MissingSecurityHeader => &MISSING_SECURITY_HEADER,
            ApiError::NoSuchBucket => &NO_SUCH_BUCKET,
            ApiError::NoSuchBucketPolicy => &NO_SUCH_BUCKET_POLICY,
            ApiError::NoSuchBucketLifecycle => &NO_SUCH_BUCKET_LIFECYCLE,
            ApiError::NoSuchLifecycleConfiguration => &NO_SUCH_LIFECYCLE_CONFIGURATION,
            ApiError::NoSuchBucketSSEConfig => &NO_SUCH_BUCKET_SSE_CONFIG,
            ApiError::NoSuchCORSConfiguration => &NO_SUCH_CORS_CONFIGURATION,
            ApiError::NoSuchWebsiteConfiguration => &NO_SUCH_WEBSITE_CONFIGURATION,
            ApiError::ReplicationConfigurationNotFoundError => {
                &REPLICATION_CONFIGURATION_NOT_FOUND_ERROR
            }
            ApiError::RemoteDestinationNotFoundError => &REMOTE_DESTINATION_NOT_FOUND_ERROR,
            ApiError::ReplicationDestinationMissingLock => &REPLICATION_DESTINATION_MISSING_LOCK,
            ApiError::RemoteTargetNotFoundError => &REMOTE_TARGET_NOT_FOUND_ERROR,
            ApiError::ReplicationRemoteConnectionError => &REPLICATION_REMOTE_CONNECTION_ERROR,
            ApiError::BucketRemoteIdenticalToSource => &BUCKET_REMOTE_IDENTICAL_TO_SOURCE,
            ApiError::BucketRemoteAlreadyExists => &BUCKET_REMOTE_ALREADY_EXISTS,
            ApiError::BucketRemoteLabelInUse => &BUCKET_REMOTE_LABEL_IN_USE,
            ApiError::BucketRemoteArnTypeInvalid => &BUCKET_REMOTE_ARN_TYPE_INVALID,
            ApiError::BucketRemoteArnInvalid => &BUCKET_REMOTE_ARN_INVALID,
            ApiError::BucketRemoteRemoveDisallowed => &BUCKET_REMOTE_REMOVE_DISALLOWED,
            ApiError::RemoteTargetNotVersionedError => &REMOTE_TARGET_NOT_VERSIONED_ERROR,
            ApiError::ReplicationSourceNotVersionedError => &REMOTE_TARGET_NOT_VERSIONED_ERROR,
            ApiError::ReplicationNeedsVersioningError => &REPLICATION_NEEDS_VERSIONING_ERROR,
            ApiError::ReplicationBucketNeedsVersioningError => {
                &REPLICATION_BUCKET_NEEDS_VERSIONING_ERROR
            }
            ApiError::ReplicationNoMatchingRuleError => &REPLICATION_NO_MATCHING_RULE_ERROR,
            ApiError::ObjectRestoreAlreadyInProgress => &OBJECT_RESTORE_ALREADY_IN_PROGRESS,
            ApiError::NoSuchKey => &NO_SUCH_KEY,
            ApiError::NoSuchUpload => &NO_SUCH_UPLOAD,
            ApiError::InvalidVersionID => &INVALID_VERSION_ID,
            ApiError::NoSuchVersion => &NO_SUCH_VERSION,
            ApiError::NotImplemented => &NOT_IMPLEMENTED,
            ApiError::PreconditionFailed => &PRECONDITION_FAILED,
            ApiError::RequestTimeTooSkewed => &REQUEST_TIME_TOO_SKEWED,
            ApiError::SignatureDoesNotMatch => &SIGNATURE_DOES_NOT_MATCH,
            ApiError::MethodNotAllowed => &METHOD_NOT_ALLOWED,
            ApiError::InvalidPart => &INVALID_PART,
            ApiError::InvalidPartOrder => &INVALID_PART_ORDER,
            ApiError::AuthorizationHeaderMalformed => &AUTHORIZATION_HEADER_MALFORMED,
            ApiError::MalformedPOSTRequest => &MALFORMED_POSTREQUEST,
            ApiError::POSTFileRequired => &POST_FILE_REQUIRED,
            ApiError::SignatureVersionNotSupported => &SIGNATURE_VERSION_NOT_SUPPORTED,
            ApiError::BucketNotEmpty => &BUCKET_NOT_EMPTY,
            ApiError::AllAccessDisabled => &ALL_ACCESS_DISABLED,
            ApiError::MalformedPolicy => &MALFORMED_POLICY,
            ApiError::MissingFields => &MISSING_FIELDS,
            ApiError::MissingCredTag => &MISSING_CRED_TAG,
            ApiError::CredMalformed => &CRED_MALFORMED,
            ApiError::InvalidRegion => &INVALID_REGION,
            ApiError::InvalidServiceS3 => &INVALID_SERVICE_S3,
            ApiError::InvalidServiceSTS => &INVALID_SERVICE_STS,
            ApiError::InvalidRequestVersion => &INVALID_REQUEST_VERSION,
            ApiError::MissingSignTag => &MISSING_SIGN_TAG,
            ApiError::MissingSignHeadersTag => &MISSING_SIGN_HEADERS_TAG,
            ApiError::MalformedDate => &MALFORMED_DATE,
            ApiError::MalformedPresignedDate => &MALFORMED_PRESIGNED_DATE,
            ApiError::MalformedCredentialDate => &MALFORMED_CREDENTIAL_DATE,
            ApiError::MalformedCredentialRegion => &INTERNAL_ERROR, // TODO
            ApiError::MalformedExpires => &MALFORMED_EXPIRES,
            ApiError::NegativeExpires => &NEGATIVE_EXPIRES,
            ApiError::AuthHeaderEmpty => &AUTH_HEADER_EMPTY,
            ApiError::ExpiredPresignRequest => &EXPIRED_PRESIGN_REQUEST,
            ApiError::RequestNotReadyYet => &REQUEST_NOT_READY_YET,
            ApiError::UnsignedHeaders => &UNSIGNED_HEADERS,
            ApiError::MissingDateHeader => &MISSING_DATE_HEADER,
            ApiError::InvalidQuerySignatureAlgo => &INVALID_QUERY_SIGNATURE_ALGO,
            ApiError::InvalidQueryParams => &INVALID_QUERY_PARAMS,
            ApiError::BucketAlreadyOwnedByYou => &BUCKET_ALREADY_OWNED_BY_YOU,
            ApiError::InvalidDuration => &INVALID_DURATION,
            ApiError::BucketAlreadyExists => &BUCKET_ALREADY_EXISTS,
            ApiError::MetadataTooLarge => &METADATA_TOO_LARGE,
            ApiError::UnsupportedMetadata => &UNSUPPORTED_METADATA,
            ApiError::MaximumExpires => &MAXIMUM_EXPIRES,
            ApiError::SlowDown => &SLOW_DOWN,
            ApiError::InvalidPrefixMarker => &INVALID_PREFIX_MARKER,
            ApiError::BadRequest => &BAD_REQUEST,
            ApiError::KeyTooLongError => &KEY_TOO_LONG_ERROR,
            ApiError::InvalidBucketObjectLockConfiguration => {
                &INVALID_BUCKET_OBJECT_LOCK_CONFIGURATION
            }
            ApiError::ObjectLockConfigurationNotFound => &OBJECT_LOCK_CONFIGURATION_NOT_FOUND,
            ApiError::ObjectLockConfigurationNotAllowed => &OBJECT_LOCK_CONFIGURATION_NOT_ALLOWED,
            ApiError::NoSuchObjectLockConfiguration => &NO_SUCH_OBJECT_LOCK_CONFIGURATION,
            ApiError::ObjectLocked => &OBJECT_LOCKED,
            ApiError::InvalidRetentionDate => &INVALID_RETENTION_DATE,
            ApiError::PastObjectLockRetainDate => &PAST_OBJECT_LOCK_RETAIN_DATE,
            ApiError::UnknownWORMModeDirective => &UNKNOWN_WORMMODE_DIRECTIVE,
            ApiError::BucketTaggingNotFound => &BUCKET_TAGGING_NOT_FOUND,
            ApiError::ObjectLockInvalidHeaders => &OBJECT_LOCK_INVALID_HEADERS,
            ApiError::InvalidTagDirective => &INVALID_TAG_DIRECTIVE,
            ApiError::InvalidEncryptionMethod => &INVALID_ENCRYPTION_METHOD,
            ApiError::InsecureSSECustomerRequest => &INSECURE_SSECUSTOMER_REQUEST,
            ApiError::SSEMultipartEncrypted => &SSEMULTIPART_ENCRYPTED,
            ApiError::SSEEncryptedObject => &SSEENCRYPTED_OBJECT,
            ApiError::InvalidEncryptionParameters => &INVALID_ENCRYPTION_PARAMETERS,
            ApiError::InvalidSSECustomerAlgorithm => &INVALID_SSECUSTOMER_ALGORITHM,
            ApiError::InvalidSSECustomerKey => &INVALID_SSECUSTOMER_KEY,
            ApiError::MissingSSECustomerKey => &MISSING_SSECUSTOMER_KEY,
            ApiError::MissingSSECustomerKeyMD5 => &MISSING_SSE_CUSTOMER_KEY_MD5,
            ApiError::SSECustomerKeyMD5Mismatch => &SSE_CUSTOMER_KEY_MD5_MISMATCH,
            ApiError::InvalidSSECustomerParameters => &INVALID_SSE_CUSTOMER_PARAMETERS,
            ApiError::IncompatibleEncryptionMethod => &INCOMPATIBLE_ENCRYPTION_METHOD,
            ApiError::KMSNotConfigured => &KMSNOT_CONFIGURED,
            ApiError::NoAccessKey => &NO_ACCESS_KEY,
            ApiError::InvalidToken => &INVALID_TOKEN,
            ApiError::EventNotification => &EVENT_NOTIFICATION,
            ApiError::ARNNotification => &ARN_NOTIFICATION,
            ApiError::RegionNotification => &REGION_NOTIFICATION,
            ApiError::OverlappingFilterNotification => &OVERLAPPING_FILTER_NOTIFICATION,
            ApiError::FilterNameInvalid => &FILTER_NAME_INVALID,
            ApiError::FilterNamePrefix => &FILTER_NAME_PREFIX,
            ApiError::FilterNameSuffix => &FILTER_NAME_SUFFIX,
            ApiError::FilterValueInvalid => &FILTER_VALUE_INVALID,
            ApiError::OverlappingConfigs => &OVERLAPPING_CONFIGS,
            ApiError::UnsupportedNotification => &UNSUPPORTED_NOTIFICATION,
            ApiError::ContentSHA256Mismatch => &CONTENT_SHA256_MISMATCH,
            ApiError::ReadQuorum => &SLOW_DOWN,  // TODO
            ApiError::WriteQuorum => &SLOW_DOWN, // TODO
            ApiError::ParentIsObject => &PARENT_IS_OBJECT,
            ApiError::StorageFull => &STORAGE_FULL,
            ApiError::RequestBodyParse => &REQUEST_BODY_PARSE,
            ApiError::ObjectExistsAsDirectory => &OBJECT_EXISTS_AS_DIRECTORY,
            ApiError::InvalidObjectName => &INVALID_OBJECT_NAME,
            ApiError::InvalidObjectNamePrefixSlash => &INVALID_OBJECT_NAME_PREFIX_SLASH,
            ApiError::InvalidResourceName => &INVALID_RESOURCE_NAME,
            ApiError::ServerNotInitialized => &SERVER_NOT_INITIALIZED,
            ApiError::OperationTimedOut => &OPERATION_TIMED_OUT,
            ApiError::ClientDisconnected => &CLIENT_DISCONNECTED,
            ApiError::OperationMaxedOut => &OPERATION_MAXED_OUT,
            ApiError::InvalidRequest => &INVALID_REQUEST,
            ApiError::TransitionStorageClassNotFoundError => {
                &TRANSITION_STORAGE_CLASS_NOT_FOUND_ERROR
            }
            ApiError::InvalidStorageClass => &INVALID_STORAGE_CLASS,
            ApiError::BackendDown => &BACKEND_DOWN,
            ApiError::MalformedJSON => &MALFORMED_JSON,
            ApiError::AdminNoSuchUser => &ADMIN_NO_SUCH_USER,
            ApiError::AdminNoSuchGroup => &ADMIN_NO_SUCH_GROUP,
            ApiError::AdminGroupNotEmpty => &ADMIN_GROUP_NOT_EMPTY,
            ApiError::AdminNoSuchPolicy => &ADMIN_NO_SUCH_POLICY,
            ApiError::AdminInvalidArgument => &ADMIN_INVALID_ARGUMENT,
            ApiError::AdminInvalidAccessKey => &ADMIN_INVALID_ACCESS_KEY,
            ApiError::AdminInvalidSecretKey => &ADMIN_INVALID_SECRET_KEY,
            ApiError::AdminConfigNoQuorum => &ADMIN_CONFIG_NO_QUORUM,
            ApiError::AdminConfigTooLarge => &ADMIN_CONFIG_TOO_LARGE,
            ApiError::AdminConfigBadJSON => &ADMIN_CONFIG_BAD_JSON,
            ApiError::AdminConfigDuplicateKeys => &ADMIN_CONFIG_DUPLICATE_KEYS,
            ApiError::AdminCredentialsMismatch => &ADMIN_CREDENTIALS_MISMATCH,
            ApiError::InsecureClientRequest => &INSECURE_CLIENT_REQUEST,
            ApiError::ObjectTampered => &OBJECT_TAMPERED,
            ApiError::AdminBucketQuotaExceeded => &ADMIN_BUCKET_QUOTA_EXCEEDED,
            ApiError::AdminNoSuchQuotaConfiguration => &ADMIN_NO_SUCH_QUOTA_CONFIGURATION,
            ApiError::HealNotImplemented => &HEAL_NOT_IMPLEMENTED,
            ApiError::HealNoSuchProcess => &HEAL_NO_SUCH_PROCESS,
            ApiError::HealInvalidClientToken => &HEAL_INVALID_CLIENT_TOKEN,
            ApiError::HealMissingBucket => &HEAL_MISSING_BUCKET,
            ApiError::HealAlreadyRunning => &HEAL_ALREADY_RUNNING,
            ApiError::HealOverlappingPaths => &HEAL_OVERLAPPING_PATHS,
            ApiError::IncorrectContinuationToken => &INCORRECT_CONTINUATION_TOKEN,
            ApiError::EmptyRequestBody => &EMPTY_REQUEST_BODY,
            ApiError::UnsupportedFunction => &UNSUPPORTED_FUNCTION,
            ApiError::InvalidExpressionType => &INVALID_EXPRESSION_TYPE,
            ApiError::Busy => &BUSY,
            ApiError::UnauthorizedAccess => &UNAUTHORIZED_ACCESS,
            ApiError::ExpressionTooLong => &EXPRESSION_TOO_LONG,
            ApiError::IllegalSQLFunctionArgument => &ILLEGAL_SQLFUNCTION_ARGUMENT,
            ApiError::InvalidKeyPath => &INVALID_KEY_PATH,
            ApiError::InvalidCompressionFormat => &INVALID_COMPRESSION_FORMAT,
            ApiError::InvalidFileHeaderInfo => &INVALID_FILE_HEADER_INFO,
            ApiError::InvalidJSONType => &INVALID_JSONTYPE,
            ApiError::InvalidQuoteFields => &INVALID_QUOTE_FIELDS,
            ApiError::InvalidRequestParameter => &INVALID_REQUEST_PARAMETER,
            ApiError::InvalidDataType => &INVALID_DATA_TYPE,
            ApiError::InvalidTextEncoding => &INVALID_TEXT_ENCODING,
            ApiError::InvalidDataSource => &INVALID_DATA_SOURCE,
            ApiError::InvalidTableAlias => &INVALID_TABLE_ALIAS,
            ApiError::MissingRequiredParameter => &MISSING_REQUIRED_PARAMETER,
            ApiError::ObjectSerializationConflict => &OBJECT_SERIALIZATION_CONFLICT,
            ApiError::UnsupportedSQLOperation => &UNSUPPORTED_SQLOPERATION,
            ApiError::UnsupportedSQLStructure => &UNSUPPORTED_SQLSTRUCTURE,
            ApiError::UnsupportedSyntax => &UNSUPPORTED_SYNTAX,
            ApiError::UnsupportedRangeHeader => &UNSUPPORTED_RANGE_HEADER,
            ApiError::LexerInvalidChar => &LEXER_INVALID_CHAR,
            ApiError::LexerInvalidOperator => &LEXER_INVALID_OPERATOR,
            ApiError::LexerInvalidLiteral => &LEXER_INVALID_LITERAL,
            ApiError::LexerInvalidIONLiteral => &LEXER_INVALID_IONLITERAL,
            ApiError::ParseExpectedDatePart => &PARSE_EXPECTED_DATE_PART,
            ApiError::ParseExpectedKeyword => &PARSE_EXPECTED_KEYWORD,
            ApiError::ParseExpectedTokenType => &PARSE_EXPECTED_TOKEN_TYPE,
            ApiError::ParseExpected2TokenTypes => &PARSE_EXPECTED_2_TOKEN_TYPE,
            ApiError::ParseExpectedNumber => &PARSE_EXPECTED_NUMBER,
            ApiError::ParseExpectedRightParenBuiltinFunctionCall => {
                &PARSE_EXPECTED_RIGHT_PAREN_BUILTIN_FUNCTION_CALL
            }
            ApiError::ParseExpectedTypeName => &PARSE_EXPECTED_TYPE_NAME,
            ApiError::ParseExpectedWhenClause => &PARSE_EXPECTED_WHEN_CLAUSE,
            ApiError::ParseUnsupportedToken => &PARSE_UNSUPPORTED_TOKEN,
            ApiError::ParseUnsupportedLiteralsGroupBy => &PARSE_UNSUPPORTED_LITERALS_GROUP_BY,
            ApiError::ParseExpectedMember => &PARSE_EXPECTED_MEMBER,
            ApiError::ParseUnsupportedSelect => &PARSE_UNSUPPORTED_SELECT,
            ApiError::ParseUnsupportedCase => &PARSE_UNSUPPORTED_CASE,
            ApiError::ParseUnsupportedCaseClause => &PARSE_UNSUPPORTED_CASE_CLAUSE,
            ApiError::ParseUnsupportedAlias => &PARSE_UNSUPPORTED_ALIAS,
            ApiError::ParseUnsupportedSyntax => &PARSE_UNSUPPORTED_SYNTAX,
            ApiError::ParseUnknownOperator => &PARSE_UNKNOWN_OPERATOR,
            ApiError::ParseMissingIdentAfterAt => &PARSE_MISSING_IDENT_AFTER_AT,
            ApiError::ParseUnexpectedOperator => &PARSE_UNEXPECTED_OPERATOR,
            ApiError::ParseUnexpectedTerm => &PARSE_UNEXPECTED_TERM,
            ApiError::ParseUnexpectedToken => &PARSE_UNEXPECTED_TOKEN,
            ApiError::ParseUnexpectedKeyword => &PARSE_UNEXPECTED_KEYWORD,
            ApiError::ParseExpectedExpression => &PARSE_EXPECTED_EXPRESSION,
            ApiError::ParseExpectedLeftParenAfterCast => &PARSE_EXPECTED_LEFT_PAREN_AFTER_CAST,
            ApiError::ParseExpectedLeftParenValueConstructor => {
                &PARSE_EXPECTED_LEFT_PAREN_VALUE_CONSTRUCTOR
            }
            ApiError::ParseExpectedLeftParenBuiltinFunctionCall => {
                &PARSE_EXPECTED_LEFT_PAREN_BUILTIN_FUNCTION_CALL
            }
            ApiError::ParseExpectedArgumentDelimiter => &PARSE_EXPECTED_ARGUMENT_DELIMITER,
            ApiError::ParseCastArity => &PARSE_CAST_ARITY,
            ApiError::ParseInvalidTypeParam => &PARSE_INVALID_TYPE_PARAM,
            ApiError::ParseEmptySelect => &PARSE_EMPTY_SELECT,
            ApiError::ParseSelectMissingFrom => &PARSE_SELECT_MISSING_FROM,
            ApiError::ParseExpectedIdentForGroupName => &PARSE_EXPECTED_IDENT_FOR_GROUP_NAME,
            ApiError::ParseExpectedIdentForAlias => &PARSE_EXPECTED_IDENT_FOR_ALIAS,
            ApiError::ParseUnsupportedCallWithStar => &PARSE_UNSUPPORTED_CALL_WITH_STAR,
            ApiError::ParseNonUnaryAgregateFunctionCall => &PARSE_NON_UNARY_AGREGATE_FUNCTION_CALL,
            ApiError::ParseMalformedJoin => &PARSE_MALFORMED_JOIN,
            ApiError::ParseExpectedIdentForAt => &PARSE_EXPECTED_IDENT_FOR_AT,
            ApiError::ParseAsteriskIsNotAloneInSelectList => {
                &PARSE_ASTERISK_IS_NOT_ALONE_IN_SELECT_LIST
            }
            ApiError::ParseCannotMixSqbAndWildcardInSelectList => {
                &PARSE_CANNOT_MIX_SQB_AND_WILDCARD_IN_SELECT_LIST
            }
            ApiError::ParseInvalidContextForWildcardInSelectList => {
                &PARSE_INVALID_CONTEXT_FOR_WILDCARD_IN_SELECT_LIST
            }
            ApiError::IncorrectSQLFunctionArgumentType => &INCORRECT_SQLFUNCTION_ARGUMENT_TYPE,
            ApiError::ValueParseFailure => &VALUE_PARSE_FAILURE,
            ApiError::EvaluatorInvalidArguments => &EVALUATOR_INVALID_ARGUMENTS,
            ApiError::IntegerOverflow => &INTEGER_OVERFLOW,
            ApiError::LikeInvalidInputs => &LIKE_INVALID_INPUTS,
            ApiError::CastFailed => &CAST_FAILED,
            ApiError::InvalidCast => &INVALID_CAST,
            ApiError::EvaluatorInvalidTimestampFormatPattern => {
                &EVALUATOR_INVALID_TIMESTAMP_FORMAT_PATTERN
            }
            ApiError::EvaluatorInvalidTimestampFormatPatternSymbolForParsing => {
                &EVALUATOR_INVALID_TIMESTAMP_FORMAT_PATTERN_SYMBOL_FOR_PARSING
            }
            ApiError::EvaluatorTimestampFormatPatternDuplicateFields => {
                &EVALUATOR_TIMESTAMP_FORMAT_PATTERN_DUPLICATE_FIELDS
            }
            ApiError::EvaluatorTimestampFormatPatternHourClockAmPmMismatch => {
                &EVALUATOR_TIMESTAMP_FORMAT_PATTERN_HOUR_CLOCK_AM_PM_MISMATCH
            }
            ApiError::EvaluatorUnterminatedTimestampFormatPatternToken => {
                &EVALUATOR_UNTERMINATED_TIMESTAMP_FORMAT_PATTERN_TOKEN
            }
            ApiError::EvaluatorInvalidTimestampFormatPatternToken => {
                &EVALUATOR_INVALID_TIMESTAMP_FORMAT_PATTERN_TOKEN
            }
            ApiError::EvaluatorInvalidTimestampFormatPatternSymbol => {
                &EVALUATOR_INVALID_TIMESTAMP_FORMAT_PATTERN_SYMBOL
            }
            ApiError::EvaluatorBindingDoesNotExist => &EVALUATOR_BINDING_DOES_NOT_EXIST,
            ApiError::MissingHeaders => &MISSING_HEADERS,
            ApiError::InvalidColumnIndex => &INVALID_COLUMN_INDEX,
            ApiError::AdminConfigNotificationTargetsFailed => {
                &ADMIN_CONFIG_NOTIFICATION_TARGETS_FAILED
            }
            ApiError::AdminProfilerNotEnabled => &ADMIN_PROFILER_NOT_ENABLED,
            ApiError::InvalidDecompressedSize => &INVALID_DECOMPRESSED_SIZE,
            ApiError::AddUserInvalidArgument => &ADD_USER_INVALID_ARGUMENT,
            ApiError::AdminAccountNotEligible => &ADMIN_ACCOUNT_NOT_ELIGIBLE,
            ApiError::AccountNotEligible => &ACCOUNT_NOT_ELIGIBLE,
            ApiError::AdminServiceAccountNotFound => &ADMIN_SERVICE_ACCOUNT_NOT_FOUND,
            ApiError::PostPolicyConditionInvalidFormat => &POST_POLICY_CONDITION_INVALID_FORMAT,
        }
    }
}

const INVALID_COPY_DEST: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "This copy request is illegal because it is trying to copy an object to itself without changing the object's metadata, storage class, website redirect location or encryption attributes.",
    StatusCode::BAD_REQUEST,
);
const INVALID_COPY_SOURCE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Copy Source must mention the source bucket and key: sourcebucket/sourcekey.",
    StatusCode::BAD_REQUEST,
);
const INVALID_METADATA_DIRECTIVE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Unknown metadata directive.",
    StatusCode::BAD_REQUEST,
);
const INVALID_STORAGE_CLASS: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidStorageClass",
    "Invalid storage class.",
    StatusCode::BAD_REQUEST,
);
const INVALID_REQUEST_BODY: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Body shouldn't be set for this request.",
    StatusCode::BAD_REQUEST,
);
const INVALID_MAX_UPLOADS: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Argument max-uploads must be an integer between 0 and 2147483647",
    StatusCode::BAD_REQUEST,
);
const INVALID_MAX_KEYS: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Argument maxKeys must be an integer between 0 and 2147483647",
    StatusCode::BAD_REQUEST,
);
const INVALID_ENCODING_METHOD: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Invalid Encoding Method specified in Request",
    StatusCode::BAD_REQUEST,
);
const INVALID_MAX_PARTS: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Argument max-parts must be an integer between 0 and 2147483647",
    StatusCode::BAD_REQUEST,
);
const INVALID_PART_NUMBER_MARKER: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Argument partNumberMarker must be an integer.",
    StatusCode::BAD_REQUEST,
);
const INVALID_PART_NUMBER: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidPartNumber",
    "The requested partnumber is not satisfiable",
    StatusCode::RANGE_NOT_SATISFIABLE,
);
const INVALID_POLICY_DOCUMENT: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidPolicyDocument",
    "The content of the form does not meet the conditions specified in the policy document.",
    StatusCode::BAD_REQUEST,
);
const ACCESS_DENIED: GenericApiErrorConst =
    GenericApiErrorConst::new("AccessDenied", "Access Denied.", StatusCode::FORBIDDEN);
const BAD_DIGEST: GenericApiErrorConst = GenericApiErrorConst::new(
    "BadDigest",
    "The Content-Md5 you specified did not match what we received.",
    StatusCode::BAD_REQUEST,
);
const ENTITY_TOO_SMALL: GenericApiErrorConst = GenericApiErrorConst::new(
    "EntityTooSmall",
    "Your proposed upload is smaller than the minimum allowed object size.",
    StatusCode::BAD_REQUEST,
);
const ENTITY_TOO_LARGE: GenericApiErrorConst = GenericApiErrorConst::new(
    "EntityTooLarge",
    "Your proposed upload exceeds the maximum allowed object size.",
    StatusCode::BAD_REQUEST,
);
const POLICY_TOO_LARGE: GenericApiErrorConst = GenericApiErrorConst::new(
    "PolicyTooLarge",
    "Policy exceeds the maximum allowed document size.",
    StatusCode::BAD_REQUEST,
);
const INCOMPLETE_BODY: GenericApiErrorConst = GenericApiErrorConst::new(
    "IncompleteBody",
    "You did not provide the number of bytes specified by the Content-Length HTTP header.",
    StatusCode::BAD_REQUEST,
);
const INTERNAL_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "InternalError",
    "We encountered an internal error, please try again.",
    StatusCode::INTERNAL_SERVER_ERROR,
);
const INVALID_ACCESS_KEY_ID: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidAccessKeyId",
    "The Access Key Id you provided does not exist in our records.",
    StatusCode::FORBIDDEN,
);
const INVALID_BUCKET_NAME: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidBucketName",
    "The specified bucket is not valid.",
    StatusCode::BAD_REQUEST,
);
const INVALID_DIGEST: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidDigest",
    "The Content-Md5 you specified is not valid.",
    StatusCode::BAD_REQUEST,
);
const INVALID_RANGE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRange",
    "The requested range is not satisfiable",
    StatusCode::RANGE_NOT_SATISFIABLE,
);
const INVALID_RANGE_PART_NUMBER: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "Cannot specify both Range header and partNumber query parameter",
    StatusCode::BAD_REQUEST,
);
const MALFORMED_XML: GenericApiErrorConst = GenericApiErrorConst::new(
    "MalformedXML",
    "The XML you provided was not well-formed or did not validate against our published schema.",
    StatusCode::BAD_REQUEST,
);
const MISSING_CONTENT_LENGTH: GenericApiErrorConst = GenericApiErrorConst::new(
    "MissingContentLength",
    "You must provide the Content-Length HTTP header.",
    StatusCode::LENGTH_REQUIRED,
);
const MISSING_CONTENT_MD5: GenericApiErrorConst = GenericApiErrorConst::new(
    "MissingContentMD5",
    "Missing required header for this request: Content-Md5.",
    StatusCode::BAD_REQUEST,
);
const MISSING_SECURITY_HEADER: GenericApiErrorConst = GenericApiErrorConst::new(
    "MissingSecurityHeader",
    "Your request was missing a required header",
    StatusCode::BAD_REQUEST,
);
const MISSING_REQUEST_BODY_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "MissingRequestBodyError",
    "Request body is empty.",
    StatusCode::LENGTH_REQUIRED,
);
const NO_SUCH_BUCKET: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchBucket",
    "The specified bucket does not exist",
    StatusCode::NOT_FOUND,
);
const NO_SUCH_BUCKET_POLICY: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchBucketPolicy",
    "The bucket policy does not exist",
    StatusCode::NOT_FOUND,
);
const NO_SUCH_BUCKET_LIFECYCLE: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchBucketLifecycle",
    "The bucket lifecycle configuration does not exist",
    StatusCode::NOT_FOUND,
);
const NO_SUCH_LIFECYCLE_CONFIGURATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchLifecycleConfiguration",
    "The lifecycle configuration does not exist",
    StatusCode::NOT_FOUND,
);
const NO_SUCH_BUCKET_SSE_CONFIG: GenericApiErrorConst = GenericApiErrorConst::new(
    "ServerSideEncryptionConfigurationNotFoundError",
    "The server side encryption configuration was not found",
    StatusCode::NOT_FOUND,
);
const NO_SUCH_KEY: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchKey",
    "The specified key does not exist.",
    StatusCode::NOT_FOUND,
);
const NO_SUCH_UPLOAD: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchUpload",
    "The specified multipart upload does not exist. The upload ID may be invalid, or the upload may have been aborted or completed.",
    StatusCode::NOT_FOUND,
);
const INVALID_VERSION_ID: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Invalid version id specified",
    StatusCode::BAD_REQUEST,
);
const NO_SUCH_VERSION: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchVersion",
    "The specified version does not exist.",
    StatusCode::NOT_FOUND,
);
const NOT_IMPLEMENTED: GenericApiErrorConst = GenericApiErrorConst::new(
    "NotImplemented",
    "A header you provided implies functionality that is not implemented",
    StatusCode::NOT_IMPLEMENTED,
);
const PRECONDITION_FAILED: GenericApiErrorConst = GenericApiErrorConst::new(
    "PreconditionFailed",
    "At least one of the pre-conditions you specified did not hold",
    StatusCode::PRECONDITION_FAILED,
);
const REQUEST_TIME_TOO_SKEWED: GenericApiErrorConst = GenericApiErrorConst::new(
    "RequestTimeTooSkewed",
    "The difference between the request time and the server's time is too large.",
    StatusCode::FORBIDDEN,
);
const SIGNATURE_DOES_NOT_MATCH: GenericApiErrorConst = GenericApiErrorConst::new(
    "SignatureDoesNotMatch",
    "The request signature we calculated does not match the signature you provided. Check your key and signing method.",
    StatusCode::FORBIDDEN,
);
const METHOD_NOT_ALLOWED: GenericApiErrorConst = GenericApiErrorConst::new(
    "MethodNotAllowed",
    "The specified method is not allowed against this resource.",
    StatusCode::METHOD_NOT_ALLOWED,
);
const INVALID_PART: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidPart",
    "One or more of the specified parts could not be found.  The part may not have been uploaded, or the specified entity tag may not match the part's entity tag.",
    StatusCode::BAD_REQUEST,
);
const INVALID_PART_ORDER: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidPartOrder",
    "The list of parts was not in ascending order. The parts list must be specified in order by part number.",
    StatusCode::BAD_REQUEST,
);
const INVALID_OBJECT_STATE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidObjectState",
    "The operation is not valid for the current state of the object.",
    StatusCode::FORBIDDEN,
);
const AUTHORIZATION_HEADER_MALFORMED: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationHeaderMalformed",
    "The authorization header is malformed; the region is wrong; expecting 'us-east-1'.",
    StatusCode::BAD_REQUEST,
);
const MALFORMED_POSTREQUEST: GenericApiErrorConst = GenericApiErrorConst::new(
    "MalformedPOSTRequest",
    "The body of your POST request is not well-formed multipart/form-data.",
    StatusCode::BAD_REQUEST,
);
const POST_FILE_REQUIRED: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "POST requires exactly one file upload per request.",
    StatusCode::BAD_REQUEST,
);
const SIGNATURE_VERSION_NOT_SUPPORTED: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "The authorization mechanism you have provided is not supported. Please use AWS4-HMAC-SHA256.",
    StatusCode::BAD_REQUEST,
);
const BUCKET_NOT_EMPTY: GenericApiErrorConst = GenericApiErrorConst::new(
    "BucketNotEmpty",
    "The bucket you tried to delete is not empty",
    StatusCode::CONFLICT,
);
const BUCKET_ALREADY_EXISTS: GenericApiErrorConst = GenericApiErrorConst::new(
    "BucketAlreadyExists",
    "The requested bucket name is not available. The bucket namespace is shared by all users of the system. Please select a different name and try again.",
    StatusCode::CONFLICT,
);
const ALL_ACCESS_DISABLED: GenericApiErrorConst = GenericApiErrorConst::new(
    "AllAccessDisabled",
    "All access to this bucket has been disabled.",
    StatusCode::FORBIDDEN,
);
const MALFORMED_POLICY: GenericApiErrorConst = GenericApiErrorConst::new(
    "MalformedPolicy",
    "Policy has invalid resource.",
    StatusCode::BAD_REQUEST,
);
const MISSING_FIELDS: GenericApiErrorConst = GenericApiErrorConst::new(
    "MissingFields",
    "Missing fields in request.",
    StatusCode::BAD_REQUEST,
);
const MISSING_CRED_TAG: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "Missing Credential field for this request.",
    StatusCode::BAD_REQUEST,
);
const CRED_MALFORMED: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationQueryParametersError",
    "Error parsing the X-Amz-Credential parameter; the Credential is mal-formed; expecting \"<YOUR-AKID>/YYYYMMDD/REGION/SERVICE/aws4_request\".",
    StatusCode::BAD_REQUEST,
);
const MALFORMED_DATE: GenericApiErrorConst = GenericApiErrorConst::new(
    "MalformedDate",
    "Invalid date format header, expected to be in ISO8601, RFC1123 or RFC1123Z time format.",
    StatusCode::BAD_REQUEST,
);
const MALFORMED_PRESIGNED_DATE: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationQueryParametersError",
    "X-Amz-Date must be in the ISO8601 Long Format \"yyyyMMdd'T'HHmmss'Z'\"",
    StatusCode::BAD_REQUEST,
);
const MALFORMED_CREDENTIAL_DATE: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationQueryParametersError",
    "Error parsing the X-Amz-Credential parameter; incorrect date format. This date in the credential must be in the format \"yyyyMMdd\".",
    StatusCode::BAD_REQUEST,
);
const INVALID_REGION: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRegion",
    "Region does not match.",
    StatusCode::BAD_REQUEST,
);
const INVALID_SERVICE_S3: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationParametersError",
    "Error parsing the Credential/X-Amz-Credential parameter; incorrect service. This endpoint belongs to \"s3\".",
    StatusCode::BAD_REQUEST,
);
const INVALID_SERVICE_STS: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationParametersError",
    "Error parsing the Credential parameter; incorrect service. This endpoint belongs to \"sts\".",
    StatusCode::BAD_REQUEST,
);
const INVALID_REQUEST_VERSION: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationQueryParametersError",
    "Error parsing the X-Amz-Credential parameter; incorrect terminal. This endpoint uses \"aws4_request\".",
    StatusCode::BAD_REQUEST,
);
const MISSING_SIGN_TAG: GenericApiErrorConst = GenericApiErrorConst::new(
    "AccessDenied",
    "Signature header missing Signature field.",
    StatusCode::BAD_REQUEST,
);
const MISSING_SIGN_HEADERS_TAG: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Signature header missing SignedHeaders field.",
    StatusCode::BAD_REQUEST,
);
const MALFORMED_EXPIRES: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationQueryParametersError",
    "X-Amz-Expires should be a number",
    StatusCode::BAD_REQUEST,
);
const NEGATIVE_EXPIRES: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationQueryParametersError",
    "X-Amz-Expires must be non-negative",
    StatusCode::BAD_REQUEST,
);
const AUTH_HEADER_EMPTY: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Authorization header is invalid -- one and only one ' ' (space) required.",
    StatusCode::BAD_REQUEST,
);
const MISSING_DATE_HEADER: GenericApiErrorConst = GenericApiErrorConst::new(
    "AccessDenied",
    "AWS authentication requires a valid Date or x-amz-date header",
    StatusCode::BAD_REQUEST,
);
const INVALID_QUERY_SIGNATURE_ALGO: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationQueryParametersError",
    "X-Amz-Algorithm only supports \"AWS4-HMAC-SHA256\".",
    StatusCode::BAD_REQUEST,
);
const EXPIRED_PRESIGN_REQUEST: GenericApiErrorConst =
    GenericApiErrorConst::new("AccessDenied", "Request has expired", StatusCode::FORBIDDEN);
const REQUEST_NOT_READY_YET: GenericApiErrorConst = GenericApiErrorConst::new(
    "AccessDenied",
    "Request is not valid yet",
    StatusCode::FORBIDDEN,
);
const SLOW_DOWN: GenericApiErrorConst = GenericApiErrorConst::new(
    "SlowDown",
    "Resource requested is unreadable, please reduce your request rate",
    StatusCode::SERVICE_UNAVAILABLE,
);
const INVALID_PREFIX_MARKER: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidPrefixMarker",
    "Invalid marker prefix combination",
    StatusCode::BAD_REQUEST,
);
const BAD_REQUEST: GenericApiErrorConst =
    GenericApiErrorConst::new("BadRequest", "400 BadRequest", StatusCode::BAD_REQUEST);
const KEY_TOO_LONG_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "KeyTooLongError",
    "Your key is too long",
    StatusCode::BAD_REQUEST,
);
const UNSIGNED_HEADERS: GenericApiErrorConst = GenericApiErrorConst::new(
    "AccessDenied",
    "There were headers present in the request which were not signed",
    StatusCode::BAD_REQUEST,
);
const INVALID_QUERY_PARAMS: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationQueryParametersError",
    "Query-string authentication version 4 requires the X-Amz-Algorithm, X-Amz-Credential, X-Amz-Signature, X-Amz-Date, X-Amz-SignedHeaders, and X-Amz-Expires parameters.",
    StatusCode::BAD_REQUEST,
);
const BUCKET_ALREADY_OWNED_BY_YOU: GenericApiErrorConst = GenericApiErrorConst::new(
    "BucketAlreadyOwnedByYou",
    "Your previous request to create the named bucket succeeded and you already own it.",
    StatusCode::CONFLICT,
);
const INVALID_DURATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidDuration",
    "Duration provided in the request is invalid.",
    StatusCode::BAD_REQUEST,
);
const INVALID_BUCKET_OBJECT_LOCK_CONFIGURATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "Bucket is missing ObjectLockConfiguration",
    StatusCode::BAD_REQUEST,
);
const BUCKET_TAGGING_NOT_FOUND: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchTagSet",
    "The TagSet does not exist",
    StatusCode::NOT_FOUND,
);
const OBJECT_LOCK_CONFIGURATION_NOT_FOUND: GenericApiErrorConst = GenericApiErrorConst::new(
    "ObjectLockConfigurationNotFoundError",
    "Object Lock configuration does not exist for this bucket",
    StatusCode::NOT_FOUND,
);
const OBJECT_LOCK_CONFIGURATION_NOT_ALLOWED: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidBucketState",
    "Object Lock configuration cannot be enabled on existing buckets",
    StatusCode::CONFLICT,
);
const NO_SUCH_CORS_CONFIGURATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchCORSConfiguration",
    "The CORS configuration does not exist",
    StatusCode::NOT_FOUND,
);
const NO_SUCH_WEBSITE_CONFIGURATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchWebsiteConfiguration",
    "The specified bucket does not have a website configuration",
    StatusCode::NOT_FOUND,
);
const REPLICATION_CONFIGURATION_NOT_FOUND_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "ReplicationConfigurationNotFoundError",
    "The replication configuration was not found",
    StatusCode::NOT_FOUND,
);
const REMOTE_DESTINATION_NOT_FOUND_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "RemoteDestinationNotFoundError",
    "The remote destination bucket does not exist",
    StatusCode::NOT_FOUND,
);
const REPLICATION_DESTINATION_MISSING_LOCK: GenericApiErrorConst = GenericApiErrorConst::new(
    "ReplicationDestinationMissingLockError",
    "The replication destination bucket does not have object locking enabled",
    StatusCode::BAD_REQUEST,
);
const REMOTE_TARGET_NOT_FOUND_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminRemoteTargetNotFoundError",
    "The remote target does not exist",
    StatusCode::NOT_FOUND,
);
const REPLICATION_REMOTE_CONNECTION_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminReplicationRemoteConnectionError",
    "Remote service connection error - please check remote service credentials and target bucket",
    StatusCode::NOT_FOUND,
);
const REPLICATION_NO_MATCHING_RULE_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkReplicationNoMatchingRule",
    "No matching replication rule found for this object prefix",
    StatusCode::BAD_REQUEST,
);
const BUCKET_REMOTE_IDENTICAL_TO_SOURCE: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminRemoteIdenticalToSource",
    "The remote target cannot be identical to source",
    StatusCode::BAD_REQUEST,
);
const BUCKET_REMOTE_ALREADY_EXISTS: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminBucketRemoteAlreadyExists",
    "The remote target already exists",
    StatusCode::BAD_REQUEST,
);
const BUCKET_REMOTE_LABEL_IN_USE: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminBucketRemoteLabelInUse",
    "The remote target with this label already exists",
    StatusCode::BAD_REQUEST,
);
const BUCKET_REMOTE_REMOVE_DISALLOWED: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminRemoteRemoveDisallowed",
    "This ARN is in use by an existing configuration",
    StatusCode::BAD_REQUEST,
);
const BUCKET_REMOTE_ARN_TYPE_INVALID: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminRemoteARNTypeInvalid",
    "The bucket remote ARN type is not valid",
    StatusCode::BAD_REQUEST,
);
const BUCKET_REMOTE_ARN_INVALID: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminRemoteArnInvalid",
    "The bucket remote ARN does not have correct format",
    StatusCode::BAD_REQUEST,
);
const REMOTE_TARGET_NOT_VERSIONED_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "RemoteTargetNotVersionedError",
    "The remote target does not have versioning enabled",
    StatusCode::BAD_REQUEST,
);
const REPLICATION_SOURCE_NOT_VERSIONED_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "ReplicationSourceNotVersionedError",
    "The replication source does not have versioning enabled",
    StatusCode::BAD_REQUEST,
);
const REPLICATION_NEEDS_VERSIONING_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "Versioning must be 'Enabled' on the bucket to apply a replication configuration",
    StatusCode::BAD_REQUEST,
);
const REPLICATION_BUCKET_NEEDS_VERSIONING_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "Versioning must be 'Enabled' on the bucket to add a replication target",
    StatusCode::BAD_REQUEST,
);
const NO_SUCH_OBJECT_LOCK_CONFIGURATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "NoSuchObjectLockConfiguration",
    "The specified object does not have a ObjectLock configuration",
    StatusCode::BAD_REQUEST,
);
const OBJECT_LOCKED: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "Object is WORM protected and cannot be overwritten",
    StatusCode::BAD_REQUEST,
);
const INVALID_RETENTION_DATE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "Date must be provided in ISO 8601 format",
    StatusCode::BAD_REQUEST,
);
const PAST_OBJECT_LOCK_RETAIN_DATE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "the retain until date must be in the future",
    StatusCode::BAD_REQUEST,
);
const UNKNOWN_WORMMODE_DIRECTIVE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "unknown wormMode directive",
    StatusCode::BAD_REQUEST,
);
const OBJECT_LOCK_INVALID_HEADERS: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "x-amz-object-lock-retain-until-date and x-amz-object-lock-mode must both be supplied",
    StatusCode::BAD_REQUEST,
);
const OBJECT_RESTORE_ALREADY_IN_PROGRESS: GenericApiErrorConst = GenericApiErrorConst::new(
    "RestoreAlreadyInProgress",
    "Object restore is already in progress",
    StatusCode::CONFLICT,
);
const TRANSITION_STORAGE_CLASS_NOT_FOUND_ERROR: GenericApiErrorConst = GenericApiErrorConst::new(
    "TransitionStorageClassNotFoundError",
    "The transition storage class was not found",
    StatusCode::NOT_FOUND,
);

/// Bucket notification related errors.
const EVENT_NOTIFICATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "A specified event is not supported for notifications.",
    StatusCode::BAD_REQUEST,
);
const ARN_NOTIFICATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "A specified destination ARN does not exist or is not well-formed. Verify the destination ARN.",
    StatusCode::BAD_REQUEST,
);
const REGION_NOTIFICATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "A specified destination is in a different region than the bucket. You must use a destination that resides in the same region as the bucket.",
    StatusCode::BAD_REQUEST,
);
const OVERLAPPING_FILTER_NOTIFICATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "An object key name filtering rule defined with overlapping prefixes, overlapping suffixes, or overlapping combinations of prefixes and suffixes for the same event types.",
    StatusCode::BAD_REQUEST,
);
const FILTER_NAME_INVALID: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "filter rule name must be either prefix or suffix",
    StatusCode::BAD_REQUEST,
);
const FILTER_NAME_PREFIX: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Cannot specify more than one prefix rule in a filter.",
    StatusCode::BAD_REQUEST,
);
const FILTER_NAME_SUFFIX: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Cannot specify more than one suffix rule in a filter.",
    StatusCode::BAD_REQUEST,
);
const FILTER_VALUE_INVALID: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Size of filter rule value cannot exceed 1024 bytes in UTF-8 representation",
    StatusCode::BAD_REQUEST,
);
const OVERLAPPING_CONFIGS: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Configurations overlap. Configurations on the same bucket cannot share a common event type.",
    StatusCode::BAD_REQUEST,
);
const UNSUPPORTED_NOTIFICATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "UnsupportedNotification",
    "Hulk server does not support Topic or Cloud Function based notifications.",
    StatusCode::BAD_REQUEST,
);
const INVALID_COPY_PART_RANGE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "The x-amz-copy-source-range value must be of the form bytes=first-last where first and last are the zero-based offsets of the first and last bytes to copy",
    StatusCode::BAD_REQUEST,
);
const INVALID_COPY_PART_RANGE_SOURCE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Range specified is not valid for source object",
    StatusCode::BAD_REQUEST,
);
const METADATA_TOO_LARGE: GenericApiErrorConst = GenericApiErrorConst::new(
    "MetadataTooLarge",
    "Your metadata headers exceed the maximum allowed metadata size.",
    StatusCode::BAD_REQUEST,
);
const INVALID_TAG_DIRECTIVE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Unknown tag directive.",
    StatusCode::BAD_REQUEST,
);
const INVALID_ENCRYPTION_METHOD: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "The encryption method specified is not supported",
    StatusCode::BAD_REQUEST,
);
const INSECURE_SSECUSTOMER_REQUEST: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "Requests specifying Server Side Encryption with Customer provided keys must be made over a secure connection.",
    StatusCode::BAD_REQUEST,
);
const SSEMULTIPART_ENCRYPTED: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "The multipart upload initiate requested encryption. Subsequent part requests must include the appropriate encryption parameters.",
    StatusCode::BAD_REQUEST,
);
const SSEENCRYPTED_OBJECT: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "The object was stored using a form of Server Side Encryption. The correct parameters must be provided to retrieve the object.",
    StatusCode::BAD_REQUEST,
);
const INVALID_ENCRYPTION_PARAMETERS: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequest",
    "The encryption parameters are not applicable to this object.",
    StatusCode::BAD_REQUEST,
);
const INVALID_SSECUSTOMER_ALGORITHM: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Requests specifying Server Side Encryption with Customer provided keys must provide a valid encryption algorithm.",
    StatusCode::BAD_REQUEST,
);
const INVALID_SSECUSTOMER_KEY: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "The secret key was invalid for the specified algorithm.",
    StatusCode::BAD_REQUEST,
);
const MISSING_SSECUSTOMER_KEY: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Requests specifying Server Side Encryption with Customer provided keys must provide an appropriate secret key.",
    StatusCode::BAD_REQUEST,
);
const MISSING_SSE_CUSTOMER_KEY_MD5: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Requests specifying Server Side Encryption with Customer provided keys must provide the client calculated MD5 of the secret key.",
    StatusCode::BAD_REQUEST,
);
const SSE_CUSTOMER_KEY_MD5_MISMATCH: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "The calculated MD5 hash of the key did not match the hash that was provided.",
    StatusCode::BAD_REQUEST,
);
const INVALID_SSE_CUSTOMER_PARAMETERS: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "The provided encryption parameters did not match the ones used originally.",
    StatusCode::BAD_REQUEST,
);
const INCOMPATIBLE_ENCRYPTION_METHOD: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Server side encryption specified with both SSE-C and SSE-S3 headers",
    StatusCode::BAD_REQUEST,
);
const KMSNOT_CONFIGURED: GenericApiErrorConst = GenericApiErrorConst::new(
    "NotImplemented",
    "Server side encryption specified but KMS is not configured",
    StatusCode::NOT_IMPLEMENTED,
);
const NO_ACCESS_KEY: GenericApiErrorConst = GenericApiErrorConst::new(
    "AccessDenied",
    "No AWSAccessKey was presented",
    StatusCode::FORBIDDEN,
);
const INVALID_TOKEN: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidTokenId",
    "The security token included in the request is invalid",
    StatusCode::FORBIDDEN,
);

/// S3 extensions.
const CONTENT_SHA256_MISMATCH: GenericApiErrorConst = GenericApiErrorConst::new(
    "XAmzContentSHA256Mismatch",
    "The provided 'x-amz-content-sha256' header does not match what was computed.",
    StatusCode::BAD_REQUEST,
);

/// Hulk extensions.
const STORAGE_FULL: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkStorageFull",
    "Storage backend has reached its minimum free disk threshold. Please delete a few objects to proceed.",
    StatusCode::INSUFFICIENT_STORAGE,
);
const PARENT_IS_OBJECT: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkParentIsObject",
    "Object-prefix is already an object, please choose a different object-prefix name.",
    StatusCode::BAD_REQUEST,
);
const REQUEST_BODY_PARSE: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkRequestBodyParse",
    "The request body failed to parse.",
    StatusCode::BAD_REQUEST,
);
const OBJECT_EXISTS_AS_DIRECTORY: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkObjectExistsAsDirectory",
    "Object name already exists as a directory.",
    StatusCode::CONFLICT,
);
const INVALID_OBJECT_NAME: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkInvalidObjectName",
    "Object name contains unsupported characters.",
    StatusCode::BAD_REQUEST,
);
const INVALID_OBJECT_NAME_PREFIX_SLASH: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkInvalidObjectName",
    "Object name contains a leading slash.",
    StatusCode::BAD_REQUEST,
);
const INVALID_RESOURCE_NAME: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkInvalidResourceName",
    "Resource name contains bad components such as \"..\" or \".\".",
    StatusCode::BAD_REQUEST,
);
const SERVER_NOT_INITIALIZED: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkServerNotInitialized",
    "Server not initialized, please try again.",
    StatusCode::SERVICE_UNAVAILABLE,
);
const MALFORMED_JSON: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkMalformedJSON",
    "The JSON you provided was not well-formed or did not validate against our published format.",
    StatusCode::BAD_REQUEST,
);
const ADMIN_NO_SUCH_USER: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminNoSuchUser",
    "The specified user does not exist.",
    StatusCode::NOT_FOUND,
);
const ADMIN_NO_SUCH_GROUP: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminNoSuchGroup",
    "The specified group does not exist.",
    StatusCode::NOT_FOUND,
);
const ADMIN_GROUP_NOT_EMPTY: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminGroupNotEmpty",
    "The specified group is not empty - cannot remove it.",
    StatusCode::BAD_REQUEST,
);
const ADMIN_NO_SUCH_POLICY: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminNoSuchPolicy",
    "The canned policy does not exist.",
    StatusCode::NOT_FOUND,
);
const ADMIN_INVALID_ARGUMENT: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminInvalidArgument",
    "Invalid arguments specified.",
    StatusCode::BAD_REQUEST,
);
const ADMIN_INVALID_ACCESS_KEY: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminInvalidAccessKey",
    "The access key is invalid.",
    StatusCode::BAD_REQUEST,
);
const ADMIN_INVALID_SECRET_KEY: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminInvalidSecretKey",
    "The secret key is invalid.",
    StatusCode::BAD_REQUEST,
);
const ADMIN_CONFIG_NO_QUORUM: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminConfigNoQuorum",
    "Configuration update failed because server quorum was not met",
    StatusCode::SERVICE_UNAVAILABLE,
);
const ADMIN_CONFIG_TOO_LARGE: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminConfigTooLarge",
    formatcp!(
        "Configuration data provided exceeds the allowed maximum of {} bytes",
        crate::config::MAX_CONFIG_JSON_SIZE,
    ),
    StatusCode::BAD_REQUEST,
);
const ADMIN_CONFIG_BAD_JSON: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminConfigBadJSON",
    "JSON configuration provided is of incorrect format",
    StatusCode::BAD_REQUEST,
);
const ADMIN_CONFIG_DUPLICATE_KEYS: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminConfigDuplicateKeys",
    "JSON configuration provided has objects with duplicate keys",
    StatusCode::BAD_REQUEST,
);
const ADMIN_CONFIG_NOTIFICATION_TARGETS_FAILED: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminNotificationTargetsTestFailed",
    "Configuration update failed due an unsuccessful attempt to connect to one or more notification servers",
    StatusCode::BAD_REQUEST,
);
const ADMIN_PROFILER_NOT_ENABLED: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminProfilerNotEnabled",
    "Unable to perform the requested operation because profiling is not enabled",
    StatusCode::BAD_REQUEST,
);
const ADMIN_CREDENTIALS_MISMATCH: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminCredentialsMismatch",
    "Credentials in config mismatch with server environment variables",
    StatusCode::SERVICE_UNAVAILABLE,
);
const ADMIN_BUCKET_QUOTA_EXCEEDED: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminBucketQuotaExceeded",
    "Bucket quota exceeded",
    StatusCode::BAD_REQUEST,
);
const ADMIN_NO_SUCH_QUOTA_CONFIGURATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkAdminNoSuchQuotaConfiguration",
    "The quota configuration does not exist",
    StatusCode::NOT_FOUND,
);
const INSECURE_CLIENT_REQUEST: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkInsecureClientRequest",
    "Cannot respond to plain-text request from TLS-encrypted server",
    StatusCode::BAD_REQUEST,
);
const OPERATION_TIMED_OUT: GenericApiErrorConst = GenericApiErrorConst::new(
    "RequestTimeout",
    "A timeout occurred while trying to lock a resource, please reduce your request rate",
    StatusCode::SERVICE_UNAVAILABLE,
);
lazy_static! {
    static ref CLIENT_DISCONNECTED: GenericApiErrorConst = GenericApiErrorConst {
        code: "ClientDisconnected",
        description: "Client disconnected before response was ready",
        http_status_code: StatusCode::from_u16(499).unwrap(), // No official code, use nginx value.
    };
}
const OPERATION_MAXED_OUT: GenericApiErrorConst = GenericApiErrorConst::new(
    "SlowDown",
    "A timeout exceeded while waiting to proceed with the request, please reduce your request rate",
    StatusCode::SERVICE_UNAVAILABLE,
);
const UNSUPPORTED_METADATA: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "Your metadata headers are not supported.",
    StatusCode::BAD_REQUEST,
);
const OBJECT_TAMPERED: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkObjectTampered",
    super::ERR_OBJECT_TAMPERED,
    StatusCode::PARTIAL_CONTENT,
);
const MAXIMUM_EXPIRES: GenericApiErrorConst = GenericApiErrorConst::new(
    "AuthorizationQueryParametersError",
    "X-Amz-Expires must be less than a week (in seconds); that is, the given X-Amz-Expires must be less than 604800 seconds",
    StatusCode::BAD_REQUEST,
);

// Generic Invalid-Request error. Should be used for response errors only for unlikely
// corner case errors for which introducing new APIErrorCode is not worth it. LogIf()
// should be used to log the error at the source of the error for debugging purposes.
const INVALID_REQUEST: GenericApiErrorConst =
    GenericApiErrorConst::new("InvalidRequest", "Invalid Request", StatusCode::BAD_REQUEST);
const HEAL_NOT_IMPLEMENTED: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkHealNotImplemented",
    "This server does not implement heal functionality.",
    StatusCode::BAD_REQUEST,
);
const HEAL_NO_SUCH_PROCESS: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkHealNoSuchProcess",
    "No such heal process is running on the server",
    StatusCode::BAD_REQUEST,
);
const HEAL_INVALID_CLIENT_TOKEN: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkHealInvalidClientToken",
    "Client token mismatch",
    StatusCode::BAD_REQUEST,
);
const HEAL_MISSING_BUCKET: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkHealMissingBucket",
    "A heal start request with a non-empty object-prefix parameter requires a bucket to be specified.",
    StatusCode::BAD_REQUEST,
);
const HEAL_ALREADY_RUNNING: GenericApiErrorConst =
    GenericApiErrorConst::new("XHulkHealAlreadyRunning", "", StatusCode::BAD_REQUEST);
const HEAL_OVERLAPPING_PATHS: GenericApiErrorConst =
    GenericApiErrorConst::new("XHulkHealOverlappingPaths", "", StatusCode::BAD_REQUEST);
const BACKEND_DOWN: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkBackendDown",
    "Object storage backend is unreachable",
    StatusCode::SERVICE_UNAVAILABLE,
);
const INCORRECT_CONTINUATION_TOKEN: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidArgument",
    "The continuation token provided is incorrect",
    StatusCode::BAD_REQUEST,
);
//S3 Select API Errors
const EMPTY_REQUEST_BODY: GenericApiErrorConst = GenericApiErrorConst::new(
    "EmptyRequestBody",
    "Request body cannot be empty.",
    StatusCode::BAD_REQUEST,
);
const UNSUPPORTED_FUNCTION: GenericApiErrorConst = GenericApiErrorConst::new(
    "UnsupportedFunction",
    "Encountered an unsupported SQL function.",
    StatusCode::BAD_REQUEST,
);
const INVALID_DATA_SOURCE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidDataSource",
    "Invalid data source type. Only CSV and JSON are supported at this time.",
    StatusCode::BAD_REQUEST,
);
const INVALID_EXPRESSION_TYPE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidExpressionType",
    "The ExpressionType is invalid. Only SQL expressions are supported at this time.",
    StatusCode::BAD_REQUEST,
);
const BUSY: GenericApiErrorConst = GenericApiErrorConst::new(
    "Busy",
    "The service is unavailable. Please retry.",
    StatusCode::SERVICE_UNAVAILABLE,
);
const UNAUTHORIZED_ACCESS: GenericApiErrorConst = GenericApiErrorConst::new(
    "UnauthorizedAccess",
    "You are not authorized to perform this operation",
    StatusCode::UNAUTHORIZED,
);
const EXPRESSION_TOO_LONG: GenericApiErrorConst = GenericApiErrorConst::new(
    "ExpressionTooLong",
    "The SQL expression is too long: The maximum byte-length for the SQL expression is 256 KB.",
    StatusCode::BAD_REQUEST,
);
const ILLEGAL_SQLFUNCTION_ARGUMENT: GenericApiErrorConst = GenericApiErrorConst::new(
    "IllegalSqlFunctionArgument",
    "Illegal argument was used in the SQL function.",
    StatusCode::BAD_REQUEST,
);
const INVALID_KEY_PATH: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidKeyPath",
    "Key path in the SQL expression is invalid.",
    StatusCode::BAD_REQUEST,
);
const INVALID_COMPRESSION_FORMAT: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidCompressionFormat",
    "The file is not in a supported compression format. Only GZIP is supported at this time.",
    StatusCode::BAD_REQUEST,
);
const INVALID_FILE_HEADER_INFO: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidFileHeaderInfo",
    "The FileHeaderInfo is invalid. Only NONE, USE, and IGNORE are supported.",
    StatusCode::BAD_REQUEST,
);
const INVALID_JSONTYPE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidJsonType",
    "The JsonType is invalid. Only DOCUMENT and LINES are supported at this time.",
    StatusCode::BAD_REQUEST,
);
const INVALID_QUOTE_FIELDS: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidQuoteFields",
    "The QuoteFields is invalid. Only ALWAYS and ASNEEDED are supported.",
    StatusCode::BAD_REQUEST,
);
const INVALID_REQUEST_PARAMETER: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidRequestParameter",
    "The value of a parameter in SelectRequest element is invalid. Check the service API documentation and try again.",
    StatusCode::BAD_REQUEST,
);
const INVALID_DATA_TYPE: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidDataType",
    "The SQL expression contains an invalid data type.",
    StatusCode::BAD_REQUEST,
);
const INVALID_TEXT_ENCODING: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidTextEncoding",
    "Invalid encoding type. Only UTF-8 encoding is supported at this time.",
    StatusCode::BAD_REQUEST,
);
const INVALID_TABLE_ALIAS: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidTableAlias",
    "The SQL expression contains an invalid table alias.",
    StatusCode::BAD_REQUEST,
);
const MISSING_REQUIRED_PARAMETER: GenericApiErrorConst = GenericApiErrorConst::new(
    "MissingRequiredParameter",
    "The SelectRequest entity is missing a required parameter. Check the service documentation and try again.",
    StatusCode::BAD_REQUEST,
);
const OBJECT_SERIALIZATION_CONFLICT: GenericApiErrorConst = GenericApiErrorConst::new(
    "ObjectSerializationConflict",
    "The SelectRequest entity can only contain one of CSV or JSON. Check the service documentation and try again.",
    StatusCode::BAD_REQUEST,
);
const UNSUPPORTED_SQLOPERATION: GenericApiErrorConst = GenericApiErrorConst::new(
    "UnsupportedSqlOperation",
    "Encountered an unsupported SQL operation.",
    StatusCode::BAD_REQUEST,
);
const UNSUPPORTED_SQLSTRUCTURE: GenericApiErrorConst = GenericApiErrorConst::new(
    "UnsupportedSqlStructure",
    "Encountered an unsupported SQL structure. Check the SQL Reference.",
    StatusCode::BAD_REQUEST,
);
const UNSUPPORTED_SYNTAX: GenericApiErrorConst = GenericApiErrorConst::new(
    "UnsupportedSyntax",
    "Encountered invalid syntax.",
    StatusCode::BAD_REQUEST,
);
const UNSUPPORTED_RANGE_HEADER: GenericApiErrorConst = GenericApiErrorConst::new(
    "UnsupportedRangeHeader",
    "Range header is not supported for this operation.",
    StatusCode::BAD_REQUEST,
);
const LEXER_INVALID_CHAR: GenericApiErrorConst = GenericApiErrorConst::new(
    "LexerInvalidChar",
    "The SQL expression contains an invalid character.",
    StatusCode::BAD_REQUEST,
);
const LEXER_INVALID_OPERATOR: GenericApiErrorConst = GenericApiErrorConst::new(
    "LexerInvalidOperator",
    "The SQL expression contains an invalid literal.",
    StatusCode::BAD_REQUEST,
);
const LEXER_INVALID_LITERAL: GenericApiErrorConst = GenericApiErrorConst::new(
    "LexerInvalidLiteral",
    "The SQL expression contains an invalid operator.",
    StatusCode::BAD_REQUEST,
);
const LEXER_INVALID_IONLITERAL: GenericApiErrorConst = GenericApiErrorConst::new(
    "LexerInvalidIONLiteral",
    "The SQL expression contains an invalid operator.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_DATE_PART: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedDatePart",
    "Did not find the expected date part in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_KEYWORD: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedKeyword",
    "Did not find the expected keyword in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_TOKEN_TYPE: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedTokenType",
    "Did not find the expected token in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_2_TOKEN_TYPE: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpected2TokenTypes",
    "Did not find the expected token in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_NUMBER: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedNumber",
    "Did not find the expected number in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_RIGHT_PAREN_BUILTIN_FUNCTION_CALL: GenericApiErrorConst =
    GenericApiErrorConst::new(
        "ParseExpectedRightParenBuiltinFunctionCall",
        "Did not find the expected right parenthesis character in the SQL expression.",
        StatusCode::BAD_REQUEST,
    );
const PARSE_EXPECTED_TYPE_NAME: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedTypeName",
    "Did not find the expected type name in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_WHEN_CLAUSE: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedWhenClause",
    "Did not find the expected WHEN clause in the SQL expression. CASE is not supported.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNSUPPORTED_TOKEN: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnsupportedToken",
    "The SQL expression contains an unsupported token.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNSUPPORTED_LITERALS_GROUP_BY: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnsupportedLiteralsGroupBy",
    "The SQL expression contains an unsupported use of GROUP BY.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_MEMBER: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedMember",
    "The SQL expression contains an unsupported use of MEMBER.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNSUPPORTED_SELECT: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnsupportedSelect",
    "The SQL expression contains an unsupported use of SELECT.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNSUPPORTED_CASE: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnsupportedCase",
    "The SQL expression contains an unsupported use of CASE.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNSUPPORTED_CASE_CLAUSE: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnsupportedCaseClause",
    "The SQL expression contains an unsupported use of CASE.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNSUPPORTED_ALIAS: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnsupportedAlias",
    "The SQL expression contains an unsupported use of ALIAS.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNSUPPORTED_SYNTAX: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnsupportedSyntax",
    "The SQL expression contains unsupported syntax.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNKNOWN_OPERATOR: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnknownOperator",
    "The SQL expression contains an invalid operator.",
    StatusCode::BAD_REQUEST,
);
const PARSE_MISSING_IDENT_AFTER_AT: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseMissingIdentAfterAt",
    "Did not find the expected identifier after the @ symbol in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNEXPECTED_OPERATOR: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnexpectedOperator",
    "The SQL expression contains an unexpected operator.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNEXPECTED_TERM: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnexpectedTerm",
    "The SQL expression contains an unexpected term.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNEXPECTED_TOKEN: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnexpectedToken",
    "The SQL expression contains an unexpected token.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNEXPECTED_KEYWORD: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnexpectedKeyword",
    "The SQL expression contains an unexpected keyword.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_EXPRESSION: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedExpression",
    "Did not find the expected SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_LEFT_PAREN_AFTER_CAST: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedLeftParenAfterCast",
    "Did not find expected the left parenthesis in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_LEFT_PAREN_VALUE_CONSTRUCTOR: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedLeftParenValueConstructor",
    "Did not find expected the left parenthesis in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_LEFT_PAREN_BUILTIN_FUNCTION_CALL: GenericApiErrorConst =
    GenericApiErrorConst::new(
        "ParseExpectedLeftParenBuiltinFunctionCall",
        "Did not find the expected left parenthesis in the SQL expression.",
        StatusCode::BAD_REQUEST,
    );
const PARSE_EXPECTED_ARGUMENT_DELIMITER: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedArgumentDelimiter",
    "Did not find the expected argument delimiter in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_CAST_ARITY: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseCastArity",
    "The SQL expression CAST has incorrect arity.",
    StatusCode::BAD_REQUEST,
);
const PARSE_INVALID_TYPE_PARAM: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseInvalidTypeParam",
    "The SQL expression contains an invalid parameter value.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EMPTY_SELECT: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseEmptySelect",
    "The SQL expression contains an empty SELECT.",
    StatusCode::BAD_REQUEST,
);
const PARSE_SELECT_MISSING_FROM: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseSelectMissingFrom",
    "GROUP is not supported in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_IDENT_FOR_GROUP_NAME: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedIdentForGroupName",
    "GROUP is not supported in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_IDENT_FOR_ALIAS: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedIdentForAlias",
    "Did not find the expected identifier for the alias in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_UNSUPPORTED_CALL_WITH_STAR: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseUnsupportedCallWithStar",
    "Only COUNT with (*) as a parameter is supported in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_NON_UNARY_AGREGATE_FUNCTION_CALL: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseNonUnaryAgregateFunctionCall",
    "Only one argument is supported for aggregate functions in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_MALFORMED_JOIN: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseMalformedJoin",
    "JOIN is not supported in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_EXPECTED_IDENT_FOR_AT: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseExpectedIdentForAt",
    "Did not find the expected identifier for AT name in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_ASTERISK_IS_NOT_ALONE_IN_SELECT_LIST: GenericApiErrorConst = GenericApiErrorConst::new(
    "ParseAsteriskIsNotAloneInSelectList",
    "Other expressions are not allowed in the SELECT list when '*' is used without dot notation in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const PARSE_CANNOT_MIX_SQB_AND_WILDCARD_IN_SELECT_LIST: GenericApiErrorConst =
    GenericApiErrorConst::new(
        "ParseCannotMixSqbAndWildcardInSelectList",
        "Cannot mix [] and * in the same expression in a SELECT list in SQL expression.",
        StatusCode::BAD_REQUEST,
    );
const PARSE_INVALID_CONTEXT_FOR_WILDCARD_IN_SELECT_LIST: GenericApiErrorConst =
    GenericApiErrorConst::new(
        "ParseInvalidContextForWildcardInSelectList",
        "Invalid use of * in SELECT list in the SQL expression.",
        StatusCode::BAD_REQUEST,
    );
const INCORRECT_SQLFUNCTION_ARGUMENT_TYPE: GenericApiErrorConst = GenericApiErrorConst::new(
    "IncorrectSqlFunctionArgumentType",
    "Incorrect type of arguments in function call in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const VALUE_PARSE_FAILURE: GenericApiErrorConst = GenericApiErrorConst::new(
    "ValueParseFailure",
    "Time stamp parse failure in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const EVALUATOR_INVALID_ARGUMENTS: GenericApiErrorConst = GenericApiErrorConst::new(
    "EvaluatorInvalidArguments",
    "Incorrect number of arguments in the function call in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const INTEGER_OVERFLOW: GenericApiErrorConst = GenericApiErrorConst::new(
    "IntegerOverflow",
    "Int overflow or underflow in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const LIKE_INVALID_INPUTS: GenericApiErrorConst = GenericApiErrorConst::new(
    "LikeInvalidInputs",
    "Invalid argument given to the LIKE clause in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const CAST_FAILED: GenericApiErrorConst = GenericApiErrorConst::new(
    "CastFailed",
    "Attempt to convert from one data type to another using CAST failed in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const INVALID_CAST: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidCast",
    "Attempt to convert from one data type to another using CAST failed in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const EVALUATOR_INVALID_TIMESTAMP_FORMAT_PATTERN: GenericApiErrorConst = GenericApiErrorConst::new(
    "EvaluatorInvalidTimestampFormatPattern",
    "Time stamp format pattern requires additional fields in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const EVALUATOR_INVALID_TIMESTAMP_FORMAT_PATTERN_SYMBOL_FOR_PARSING: GenericApiErrorConst = GenericApiErrorConst::new(
    "EvaluatorInvalidTimestampFormatPatternSymbolForParsing",
    "Time stamp format pattern contains a valid format symbol that cannot be applied to time stamp parsing in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const EVALUATOR_TIMESTAMP_FORMAT_PATTERN_DUPLICATE_FIELDS: GenericApiErrorConst = GenericApiErrorConst::new(
    "EvaluatorTimestampFormatPatternDuplicateFields",
    "Time stamp format pattern contains multiple format specifiers representing the time stamp field in the SQL expression.",
    StatusCode::BAD_REQUEST,
);
const EVALUATOR_TIMESTAMP_FORMAT_PATTERN_HOUR_CLOCK_AM_PM_MISMATCH: GenericApiErrorConst =
    GenericApiErrorConst::new(
        "EvaluatorUnterminatedTimestampFormatPatternToken",
        "Time stamp format pattern contains unterminated token in the SQL expression.",
        StatusCode::BAD_REQUEST,
    );
const EVALUATOR_UNTERMINATED_TIMESTAMP_FORMAT_PATTERN_TOKEN: GenericApiErrorConst =
    GenericApiErrorConst::new(
        "EvaluatorInvalidTimestampFormatPatternToken",
        "Time stamp format pattern contains an invalid token in the SQL expression.",
        StatusCode::BAD_REQUEST,
    );
const EVALUATOR_INVALID_TIMESTAMP_FORMAT_PATTERN_TOKEN: GenericApiErrorConst =
    GenericApiErrorConst::new(
        "EvaluatorInvalidTimestampFormatPatternToken",
        "Time stamp format pattern contains an invalid token in the SQL expression.",
        StatusCode::BAD_REQUEST,
    );
const EVALUATOR_INVALID_TIMESTAMP_FORMAT_PATTERN_SYMBOL: GenericApiErrorConst =
    GenericApiErrorConst::new(
        "EvaluatorInvalidTimestampFormatPatternSymbol",
        "Time stamp format pattern contains an invalid symbol in the SQL expression.",
        StatusCode::BAD_REQUEST,
    );
const EVALUATOR_BINDING_DOES_NOT_EXIST: GenericApiErrorConst = GenericApiErrorConst::new(
    "ErrEvaluatorBindingDoesNotExist",
    "A column name or a path provided does not exist in the SQL expression",
    StatusCode::BAD_REQUEST,
);
const MISSING_HEADERS: GenericApiErrorConst = GenericApiErrorConst::new(
    "MissingHeaders",
    "Some headers in the query are missing from the file. Check the file and try again.",
    StatusCode::BAD_REQUEST,
);
const INVALID_COLUMN_INDEX: GenericApiErrorConst = GenericApiErrorConst::new(
    "InvalidColumnIndex",
    "The column index is invalid. Please check the service documentation and try again.",
    StatusCode::BAD_REQUEST,
);
const INVALID_DECOMPRESSED_SIZE: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkInvalidDecompressedSize",
    "The data provided is unfit for decompression",
    StatusCode::BAD_REQUEST,
);
const ADD_USER_INVALID_ARGUMENT: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkInvalidIAMCredentials",
    "User is not allowed to be same as admin access key",
    StatusCode::FORBIDDEN,
);
const ADMIN_ACCOUNT_NOT_ELIGIBLE: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkInvalidIAMCredentials",
    "The administrator key is not eligible for this operation",
    StatusCode::FORBIDDEN,
);
const ACCOUNT_NOT_ELIGIBLE: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkInvalidIAMCredentials",
    "The account key is not eligible for this operation",
    StatusCode::FORBIDDEN,
);
const ADMIN_SERVICE_ACCOUNT_NOT_FOUND: GenericApiErrorConst = GenericApiErrorConst::new(
    "XHulkInvalidIAMCredentials",
    "The specified service account is not found",
    StatusCode::NOT_FOUND,
);
const POST_POLICY_CONDITION_INVALID_FORMAT: GenericApiErrorConst = GenericApiErrorConst::new(
    "PostPolicyInvalidKeyName",
    "Invalid according to Policy: Policy Condition failed",
    StatusCode::FORBIDDEN,
);
