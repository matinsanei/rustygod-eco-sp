//! GENERATED — DO NOT EDIT. Source: scripts/schema_codegen.py
//! Saleor schema.graphql shapes, dashboard-selected fields only.
//! All outputs relaxed-nullable; stubs are zero-cost (no DB).
//! Real-data wiring: replace stub bodies, keep names.

use async_graphql::*;
use chrono::{DateTime, Utc};

use crate::common::PageInfo;


#[derive(Clone, Debug)]
pub struct GenUpload;

#[Scalar(name = "Upload")]
impl ScalarType for GenUpload {
    fn parse(value: Value) -> InputValueResult<Self> {
        match &value { Value::String(_) | Value::Null => Ok(GenUpload), _ => Err(InputValueError::expected_type(value)), }
    }
    fn to_value(&self) -> Value { Value::Null }
}


macro_rules! gen_string_scalar {
    ($rust:ident, $gql:literal) => {
        #[derive(Clone, Debug)]
        pub struct $rust(pub String);
        #[Scalar(name = $gql)]
        impl ScalarType for $rust {
            fn parse(value: Value) -> InputValueResult<Self> {
                match &value { Value::String(s) => Ok($rust(s.clone())), Value::Number(n) => Ok($rust(n.to_string())), Value::Null => Ok($rust(String::new())), _ => Err(InputValueError::expected_type(value)), }
            }
            fn to_value(&self) -> Value { Value::String(self.0.clone()) }
        }
    };
}

gen_string_scalar!(GenDecimal, "Decimal");

gen_string_scalar!(GenPositiveDecimal, "PositiveDecimal");

gen_string_scalar!(GenJSONString, "JSONString");

#[derive(Clone, Debug)]
pub struct GenWeightScalar(pub String);

#[Scalar(name = "WeightScalar")]
impl ScalarType for GenWeightScalar {
    fn parse(value: Value) -> InputValueResult<Self> {
        match &value { Value::String(s) => Ok(GenWeightScalar(s.clone())), Value::Number(n) => Ok(GenWeightScalar(n.to_string())), Value::Null => Ok(GenWeightScalar(String::new())), _ => Err(InputValueError::expected_type(value)), }
    }
    fn to_value(&self) -> Value { Value::String(self.0.clone()) }
}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccountConfirmModeEnum {

    #[graphql(name = "MERGE_DISABLED")]
    MERGEDISABLED,

    #[graphql(name = "REQUIRE_PASSWORD")]
    REQUIREPASSWORD,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccountErrorCode {

    #[graphql(name = "ACTIVATE_OWN_ACCOUNT")]
    ACTIVATEOWNACCOUNT,

    #[graphql(name = "ACTIVATE_SUPERUSER_ACCOUNT")]
    ACTIVATESUPERUSERACCOUNT,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "DEACTIVATE_OWN_ACCOUNT")]
    DEACTIVATEOWNACCOUNT,

    #[graphql(name = "DEACTIVATE_SUPERUSER_ACCOUNT")]
    DEACTIVATESUPERUSERACCOUNT,

    #[graphql(name = "DELETE_NON_STAFF_USER")]
    DELETENONSTAFFUSER,

    #[graphql(name = "DELETE_OWN_ACCOUNT")]
    DELETEOWNACCOUNT,

    #[graphql(name = "DELETE_STAFF_ACCOUNT")]
    DELETESTAFFACCOUNT,

    #[graphql(name = "DELETE_SUPERUSER_ACCOUNT")]
    DELETESUPERUSERACCOUNT,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INACTIVE")]
    INACTIVE,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "INVALID_PASSWORD")]
    INVALIDPASSWORD,

    #[graphql(name = "LEFT_NOT_MANAGEABLE_PERMISSION")]
    LEFTNOTMANAGEABLEPERMISSION,

    #[graphql(name = "INVALID_CREDENTIALS")]
    INVALIDCREDENTIALS,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "OUT_OF_SCOPE_USER")]
    OUTOFSCOPEUSER,

    #[graphql(name = "OUT_OF_SCOPE_GROUP")]
    OUTOFSCOPEGROUP,

    #[graphql(name = "OUT_OF_SCOPE_PERMISSION")]
    OUTOFSCOPEPERMISSION,

    #[graphql(name = "PASSWORD_ENTIRELY_NUMERIC")]
    PASSWORDENTIRELYNUMERIC,

    #[graphql(name = "PASSWORD_TOO_COMMON")]
    PASSWORDTOOCOMMON,

    #[graphql(name = "PASSWORD_TOO_SHORT")]
    PASSWORDTOOSHORT,

    #[graphql(name = "PASSWORD_TOO_SIMILAR")]
    PASSWORDTOOSIMILAR,

    #[graphql(name = "PASSWORD_RESET_ALREADY_REQUESTED")]
    PASSWORDRESETALREADYREQUESTED,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "JWT_SIGNATURE_EXPIRED")]
    JWTSIGNATUREEXPIRED,

    #[graphql(name = "JWT_INVALID_TOKEN")]
    JWTINVALIDTOKEN,

    #[graphql(name = "JWT_DECODE_ERROR")]
    JWTDECODEERROR,

    #[graphql(name = "JWT_MISSING_TOKEN")]
    JWTMISSINGTOKEN,

    #[graphql(name = "JWT_INVALID_CSRF_TOKEN")]
    JWTINVALIDCSRFTOKEN,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

    #[graphql(name = "MISSING_CHANNEL_SLUG")]
    MISSINGCHANNELSLUG,

    #[graphql(name = "ACCOUNT_NOT_CONFIRMED")]
    ACCOUNTNOTCONFIRMED,

    #[graphql(name = "LOGIN_ATTEMPT_DELAYED")]
    LOGINATTEMPTDELAYED,

    #[graphql(name = "DISABLED_AUTHENTICATION_METHOD")]
    DISABLEDAUTHENTICATIONMETHOD,

    #[graphql(name = "UNKNOWN_IP_ADDRESS")]
    UNKNOWNIPADDRESS,

    #[graphql(name = "FILE_SIZE_LIMIT_EXCEEDED")]
    FILESIZELIMITEXCEEDED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddressTypeEnum {

    #[graphql(name = "BILLING")]
    BILLING,

    #[graphql(name = "SHIPPING")]
    SHIPPING,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocationStrategyEnum {

    #[graphql(name = "PRIORITIZE_SORTING_ORDER")]
    PRIORITIZESORTINGORDER,

    #[graphql(name = "PRIORITIZE_HIGH_STOCK")]
    PRIORITIZEHIGHSTOCK,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnnouncementImportanceEnum {

    #[graphql(name = "CRITICAL")]
    CRITICAL,

    #[graphql(name = "HIGH")]
    HIGH,

    #[graphql(name = "MODERATE")]
    MODERATE,

    #[graphql(name = "LOW")]
    LOW,

    #[graphql(name = "UNSET")]
    UNSET,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppErrorCode {

    #[graphql(name = "FORBIDDEN")]
    FORBIDDEN,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "INVALID_STATUS")]
    INVALIDSTATUS,

    #[graphql(name = "INVALID_PERMISSION")]
    INVALIDPERMISSION,

    #[graphql(name = "INVALID_URL_FORMAT")]
    INVALIDURLFORMAT,

    #[graphql(name = "INVALID_MANIFEST_FORMAT")]
    INVALIDMANIFESTFORMAT,

    #[graphql(name = "INVALID_CUSTOM_HEADERS")]
    INVALIDCUSTOMHEADERS,

    #[graphql(name = "DUPLICATED_EXTENSION_IDENTIFIER")]
    DUPLICATEDEXTENSIONIDENTIFIER,

    #[graphql(name = "DUPLICATED_WEBHOOK_IDENTIFIER")]
    DUPLICATEDWEBHOOKIDENTIFIER,

    #[graphql(name = "MANIFEST_URL_CANT_CONNECT")]
    MANIFESTURLCANTCONNECT,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "OUT_OF_SCOPE_APP")]
    OUTOFSCOPEAPP,

    #[graphql(name = "OUT_OF_SCOPE_PERMISSION")]
    OUTOFSCOPEPERMISSION,

    #[graphql(name = "UNSUPPORTED_SALEOR_VERSION")]
    UNSUPPORTEDSALEORVERSION,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppProblemCreateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppProblemDismissErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "OUT_OF_SCOPE_APP")]
    OUTOFSCOPEAPP,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppProblemDismissedByEnum {

    #[graphql(name = "APP")]
    APP,

    #[graphql(name = "USER")]
    USER,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "CREATION_DATE")]
    CREATIONDATE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AreaUnitsEnum {

    #[graphql(name = "SQ_MM")]
    SQMM,

    #[graphql(name = "SQ_CM")]
    SQCM,

    #[graphql(name = "SQ_DM")]
    SQDM,

    #[graphql(name = "SQ_M")]
    SQM,

    #[graphql(name = "SQ_KM")]
    SQKM,

    #[graphql(name = "SQ_FT")]
    SQFT,

    #[graphql(name = "SQ_YD")]
    SQYD,

    #[graphql(name = "SQ_INCH")]
    SQINCH,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttributeBulkCreateErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "BLANK")]
    BLANK,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "MAX_LENGTH")]
    MAXLENGTH,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttributeBulkUpdateErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "BLANK")]
    BLANK,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "MAX_LENGTH")]
    MAXLENGTH,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttributeChoicesSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "SLUG")]
    SLUG,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttributeEntityTypeEnum {

    #[graphql(name = "PAGE")]
    PAGE,

    #[graphql(name = "PRODUCT")]
    PRODUCT,

    #[graphql(name = "PRODUCT_VARIANT")]
    PRODUCTVARIANT,

    #[graphql(name = "CATEGORY")]
    CATEGORY,

    #[graphql(name = "COLLECTION")]
    COLLECTION,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttributeErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttributeInputTypeEnum {

    #[graphql(name = "DROPDOWN")]
    DROPDOWN,

    #[graphql(name = "MULTISELECT")]
    MULTISELECT,

    #[graphql(name = "FILE")]
    FILE,

    #[graphql(name = "REFERENCE")]
    REFERENCE,

    #[graphql(name = "SINGLE_REFERENCE")]
    SINGLEREFERENCE,

    #[graphql(name = "NUMERIC")]
    NUMERIC,

    #[graphql(name = "RICH_TEXT")]
    RICHTEXT,

    #[graphql(name = "PLAIN_TEXT")]
    PLAINTEXT,

    #[graphql(name = "SWATCH")]
    SWATCH,

    #[graphql(name = "BOOLEAN")]
    BOOLEAN,

    #[graphql(name = "DATE")]
    DATE,

    #[graphql(name = "DATE_TIME")]
    DATETIME,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttributeSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "SLUG")]
    SLUG,

    #[graphql(name = "VALUE_REQUIRED")]
    VALUEREQUIRED,

    #[graphql(name = "IS_VARIANT_ONLY")]
    ISVARIANTONLY,

    #[graphql(name = "VISIBLE_IN_STOREFRONT")]
    VISIBLEINSTOREFRONT,

    #[graphql(name = "FILTERABLE_IN_STOREFRONT")]
    FILTERABLEINSTOREFRONT,

    #[graphql(name = "FILTERABLE_IN_DASHBOARD")]
    FILTERABLEINDASHBOARD,

    #[graphql(name = "STOREFRONT_SEARCH_POSITION")]
    STOREFRONTSEARCHPOSITION,

    #[graphql(name = "AVAILABLE_IN_GRID")]
    AVAILABLEINGRID,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttributeTranslateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttributeTypeEnum {

    #[graphql(name = "PRODUCT_TYPE")]
    PRODUCTTYPE,

    #[graphql(name = "PAGE_TYPE")]
    PAGETYPE,

    #[graphql(name = "CUSTOMER_TYPE")]
    CUSTOMERTYPE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AttributeValueTranslateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CategorySortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "PRODUCT_COUNT")]
    PRODUCTCOUNT,

    #[graphql(name = "SUBCATEGORY_COUNT")]
    SUBCATEGORYCOUNT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChannelErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "CHANNELS_CURRENCY_MUST_BE_THE_SAME")]
    CHANNELSCURRENCYMUSTBETHESAME,

    #[graphql(name = "CHANNEL_WITH_ORDERS")]
    CHANNELWITHORDERS,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckoutAuthorizeStatusEnum {

    #[graphql(name = "NONE")]
    NONE,

    #[graphql(name = "PARTIAL")]
    PARTIAL,

    #[graphql(name = "FULL")]
    FULL,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckoutChargeStatusEnum {

    #[graphql(name = "NONE")]
    NONE,

    #[graphql(name = "PARTIAL")]
    PARTIAL,

    #[graphql(name = "FULL")]
    FULL,

    #[graphql(name = "OVERCHARGED")]
    OVERCHARGED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckoutCreateFromOrderErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "ORDER_NOT_FOUND")]
    ORDERNOTFOUND,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

    #[graphql(name = "TAX_ERROR")]
    TAXERROR,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckoutCreateFromOrderUnavailableVariantErrorCode {

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "PRODUCT_UNAVAILABLE_FOR_PURCHASE")]
    PRODUCTUNAVAILABLEFORPURCHASE,

    #[graphql(name = "UNAVAILABLE_VARIANT_IN_CHANNEL")]
    UNAVAILABLEVARIANTINCHANNEL,

    #[graphql(name = "PRODUCT_NOT_PUBLISHED")]
    PRODUCTNOTPUBLISHED,

    #[graphql(name = "QUANTITY_GREATER_THAN_LIMIT")]
    QUANTITYGREATERTHANLIMIT,

    #[graphql(name = "INSUFFICIENT_STOCK")]
    INSUFFICIENTSTOCK,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckoutErrorCode {

    #[graphql(name = "BILLING_ADDRESS_NOT_SET")]
    BILLINGADDRESSNOTSET,

    #[graphql(name = "CHECKOUT_NOT_FULLY_PAID")]
    CHECKOUTNOTFULLYPAID,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "PRODUCT_NOT_PUBLISHED")]
    PRODUCTNOTPUBLISHED,

    #[graphql(name = "PRODUCT_UNAVAILABLE_FOR_PURCHASE")]
    PRODUCTUNAVAILABLEFORPURCHASE,

    #[graphql(name = "INSUFFICIENT_STOCK")]
    INSUFFICIENTSTOCK,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "INVALID_SHIPPING_METHOD")]
    INVALIDSHIPPINGMETHOD,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "PAYMENT_ERROR")]
    PAYMENTERROR,

    #[graphql(name = "PRICE_OVERRIDE_REASON_WITHOUT_OVERRIDE")]
    PRICEOVERRIDEREASONWITHOUTOVERRIDE,

    #[graphql(name = "QUANTITY_GREATER_THAN_LIMIT")]
    QUANTITYGREATERTHANLIMIT,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "SHIPPING_ADDRESS_NOT_SET")]
    SHIPPINGADDRESSNOTSET,

    #[graphql(name = "SHIPPING_METHOD_NOT_APPLICABLE")]
    SHIPPINGMETHODNOTAPPLICABLE,

    #[graphql(name = "DELIVERY_METHOD_NOT_APPLICABLE")]
    DELIVERYMETHODNOTAPPLICABLE,

    #[graphql(name = "SHIPPING_METHOD_NOT_SET")]
    SHIPPINGMETHODNOTSET,

    #[graphql(name = "SHIPPING_NOT_REQUIRED")]
    SHIPPINGNOTREQUIRED,

    #[graphql(name = "TAX_ERROR")]
    TAXERROR,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "VOUCHER_NOT_APPLICABLE")]
    VOUCHERNOTAPPLICABLE,

    #[graphql(name = "GIFT_CARD_NOT_APPLICABLE")]
    GIFTCARDNOTAPPLICABLE,

    #[graphql(name = "ZERO_QUANTITY")]
    ZEROQUANTITY,

    #[graphql(name = "MISSING_CHANNEL_SLUG")]
    MISSINGCHANNELSLUG,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

    #[graphql(name = "UNAVAILABLE_VARIANT_IN_CHANNEL")]
    UNAVAILABLEVARIANTINCHANNEL,

    #[graphql(name = "EMAIL_NOT_SET")]
    EMAILNOTSET,

    #[graphql(name = "NO_LINES")]
    NOLINES,

    #[graphql(name = "INACTIVE_PAYMENT")]
    INACTIVEPAYMENT,

    #[graphql(name = "NON_EDITABLE_GIFT_LINE")]
    NONEDITABLEGIFTLINE,

    #[graphql(name = "NON_REMOVABLE_GIFT_LINE")]
    NONREMOVABLEGIFTLINE,

    #[graphql(name = "SHIPPING_CHANGE_FORBIDDEN")]
    SHIPPINGCHANGEFORBIDDEN,

    #[graphql(name = "MISSING_ADDRESS_DATA")]
    MISSINGADDRESSDATA,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckoutSortField {

    #[graphql(name = "CREATION_DATE")]
    CREATIONDATE,

    #[graphql(name = "CUSTOMER")]
    CUSTOMER,

    #[graphql(name = "PAYMENT")]
    PAYMENT,

    #[graphql(name = "RANK")]
    RANK,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CircuitBreakerStateEnum {

    #[graphql(name = "CLOSED")]
    CLOSED,

    #[graphql(name = "HALF_OPEN")]
    HALFOPEN,

    #[graphql(name = "OPEN")]
    OPEN,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CollectionErrorCode {

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "CANNOT_MANAGE_PRODUCT_WITHOUT_VARIANT")]
    CANNOTMANAGEPRODUCTWITHOUTVARIANT,

    #[graphql(name = "FILE_SIZE_LIMIT_EXCEEDED")]
    FILESIZELIMITEXCEEDED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CollectionPublished {

    #[graphql(name = "PUBLISHED")]
    PUBLISHED,

    #[graphql(name = "HIDDEN")]
    HIDDEN,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CollectionSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "AVAILABILITY")]
    AVAILABILITY,

    #[graphql(name = "PRODUCT_COUNT")]
    PRODUCTCOUNT,

    #[graphql(name = "PUBLICATION_DATE")]
    PUBLICATIONDATE,

    #[graphql(name = "PUBLISHED_AT")]
    PUBLISHEDAT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConfigurationTypeFieldEnum {

    #[graphql(name = "STRING")]
    STRING,

    #[graphql(name = "MULTILINE")]
    MULTILINE,

    #[graphql(name = "BOOLEAN")]
    BOOLEAN,

    #[graphql(name = "SECRET")]
    SECRET,

    #[graphql(name = "PASSWORD")]
    PASSWORD,

    #[graphql(name = "SECRETMULTILINE")]
    SECRETMULTILINE,

    #[graphql(name = "OUTPUT")]
    OUTPUT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CountryCode {

    #[graphql(name = "AF")]
    AF,

    #[graphql(name = "AX")]
    AX,

    #[graphql(name = "AL")]
    AL,

    #[graphql(name = "DZ")]
    DZ,

    #[graphql(name = "AS")]
    AS,

    #[graphql(name = "AD")]
    AD,

    #[graphql(name = "AO")]
    AO,

    #[graphql(name = "AI")]
    AI,

    #[graphql(name = "AQ")]
    AQ,

    #[graphql(name = "AG")]
    AG,

    #[graphql(name = "AR")]
    AR,

    #[graphql(name = "AM")]
    AM,

    #[graphql(name = "AW")]
    AW,

    #[graphql(name = "AU")]
    AU,

    #[graphql(name = "AT")]
    AT,

    #[graphql(name = "AZ")]
    AZ,

    #[graphql(name = "BS")]
    BS,

    #[graphql(name = "BH")]
    BH,

    #[graphql(name = "BD")]
    BD,

    #[graphql(name = "BB")]
    BB,

    #[graphql(name = "BY")]
    BY,

    #[graphql(name = "BE")]
    BE,

    #[graphql(name = "BZ")]
    BZ,

    #[graphql(name = "BJ")]
    BJ,

    #[graphql(name = "BM")]
    BM,

    #[graphql(name = "BT")]
    BT,

    #[graphql(name = "BO")]
    BO,

    #[graphql(name = "BQ")]
    BQ,

    #[graphql(name = "BA")]
    BA,

    #[graphql(name = "BW")]
    BW,

    #[graphql(name = "BV")]
    BV,

    #[graphql(name = "BR")]
    BR,

    #[graphql(name = "IO")]
    IO,

    #[graphql(name = "BN")]
    BN,

    #[graphql(name = "BG")]
    BG,

    #[graphql(name = "BF")]
    BF,

    #[graphql(name = "BI")]
    BI,

    #[graphql(name = "CV")]
    CV,

    #[graphql(name = "KH")]
    KH,

    #[graphql(name = "CM")]
    CM,

    #[graphql(name = "CA")]
    CA,

    #[graphql(name = "KY")]
    KY,

    #[graphql(name = "CF")]
    CF,

    #[graphql(name = "TD")]
    TD,

    #[graphql(name = "CL")]
    CL,

    #[graphql(name = "CN")]
    CN,

    #[graphql(name = "CX")]
    CX,

    #[graphql(name = "CC")]
    CC,

    #[graphql(name = "CO")]
    CO,

    #[graphql(name = "KM")]
    KM,

    #[graphql(name = "CG")]
    CG,

    #[graphql(name = "CD")]
    CD,

    #[graphql(name = "CK")]
    CK,

    #[graphql(name = "CR")]
    CR,

    #[graphql(name = "CI")]
    CI,

    #[graphql(name = "HR")]
    HR,

    #[graphql(name = "CU")]
    CU,

    #[graphql(name = "CW")]
    CW,

    #[graphql(name = "CY")]
    CY,

    #[graphql(name = "CZ")]
    CZ,

    #[graphql(name = "DK")]
    DK,

    #[graphql(name = "DJ")]
    DJ,

    #[graphql(name = "DM")]
    DM,

    #[graphql(name = "DO")]
    DO,

    #[graphql(name = "EC")]
    EC,

    #[graphql(name = "EG")]
    EG,

    #[graphql(name = "SV")]
    SV,

    #[graphql(name = "GQ")]
    GQ,

    #[graphql(name = "ER")]
    ER,

    #[graphql(name = "EE")]
    EE,

    #[graphql(name = "SZ")]
    SZ,

    #[graphql(name = "ET")]
    ET,

    #[graphql(name = "EU")]
    EU,

    #[graphql(name = "FK")]
    FK,

    #[graphql(name = "FO")]
    FO,

    #[graphql(name = "FJ")]
    FJ,

    #[graphql(name = "FI")]
    FI,

    #[graphql(name = "FR")]
    FR,

    #[graphql(name = "GF")]
    GF,

    #[graphql(name = "PF")]
    PF,

    #[graphql(name = "TF")]
    TF,

    #[graphql(name = "GA")]
    GA,

    #[graphql(name = "GM")]
    GM,

    #[graphql(name = "GE")]
    GE,

    #[graphql(name = "DE")]
    DE,

    #[graphql(name = "GH")]
    GH,

    #[graphql(name = "GI")]
    GI,

    #[graphql(name = "GR")]
    GR,

    #[graphql(name = "GL")]
    GL,

    #[graphql(name = "GD")]
    GD,

    #[graphql(name = "GP")]
    GP,

    #[graphql(name = "GU")]
    GU,

    #[graphql(name = "GT")]
    GT,

    #[graphql(name = "GG")]
    GG,

    #[graphql(name = "GN")]
    GN,

    #[graphql(name = "GW")]
    GW,

    #[graphql(name = "GY")]
    GY,

    #[graphql(name = "HT")]
    HT,

    #[graphql(name = "HM")]
    HM,

    #[graphql(name = "VA")]
    VA,

    #[graphql(name = "HN")]
    HN,

    #[graphql(name = "HK")]
    HK,

    #[graphql(name = "HU")]
    HU,

    #[graphql(name = "IS")]
    IS,

    #[graphql(name = "IN")]
    IN,

    #[graphql(name = "ID")]
    ID,

    #[graphql(name = "IR")]
    IR,

    #[graphql(name = "IQ")]
    IQ,

    #[graphql(name = "IE")]
    IE,

    #[graphql(name = "IM")]
    IM,

    #[graphql(name = "IL")]
    IL,

    #[graphql(name = "IT")]
    IT,

    #[graphql(name = "JM")]
    JM,

    #[graphql(name = "JP")]
    JP,

    #[graphql(name = "JE")]
    JE,

    #[graphql(name = "JO")]
    JO,

    #[graphql(name = "KZ")]
    KZ,

    #[graphql(name = "KE")]
    KE,

    #[graphql(name = "KI")]
    KI,

    #[graphql(name = "XK")]
    XK,

    #[graphql(name = "KW")]
    KW,

    #[graphql(name = "KG")]
    KG,

    #[graphql(name = "LA")]
    LA,

    #[graphql(name = "LV")]
    LV,

    #[graphql(name = "LB")]
    LB,

    #[graphql(name = "LS")]
    LS,

    #[graphql(name = "LR")]
    LR,

    #[graphql(name = "LY")]
    LY,

    #[graphql(name = "LI")]
    LI,

    #[graphql(name = "LT")]
    LT,

    #[graphql(name = "LU")]
    LU,

    #[graphql(name = "MO")]
    MO,

    #[graphql(name = "MG")]
    MG,

    #[graphql(name = "MW")]
    MW,

    #[graphql(name = "MY")]
    MY,

    #[graphql(name = "MV")]
    MV,

    #[graphql(name = "ML")]
    ML,

    #[graphql(name = "MT")]
    MT,

    #[graphql(name = "MH")]
    MH,

    #[graphql(name = "MQ")]
    MQ,

    #[graphql(name = "MR")]
    MR,

    #[graphql(name = "MU")]
    MU,

    #[graphql(name = "YT")]
    YT,

    #[graphql(name = "MX")]
    MX,

    #[graphql(name = "FM")]
    FM,

    #[graphql(name = "MD")]
    MD,

    #[graphql(name = "MC")]
    MC,

    #[graphql(name = "MN")]
    MN,

    #[graphql(name = "ME")]
    ME,

    #[graphql(name = "MS")]
    MS,

    #[graphql(name = "MA")]
    MA,

    #[graphql(name = "MZ")]
    MZ,

    #[graphql(name = "MM")]
    MM,

    #[graphql(name = "NA")]
    NA,

    #[graphql(name = "NR")]
    NR,

    #[graphql(name = "NP")]
    NP,

    #[graphql(name = "NL")]
    NL,

    #[graphql(name = "NC")]
    NC,

    #[graphql(name = "NZ")]
    NZ,

    #[graphql(name = "NI")]
    NI,

    #[graphql(name = "NE")]
    NE,

    #[graphql(name = "NG")]
    NG,

    #[graphql(name = "NU")]
    NU,

    #[graphql(name = "NF")]
    NF,

    #[graphql(name = "KP")]
    KP,

    #[graphql(name = "MK")]
    MK,

    #[graphql(name = "MP")]
    MP,

    #[graphql(name = "NO")]
    NO,

    #[graphql(name = "OM")]
    OM,

    #[graphql(name = "PK")]
    PK,

    #[graphql(name = "PW")]
    PW,

    #[graphql(name = "PS")]
    PS,

    #[graphql(name = "PA")]
    PA,

    #[graphql(name = "PG")]
    PG,

    #[graphql(name = "PY")]
    PY,

    #[graphql(name = "PE")]
    PE,

    #[graphql(name = "PH")]
    PH,

    #[graphql(name = "PN")]
    PN,

    #[graphql(name = "PL")]
    PL,

    #[graphql(name = "PT")]
    PT,

    #[graphql(name = "PR")]
    PR,

    #[graphql(name = "QA")]
    QA,

    #[graphql(name = "RE")]
    RE,

    #[graphql(name = "RO")]
    RO,

    #[graphql(name = "RU")]
    RU,

    #[graphql(name = "RW")]
    RW,

    #[graphql(name = "BL")]
    BL,

    #[graphql(name = "SH")]
    SH,

    #[graphql(name = "KN")]
    KN,

    #[graphql(name = "LC")]
    LC,

    #[graphql(name = "MF")]
    MF,

    #[graphql(name = "PM")]
    PM,

    #[graphql(name = "VC")]
    VC,

    #[graphql(name = "WS")]
    WS,

    #[graphql(name = "SM")]
    SM,

    #[graphql(name = "ST")]
    ST,

    #[graphql(name = "SA")]
    SA,

    #[graphql(name = "SN")]
    SN,

    #[graphql(name = "RS")]
    RS,

    #[graphql(name = "SC")]
    SC,

    #[graphql(name = "SL")]
    SL,

    #[graphql(name = "SG")]
    SG,

    #[graphql(name = "SX")]
    SX,

    #[graphql(name = "SK")]
    SK,

    #[graphql(name = "SI")]
    SI,

    #[graphql(name = "SB")]
    SB,

    #[graphql(name = "SO")]
    SO,

    #[graphql(name = "ZA")]
    ZA,

    #[graphql(name = "GS")]
    GS,

    #[graphql(name = "KR")]
    KR,

    #[graphql(name = "SS")]
    SS,

    #[graphql(name = "ES")]
    ES,

    #[graphql(name = "LK")]
    LK,

    #[graphql(name = "SD")]
    SD,

    #[graphql(name = "SR")]
    SR,

    #[graphql(name = "SJ")]
    SJ,

    #[graphql(name = "SE")]
    SE,

    #[graphql(name = "CH")]
    CH,

    #[graphql(name = "SY")]
    SY,

    #[graphql(name = "TW")]
    TW,

    #[graphql(name = "TJ")]
    TJ,

    #[graphql(name = "TZ")]
    TZ,

    #[graphql(name = "TH")]
    TH,

    #[graphql(name = "TL")]
    TL,

    #[graphql(name = "TG")]
    TG,

    #[graphql(name = "TK")]
    TK,

    #[graphql(name = "TO")]
    TO,

    #[graphql(name = "TT")]
    TT,

    #[graphql(name = "TN")]
    TN,

    #[graphql(name = "TR")]
    TR,

    #[graphql(name = "TM")]
    TM,

    #[graphql(name = "TC")]
    TC,

    #[graphql(name = "TV")]
    TV,

    #[graphql(name = "UG")]
    UG,

    #[graphql(name = "UA")]
    UA,

    #[graphql(name = "AE")]
    AE,

    #[graphql(name = "GB")]
    GB,

    #[graphql(name = "UM")]
    UM,

    #[graphql(name = "US")]
    US,

    #[graphql(name = "UY")]
    UY,

    #[graphql(name = "UZ")]
    UZ,

    #[graphql(name = "VU")]
    VU,

    #[graphql(name = "VE")]
    VE,

    #[graphql(name = "VN")]
    VN,

    #[graphql(name = "VG")]
    VG,

    #[graphql(name = "VI")]
    VI,

    #[graphql(name = "WF")]
    WF,

    #[graphql(name = "EH")]
    EH,

    #[graphql(name = "YE")]
    YE,

    #[graphql(name = "ZM")]
    ZM,

    #[graphql(name = "ZW")]
    ZW,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CustomerBulkUpdateErrorCode {

    #[graphql(name = "BLANK")]
    BLANK,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "MAX_LENGTH")]
    MAXLENGTH,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CustomerEventsEnum {

    #[graphql(name = "ACCOUNT_CREATED")]
    ACCOUNTCREATED,

    #[graphql(name = "ACCOUNT_ACTIVATED")]
    ACCOUNTACTIVATED,

    #[graphql(name = "ACCOUNT_DEACTIVATED")]
    ACCOUNTDEACTIVATED,

    #[graphql(name = "PASSWORD_RESET_LINK_SENT")]
    PASSWORDRESETLINKSENT,

    #[graphql(name = "PASSWORD_RESET")]
    PASSWORDRESET,

    #[graphql(name = "EMAIL_CHANGED_REQUEST")]
    EMAILCHANGEDREQUEST,

    #[graphql(name = "PASSWORD_CHANGED")]
    PASSWORDCHANGED,

    #[graphql(name = "EMAIL_CHANGED")]
    EMAILCHANGED,

    #[graphql(name = "PLACED_ORDER")]
    PLACEDORDER,

    #[graphql(name = "NOTE_ADDED_TO_ORDER")]
    NOTEADDEDTOORDER,

    #[graphql(name = "DIGITAL_LINK_DOWNLOADED")]
    DIGITALLINKDOWNLOADED,

    #[graphql(name = "CUSTOMER_DELETED")]
    CUSTOMERDELETED,

    #[graphql(name = "NAME_ASSIGNED")]
    NAMEASSIGNED,

    #[graphql(name = "EMAIL_ASSIGNED")]
    EMAILASSIGNED,

    #[graphql(name = "NOTE_ADDED")]
    NOTEADDED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CustomerTypeAssignAttributesErrorCode {

    #[graphql(name = "ATTRIBUTE_ALREADY_ASSIGNED")]
    ATTRIBUTEALREADYASSIGNED,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CustomerTypeCreateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CustomerTypeDeleteErrorCode {

    #[graphql(name = "CANNOT_DELETE_DEFAULT")]
    CANNOTDELETEDEFAULT,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CustomerTypeReorderAttributesErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CustomerTypeSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "SLUG")]
    SLUG,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CustomerTypeUnassignAttributesErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum CustomerTypeUpdateErrorCode {

    #[graphql(name = "CANNOT_UNSET_DEFAULT")]
    CANNOTUNSETDEFAULT,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeliveryOptionsCalculateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "INVALID")]
    INVALID,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiscountErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "CANNOT_MANAGE_PRODUCT_WITHOUT_VARIANT")]
    CANNOTMANAGEPRODUCTWITHOUTVARIANT,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "VOUCHER_ALREADY_USED")]
    VOUCHERALREADYUSED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiscountStatusEnum {

    #[graphql(name = "ACTIVE")]
    ACTIVE,

    #[graphql(name = "EXPIRED")]
    EXPIRED,

    #[graphql(name = "SCHEDULED")]
    SCHEDULED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiscountValueTypeEnum {

    #[graphql(name = "FIXED")]
    FIXED,

    #[graphql(name = "PERCENTAGE")]
    PERCENTAGE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DistanceUnitsEnum {

    #[graphql(name = "MM")]
    MM,

    #[graphql(name = "CM")]
    CM,

    #[graphql(name = "DM")]
    DM,

    #[graphql(name = "M")]
    M,

    #[graphql(name = "KM")]
    KM,

    #[graphql(name = "FT")]
    FT,

    #[graphql(name = "YD")]
    YD,

    #[graphql(name = "INCH")]
    INCH,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorPolicyEnum {

    #[graphql(name = "IGNORE_FAILED")]
    IGNOREFAILED,

    #[graphql(name = "REJECT_EVERYTHING")]
    REJECTEVERYTHING,

    #[graphql(name = "REJECT_FAILED_ROWS")]
    REJECTFAILEDROWS,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventDeliveryAttemptSortField {

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventDeliverySortField {

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventDeliveryStatusEnum {

    #[graphql(name = "PENDING")]
    PENDING,

    #[graphql(name = "SUCCESS")]
    SUCCESS,

    #[graphql(name = "FAILED")]
    FAILED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportEventsEnum {

    #[graphql(name = "EXPORT_PENDING")]
    EXPORTPENDING,

    #[graphql(name = "EXPORT_SUCCESS")]
    EXPORTSUCCESS,

    #[graphql(name = "EXPORT_FAILED")]
    EXPORTFAILED,

    #[graphql(name = "EXPORT_DELETED")]
    EXPORTDELETED,

    #[graphql(name = "EXPORTED_FILE_SENT")]
    EXPORTEDFILESENT,

    #[graphql(name = "EXPORT_FAILED_INFO_SENT")]
    EXPORTFAILEDINFOSENT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportFileSortField {

    #[graphql(name = "STATUS")]
    STATUS,

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

    #[graphql(name = "UPDATED_AT")]
    UPDATEDAT,

    #[graphql(name = "LAST_MODIFIED_AT")]
    LASTMODIFIEDAT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportScope {

    #[graphql(name = "ALL")]
    ALL,

    #[graphql(name = "IDS")]
    IDS,

    #[graphql(name = "FILTER")]
    FILTER,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExternalNotificationErrorCodes {

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "INVALID_MODEL_TYPE")]
    INVALIDMODELTYPE,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileTypesEnum {

    #[graphql(name = "CSV")]
    CSV,

    #[graphql(name = "XLSX")]
    XLSX,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum FulfillmentStatus {

    #[graphql(name = "FULFILLED")]
    FULFILLED,

    #[graphql(name = "REFUNDED")]
    REFUNDED,

    #[graphql(name = "RETURNED")]
    RETURNED,

    #[graphql(name = "REPLACED")]
    REPLACED,

    #[graphql(name = "REFUNDED_AND_RETURNED")]
    REFUNDEDANDRETURNED,

    #[graphql(name = "CANCELED")]
    CANCELED,

    #[graphql(name = "WAITING_FOR_APPROVAL")]
    WAITINGFORAPPROVAL,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GiftCardErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "EXPIRED_GIFT_CARD")]
    EXPIREDGIFTCARD,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "CANNOT_ASSIGN")]
    CANNOTASSIGN,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GiftCardEventsEnum {

    #[graphql(name = "ISSUED")]
    ISSUED,

    #[graphql(name = "BOUGHT")]
    BOUGHT,

    #[graphql(name = "UPDATED")]
    UPDATED,

    #[graphql(name = "ACTIVATED")]
    ACTIVATED,

    #[graphql(name = "DEACTIVATED")]
    DEACTIVATED,

    #[graphql(name = "BALANCE_RESET")]
    BALANCERESET,

    #[graphql(name = "EXPIRY_DATE_UPDATED")]
    EXPIRYDATEUPDATED,

    #[graphql(name = "TAGS_UPDATED")]
    TAGSUPDATED,

    #[graphql(name = "SENT_TO_CUSTOMER")]
    SENTTOCUSTOMER,

    #[graphql(name = "RESENT")]
    RESENT,

    #[graphql(name = "NOTE_ADDED")]
    NOTEADDED,

    #[graphql(name = "USED_IN_ORDER")]
    USEDINORDER,

    #[graphql(name = "REFUNDED_IN_ORDER")]
    REFUNDEDINORDER,

    #[graphql(name = "BALANCE_ADJUSTED")]
    BALANCEADJUSTED,

    #[graphql(name = "ASSIGNED_TO_USER")]
    ASSIGNEDTOUSER,

    #[graphql(name = "UNASSIGNED_FROM_USER")]
    UNASSIGNEDFROMUSER,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GiftCardSettingsErrorCode {

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GiftCardSettingsExpiryTypeEnum {

    #[graphql(name = "NEVER_EXPIRE")]
    NEVEREXPIRE,

    #[graphql(name = "EXPIRY_PERIOD")]
    EXPIRYPERIOD,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GiftCardSortField {

    #[graphql(name = "PRODUCT")]
    PRODUCT,

    #[graphql(name = "USED_BY")]
    USEDBY,

    #[graphql(name = "CURRENT_BALANCE")]
    CURRENTBALANCE,

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

    #[graphql(name = "RANK")]
    RANK,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum InvoiceErrorCode {

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "NOT_READY")]
    NOTREADY,

    #[graphql(name = "URL_NOT_SET")]
    URLNOTSET,

    #[graphql(name = "EMAIL_NOT_SET")]
    EMAILNOTSET,

    #[graphql(name = "NUMBER_NOT_SET")]
    NUMBERNOTSET,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "INVALID_STATUS")]
    INVALIDSTATUS,

    #[graphql(name = "NO_INVOICE_PLUGIN")]
    NOINVOICEPLUGIN,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobStatusEnum {

    #[graphql(name = "PENDING")]
    PENDING,

    #[graphql(name = "SUCCESS")]
    SUCCESS,

    #[graphql(name = "FAILED")]
    FAILED,

    #[graphql(name = "DELETED")]
    DELETED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LanguageCodeEnum {

    #[graphql(name = "AF")]
    AF,

    #[graphql(name = "AF_NA")]
    AFNA,

    #[graphql(name = "AF_ZA")]
    AFZA,

    #[graphql(name = "AGQ")]
    AGQ,

    #[graphql(name = "AGQ_CM")]
    AGQCM,

    #[graphql(name = "AK")]
    AK,

    #[graphql(name = "AK_GH")]
    AKGH,

    #[graphql(name = "AM")]
    AM,

    #[graphql(name = "AM_ET")]
    AMET,

    #[graphql(name = "AR")]
    AR,

    #[graphql(name = "AR_AE")]
    ARAE,

    #[graphql(name = "AR_BH")]
    ARBH,

    #[graphql(name = "AR_DJ")]
    ARDJ,

    #[graphql(name = "AR_DZ")]
    ARDZ,

    #[graphql(name = "AR_EG")]
    AREG,

    #[graphql(name = "AR_EH")]
    AREH,

    #[graphql(name = "AR_ER")]
    ARER,

    #[graphql(name = "AR_IL")]
    ARIL,

    #[graphql(name = "AR_IQ")]
    ARIQ,

    #[graphql(name = "AR_JO")]
    ARJO,

    #[graphql(name = "AR_KM")]
    ARKM,

    #[graphql(name = "AR_KW")]
    ARKW,

    #[graphql(name = "AR_LB")]
    ARLB,

    #[graphql(name = "AR_LY")]
    ARLY,

    #[graphql(name = "AR_MA")]
    ARMA,

    #[graphql(name = "AR_MR")]
    ARMR,

    #[graphql(name = "AR_OM")]
    AROM,

    #[graphql(name = "AR_PS")]
    ARPS,

    #[graphql(name = "AR_QA")]
    ARQA,

    #[graphql(name = "AR_SA")]
    ARSA,

    #[graphql(name = "AR_SD")]
    ARSD,

    #[graphql(name = "AR_SO")]
    ARSO,

    #[graphql(name = "AR_SS")]
    ARSS,

    #[graphql(name = "AR_SY")]
    ARSY,

    #[graphql(name = "AR_TD")]
    ARTD,

    #[graphql(name = "AR_TN")]
    ARTN,

    #[graphql(name = "AR_YE")]
    ARYE,

    #[graphql(name = "AS")]
    AS,

    #[graphql(name = "AS_IN")]
    ASIN,

    #[graphql(name = "ASA")]
    ASA,

    #[graphql(name = "ASA_TZ")]
    ASATZ,

    #[graphql(name = "AST")]
    AST,

    #[graphql(name = "AST_ES")]
    ASTES,

    #[graphql(name = "AZ")]
    AZ,

    #[graphql(name = "AZ_CYRL")]
    AZCYRL,

    #[graphql(name = "AZ_CYRL_AZ")]
    AZCYRLAZ,

    #[graphql(name = "AZ_LATN")]
    AZLATN,

    #[graphql(name = "AZ_LATN_AZ")]
    AZLATNAZ,

    #[graphql(name = "BAS")]
    BAS,

    #[graphql(name = "BAS_CM")]
    BASCM,

    #[graphql(name = "BE")]
    BE,

    #[graphql(name = "BE_BY")]
    BEBY,

    #[graphql(name = "BEM")]
    BEM,

    #[graphql(name = "BEM_ZM")]
    BEMZM,

    #[graphql(name = "BEZ")]
    BEZ,

    #[graphql(name = "BEZ_TZ")]
    BEZTZ,

    #[graphql(name = "BG")]
    BG,

    #[graphql(name = "BG_BG")]
    BGBG,

    #[graphql(name = "BM")]
    BM,

    #[graphql(name = "BM_ML")]
    BMML,

    #[graphql(name = "BN")]
    BN,

    #[graphql(name = "BN_BD")]
    BNBD,

    #[graphql(name = "BN_IN")]
    BNIN,

    #[graphql(name = "BO")]
    BO,

    #[graphql(name = "BO_CN")]
    BOCN,

    #[graphql(name = "BO_IN")]
    BOIN,

    #[graphql(name = "BR")]
    BR,

    #[graphql(name = "BR_FR")]
    BRFR,

    #[graphql(name = "BRX")]
    BRX,

    #[graphql(name = "BRX_IN")]
    BRXIN,

    #[graphql(name = "BS")]
    BS,

    #[graphql(name = "BS_CYRL")]
    BSCYRL,

    #[graphql(name = "BS_CYRL_BA")]
    BSCYRLBA,

    #[graphql(name = "BS_LATN")]
    BSLATN,

    #[graphql(name = "BS_LATN_BA")]
    BSLATNBA,

    #[graphql(name = "CA")]
    CA,

    #[graphql(name = "CA_AD")]
    CAAD,

    #[graphql(name = "CA_ES")]
    CAES,

    #[graphql(name = "CA_ES_VALENCIA")]
    CAESVALENCIA,

    #[graphql(name = "CA_FR")]
    CAFR,

    #[graphql(name = "CA_IT")]
    CAIT,

    #[graphql(name = "CCP")]
    CCP,

    #[graphql(name = "CCP_BD")]
    CCPBD,

    #[graphql(name = "CCP_IN")]
    CCPIN,

    #[graphql(name = "CE")]
    CE,

    #[graphql(name = "CE_RU")]
    CERU,

    #[graphql(name = "CEB")]
    CEB,

    #[graphql(name = "CEB_PH")]
    CEBPH,

    #[graphql(name = "CGG")]
    CGG,

    #[graphql(name = "CGG_UG")]
    CGGUG,

    #[graphql(name = "CHR")]
    CHR,

    #[graphql(name = "CHR_US")]
    CHRUS,

    #[graphql(name = "CKB")]
    CKB,

    #[graphql(name = "CKB_IQ")]
    CKBIQ,

    #[graphql(name = "CKB_IR")]
    CKBIR,

    #[graphql(name = "CS")]
    CS,

    #[graphql(name = "CS_CZ")]
    CSCZ,

    #[graphql(name = "CU")]
    CU,

    #[graphql(name = "CU_RU")]
    CURU,

    #[graphql(name = "CY")]
    CY,

    #[graphql(name = "CY_GB")]
    CYGB,

    #[graphql(name = "DA")]
    DA,

    #[graphql(name = "DA_DK")]
    DADK,

    #[graphql(name = "DA_GL")]
    DAGL,

    #[graphql(name = "DAV")]
    DAV,

    #[graphql(name = "DAV_KE")]
    DAVKE,

    #[graphql(name = "DE")]
    DE,

    #[graphql(name = "DE_AT")]
    DEAT,

    #[graphql(name = "DE_BE")]
    DEBE,

    #[graphql(name = "DE_CH")]
    DECH,

    #[graphql(name = "DE_DE")]
    DEDE,

    #[graphql(name = "DE_IT")]
    DEIT,

    #[graphql(name = "DE_LI")]
    DELI,

    #[graphql(name = "DE_LU")]
    DELU,

    #[graphql(name = "DJE")]
    DJE,

    #[graphql(name = "DJE_NE")]
    DJENE,

    #[graphql(name = "DSB")]
    DSB,

    #[graphql(name = "DSB_DE")]
    DSBDE,

    #[graphql(name = "DUA")]
    DUA,

    #[graphql(name = "DUA_CM")]
    DUACM,

    #[graphql(name = "DYO")]
    DYO,

    #[graphql(name = "DYO_SN")]
    DYOSN,

    #[graphql(name = "DZ")]
    DZ,

    #[graphql(name = "DZ_BT")]
    DZBT,

    #[graphql(name = "EBU")]
    EBU,

    #[graphql(name = "EBU_KE")]
    EBUKE,

    #[graphql(name = "EE")]
    EE,

    #[graphql(name = "EE_GH")]
    EEGH,

    #[graphql(name = "EE_TG")]
    EETG,

    #[graphql(name = "EL")]
    EL,

    #[graphql(name = "EL_CY")]
    ELCY,

    #[graphql(name = "EL_GR")]
    ELGR,

    #[graphql(name = "EN")]
    EN,

    #[graphql(name = "EN_AE")]
    ENAE,

    #[graphql(name = "EN_AG")]
    ENAG,

    #[graphql(name = "EN_AI")]
    ENAI,

    #[graphql(name = "EN_AS")]
    ENAS,

    #[graphql(name = "EN_AT")]
    ENAT,

    #[graphql(name = "EN_AU")]
    ENAU,

    #[graphql(name = "EN_BB")]
    ENBB,

    #[graphql(name = "EN_BE")]
    ENBE,

    #[graphql(name = "EN_BI")]
    ENBI,

    #[graphql(name = "EN_BM")]
    ENBM,

    #[graphql(name = "EN_BS")]
    ENBS,

    #[graphql(name = "EN_BW")]
    ENBW,

    #[graphql(name = "EN_BZ")]
    ENBZ,

    #[graphql(name = "EN_CA")]
    ENCA,

    #[graphql(name = "EN_CC")]
    ENCC,

    #[graphql(name = "EN_CH")]
    ENCH,

    #[graphql(name = "EN_CK")]
    ENCK,

    #[graphql(name = "EN_CM")]
    ENCM,

    #[graphql(name = "EN_CX")]
    ENCX,

    #[graphql(name = "EN_CY")]
    ENCY,

    #[graphql(name = "EN_DE")]
    ENDE,

    #[graphql(name = "EN_DG")]
    ENDG,

    #[graphql(name = "EN_DK")]
    ENDK,

    #[graphql(name = "EN_DM")]
    ENDM,

    #[graphql(name = "EN_ER")]
    ENER,

    #[graphql(name = "EN_FI")]
    ENFI,

    #[graphql(name = "EN_FJ")]
    ENFJ,

    #[graphql(name = "EN_FK")]
    ENFK,

    #[graphql(name = "EN_FM")]
    ENFM,

    #[graphql(name = "EN_GB")]
    ENGB,

    #[graphql(name = "EN_GD")]
    ENGD,

    #[graphql(name = "EN_GG")]
    ENGG,

    #[graphql(name = "EN_GH")]
    ENGH,

    #[graphql(name = "EN_GI")]
    ENGI,

    #[graphql(name = "EN_GM")]
    ENGM,

    #[graphql(name = "EN_GU")]
    ENGU,

    #[graphql(name = "EN_GY")]
    ENGY,

    #[graphql(name = "EN_HK")]
    ENHK,

    #[graphql(name = "EN_IE")]
    ENIE,

    #[graphql(name = "EN_IL")]
    ENIL,

    #[graphql(name = "EN_IM")]
    ENIM,

    #[graphql(name = "EN_IN")]
    ENIN,

    #[graphql(name = "EN_IO")]
    ENIO,

    #[graphql(name = "EN_JE")]
    ENJE,

    #[graphql(name = "EN_JM")]
    ENJM,

    #[graphql(name = "EN_KE")]
    ENKE,

    #[graphql(name = "EN_KI")]
    ENKI,

    #[graphql(name = "EN_KN")]
    ENKN,

    #[graphql(name = "EN_KY")]
    ENKY,

    #[graphql(name = "EN_LC")]
    ENLC,

    #[graphql(name = "EN_LR")]
    ENLR,

    #[graphql(name = "EN_LS")]
    ENLS,

    #[graphql(name = "EN_MG")]
    ENMG,

    #[graphql(name = "EN_MH")]
    ENMH,

    #[graphql(name = "EN_MO")]
    ENMO,

    #[graphql(name = "EN_MP")]
    ENMP,

    #[graphql(name = "EN_MS")]
    ENMS,

    #[graphql(name = "EN_MT")]
    ENMT,

    #[graphql(name = "EN_MU")]
    ENMU,

    #[graphql(name = "EN_MW")]
    ENMW,

    #[graphql(name = "EN_MY")]
    ENMY,

    #[graphql(name = "EN_NA")]
    ENNA,

    #[graphql(name = "EN_NF")]
    ENNF,

    #[graphql(name = "EN_NG")]
    ENNG,

    #[graphql(name = "EN_NL")]
    ENNL,

    #[graphql(name = "EN_NR")]
    ENNR,

    #[graphql(name = "EN_NU")]
    ENNU,

    #[graphql(name = "EN_NZ")]
    ENNZ,

    #[graphql(name = "EN_PG")]
    ENPG,

    #[graphql(name = "EN_PH")]
    ENPH,

    #[graphql(name = "EN_PK")]
    ENPK,

    #[graphql(name = "EN_PN")]
    ENPN,

    #[graphql(name = "EN_PR")]
    ENPR,

    #[graphql(name = "EN_PW")]
    ENPW,

    #[graphql(name = "EN_RW")]
    ENRW,

    #[graphql(name = "EN_SB")]
    ENSB,

    #[graphql(name = "EN_SC")]
    ENSC,

    #[graphql(name = "EN_SD")]
    ENSD,

    #[graphql(name = "EN_SE")]
    ENSE,

    #[graphql(name = "EN_SG")]
    ENSG,

    #[graphql(name = "EN_SH")]
    ENSH,

    #[graphql(name = "EN_SI")]
    ENSI,

    #[graphql(name = "EN_SL")]
    ENSL,

    #[graphql(name = "EN_SS")]
    ENSS,

    #[graphql(name = "EN_SX")]
    ENSX,

    #[graphql(name = "EN_SZ")]
    ENSZ,

    #[graphql(name = "EN_TC")]
    ENTC,

    #[graphql(name = "EN_TK")]
    ENTK,

    #[graphql(name = "EN_TO")]
    ENTO,

    #[graphql(name = "EN_TT")]
    ENTT,

    #[graphql(name = "EN_TV")]
    ENTV,

    #[graphql(name = "EN_TZ")]
    ENTZ,

    #[graphql(name = "EN_UG")]
    ENUG,

    #[graphql(name = "EN_UM")]
    ENUM,

    #[graphql(name = "EN_US")]
    ENUS,

    #[graphql(name = "EN_VC")]
    ENVC,

    #[graphql(name = "EN_VG")]
    ENVG,

    #[graphql(name = "EN_VI")]
    ENVI,

    #[graphql(name = "EN_VU")]
    ENVU,

    #[graphql(name = "EN_WS")]
    ENWS,

    #[graphql(name = "EN_ZA")]
    ENZA,

    #[graphql(name = "EN_ZM")]
    ENZM,

    #[graphql(name = "EN_ZW")]
    ENZW,

    #[graphql(name = "EO")]
    EO,

    #[graphql(name = "ES")]
    ES,

    #[graphql(name = "ES_AR")]
    ESAR,

    #[graphql(name = "ES_BO")]
    ESBO,

    #[graphql(name = "ES_BR")]
    ESBR,

    #[graphql(name = "ES_BZ")]
    ESBZ,

    #[graphql(name = "ES_CL")]
    ESCL,

    #[graphql(name = "ES_CO")]
    ESCO,

    #[graphql(name = "ES_CR")]
    ESCR,

    #[graphql(name = "ES_CU")]
    ESCU,

    #[graphql(name = "ES_DO")]
    ESDO,

    #[graphql(name = "ES_EA")]
    ESEA,

    #[graphql(name = "ES_EC")]
    ESEC,

    #[graphql(name = "ES_ES")]
    ESES,

    #[graphql(name = "ES_GQ")]
    ESGQ,

    #[graphql(name = "ES_GT")]
    ESGT,

    #[graphql(name = "ES_HN")]
    ESHN,

    #[graphql(name = "ES_IC")]
    ESIC,

    #[graphql(name = "ES_MX")]
    ESMX,

    #[graphql(name = "ES_NI")]
    ESNI,

    #[graphql(name = "ES_PA")]
    ESPA,

    #[graphql(name = "ES_PE")]
    ESPE,

    #[graphql(name = "ES_PH")]
    ESPH,

    #[graphql(name = "ES_PR")]
    ESPR,

    #[graphql(name = "ES_PY")]
    ESPY,

    #[graphql(name = "ES_SV")]
    ESSV,

    #[graphql(name = "ES_US")]
    ESUS,

    #[graphql(name = "ES_UY")]
    ESUY,

    #[graphql(name = "ES_VE")]
    ESVE,

    #[graphql(name = "ET")]
    ET,

    #[graphql(name = "ET_EE")]
    ETEE,

    #[graphql(name = "EU")]
    EU,

    #[graphql(name = "EU_ES")]
    EUES,

    #[graphql(name = "EWO")]
    EWO,

    #[graphql(name = "EWO_CM")]
    EWOCM,

    #[graphql(name = "FA")]
    FA,

    #[graphql(name = "FA_AF")]
    FAAF,

    #[graphql(name = "FA_IR")]
    FAIR,

    #[graphql(name = "FF")]
    FF,

    #[graphql(name = "FF_ADLM")]
    FFADLM,

    #[graphql(name = "FF_ADLM_BF")]
    FFADLMBF,

    #[graphql(name = "FF_ADLM_CM")]
    FFADLMCM,

    #[graphql(name = "FF_ADLM_GH")]
    FFADLMGH,

    #[graphql(name = "FF_ADLM_GM")]
    FFADLMGM,

    #[graphql(name = "FF_ADLM_GN")]
    FFADLMGN,

    #[graphql(name = "FF_ADLM_GW")]
    FFADLMGW,

    #[graphql(name = "FF_ADLM_LR")]
    FFADLMLR,

    #[graphql(name = "FF_ADLM_MR")]
    FFADLMMR,

    #[graphql(name = "FF_ADLM_NE")]
    FFADLMNE,

    #[graphql(name = "FF_ADLM_NG")]
    FFADLMNG,

    #[graphql(name = "FF_ADLM_SL")]
    FFADLMSL,

    #[graphql(name = "FF_ADLM_SN")]
    FFADLMSN,

    #[graphql(name = "FF_LATN")]
    FFLATN,

    #[graphql(name = "FF_LATN_BF")]
    FFLATNBF,

    #[graphql(name = "FF_LATN_CM")]
    FFLATNCM,

    #[graphql(name = "FF_LATN_GH")]
    FFLATNGH,

    #[graphql(name = "FF_LATN_GM")]
    FFLATNGM,

    #[graphql(name = "FF_LATN_GN")]
    FFLATNGN,

    #[graphql(name = "FF_LATN_GW")]
    FFLATNGW,

    #[graphql(name = "FF_LATN_LR")]
    FFLATNLR,

    #[graphql(name = "FF_LATN_MR")]
    FFLATNMR,

    #[graphql(name = "FF_LATN_NE")]
    FFLATNNE,

    #[graphql(name = "FF_LATN_NG")]
    FFLATNNG,

    #[graphql(name = "FF_LATN_SL")]
    FFLATNSL,

    #[graphql(name = "FF_LATN_SN")]
    FFLATNSN,

    #[graphql(name = "FI")]
    FI,

    #[graphql(name = "FI_FI")]
    FIFI,

    #[graphql(name = "FIL")]
    FIL,

    #[graphql(name = "FIL_PH")]
    FILPH,

    #[graphql(name = "FO")]
    FO,

    #[graphql(name = "FO_DK")]
    FODK,

    #[graphql(name = "FO_FO")]
    FOFO,

    #[graphql(name = "FR")]
    FR,

    #[graphql(name = "FR_BE")]
    FRBE,

    #[graphql(name = "FR_BF")]
    FRBF,

    #[graphql(name = "FR_BI")]
    FRBI,

    #[graphql(name = "FR_BJ")]
    FRBJ,

    #[graphql(name = "FR_BL")]
    FRBL,

    #[graphql(name = "FR_CA")]
    FRCA,

    #[graphql(name = "FR_CD")]
    FRCD,

    #[graphql(name = "FR_CF")]
    FRCF,

    #[graphql(name = "FR_CG")]
    FRCG,

    #[graphql(name = "FR_CH")]
    FRCH,

    #[graphql(name = "FR_CI")]
    FRCI,

    #[graphql(name = "FR_CM")]
    FRCM,

    #[graphql(name = "FR_DJ")]
    FRDJ,

    #[graphql(name = "FR_DZ")]
    FRDZ,

    #[graphql(name = "FR_FR")]
    FRFR,

    #[graphql(name = "FR_GA")]
    FRGA,

    #[graphql(name = "FR_GF")]
    FRGF,

    #[graphql(name = "FR_GN")]
    FRGN,

    #[graphql(name = "FR_GP")]
    FRGP,

    #[graphql(name = "FR_GQ")]
    FRGQ,

    #[graphql(name = "FR_HT")]
    FRHT,

    #[graphql(name = "FR_KM")]
    FRKM,

    #[graphql(name = "FR_LU")]
    FRLU,

    #[graphql(name = "FR_MA")]
    FRMA,

    #[graphql(name = "FR_MC")]
    FRMC,

    #[graphql(name = "FR_MF")]
    FRMF,

    #[graphql(name = "FR_MG")]
    FRMG,

    #[graphql(name = "FR_ML")]
    FRML,

    #[graphql(name = "FR_MQ")]
    FRMQ,

    #[graphql(name = "FR_MR")]
    FRMR,

    #[graphql(name = "FR_MU")]
    FRMU,

    #[graphql(name = "FR_NC")]
    FRNC,

    #[graphql(name = "FR_NE")]
    FRNE,

    #[graphql(name = "FR_PF")]
    FRPF,

    #[graphql(name = "FR_PM")]
    FRPM,

    #[graphql(name = "FR_RE")]
    FRRE,

    #[graphql(name = "FR_RW")]
    FRRW,

    #[graphql(name = "FR_SC")]
    FRSC,

    #[graphql(name = "FR_SN")]
    FRSN,

    #[graphql(name = "FR_SY")]
    FRSY,

    #[graphql(name = "FR_TD")]
    FRTD,

    #[graphql(name = "FR_TG")]
    FRTG,

    #[graphql(name = "FR_TN")]
    FRTN,

    #[graphql(name = "FR_VU")]
    FRVU,

    #[graphql(name = "FR_WF")]
    FRWF,

    #[graphql(name = "FR_YT")]
    FRYT,

    #[graphql(name = "FUR")]
    FUR,

    #[graphql(name = "FUR_IT")]
    FURIT,

    #[graphql(name = "FY")]
    FY,

    #[graphql(name = "FY_NL")]
    FYNL,

    #[graphql(name = "GA")]
    GA,

    #[graphql(name = "GA_GB")]
    GAGB,

    #[graphql(name = "GA_IE")]
    GAIE,

    #[graphql(name = "GD")]
    GD,

    #[graphql(name = "GD_GB")]
    GDGB,

    #[graphql(name = "GL")]
    GL,

    #[graphql(name = "GL_ES")]
    GLES,

    #[graphql(name = "GSW")]
    GSW,

    #[graphql(name = "GSW_CH")]
    GSWCH,

    #[graphql(name = "GSW_FR")]
    GSWFR,

    #[graphql(name = "GSW_LI")]
    GSWLI,

    #[graphql(name = "GU")]
    GU,

    #[graphql(name = "GU_IN")]
    GUIN,

    #[graphql(name = "GUZ")]
    GUZ,

    #[graphql(name = "GUZ_KE")]
    GUZKE,

    #[graphql(name = "GV")]
    GV,

    #[graphql(name = "GV_IM")]
    GVIM,

    #[graphql(name = "HA")]
    HA,

    #[graphql(name = "HA_GH")]
    HAGH,

    #[graphql(name = "HA_NE")]
    HANE,

    #[graphql(name = "HA_NG")]
    HANG,

    #[graphql(name = "HAW")]
    HAW,

    #[graphql(name = "HAW_US")]
    HAWUS,

    #[graphql(name = "HE")]
    HE,

    #[graphql(name = "HE_IL")]
    HEIL,

    #[graphql(name = "HI")]
    HI,

    #[graphql(name = "HI_IN")]
    HIIN,

    #[graphql(name = "HR")]
    HR,

    #[graphql(name = "HR_BA")]
    HRBA,

    #[graphql(name = "HR_HR")]
    HRHR,

    #[graphql(name = "HSB")]
    HSB,

    #[graphql(name = "HSB_DE")]
    HSBDE,

    #[graphql(name = "HU")]
    HU,

    #[graphql(name = "HU_HU")]
    HUHU,

    #[graphql(name = "HY")]
    HY,

    #[graphql(name = "HY_AM")]
    HYAM,

    #[graphql(name = "IA")]
    IA,

    #[graphql(name = "ID")]
    ID,

    #[graphql(name = "ID_ID")]
    IDID,

    #[graphql(name = "IG")]
    IG,

    #[graphql(name = "IG_NG")]
    IGNG,

    #[graphql(name = "II")]
    II,

    #[graphql(name = "II_CN")]
    IICN,

    #[graphql(name = "IS")]
    IS,

    #[graphql(name = "IS_IS")]
    ISIS,

    #[graphql(name = "IT")]
    IT,

    #[graphql(name = "IT_CH")]
    ITCH,

    #[graphql(name = "IT_IT")]
    ITIT,

    #[graphql(name = "IT_SM")]
    ITSM,

    #[graphql(name = "IT_VA")]
    ITVA,

    #[graphql(name = "JA")]
    JA,

    #[graphql(name = "JA_JP")]
    JAJP,

    #[graphql(name = "JGO")]
    JGO,

    #[graphql(name = "JGO_CM")]
    JGOCM,

    #[graphql(name = "JMC")]
    JMC,

    #[graphql(name = "JMC_TZ")]
    JMCTZ,

    #[graphql(name = "JV")]
    JV,

    #[graphql(name = "JV_ID")]
    JVID,

    #[graphql(name = "KA")]
    KA,

    #[graphql(name = "KA_GE")]
    KAGE,

    #[graphql(name = "KAB")]
    KAB,

    #[graphql(name = "KAB_DZ")]
    KABDZ,

    #[graphql(name = "KAM")]
    KAM,

    #[graphql(name = "KAM_KE")]
    KAMKE,

    #[graphql(name = "KDE")]
    KDE,

    #[graphql(name = "KDE_TZ")]
    KDETZ,

    #[graphql(name = "KEA")]
    KEA,

    #[graphql(name = "KEA_CV")]
    KEACV,

    #[graphql(name = "KHQ")]
    KHQ,

    #[graphql(name = "KHQ_ML")]
    KHQML,

    #[graphql(name = "KI")]
    KI,

    #[graphql(name = "KI_KE")]
    KIKE,

    #[graphql(name = "KK")]
    KK,

    #[graphql(name = "KK_KZ")]
    KKKZ,

    #[graphql(name = "KKJ")]
    KKJ,

    #[graphql(name = "KKJ_CM")]
    KKJCM,

    #[graphql(name = "KL")]
    KL,

    #[graphql(name = "KL_GL")]
    KLGL,

    #[graphql(name = "KLN")]
    KLN,

    #[graphql(name = "KLN_KE")]
    KLNKE,

    #[graphql(name = "KM")]
    KM,

    #[graphql(name = "KM_KH")]
    KMKH,

    #[graphql(name = "KN")]
    KN,

    #[graphql(name = "KN_IN")]
    KNIN,

    #[graphql(name = "KO")]
    KO,

    #[graphql(name = "KO_KP")]
    KOKP,

    #[graphql(name = "KO_KR")]
    KOKR,

    #[graphql(name = "KOK")]
    KOK,

    #[graphql(name = "KOK_IN")]
    KOKIN,

    #[graphql(name = "KS")]
    KS,

    #[graphql(name = "KS_ARAB")]
    KSARAB,

    #[graphql(name = "KS_ARAB_IN")]
    KSARABIN,

    #[graphql(name = "KSB")]
    KSB,

    #[graphql(name = "KSB_TZ")]
    KSBTZ,

    #[graphql(name = "KSF")]
    KSF,

    #[graphql(name = "KSF_CM")]
    KSFCM,

    #[graphql(name = "KSH")]
    KSH,

    #[graphql(name = "KSH_DE")]
    KSHDE,

    #[graphql(name = "KU")]
    KU,

    #[graphql(name = "KU_TR")]
    KUTR,

    #[graphql(name = "KW")]
    KW,

    #[graphql(name = "KW_GB")]
    KWGB,

    #[graphql(name = "KY")]
    KY,

    #[graphql(name = "KY_KG")]
    KYKG,

    #[graphql(name = "LAG")]
    LAG,

    #[graphql(name = "LAG_TZ")]
    LAGTZ,

    #[graphql(name = "LB")]
    LB,

    #[graphql(name = "LB_LU")]
    LBLU,

    #[graphql(name = "LG")]
    LG,

    #[graphql(name = "LG_UG")]
    LGUG,

    #[graphql(name = "LKT")]
    LKT,

    #[graphql(name = "LKT_US")]
    LKTUS,

    #[graphql(name = "LN")]
    LN,

    #[graphql(name = "LN_AO")]
    LNAO,

    #[graphql(name = "LN_CD")]
    LNCD,

    #[graphql(name = "LN_CF")]
    LNCF,

    #[graphql(name = "LN_CG")]
    LNCG,

    #[graphql(name = "LO")]
    LO,

    #[graphql(name = "LO_LA")]
    LOLA,

    #[graphql(name = "LRC")]
    LRC,

    #[graphql(name = "LRC_IQ")]
    LRCIQ,

    #[graphql(name = "LRC_IR")]
    LRCIR,

    #[graphql(name = "LT")]
    LT,

    #[graphql(name = "LT_LT")]
    LTLT,

    #[graphql(name = "LU")]
    LU,

    #[graphql(name = "LU_CD")]
    LUCD,

    #[graphql(name = "LUO")]
    LUO,

    #[graphql(name = "LUO_KE")]
    LUOKE,

    #[graphql(name = "LUY")]
    LUY,

    #[graphql(name = "LUY_KE")]
    LUYKE,

    #[graphql(name = "LV")]
    LV,

    #[graphql(name = "LV_LV")]
    LVLV,

    #[graphql(name = "MAI")]
    MAI,

    #[graphql(name = "MAI_IN")]
    MAIIN,

    #[graphql(name = "MAS")]
    MAS,

    #[graphql(name = "MAS_KE")]
    MASKE,

    #[graphql(name = "MAS_TZ")]
    MASTZ,

    #[graphql(name = "MER")]
    MER,

    #[graphql(name = "MER_KE")]
    MERKE,

    #[graphql(name = "MFE")]
    MFE,

    #[graphql(name = "MFE_MU")]
    MFEMU,

    #[graphql(name = "MG")]
    MG,

    #[graphql(name = "MG_MG")]
    MGMG,

    #[graphql(name = "MGH")]
    MGH,

    #[graphql(name = "MGH_MZ")]
    MGHMZ,

    #[graphql(name = "MGO")]
    MGO,

    #[graphql(name = "MGO_CM")]
    MGOCM,

    #[graphql(name = "MI")]
    MI,

    #[graphql(name = "MI_NZ")]
    MINZ,

    #[graphql(name = "MK")]
    MK,

    #[graphql(name = "MK_MK")]
    MKMK,

    #[graphql(name = "ML")]
    ML,

    #[graphql(name = "ML_IN")]
    MLIN,

    #[graphql(name = "MN")]
    MN,

    #[graphql(name = "MN_MN")]
    MNMN,

    #[graphql(name = "MNI")]
    MNI,

    #[graphql(name = "MNI_BENG")]
    MNIBENG,

    #[graphql(name = "MNI_BENG_IN")]
    MNIBENGIN,

    #[graphql(name = "MR")]
    MR,

    #[graphql(name = "MR_IN")]
    MRIN,

    #[graphql(name = "MS")]
    MS,

    #[graphql(name = "MS_BN")]
    MSBN,

    #[graphql(name = "MS_ID")]
    MSID,

    #[graphql(name = "MS_MY")]
    MSMY,

    #[graphql(name = "MS_SG")]
    MSSG,

    #[graphql(name = "MT")]
    MT,

    #[graphql(name = "MT_MT")]
    MTMT,

    #[graphql(name = "MUA")]
    MUA,

    #[graphql(name = "MUA_CM")]
    MUACM,

    #[graphql(name = "MY")]
    MY,

    #[graphql(name = "MY_MM")]
    MYMM,

    #[graphql(name = "MZN")]
    MZN,

    #[graphql(name = "MZN_IR")]
    MZNIR,

    #[graphql(name = "NAQ")]
    NAQ,

    #[graphql(name = "NAQ_NA")]
    NAQNA,

    #[graphql(name = "NB")]
    NB,

    #[graphql(name = "NB_NO")]
    NBNO,

    #[graphql(name = "NB_SJ")]
    NBSJ,

    #[graphql(name = "ND")]
    ND,

    #[graphql(name = "ND_ZW")]
    NDZW,

    #[graphql(name = "NDS")]
    NDS,

    #[graphql(name = "NDS_DE")]
    NDSDE,

    #[graphql(name = "NDS_NL")]
    NDSNL,

    #[graphql(name = "NE")]
    NE,

    #[graphql(name = "NE_IN")]
    NEIN,

    #[graphql(name = "NE_NP")]
    NENP,

    #[graphql(name = "NL")]
    NL,

    #[graphql(name = "NL_AW")]
    NLAW,

    #[graphql(name = "NL_BE")]
    NLBE,

    #[graphql(name = "NL_BQ")]
    NLBQ,

    #[graphql(name = "NL_CW")]
    NLCW,

    #[graphql(name = "NL_NL")]
    NLNL,

    #[graphql(name = "NL_SR")]
    NLSR,

    #[graphql(name = "NL_SX")]
    NLSX,

    #[graphql(name = "NMG")]
    NMG,

    #[graphql(name = "NMG_CM")]
    NMGCM,

    #[graphql(name = "NN")]
    NN,

    #[graphql(name = "NN_NO")]
    NNNO,

    #[graphql(name = "NNH")]
    NNH,

    #[graphql(name = "NNH_CM")]
    NNHCM,

    #[graphql(name = "NUS")]
    NUS,

    #[graphql(name = "NUS_SS")]
    NUSSS,

    #[graphql(name = "NYN")]
    NYN,

    #[graphql(name = "NYN_UG")]
    NYNUG,

    #[graphql(name = "OM")]
    OM,

    #[graphql(name = "OM_ET")]
    OMET,

    #[graphql(name = "OM_KE")]
    OMKE,

    #[graphql(name = "OR")]
    OR,

    #[graphql(name = "OR_IN")]
    ORIN,

    #[graphql(name = "OS")]
    OS,

    #[graphql(name = "OS_GE")]
    OSGE,

    #[graphql(name = "OS_RU")]
    OSRU,

    #[graphql(name = "PA")]
    PA,

    #[graphql(name = "PA_ARAB")]
    PAARAB,

    #[graphql(name = "PA_ARAB_PK")]
    PAARABPK,

    #[graphql(name = "PA_GURU")]
    PAGURU,

    #[graphql(name = "PA_GURU_IN")]
    PAGURUIN,

    #[graphql(name = "PCM")]
    PCM,

    #[graphql(name = "PCM_NG")]
    PCMNG,

    #[graphql(name = "PL")]
    PL,

    #[graphql(name = "PL_PL")]
    PLPL,

    #[graphql(name = "PRG")]
    PRG,

    #[graphql(name = "PS")]
    PS,

    #[graphql(name = "PS_AF")]
    PSAF,

    #[graphql(name = "PS_PK")]
    PSPK,

    #[graphql(name = "PT")]
    PT,

    #[graphql(name = "PT_AO")]
    PTAO,

    #[graphql(name = "PT_BR")]
    PTBR,

    #[graphql(name = "PT_CH")]
    PTCH,

    #[graphql(name = "PT_CV")]
    PTCV,

    #[graphql(name = "PT_GQ")]
    PTGQ,

    #[graphql(name = "PT_GW")]
    PTGW,

    #[graphql(name = "PT_LU")]
    PTLU,

    #[graphql(name = "PT_MO")]
    PTMO,

    #[graphql(name = "PT_MZ")]
    PTMZ,

    #[graphql(name = "PT_PT")]
    PTPT,

    #[graphql(name = "PT_ST")]
    PTST,

    #[graphql(name = "PT_TL")]
    PTTL,

    #[graphql(name = "QU")]
    QU,

    #[graphql(name = "QU_BO")]
    QUBO,

    #[graphql(name = "QU_EC")]
    QUEC,

    #[graphql(name = "QU_PE")]
    QUPE,

    #[graphql(name = "RM")]
    RM,

    #[graphql(name = "RM_CH")]
    RMCH,

    #[graphql(name = "RN")]
    RN,

    #[graphql(name = "RN_BI")]
    RNBI,

    #[graphql(name = "RO")]
    RO,

    #[graphql(name = "RO_MD")]
    ROMD,

    #[graphql(name = "RO_RO")]
    RORO,

    #[graphql(name = "ROF")]
    ROF,

    #[graphql(name = "ROF_TZ")]
    ROFTZ,

    #[graphql(name = "RU")]
    RU,

    #[graphql(name = "RU_BY")]
    RUBY,

    #[graphql(name = "RU_KG")]
    RUKG,

    #[graphql(name = "RU_KZ")]
    RUKZ,

    #[graphql(name = "RU_MD")]
    RUMD,

    #[graphql(name = "RU_RU")]
    RURU,

    #[graphql(name = "RU_UA")]
    RUUA,

    #[graphql(name = "RW")]
    RW,

    #[graphql(name = "RW_RW")]
    RWRW,

    #[graphql(name = "RWK")]
    RWK,

    #[graphql(name = "RWK_TZ")]
    RWKTZ,

    #[graphql(name = "SAH")]
    SAH,

    #[graphql(name = "SAH_RU")]
    SAHRU,

    #[graphql(name = "SAQ")]
    SAQ,

    #[graphql(name = "SAQ_KE")]
    SAQKE,

    #[graphql(name = "SAT")]
    SAT,

    #[graphql(name = "SAT_OLCK")]
    SATOLCK,

    #[graphql(name = "SAT_OLCK_IN")]
    SATOLCKIN,

    #[graphql(name = "SBP")]
    SBP,

    #[graphql(name = "SBP_TZ")]
    SBPTZ,

    #[graphql(name = "SD")]
    SD,

    #[graphql(name = "SD_ARAB")]
    SDARAB,

    #[graphql(name = "SD_ARAB_PK")]
    SDARABPK,

    #[graphql(name = "SD_DEVA")]
    SDDEVA,

    #[graphql(name = "SD_DEVA_IN")]
    SDDEVAIN,

    #[graphql(name = "SE")]
    SE,

    #[graphql(name = "SE_FI")]
    SEFI,

    #[graphql(name = "SE_NO")]
    SENO,

    #[graphql(name = "SE_SE")]
    SESE,

    #[graphql(name = "SEH")]
    SEH,

    #[graphql(name = "SEH_MZ")]
    SEHMZ,

    #[graphql(name = "SES")]
    SES,

    #[graphql(name = "SES_ML")]
    SESML,

    #[graphql(name = "SG")]
    SG,

    #[graphql(name = "SG_CF")]
    SGCF,

    #[graphql(name = "SHI")]
    SHI,

    #[graphql(name = "SHI_LATN")]
    SHILATN,

    #[graphql(name = "SHI_LATN_MA")]
    SHILATNMA,

    #[graphql(name = "SHI_TFNG")]
    SHITFNG,

    #[graphql(name = "SHI_TFNG_MA")]
    SHITFNGMA,

    #[graphql(name = "SI")]
    SI,

    #[graphql(name = "SI_LK")]
    SILK,

    #[graphql(name = "SK")]
    SK,

    #[graphql(name = "SK_SK")]
    SKSK,

    #[graphql(name = "SL")]
    SL,

    #[graphql(name = "SL_SI")]
    SLSI,

    #[graphql(name = "SMN")]
    SMN,

    #[graphql(name = "SMN_FI")]
    SMNFI,

    #[graphql(name = "SN")]
    SN,

    #[graphql(name = "SN_ZW")]
    SNZW,

    #[graphql(name = "SO")]
    SO,

    #[graphql(name = "SO_DJ")]
    SODJ,

    #[graphql(name = "SO_ET")]
    SOET,

    #[graphql(name = "SO_KE")]
    SOKE,

    #[graphql(name = "SO_SO")]
    SOSO,

    #[graphql(name = "SQ")]
    SQ,

    #[graphql(name = "SQ_AL")]
    SQAL,

    #[graphql(name = "SQ_MK")]
    SQMK,

    #[graphql(name = "SQ_XK")]
    SQXK,

    #[graphql(name = "SR")]
    SR,

    #[graphql(name = "SR_CYRL")]
    SRCYRL,

    #[graphql(name = "SR_CYRL_BA")]
    SRCYRLBA,

    #[graphql(name = "SR_CYRL_ME")]
    SRCYRLME,

    #[graphql(name = "SR_CYRL_RS")]
    SRCYRLRS,

    #[graphql(name = "SR_CYRL_XK")]
    SRCYRLXK,

    #[graphql(name = "SR_LATN")]
    SRLATN,

    #[graphql(name = "SR_LATN_BA")]
    SRLATNBA,

    #[graphql(name = "SR_LATN_ME")]
    SRLATNME,

    #[graphql(name = "SR_LATN_RS")]
    SRLATNRS,

    #[graphql(name = "SR_LATN_XK")]
    SRLATNXK,

    #[graphql(name = "SU")]
    SU,

    #[graphql(name = "SU_LATN")]
    SULATN,

    #[graphql(name = "SU_LATN_ID")]
    SULATNID,

    #[graphql(name = "SV")]
    SV,

    #[graphql(name = "SV_AX")]
    SVAX,

    #[graphql(name = "SV_FI")]
    SVFI,

    #[graphql(name = "SV_SE")]
    SVSE,

    #[graphql(name = "SW")]
    SW,

    #[graphql(name = "SW_CD")]
    SWCD,

    #[graphql(name = "SW_KE")]
    SWKE,

    #[graphql(name = "SW_TZ")]
    SWTZ,

    #[graphql(name = "SW_UG")]
    SWUG,

    #[graphql(name = "TA")]
    TA,

    #[graphql(name = "TA_IN")]
    TAIN,

    #[graphql(name = "TA_LK")]
    TALK,

    #[graphql(name = "TA_MY")]
    TAMY,

    #[graphql(name = "TA_SG")]
    TASG,

    #[graphql(name = "TE")]
    TE,

    #[graphql(name = "TE_IN")]
    TEIN,

    #[graphql(name = "TEO")]
    TEO,

    #[graphql(name = "TEO_KE")]
    TEOKE,

    #[graphql(name = "TEO_UG")]
    TEOUG,

    #[graphql(name = "TG")]
    TG,

    #[graphql(name = "TG_TJ")]
    TGTJ,

    #[graphql(name = "TH")]
    TH,

    #[graphql(name = "TH_TH")]
    THTH,

    #[graphql(name = "TI")]
    TI,

    #[graphql(name = "TI_ER")]
    TIER,

    #[graphql(name = "TI_ET")]
    TIET,

    #[graphql(name = "TK")]
    TK,

    #[graphql(name = "TK_TM")]
    TKTM,

    #[graphql(name = "TO")]
    TO,

    #[graphql(name = "TO_TO")]
    TOTO,

    #[graphql(name = "TR")]
    TR,

    #[graphql(name = "TR_CY")]
    TRCY,

    #[graphql(name = "TR_TR")]
    TRTR,

    #[graphql(name = "TT")]
    TT,

    #[graphql(name = "TT_RU")]
    TTRU,

    #[graphql(name = "TWQ")]
    TWQ,

    #[graphql(name = "TWQ_NE")]
    TWQNE,

    #[graphql(name = "TZM")]
    TZM,

    #[graphql(name = "TZM_MA")]
    TZMMA,

    #[graphql(name = "UG")]
    UG,

    #[graphql(name = "UG_CN")]
    UGCN,

    #[graphql(name = "UK")]
    UK,

    #[graphql(name = "UK_UA")]
    UKUA,

    #[graphql(name = "UR")]
    UR,

    #[graphql(name = "UR_IN")]
    URIN,

    #[graphql(name = "UR_PK")]
    URPK,

    #[graphql(name = "UZ")]
    UZ,

    #[graphql(name = "UZ_ARAB")]
    UZARAB,

    #[graphql(name = "UZ_ARAB_AF")]
    UZARABAF,

    #[graphql(name = "UZ_CYRL")]
    UZCYRL,

    #[graphql(name = "UZ_CYRL_UZ")]
    UZCYRLUZ,

    #[graphql(name = "UZ_LATN")]
    UZLATN,

    #[graphql(name = "UZ_LATN_UZ")]
    UZLATNUZ,

    #[graphql(name = "VAI")]
    VAI,

    #[graphql(name = "VAI_LATN")]
    VAILATN,

    #[graphql(name = "VAI_LATN_LR")]
    VAILATNLR,

    #[graphql(name = "VAI_VAII")]
    VAIVAII,

    #[graphql(name = "VAI_VAII_LR")]
    VAIVAIILR,

    #[graphql(name = "VI")]
    VI,

    #[graphql(name = "VI_VN")]
    VIVN,

    #[graphql(name = "VO")]
    VO,

    #[graphql(name = "VUN")]
    VUN,

    #[graphql(name = "VUN_TZ")]
    VUNTZ,

    #[graphql(name = "WAE")]
    WAE,

    #[graphql(name = "WAE_CH")]
    WAECH,

    #[graphql(name = "WO")]
    WO,

    #[graphql(name = "WO_SN")]
    WOSN,

    #[graphql(name = "XH")]
    XH,

    #[graphql(name = "XH_ZA")]
    XHZA,

    #[graphql(name = "XOG")]
    XOG,

    #[graphql(name = "XOG_UG")]
    XOGUG,

    #[graphql(name = "YAV")]
    YAV,

    #[graphql(name = "YAV_CM")]
    YAVCM,

    #[graphql(name = "YI")]
    YI,

    #[graphql(name = "YO")]
    YO,

    #[graphql(name = "YO_BJ")]
    YOBJ,

    #[graphql(name = "YO_NG")]
    YONG,

    #[graphql(name = "YUE")]
    YUE,

    #[graphql(name = "YUE_HANS")]
    YUEHANS,

    #[graphql(name = "YUE_HANS_CN")]
    YUEHANSCN,

    #[graphql(name = "YUE_HANT")]
    YUEHANT,

    #[graphql(name = "YUE_HANT_HK")]
    YUEHANTHK,

    #[graphql(name = "ZGH")]
    ZGH,

    #[graphql(name = "ZGH_MA")]
    ZGHMA,

    #[graphql(name = "ZH")]
    ZH,

    #[graphql(name = "ZH_HANS")]
    ZHHANS,

    #[graphql(name = "ZH_HANS_CN")]
    ZHHANSCN,

    #[graphql(name = "ZH_HANS_HK")]
    ZHHANSHK,

    #[graphql(name = "ZH_HANS_MO")]
    ZHHANSMO,

    #[graphql(name = "ZH_HANS_SG")]
    ZHHANSSG,

    #[graphql(name = "ZH_HANT")]
    ZHHANT,

    #[graphql(name = "ZH_HANT_HK")]
    ZHHANTHK,

    #[graphql(name = "ZH_HANT_MO")]
    ZHHANTMO,

    #[graphql(name = "ZH_HANT_TW")]
    ZHHANTTW,

    #[graphql(name = "ZU")]
    ZU,

    #[graphql(name = "ZU_ZA")]
    ZUZA,

}


/// Saleor `LanguageCodeEnum` value for a variant ('af-na').

pub fn language_code_value(v: &LanguageCodeEnum) -> &'static str {

    match v {

        LanguageCodeEnum::AF => "af",

        LanguageCodeEnum::AFNA => "af-na",

        LanguageCodeEnum::AFZA => "af-za",

        LanguageCodeEnum::AGQ => "agq",

        LanguageCodeEnum::AGQCM => "agq-cm",

        LanguageCodeEnum::AK => "ak",

        LanguageCodeEnum::AKGH => "ak-gh",

        LanguageCodeEnum::AM => "am",

        LanguageCodeEnum::AMET => "am-et",

        LanguageCodeEnum::AR => "ar",

        LanguageCodeEnum::ARAE => "ar-ae",

        LanguageCodeEnum::ARBH => "ar-bh",

        LanguageCodeEnum::ARDJ => "ar-dj",

        LanguageCodeEnum::ARDZ => "ar-dz",

        LanguageCodeEnum::AREG => "ar-eg",

        LanguageCodeEnum::AREH => "ar-eh",

        LanguageCodeEnum::ARER => "ar-er",

        LanguageCodeEnum::ARIL => "ar-il",

        LanguageCodeEnum::ARIQ => "ar-iq",

        LanguageCodeEnum::ARJO => "ar-jo",

        LanguageCodeEnum::ARKM => "ar-km",

        LanguageCodeEnum::ARKW => "ar-kw",

        LanguageCodeEnum::ARLB => "ar-lb",

        LanguageCodeEnum::ARLY => "ar-ly",

        LanguageCodeEnum::ARMA => "ar-ma",

        LanguageCodeEnum::ARMR => "ar-mr",

        LanguageCodeEnum::AROM => "ar-om",

        LanguageCodeEnum::ARPS => "ar-ps",

        LanguageCodeEnum::ARQA => "ar-qa",

        LanguageCodeEnum::ARSA => "ar-sa",

        LanguageCodeEnum::ARSD => "ar-sd",

        LanguageCodeEnum::ARSO => "ar-so",

        LanguageCodeEnum::ARSS => "ar-ss",

        LanguageCodeEnum::ARSY => "ar-sy",

        LanguageCodeEnum::ARTD => "ar-td",

        LanguageCodeEnum::ARTN => "ar-tn",

        LanguageCodeEnum::ARYE => "ar-ye",

        LanguageCodeEnum::AS => "as",

        LanguageCodeEnum::ASIN => "as-in",

        LanguageCodeEnum::ASA => "asa",

        LanguageCodeEnum::ASATZ => "asa-tz",

        LanguageCodeEnum::AST => "ast",

        LanguageCodeEnum::ASTES => "ast-es",

        LanguageCodeEnum::AZ => "az",

        LanguageCodeEnum::AZCYRL => "az-cyrl",

        LanguageCodeEnum::AZCYRLAZ => "az-cyrl-az",

        LanguageCodeEnum::AZLATN => "az-latn",

        LanguageCodeEnum::AZLATNAZ => "az-latn-az",

        LanguageCodeEnum::BAS => "bas",

        LanguageCodeEnum::BASCM => "bas-cm",

        LanguageCodeEnum::BE => "be",

        LanguageCodeEnum::BEBY => "be-by",

        LanguageCodeEnum::BEM => "bem",

        LanguageCodeEnum::BEMZM => "bem-zm",

        LanguageCodeEnum::BEZ => "bez",

        LanguageCodeEnum::BEZTZ => "bez-tz",

        LanguageCodeEnum::BG => "bg",

        LanguageCodeEnum::BGBG => "bg-bg",

        LanguageCodeEnum::BM => "bm",

        LanguageCodeEnum::BMML => "bm-ml",

        LanguageCodeEnum::BN => "bn",

        LanguageCodeEnum::BNBD => "bn-bd",

        LanguageCodeEnum::BNIN => "bn-in",

        LanguageCodeEnum::BO => "bo",

        LanguageCodeEnum::BOCN => "bo-cn",

        LanguageCodeEnum::BOIN => "bo-in",

        LanguageCodeEnum::BR => "br",

        LanguageCodeEnum::BRFR => "br-fr",

        LanguageCodeEnum::BRX => "brx",

        LanguageCodeEnum::BRXIN => "brx-in",

        LanguageCodeEnum::BS => "bs",

        LanguageCodeEnum::BSCYRL => "bs-cyrl",

        LanguageCodeEnum::BSCYRLBA => "bs-cyrl-ba",

        LanguageCodeEnum::BSLATN => "bs-latn",

        LanguageCodeEnum::BSLATNBA => "bs-latn-ba",

        LanguageCodeEnum::CA => "ca",

        LanguageCodeEnum::CAAD => "ca-ad",

        LanguageCodeEnum::CAES => "ca-es",

        LanguageCodeEnum::CAESVALENCIA => "ca-es-valencia",

        LanguageCodeEnum::CAFR => "ca-fr",

        LanguageCodeEnum::CAIT => "ca-it",

        LanguageCodeEnum::CCP => "ccp",

        LanguageCodeEnum::CCPBD => "ccp-bd",

        LanguageCodeEnum::CCPIN => "ccp-in",

        LanguageCodeEnum::CE => "ce",

        LanguageCodeEnum::CERU => "ce-ru",

        LanguageCodeEnum::CEB => "ceb",

        LanguageCodeEnum::CEBPH => "ceb-ph",

        LanguageCodeEnum::CGG => "cgg",

        LanguageCodeEnum::CGGUG => "cgg-ug",

        LanguageCodeEnum::CHR => "chr",

        LanguageCodeEnum::CHRUS => "chr-us",

        LanguageCodeEnum::CKB => "ckb",

        LanguageCodeEnum::CKBIQ => "ckb-iq",

        LanguageCodeEnum::CKBIR => "ckb-ir",

        LanguageCodeEnum::CS => "cs",

        LanguageCodeEnum::CSCZ => "cs-cz",

        LanguageCodeEnum::CU => "cu",

        LanguageCodeEnum::CURU => "cu-ru",

        LanguageCodeEnum::CY => "cy",

        LanguageCodeEnum::CYGB => "cy-gb",

        LanguageCodeEnum::DA => "da",

        LanguageCodeEnum::DADK => "da-dk",

        LanguageCodeEnum::DAGL => "da-gl",

        LanguageCodeEnum::DAV => "dav",

        LanguageCodeEnum::DAVKE => "dav-ke",

        LanguageCodeEnum::DE => "de",

        LanguageCodeEnum::DEAT => "de-at",

        LanguageCodeEnum::DEBE => "de-be",

        LanguageCodeEnum::DECH => "de-ch",

        LanguageCodeEnum::DEDE => "de-de",

        LanguageCodeEnum::DEIT => "de-it",

        LanguageCodeEnum::DELI => "de-li",

        LanguageCodeEnum::DELU => "de-lu",

        LanguageCodeEnum::DJE => "dje",

        LanguageCodeEnum::DJENE => "dje-ne",

        LanguageCodeEnum::DSB => "dsb",

        LanguageCodeEnum::DSBDE => "dsb-de",

        LanguageCodeEnum::DUA => "dua",

        LanguageCodeEnum::DUACM => "dua-cm",

        LanguageCodeEnum::DYO => "dyo",

        LanguageCodeEnum::DYOSN => "dyo-sn",

        LanguageCodeEnum::DZ => "dz",

        LanguageCodeEnum::DZBT => "dz-bt",

        LanguageCodeEnum::EBU => "ebu",

        LanguageCodeEnum::EBUKE => "ebu-ke",

        LanguageCodeEnum::EE => "ee",

        LanguageCodeEnum::EEGH => "ee-gh",

        LanguageCodeEnum::EETG => "ee-tg",

        LanguageCodeEnum::EL => "el",

        LanguageCodeEnum::ELCY => "el-cy",

        LanguageCodeEnum::ELGR => "el-gr",

        LanguageCodeEnum::EN => "en",

        LanguageCodeEnum::ENAE => "en-ae",

        LanguageCodeEnum::ENAG => "en-ag",

        LanguageCodeEnum::ENAI => "en-ai",

        LanguageCodeEnum::ENAS => "en-as",

        LanguageCodeEnum::ENAT => "en-at",

        LanguageCodeEnum::ENAU => "en-au",

        LanguageCodeEnum::ENBB => "en-bb",

        LanguageCodeEnum::ENBE => "en-be",

        LanguageCodeEnum::ENBI => "en-bi",

        LanguageCodeEnum::ENBM => "en-bm",

        LanguageCodeEnum::ENBS => "en-bs",

        LanguageCodeEnum::ENBW => "en-bw",

        LanguageCodeEnum::ENBZ => "en-bz",

        LanguageCodeEnum::ENCA => "en-ca",

        LanguageCodeEnum::ENCC => "en-cc",

        LanguageCodeEnum::ENCH => "en-ch",

        LanguageCodeEnum::ENCK => "en-ck",

        LanguageCodeEnum::ENCM => "en-cm",

        LanguageCodeEnum::ENCX => "en-cx",

        LanguageCodeEnum::ENCY => "en-cy",

        LanguageCodeEnum::ENDE => "en-de",

        LanguageCodeEnum::ENDG => "en-dg",

        LanguageCodeEnum::ENDK => "en-dk",

        LanguageCodeEnum::ENDM => "en-dm",

        LanguageCodeEnum::ENER => "en-er",

        LanguageCodeEnum::ENFI => "en-fi",

        LanguageCodeEnum::ENFJ => "en-fj",

        LanguageCodeEnum::ENFK => "en-fk",

        LanguageCodeEnum::ENFM => "en-fm",

        LanguageCodeEnum::ENGB => "en-gb",

        LanguageCodeEnum::ENGD => "en-gd",

        LanguageCodeEnum::ENGG => "en-gg",

        LanguageCodeEnum::ENGH => "en-gh",

        LanguageCodeEnum::ENGI => "en-gi",

        LanguageCodeEnum::ENGM => "en-gm",

        LanguageCodeEnum::ENGU => "en-gu",

        LanguageCodeEnum::ENGY => "en-gy",

        LanguageCodeEnum::ENHK => "en-hk",

        LanguageCodeEnum::ENIE => "en-ie",

        LanguageCodeEnum::ENIL => "en-il",

        LanguageCodeEnum::ENIM => "en-im",

        LanguageCodeEnum::ENIN => "en-in",

        LanguageCodeEnum::ENIO => "en-io",

        LanguageCodeEnum::ENJE => "en-je",

        LanguageCodeEnum::ENJM => "en-jm",

        LanguageCodeEnum::ENKE => "en-ke",

        LanguageCodeEnum::ENKI => "en-ki",

        LanguageCodeEnum::ENKN => "en-kn",

        LanguageCodeEnum::ENKY => "en-ky",

        LanguageCodeEnum::ENLC => "en-lc",

        LanguageCodeEnum::ENLR => "en-lr",

        LanguageCodeEnum::ENLS => "en-ls",

        LanguageCodeEnum::ENMG => "en-mg",

        LanguageCodeEnum::ENMH => "en-mh",

        LanguageCodeEnum::ENMO => "en-mo",

        LanguageCodeEnum::ENMP => "en-mp",

        LanguageCodeEnum::ENMS => "en-ms",

        LanguageCodeEnum::ENMT => "en-mt",

        LanguageCodeEnum::ENMU => "en-mu",

        LanguageCodeEnum::ENMW => "en-mw",

        LanguageCodeEnum::ENMY => "en-my",

        LanguageCodeEnum::ENNA => "en-na",

        LanguageCodeEnum::ENNF => "en-nf",

        LanguageCodeEnum::ENNG => "en-ng",

        LanguageCodeEnum::ENNL => "en-nl",

        LanguageCodeEnum::ENNR => "en-nr",

        LanguageCodeEnum::ENNU => "en-nu",

        LanguageCodeEnum::ENNZ => "en-nz",

        LanguageCodeEnum::ENPG => "en-pg",

        LanguageCodeEnum::ENPH => "en-ph",

        LanguageCodeEnum::ENPK => "en-pk",

        LanguageCodeEnum::ENPN => "en-pn",

        LanguageCodeEnum::ENPR => "en-pr",

        LanguageCodeEnum::ENPW => "en-pw",

        LanguageCodeEnum::ENRW => "en-rw",

        LanguageCodeEnum::ENSB => "en-sb",

        LanguageCodeEnum::ENSC => "en-sc",

        LanguageCodeEnum::ENSD => "en-sd",

        LanguageCodeEnum::ENSE => "en-se",

        LanguageCodeEnum::ENSG => "en-sg",

        LanguageCodeEnum::ENSH => "en-sh",

        LanguageCodeEnum::ENSI => "en-si",

        LanguageCodeEnum::ENSL => "en-sl",

        LanguageCodeEnum::ENSS => "en-ss",

        LanguageCodeEnum::ENSX => "en-sx",

        LanguageCodeEnum::ENSZ => "en-sz",

        LanguageCodeEnum::ENTC => "en-tc",

        LanguageCodeEnum::ENTK => "en-tk",

        LanguageCodeEnum::ENTO => "en-to",

        LanguageCodeEnum::ENTT => "en-tt",

        LanguageCodeEnum::ENTV => "en-tv",

        LanguageCodeEnum::ENTZ => "en-tz",

        LanguageCodeEnum::ENUG => "en-ug",

        LanguageCodeEnum::ENUM => "en-um",

        LanguageCodeEnum::ENUS => "en-us",

        LanguageCodeEnum::ENVC => "en-vc",

        LanguageCodeEnum::ENVG => "en-vg",

        LanguageCodeEnum::ENVI => "en-vi",

        LanguageCodeEnum::ENVU => "en-vu",

        LanguageCodeEnum::ENWS => "en-ws",

        LanguageCodeEnum::ENZA => "en-za",

        LanguageCodeEnum::ENZM => "en-zm",

        LanguageCodeEnum::ENZW => "en-zw",

        LanguageCodeEnum::EO => "eo",

        LanguageCodeEnum::ES => "es",

        LanguageCodeEnum::ESAR => "es-ar",

        LanguageCodeEnum::ESBO => "es-bo",

        LanguageCodeEnum::ESBR => "es-br",

        LanguageCodeEnum::ESBZ => "es-bz",

        LanguageCodeEnum::ESCL => "es-cl",

        LanguageCodeEnum::ESCO => "es-co",

        LanguageCodeEnum::ESCR => "es-cr",

        LanguageCodeEnum::ESCU => "es-cu",

        LanguageCodeEnum::ESDO => "es-do",

        LanguageCodeEnum::ESEA => "es-ea",

        LanguageCodeEnum::ESEC => "es-ec",

        LanguageCodeEnum::ESES => "es-es",

        LanguageCodeEnum::ESGQ => "es-gq",

        LanguageCodeEnum::ESGT => "es-gt",

        LanguageCodeEnum::ESHN => "es-hn",

        LanguageCodeEnum::ESIC => "es-ic",

        LanguageCodeEnum::ESMX => "es-mx",

        LanguageCodeEnum::ESNI => "es-ni",

        LanguageCodeEnum::ESPA => "es-pa",

        LanguageCodeEnum::ESPE => "es-pe",

        LanguageCodeEnum::ESPH => "es-ph",

        LanguageCodeEnum::ESPR => "es-pr",

        LanguageCodeEnum::ESPY => "es-py",

        LanguageCodeEnum::ESSV => "es-sv",

        LanguageCodeEnum::ESUS => "es-us",

        LanguageCodeEnum::ESUY => "es-uy",

        LanguageCodeEnum::ESVE => "es-ve",

        LanguageCodeEnum::ET => "et",

        LanguageCodeEnum::ETEE => "et-ee",

        LanguageCodeEnum::EU => "eu",

        LanguageCodeEnum::EUES => "eu-es",

        LanguageCodeEnum::EWO => "ewo",

        LanguageCodeEnum::EWOCM => "ewo-cm",

        LanguageCodeEnum::FA => "fa",

        LanguageCodeEnum::FAAF => "fa-af",

        LanguageCodeEnum::FAIR => "fa-ir",

        LanguageCodeEnum::FF => "ff",

        LanguageCodeEnum::FFADLM => "ff-adlm",

        LanguageCodeEnum::FFADLMBF => "ff-adlm-bf",

        LanguageCodeEnum::FFADLMCM => "ff-adlm-cm",

        LanguageCodeEnum::FFADLMGH => "ff-adlm-gh",

        LanguageCodeEnum::FFADLMGM => "ff-adlm-gm",

        LanguageCodeEnum::FFADLMGN => "ff-adlm-gn",

        LanguageCodeEnum::FFADLMGW => "ff-adlm-gw",

        LanguageCodeEnum::FFADLMLR => "ff-adlm-lr",

        LanguageCodeEnum::FFADLMMR => "ff-adlm-mr",

        LanguageCodeEnum::FFADLMNE => "ff-adlm-ne",

        LanguageCodeEnum::FFADLMNG => "ff-adlm-ng",

        LanguageCodeEnum::FFADLMSL => "ff-adlm-sl",

        LanguageCodeEnum::FFADLMSN => "ff-adlm-sn",

        LanguageCodeEnum::FFLATN => "ff-latn",

        LanguageCodeEnum::FFLATNBF => "ff-latn-bf",

        LanguageCodeEnum::FFLATNCM => "ff-latn-cm",

        LanguageCodeEnum::FFLATNGH => "ff-latn-gh",

        LanguageCodeEnum::FFLATNGM => "ff-latn-gm",

        LanguageCodeEnum::FFLATNGN => "ff-latn-gn",

        LanguageCodeEnum::FFLATNGW => "ff-latn-gw",

        LanguageCodeEnum::FFLATNLR => "ff-latn-lr",

        LanguageCodeEnum::FFLATNMR => "ff-latn-mr",

        LanguageCodeEnum::FFLATNNE => "ff-latn-ne",

        LanguageCodeEnum::FFLATNNG => "ff-latn-ng",

        LanguageCodeEnum::FFLATNSL => "ff-latn-sl",

        LanguageCodeEnum::FFLATNSN => "ff-latn-sn",

        LanguageCodeEnum::FI => "fi",

        LanguageCodeEnum::FIFI => "fi-fi",

        LanguageCodeEnum::FIL => "fil",

        LanguageCodeEnum::FILPH => "fil-ph",

        LanguageCodeEnum::FO => "fo",

        LanguageCodeEnum::FODK => "fo-dk",

        LanguageCodeEnum::FOFO => "fo-fo",

        LanguageCodeEnum::FR => "fr",

        LanguageCodeEnum::FRBE => "fr-be",

        LanguageCodeEnum::FRBF => "fr-bf",

        LanguageCodeEnum::FRBI => "fr-bi",

        LanguageCodeEnum::FRBJ => "fr-bj",

        LanguageCodeEnum::FRBL => "fr-bl",

        LanguageCodeEnum::FRCA => "fr-ca",

        LanguageCodeEnum::FRCD => "fr-cd",

        LanguageCodeEnum::FRCF => "fr-cf",

        LanguageCodeEnum::FRCG => "fr-cg",

        LanguageCodeEnum::FRCH => "fr-ch",

        LanguageCodeEnum::FRCI => "fr-ci",

        LanguageCodeEnum::FRCM => "fr-cm",

        LanguageCodeEnum::FRDJ => "fr-dj",

        LanguageCodeEnum::FRDZ => "fr-dz",

        LanguageCodeEnum::FRFR => "fr-fr",

        LanguageCodeEnum::FRGA => "fr-ga",

        LanguageCodeEnum::FRGF => "fr-gf",

        LanguageCodeEnum::FRGN => "fr-gn",

        LanguageCodeEnum::FRGP => "fr-gp",

        LanguageCodeEnum::FRGQ => "fr-gq",

        LanguageCodeEnum::FRHT => "fr-ht",

        LanguageCodeEnum::FRKM => "fr-km",

        LanguageCodeEnum::FRLU => "fr-lu",

        LanguageCodeEnum::FRMA => "fr-ma",

        LanguageCodeEnum::FRMC => "fr-mc",

        LanguageCodeEnum::FRMF => "fr-mf",

        LanguageCodeEnum::FRMG => "fr-mg",

        LanguageCodeEnum::FRML => "fr-ml",

        LanguageCodeEnum::FRMQ => "fr-mq",

        LanguageCodeEnum::FRMR => "fr-mr",

        LanguageCodeEnum::FRMU => "fr-mu",

        LanguageCodeEnum::FRNC => "fr-nc",

        LanguageCodeEnum::FRNE => "fr-ne",

        LanguageCodeEnum::FRPF => "fr-pf",

        LanguageCodeEnum::FRPM => "fr-pm",

        LanguageCodeEnum::FRRE => "fr-re",

        LanguageCodeEnum::FRRW => "fr-rw",

        LanguageCodeEnum::FRSC => "fr-sc",

        LanguageCodeEnum::FRSN => "fr-sn",

        LanguageCodeEnum::FRSY => "fr-sy",

        LanguageCodeEnum::FRTD => "fr-td",

        LanguageCodeEnum::FRTG => "fr-tg",

        LanguageCodeEnum::FRTN => "fr-tn",

        LanguageCodeEnum::FRVU => "fr-vu",

        LanguageCodeEnum::FRWF => "fr-wf",

        LanguageCodeEnum::FRYT => "fr-yt",

        LanguageCodeEnum::FUR => "fur",

        LanguageCodeEnum::FURIT => "fur-it",

        LanguageCodeEnum::FY => "fy",

        LanguageCodeEnum::FYNL => "fy-nl",

        LanguageCodeEnum::GA => "ga",

        LanguageCodeEnum::GAGB => "ga-gb",

        LanguageCodeEnum::GAIE => "ga-ie",

        LanguageCodeEnum::GD => "gd",

        LanguageCodeEnum::GDGB => "gd-gb",

        LanguageCodeEnum::GL => "gl",

        LanguageCodeEnum::GLES => "gl-es",

        LanguageCodeEnum::GSW => "gsw",

        LanguageCodeEnum::GSWCH => "gsw-ch",

        LanguageCodeEnum::GSWFR => "gsw-fr",

        LanguageCodeEnum::GSWLI => "gsw-li",

        LanguageCodeEnum::GU => "gu",

        LanguageCodeEnum::GUIN => "gu-in",

        LanguageCodeEnum::GUZ => "guz",

        LanguageCodeEnum::GUZKE => "guz-ke",

        LanguageCodeEnum::GV => "gv",

        LanguageCodeEnum::GVIM => "gv-im",

        LanguageCodeEnum::HA => "ha",

        LanguageCodeEnum::HAGH => "ha-gh",

        LanguageCodeEnum::HANE => "ha-ne",

        LanguageCodeEnum::HANG => "ha-ng",

        LanguageCodeEnum::HAW => "haw",

        LanguageCodeEnum::HAWUS => "haw-us",

        LanguageCodeEnum::HE => "he",

        LanguageCodeEnum::HEIL => "he-il",

        LanguageCodeEnum::HI => "hi",

        LanguageCodeEnum::HIIN => "hi-in",

        LanguageCodeEnum::HR => "hr",

        LanguageCodeEnum::HRBA => "hr-ba",

        LanguageCodeEnum::HRHR => "hr-hr",

        LanguageCodeEnum::HSB => "hsb",

        LanguageCodeEnum::HSBDE => "hsb-de",

        LanguageCodeEnum::HU => "hu",

        LanguageCodeEnum::HUHU => "hu-hu",

        LanguageCodeEnum::HY => "hy",

        LanguageCodeEnum::HYAM => "hy-am",

        LanguageCodeEnum::IA => "ia",

        LanguageCodeEnum::ID => "id",

        LanguageCodeEnum::IDID => "id-id",

        LanguageCodeEnum::IG => "ig",

        LanguageCodeEnum::IGNG => "ig-ng",

        LanguageCodeEnum::II => "ii",

        LanguageCodeEnum::IICN => "ii-cn",

        LanguageCodeEnum::IS => "is",

        LanguageCodeEnum::ISIS => "is-is",

        LanguageCodeEnum::IT => "it",

        LanguageCodeEnum::ITCH => "it-ch",

        LanguageCodeEnum::ITIT => "it-it",

        LanguageCodeEnum::ITSM => "it-sm",

        LanguageCodeEnum::ITVA => "it-va",

        LanguageCodeEnum::JA => "ja",

        LanguageCodeEnum::JAJP => "ja-jp",

        LanguageCodeEnum::JGO => "jgo",

        LanguageCodeEnum::JGOCM => "jgo-cm",

        LanguageCodeEnum::JMC => "jmc",

        LanguageCodeEnum::JMCTZ => "jmc-tz",

        LanguageCodeEnum::JV => "jv",

        LanguageCodeEnum::JVID => "jv-id",

        LanguageCodeEnum::KA => "ka",

        LanguageCodeEnum::KAGE => "ka-ge",

        LanguageCodeEnum::KAB => "kab",

        LanguageCodeEnum::KABDZ => "kab-dz",

        LanguageCodeEnum::KAM => "kam",

        LanguageCodeEnum::KAMKE => "kam-ke",

        LanguageCodeEnum::KDE => "kde",

        LanguageCodeEnum::KDETZ => "kde-tz",

        LanguageCodeEnum::KEA => "kea",

        LanguageCodeEnum::KEACV => "kea-cv",

        LanguageCodeEnum::KHQ => "khq",

        LanguageCodeEnum::KHQML => "khq-ml",

        LanguageCodeEnum::KI => "ki",

        LanguageCodeEnum::KIKE => "ki-ke",

        LanguageCodeEnum::KK => "kk",

        LanguageCodeEnum::KKKZ => "kk-kz",

        LanguageCodeEnum::KKJ => "kkj",

        LanguageCodeEnum::KKJCM => "kkj-cm",

        LanguageCodeEnum::KL => "kl",

        LanguageCodeEnum::KLGL => "kl-gl",

        LanguageCodeEnum::KLN => "kln",

        LanguageCodeEnum::KLNKE => "kln-ke",

        LanguageCodeEnum::KM => "km",

        LanguageCodeEnum::KMKH => "km-kh",

        LanguageCodeEnum::KN => "kn",

        LanguageCodeEnum::KNIN => "kn-in",

        LanguageCodeEnum::KO => "ko",

        LanguageCodeEnum::KOKP => "ko-kp",

        LanguageCodeEnum::KOKR => "ko-kr",

        LanguageCodeEnum::KOK => "kok",

        LanguageCodeEnum::KOKIN => "kok-in",

        LanguageCodeEnum::KS => "ks",

        LanguageCodeEnum::KSARAB => "ks-arab",

        LanguageCodeEnum::KSARABIN => "ks-arab-in",

        LanguageCodeEnum::KSB => "ksb",

        LanguageCodeEnum::KSBTZ => "ksb-tz",

        LanguageCodeEnum::KSF => "ksf",

        LanguageCodeEnum::KSFCM => "ksf-cm",

        LanguageCodeEnum::KSH => "ksh",

        LanguageCodeEnum::KSHDE => "ksh-de",

        LanguageCodeEnum::KU => "ku",

        LanguageCodeEnum::KUTR => "ku-tr",

        LanguageCodeEnum::KW => "kw",

        LanguageCodeEnum::KWGB => "kw-gb",

        LanguageCodeEnum::KY => "ky",

        LanguageCodeEnum::KYKG => "ky-kg",

        LanguageCodeEnum::LAG => "lag",

        LanguageCodeEnum::LAGTZ => "lag-tz",

        LanguageCodeEnum::LB => "lb",

        LanguageCodeEnum::LBLU => "lb-lu",

        LanguageCodeEnum::LG => "lg",

        LanguageCodeEnum::LGUG => "lg-ug",

        LanguageCodeEnum::LKT => "lkt",

        LanguageCodeEnum::LKTUS => "lkt-us",

        LanguageCodeEnum::LN => "ln",

        LanguageCodeEnum::LNAO => "ln-ao",

        LanguageCodeEnum::LNCD => "ln-cd",

        LanguageCodeEnum::LNCF => "ln-cf",

        LanguageCodeEnum::LNCG => "ln-cg",

        LanguageCodeEnum::LO => "lo",

        LanguageCodeEnum::LOLA => "lo-la",

        LanguageCodeEnum::LRC => "lrc",

        LanguageCodeEnum::LRCIQ => "lrc-iq",

        LanguageCodeEnum::LRCIR => "lrc-ir",

        LanguageCodeEnum::LT => "lt",

        LanguageCodeEnum::LTLT => "lt-lt",

        LanguageCodeEnum::LU => "lu",

        LanguageCodeEnum::LUCD => "lu-cd",

        LanguageCodeEnum::LUO => "luo",

        LanguageCodeEnum::LUOKE => "luo-ke",

        LanguageCodeEnum::LUY => "luy",

        LanguageCodeEnum::LUYKE => "luy-ke",

        LanguageCodeEnum::LV => "lv",

        LanguageCodeEnum::LVLV => "lv-lv",

        LanguageCodeEnum::MAI => "mai",

        LanguageCodeEnum::MAIIN => "mai-in",

        LanguageCodeEnum::MAS => "mas",

        LanguageCodeEnum::MASKE => "mas-ke",

        LanguageCodeEnum::MASTZ => "mas-tz",

        LanguageCodeEnum::MER => "mer",

        LanguageCodeEnum::MERKE => "mer-ke",

        LanguageCodeEnum::MFE => "mfe",

        LanguageCodeEnum::MFEMU => "mfe-mu",

        LanguageCodeEnum::MG => "mg",

        LanguageCodeEnum::MGMG => "mg-mg",

        LanguageCodeEnum::MGH => "mgh",

        LanguageCodeEnum::MGHMZ => "mgh-mz",

        LanguageCodeEnum::MGO => "mgo",

        LanguageCodeEnum::MGOCM => "mgo-cm",

        LanguageCodeEnum::MI => "mi",

        LanguageCodeEnum::MINZ => "mi-nz",

        LanguageCodeEnum::MK => "mk",

        LanguageCodeEnum::MKMK => "mk-mk",

        LanguageCodeEnum::ML => "ml",

        LanguageCodeEnum::MLIN => "ml-in",

        LanguageCodeEnum::MN => "mn",

        LanguageCodeEnum::MNMN => "mn-mn",

        LanguageCodeEnum::MNI => "mni",

        LanguageCodeEnum::MNIBENG => "mni-beng",

        LanguageCodeEnum::MNIBENGIN => "mni-beng-in",

        LanguageCodeEnum::MR => "mr",

        LanguageCodeEnum::MRIN => "mr-in",

        LanguageCodeEnum::MS => "ms",

        LanguageCodeEnum::MSBN => "ms-bn",

        LanguageCodeEnum::MSID => "ms-id",

        LanguageCodeEnum::MSMY => "ms-my",

        LanguageCodeEnum::MSSG => "ms-sg",

        LanguageCodeEnum::MT => "mt",

        LanguageCodeEnum::MTMT => "mt-mt",

        LanguageCodeEnum::MUA => "mua",

        LanguageCodeEnum::MUACM => "mua-cm",

        LanguageCodeEnum::MY => "my",

        LanguageCodeEnum::MYMM => "my-mm",

        LanguageCodeEnum::MZN => "mzn",

        LanguageCodeEnum::MZNIR => "mzn-ir",

        LanguageCodeEnum::NAQ => "naq",

        LanguageCodeEnum::NAQNA => "naq-na",

        LanguageCodeEnum::NB => "nb",

        LanguageCodeEnum::NBNO => "nb-no",

        LanguageCodeEnum::NBSJ => "nb-sj",

        LanguageCodeEnum::ND => "nd",

        LanguageCodeEnum::NDZW => "nd-zw",

        LanguageCodeEnum::NDS => "nds",

        LanguageCodeEnum::NDSDE => "nds-de",

        LanguageCodeEnum::NDSNL => "nds-nl",

        LanguageCodeEnum::NE => "ne",

        LanguageCodeEnum::NEIN => "ne-in",

        LanguageCodeEnum::NENP => "ne-np",

        LanguageCodeEnum::NL => "nl",

        LanguageCodeEnum::NLAW => "nl-aw",

        LanguageCodeEnum::NLBE => "nl-be",

        LanguageCodeEnum::NLBQ => "nl-bq",

        LanguageCodeEnum::NLCW => "nl-cw",

        LanguageCodeEnum::NLNL => "nl-nl",

        LanguageCodeEnum::NLSR => "nl-sr",

        LanguageCodeEnum::NLSX => "nl-sx",

        LanguageCodeEnum::NMG => "nmg",

        LanguageCodeEnum::NMGCM => "nmg-cm",

        LanguageCodeEnum::NN => "nn",

        LanguageCodeEnum::NNNO => "nn-no",

        LanguageCodeEnum::NNH => "nnh",

        LanguageCodeEnum::NNHCM => "nnh-cm",

        LanguageCodeEnum::NUS => "nus",

        LanguageCodeEnum::NUSSS => "nus-ss",

        LanguageCodeEnum::NYN => "nyn",

        LanguageCodeEnum::NYNUG => "nyn-ug",

        LanguageCodeEnum::OM => "om",

        LanguageCodeEnum::OMET => "om-et",

        LanguageCodeEnum::OMKE => "om-ke",

        LanguageCodeEnum::OR => "or",

        LanguageCodeEnum::ORIN => "or-in",

        LanguageCodeEnum::OS => "os",

        LanguageCodeEnum::OSGE => "os-ge",

        LanguageCodeEnum::OSRU => "os-ru",

        LanguageCodeEnum::PA => "pa",

        LanguageCodeEnum::PAARAB => "pa-arab",

        LanguageCodeEnum::PAARABPK => "pa-arab-pk",

        LanguageCodeEnum::PAGURU => "pa-guru",

        LanguageCodeEnum::PAGURUIN => "pa-guru-in",

        LanguageCodeEnum::PCM => "pcm",

        LanguageCodeEnum::PCMNG => "pcm-ng",

        LanguageCodeEnum::PL => "pl",

        LanguageCodeEnum::PLPL => "pl-pl",

        LanguageCodeEnum::PRG => "prg",

        LanguageCodeEnum::PS => "ps",

        LanguageCodeEnum::PSAF => "ps-af",

        LanguageCodeEnum::PSPK => "ps-pk",

        LanguageCodeEnum::PT => "pt",

        LanguageCodeEnum::PTAO => "pt-ao",

        LanguageCodeEnum::PTBR => "pt-br",

        LanguageCodeEnum::PTCH => "pt-ch",

        LanguageCodeEnum::PTCV => "pt-cv",

        LanguageCodeEnum::PTGQ => "pt-gq",

        LanguageCodeEnum::PTGW => "pt-gw",

        LanguageCodeEnum::PTLU => "pt-lu",

        LanguageCodeEnum::PTMO => "pt-mo",

        LanguageCodeEnum::PTMZ => "pt-mz",

        LanguageCodeEnum::PTPT => "pt-pt",

        LanguageCodeEnum::PTST => "pt-st",

        LanguageCodeEnum::PTTL => "pt-tl",

        LanguageCodeEnum::QU => "qu",

        LanguageCodeEnum::QUBO => "qu-bo",

        LanguageCodeEnum::QUEC => "qu-ec",

        LanguageCodeEnum::QUPE => "qu-pe",

        LanguageCodeEnum::RM => "rm",

        LanguageCodeEnum::RMCH => "rm-ch",

        LanguageCodeEnum::RN => "rn",

        LanguageCodeEnum::RNBI => "rn-bi",

        LanguageCodeEnum::RO => "ro",

        LanguageCodeEnum::ROMD => "ro-md",

        LanguageCodeEnum::RORO => "ro-ro",

        LanguageCodeEnum::ROF => "rof",

        LanguageCodeEnum::ROFTZ => "rof-tz",

        LanguageCodeEnum::RU => "ru",

        LanguageCodeEnum::RUBY => "ru-by",

        LanguageCodeEnum::RUKG => "ru-kg",

        LanguageCodeEnum::RUKZ => "ru-kz",

        LanguageCodeEnum::RUMD => "ru-md",

        LanguageCodeEnum::RURU => "ru-ru",

        LanguageCodeEnum::RUUA => "ru-ua",

        LanguageCodeEnum::RW => "rw",

        LanguageCodeEnum::RWRW => "rw-rw",

        LanguageCodeEnum::RWK => "rwk",

        LanguageCodeEnum::RWKTZ => "rwk-tz",

        LanguageCodeEnum::SAH => "sah",

        LanguageCodeEnum::SAHRU => "sah-ru",

        LanguageCodeEnum::SAQ => "saq",

        LanguageCodeEnum::SAQKE => "saq-ke",

        LanguageCodeEnum::SAT => "sat",

        LanguageCodeEnum::SATOLCK => "sat-olck",

        LanguageCodeEnum::SATOLCKIN => "sat-olck-in",

        LanguageCodeEnum::SBP => "sbp",

        LanguageCodeEnum::SBPTZ => "sbp-tz",

        LanguageCodeEnum::SD => "sd",

        LanguageCodeEnum::SDARAB => "sd-arab",

        LanguageCodeEnum::SDARABPK => "sd-arab-pk",

        LanguageCodeEnum::SDDEVA => "sd-deva",

        LanguageCodeEnum::SDDEVAIN => "sd-deva-in",

        LanguageCodeEnum::SE => "se",

        LanguageCodeEnum::SEFI => "se-fi",

        LanguageCodeEnum::SENO => "se-no",

        LanguageCodeEnum::SESE => "se-se",

        LanguageCodeEnum::SEH => "seh",

        LanguageCodeEnum::SEHMZ => "seh-mz",

        LanguageCodeEnum::SES => "ses",

        LanguageCodeEnum::SESML => "ses-ml",

        LanguageCodeEnum::SG => "sg",

        LanguageCodeEnum::SGCF => "sg-cf",

        LanguageCodeEnum::SHI => "shi",

        LanguageCodeEnum::SHILATN => "shi-latn",

        LanguageCodeEnum::SHILATNMA => "shi-latn-ma",

        LanguageCodeEnum::SHITFNG => "shi-tfng",

        LanguageCodeEnum::SHITFNGMA => "shi-tfng-ma",

        LanguageCodeEnum::SI => "si",

        LanguageCodeEnum::SILK => "si-lk",

        LanguageCodeEnum::SK => "sk",

        LanguageCodeEnum::SKSK => "sk-sk",

        LanguageCodeEnum::SL => "sl",

        LanguageCodeEnum::SLSI => "sl-si",

        LanguageCodeEnum::SMN => "smn",

        LanguageCodeEnum::SMNFI => "smn-fi",

        LanguageCodeEnum::SN => "sn",

        LanguageCodeEnum::SNZW => "sn-zw",

        LanguageCodeEnum::SO => "so",

        LanguageCodeEnum::SODJ => "so-dj",

        LanguageCodeEnum::SOET => "so-et",

        LanguageCodeEnum::SOKE => "so-ke",

        LanguageCodeEnum::SOSO => "so-so",

        LanguageCodeEnum::SQ => "sq",

        LanguageCodeEnum::SQAL => "sq-al",

        LanguageCodeEnum::SQMK => "sq-mk",

        LanguageCodeEnum::SQXK => "sq-xk",

        LanguageCodeEnum::SR => "sr",

        LanguageCodeEnum::SRCYRL => "sr-cyrl",

        LanguageCodeEnum::SRCYRLBA => "sr-cyrl-ba",

        LanguageCodeEnum::SRCYRLME => "sr-cyrl-me",

        LanguageCodeEnum::SRCYRLRS => "sr-cyrl-rs",

        LanguageCodeEnum::SRCYRLXK => "sr-cyrl-xk",

        LanguageCodeEnum::SRLATN => "sr-latn",

        LanguageCodeEnum::SRLATNBA => "sr-latn-ba",

        LanguageCodeEnum::SRLATNME => "sr-latn-me",

        LanguageCodeEnum::SRLATNRS => "sr-latn-rs",

        LanguageCodeEnum::SRLATNXK => "sr-latn-xk",

        LanguageCodeEnum::SU => "su",

        LanguageCodeEnum::SULATN => "su-latn",

        LanguageCodeEnum::SULATNID => "su-latn-id",

        LanguageCodeEnum::SV => "sv",

        LanguageCodeEnum::SVAX => "sv-ax",

        LanguageCodeEnum::SVFI => "sv-fi",

        LanguageCodeEnum::SVSE => "sv-se",

        LanguageCodeEnum::SW => "sw",

        LanguageCodeEnum::SWCD => "sw-cd",

        LanguageCodeEnum::SWKE => "sw-ke",

        LanguageCodeEnum::SWTZ => "sw-tz",

        LanguageCodeEnum::SWUG => "sw-ug",

        LanguageCodeEnum::TA => "ta",

        LanguageCodeEnum::TAIN => "ta-in",

        LanguageCodeEnum::TALK => "ta-lk",

        LanguageCodeEnum::TAMY => "ta-my",

        LanguageCodeEnum::TASG => "ta-sg",

        LanguageCodeEnum::TE => "te",

        LanguageCodeEnum::TEIN => "te-in",

        LanguageCodeEnum::TEO => "teo",

        LanguageCodeEnum::TEOKE => "teo-ke",

        LanguageCodeEnum::TEOUG => "teo-ug",

        LanguageCodeEnum::TG => "tg",

        LanguageCodeEnum::TGTJ => "tg-tj",

        LanguageCodeEnum::TH => "th",

        LanguageCodeEnum::THTH => "th-th",

        LanguageCodeEnum::TI => "ti",

        LanguageCodeEnum::TIER => "ti-er",

        LanguageCodeEnum::TIET => "ti-et",

        LanguageCodeEnum::TK => "tk",

        LanguageCodeEnum::TKTM => "tk-tm",

        LanguageCodeEnum::TO => "to",

        LanguageCodeEnum::TOTO => "to-to",

        LanguageCodeEnum::TR => "tr",

        LanguageCodeEnum::TRCY => "tr-cy",

        LanguageCodeEnum::TRTR => "tr-tr",

        LanguageCodeEnum::TT => "tt",

        LanguageCodeEnum::TTRU => "tt-ru",

        LanguageCodeEnum::TWQ => "twq",

        LanguageCodeEnum::TWQNE => "twq-ne",

        LanguageCodeEnum::TZM => "tzm",

        LanguageCodeEnum::TZMMA => "tzm-ma",

        LanguageCodeEnum::UG => "ug",

        LanguageCodeEnum::UGCN => "ug-cn",

        LanguageCodeEnum::UK => "uk",

        LanguageCodeEnum::UKUA => "uk-ua",

        LanguageCodeEnum::UR => "ur",

        LanguageCodeEnum::URIN => "ur-in",

        LanguageCodeEnum::URPK => "ur-pk",

        LanguageCodeEnum::UZ => "uz",

        LanguageCodeEnum::UZARAB => "uz-arab",

        LanguageCodeEnum::UZARABAF => "uz-arab-af",

        LanguageCodeEnum::UZCYRL => "uz-cyrl",

        LanguageCodeEnum::UZCYRLUZ => "uz-cyrl-uz",

        LanguageCodeEnum::UZLATN => "uz-latn",

        LanguageCodeEnum::UZLATNUZ => "uz-latn-uz",

        LanguageCodeEnum::VAI => "vai",

        LanguageCodeEnum::VAILATN => "vai-latn",

        LanguageCodeEnum::VAILATNLR => "vai-latn-lr",

        LanguageCodeEnum::VAIVAII => "vai-vaii",

        LanguageCodeEnum::VAIVAIILR => "vai-vaii-lr",

        LanguageCodeEnum::VI => "vi",

        LanguageCodeEnum::VIVN => "vi-vn",

        LanguageCodeEnum::VO => "vo",

        LanguageCodeEnum::VUN => "vun",

        LanguageCodeEnum::VUNTZ => "vun-tz",

        LanguageCodeEnum::WAE => "wae",

        LanguageCodeEnum::WAECH => "wae-ch",

        LanguageCodeEnum::WO => "wo",

        LanguageCodeEnum::WOSN => "wo-sn",

        LanguageCodeEnum::XH => "xh",

        LanguageCodeEnum::XHZA => "xh-za",

        LanguageCodeEnum::XOG => "xog",

        LanguageCodeEnum::XOGUG => "xog-ug",

        LanguageCodeEnum::YAV => "yav",

        LanguageCodeEnum::YAVCM => "yav-cm",

        LanguageCodeEnum::YI => "yi",

        LanguageCodeEnum::YO => "yo",

        LanguageCodeEnum::YOBJ => "yo-bj",

        LanguageCodeEnum::YONG => "yo-ng",

        LanguageCodeEnum::YUE => "yue",

        LanguageCodeEnum::YUEHANS => "yue-hans",

        LanguageCodeEnum::YUEHANSCN => "yue-hans-cn",

        LanguageCodeEnum::YUEHANT => "yue-hant",

        LanguageCodeEnum::YUEHANTHK => "yue-hant-hk",

        LanguageCodeEnum::ZGH => "zgh",

        LanguageCodeEnum::ZGHMA => "zgh-ma",

        LanguageCodeEnum::ZH => "zh",

        LanguageCodeEnum::ZHHANS => "zh-hans",

        LanguageCodeEnum::ZHHANSCN => "zh-hans-cn",

        LanguageCodeEnum::ZHHANSHK => "zh-hans-hk",

        LanguageCodeEnum::ZHHANSMO => "zh-hans-mo",

        LanguageCodeEnum::ZHHANSSG => "zh-hans-sg",

        LanguageCodeEnum::ZHHANT => "zh-hant",

        LanguageCodeEnum::ZHHANTHK => "zh-hant-hk",

        LanguageCodeEnum::ZHHANTMO => "zh-hant-mo",

        LanguageCodeEnum::ZHHANTTW => "zh-hant-tw",

        LanguageCodeEnum::ZU => "zu",

        LanguageCodeEnum::ZUZA => "zu-za",

    }
}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkAsPaidStrategyEnum {

    #[graphql(name = "TRANSACTION_FLOW")]
    TRANSACTIONFLOW,

    #[graphql(name = "PAYMENT_FLOW")]
    PAYMENTFLOW,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeasurementUnitsEnum {

    #[graphql(name = "MM")]
    MM,

    #[graphql(name = "CM")]
    CM,

    #[graphql(name = "DM")]
    DM,

    #[graphql(name = "M")]
    M,

    #[graphql(name = "KM")]
    KM,

    #[graphql(name = "FT")]
    FT,

    #[graphql(name = "YD")]
    YD,

    #[graphql(name = "INCH")]
    INCH,

    #[graphql(name = "SQ_MM")]
    SQMM,

    #[graphql(name = "SQ_CM")]
    SQCM,

    #[graphql(name = "SQ_DM")]
    SQDM,

    #[graphql(name = "SQ_M")]
    SQM,

    #[graphql(name = "SQ_KM")]
    SQKM,

    #[graphql(name = "SQ_FT")]
    SQFT,

    #[graphql(name = "SQ_YD")]
    SQYD,

    #[graphql(name = "SQ_INCH")]
    SQINCH,

    #[graphql(name = "CUBIC_MILLIMETER")]
    CUBICMILLIMETER,

    #[graphql(name = "CUBIC_CENTIMETER")]
    CUBICCENTIMETER,

    #[graphql(name = "CUBIC_DECIMETER")]
    CUBICDECIMETER,

    #[graphql(name = "CUBIC_METER")]
    CUBICMETER,

    #[graphql(name = "LITER")]
    LITER,

    #[graphql(name = "CUBIC_FOOT")]
    CUBICFOOT,

    #[graphql(name = "CUBIC_INCH")]
    CUBICINCH,

    #[graphql(name = "CUBIC_YARD")]
    CUBICYARD,

    #[graphql(name = "QT")]
    QT,

    #[graphql(name = "PINT")]
    PINT,

    #[graphql(name = "FL_OZ")]
    FLOZ,

    #[graphql(name = "ACRE_IN")]
    ACREIN,

    #[graphql(name = "ACRE_FT")]
    ACREFT,

    #[graphql(name = "G")]
    G,

    #[graphql(name = "LB")]
    LB,

    #[graphql(name = "OZ")]
    OZ,

    #[graphql(name = "KG")]
    KG,

    #[graphql(name = "TONNE")]
    TONNE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MediaChoicesSortField {

    #[graphql(name = "ID")]
    ID,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuErrorCode {

    #[graphql(name = "CANNOT_ASSIGN_NODE")]
    CANNOTASSIGNNODE,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "INVALID_MENU_ITEM")]
    INVALIDMENUITEM,

    #[graphql(name = "NO_MENU_ITEM_PROVIDED")]
    NOMENUITEMPROVIDED,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "TOO_MANY_MENU_ITEMS")]
    TOOMANYMENUITEMS,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuItemsSortField {

    #[graphql(name = "NAME")]
    NAME,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "ITEMS_COUNT")]
    ITEMSCOUNT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum MetadataErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "NOT_UPDATED")]
    NOTUPDATED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavigationType {

    #[graphql(name = "MAIN")]
    MAIN,

    #[graphql(name = "SECONDARY")]
    SECONDARY,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderAction {

    #[graphql(name = "CAPTURE")]
    CAPTURE,

    #[graphql(name = "MARK_AS_PAID")]
    MARKASPAID,

    #[graphql(name = "REFUND")]
    REFUND,

    #[graphql(name = "VOID")]
    VOID,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderAuthorizeStatusEnum {

    #[graphql(name = "NONE")]
    NONE,

    #[graphql(name = "PARTIAL")]
    PARTIAL,

    #[graphql(name = "FULL")]
    FULL,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderBulkCreateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "BULK_LIMIT")]
    BULKLIMIT,

    #[graphql(name = "TOO_MANY_IDENTIFIERS")]
    TOOMANYIDENTIFIERS,

    #[graphql(name = "FUTURE_DATE")]
    FUTUREDATE,

    #[graphql(name = "INVALID_QUANTITY")]
    INVALIDQUANTITY,

    #[graphql(name = "PRICE_ERROR")]
    PRICEERROR,

    #[graphql(name = "NOTE_LENGTH")]
    NOTELENGTH,

    #[graphql(name = "INSUFFICIENT_STOCK")]
    INSUFFICIENTSTOCK,

    #[graphql(name = "NON_EXISTING_STOCK")]
    NONEXISTINGSTOCK,

    #[graphql(name = "NO_RELATED_ORDER_LINE")]
    NORELATEDORDERLINE,

    #[graphql(name = "NEGATIVE_INDEX")]
    NEGATIVEINDEX,

    #[graphql(name = "ORDER_LINE_FULFILLMENT_LINE_MISMATCH")]
    ORDERLINEFULFILLMENTLINEMISMATCH,

    #[graphql(name = "METADATA_KEY_REQUIRED")]
    METADATAKEYREQUIRED,

    #[graphql(name = "INCORRECT_CURRENCY")]
    INCORRECTCURRENCY,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderChargeStatusEnum {

    #[graphql(name = "NONE")]
    NONE,

    #[graphql(name = "PARTIAL")]
    PARTIAL,

    #[graphql(name = "FULL")]
    FULL,

    #[graphql(name = "OVERCHARGED")]
    OVERCHARGED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderCreateFromCheckoutErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "CHECKOUT_NOT_FOUND")]
    CHECKOUTNOTFOUND,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

    #[graphql(name = "INSUFFICIENT_STOCK")]
    INSUFFICIENTSTOCK,

    #[graphql(name = "VOUCHER_NOT_APPLICABLE")]
    VOUCHERNOTAPPLICABLE,

    #[graphql(name = "GIFT_CARD_NOT_APPLICABLE")]
    GIFTCARDNOTAPPLICABLE,

    #[graphql(name = "TAX_ERROR")]
    TAXERROR,

    #[graphql(name = "SHIPPING_METHOD_NOT_SET")]
    SHIPPINGMETHODNOTSET,

    #[graphql(name = "BILLING_ADDRESS_NOT_SET")]
    BILLINGADDRESSNOTSET,

    #[graphql(name = "SHIPPING_ADDRESS_NOT_SET")]
    SHIPPINGADDRESSNOTSET,

    #[graphql(name = "INVALID_SHIPPING_METHOD")]
    INVALIDSHIPPINGMETHOD,

    #[graphql(name = "NO_LINES")]
    NOLINES,

    #[graphql(name = "EMAIL_NOT_SET")]
    EMAILNOTSET,

    #[graphql(name = "UNAVAILABLE_VARIANT_IN_CHANNEL")]
    UNAVAILABLEVARIANTINCHANNEL,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderDirection {

    #[graphql(name = "ASC")]
    ASC,

    #[graphql(name = "DESC")]
    DESC,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderDiscountType {

    #[graphql(name = "SALE")]
    SALE,

    #[graphql(name = "VOUCHER")]
    VOUCHER,

    #[graphql(name = "MANUAL")]
    MANUAL,

    #[graphql(name = "PROMOTION")]
    PROMOTION,

    #[graphql(name = "ORDER_PROMOTION")]
    ORDERPROMOTION,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderErrorCode {

    #[graphql(name = "BILLING_ADDRESS_NOT_SET")]
    BILLINGADDRESSNOTSET,

    #[graphql(name = "CANNOT_CANCEL_FULFILLMENT")]
    CANNOTCANCELFULFILLMENT,

    #[graphql(name = "CANNOT_CANCEL_ORDER")]
    CANNOTCANCELORDER,

    #[graphql(name = "CANNOT_DELETE")]
    CANNOTDELETE,

    #[graphql(name = "CANNOT_DISCOUNT")]
    CANNOTDISCOUNT,

    #[graphql(name = "CANNOT_REFUND")]
    CANNOTREFUND,

    #[graphql(name = "CANNOT_FULFILL_UNPAID_ORDER")]
    CANNOTFULFILLUNPAIDORDER,

    #[graphql(name = "CAPTURE_INACTIVE_PAYMENT")]
    CAPTUREINACTIVEPAYMENT,

    #[graphql(name = "GIFT_CARD_LINE")]
    GIFTCARDLINE,

    #[graphql(name = "NOT_EDITABLE")]
    NOTEDITABLE,

    #[graphql(name = "FULFILL_ORDER_LINE")]
    FULFILLORDERLINE,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "PRODUCT_NOT_PUBLISHED")]
    PRODUCTNOTPUBLISHED,

    #[graphql(name = "PRODUCT_UNAVAILABLE_FOR_PURCHASE")]
    PRODUCTUNAVAILABLEFORPURCHASE,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "ORDER_NO_SHIPPING_ADDRESS")]
    ORDERNOSHIPPINGADDRESS,

    #[graphql(name = "PAYMENT_ERROR")]
    PAYMENTERROR,

    #[graphql(name = "PAYMENT_MISSING")]
    PAYMENTMISSING,

    #[graphql(name = "TRANSACTION_ERROR")]
    TRANSACTIONERROR,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "SHIPPING_METHOD_NOT_APPLICABLE")]
    SHIPPINGMETHODNOTAPPLICABLE,

    #[graphql(name = "SHIPPING_METHOD_REQUIRED")]
    SHIPPINGMETHODREQUIRED,

    #[graphql(name = "TAX_ERROR")]
    TAXERROR,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "VOID_INACTIVE_PAYMENT")]
    VOIDINACTIVEPAYMENT,

    #[graphql(name = "ZERO_QUANTITY")]
    ZEROQUANTITY,

    #[graphql(name = "INVALID_QUANTITY")]
    INVALIDQUANTITY,

    #[graphql(name = "INSUFFICIENT_STOCK")]
    INSUFFICIENTSTOCK,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "NOT_AVAILABLE_IN_CHANNEL")]
    NOTAVAILABLEINCHANNEL,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

    #[graphql(name = "INVALID_VOUCHER")]
    INVALIDVOUCHER,

    #[graphql(name = "INVALID_VOUCHER_CODE")]
    INVALIDVOUCHERCODE,

    #[graphql(name = "NON_EDITABLE_GIFT_LINE")]
    NONEDITABLEGIFTLINE,

    #[graphql(name = "NON_REMOVABLE_GIFT_LINE")]
    NONREMOVABLEGIFTLINE,

    #[graphql(name = "MISSING_ADDRESS_DATA")]
    MISSINGADDRESSDATA,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderEventsEmailsEnum {

    #[graphql(name = "PAYMENT_CONFIRMATION")]
    PAYMENTCONFIRMATION,

    #[graphql(name = "CONFIRMED")]
    CONFIRMED,

    #[graphql(name = "SHIPPING_CONFIRMATION")]
    SHIPPINGCONFIRMATION,

    #[graphql(name = "TRACKING_UPDATED")]
    TRACKINGUPDATED,

    #[graphql(name = "ORDER_CONFIRMATION")]
    ORDERCONFIRMATION,

    #[graphql(name = "ORDER_CANCEL")]
    ORDERCANCEL,

    #[graphql(name = "ORDER_REFUND")]
    ORDERREFUND,

    #[graphql(name = "FULFILLMENT_CONFIRMATION")]
    FULFILLMENTCONFIRMATION,

    #[graphql(name = "DIGITAL_LINKS")]
    DIGITALLINKS,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderEventsEnum {

    #[graphql(name = "DRAFT_CREATED")]
    DRAFTCREATED,

    #[graphql(name = "DRAFT_CREATED_FROM_REPLACE")]
    DRAFTCREATEDFROMREPLACE,

    #[graphql(name = "ADDED_PRODUCTS")]
    ADDEDPRODUCTS,

    #[graphql(name = "REMOVED_PRODUCTS")]
    REMOVEDPRODUCTS,

    #[graphql(name = "PLACED")]
    PLACED,

    #[graphql(name = "PLACED_FROM_DRAFT")]
    PLACEDFROMDRAFT,

    #[graphql(name = "PLACED_AUTOMATICALLY_FROM_PAID_CHECKOUT")]
    PLACEDAUTOMATICALLYFROMPAIDCHECKOUT,

    #[graphql(name = "OVERSOLD_ITEMS")]
    OVERSOLDITEMS,

    #[graphql(name = "CANCELED")]
    CANCELED,

    #[graphql(name = "EXPIRED")]
    EXPIRED,

    #[graphql(name = "ORDER_MARKED_AS_PAID")]
    ORDERMARKEDASPAID,

    #[graphql(name = "ORDER_FULLY_PAID")]
    ORDERFULLYPAID,

    #[graphql(name = "ORDER_REPLACEMENT_CREATED")]
    ORDERREPLACEMENTCREATED,

    #[graphql(name = "ORDER_DISCOUNT_ADDED")]
    ORDERDISCOUNTADDED,

    #[graphql(name = "ORDER_DISCOUNT_AUTOMATICALLY_UPDATED")]
    ORDERDISCOUNTAUTOMATICALLYUPDATED,

    #[graphql(name = "ORDER_DISCOUNT_UPDATED")]
    ORDERDISCOUNTUPDATED,

    #[graphql(name = "ORDER_DISCOUNT_DELETED")]
    ORDERDISCOUNTDELETED,

    #[graphql(name = "ORDER_LINE_DISCOUNT_UPDATED")]
    ORDERLINEDISCOUNTUPDATED,

    #[graphql(name = "ORDER_LINE_DISCOUNT_REMOVED")]
    ORDERLINEDISCOUNTREMOVED,

    #[graphql(name = "ORDER_LINE_PRODUCT_DELETED")]
    ORDERLINEPRODUCTDELETED,

    #[graphql(name = "ORDER_LINE_VARIANT_DELETED")]
    ORDERLINEVARIANTDELETED,

    #[graphql(name = "UPDATED_ADDRESS")]
    UPDATEDADDRESS,

    #[graphql(name = "EMAIL_SENT")]
    EMAILSENT,

    #[graphql(name = "CONFIRMED")]
    CONFIRMED,

    #[graphql(name = "PAYMENT_AUTHORIZED")]
    PAYMENTAUTHORIZED,

    #[graphql(name = "PAYMENT_CAPTURED")]
    PAYMENTCAPTURED,

    #[graphql(name = "EXTERNAL_SERVICE_NOTIFICATION")]
    EXTERNALSERVICENOTIFICATION,

    #[graphql(name = "PAYMENT_REFUNDED")]
    PAYMENTREFUNDED,

    #[graphql(name = "PAYMENT_VOIDED")]
    PAYMENTVOIDED,

    #[graphql(name = "PAYMENT_FAILED")]
    PAYMENTFAILED,

    #[graphql(name = "TRANSACTION_EVENT")]
    TRANSACTIONEVENT,

    #[graphql(name = "TRANSACTION_CHARGE_REQUESTED")]
    TRANSACTIONCHARGEREQUESTED,

    #[graphql(name = "TRANSACTION_REFUND_REQUESTED")]
    TRANSACTIONREFUNDREQUESTED,

    #[graphql(name = "TRANSACTION_CANCEL_REQUESTED")]
    TRANSACTIONCANCELREQUESTED,

    #[graphql(name = "TRANSACTION_MARK_AS_PAID_FAILED")]
    TRANSACTIONMARKASPAIDFAILED,

    #[graphql(name = "INVOICE_REQUESTED")]
    INVOICEREQUESTED,

    #[graphql(name = "INVOICE_GENERATED")]
    INVOICEGENERATED,

    #[graphql(name = "INVOICE_UPDATED")]
    INVOICEUPDATED,

    #[graphql(name = "INVOICE_SENT")]
    INVOICESENT,

    #[graphql(name = "FULFILLMENT_CANCELED")]
    FULFILLMENTCANCELED,

    #[graphql(name = "FULFILLMENT_RESTOCKED_ITEMS")]
    FULFILLMENTRESTOCKEDITEMS,

    #[graphql(name = "FULFILLMENT_FULFILLED_ITEMS")]
    FULFILLMENTFULFILLEDITEMS,

    #[graphql(name = "FULFILLMENT_REFUNDED")]
    FULFILLMENTREFUNDED,

    #[graphql(name = "FULFILLMENT_RETURNED")]
    FULFILLMENTRETURNED,

    #[graphql(name = "FULFILLMENT_REPLACED")]
    FULFILLMENTREPLACED,

    #[graphql(name = "FULFILLMENT_AWAITS_APPROVAL")]
    FULFILLMENTAWAITSAPPROVAL,

    #[graphql(name = "TRACKING_UPDATED")]
    TRACKINGUPDATED,

    #[graphql(name = "NOTE_ADDED")]
    NOTEADDED,

    #[graphql(name = "NOTE_UPDATED")]
    NOTEUPDATED,

    #[graphql(name = "OTHER")]
    OTHER,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderGrantRefundCreateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "NOT_CONFIGURED")]
    NOTCONFIGURED,

    #[graphql(name = "SHIPPING_COSTS_ALREADY_GRANTED")]
    SHIPPINGCOSTSALREADYGRANTED,

    #[graphql(name = "AMOUNT_GREATER_THAN_AVAILABLE")]
    AMOUNTGREATERTHANAVAILABLE,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "INVALID")]
    INVALID,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderGrantRefundCreateLineErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "QUANTITY_GREATER_THAN_AVAILABLE")]
    QUANTITYGREATERTHANAVAILABLE,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_CONFIGURED")]
    NOTCONFIGURED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderGrantRefundUpdateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "NOT_CONFIGURED")]
    NOTCONFIGURED,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "AMOUNT_GREATER_THAN_AVAILABLE")]
    AMOUNTGREATERTHANAVAILABLE,

    #[graphql(name = "SHIPPING_COSTS_ALREADY_GRANTED")]
    SHIPPINGCOSTSALREADYGRANTED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderGrantRefundUpdateLineErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "QUANTITY_GREATER_THAN_AVAILABLE")]
    QUANTITYGREATERTHANAVAILABLE,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_CONFIGURED")]
    NOTCONFIGURED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderGrantedRefundStatusEnum {

    #[graphql(name = "NONE")]
    NONE,

    #[graphql(name = "PENDING")]
    PENDING,

    #[graphql(name = "SUCCESS")]
    SUCCESS,

    #[graphql(name = "FAILURE")]
    FAILURE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderNoteAddErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderNoteUpdateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderOriginEnum {

    #[graphql(name = "CHECKOUT")]
    CHECKOUT,

    #[graphql(name = "DRAFT")]
    DRAFT,

    #[graphql(name = "REISSUE")]
    REISSUE,

    #[graphql(name = "BULK_CREATE")]
    BULKCREATE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderSettingsErrorCode {

    #[graphql(name = "INVALID")]
    INVALID,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderSortField {

    #[graphql(name = "NUMBER")]
    NUMBER,

    #[graphql(name = "RANK")]
    RANK,

    #[graphql(name = "CREATION_DATE")]
    CREATIONDATE,

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

    #[graphql(name = "LAST_MODIFIED_AT")]
    LASTMODIFIEDAT,

    #[graphql(name = "CUSTOMER")]
    CUSTOMER,

    #[graphql(name = "PAYMENT")]
    PAYMENT,

    #[graphql(name = "FULFILLMENT_STATUS")]
    FULFILLMENTSTATUS,

    #[graphql(name = "STATUS")]
    STATUS,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderStatus {

    #[graphql(name = "DRAFT")]
    DRAFT,

    #[graphql(name = "UNCONFIRMED")]
    UNCONFIRMED,

    #[graphql(name = "UNFULFILLED")]
    UNFULFILLED,

    #[graphql(name = "PARTIALLY_FULFILLED")]
    PARTIALLYFULFILLED,

    #[graphql(name = "PARTIALLY_RETURNED")]
    PARTIALLYRETURNED,

    #[graphql(name = "RETURNED")]
    RETURNED,

    #[graphql(name = "FULFILLED")]
    FULFILLED,

    #[graphql(name = "CANCELED")]
    CANCELED,

    #[graphql(name = "EXPIRED")]
    EXPIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrderStatusFilter {

    #[graphql(name = "READY_TO_FULFILL")]
    READYTOFULFILL,

    #[graphql(name = "READY_TO_CAPTURE")]
    READYTOCAPTURE,

    #[graphql(name = "UNFULFILLED")]
    UNFULFILLED,

    #[graphql(name = "UNCONFIRMED")]
    UNCONFIRMED,

    #[graphql(name = "PARTIALLY_FULFILLED")]
    PARTIALLYFULFILLED,

    #[graphql(name = "FULFILLED")]
    FULFILLED,

    #[graphql(name = "CANCELED")]
    CANCELED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "ATTRIBUTE_ALREADY_ASSIGNED")]
    ATTRIBUTEALREADYASSIGNED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageSortField {

    #[graphql(name = "TITLE")]
    TITLE,

    #[graphql(name = "SLUG")]
    SLUG,

    #[graphql(name = "VISIBILITY")]
    VISIBILITY,

    #[graphql(name = "CREATION_DATE")]
    CREATIONDATE,

    #[graphql(name = "PUBLICATION_DATE")]
    PUBLICATIONDATE,

    #[graphql(name = "PUBLISHED_AT")]
    PUBLISHEDAT,

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

    #[graphql(name = "RANK")]
    RANK,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageTypeSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "SLUG")]
    SLUG,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PasswordLoginModeEnum {

    #[graphql(name = "ENABLED")]
    ENABLED,

    #[graphql(name = "CUSTOMERS_ONLY")]
    CUSTOMERSONLY,

    #[graphql(name = "DISABLED")]
    DISABLED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentChargeStatusEnum {

    #[graphql(name = "NOT_CHARGED")]
    NOTCHARGED,

    #[graphql(name = "PENDING")]
    PENDING,

    #[graphql(name = "PARTIALLY_CHARGED")]
    PARTIALLYCHARGED,

    #[graphql(name = "FULLY_CHARGED")]
    FULLYCHARGED,

    #[graphql(name = "PARTIALLY_REFUNDED")]
    PARTIALLYREFUNDED,

    #[graphql(name = "FULLY_REFUNDED")]
    FULLYREFUNDED,

    #[graphql(name = "REFUSED")]
    REFUSED,

    #[graphql(name = "CANCELLED")]
    CANCELLED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentErrorCode {

    #[graphql(name = "BILLING_ADDRESS_NOT_SET")]
    BILLINGADDRESSNOTSET,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "PARTIAL_PAYMENT_NOT_ALLOWED")]
    PARTIALPAYMENTNOTALLOWED,

    #[graphql(name = "SHIPPING_ADDRESS_NOT_SET")]
    SHIPPINGADDRESSNOTSET,

    #[graphql(name = "INVALID_SHIPPING_METHOD")]
    INVALIDSHIPPINGMETHOD,

    #[graphql(name = "SHIPPING_METHOD_NOT_SET")]
    SHIPPINGMETHODNOTSET,

    #[graphql(name = "PAYMENT_ERROR")]
    PAYMENTERROR,

    #[graphql(name = "NOT_SUPPORTED_GATEWAY")]
    NOTSUPPORTEDGATEWAY,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

    #[graphql(name = "BALANCE_CHECK_ERROR")]
    BALANCECHECKERROR,

    #[graphql(name = "CHECKOUT_EMAIL_NOT_SET")]
    CHECKOUTEMAILNOTSET,

    #[graphql(name = "UNAVAILABLE_VARIANT_IN_CHANNEL")]
    UNAVAILABLEVARIANTINCHANNEL,

    #[graphql(name = "NO_CHECKOUT_LINES")]
    NOCHECKOUTLINES,

    #[graphql(name = "CHECKOUT_COMPLETION_IN_PROGRESS")]
    CHECKOUTCOMPLETIONINPROGRESS,

    #[graphql(name = "CHECKOUT_HAS_TRANSACTION")]
    CHECKOUTHASTRANSACTION,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentGatewayConfigErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentGatewayInitializeErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentGatewayInitializeTokenizationErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

    #[graphql(name = "GATEWAY_ERROR")]
    GATEWAYERROR,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentGatewayInitializeTokenizationResult {

    #[graphql(name = "SUCCESSFULLY_INITIALIZED")]
    SUCCESSFULLYINITIALIZED,

    #[graphql(name = "FAILED_TO_INITIALIZE")]
    FAILEDTOINITIALIZE,

    #[graphql(name = "FAILED_TO_DELIVER")]
    FAILEDTODELIVER,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentMethodInitializeTokenizationErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

    #[graphql(name = "GATEWAY_ERROR")]
    GATEWAYERROR,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentMethodProcessTokenizationErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

    #[graphql(name = "GATEWAY_ERROR")]
    GATEWAYERROR,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentMethodTokenizationResult {

    #[graphql(name = "SUCCESSFULLY_TOKENIZED")]
    SUCCESSFULLYTOKENIZED,

    #[graphql(name = "PENDING")]
    PENDING,

    #[graphql(name = "ADDITIONAL_ACTION_REQUIRED")]
    ADDITIONALACTIONREQUIRED,

    #[graphql(name = "FAILED_TO_TOKENIZE")]
    FAILEDTOTOKENIZE,

    #[graphql(name = "FAILED_TO_DELIVER")]
    FAILEDTODELIVER,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PaymentMethodTypeEnum {

    #[graphql(name = "CARD")]
    CARD,

    #[graphql(name = "OTHER")]
    OTHER,

    #[graphql(name = "GIFT_CARD")]
    GIFTCARD,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PermissionEnum {

    #[graphql(name = "MANAGE_USERS")]
    MANAGEUSERS,

    #[graphql(name = "MANAGE_STAFF")]
    MANAGESTAFF,

    #[graphql(name = "IMPERSONATE_USER")]
    IMPERSONATEUSER,

    #[graphql(name = "MANAGE_APPS")]
    MANAGEAPPS,

    #[graphql(name = "MANAGE_OBSERVABILITY")]
    MANAGEOBSERVABILITY,

    #[graphql(name = "MANAGE_CHECKOUTS")]
    MANAGECHECKOUTS,

    #[graphql(name = "HANDLE_CHECKOUTS")]
    HANDLECHECKOUTS,

    #[graphql(name = "HANDLE_TAXES")]
    HANDLETAXES,

    #[graphql(name = "MANAGE_TAXES")]
    MANAGETAXES,

    #[graphql(name = "MANAGE_CHANNELS")]
    MANAGECHANNELS,

    #[graphql(name = "MANAGE_CUSTOMER_TYPES_AND_ATTRIBUTES")]
    MANAGECUSTOMERTYPESANDATTRIBUTES,

    #[graphql(name = "MANAGE_DISCOUNTS")]
    MANAGEDISCOUNTS,

    #[graphql(name = "MANAGE_GIFT_CARD")]
    MANAGEGIFTCARD,

    #[graphql(name = "MANAGE_MENUS")]
    MANAGEMENUS,

    #[graphql(name = "MANAGE_ORDERS")]
    MANAGEORDERS,

    #[graphql(name = "MANAGE_ORDERS_IMPORT")]
    MANAGEORDERSIMPORT,

    #[graphql(name = "MANAGE_PAGES")]
    MANAGEPAGES,

    #[graphql(name = "MANAGE_PAGE_TYPES_AND_ATTRIBUTES")]
    MANAGEPAGETYPESANDATTRIBUTES,

    #[graphql(name = "HANDLE_PAYMENTS")]
    HANDLEPAYMENTS,

    #[graphql(name = "MANAGE_PLUGINS")]
    MANAGEPLUGINS,

    #[graphql(name = "MANAGE_PRODUCTS")]
    MANAGEPRODUCTS,

    #[graphql(name = "MANAGE_PRODUCT_TYPES_AND_ATTRIBUTES")]
    MANAGEPRODUCTTYPESANDATTRIBUTES,

    #[graphql(name = "MANAGE_SHIPPING")]
    MANAGESHIPPING,

    #[graphql(name = "MANAGE_SETTINGS")]
    MANAGESETTINGS,

    #[graphql(name = "MANAGE_TRANSLATIONS")]
    MANAGETRANSLATIONS,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PermissionGroupErrorCode {

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "ASSIGN_NON_STAFF_MEMBER")]
    ASSIGNNONSTAFFMEMBER,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "CANNOT_REMOVE_FROM_LAST_GROUP")]
    CANNOTREMOVEFROMLASTGROUP,

    #[graphql(name = "LEFT_NOT_MANAGEABLE_PERMISSION")]
    LEFTNOTMANAGEABLEPERMISSION,

    #[graphql(name = "OUT_OF_SCOPE_PERMISSION")]
    OUTOFSCOPEPERMISSION,

    #[graphql(name = "OUT_OF_SCOPE_USER")]
    OUTOFSCOPEUSER,

    #[graphql(name = "OUT_OF_SCOPE_CHANNEL")]
    OUTOFSCOPECHANNEL,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PermissionGroupSortField {

    #[graphql(name = "NAME")]
    NAME,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PluginConfigurationType {

    #[graphql(name = "PER_CHANNEL")]
    PERCHANNEL,

    #[graphql(name = "GLOBAL")]
    GLOBAL,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PluginErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "PLUGIN_MISCONFIGURED")]
    PLUGINMISCONFIGURED,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PluginSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "IS_ACTIVE")]
    ISACTIVE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PostalCodeRuleInclusionTypeEnum {

    #[graphql(name = "INCLUDE")]
    INCLUDE,

    #[graphql(name = "EXCLUDE")]
    EXCLUDE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductAttributeType {

    #[graphql(name = "PRODUCT")]
    PRODUCT,

    #[graphql(name = "VARIANT")]
    VARIANT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductBulkCreateErrorCode {

    #[graphql(name = "ATTRIBUTE_ALREADY_ASSIGNED")]
    ATTRIBUTEALREADYASSIGNED,

    #[graphql(name = "ATTRIBUTE_CANNOT_BE_ASSIGNED")]
    ATTRIBUTECANNOTBEASSIGNED,

    #[graphql(name = "ATTRIBUTE_VARIANTS_DISABLED")]
    ATTRIBUTEVARIANTSDISABLED,

    #[graphql(name = "BLANK")]
    BLANK,

    #[graphql(name = "MAX_LENGTH")]
    MAXLENGTH,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "INVALID_PRICE")]
    INVALIDPRICE,

    #[graphql(name = "PRODUCT_WITHOUT_CATEGORY")]
    PRODUCTWITHOUTCATEGORY,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "PRODUCT_NOT_ASSIGNED_TO_CHANNEL")]
    PRODUCTNOTASSIGNEDTOCHANNEL,

    #[graphql(name = "UNSUPPORTED_MEDIA_PROVIDER")]
    UNSUPPORTEDMEDIAPROVIDER,

    #[graphql(name = "FILE_SIZE_LIMIT_EXCEEDED")]
    FILESIZELIMITEXCEEDED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "ATTRIBUTE_ALREADY_ASSIGNED")]
    ATTRIBUTEALREADYASSIGNED,

    #[graphql(name = "ATTRIBUTE_CANNOT_BE_ASSIGNED")]
    ATTRIBUTECANNOTBEASSIGNED,

    #[graphql(name = "ATTRIBUTE_VARIANTS_DISABLED")]
    ATTRIBUTEVARIANTSDISABLED,

    #[graphql(name = "MEDIA_ALREADY_ASSIGNED")]
    MEDIAALREADYASSIGNED,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "INVALID_PRICE")]
    INVALIDPRICE,

    #[graphql(name = "PRODUCT_WITHOUT_CATEGORY")]
    PRODUCTWITHOUTCATEGORY,

    #[graphql(name = "NOT_PRODUCTS_IMAGE")]
    NOTPRODUCTSIMAGE,

    #[graphql(name = "NOT_PRODUCTS_VARIANT")]
    NOTPRODUCTSVARIANT,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "CANNOT_MANAGE_PRODUCT_WITHOUT_VARIANT")]
    CANNOTMANAGEPRODUCTWITHOUTVARIANT,

    #[graphql(name = "PRODUCT_NOT_ASSIGNED_TO_CHANNEL")]
    PRODUCTNOTASSIGNEDTOCHANNEL,

    #[graphql(name = "UNSUPPORTED_MEDIA_PROVIDER")]
    UNSUPPORTEDMEDIAPROVIDER,

    #[graphql(name = "PREORDER_VARIANT_CANNOT_BE_DEACTIVATED")]
    PREORDERVARIANTCANNOTBEDEACTIVATED,

    #[graphql(name = "INVALID_FILE_TYPE")]
    INVALIDFILETYPE,

    #[graphql(name = "UNSUPPORTED_MIME_TYPE")]
    UNSUPPORTEDMIMETYPE,

    #[graphql(name = "FILE_SIZE_LIMIT_EXCEEDED")]
    FILESIZELIMITEXCEEDED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductFieldEnum {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "DESCRIPTION")]
    DESCRIPTION,

    #[graphql(name = "PRODUCT_TYPE")]
    PRODUCTTYPE,

    #[graphql(name = "CATEGORY")]
    CATEGORY,

    #[graphql(name = "PRODUCT_WEIGHT")]
    PRODUCTWEIGHT,

    #[graphql(name = "COLLECTIONS")]
    COLLECTIONS,

    #[graphql(name = "CHARGE_TAXES")]
    CHARGETAXES,

    #[graphql(name = "PRODUCT_MEDIA")]
    PRODUCTMEDIA,

    #[graphql(name = "VARIANT_ID")]
    VARIANTID,

    #[graphql(name = "VARIANT_SKU")]
    VARIANTSKU,

    #[graphql(name = "VARIANT_WEIGHT")]
    VARIANTWEIGHT,

    #[graphql(name = "VARIANT_MEDIA")]
    VARIANTMEDIA,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductMediaType {

    #[graphql(name = "IMAGE")]
    IMAGE,

    #[graphql(name = "VIDEO")]
    VIDEO,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductOrderField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "RANK")]
    RANK,

    #[graphql(name = "PRICE")]
    PRICE,

    #[graphql(name = "MINIMAL_PRICE")]
    MINIMALPRICE,

    #[graphql(name = "LAST_MODIFIED")]
    LASTMODIFIED,

    #[graphql(name = "DATE")]
    DATE,

    #[graphql(name = "TYPE")]
    TYPE,

    #[graphql(name = "PUBLISHED")]
    PUBLISHED,

    #[graphql(name = "PUBLICATION_DATE")]
    PUBLICATIONDATE,

    #[graphql(name = "PUBLISHED_AT")]
    PUBLISHEDAT,

    #[graphql(name = "LAST_MODIFIED_AT")]
    LASTMODIFIEDAT,

    #[graphql(name = "COLLECTION")]
    COLLECTION,

    #[graphql(name = "RATING")]
    RATING,

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductTranslateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductTypeConfigurable {

    #[graphql(name = "CONFIGURABLE")]
    CONFIGURABLE,

    #[graphql(name = "SIMPLE")]
    SIMPLE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductTypeEnum {

    #[graphql(name = "DIGITAL")]
    DIGITAL,

    #[graphql(name = "SHIPPABLE")]
    SHIPPABLE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductTypeKindEnum {

    #[graphql(name = "NORMAL")]
    NORMAL,

    #[graphql(name = "GIFT_CARD")]
    GIFTCARD,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductTypeSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "DIGITAL")]
    DIGITAL,

    #[graphql(name = "SHIPPING_REQUIRED")]
    SHIPPINGREQUIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductVariantBulkErrorCode {

    #[graphql(name = "ATTRIBUTE_ALREADY_ASSIGNED")]
    ATTRIBUTEALREADYASSIGNED,

    #[graphql(name = "ATTRIBUTE_CANNOT_BE_ASSIGNED")]
    ATTRIBUTECANNOTBEASSIGNED,

    #[graphql(name = "ATTRIBUTE_VARIANTS_DISABLED")]
    ATTRIBUTEVARIANTSDISABLED,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "INVALID_PRICE")]
    INVALIDPRICE,

    #[graphql(name = "NOT_PRODUCTS_VARIANT")]
    NOTPRODUCTSVARIANT,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "PRODUCT_NOT_ASSIGNED_TO_CHANNEL")]
    PRODUCTNOTASSIGNEDTOCHANNEL,

    #[graphql(name = "STOCK_ALREADY_EXISTS")]
    STOCKALREADYEXISTS,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductVariantSortField {

    #[graphql(name = "LAST_MODIFIED_AT")]
    LASTMODIFIEDAT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProductVariantTranslateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromotionCreateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "MULTIPLE_CURRENCIES_NOT_ALLOWED")]
    MULTIPLECURRENCIESNOTALLOWED,

    #[graphql(name = "INVALID_PRECISION")]
    INVALIDPRECISION,

    #[graphql(name = "MISSING_CHANNELS")]
    MISSINGCHANNELS,

    #[graphql(name = "RULES_NUMBER_LIMIT")]
    RULESNUMBERLIMIT,

    #[graphql(name = "GIFTS_NUMBER_LIMIT")]
    GIFTSNUMBERLIMIT,

    #[graphql(name = "INVALID_GIFT_TYPE")]
    INVALIDGIFTTYPE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromotionDeleteErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromotionEventsEnum {

    #[graphql(name = "PROMOTION_CREATED")]
    PROMOTIONCREATED,

    #[graphql(name = "PROMOTION_UPDATED")]
    PROMOTIONUPDATED,

    #[graphql(name = "PROMOTION_STARTED")]
    PROMOTIONSTARTED,

    #[graphql(name = "PROMOTION_ENDED")]
    PROMOTIONENDED,

    #[graphql(name = "RULE_CREATED")]
    RULECREATED,

    #[graphql(name = "RULE_UPDATED")]
    RULEUPDATED,

    #[graphql(name = "RULE_DELETED")]
    RULEDELETED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromotionRuleCreateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "MULTIPLE_CURRENCIES_NOT_ALLOWED")]
    MULTIPLECURRENCIESNOTALLOWED,

    #[graphql(name = "INVALID_PRECISION")]
    INVALIDPRECISION,

    #[graphql(name = "MISSING_CHANNELS")]
    MISSINGCHANNELS,

    #[graphql(name = "RULES_NUMBER_LIMIT")]
    RULESNUMBERLIMIT,

    #[graphql(name = "GIFTS_NUMBER_LIMIT")]
    GIFTSNUMBERLIMIT,

    #[graphql(name = "INVALID_GIFT_TYPE")]
    INVALIDGIFTTYPE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromotionRuleDeleteErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromotionRuleUpdateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "MISSING_CHANNELS")]
    MISSINGCHANNELS,

    #[graphql(name = "MULTIPLE_CURRENCIES_NOT_ALLOWED")]
    MULTIPLECURRENCIESNOTALLOWED,

    #[graphql(name = "INVALID_PRECISION")]
    INVALIDPRECISION,

    #[graphql(name = "INVALID_GIFT_TYPE")]
    INVALIDGIFTTYPE,

    #[graphql(name = "GIFTS_NUMBER_LIMIT")]
    GIFTSNUMBERLIMIT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromotionSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "START_DATE")]
    STARTDATE,

    #[graphql(name = "END_DATE")]
    ENDDATE,

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromotionTypeEnum {

    #[graphql(name = "CATALOGUE")]
    CATALOGUE,

    #[graphql(name = "ORDER")]
    ORDER,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromotionUpdateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "INVALID")]
    INVALID,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RefundSettingsErrorCode {

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReportingPeriod {

    #[graphql(name = "TODAY")]
    TODAY,

    #[graphql(name = "THIS_MONTH")]
    THISMONTH,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReturnSettingsErrorCode {

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RewardTypeEnum {

    #[graphql(name = "SUBTOTAL_DISCOUNT")]
    SUBTOTALDISCOUNT,

    #[graphql(name = "GIFT")]
    GIFT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RewardValueTypeEnum {

    #[graphql(name = "FIXED")]
    FIXED,

    #[graphql(name = "PERCENTAGE")]
    PERCENTAGE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SaleSortField {

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "START_DATE")]
    STARTDATE,

    #[graphql(name = "END_DATE")]
    ENDDATE,

    #[graphql(name = "VALUE")]
    VALUE,

    #[graphql(name = "TYPE")]
    TYPE,

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

    #[graphql(name = "LAST_MODIFIED_AT")]
    LASTMODIFIEDAT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SaleType {

    #[graphql(name = "FIXED")]
    FIXED,

    #[graphql(name = "PERCENTAGE")]
    PERCENTAGE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SendConfirmationEmailErrorCode {

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "ACCOUNT_CONFIRMED")]
    ACCOUNTCONFIRMED,

    #[graphql(name = "CONFIRMATION_ALREADY_REQUESTED")]
    CONFIRMATIONALREADYREQUESTED,

    #[graphql(name = "MISSING_CHANNEL_SLUG")]
    MISSINGCHANNELSLUG,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShippingErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "MAX_LESS_THAN_MIN")]
    MAXLESSTHANMIN,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShippingMethodTypeEnum {

    #[graphql(name = "PRICE")]
    PRICE,

    #[graphql(name = "WEIGHT")]
    WEIGHT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShopErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "CANNOT_FETCH_TAX_RATES")]
    CANNOTFETCHTAXRATES,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "PASSWORD_AUTH_RESTRICTION")]
    PASSWORDAUTHRESTRICTION,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum StaffMemberStatus {

    #[graphql(name = "ACTIVE")]
    ACTIVE,

    #[graphql(name = "DEACTIVATED")]
    DEACTIVATED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum StockAvailability {

    #[graphql(name = "IN_STOCK")]
    INSTOCK,

    #[graphql(name = "OUT_OF_STOCK")]
    OUTOFSTOCK,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum StockBulkUpdateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum StockErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum StockUpdatePolicyEnum {

    #[graphql(name = "SKIP")]
    SKIP,

    #[graphql(name = "UPDATE")]
    UPDATE,

    #[graphql(name = "FORCE")]
    FORCE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum StorePaymentMethodEnum {

    #[graphql(name = "ON_SESSION")]
    ONSESSION,

    #[graphql(name = "OFF_SESSION")]
    OFFSESSION,

    #[graphql(name = "NONE")]
    NONE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum StoredPaymentMethodRequestDeleteErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "CHANNEL_INACTIVE")]
    CHANNELINACTIVE,

    #[graphql(name = "GATEWAY_ERROR")]
    GATEWAYERROR,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum StoredPaymentMethodRequestDeleteResult {

    #[graphql(name = "SUCCESSFULLY_DELETED")]
    SUCCESSFULLYDELETED,

    #[graphql(name = "FAILED_TO_DELETE")]
    FAILEDTODELETE,

    #[graphql(name = "FAILED_TO_DELIVER")]
    FAILEDTODELIVER,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxCalculationStrategy {

    #[graphql(name = "FLAT_RATES")]
    FLATRATES,

    #[graphql(name = "TAX_APP")]
    TAXAPP,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxClassCreateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxClassDeleteErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxClassSortField {

    #[graphql(name = "NAME")]
    NAME,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxClassUpdateErrorCode {

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxConfigurationUpdateErrorCode {

    #[graphql(name = "DUPLICATED_INPUT_ITEM")]
    DUPLICATEDINPUTITEM,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxCountryConfigurationDeleteErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxCountryConfigurationUpdateErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "ONLY_ONE_DEFAULT_COUNTRY_RATE_ALLOWED")]
    ONLYONEDEFAULTCOUNTRYRATEALLOWED,

    #[graphql(name = "CANNOT_CREATE_NEGATIVE_RATE")]
    CANNOTCREATENEGATIVERATE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxExemptionManageErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "NOT_EDITABLE_ORDER")]
    NOTEDITABLEORDER,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaxableObjectDiscountTypeEnum {

    #[graphql(name = "SUBTOTAL")]
    SUBTOTAL,

    #[graphql(name = "SHIPPING")]
    SHIPPING,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThumbnailFormatEnum {

    #[graphql(name = "ORIGINAL")]
    ORIGINAL,

    #[graphql(name = "AVIF")]
    AVIF,

    #[graphql(name = "WEBP")]
    WEBP,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimePeriodTypeEnum {

    #[graphql(name = "DAY")]
    DAY,

    #[graphql(name = "WEEK")]
    WEEK,

    #[graphql(name = "MONTH")]
    MONTH,

    #[graphql(name = "YEAR")]
    YEAR,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenizedPaymentFlowEnum {

    #[graphql(name = "INTERACTIVE")]
    INTERACTIVE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionActionEnum {

    #[graphql(name = "CHARGE")]
    CHARGE,

    #[graphql(name = "REFUND")]
    REFUND,

    #[graphql(name = "CANCEL")]
    CANCEL,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionCreateErrorCode {

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "INCORRECT_CURRENCY")]
    INCORRECTCURRENCY,

    #[graphql(name = "METADATA_KEY_REQUIRED")]
    METADATAKEYREQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionEventReportErrorCode {

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "INCORRECT_DETAILS")]
    INCORRECTDETAILS,

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionEventTypeEnum {

    #[graphql(name = "AUTHORIZATION_SUCCESS")]
    AUTHORIZATIONSUCCESS,

    #[graphql(name = "AUTHORIZATION_FAILURE")]
    AUTHORIZATIONFAILURE,

    #[graphql(name = "AUTHORIZATION_ADJUSTMENT")]
    AUTHORIZATIONADJUSTMENT,

    #[graphql(name = "AUTHORIZATION_REQUEST")]
    AUTHORIZATIONREQUEST,

    #[graphql(name = "AUTHORIZATION_ACTION_REQUIRED")]
    AUTHORIZATIONACTIONREQUIRED,

    #[graphql(name = "CHARGE_ACTION_REQUIRED")]
    CHARGEACTIONREQUIRED,

    #[graphql(name = "CHARGE_SUCCESS")]
    CHARGESUCCESS,

    #[graphql(name = "CHARGE_FAILURE")]
    CHARGEFAILURE,

    #[graphql(name = "CHARGE_BACK")]
    CHARGEBACK,

    #[graphql(name = "CHARGE_REQUEST")]
    CHARGEREQUEST,

    #[graphql(name = "REFUND_SUCCESS")]
    REFUNDSUCCESS,

    #[graphql(name = "REFUND_FAILURE")]
    REFUNDFAILURE,

    #[graphql(name = "REFUND_REVERSE")]
    REFUNDREVERSE,

    #[graphql(name = "REFUND_REQUEST")]
    REFUNDREQUEST,

    #[graphql(name = "CANCEL_SUCCESS")]
    CANCELSUCCESS,

    #[graphql(name = "CANCEL_FAILURE")]
    CANCELFAILURE,

    #[graphql(name = "CANCEL_REQUEST")]
    CANCELREQUEST,

    #[graphql(name = "INFO")]
    INFO,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionFlowStrategyEnum {

    #[graphql(name = "AUTHORIZATION")]
    AUTHORIZATION,

    #[graphql(name = "CHARGE")]
    CHARGE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionInitializeErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "CHECKOUT_COMPLETION_IN_PROGRESS")]
    CHECKOUTCOMPLETIONINPROGRESS,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionKind {

    #[graphql(name = "EXTERNAL")]
    EXTERNAL,

    #[graphql(name = "AUTH")]
    AUTH,

    #[graphql(name = "PENDING")]
    PENDING,

    #[graphql(name = "ACTION_TO_CONFIRM")]
    ACTIONTOCONFIRM,

    #[graphql(name = "REFUND")]
    REFUND,

    #[graphql(name = "REFUND_ONGOING")]
    REFUNDONGOING,

    #[graphql(name = "CAPTURE")]
    CAPTURE,

    #[graphql(name = "VOID")]
    VOID,

    #[graphql(name = "CONFIRM")]
    CONFIRM,

    #[graphql(name = "CANCEL")]
    CANCEL,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionProcessErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "TRANSACTION_ALREADY_PROCESSED")]
    TRANSACTIONALREADYPROCESSED,

    #[graphql(name = "MISSING_PAYMENT_APP_RELATION")]
    MISSINGPAYMENTAPPRELATION,

    #[graphql(name = "MISSING_PAYMENT_APP")]
    MISSINGPAYMENTAPP,

    #[graphql(name = "CHECKOUT_COMPLETION_IN_PROGRESS")]
    CHECKOUTCOMPLETIONINPROGRESS,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionRequestActionErrorCode {

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "MISSING_TRANSACTION_ACTION_REQUEST_WEBHOOK")]
    MISSINGTRANSACTIONACTIONREQUESTWEBHOOK,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionRequestRefundForGrantedRefundErrorCode {

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "AMOUNT_GREATER_THAN_AVAILABLE")]
    AMOUNTGREATERTHANAVAILABLE,

    #[graphql(name = "MISSING_TRANSACTION_ACTION_REQUEST_WEBHOOK")]
    MISSINGTRANSACTIONACTIONREQUESTWEBHOOK,

    #[graphql(name = "REFUND_ALREADY_PROCESSED")]
    REFUNDALREADYPROCESSED,

    #[graphql(name = "REFUND_IS_PENDING")]
    REFUNDISPENDING,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionSortField {

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

    #[graphql(name = "MODIFIED_AT")]
    MODIFIEDAT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransactionUpdateErrorCode {

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "INCORRECT_CURRENCY")]
    INCORRECTCURRENCY,

    #[graphql(name = "METADATA_KEY_REQUIRED")]
    METADATAKEYREQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TranslatableKinds {

    #[graphql(name = "ATTRIBUTE")]
    ATTRIBUTE,

    #[graphql(name = "ATTRIBUTE_VALUE")]
    ATTRIBUTEVALUE,

    #[graphql(name = "CATEGORY")]
    CATEGORY,

    #[graphql(name = "COLLECTION")]
    COLLECTION,

    #[graphql(name = "MENU_ITEM")]
    MENUITEM,

    #[graphql(name = "PAGE")]
    PAGE,

    #[graphql(name = "PRODUCT")]
    PRODUCT,

    #[graphql(name = "PROMOTION")]
    PROMOTION,

    #[graphql(name = "PROMOTION_RULE")]
    PROMOTIONRULE,

    #[graphql(name = "SALE")]
    SALE,

    #[graphql(name = "SHIPPING_METHOD")]
    SHIPPINGMETHOD,

    #[graphql(name = "VARIANT")]
    VARIANT,

    #[graphql(name = "VOUCHER")]
    VOUCHER,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TranslationErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum UploadErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID_FILE_TYPE")]
    INVALIDFILETYPE,

    #[graphql(name = "UNSUPPORTED_MIME_TYPE")]
    UNSUPPORTEDMIMETYPE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum UserSortField {

    #[graphql(name = "FIRST_NAME")]
    FIRSTNAME,

    #[graphql(name = "LAST_NAME")]
    LASTNAME,

    #[graphql(name = "EMAIL")]
    EMAIL,

    #[graphql(name = "ORDER_COUNT")]
    ORDERCOUNT,

    #[graphql(name = "CREATED_AT")]
    CREATEDAT,

    #[graphql(name = "LAST_MODIFIED_AT")]
    LASTMODIFIEDAT,

    #[graphql(name = "RANK")]
    RANK,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum VariantAttributeScope {

    #[graphql(name = "ALL")]
    ALL,

    #[graphql(name = "VARIANT_SELECTION")]
    VARIANTSELECTION,

    #[graphql(name = "NOT_VARIANT_SELECTION")]
    NOTVARIANTSELECTION,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum VolumeUnitsEnum {

    #[graphql(name = "CUBIC_MILLIMETER")]
    CUBICMILLIMETER,

    #[graphql(name = "CUBIC_CENTIMETER")]
    CUBICCENTIMETER,

    #[graphql(name = "CUBIC_DECIMETER")]
    CUBICDECIMETER,

    #[graphql(name = "CUBIC_METER")]
    CUBICMETER,

    #[graphql(name = "LITER")]
    LITER,

    #[graphql(name = "CUBIC_FOOT")]
    CUBICFOOT,

    #[graphql(name = "CUBIC_INCH")]
    CUBICINCH,

    #[graphql(name = "CUBIC_YARD")]
    CUBICYARD,

    #[graphql(name = "QT")]
    QT,

    #[graphql(name = "PINT")]
    PINT,

    #[graphql(name = "FL_OZ")]
    FLOZ,

    #[graphql(name = "ACRE_IN")]
    ACREIN,

    #[graphql(name = "ACRE_FT")]
    ACREFT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoucherCodeBulkDeleteErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "INVALID")]
    INVALID,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoucherDiscountType {

    #[graphql(name = "FIXED")]
    FIXED,

    #[graphql(name = "PERCENTAGE")]
    PERCENTAGE,

    #[graphql(name = "SHIPPING")]
    SHIPPING,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoucherSortField {

    #[graphql(name = "CODE")]
    CODE,

    #[graphql(name = "NAME")]
    NAME,

    #[graphql(name = "START_DATE")]
    STARTDATE,

    #[graphql(name = "END_DATE")]
    ENDDATE,

    #[graphql(name = "VALUE")]
    VALUE,

    #[graphql(name = "TYPE")]
    TYPE,

    #[graphql(name = "USAGE_LIMIT")]
    USAGELIMIT,

    #[graphql(name = "MINIMUM_SPENT_AMOUNT")]
    MINIMUMSPENTAMOUNT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum VoucherTypeEnum {

    #[graphql(name = "SHIPPING")]
    SHIPPING,

    #[graphql(name = "ENTIRE_ORDER")]
    ENTIREORDER,

    #[graphql(name = "SPECIFIC_PRODUCT")]
    SPECIFICPRODUCT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WarehouseClickAndCollectOptionEnum {

    #[graphql(name = "DISABLED")]
    DISABLED,

    #[graphql(name = "LOCAL")]
    LOCAL,

    #[graphql(name = "ALL")]
    ALL,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WarehouseErrorCode {

    #[graphql(name = "ALREADY_EXISTS")]
    ALREADYEXISTS,

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WarehouseSortField {

    #[graphql(name = "NAME")]
    NAME,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WebhookDryRunErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "INVALID_ID")]
    INVALIDID,

    #[graphql(name = "MISSING_PERMISSION")]
    MISSINGPERMISSION,

    #[graphql(name = "TYPE_NOT_SUPPORTED")]
    TYPENOTSUPPORTED,

    #[graphql(name = "SYNTAX")]
    SYNTAX,

    #[graphql(name = "MISSING_SUBSCRIPTION")]
    MISSINGSUBSCRIPTION,

    #[graphql(name = "UNABLE_TO_PARSE")]
    UNABLETOPARSE,

    #[graphql(name = "MISSING_EVENT")]
    MISSINGEVENT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WebhookErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "INVALID")]
    INVALID,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "REQUIRED")]
    REQUIRED,

    #[graphql(name = "UNIQUE")]
    UNIQUE,

    #[graphql(name = "DELETE_FAILED")]
    DELETEFAILED,

    #[graphql(name = "SYNTAX")]
    SYNTAX,

    #[graphql(name = "MISSING_SUBSCRIPTION")]
    MISSINGSUBSCRIPTION,

    #[graphql(name = "UNABLE_TO_PARSE")]
    UNABLETOPARSE,

    #[graphql(name = "MISSING_EVENT")]
    MISSINGEVENT,

    #[graphql(name = "INVALID_CUSTOM_HEADERS")]
    INVALIDCUSTOMHEADERS,

    #[graphql(name = "INVALID_NOTIFY_WITH_SUBSCRIPTION")]
    INVALIDNOTIFYWITHSUBSCRIPTION,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WebhookEventTypeAsyncEnum {

    #[graphql(name = "ANY_EVENTS")]
    ANYEVENTS,

    #[graphql(name = "ACCOUNT_CONFIRMATION_REQUESTED")]
    ACCOUNTCONFIRMATIONREQUESTED,

    #[graphql(name = "ACCOUNT_CHANGE_EMAIL_REQUESTED")]
    ACCOUNTCHANGEEMAILREQUESTED,

    #[graphql(name = "ACCOUNT_EMAIL_CHANGED")]
    ACCOUNTEMAILCHANGED,

    #[graphql(name = "ACCOUNT_SET_PASSWORD_REQUESTED")]
    ACCOUNTSETPASSWORDREQUESTED,

    #[graphql(name = "ACCOUNT_CONFIRMED")]
    ACCOUNTCONFIRMED,

    #[graphql(name = "ACCOUNT_DELETE_REQUESTED")]
    ACCOUNTDELETEREQUESTED,

    #[graphql(name = "ACCOUNT_DELETED")]
    ACCOUNTDELETED,

    #[graphql(name = "ADDRESS_CREATED")]
    ADDRESSCREATED,

    #[graphql(name = "ADDRESS_UPDATED")]
    ADDRESSUPDATED,

    #[graphql(name = "ADDRESS_DELETED")]
    ADDRESSDELETED,

    #[graphql(name = "APP_INSTALLED")]
    APPINSTALLED,

    #[graphql(name = "APP_UPDATED")]
    APPUPDATED,

    #[graphql(name = "APP_DELETED")]
    APPDELETED,

    #[graphql(name = "APP_STATUS_CHANGED")]
    APPSTATUSCHANGED,

    #[graphql(name = "ATTRIBUTE_CREATED")]
    ATTRIBUTECREATED,

    #[graphql(name = "ATTRIBUTE_UPDATED")]
    ATTRIBUTEUPDATED,

    #[graphql(name = "ATTRIBUTE_DELETED")]
    ATTRIBUTEDELETED,

    #[graphql(name = "ATTRIBUTE_VALUE_CREATED")]
    ATTRIBUTEVALUECREATED,

    #[graphql(name = "ATTRIBUTE_VALUE_UPDATED")]
    ATTRIBUTEVALUEUPDATED,

    #[graphql(name = "ATTRIBUTE_VALUE_DELETED")]
    ATTRIBUTEVALUEDELETED,

    #[graphql(name = "CATEGORY_CREATED")]
    CATEGORYCREATED,

    #[graphql(name = "CATEGORY_UPDATED")]
    CATEGORYUPDATED,

    #[graphql(name = "CATEGORY_DELETED")]
    CATEGORYDELETED,

    #[graphql(name = "CHANNEL_CREATED")]
    CHANNELCREATED,

    #[graphql(name = "CHANNEL_UPDATED")]
    CHANNELUPDATED,

    #[graphql(name = "CHANNEL_DELETED")]
    CHANNELDELETED,

    #[graphql(name = "CHANNEL_STATUS_CHANGED")]
    CHANNELSTATUSCHANGED,

    #[graphql(name = "CHANNEL_METADATA_UPDATED")]
    CHANNELMETADATAUPDATED,

    #[graphql(name = "GIFT_CARD_CREATED")]
    GIFTCARDCREATED,

    #[graphql(name = "GIFT_CARD_UPDATED")]
    GIFTCARDUPDATED,

    #[graphql(name = "GIFT_CARD_DELETED")]
    GIFTCARDDELETED,

    #[graphql(name = "GIFT_CARD_SENT")]
    GIFTCARDSENT,

    #[graphql(name = "GIFT_CARD_STATUS_CHANGED")]
    GIFTCARDSTATUSCHANGED,

    #[graphql(name = "GIFT_CARD_METADATA_UPDATED")]
    GIFTCARDMETADATAUPDATED,

    #[graphql(name = "GIFT_CARD_EXPORT_COMPLETED")]
    GIFTCARDEXPORTCOMPLETED,

    #[graphql(name = "MENU_CREATED")]
    MENUCREATED,

    #[graphql(name = "MENU_UPDATED")]
    MENUUPDATED,

    #[graphql(name = "MENU_DELETED")]
    MENUDELETED,

    #[graphql(name = "MENU_ITEM_CREATED")]
    MENUITEMCREATED,

    #[graphql(name = "MENU_ITEM_UPDATED")]
    MENUITEMUPDATED,

    #[graphql(name = "MENU_ITEM_DELETED")]
    MENUITEMDELETED,

    #[graphql(name = "ORDER_CREATED")]
    ORDERCREATED,

    #[graphql(name = "ORDER_CONFIRMED")]
    ORDERCONFIRMED,

    #[graphql(name = "ORDER_PAID")]
    ORDERPAID,

    #[graphql(name = "ORDER_FULLY_PAID")]
    ORDERFULLYPAID,

    #[graphql(name = "ORDER_REFUNDED")]
    ORDERREFUNDED,

    #[graphql(name = "ORDER_FULLY_REFUNDED")]
    ORDERFULLYREFUNDED,

    #[graphql(name = "ORDER_UPDATED")]
    ORDERUPDATED,

    #[graphql(name = "ORDER_CANCELLED")]
    ORDERCANCELLED,

    #[graphql(name = "ORDER_EXPIRED")]
    ORDEREXPIRED,

    #[graphql(name = "ORDER_FULFILLED")]
    ORDERFULFILLED,

    #[graphql(name = "ORDER_METADATA_UPDATED")]
    ORDERMETADATAUPDATED,

    #[graphql(name = "ORDER_BULK_CREATED")]
    ORDERBULKCREATED,

    #[graphql(name = "FULFILLMENT_CREATED")]
    FULFILLMENTCREATED,

    #[graphql(name = "FULFILLMENT_CANCELED")]
    FULFILLMENTCANCELED,

    #[graphql(name = "FULFILLMENT_APPROVED")]
    FULFILLMENTAPPROVED,

    #[graphql(name = "FULFILLMENT_METADATA_UPDATED")]
    FULFILLMENTMETADATAUPDATED,

    #[graphql(name = "FULFILLMENT_TRACKING_NUMBER_UPDATED")]
    FULFILLMENTTRACKINGNUMBERUPDATED,

    #[graphql(name = "DRAFT_ORDER_CREATED")]
    DRAFTORDERCREATED,

    #[graphql(name = "DRAFT_ORDER_UPDATED")]
    DRAFTORDERUPDATED,

    #[graphql(name = "DRAFT_ORDER_DELETED")]
    DRAFTORDERDELETED,

    #[graphql(name = "SALE_CREATED")]
    SALECREATED,

    #[graphql(name = "SALE_UPDATED")]
    SALEUPDATED,

    #[graphql(name = "SALE_DELETED")]
    SALEDELETED,

    #[graphql(name = "SALE_TOGGLE")]
    SALETOGGLE,

    #[graphql(name = "PROMOTION_CREATED")]
    PROMOTIONCREATED,

    #[graphql(name = "PROMOTION_UPDATED")]
    PROMOTIONUPDATED,

    #[graphql(name = "PROMOTION_DELETED")]
    PROMOTIONDELETED,

    #[graphql(name = "PROMOTION_STARTED")]
    PROMOTIONSTARTED,

    #[graphql(name = "PROMOTION_ENDED")]
    PROMOTIONENDED,

    #[graphql(name = "PROMOTION_RULE_CREATED")]
    PROMOTIONRULECREATED,

    #[graphql(name = "PROMOTION_RULE_UPDATED")]
    PROMOTIONRULEUPDATED,

    #[graphql(name = "PROMOTION_RULE_DELETED")]
    PROMOTIONRULEDELETED,

    #[graphql(name = "INVOICE_REQUESTED")]
    INVOICEREQUESTED,

    #[graphql(name = "INVOICE_DELETED")]
    INVOICEDELETED,

    #[graphql(name = "INVOICE_SENT")]
    INVOICESENT,

    #[graphql(name = "CUSTOMER_CREATED")]
    CUSTOMERCREATED,

    #[graphql(name = "CUSTOMER_UPDATED")]
    CUSTOMERUPDATED,

    #[graphql(name = "CUSTOMER_DELETED")]
    CUSTOMERDELETED,

    #[graphql(name = "CUSTOMER_METADATA_UPDATED")]
    CUSTOMERMETADATAUPDATED,

    #[graphql(name = "CUSTOMER_TYPE_CREATED")]
    CUSTOMERTYPECREATED,

    #[graphql(name = "CUSTOMER_TYPE_UPDATED")]
    CUSTOMERTYPEUPDATED,

    #[graphql(name = "CUSTOMER_TYPE_DELETED")]
    CUSTOMERTYPEDELETED,

    #[graphql(name = "COLLECTION_CREATED")]
    COLLECTIONCREATED,

    #[graphql(name = "COLLECTION_UPDATED")]
    COLLECTIONUPDATED,

    #[graphql(name = "COLLECTION_DELETED")]
    COLLECTIONDELETED,

    #[graphql(name = "COLLECTION_METADATA_UPDATED")]
    COLLECTIONMETADATAUPDATED,

    #[graphql(name = "PRODUCT_CREATED")]
    PRODUCTCREATED,

    #[graphql(name = "PRODUCT_UPDATED")]
    PRODUCTUPDATED,

    #[graphql(name = "PRODUCT_DELETED")]
    PRODUCTDELETED,

    #[graphql(name = "PRODUCT_METADATA_UPDATED")]
    PRODUCTMETADATAUPDATED,

    #[graphql(name = "PRODUCT_EXPORT_COMPLETED")]
    PRODUCTEXPORTCOMPLETED,

    #[graphql(name = "PRODUCT_MEDIA_CREATED")]
    PRODUCTMEDIACREATED,

    #[graphql(name = "PRODUCT_MEDIA_UPDATED")]
    PRODUCTMEDIAUPDATED,

    #[graphql(name = "PRODUCT_MEDIA_DELETED")]
    PRODUCTMEDIADELETED,

    #[graphql(name = "PRODUCT_VARIANT_CREATED")]
    PRODUCTVARIANTCREATED,

    #[graphql(name = "PRODUCT_VARIANT_UPDATED")]
    PRODUCTVARIANTUPDATED,

    #[graphql(name = "PRODUCT_VARIANT_DELETED")]
    PRODUCTVARIANTDELETED,

    #[graphql(name = "PRODUCT_VARIANT_METADATA_UPDATED")]
    PRODUCTVARIANTMETADATAUPDATED,

    #[graphql(name = "PRODUCT_VARIANT_OUT_OF_STOCK")]
    PRODUCTVARIANTOUTOFSTOCK,

    #[graphql(name = "PRODUCT_VARIANT_BACK_IN_STOCK")]
    PRODUCTVARIANTBACKINSTOCK,

    #[graphql(name = "PRODUCT_VARIANT_STOCK_UPDATED")]
    PRODUCTVARIANTSTOCKUPDATED,

    #[graphql(name = "PRODUCT_VARIANT_OUT_OF_STOCK_IN_CHANNEL")]
    PRODUCTVARIANTOUTOFSTOCKINCHANNEL,

    #[graphql(name = "PRODUCT_VARIANT_BACK_IN_STOCK_IN_CHANNEL")]
    PRODUCTVARIANTBACKINSTOCKINCHANNEL,

    #[graphql(name = "PRODUCT_VARIANT_OUT_OF_STOCK_FOR_CLICK_AND_COLLECT")]
    PRODUCTVARIANTOUTOFSTOCKFORCLICKANDCOLLECT,

    #[graphql(name = "PRODUCT_VARIANT_BACK_IN_STOCK_FOR_CLICK_AND_COLLECT")]
    PRODUCTVARIANTBACKINSTOCKFORCLICKANDCOLLECT,

    #[graphql(name = "PRODUCT_VARIANT_DISCOUNTED_PRICE_UPDATED")]
    PRODUCTVARIANTDISCOUNTEDPRICEUPDATED,

    #[graphql(name = "CHECKOUT_CREATED")]
    CHECKOUTCREATED,

    #[graphql(name = "CHECKOUT_UPDATED")]
    CHECKOUTUPDATED,

    #[graphql(name = "CHECKOUT_FULLY_AUTHORIZED")]
    CHECKOUTFULLYAUTHORIZED,

    #[graphql(name = "CHECKOUT_FULLY_PAID")]
    CHECKOUTFULLYPAID,

    #[graphql(name = "CHECKOUT_METADATA_UPDATED")]
    CHECKOUTMETADATAUPDATED,

    #[graphql(name = "NOTIFY_USER")]
    NOTIFYUSER,

    #[graphql(name = "PAGE_CREATED")]
    PAGECREATED,

    #[graphql(name = "PAGE_UPDATED")]
    PAGEUPDATED,

    #[graphql(name = "PAGE_DELETED")]
    PAGEDELETED,

    #[graphql(name = "PAGE_TYPE_CREATED")]
    PAGETYPECREATED,

    #[graphql(name = "PAGE_TYPE_UPDATED")]
    PAGETYPEUPDATED,

    #[graphql(name = "PAGE_TYPE_DELETED")]
    PAGETYPEDELETED,

    #[graphql(name = "PERMISSION_GROUP_CREATED")]
    PERMISSIONGROUPCREATED,

    #[graphql(name = "PERMISSION_GROUP_UPDATED")]
    PERMISSIONGROUPUPDATED,

    #[graphql(name = "PERMISSION_GROUP_DELETED")]
    PERMISSIONGROUPDELETED,

    #[graphql(name = "SHIPPING_PRICE_CREATED")]
    SHIPPINGPRICECREATED,

    #[graphql(name = "SHIPPING_PRICE_UPDATED")]
    SHIPPINGPRICEUPDATED,

    #[graphql(name = "SHIPPING_PRICE_DELETED")]
    SHIPPINGPRICEDELETED,

    #[graphql(name = "SHIPPING_ZONE_CREATED")]
    SHIPPINGZONECREATED,

    #[graphql(name = "SHIPPING_ZONE_UPDATED")]
    SHIPPINGZONEUPDATED,

    #[graphql(name = "SHIPPING_ZONE_DELETED")]
    SHIPPINGZONEDELETED,

    #[graphql(name = "SHIPPING_ZONE_METADATA_UPDATED")]
    SHIPPINGZONEMETADATAUPDATED,

    #[graphql(name = "STAFF_CREATED")]
    STAFFCREATED,

    #[graphql(name = "STAFF_UPDATED")]
    STAFFUPDATED,

    #[graphql(name = "STAFF_DELETED")]
    STAFFDELETED,

    #[graphql(name = "STAFF_SET_PASSWORD_REQUESTED")]
    STAFFSETPASSWORDREQUESTED,

    #[graphql(name = "TRANSACTION_ITEM_METADATA_UPDATED")]
    TRANSACTIONITEMMETADATAUPDATED,

    #[graphql(name = "TRANSLATION_CREATED")]
    TRANSLATIONCREATED,

    #[graphql(name = "TRANSLATION_UPDATED")]
    TRANSLATIONUPDATED,

    #[graphql(name = "WAREHOUSE_CREATED")]
    WAREHOUSECREATED,

    #[graphql(name = "WAREHOUSE_UPDATED")]
    WAREHOUSEUPDATED,

    #[graphql(name = "WAREHOUSE_DELETED")]
    WAREHOUSEDELETED,

    #[graphql(name = "WAREHOUSE_METADATA_UPDATED")]
    WAREHOUSEMETADATAUPDATED,

    #[graphql(name = "VOUCHER_CREATED")]
    VOUCHERCREATED,

    #[graphql(name = "VOUCHER_UPDATED")]
    VOUCHERUPDATED,

    #[graphql(name = "VOUCHER_DELETED")]
    VOUCHERDELETED,

    #[graphql(name = "VOUCHER_CODES_CREATED")]
    VOUCHERCODESCREATED,

    #[graphql(name = "VOUCHER_CODES_DELETED")]
    VOUCHERCODESDELETED,

    #[graphql(name = "VOUCHER_METADATA_UPDATED")]
    VOUCHERMETADATAUPDATED,

    #[graphql(name = "VOUCHER_CODE_EXPORT_COMPLETED")]
    VOUCHERCODEEXPORTCOMPLETED,

    #[graphql(name = "OBSERVABILITY")]
    OBSERVABILITY,

    #[graphql(name = "THUMBNAIL_CREATED")]
    THUMBNAILCREATED,

    #[graphql(name = "SHOP_METADATA_UPDATED")]
    SHOPMETADATAUPDATED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WebhookEventTypeEnum {

    #[graphql(name = "ANY_EVENTS")]
    ANYEVENTS,

    #[graphql(name = "ACCOUNT_CONFIRMATION_REQUESTED")]
    ACCOUNTCONFIRMATIONREQUESTED,

    #[graphql(name = "ACCOUNT_CHANGE_EMAIL_REQUESTED")]
    ACCOUNTCHANGEEMAILREQUESTED,

    #[graphql(name = "ACCOUNT_EMAIL_CHANGED")]
    ACCOUNTEMAILCHANGED,

    #[graphql(name = "ACCOUNT_SET_PASSWORD_REQUESTED")]
    ACCOUNTSETPASSWORDREQUESTED,

    #[graphql(name = "ACCOUNT_CONFIRMED")]
    ACCOUNTCONFIRMED,

    #[graphql(name = "ACCOUNT_DELETE_REQUESTED")]
    ACCOUNTDELETEREQUESTED,

    #[graphql(name = "ACCOUNT_DELETED")]
    ACCOUNTDELETED,

    #[graphql(name = "ADDRESS_CREATED")]
    ADDRESSCREATED,

    #[graphql(name = "ADDRESS_UPDATED")]
    ADDRESSUPDATED,

    #[graphql(name = "ADDRESS_DELETED")]
    ADDRESSDELETED,

    #[graphql(name = "APP_INSTALLED")]
    APPINSTALLED,

    #[graphql(name = "APP_UPDATED")]
    APPUPDATED,

    #[graphql(name = "APP_DELETED")]
    APPDELETED,

    #[graphql(name = "APP_STATUS_CHANGED")]
    APPSTATUSCHANGED,

    #[graphql(name = "ATTRIBUTE_CREATED")]
    ATTRIBUTECREATED,

    #[graphql(name = "ATTRIBUTE_UPDATED")]
    ATTRIBUTEUPDATED,

    #[graphql(name = "ATTRIBUTE_DELETED")]
    ATTRIBUTEDELETED,

    #[graphql(name = "ATTRIBUTE_VALUE_CREATED")]
    ATTRIBUTEVALUECREATED,

    #[graphql(name = "ATTRIBUTE_VALUE_UPDATED")]
    ATTRIBUTEVALUEUPDATED,

    #[graphql(name = "ATTRIBUTE_VALUE_DELETED")]
    ATTRIBUTEVALUEDELETED,

    #[graphql(name = "CATEGORY_CREATED")]
    CATEGORYCREATED,

    #[graphql(name = "CATEGORY_UPDATED")]
    CATEGORYUPDATED,

    #[graphql(name = "CATEGORY_DELETED")]
    CATEGORYDELETED,

    #[graphql(name = "CHANNEL_CREATED")]
    CHANNELCREATED,

    #[graphql(name = "CHANNEL_UPDATED")]
    CHANNELUPDATED,

    #[graphql(name = "CHANNEL_DELETED")]
    CHANNELDELETED,

    #[graphql(name = "CHANNEL_STATUS_CHANGED")]
    CHANNELSTATUSCHANGED,

    #[graphql(name = "CHANNEL_METADATA_UPDATED")]
    CHANNELMETADATAUPDATED,

    #[graphql(name = "GIFT_CARD_CREATED")]
    GIFTCARDCREATED,

    #[graphql(name = "GIFT_CARD_UPDATED")]
    GIFTCARDUPDATED,

    #[graphql(name = "GIFT_CARD_DELETED")]
    GIFTCARDDELETED,

    #[graphql(name = "GIFT_CARD_SENT")]
    GIFTCARDSENT,

    #[graphql(name = "GIFT_CARD_STATUS_CHANGED")]
    GIFTCARDSTATUSCHANGED,

    #[graphql(name = "GIFT_CARD_METADATA_UPDATED")]
    GIFTCARDMETADATAUPDATED,

    #[graphql(name = "GIFT_CARD_EXPORT_COMPLETED")]
    GIFTCARDEXPORTCOMPLETED,

    #[graphql(name = "MENU_CREATED")]
    MENUCREATED,

    #[graphql(name = "MENU_UPDATED")]
    MENUUPDATED,

    #[graphql(name = "MENU_DELETED")]
    MENUDELETED,

    #[graphql(name = "MENU_ITEM_CREATED")]
    MENUITEMCREATED,

    #[graphql(name = "MENU_ITEM_UPDATED")]
    MENUITEMUPDATED,

    #[graphql(name = "MENU_ITEM_DELETED")]
    MENUITEMDELETED,

    #[graphql(name = "ORDER_CREATED")]
    ORDERCREATED,

    #[graphql(name = "ORDER_CONFIRMED")]
    ORDERCONFIRMED,

    #[graphql(name = "ORDER_PAID")]
    ORDERPAID,

    #[graphql(name = "ORDER_FULLY_PAID")]
    ORDERFULLYPAID,

    #[graphql(name = "ORDER_REFUNDED")]
    ORDERREFUNDED,

    #[graphql(name = "ORDER_FULLY_REFUNDED")]
    ORDERFULLYREFUNDED,

    #[graphql(name = "ORDER_UPDATED")]
    ORDERUPDATED,

    #[graphql(name = "ORDER_CANCELLED")]
    ORDERCANCELLED,

    #[graphql(name = "ORDER_EXPIRED")]
    ORDEREXPIRED,

    #[graphql(name = "ORDER_FULFILLED")]
    ORDERFULFILLED,

    #[graphql(name = "ORDER_METADATA_UPDATED")]
    ORDERMETADATAUPDATED,

    #[graphql(name = "ORDER_BULK_CREATED")]
    ORDERBULKCREATED,

    #[graphql(name = "FULFILLMENT_CREATED")]
    FULFILLMENTCREATED,

    #[graphql(name = "FULFILLMENT_CANCELED")]
    FULFILLMENTCANCELED,

    #[graphql(name = "FULFILLMENT_APPROVED")]
    FULFILLMENTAPPROVED,

    #[graphql(name = "FULFILLMENT_METADATA_UPDATED")]
    FULFILLMENTMETADATAUPDATED,

    #[graphql(name = "FULFILLMENT_TRACKING_NUMBER_UPDATED")]
    FULFILLMENTTRACKINGNUMBERUPDATED,

    #[graphql(name = "DRAFT_ORDER_CREATED")]
    DRAFTORDERCREATED,

    #[graphql(name = "DRAFT_ORDER_UPDATED")]
    DRAFTORDERUPDATED,

    #[graphql(name = "DRAFT_ORDER_DELETED")]
    DRAFTORDERDELETED,

    #[graphql(name = "SALE_CREATED")]
    SALECREATED,

    #[graphql(name = "SALE_UPDATED")]
    SALEUPDATED,

    #[graphql(name = "SALE_DELETED")]
    SALEDELETED,

    #[graphql(name = "SALE_TOGGLE")]
    SALETOGGLE,

    #[graphql(name = "PROMOTION_CREATED")]
    PROMOTIONCREATED,

    #[graphql(name = "PROMOTION_UPDATED")]
    PROMOTIONUPDATED,

    #[graphql(name = "PROMOTION_DELETED")]
    PROMOTIONDELETED,

    #[graphql(name = "PROMOTION_STARTED")]
    PROMOTIONSTARTED,

    #[graphql(name = "PROMOTION_ENDED")]
    PROMOTIONENDED,

    #[graphql(name = "PROMOTION_RULE_CREATED")]
    PROMOTIONRULECREATED,

    #[graphql(name = "PROMOTION_RULE_UPDATED")]
    PROMOTIONRULEUPDATED,

    #[graphql(name = "PROMOTION_RULE_DELETED")]
    PROMOTIONRULEDELETED,

    #[graphql(name = "INVOICE_REQUESTED")]
    INVOICEREQUESTED,

    #[graphql(name = "INVOICE_DELETED")]
    INVOICEDELETED,

    #[graphql(name = "INVOICE_SENT")]
    INVOICESENT,

    #[graphql(name = "CUSTOMER_CREATED")]
    CUSTOMERCREATED,

    #[graphql(name = "CUSTOMER_UPDATED")]
    CUSTOMERUPDATED,

    #[graphql(name = "CUSTOMER_DELETED")]
    CUSTOMERDELETED,

    #[graphql(name = "CUSTOMER_METADATA_UPDATED")]
    CUSTOMERMETADATAUPDATED,

    #[graphql(name = "CUSTOMER_TYPE_CREATED")]
    CUSTOMERTYPECREATED,

    #[graphql(name = "CUSTOMER_TYPE_UPDATED")]
    CUSTOMERTYPEUPDATED,

    #[graphql(name = "CUSTOMER_TYPE_DELETED")]
    CUSTOMERTYPEDELETED,

    #[graphql(name = "COLLECTION_CREATED")]
    COLLECTIONCREATED,

    #[graphql(name = "COLLECTION_UPDATED")]
    COLLECTIONUPDATED,

    #[graphql(name = "COLLECTION_DELETED")]
    COLLECTIONDELETED,

    #[graphql(name = "COLLECTION_METADATA_UPDATED")]
    COLLECTIONMETADATAUPDATED,

    #[graphql(name = "PRODUCT_CREATED")]
    PRODUCTCREATED,

    #[graphql(name = "PRODUCT_UPDATED")]
    PRODUCTUPDATED,

    #[graphql(name = "PRODUCT_DELETED")]
    PRODUCTDELETED,

    #[graphql(name = "PRODUCT_METADATA_UPDATED")]
    PRODUCTMETADATAUPDATED,

    #[graphql(name = "PRODUCT_EXPORT_COMPLETED")]
    PRODUCTEXPORTCOMPLETED,

    #[graphql(name = "PRODUCT_MEDIA_CREATED")]
    PRODUCTMEDIACREATED,

    #[graphql(name = "PRODUCT_MEDIA_UPDATED")]
    PRODUCTMEDIAUPDATED,

    #[graphql(name = "PRODUCT_MEDIA_DELETED")]
    PRODUCTMEDIADELETED,

    #[graphql(name = "PRODUCT_VARIANT_CREATED")]
    PRODUCTVARIANTCREATED,

    #[graphql(name = "PRODUCT_VARIANT_UPDATED")]
    PRODUCTVARIANTUPDATED,

    #[graphql(name = "PRODUCT_VARIANT_DELETED")]
    PRODUCTVARIANTDELETED,

    #[graphql(name = "PRODUCT_VARIANT_METADATA_UPDATED")]
    PRODUCTVARIANTMETADATAUPDATED,

    #[graphql(name = "PRODUCT_VARIANT_OUT_OF_STOCK")]
    PRODUCTVARIANTOUTOFSTOCK,

    #[graphql(name = "PRODUCT_VARIANT_BACK_IN_STOCK")]
    PRODUCTVARIANTBACKINSTOCK,

    #[graphql(name = "PRODUCT_VARIANT_STOCK_UPDATED")]
    PRODUCTVARIANTSTOCKUPDATED,

    #[graphql(name = "PRODUCT_VARIANT_OUT_OF_STOCK_IN_CHANNEL")]
    PRODUCTVARIANTOUTOFSTOCKINCHANNEL,

    #[graphql(name = "PRODUCT_VARIANT_BACK_IN_STOCK_IN_CHANNEL")]
    PRODUCTVARIANTBACKINSTOCKINCHANNEL,

    #[graphql(name = "PRODUCT_VARIANT_OUT_OF_STOCK_FOR_CLICK_AND_COLLECT")]
    PRODUCTVARIANTOUTOFSTOCKFORCLICKANDCOLLECT,

    #[graphql(name = "PRODUCT_VARIANT_BACK_IN_STOCK_FOR_CLICK_AND_COLLECT")]
    PRODUCTVARIANTBACKINSTOCKFORCLICKANDCOLLECT,

    #[graphql(name = "PRODUCT_VARIANT_DISCOUNTED_PRICE_UPDATED")]
    PRODUCTVARIANTDISCOUNTEDPRICEUPDATED,

    #[graphql(name = "CHECKOUT_CREATED")]
    CHECKOUTCREATED,

    #[graphql(name = "CHECKOUT_UPDATED")]
    CHECKOUTUPDATED,

    #[graphql(name = "CHECKOUT_FULLY_AUTHORIZED")]
    CHECKOUTFULLYAUTHORIZED,

    #[graphql(name = "CHECKOUT_FULLY_PAID")]
    CHECKOUTFULLYPAID,

    #[graphql(name = "CHECKOUT_METADATA_UPDATED")]
    CHECKOUTMETADATAUPDATED,

    #[graphql(name = "NOTIFY_USER")]
    NOTIFYUSER,

    #[graphql(name = "PAGE_CREATED")]
    PAGECREATED,

    #[graphql(name = "PAGE_UPDATED")]
    PAGEUPDATED,

    #[graphql(name = "PAGE_DELETED")]
    PAGEDELETED,

    #[graphql(name = "PAGE_TYPE_CREATED")]
    PAGETYPECREATED,

    #[graphql(name = "PAGE_TYPE_UPDATED")]
    PAGETYPEUPDATED,

    #[graphql(name = "PAGE_TYPE_DELETED")]
    PAGETYPEDELETED,

    #[graphql(name = "PERMISSION_GROUP_CREATED")]
    PERMISSIONGROUPCREATED,

    #[graphql(name = "PERMISSION_GROUP_UPDATED")]
    PERMISSIONGROUPUPDATED,

    #[graphql(name = "PERMISSION_GROUP_DELETED")]
    PERMISSIONGROUPDELETED,

    #[graphql(name = "SHIPPING_PRICE_CREATED")]
    SHIPPINGPRICECREATED,

    #[graphql(name = "SHIPPING_PRICE_UPDATED")]
    SHIPPINGPRICEUPDATED,

    #[graphql(name = "SHIPPING_PRICE_DELETED")]
    SHIPPINGPRICEDELETED,

    #[graphql(name = "SHIPPING_ZONE_CREATED")]
    SHIPPINGZONECREATED,

    #[graphql(name = "SHIPPING_ZONE_UPDATED")]
    SHIPPINGZONEUPDATED,

    #[graphql(name = "SHIPPING_ZONE_DELETED")]
    SHIPPINGZONEDELETED,

    #[graphql(name = "SHIPPING_ZONE_METADATA_UPDATED")]
    SHIPPINGZONEMETADATAUPDATED,

    #[graphql(name = "STAFF_CREATED")]
    STAFFCREATED,

    #[graphql(name = "STAFF_UPDATED")]
    STAFFUPDATED,

    #[graphql(name = "STAFF_DELETED")]
    STAFFDELETED,

    #[graphql(name = "STAFF_SET_PASSWORD_REQUESTED")]
    STAFFSETPASSWORDREQUESTED,

    #[graphql(name = "TRANSACTION_ITEM_METADATA_UPDATED")]
    TRANSACTIONITEMMETADATAUPDATED,

    #[graphql(name = "TRANSLATION_CREATED")]
    TRANSLATIONCREATED,

    #[graphql(name = "TRANSLATION_UPDATED")]
    TRANSLATIONUPDATED,

    #[graphql(name = "WAREHOUSE_CREATED")]
    WAREHOUSECREATED,

    #[graphql(name = "WAREHOUSE_UPDATED")]
    WAREHOUSEUPDATED,

    #[graphql(name = "WAREHOUSE_DELETED")]
    WAREHOUSEDELETED,

    #[graphql(name = "WAREHOUSE_METADATA_UPDATED")]
    WAREHOUSEMETADATAUPDATED,

    #[graphql(name = "VOUCHER_CREATED")]
    VOUCHERCREATED,

    #[graphql(name = "VOUCHER_UPDATED")]
    VOUCHERUPDATED,

    #[graphql(name = "VOUCHER_DELETED")]
    VOUCHERDELETED,

    #[graphql(name = "VOUCHER_CODES_CREATED")]
    VOUCHERCODESCREATED,

    #[graphql(name = "VOUCHER_CODES_DELETED")]
    VOUCHERCODESDELETED,

    #[graphql(name = "VOUCHER_METADATA_UPDATED")]
    VOUCHERMETADATAUPDATED,

    #[graphql(name = "VOUCHER_CODE_EXPORT_COMPLETED")]
    VOUCHERCODEEXPORTCOMPLETED,

    #[graphql(name = "OBSERVABILITY")]
    OBSERVABILITY,

    #[graphql(name = "THUMBNAIL_CREATED")]
    THUMBNAILCREATED,

    #[graphql(name = "SHOP_METADATA_UPDATED")]
    SHOPMETADATAUPDATED,

    #[graphql(name = "PAYMENT_LIST_GATEWAYS")]
    PAYMENTLISTGATEWAYS,

    #[graphql(name = "PAYMENT_AUTHORIZE")]
    PAYMENTAUTHORIZE,

    #[graphql(name = "PAYMENT_CAPTURE")]
    PAYMENTCAPTURE,

    #[graphql(name = "PAYMENT_REFUND")]
    PAYMENTREFUND,

    #[graphql(name = "PAYMENT_VOID")]
    PAYMENTVOID,

    #[graphql(name = "PAYMENT_CONFIRM")]
    PAYMENTCONFIRM,

    #[graphql(name = "PAYMENT_PROCESS")]
    PAYMENTPROCESS,

    #[graphql(name = "CHECKOUT_CALCULATE_TAXES")]
    CHECKOUTCALCULATETAXES,

    #[graphql(name = "ORDER_CALCULATE_TAXES")]
    ORDERCALCULATETAXES,

    #[graphql(name = "TRANSACTION_CHARGE_REQUESTED")]
    TRANSACTIONCHARGEREQUESTED,

    #[graphql(name = "TRANSACTION_REFUND_REQUESTED")]
    TRANSACTIONREFUNDREQUESTED,

    #[graphql(name = "TRANSACTION_CANCELATION_REQUESTED")]
    TRANSACTIONCANCELATIONREQUESTED,

    #[graphql(name = "SHIPPING_LIST_METHODS_FOR_CHECKOUT")]
    SHIPPINGLISTMETHODSFORCHECKOUT,

    #[graphql(name = "CHECKOUT_FILTER_SHIPPING_METHODS")]
    CHECKOUTFILTERSHIPPINGMETHODS,

    #[graphql(name = "ORDER_FILTER_SHIPPING_METHODS")]
    ORDERFILTERSHIPPINGMETHODS,

    #[graphql(name = "PAYMENT_GATEWAY_INITIALIZE_SESSION")]
    PAYMENTGATEWAYINITIALIZESESSION,

    #[graphql(name = "TRANSACTION_INITIALIZE_SESSION")]
    TRANSACTIONINITIALIZESESSION,

    #[graphql(name = "TRANSACTION_PROCESS_SESSION")]
    TRANSACTIONPROCESSSESSION,

    #[graphql(name = "LIST_STORED_PAYMENT_METHODS")]
    LISTSTOREDPAYMENTMETHODS,

    #[graphql(name = "STORED_PAYMENT_METHOD_DELETE_REQUESTED")]
    STOREDPAYMENTMETHODDELETEREQUESTED,

    #[graphql(name = "PAYMENT_GATEWAY_INITIALIZE_TOKENIZATION_SESSION")]
    PAYMENTGATEWAYINITIALIZETOKENIZATIONSESSION,

    #[graphql(name = "PAYMENT_METHOD_INITIALIZE_TOKENIZATION_SESSION")]
    PAYMENTMETHODINITIALIZETOKENIZATIONSESSION,

    #[graphql(name = "PAYMENT_METHOD_PROCESS_TOKENIZATION_SESSION")]
    PAYMENTMETHODPROCESSTOKENIZATIONSESSION,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WebhookEventTypeSyncEnum {

    #[graphql(name = "PAYMENT_LIST_GATEWAYS")]
    PAYMENTLISTGATEWAYS,

    #[graphql(name = "PAYMENT_AUTHORIZE")]
    PAYMENTAUTHORIZE,

    #[graphql(name = "PAYMENT_CAPTURE")]
    PAYMENTCAPTURE,

    #[graphql(name = "PAYMENT_REFUND")]
    PAYMENTREFUND,

    #[graphql(name = "PAYMENT_VOID")]
    PAYMENTVOID,

    #[graphql(name = "PAYMENT_CONFIRM")]
    PAYMENTCONFIRM,

    #[graphql(name = "PAYMENT_PROCESS")]
    PAYMENTPROCESS,

    #[graphql(name = "CHECKOUT_CALCULATE_TAXES")]
    CHECKOUTCALCULATETAXES,

    #[graphql(name = "ORDER_CALCULATE_TAXES")]
    ORDERCALCULATETAXES,

    #[graphql(name = "TRANSACTION_CHARGE_REQUESTED")]
    TRANSACTIONCHARGEREQUESTED,

    #[graphql(name = "TRANSACTION_REFUND_REQUESTED")]
    TRANSACTIONREFUNDREQUESTED,

    #[graphql(name = "TRANSACTION_CANCELATION_REQUESTED")]
    TRANSACTIONCANCELATIONREQUESTED,

    #[graphql(name = "SHIPPING_LIST_METHODS_FOR_CHECKOUT")]
    SHIPPINGLISTMETHODSFORCHECKOUT,

    #[graphql(name = "CHECKOUT_FILTER_SHIPPING_METHODS")]
    CHECKOUTFILTERSHIPPINGMETHODS,

    #[graphql(name = "ORDER_FILTER_SHIPPING_METHODS")]
    ORDERFILTERSHIPPINGMETHODS,

    #[graphql(name = "PAYMENT_GATEWAY_INITIALIZE_SESSION")]
    PAYMENTGATEWAYINITIALIZESESSION,

    #[graphql(name = "TRANSACTION_INITIALIZE_SESSION")]
    TRANSACTIONINITIALIZESESSION,

    #[graphql(name = "TRANSACTION_PROCESS_SESSION")]
    TRANSACTIONPROCESSSESSION,

    #[graphql(name = "LIST_STORED_PAYMENT_METHODS")]
    LISTSTOREDPAYMENTMETHODS,

    #[graphql(name = "STORED_PAYMENT_METHOD_DELETE_REQUESTED")]
    STOREDPAYMENTMETHODDELETEREQUESTED,

    #[graphql(name = "PAYMENT_GATEWAY_INITIALIZE_TOKENIZATION_SESSION")]
    PAYMENTGATEWAYINITIALIZETOKENIZATIONSESSION,

    #[graphql(name = "PAYMENT_METHOD_INITIALIZE_TOKENIZATION_SESSION")]
    PAYMENTMETHODINITIALIZETOKENIZATIONSESSION,

    #[graphql(name = "PAYMENT_METHOD_PROCESS_TOKENIZATION_SESSION")]
    PAYMENTMETHODPROCESSTOKENIZATIONSESSION,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WebhookSampleEventTypeEnum {

    #[graphql(name = "ACCOUNT_CONFIRMATION_REQUESTED")]
    ACCOUNTCONFIRMATIONREQUESTED,

    #[graphql(name = "ACCOUNT_CHANGE_EMAIL_REQUESTED")]
    ACCOUNTCHANGEEMAILREQUESTED,

    #[graphql(name = "ACCOUNT_EMAIL_CHANGED")]
    ACCOUNTEMAILCHANGED,

    #[graphql(name = "ACCOUNT_SET_PASSWORD_REQUESTED")]
    ACCOUNTSETPASSWORDREQUESTED,

    #[graphql(name = "ACCOUNT_CONFIRMED")]
    ACCOUNTCONFIRMED,

    #[graphql(name = "ACCOUNT_DELETE_REQUESTED")]
    ACCOUNTDELETEREQUESTED,

    #[graphql(name = "ACCOUNT_DELETED")]
    ACCOUNTDELETED,

    #[graphql(name = "ADDRESS_CREATED")]
    ADDRESSCREATED,

    #[graphql(name = "ADDRESS_UPDATED")]
    ADDRESSUPDATED,

    #[graphql(name = "ADDRESS_DELETED")]
    ADDRESSDELETED,

    #[graphql(name = "APP_INSTALLED")]
    APPINSTALLED,

    #[graphql(name = "APP_UPDATED")]
    APPUPDATED,

    #[graphql(name = "APP_DELETED")]
    APPDELETED,

    #[graphql(name = "APP_STATUS_CHANGED")]
    APPSTATUSCHANGED,

    #[graphql(name = "ATTRIBUTE_CREATED")]
    ATTRIBUTECREATED,

    #[graphql(name = "ATTRIBUTE_UPDATED")]
    ATTRIBUTEUPDATED,

    #[graphql(name = "ATTRIBUTE_DELETED")]
    ATTRIBUTEDELETED,

    #[graphql(name = "ATTRIBUTE_VALUE_CREATED")]
    ATTRIBUTEVALUECREATED,

    #[graphql(name = "ATTRIBUTE_VALUE_UPDATED")]
    ATTRIBUTEVALUEUPDATED,

    #[graphql(name = "ATTRIBUTE_VALUE_DELETED")]
    ATTRIBUTEVALUEDELETED,

    #[graphql(name = "CATEGORY_CREATED")]
    CATEGORYCREATED,

    #[graphql(name = "CATEGORY_UPDATED")]
    CATEGORYUPDATED,

    #[graphql(name = "CATEGORY_DELETED")]
    CATEGORYDELETED,

    #[graphql(name = "CHANNEL_CREATED")]
    CHANNELCREATED,

    #[graphql(name = "CHANNEL_UPDATED")]
    CHANNELUPDATED,

    #[graphql(name = "CHANNEL_DELETED")]
    CHANNELDELETED,

    #[graphql(name = "CHANNEL_STATUS_CHANGED")]
    CHANNELSTATUSCHANGED,

    #[graphql(name = "CHANNEL_METADATA_UPDATED")]
    CHANNELMETADATAUPDATED,

    #[graphql(name = "GIFT_CARD_CREATED")]
    GIFTCARDCREATED,

    #[graphql(name = "GIFT_CARD_UPDATED")]
    GIFTCARDUPDATED,

    #[graphql(name = "GIFT_CARD_DELETED")]
    GIFTCARDDELETED,

    #[graphql(name = "GIFT_CARD_SENT")]
    GIFTCARDSENT,

    #[graphql(name = "GIFT_CARD_STATUS_CHANGED")]
    GIFTCARDSTATUSCHANGED,

    #[graphql(name = "GIFT_CARD_METADATA_UPDATED")]
    GIFTCARDMETADATAUPDATED,

    #[graphql(name = "GIFT_CARD_EXPORT_COMPLETED")]
    GIFTCARDEXPORTCOMPLETED,

    #[graphql(name = "MENU_CREATED")]
    MENUCREATED,

    #[graphql(name = "MENU_UPDATED")]
    MENUUPDATED,

    #[graphql(name = "MENU_DELETED")]
    MENUDELETED,

    #[graphql(name = "MENU_ITEM_CREATED")]
    MENUITEMCREATED,

    #[graphql(name = "MENU_ITEM_UPDATED")]
    MENUITEMUPDATED,

    #[graphql(name = "MENU_ITEM_DELETED")]
    MENUITEMDELETED,

    #[graphql(name = "ORDER_CREATED")]
    ORDERCREATED,

    #[graphql(name = "ORDER_CONFIRMED")]
    ORDERCONFIRMED,

    #[graphql(name = "ORDER_PAID")]
    ORDERPAID,

    #[graphql(name = "ORDER_FULLY_PAID")]
    ORDERFULLYPAID,

    #[graphql(name = "ORDER_REFUNDED")]
    ORDERREFUNDED,

    #[graphql(name = "ORDER_FULLY_REFUNDED")]
    ORDERFULLYREFUNDED,

    #[graphql(name = "ORDER_UPDATED")]
    ORDERUPDATED,

    #[graphql(name = "ORDER_CANCELLED")]
    ORDERCANCELLED,

    #[graphql(name = "ORDER_EXPIRED")]
    ORDEREXPIRED,

    #[graphql(name = "ORDER_FULFILLED")]
    ORDERFULFILLED,

    #[graphql(name = "ORDER_METADATA_UPDATED")]
    ORDERMETADATAUPDATED,

    #[graphql(name = "ORDER_BULK_CREATED")]
    ORDERBULKCREATED,

    #[graphql(name = "FULFILLMENT_CREATED")]
    FULFILLMENTCREATED,

    #[graphql(name = "FULFILLMENT_CANCELED")]
    FULFILLMENTCANCELED,

    #[graphql(name = "FULFILLMENT_APPROVED")]
    FULFILLMENTAPPROVED,

    #[graphql(name = "FULFILLMENT_METADATA_UPDATED")]
    FULFILLMENTMETADATAUPDATED,

    #[graphql(name = "FULFILLMENT_TRACKING_NUMBER_UPDATED")]
    FULFILLMENTTRACKINGNUMBERUPDATED,

    #[graphql(name = "DRAFT_ORDER_CREATED")]
    DRAFTORDERCREATED,

    #[graphql(name = "DRAFT_ORDER_UPDATED")]
    DRAFTORDERUPDATED,

    #[graphql(name = "DRAFT_ORDER_DELETED")]
    DRAFTORDERDELETED,

    #[graphql(name = "SALE_CREATED")]
    SALECREATED,

    #[graphql(name = "SALE_UPDATED")]
    SALEUPDATED,

    #[graphql(name = "SALE_DELETED")]
    SALEDELETED,

    #[graphql(name = "SALE_TOGGLE")]
    SALETOGGLE,

    #[graphql(name = "PROMOTION_CREATED")]
    PROMOTIONCREATED,

    #[graphql(name = "PROMOTION_UPDATED")]
    PROMOTIONUPDATED,

    #[graphql(name = "PROMOTION_DELETED")]
    PROMOTIONDELETED,

    #[graphql(name = "PROMOTION_STARTED")]
    PROMOTIONSTARTED,

    #[graphql(name = "PROMOTION_ENDED")]
    PROMOTIONENDED,

    #[graphql(name = "PROMOTION_RULE_CREATED")]
    PROMOTIONRULECREATED,

    #[graphql(name = "PROMOTION_RULE_UPDATED")]
    PROMOTIONRULEUPDATED,

    #[graphql(name = "PROMOTION_RULE_DELETED")]
    PROMOTIONRULEDELETED,

    #[graphql(name = "INVOICE_REQUESTED")]
    INVOICEREQUESTED,

    #[graphql(name = "INVOICE_DELETED")]
    INVOICEDELETED,

    #[graphql(name = "INVOICE_SENT")]
    INVOICESENT,

    #[graphql(name = "CUSTOMER_CREATED")]
    CUSTOMERCREATED,

    #[graphql(name = "CUSTOMER_UPDATED")]
    CUSTOMERUPDATED,

    #[graphql(name = "CUSTOMER_DELETED")]
    CUSTOMERDELETED,

    #[graphql(name = "CUSTOMER_METADATA_UPDATED")]
    CUSTOMERMETADATAUPDATED,

    #[graphql(name = "CUSTOMER_TYPE_CREATED")]
    CUSTOMERTYPECREATED,

    #[graphql(name = "CUSTOMER_TYPE_UPDATED")]
    CUSTOMERTYPEUPDATED,

    #[graphql(name = "CUSTOMER_TYPE_DELETED")]
    CUSTOMERTYPEDELETED,

    #[graphql(name = "COLLECTION_CREATED")]
    COLLECTIONCREATED,

    #[graphql(name = "COLLECTION_UPDATED")]
    COLLECTIONUPDATED,

    #[graphql(name = "COLLECTION_DELETED")]
    COLLECTIONDELETED,

    #[graphql(name = "COLLECTION_METADATA_UPDATED")]
    COLLECTIONMETADATAUPDATED,

    #[graphql(name = "PRODUCT_CREATED")]
    PRODUCTCREATED,

    #[graphql(name = "PRODUCT_UPDATED")]
    PRODUCTUPDATED,

    #[graphql(name = "PRODUCT_DELETED")]
    PRODUCTDELETED,

    #[graphql(name = "PRODUCT_METADATA_UPDATED")]
    PRODUCTMETADATAUPDATED,

    #[graphql(name = "PRODUCT_EXPORT_COMPLETED")]
    PRODUCTEXPORTCOMPLETED,

    #[graphql(name = "PRODUCT_MEDIA_CREATED")]
    PRODUCTMEDIACREATED,

    #[graphql(name = "PRODUCT_MEDIA_UPDATED")]
    PRODUCTMEDIAUPDATED,

    #[graphql(name = "PRODUCT_MEDIA_DELETED")]
    PRODUCTMEDIADELETED,

    #[graphql(name = "PRODUCT_VARIANT_CREATED")]
    PRODUCTVARIANTCREATED,

    #[graphql(name = "PRODUCT_VARIANT_UPDATED")]
    PRODUCTVARIANTUPDATED,

    #[graphql(name = "PRODUCT_VARIANT_DELETED")]
    PRODUCTVARIANTDELETED,

    #[graphql(name = "PRODUCT_VARIANT_METADATA_UPDATED")]
    PRODUCTVARIANTMETADATAUPDATED,

    #[graphql(name = "PRODUCT_VARIANT_OUT_OF_STOCK")]
    PRODUCTVARIANTOUTOFSTOCK,

    #[graphql(name = "PRODUCT_VARIANT_BACK_IN_STOCK")]
    PRODUCTVARIANTBACKINSTOCK,

    #[graphql(name = "PRODUCT_VARIANT_STOCK_UPDATED")]
    PRODUCTVARIANTSTOCKUPDATED,

    #[graphql(name = "PRODUCT_VARIANT_OUT_OF_STOCK_IN_CHANNEL")]
    PRODUCTVARIANTOUTOFSTOCKINCHANNEL,

    #[graphql(name = "PRODUCT_VARIANT_BACK_IN_STOCK_IN_CHANNEL")]
    PRODUCTVARIANTBACKINSTOCKINCHANNEL,

    #[graphql(name = "PRODUCT_VARIANT_OUT_OF_STOCK_FOR_CLICK_AND_COLLECT")]
    PRODUCTVARIANTOUTOFSTOCKFORCLICKANDCOLLECT,

    #[graphql(name = "PRODUCT_VARIANT_BACK_IN_STOCK_FOR_CLICK_AND_COLLECT")]
    PRODUCTVARIANTBACKINSTOCKFORCLICKANDCOLLECT,

    #[graphql(name = "PRODUCT_VARIANT_DISCOUNTED_PRICE_UPDATED")]
    PRODUCTVARIANTDISCOUNTEDPRICEUPDATED,

    #[graphql(name = "CHECKOUT_CREATED")]
    CHECKOUTCREATED,

    #[graphql(name = "CHECKOUT_UPDATED")]
    CHECKOUTUPDATED,

    #[graphql(name = "CHECKOUT_FULLY_AUTHORIZED")]
    CHECKOUTFULLYAUTHORIZED,

    #[graphql(name = "CHECKOUT_FULLY_PAID")]
    CHECKOUTFULLYPAID,

    #[graphql(name = "CHECKOUT_METADATA_UPDATED")]
    CHECKOUTMETADATAUPDATED,

    #[graphql(name = "NOTIFY_USER")]
    NOTIFYUSER,

    #[graphql(name = "PAGE_CREATED")]
    PAGECREATED,

    #[graphql(name = "PAGE_UPDATED")]
    PAGEUPDATED,

    #[graphql(name = "PAGE_DELETED")]
    PAGEDELETED,

    #[graphql(name = "PAGE_TYPE_CREATED")]
    PAGETYPECREATED,

    #[graphql(name = "PAGE_TYPE_UPDATED")]
    PAGETYPEUPDATED,

    #[graphql(name = "PAGE_TYPE_DELETED")]
    PAGETYPEDELETED,

    #[graphql(name = "PERMISSION_GROUP_CREATED")]
    PERMISSIONGROUPCREATED,

    #[graphql(name = "PERMISSION_GROUP_UPDATED")]
    PERMISSIONGROUPUPDATED,

    #[graphql(name = "PERMISSION_GROUP_DELETED")]
    PERMISSIONGROUPDELETED,

    #[graphql(name = "SHIPPING_PRICE_CREATED")]
    SHIPPINGPRICECREATED,

    #[graphql(name = "SHIPPING_PRICE_UPDATED")]
    SHIPPINGPRICEUPDATED,

    #[graphql(name = "SHIPPING_PRICE_DELETED")]
    SHIPPINGPRICEDELETED,

    #[graphql(name = "SHIPPING_ZONE_CREATED")]
    SHIPPINGZONECREATED,

    #[graphql(name = "SHIPPING_ZONE_UPDATED")]
    SHIPPINGZONEUPDATED,

    #[graphql(name = "SHIPPING_ZONE_DELETED")]
    SHIPPINGZONEDELETED,

    #[graphql(name = "SHIPPING_ZONE_METADATA_UPDATED")]
    SHIPPINGZONEMETADATAUPDATED,

    #[graphql(name = "STAFF_CREATED")]
    STAFFCREATED,

    #[graphql(name = "STAFF_UPDATED")]
    STAFFUPDATED,

    #[graphql(name = "STAFF_DELETED")]
    STAFFDELETED,

    #[graphql(name = "STAFF_SET_PASSWORD_REQUESTED")]
    STAFFSETPASSWORDREQUESTED,

    #[graphql(name = "TRANSACTION_ITEM_METADATA_UPDATED")]
    TRANSACTIONITEMMETADATAUPDATED,

    #[graphql(name = "TRANSLATION_CREATED")]
    TRANSLATIONCREATED,

    #[graphql(name = "TRANSLATION_UPDATED")]
    TRANSLATIONUPDATED,

    #[graphql(name = "WAREHOUSE_CREATED")]
    WAREHOUSECREATED,

    #[graphql(name = "WAREHOUSE_UPDATED")]
    WAREHOUSEUPDATED,

    #[graphql(name = "WAREHOUSE_DELETED")]
    WAREHOUSEDELETED,

    #[graphql(name = "WAREHOUSE_METADATA_UPDATED")]
    WAREHOUSEMETADATAUPDATED,

    #[graphql(name = "VOUCHER_CREATED")]
    VOUCHERCREATED,

    #[graphql(name = "VOUCHER_UPDATED")]
    VOUCHERUPDATED,

    #[graphql(name = "VOUCHER_DELETED")]
    VOUCHERDELETED,

    #[graphql(name = "VOUCHER_CODES_CREATED")]
    VOUCHERCODESCREATED,

    #[graphql(name = "VOUCHER_CODES_DELETED")]
    VOUCHERCODESDELETED,

    #[graphql(name = "VOUCHER_METADATA_UPDATED")]
    VOUCHERMETADATAUPDATED,

    #[graphql(name = "VOUCHER_CODE_EXPORT_COMPLETED")]
    VOUCHERCODEEXPORTCOMPLETED,

    #[graphql(name = "OBSERVABILITY")]
    OBSERVABILITY,

    #[graphql(name = "THUMBNAIL_CREATED")]
    THUMBNAILCREATED,

    #[graphql(name = "SHOP_METADATA_UPDATED")]
    SHOPMETADATAUPDATED,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WebhookTriggerErrorCode {

    #[graphql(name = "GRAPHQL_ERROR")]
    GRAPHQLERROR,

    #[graphql(name = "NOT_FOUND")]
    NOTFOUND,

    #[graphql(name = "INVALID_ID")]
    INVALIDID,

    #[graphql(name = "MISSING_PERMISSION")]
    MISSINGPERMISSION,

    #[graphql(name = "TYPE_NOT_SUPPORTED")]
    TYPENOTSUPPORTED,

    #[graphql(name = "SYNTAX")]
    SYNTAX,

    #[graphql(name = "MISSING_SUBSCRIPTION")]
    MISSINGSUBSCRIPTION,

    #[graphql(name = "UNABLE_TO_PARSE")]
    UNABLETOPARSE,

    #[graphql(name = "MISSING_QUERY")]
    MISSINGQUERY,

    #[graphql(name = "MISSING_EVENT")]
    MISSINGEVENT,

}


#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeightUnitsEnum {

    #[graphql(name = "G")]
    G,

    #[graphql(name = "LB")]
    LB,

    #[graphql(name = "OZ")]
    OZ,

    #[graphql(name = "KG")]
    KG,

    #[graphql(name = "TONNE")]
    TONNE,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AccountInput")]
pub struct AccountInput {

    #[graphql(name = "firstName")]
    pub first_name: Option<String>,

    #[graphql(name = "lastName")]
    pub last_name: Option<String>,

    #[graphql(name = "languageCode")]
    pub language_code: Option<LanguageCodeEnum>,

    #[graphql(name = "defaultBillingAddress")]
    pub default_billing_address: Option<AddressInput>,

    #[graphql(name = "defaultShippingAddress")]
    pub default_shipping_address: Option<AddressInput>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AccountRegisterInput")]
pub struct AccountRegisterInput {

    #[graphql(name = "firstName")]
    pub first_name: Option<String>,

    #[graphql(name = "lastName")]
    pub last_name: Option<String>,

    #[graphql(name = "languageCode")]
    pub language_code: Option<LanguageCodeEnum>,

    #[graphql(name = "email")]
    pub email: String,

    #[graphql(name = "password")]
    pub password: String,

    #[graphql(name = "redirectUrl")]
    pub redirect_url: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AddressFilterInput")]
pub struct AddressFilterInput {

    #[graphql(name = "phoneNumber")]
    pub phone_number: Option<StringFilterInput>,

    #[graphql(name = "country")]
    pub country: Option<CountryCodeEnumFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AddressInput")]
pub struct AddressInput {

    #[graphql(name = "firstName")]
    pub first_name: Option<String>,

    #[graphql(name = "lastName")]
    pub last_name: Option<String>,

    #[graphql(name = "companyName")]
    pub company_name: Option<String>,

    #[graphql(name = "streetAddress1")]
    pub street_address1: Option<String>,

    #[graphql(name = "streetAddress2")]
    pub street_address2: Option<String>,

    #[graphql(name = "city")]
    pub city: Option<String>,

    #[graphql(name = "cityArea")]
    pub city_area: Option<String>,

    #[graphql(name = "postalCode")]
    pub postal_code: Option<String>,

    #[graphql(name = "country")]
    pub country: Option<CountryCode>,

    #[graphql(name = "countryArea")]
    pub country_area: Option<String>,

    #[graphql(name = "phone")]
    pub phone: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "skipValidation")]
    pub skip_validation: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppExtensionFilterInput")]
pub struct AppExtensionFilterInput {

    #[graphql(name = "mountName")]
    pub mount_name: Option<Vec<String>>,

    #[graphql(name = "targetName")]
    pub target_name: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppFilterInput")]
pub struct AppFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "type")]
    pub r#type: Option<crate::apps::AppTypeEnum>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppInput")]
pub struct AppInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "identifier")]
    pub identifier: Option<String>,

    #[graphql(name = "permissions")]
    pub permissions: Option<Vec<PermissionEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppInstallInput")]
pub struct AppInstallInput {

    #[graphql(name = "appName")]
    pub app_name: String,

    #[graphql(name = "manifestUrl")]
    pub manifest_url: String,

    #[graphql(name = "activateAfterInstallation")]
    pub activate_after_installation: Option<bool>,

    #[graphql(name = "permissions")]
    pub permissions: Option<Vec<PermissionEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppProblemCreateInput")]
pub struct AppProblemCreateInput {

    #[graphql(name = "message")]
    pub message: String,

    #[graphql(name = "key")]
    pub key: String,

    #[graphql(name = "criticalThreshold")]
    pub critical_threshold: Option<i32>,

    #[graphql(name = "aggregationPeriod")]
    pub aggregation_period: Option<i32>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppProblemDismissByAppInput")]
pub struct AppProblemDismissByAppInput {

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "keys")]
    pub keys: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppProblemDismissByStaffWithIdsInput")]
pub struct AppProblemDismissByStaffWithIdsInput {

    #[graphql(name = "ids")]
    pub ids: Vec<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppProblemDismissByStaffWithKeysInput")]
pub struct AppProblemDismissByStaffWithKeysInput {

    #[graphql(name = "keys")]
    pub keys: Vec<String>,

    #[graphql(name = "app")]
    pub app: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppProblemDismissInput")]
pub struct AppProblemDismissInput {

    #[graphql(name = "byApp")]
    pub by_app: Option<AppProblemDismissByAppInput>,

    #[graphql(name = "byStaffWithIds")]
    pub by_staff_with_ids: Option<AppProblemDismissByStaffWithIdsInput>,

    #[graphql(name = "byStaffWithKeys")]
    pub by_staff_with_keys: Option<AppProblemDismissByStaffWithKeysInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppSortingInput")]
pub struct AppSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: AppSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AppTokenInput")]
pub struct AppTokenInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "app")]
    pub app: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AssignedAttributeReferenceInput")]
pub struct AssignedAttributeReferenceInput {

    #[graphql(name = "referencedIds")]
    pub referenced_ids: Option<ContainsFilterInput>,

    #[graphql(name = "pageSlugs")]
    pub page_slugs: Option<ContainsFilterInput>,

    #[graphql(name = "productSlugs")]
    pub product_slugs: Option<ContainsFilterInput>,

    #[graphql(name = "productVariantSkus")]
    pub product_variant_skus: Option<ContainsFilterInput>,

    #[graphql(name = "categorySlugs")]
    pub category_slugs: Option<ContainsFilterInput>,

    #[graphql(name = "collectionSlugs")]
    pub collection_slugs: Option<ContainsFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AssignedAttributeValueInput")]
pub struct AssignedAttributeValueInput {

    #[graphql(name = "slug")]
    pub slug: Option<StringFilterInput>,

    #[graphql(name = "name")]
    pub name: Option<StringFilterInput>,

    #[graphql(name = "numeric")]
    pub numeric: Option<DecimalFilterInput>,

    #[graphql(name = "date")]
    pub date: Option<DateRangeInput>,

    #[graphql(name = "dateTime")]
    pub date_time: Option<DateTimeRangeInput>,

    #[graphql(name = "boolean")]
    pub boolean: Option<bool>,

    #[graphql(name = "reference")]
    pub reference: Option<AssignedAttributeReferenceInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AssignedAttributeWhereInput")]
pub struct AssignedAttributeWhereInput {

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "value")]
    pub value: Option<AssignedAttributeValueInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeBulkTranslateInput")]
pub struct AttributeBulkTranslateInput {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "languageCode")]
    pub language_code: LanguageCodeEnum,

    #[graphql(name = "translationFields")]
    pub translation_fields: NameTranslationInput,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeBulkUpdateInput")]
pub struct AttributeBulkUpdateInput {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "fields")]
    pub fields: AttributeUpdateInput,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeChoicesSortingInput")]
pub struct AttributeChoicesSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: AttributeChoicesSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeCreateInput")]
pub struct AttributeCreateInput {

    #[graphql(name = "inputType")]
    pub input_type: Option<AttributeInputTypeEnum>,

    #[graphql(name = "entityType")]
    pub entity_type: Option<AttributeEntityTypeEnum>,

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "type")]
    pub r#type: AttributeTypeEnum,

    #[graphql(name = "unit")]
    pub unit: Option<MeasurementUnitsEnum>,

    #[graphql(name = "values")]
    pub values: Option<Vec<AttributeValueCreateInput>>,

    #[graphql(name = "valueRequired")]
    pub value_required: Option<bool>,

    #[graphql(name = "isVariantOnly")]
    pub is_variant_only: Option<bool>,

    #[graphql(name = "visibleInStorefront")]
    pub visible_in_storefront: Option<bool>,

    #[graphql(name = "filterableInStorefront")]
    pub filterable_in_storefront: Option<bool>,

    #[graphql(name = "filterableInDashboard")]
    pub filterable_in_dashboard: Option<bool>,

    #[graphql(name = "storefrontSearchPosition")]
    pub storefront_search_position: Option<i32>,

    #[graphql(name = "availableInGrid")]
    pub available_in_grid: Option<bool>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "referenceTypes")]
    pub reference_types: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeEntityTypeEnumFilterInput")]
pub struct AttributeEntityTypeEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<AttributeEntityTypeEnum>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<AttributeEntityTypeEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeFilterInput")]
pub struct AttributeFilterInput {

    #[graphql(name = "valueRequired")]
    pub value_required: Option<bool>,

    #[graphql(name = "isVariantOnly")]
    pub is_variant_only: Option<bool>,

    #[graphql(name = "visibleInStorefront")]
    pub visible_in_storefront: Option<bool>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "type")]
    pub r#type: Option<AttributeTypeEnum>,

    #[graphql(name = "inCollection")]
    pub in_collection: Option<ID>,

    #[graphql(name = "inCategory")]
    pub in_category: Option<ID>,

    #[graphql(name = "slugs")]
    pub slugs: Option<Vec<String>>,

    #[graphql(name = "filterableInStorefront")]
    pub filterable_in_storefront: Option<bool>,

    #[graphql(name = "availableInGrid")]
    pub available_in_grid: Option<bool>,

    #[graphql(name = "filterableInDashboard")]
    pub filterable_in_dashboard: Option<bool>,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeInput")]
pub struct AttributeInput {

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "value")]
    pub value: Option<AssignedAttributeValueInput>,

    #[graphql(name = "values")]
    pub values: Option<Vec<String>>,

    #[graphql(name = "valuesRange")]
    pub values_range: Option<IntRangeInput>,

    #[graphql(name = "dateTime")]
    pub date_time: Option<DateTimeRangeInput>,

    #[graphql(name = "date")]
    pub date: Option<DateRangeInput>,

    #[graphql(name = "boolean")]
    pub boolean: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeInputTypeEnumFilterInput")]
pub struct AttributeInputTypeEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<AttributeInputTypeEnum>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<AttributeInputTypeEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeSortingInput")]
pub struct AttributeSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: AttributeSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeTypeEnumFilterInput")]
pub struct AttributeTypeEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<AttributeTypeEnum>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<AttributeTypeEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeUpdateInput")]
pub struct AttributeUpdateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "unit")]
    pub unit: Option<MeasurementUnitsEnum>,

    #[graphql(name = "removeValues")]
    pub remove_values: Option<Vec<ID>>,

    #[graphql(name = "addValues")]
    pub add_values: Option<Vec<AttributeValueUpdateInput>>,

    #[graphql(name = "valueRequired")]
    pub value_required: Option<bool>,

    #[graphql(name = "isVariantOnly")]
    pub is_variant_only: Option<bool>,

    #[graphql(name = "visibleInStorefront")]
    pub visible_in_storefront: Option<bool>,

    #[graphql(name = "filterableInStorefront")]
    pub filterable_in_storefront: Option<bool>,

    #[graphql(name = "filterableInDashboard")]
    pub filterable_in_dashboard: Option<bool>,

    #[graphql(name = "storefrontSearchPosition")]
    pub storefront_search_position: Option<i32>,

    #[graphql(name = "availableInGrid")]
    pub available_in_grid: Option<bool>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "referenceTypes")]
    pub reference_types: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeValueBulkTranslateInput")]
pub struct AttributeValueBulkTranslateInput {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "languageCode")]
    pub language_code: LanguageCodeEnum,

    #[graphql(name = "translationFields")]
    pub translation_fields: AttributeValueTranslationInput,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeValueCreateInput")]
pub struct AttributeValueCreateInput {

    #[graphql(name = "value")]
    pub value: Option<String>,

    #[graphql(name = "richText")]
    pub rich_text: Option<GenJSONString>,

    #[graphql(name = "plainText")]
    pub plain_text: Option<String>,

    #[graphql(name = "fileUrl")]
    pub file_url: Option<String>,

    #[graphql(name = "contentType")]
    pub content_type: Option<String>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "name")]
    pub name: String,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeValueFilterInput")]
pub struct AttributeValueFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "slugs")]
    pub slugs: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeValueInput")]
pub struct AttributeValueInput {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "values")]
    pub values: Option<Vec<String>>,

    #[graphql(name = "dropdown")]
    pub dropdown: Option<AttributeValueSelectableTypeInput>,

    #[graphql(name = "swatch")]
    pub swatch: Option<AttributeValueSelectableTypeInput>,

    #[graphql(name = "multiselect")]
    pub multiselect: Option<Vec<AttributeValueSelectableTypeInput>>,

    #[graphql(name = "numeric")]
    pub numeric: Option<String>,

    #[graphql(name = "file")]
    pub file: Option<String>,

    #[graphql(name = "contentType")]
    pub content_type: Option<String>,

    #[graphql(name = "reference")]
    pub reference: Option<ID>,

    #[graphql(name = "references")]
    pub references: Option<Vec<ID>>,

    #[graphql(name = "richText")]
    pub rich_text: Option<GenJSONString>,

    #[graphql(name = "plainText")]
    pub plain_text: Option<String>,

    #[graphql(name = "boolean")]
    pub boolean: Option<bool>,

    #[graphql(name = "date")]
    pub date: Option<DateTime<Utc>>,

    #[graphql(name = "dateTime")]
    pub date_time: Option<DateTime<Utc>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeValueSelectableTypeInput")]
pub struct AttributeValueSelectableTypeInput {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "value")]
    pub value: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeValueTranslationInput")]
pub struct AttributeValueTranslationInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "richText")]
    pub rich_text: Option<GenJSONString>,

    #[graphql(name = "plainText")]
    pub plain_text: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeValueUpdateInput")]
pub struct AttributeValueUpdateInput {

    #[graphql(name = "value")]
    pub value: Option<String>,

    #[graphql(name = "richText")]
    pub rich_text: Option<GenJSONString>,

    #[graphql(name = "plainText")]
    pub plain_text: Option<String>,

    #[graphql(name = "fileUrl")]
    pub file_url: Option<String>,

    #[graphql(name = "contentType")]
    pub content_type: Option<String>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeValueWhereInput")]
pub struct AttributeValueWhereInput {

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "name")]
    pub name: Option<StringFilterInput>,

    #[graphql(name = "slug")]
    pub slug: Option<StringFilterInput>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<AttributeValueWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<AttributeValueWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "AttributeWhereInput")]
pub struct AttributeWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "name")]
    pub name: Option<StringFilterInput>,

    #[graphql(name = "slug")]
    pub slug: Option<StringFilterInput>,

    #[graphql(name = "withChoices")]
    pub with_choices: Option<bool>,

    #[graphql(name = "inputType")]
    pub input_type: Option<AttributeInputTypeEnumFilterInput>,

    #[graphql(name = "entityType")]
    pub entity_type: Option<AttributeEntityTypeEnumFilterInput>,

    #[graphql(name = "type")]
    pub r#type: Option<AttributeTypeEnumFilterInput>,

    #[graphql(name = "unit")]
    pub unit: Option<MeasurementUnitsEnumFilterInput>,

    #[graphql(name = "inCollection")]
    pub in_collection: Option<ID>,

    #[graphql(name = "inCategory")]
    pub in_category: Option<ID>,

    #[graphql(name = "valueRequired")]
    pub value_required: Option<bool>,

    #[graphql(name = "visibleInStorefront")]
    pub visible_in_storefront: Option<bool>,

    #[graphql(name = "filterableInDashboard")]
    pub filterable_in_dashboard: Option<bool>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<AttributeWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<AttributeWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "BulkAttributeValueInput")]
pub struct BulkAttributeValueInput {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "values")]
    pub values: Option<Vec<String>>,

    #[graphql(name = "dropdown")]
    pub dropdown: Option<AttributeValueSelectableTypeInput>,

    #[graphql(name = "swatch")]
    pub swatch: Option<AttributeValueSelectableTypeInput>,

    #[graphql(name = "multiselect")]
    pub multiselect: Option<Vec<AttributeValueSelectableTypeInput>>,

    #[graphql(name = "numeric")]
    pub numeric: Option<String>,

    #[graphql(name = "file")]
    pub file: Option<String>,

    #[graphql(name = "contentType")]
    pub content_type: Option<String>,

    #[graphql(name = "reference")]
    pub reference: Option<ID>,

    #[graphql(name = "references")]
    pub references: Option<Vec<ID>>,

    #[graphql(name = "richText")]
    pub rich_text: Option<GenJSONString>,

    #[graphql(name = "plainText")]
    pub plain_text: Option<String>,

    #[graphql(name = "boolean")]
    pub boolean: Option<bool>,

    #[graphql(name = "date")]
    pub date: Option<DateTime<Utc>>,

    #[graphql(name = "dateTime")]
    pub date_time: Option<DateTime<Utc>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CardInput")]
pub struct CardInput {

    #[graphql(name = "code")]
    pub code: String,

    #[graphql(name = "cvc")]
    pub cvc: Option<String>,

    #[graphql(name = "money")]
    pub money: MoneyInput,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CardPaymentMethodDetailsInput")]
pub struct CardPaymentMethodDetailsInput {

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "brand")]
    pub brand: Option<String>,

    #[graphql(name = "firstDigits")]
    pub first_digits: Option<String>,

    #[graphql(name = "lastDigits")]
    pub last_digits: Option<String>,

    #[graphql(name = "expMonth")]
    pub exp_month: Option<i32>,

    #[graphql(name = "expYear")]
    pub exp_year: Option<i32>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CatalogueInput")]
pub struct CatalogueInput {

    #[graphql(name = "products")]
    pub products: Option<Vec<ID>>,

    #[graphql(name = "categories")]
    pub categories: Option<Vec<ID>>,

    #[graphql(name = "collections")]
    pub collections: Option<Vec<ID>>,

    #[graphql(name = "variants")]
    pub variants: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CataloguePredicateInput")]
pub struct CataloguePredicateInput {

    #[graphql(name = "variantPredicate")]
    pub variant_predicate: Option<ProductVariantWhereInput>,

    #[graphql(name = "productPredicate")]
    pub product_predicate: Option<ProductWhereInput>,

    #[graphql(name = "categoryPredicate")]
    pub category_predicate: Option<CategoryWhereInput>,

    #[graphql(name = "collectionPredicate")]
    pub collection_predicate: Option<CollectionWhereInput>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<CataloguePredicateInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<CataloguePredicateInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CategoryFilterInput")]
pub struct CategoryFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "slugs")]
    pub slugs: Option<Vec<String>>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CategoryInput")]
pub struct CategoryInput {

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "seo")]
    pub seo: Option<SeoInput>,

    #[graphql(name = "backgroundImage")]
    pub background_image: Option<GenUpload>,

    #[graphql(name = "backgroundImageAlt")]
    pub background_image_alt: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CategorySortingInput")]
pub struct CategorySortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

    #[graphql(name = "field")]
    pub field: CategorySortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CategoryWhereInput")]
pub struct CategoryWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<CategoryWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<CategoryWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ChannelCreateInput")]
pub struct ChannelCreateInput {

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "stockSettings")]
    pub stock_settings: Option<StockSettingsInput>,

    #[graphql(name = "addShippingZones")]
    pub add_shipping_zones: Option<Vec<ID>>,

    #[graphql(name = "addWarehouses")]
    pub add_warehouses: Option<Vec<ID>>,

    #[graphql(name = "orderSettings")]
    pub order_settings: Option<OrderSettingsInput>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "checkoutSettings")]
    pub checkout_settings: Option<CheckoutSettingsInput>,

    #[graphql(name = "paymentSettings")]
    pub payment_settings: Option<PaymentSettingsInput>,

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "slug")]
    pub slug: String,

    #[graphql(name = "currencyCode")]
    pub currency_code: String,

    #[graphql(name = "defaultCountry")]
    pub default_country: CountryCode,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ChannelDeleteInput")]
pub struct ChannelDeleteInput {

    #[graphql(name = "channelId")]
    pub channel_id: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ChannelListingUpdateInput")]
pub struct ChannelListingUpdateInput {

    #[graphql(name = "channelListing")]
    pub channel_listing: ID,

    #[graphql(name = "price")]
    pub price: Option<GenPositiveDecimal>,

    #[graphql(name = "costPrice")]
    pub cost_price: Option<GenPositiveDecimal>,

    #[graphql(name = "priorPrice")]
    pub prior_price: Option<GenPositiveDecimal>,

    #[graphql(name = "preorderThreshold")]
    pub preorder_threshold: Option<i32>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ChannelUpdateInput")]
pub struct ChannelUpdateInput {

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "stockSettings")]
    pub stock_settings: Option<StockSettingsInput>,

    #[graphql(name = "addShippingZones")]
    pub add_shipping_zones: Option<Vec<ID>>,

    #[graphql(name = "addWarehouses")]
    pub add_warehouses: Option<Vec<ID>>,

    #[graphql(name = "orderSettings")]
    pub order_settings: Option<OrderSettingsInput>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "checkoutSettings")]
    pub checkout_settings: Option<CheckoutSettingsInput>,

    #[graphql(name = "paymentSettings")]
    pub payment_settings: Option<PaymentSettingsInput>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "defaultCountry")]
    pub default_country: Option<CountryCode>,

    #[graphql(name = "removeShippingZones")]
    pub remove_shipping_zones: Option<Vec<ID>>,

    #[graphql(name = "removeWarehouses")]
    pub remove_warehouses: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CheckoutAddressValidationRules")]
pub struct CheckoutAddressValidationRules {

    #[graphql(name = "checkRequiredFields")]
    pub check_required_fields: Option<bool>,

    #[graphql(name = "checkFieldsFormat")]
    pub check_fields_format: Option<bool>,

    #[graphql(name = "enableFieldsNormalization")]
    pub enable_fields_normalization: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CheckoutAutoCompleteInput")]
pub struct CheckoutAutoCompleteInput {

    #[graphql(name = "enabled")]
    pub enabled: bool,

    #[graphql(name = "delay")]
    pub delay: Option<i32>,

    #[graphql(name = "cutOffDate")]
    pub cut_off_date: Option<DateTime<Utc>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CheckoutCreateInput")]
pub struct CheckoutCreateInput {

    #[graphql(name = "channel")]
    pub channel: Option<String>,

    #[graphql(name = "lines")]
    pub lines: Vec<CheckoutLineInput>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "saveShippingAddress")]
    pub save_shipping_address: Option<bool>,

    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<AddressInput>,

    #[graphql(name = "saveBillingAddress")]
    pub save_billing_address: Option<bool>,

    #[graphql(name = "billingAddress")]
    pub billing_address: Option<AddressInput>,

    #[graphql(name = "languageCode")]
    pub language_code: Option<LanguageCodeEnum>,

    #[graphql(name = "validationRules")]
    pub validation_rules: Option<CheckoutValidationRules>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CheckoutFilterInput")]
pub struct CheckoutFilterInput {

    #[graphql(name = "customer")]
    pub customer: Option<String>,

    #[graphql(name = "created")]
    pub created: Option<DateRangeInput>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "channels")]
    pub channels: Option<Vec<ID>>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateRangeInput>,

    #[graphql(name = "authorizeStatus")]
    pub authorize_status: Option<Vec<CheckoutAuthorizeStatusEnum>>,

    #[graphql(name = "chargeStatus")]
    pub charge_status: Option<Vec<CheckoutChargeStatusEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CheckoutLineInput")]
pub struct CheckoutLineInput {

    #[graphql(name = "quantity")]
    pub quantity: i32,

    #[graphql(name = "variantId")]
    pub variant_id: ID,

    #[graphql(name = "price")]
    pub price: Option<GenPositiveDecimal>,

    #[graphql(name = "priceOverrideReason")]
    pub price_override_reason: Option<String>,

    #[graphql(name = "forceNewLine")]
    pub force_new_line: Option<bool>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CheckoutLineUpdateInput")]
pub struct CheckoutLineUpdateInput {

    #[graphql(name = "variantId")]
    pub variant_id: Option<ID>,

    #[graphql(name = "quantity")]
    pub quantity: Option<i32>,

    #[graphql(name = "price")]
    pub price: Option<GenPositiveDecimal>,

    #[graphql(name = "priceOverrideReason")]
    pub price_override_reason: Option<String>,

    #[graphql(name = "lineId")]
    pub line_id: Option<ID>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CheckoutSettingsInput")]
pub struct CheckoutSettingsInput {

    #[graphql(name = "useLegacyErrorFlow")]
    pub use_legacy_error_flow: Option<bool>,

    #[graphql(name = "automaticallyCompleteFullyPaidCheckouts")]
    pub automatically_complete_fully_paid_checkouts: Option<bool>,

    #[graphql(name = "automaticCompletion")]
    pub automatic_completion: Option<CheckoutAutoCompleteInput>,

    #[graphql(name = "allowLegacyGiftCardUse")]
    pub allow_legacy_gift_card_use: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CheckoutSortingInput")]
pub struct CheckoutSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: CheckoutSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CheckoutValidationRules")]
pub struct CheckoutValidationRules {

    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<CheckoutAddressValidationRules>,

    #[graphql(name = "billingAddress")]
    pub billing_address: Option<CheckoutAddressValidationRules>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CollectionChannelListingUpdateInput")]
pub struct CollectionChannelListingUpdateInput {

    #[graphql(name = "addChannels")]
    pub add_channels: Option<Vec<PublishableChannelListingInput>>,

    #[graphql(name = "removeChannels")]
    pub remove_channels: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CollectionCreateInput")]
pub struct CollectionCreateInput {

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "backgroundImage")]
    pub background_image: Option<GenUpload>,

    #[graphql(name = "backgroundImageAlt")]
    pub background_image_alt: Option<String>,

    #[graphql(name = "seo")]
    pub seo: Option<SeoInput>,

    #[graphql(name = "publicationDate")]
    pub publication_date: Option<DateTime<Utc>>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "products")]
    pub products: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CollectionFilterInput")]
pub struct CollectionFilterInput {

    #[graphql(name = "published")]
    pub published: Option<CollectionPublished>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "slugs")]
    pub slugs: Option<Vec<String>>,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CollectionInput")]
pub struct CollectionInput {

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "backgroundImage")]
    pub background_image: Option<GenUpload>,

    #[graphql(name = "backgroundImageAlt")]
    pub background_image_alt: Option<String>,

    #[graphql(name = "seo")]
    pub seo: Option<SeoInput>,

    #[graphql(name = "publicationDate")]
    pub publication_date: Option<DateTime<Utc>>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CollectionSortingInput")]
pub struct CollectionSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

    #[graphql(name = "field")]
    pub field: CollectionSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CollectionWhereInput")]
pub struct CollectionWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<CollectionWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<CollectionWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ConfigurationItemInput")]
pub struct ConfigurationItemInput {

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "value")]
    pub value: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ContainsFilterInput")]
pub struct ContainsFilterInput {

    #[graphql(name = "containsAny")]
    pub contains_any: Option<Vec<String>>,

    #[graphql(name = "containsAll")]
    pub contains_all: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CountryCodeEnumFilterInput")]
pub struct CountryCodeEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<CountryCode>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<CountryCode>>,

    #[graphql(name = "notOneOf")]
    pub not_one_of: Option<Vec<CountryCode>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CountryFilterInput")]
pub struct CountryFilterInput {

    #[graphql(name = "attachedToShippingZones")]
    pub attached_to_shipping_zones: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CountryRateInput")]
pub struct CountryRateInput {

    #[graphql(name = "countryCode")]
    pub country_code: CountryCode,

    #[graphql(name = "rate")]
    pub rate: f64,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CountryRateUpdateInput")]
pub struct CountryRateUpdateInput {

    #[graphql(name = "countryCode")]
    pub country_code: CountryCode,

    #[graphql(name = "rate")]
    pub rate: Option<f64>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CustomerBulkUpdateInput")]
pub struct CustomerBulkUpdateInput {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "input")]
    pub input: CustomerInput,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CustomerFilterInput")]
pub struct CustomerFilterInput {

    #[graphql(name = "dateJoined")]
    pub date_joined: Option<DateRangeInput>,

    #[graphql(name = "numberOfOrders")]
    pub number_of_orders: Option<IntRangeInput>,

    #[graphql(name = "placedOrders")]
    pub placed_orders: Option<DateRangeInput>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CustomerInput")]
pub struct CustomerInput {

    #[graphql(name = "defaultBillingAddress")]
    pub default_billing_address: Option<AddressInput>,

    #[graphql(name = "defaultShippingAddress")]
    pub default_shipping_address: Option<AddressInput>,

    #[graphql(name = "firstName")]
    pub first_name: Option<String>,

    #[graphql(name = "lastName")]
    pub last_name: Option<String>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "note")]
    pub note: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "languageCode")]
    pub language_code: Option<LanguageCodeEnum>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "isConfirmed")]
    pub is_confirmed: Option<bool>,

    #[graphql(name = "customerType")]
    pub customer_type: Option<ID>,

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AttributeValueInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CustomerOrderWhereInput")]
pub struct CustomerOrderWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<MetadataFilterInput>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "number")]
    pub number: Option<IntFilterInput>,

    #[graphql(name = "channelId")]
    pub channel_id: Option<GlobalIDFilterInput>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTimeRangeInput>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

    #[graphql(name = "userEmail")]
    pub user_email: Option<StringFilterInput>,

    #[graphql(name = "authorizeStatus")]
    pub authorize_status: Option<OrderAuthorizeStatusEnumFilterInput>,

    #[graphql(name = "chargeStatus")]
    pub charge_status: Option<OrderChargeStatusEnumFilterInput>,

    #[graphql(name = "status")]
    pub status: Option<OrderStatusEnumFilterInput>,

    #[graphql(name = "checkoutToken")]
    pub checkout_token: Option<UUIDFilterInput>,

    #[graphql(name = "checkoutId")]
    pub checkout_id: Option<GlobalIDFilterInput>,

    #[graphql(name = "isClickAndCollect")]
    pub is_click_and_collect: Option<bool>,

    #[graphql(name = "isGiftCardUsed")]
    pub is_gift_card_used: Option<bool>,

    #[graphql(name = "isGiftCardBought")]
    pub is_gift_card_bought: Option<bool>,

    #[graphql(name = "voucherCode")]
    pub voucher_code: Option<StringFilterInput>,

    #[graphql(name = "hasInvoices")]
    pub has_invoices: Option<bool>,

    #[graphql(name = "invoices")]
    pub invoices: Option<Vec<InvoiceFilterInput>>,

    #[graphql(name = "hasFulfillments")]
    pub has_fulfillments: Option<bool>,

    #[graphql(name = "linesCount")]
    pub lines_count: Option<IntFilterInput>,

    #[graphql(name = "totalGross")]
    pub total_gross: Option<PriceFilterInput>,

    #[graphql(name = "totalNet")]
    pub total_net: Option<PriceFilterInput>,

    #[graphql(name = "productTypeId")]
    pub product_type_id: Option<GlobalIDFilterInput>,

    #[graphql(name = "billingAddress")]
    pub billing_address: Option<AddressFilterInput>,

    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<AddressFilterInput>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<CustomerOrderWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<CustomerOrderWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CustomerTypeCreateInput")]
pub struct CustomerTypeCreateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "isDefault")]
    pub is_default: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CustomerTypeSortingInput")]
pub struct CustomerTypeSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: CustomerTypeSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CustomerTypeUpdateInput")]
pub struct CustomerTypeUpdateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "isDefault")]
    pub is_default: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CustomerTypeWhereInput")]
pub struct CustomerTypeWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<MetadataFilterInput>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "name")]
    pub name: Option<StringFilterInput>,

    #[graphql(name = "slug")]
    pub slug: Option<StringFilterInput>,

    #[graphql(name = "isDefault")]
    pub is_default: Option<bool>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<CustomerTypeWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<CustomerTypeWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CustomerWhereInput")]
pub struct CustomerWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<MetadataFilterInput>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "email")]
    pub email: Option<StringFilterInput>,

    #[graphql(name = "firstName")]
    pub first_name: Option<StringFilterInput>,

    #[graphql(name = "lastName")]
    pub last_name: Option<StringFilterInput>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "dateJoined")]
    pub date_joined: Option<DateTimeRangeInput>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

    #[graphql(name = "placedOrdersAt")]
    pub placed_orders_at: Option<DateTimeRangeInput>,

    #[graphql(name = "addresses")]
    pub addresses: Option<AddressFilterInput>,

    #[graphql(name = "numberOfOrders")]
    pub number_of_orders: Option<IntFilterInput>,

    #[graphql(name = "customerType")]
    pub customer_type: Option<GlobalIDFilterInput>,

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AssignedAttributeWhereInput>>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<CustomerWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<CustomerWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "DateRangeInput")]
pub struct DateRangeInput {

    #[graphql(name = "gte")]
    pub gte: Option<DateTime<Utc>>,

    #[graphql(name = "lte")]
    pub lte: Option<DateTime<Utc>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "DateTimeFilterInput")]
pub struct DateTimeFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<DateTime<Utc>>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<DateTime<Utc>>>,

    #[graphql(name = "range")]
    pub range: Option<DateTimeRangeInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "DateTimeRangeInput")]
pub struct DateTimeRangeInput {

    #[graphql(name = "gte")]
    pub gte: Option<DateTime<Utc>>,

    #[graphql(name = "lte")]
    pub lte: Option<DateTime<Utc>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "DecimalFilterInput")]
pub struct DecimalFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<GenDecimal>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<GenDecimal>>,

    #[graphql(name = "range")]
    pub range: Option<DecimalRangeInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "DecimalRangeInput")]
pub struct DecimalRangeInput {

    #[graphql(name = "gte")]
    pub gte: Option<GenDecimal>,

    #[graphql(name = "lte")]
    pub lte: Option<GenDecimal>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "DiscountedObjectWhereInput")]
pub struct DiscountedObjectWhereInput {

    #[graphql(name = "baseSubtotalPrice")]
    pub base_subtotal_price: Option<DecimalFilterInput>,

    #[graphql(name = "baseTotalPrice")]
    pub base_total_price: Option<DecimalFilterInput>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<DiscountedObjectWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<DiscountedObjectWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "DraftOrderCreateInput")]
pub struct DraftOrderCreateInput {

    #[graphql(name = "billingAddress")]
    pub billing_address: Option<AddressInput>,

    #[graphql(name = "saveBillingAddress")]
    pub save_billing_address: Option<bool>,

    #[graphql(name = "user")]
    pub user: Option<ID>,

    #[graphql(name = "userEmail")]
    pub user_email: Option<String>,

    #[graphql(name = "discount")]
    pub discount: Option<GenPositiveDecimal>,

    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<AddressInput>,

    #[graphql(name = "saveShippingAddress")]
    pub save_shipping_address: Option<bool>,

    #[graphql(name = "shippingMethod")]
    pub shipping_method: Option<ID>,

    #[graphql(name = "voucher")]
    pub voucher: Option<ID>,

    #[graphql(name = "voucherCode")]
    pub voucher_code: Option<String>,

    #[graphql(name = "customerNote")]
    pub customer_note: Option<String>,

    #[graphql(name = "channelId")]
    pub channel_id: Option<ID>,

    #[graphql(name = "redirectUrl")]
    pub redirect_url: Option<String>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "languageCode")]
    pub language_code: Option<LanguageCodeEnum>,

    #[graphql(name = "lines")]
    pub lines: Option<Vec<OrderLineCreateInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "DraftOrderInput")]
pub struct DraftOrderInput {

    #[graphql(name = "billingAddress")]
    pub billing_address: Option<AddressInput>,

    #[graphql(name = "saveBillingAddress")]
    pub save_billing_address: Option<bool>,

    #[graphql(name = "user")]
    pub user: Option<ID>,

    #[graphql(name = "userEmail")]
    pub user_email: Option<String>,

    #[graphql(name = "discount")]
    pub discount: Option<GenPositiveDecimal>,

    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<AddressInput>,

    #[graphql(name = "saveShippingAddress")]
    pub save_shipping_address: Option<bool>,

    #[graphql(name = "shippingMethod")]
    pub shipping_method: Option<ID>,

    #[graphql(name = "voucher")]
    pub voucher: Option<ID>,

    #[graphql(name = "voucherCode")]
    pub voucher_code: Option<String>,

    #[graphql(name = "customerNote")]
    pub customer_note: Option<String>,

    #[graphql(name = "channelId")]
    pub channel_id: Option<ID>,

    #[graphql(name = "redirectUrl")]
    pub redirect_url: Option<String>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "languageCode")]
    pub language_code: Option<LanguageCodeEnum>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "DraftOrderWhereInput")]
pub struct DraftOrderWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<MetadataFilterInput>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "number")]
    pub number: Option<IntFilterInput>,

    #[graphql(name = "channelId")]
    pub channel_id: Option<GlobalIDFilterInput>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTimeRangeInput>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

    #[graphql(name = "user")]
    pub user: Option<GlobalIDFilterInput>,

    #[graphql(name = "userEmail")]
    pub user_email: Option<StringFilterInput>,

    #[graphql(name = "authorizeStatus")]
    pub authorize_status: Option<OrderAuthorizeStatusEnumFilterInput>,

    #[graphql(name = "chargeStatus")]
    pub charge_status: Option<OrderChargeStatusEnumFilterInput>,

    #[graphql(name = "isClickAndCollect")]
    pub is_click_and_collect: Option<bool>,

    #[graphql(name = "voucherCode")]
    pub voucher_code: Option<StringFilterInput>,

    #[graphql(name = "lines")]
    pub lines: Option<Vec<LinesFilterInput>>,

    #[graphql(name = "linesCount")]
    pub lines_count: Option<IntFilterInput>,

    #[graphql(name = "transactions")]
    pub transactions: Option<Vec<TransactionFilterInput>>,

    #[graphql(name = "totalGross")]
    pub total_gross: Option<PriceFilterInput>,

    #[graphql(name = "totalNet")]
    pub total_net: Option<PriceFilterInput>,

    #[graphql(name = "productTypeId")]
    pub product_type_id: Option<GlobalIDFilterInput>,

    #[graphql(name = "events")]
    pub events: Option<Vec<OrderEventFilterInput>>,

    #[graphql(name = "billingAddress")]
    pub billing_address: Option<AddressFilterInput>,

    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<AddressFilterInput>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<DraftOrderWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<DraftOrderWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "EventDeliveryAttemptSortingInput")]
pub struct EventDeliveryAttemptSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: EventDeliveryAttemptSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "EventDeliveryFilterInput")]
pub struct EventDeliveryFilterInput {

    #[graphql(name = "status")]
    pub status: Option<EventDeliveryStatusEnum>,

    #[graphql(name = "eventType")]
    pub event_type: Option<WebhookEventTypeEnum>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "EventDeliverySortingInput")]
pub struct EventDeliverySortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: EventDeliverySortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ExportFileFilterInput")]
pub struct ExportFileFilterInput {

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTimeRangeInput>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

    #[graphql(name = "status")]
    pub status: Option<JobStatusEnum>,

    #[graphql(name = "user")]
    pub user: Option<String>,

    #[graphql(name = "app")]
    pub app: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ExportFileSortingInput")]
pub struct ExportFileSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: ExportFileSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ExportGiftCardsInput")]
pub struct ExportGiftCardsInput {

    #[graphql(name = "scope")]
    pub scope: ExportScope,

    #[graphql(name = "filter")]
    pub filter: Option<GiftCardFilterInput>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "fileType")]
    pub file_type: FileTypesEnum,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ExportInfoInput")]
pub struct ExportInfoInput {

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<ID>>,

    #[graphql(name = "warehouses")]
    pub warehouses: Option<Vec<ID>>,

    #[graphql(name = "channels")]
    pub channels: Option<Vec<ID>>,

    #[graphql(name = "fields")]
    pub fields: Option<Vec<ProductFieldEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ExportProductsInput")]
pub struct ExportProductsInput {

    #[graphql(name = "scope")]
    pub scope: ExportScope,

    #[graphql(name = "filter")]
    pub filter: Option<ProductFilterInput>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "exportInfo")]
    pub export_info: Option<ExportInfoInput>,

    #[graphql(name = "fileType")]
    pub file_type: FileTypesEnum,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ExportVoucherCodesInput")]
pub struct ExportVoucherCodesInput {

    #[graphql(name = "voucherId")]
    pub voucher_id: Option<ID>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "fileType")]
    pub file_type: FileTypesEnum,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ExternalNotificationTriggerInput")]
pub struct ExternalNotificationTriggerInput {

    #[graphql(name = "ids")]
    pub ids: Vec<ID>,

    #[graphql(name = "extraPayload")]
    pub extra_payload: Option<GenJSONString>,

    #[graphql(name = "externalEventType")]
    pub external_event_type: String,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "FulfillmentCancelInput")]
pub struct FulfillmentCancelInput {

    #[graphql(name = "warehouseId")]
    pub warehouse_id: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "FulfillmentFilterInput")]
pub struct FulfillmentFilterInput {

    #[graphql(name = "status")]
    pub status: Option<FulfillmentStatusEnumFilterInput>,

    #[graphql(name = "metadata")]
    pub metadata: Option<MetadataFilterInput>,

    #[graphql(name = "warehouse")]
    pub warehouse: Option<FulfillmentWarehouseFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "FulfillmentStatusEnumFilterInput")]
pub struct FulfillmentStatusEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<FulfillmentStatus>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<FulfillmentStatus>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "FulfillmentUpdateTrackingInput")]
pub struct FulfillmentUpdateTrackingInput {

    #[graphql(name = "trackingNumber")]
    pub tracking_number: Option<String>,

    #[graphql(name = "notifyCustomer")]
    pub notify_customer: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "FulfillmentWarehouseFilterInput")]
pub struct FulfillmentWarehouseFilterInput {

    #[graphql(name = "id")]
    pub id: Option<GlobalIDFilterInput>,

    #[graphql(name = "slug")]
    pub slug: Option<StringFilterInput>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<StringFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardAddNoteInput")]
pub struct GiftCardAddNoteInput {

    #[graphql(name = "message")]
    pub message: String,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardBulkCreateInput")]
pub struct GiftCardBulkCreateInput {

    #[graphql(name = "count")]
    pub count: i32,

    #[graphql(name = "balance")]
    pub balance: PriceInput,

    #[graphql(name = "tags")]
    pub tags: Option<Vec<String>>,

    #[graphql(name = "expiryDate")]
    pub expiry_date: Option<DateTime<Utc>>,

    #[graphql(name = "isActive")]
    pub is_active: bool,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardCreateInput")]
pub struct GiftCardCreateInput {

    #[graphql(name = "addTags")]
    pub add_tags: Option<Vec<String>>,

    #[graphql(name = "expiryDate")]
    pub expiry_date: Option<DateTime<Utc>>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "startDate")]
    pub start_date: Option<DateTime<Utc>>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTime<Utc>>,

    #[graphql(name = "balance")]
    pub balance: PriceInput,

    #[graphql(name = "userEmail")]
    pub user_email: Option<String>,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

    #[graphql(name = "isActive")]
    pub is_active: bool,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "note")]
    pub note: Option<String>,

    #[graphql(name = "assignedTo")]
    pub assigned_to: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardEventFilterInput")]
pub struct GiftCardEventFilterInput {

    #[graphql(name = "type")]
    pub r#type: Option<GiftCardEventsEnum>,

    #[graphql(name = "orders")]
    pub orders: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardFilterInput")]
pub struct GiftCardFilterInput {

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "tags")]
    pub tags: Option<Vec<String>>,

    #[graphql(name = "products")]
    pub products: Option<Vec<ID>>,

    #[graphql(name = "usedBy")]
    pub used_by: Option<Vec<ID>>,

    #[graphql(name = "assignedTo")]
    pub assigned_to: Option<Vec<ID>>,

    #[graphql(name = "used")]
    pub used: Option<bool>,

    #[graphql(name = "currency")]
    pub currency: Option<String>,

    #[graphql(name = "currentBalance")]
    pub current_balance: Option<PriceRangeInput>,

    #[graphql(name = "initialBalance")]
    pub initial_balance: Option<PriceRangeInput>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "createdByEmail")]
    pub created_by_email: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardPaymentMethodDetailsInput")]
pub struct GiftCardPaymentMethodDetailsInput {

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "brand")]
    pub brand: Option<String>,

    #[graphql(name = "lastChars")]
    pub last_chars: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardResendInput")]
pub struct GiftCardResendInput {

    #[graphql(name = "id")]
    pub id: ID,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "channel")]
    pub channel: String,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardSettingsUpdateInput")]
pub struct GiftCardSettingsUpdateInput {

    #[graphql(name = "expiryType")]
    pub expiry_type: Option<GiftCardSettingsExpiryTypeEnum>,

    #[graphql(name = "expiryPeriod")]
    pub expiry_period: Option<TimePeriodInputType>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardSortingInput")]
pub struct GiftCardSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: GiftCardSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardTagFilterInput")]
pub struct GiftCardTagFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GiftCardUpdateInput")]
pub struct GiftCardUpdateInput {

    #[graphql(name = "addTags")]
    pub add_tags: Option<Vec<String>>,

    #[graphql(name = "expiryDate")]
    pub expiry_date: Option<DateTime<Utc>>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "startDate")]
    pub start_date: Option<DateTime<Utc>>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTime<Utc>>,

    #[graphql(name = "removeTags")]
    pub remove_tags: Option<Vec<String>>,

    #[graphql(name = "balanceAmount")]
    pub balance_amount: Option<GenPositiveDecimal>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "GlobalIDFilterInput")]
pub struct GlobalIDFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<ID>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "IntFilterInput")]
pub struct IntFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<i32>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<i32>>,

    #[graphql(name = "range")]
    pub range: Option<IntRangeInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "IntRangeInput")]
pub struct IntRangeInput {

    #[graphql(name = "gte")]
    pub gte: Option<i32>,

    #[graphql(name = "lte")]
    pub lte: Option<i32>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "InvoiceCreateInput")]
pub struct InvoiceCreateInput {

    #[graphql(name = "number")]
    pub number: String,

    #[graphql(name = "url")]
    pub url: String,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "InvoiceFilterInput")]
pub struct InvoiceFilterInput {

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTimeRangeInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "LinesFilterInput")]
pub struct LinesFilterInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<MetadataFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MeasurementUnitsEnumFilterInput")]
pub struct MeasurementUnitsEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<MeasurementUnitsEnum>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<MeasurementUnitsEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MediaInput")]
pub struct MediaInput {

    #[graphql(name = "alt")]
    pub alt: Option<String>,

    #[graphql(name = "image")]
    pub image: Option<GenUpload>,

    #[graphql(name = "mediaUrl")]
    pub media_url: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MediaSortingInput")]
pub struct MediaSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: MediaChoicesSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MenuCreateInput")]
pub struct MenuCreateInput {

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "items")]
    pub items: Option<Vec<MenuItemInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MenuFilterInput")]
pub struct MenuFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<Vec<String>>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "slugs")]
    pub slugs: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MenuInput")]
pub struct MenuInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MenuItemCreateInput")]
pub struct MenuItemCreateInput {

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "url")]
    pub url: Option<String>,

    #[graphql(name = "category")]
    pub category: Option<ID>,

    #[graphql(name = "collection")]
    pub collection: Option<ID>,

    #[graphql(name = "page")]
    pub page: Option<ID>,

    #[graphql(name = "menu")]
    pub menu: ID,

    #[graphql(name = "parent")]
    pub parent: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MenuItemFilterInput")]
pub struct MenuItemFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MenuItemInput")]
pub struct MenuItemInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "url")]
    pub url: Option<String>,

    #[graphql(name = "category")]
    pub category: Option<ID>,

    #[graphql(name = "collection")]
    pub collection: Option<ID>,

    #[graphql(name = "page")]
    pub page: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MenuItemMoveInput")]
pub struct MenuItemMoveInput {

    #[graphql(name = "itemId")]
    pub item_id: ID,

    #[graphql(name = "parentId")]
    pub parent_id: Option<ID>,

    #[graphql(name = "sortOrder")]
    pub sort_order: Option<i32>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MenuItemSortingInput")]
pub struct MenuItemSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: MenuItemsSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MenuSortingInput")]
pub struct MenuSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: MenuSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MetadataFilter")]
pub struct MetadataFilter {

    #[graphql(name = "key")]
    pub key: String,

    #[graphql(name = "value")]
    pub value: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MetadataFilterInput")]
pub struct MetadataFilterInput {

    #[graphql(name = "key")]
    pub key: String,

    #[graphql(name = "value")]
    pub value: Option<MetadataValueFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MetadataValueFilterInput")]
pub struct MetadataValueFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<String>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MoneyInput")]
pub struct MoneyInput {

    #[graphql(name = "currency")]
    pub currency: String,

    #[graphql(name = "amount")]
    pub amount: GenPositiveDecimal,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "MoveProductInput")]
pub struct MoveProductInput {

    #[graphql(name = "productId")]
    pub product_id: ID,

    #[graphql(name = "sortOrder")]
    pub sort_order: Option<i32>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "NameTranslationInput")]
pub struct NameTranslationInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderAddNoteInput")]
pub struct OrderAddNoteInput {

    #[graphql(name = "message")]
    pub message: String,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderAuthorizeStatusEnumFilterInput")]
pub struct OrderAuthorizeStatusEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<OrderAuthorizeStatusEnum>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<OrderAuthorizeStatusEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderBulkCreateDeliveryMethodInput")]
pub struct OrderBulkCreateDeliveryMethodInput {

    #[graphql(name = "warehouseId")]
    pub warehouse_id: Option<ID>,

    #[graphql(name = "warehouseName")]
    pub warehouse_name: Option<String>,

    #[graphql(name = "shippingMethodId")]
    pub shipping_method_id: Option<ID>,

    #[graphql(name = "shippingMethodName")]
    pub shipping_method_name: Option<String>,

    #[graphql(name = "shippingPrice")]
    pub shipping_price: Option<TaxedMoneyInput>,

    #[graphql(name = "shippingTaxRate")]
    pub shipping_tax_rate: Option<GenPositiveDecimal>,

    #[graphql(name = "shippingTaxClassId")]
    pub shipping_tax_class_id: Option<ID>,

    #[graphql(name = "shippingTaxClassName")]
    pub shipping_tax_class_name: Option<String>,

    #[graphql(name = "shippingTaxClassMetadata")]
    pub shipping_tax_class_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "shippingTaxClassPrivateMetadata")]
    pub shipping_tax_class_private_metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderBulkCreateFulfillmentInput")]
pub struct OrderBulkCreateFulfillmentInput {

    #[graphql(name = "trackingCode")]
    pub tracking_code: Option<String>,

    #[graphql(name = "lines")]
    pub lines: Option<Vec<OrderBulkCreateFulfillmentLineInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderBulkCreateFulfillmentLineInput")]
pub struct OrderBulkCreateFulfillmentLineInput {

    #[graphql(name = "variantId")]
    pub variant_id: Option<ID>,

    #[graphql(name = "variantSku")]
    pub variant_sku: Option<String>,

    #[graphql(name = "variantExternalReference")]
    pub variant_external_reference: Option<String>,

    #[graphql(name = "quantity")]
    pub quantity: i32,

    #[graphql(name = "warehouse")]
    pub warehouse: ID,

    #[graphql(name = "orderLineIndex")]
    pub order_line_index: i32,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderBulkCreateInput")]
pub struct OrderBulkCreateInput {

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "channel")]
    pub channel: String,

    #[graphql(name = "createdAt")]
    pub created_at: DateTime<Utc>,

    #[graphql(name = "status")]
    pub status: Option<OrderStatus>,

    #[graphql(name = "user")]
    pub user: OrderBulkCreateUserInput,

    #[graphql(name = "billingAddress")]
    pub billing_address: AddressInput,

    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<AddressInput>,

    #[graphql(name = "currency")]
    pub currency: String,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "customerNote")]
    pub customer_note: Option<String>,

    #[graphql(name = "notes")]
    pub notes: Option<Vec<OrderBulkCreateNoteInput>>,

    #[graphql(name = "languageCode")]
    pub language_code: LanguageCodeEnum,

    #[graphql(name = "displayGrossPrices")]
    pub display_gross_prices: Option<bool>,

    #[graphql(name = "weight")]
    pub weight: Option<GenWeightScalar>,

    #[graphql(name = "redirectUrl")]
    pub redirect_url: Option<String>,

    #[graphql(name = "lines")]
    pub lines: Vec<OrderBulkCreateOrderLineInput>,

    #[graphql(name = "deliveryMethod")]
    pub delivery_method: Option<OrderBulkCreateDeliveryMethodInput>,

    #[graphql(name = "giftCards")]
    pub gift_cards: Option<Vec<String>>,

    #[graphql(name = "voucherCode")]
    pub voucher_code: Option<String>,

    #[graphql(name = "discounts")]
    pub discounts: Option<Vec<OrderDiscountCommonInput>>,

    #[graphql(name = "fulfillments")]
    pub fulfillments: Option<Vec<OrderBulkCreateFulfillmentInput>>,

    #[graphql(name = "transactions")]
    pub transactions: Option<Vec<TransactionCreateInput>>,

    #[graphql(name = "invoices")]
    pub invoices: Option<Vec<OrderBulkCreateInvoiceInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderBulkCreateInvoiceInput")]
pub struct OrderBulkCreateInvoiceInput {

    #[graphql(name = "createdAt")]
    pub created_at: DateTime<Utc>,

    #[graphql(name = "number")]
    pub number: Option<String>,

    #[graphql(name = "url")]
    pub url: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderBulkCreateNoteInput")]
pub struct OrderBulkCreateNoteInput {

    #[graphql(name = "message")]
    pub message: String,

    #[graphql(name = "date")]
    pub date: Option<DateTime<Utc>>,

    #[graphql(name = "userId")]
    pub user_id: Option<ID>,

    #[graphql(name = "userEmail")]
    pub user_email: Option<ID>,

    #[graphql(name = "userExternalReference")]
    pub user_external_reference: Option<ID>,

    #[graphql(name = "appId")]
    pub app_id: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderBulkCreateOrderLineInput")]
pub struct OrderBulkCreateOrderLineInput {

    #[graphql(name = "variantId")]
    pub variant_id: Option<ID>,

    #[graphql(name = "variantSku")]
    pub variant_sku: Option<String>,

    #[graphql(name = "variantExternalReference")]
    pub variant_external_reference: Option<String>,

    #[graphql(name = "variantName")]
    pub variant_name: Option<String>,

    #[graphql(name = "productName")]
    pub product_name: Option<String>,

    #[graphql(name = "productSku")]
    pub product_sku: Option<String>,

    #[graphql(name = "translatedVariantName")]
    pub translated_variant_name: Option<String>,

    #[graphql(name = "translatedProductName")]
    pub translated_product_name: Option<String>,

    #[graphql(name = "createdAt")]
    pub created_at: DateTime<Utc>,

    #[graphql(name = "isShippingRequired")]
    pub is_shipping_required: bool,

    #[graphql(name = "isGiftCard")]
    pub is_gift_card: bool,

    #[graphql(name = "quantity")]
    pub quantity: i32,

    #[graphql(name = "totalPrice")]
    pub total_price: TaxedMoneyInput,

    #[graphql(name = "undiscountedTotalPrice")]
    pub undiscounted_total_price: TaxedMoneyInput,

    #[graphql(name = "unitDiscountReason")]
    pub unit_discount_reason: Option<String>,

    #[graphql(name = "unitDiscountType")]
    pub unit_discount_type: Option<DiscountValueTypeEnum>,

    #[graphql(name = "unitDiscountValue")]
    pub unit_discount_value: Option<GenPositiveDecimal>,

    #[graphql(name = "warehouse")]
    pub warehouse: ID,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "taxRate")]
    pub tax_rate: Option<GenPositiveDecimal>,

    #[graphql(name = "taxClassId")]
    pub tax_class_id: Option<ID>,

    #[graphql(name = "taxClassName")]
    pub tax_class_name: Option<String>,

    #[graphql(name = "taxClassMetadata")]
    pub tax_class_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "taxClassPrivateMetadata")]
    pub tax_class_private_metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderBulkCreateUserInput")]
pub struct OrderBulkCreateUserInput {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderChargeStatusEnumFilterInput")]
pub struct OrderChargeStatusEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<OrderChargeStatusEnum>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<OrderChargeStatusEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderDiscountCommonInput")]
pub struct OrderDiscountCommonInput {

    #[graphql(name = "valueType")]
    pub value_type: DiscountValueTypeEnum,

    #[graphql(name = "value")]
    pub value: GenPositiveDecimal,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderDraftFilterInput")]
pub struct OrderDraftFilterInput {

    #[graphql(name = "customer")]
    pub customer: Option<String>,

    #[graphql(name = "created")]
    pub created: Option<DateRangeInput>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "channels")]
    pub channels: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderEventFilterInput")]
pub struct OrderEventFilterInput {

    #[graphql(name = "date")]
    pub date: Option<DateTimeRangeInput>,

    #[graphql(name = "type")]
    pub r#type: Option<OrderEventTypeEnumFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderEventTypeEnumFilterInput")]
pub struct OrderEventTypeEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<OrderEventsEnum>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<OrderEventsEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderFilterInput")]
pub struct OrderFilterInput {

    #[graphql(name = "paymentStatus")]
    pub payment_status: Option<Vec<PaymentChargeStatusEnum>>,

    #[graphql(name = "status")]
    pub status: Option<Vec<OrderStatusFilter>>,

    #[graphql(name = "customer")]
    pub customer: Option<String>,

    #[graphql(name = "created")]
    pub created: Option<DateRangeInput>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "channels")]
    pub channels: Option<Vec<ID>>,

    #[graphql(name = "authorizeStatus")]
    pub authorize_status: Option<Vec<OrderAuthorizeStatusEnum>>,

    #[graphql(name = "chargeStatus")]
    pub charge_status: Option<Vec<OrderChargeStatusEnum>>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

    #[graphql(name = "isClickAndCollect")]
    pub is_click_and_collect: Option<bool>,

    #[graphql(name = "isPreorder")]
    pub is_preorder: Option<bool>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "checkoutTokens")]
    pub checkout_tokens: Option<Vec<String>>,

    #[graphql(name = "giftCardUsed")]
    pub gift_card_used: Option<bool>,

    #[graphql(name = "giftCardBought")]
    pub gift_card_bought: Option<bool>,

    #[graphql(name = "numbers")]
    pub numbers: Option<Vec<String>>,

    #[graphql(name = "checkoutIds")]
    pub checkout_ids: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderFulfillInput")]
pub struct OrderFulfillInput {

    #[graphql(name = "lines")]
    pub lines: Vec<OrderFulfillLineInput>,

    #[graphql(name = "notifyCustomer")]
    pub notify_customer: Option<bool>,

    #[graphql(name = "allowStockToBeExceeded")]
    pub allow_stock_to_be_exceeded: Option<bool>,

    #[graphql(name = "trackingNumber")]
    pub tracking_number: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderFulfillLineInput")]
pub struct OrderFulfillLineInput {

    #[graphql(name = "orderLineId")]
    pub order_line_id: Option<ID>,

    #[graphql(name = "stocks")]
    pub stocks: Vec<OrderFulfillStockInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderFulfillStockInput")]
pub struct OrderFulfillStockInput {

    #[graphql(name = "quantity")]
    pub quantity: i32,

    #[graphql(name = "warehouse")]
    pub warehouse: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderGrantRefundCreateInput")]
pub struct OrderGrantRefundCreateInput {

    #[graphql(name = "amount")]
    pub amount: Option<GenDecimal>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<ID>,

    #[graphql(name = "lines")]
    pub lines: Option<Vec<OrderGrantRefundCreateLineInput>>,

    #[graphql(name = "grantRefundForShipping")]
    pub grant_refund_for_shipping: Option<bool>,

    #[graphql(name = "transactionId")]
    pub transaction_id: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderGrantRefundCreateLineInput")]
pub struct OrderGrantRefundCreateLineInput {

    #[graphql(name = "id")]
    pub id: ID,

    #[graphql(name = "quantity")]
    pub quantity: i32,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderGrantRefundUpdateInput")]
pub struct OrderGrantRefundUpdateInput {

    #[graphql(name = "amount")]
    pub amount: Option<GenDecimal>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<ID>,

    #[graphql(name = "addLines")]
    pub add_lines: Option<Vec<OrderGrantRefundUpdateLineAddInput>>,

    #[graphql(name = "removeLines")]
    pub remove_lines: Option<Vec<ID>>,

    #[graphql(name = "grantRefundForShipping")]
    pub grant_refund_for_shipping: Option<bool>,

    #[graphql(name = "transactionId")]
    pub transaction_id: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderGrantRefundUpdateLineAddInput")]
pub struct OrderGrantRefundUpdateLineAddInput {

    #[graphql(name = "id")]
    pub id: ID,

    #[graphql(name = "quantity")]
    pub quantity: i32,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderLineCreateInput")]
pub struct OrderLineCreateInput {

    #[graphql(name = "quantity")]
    pub quantity: i32,

    #[graphql(name = "variantId")]
    pub variant_id: ID,

    #[graphql(name = "forceNewLine")]
    pub force_new_line: Option<bool>,

    #[graphql(name = "price")]
    pub price: Option<GenPositiveDecimal>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderLineInput")]
pub struct OrderLineInput {

    #[graphql(name = "quantity")]
    pub quantity: i32,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderNoteInput")]
pub struct OrderNoteInput {

    #[graphql(name = "message")]
    pub message: String,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderPredicateInput")]
pub struct OrderPredicateInput {

    #[graphql(name = "discountedObjectPredicate")]
    pub discounted_object_predicate: Option<DiscountedObjectWhereInput>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<OrderPredicateInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<OrderPredicateInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderRefundFulfillmentLineInput")]
pub struct OrderRefundFulfillmentLineInput {

    #[graphql(name = "fulfillmentLineId")]
    pub fulfillment_line_id: ID,

    #[graphql(name = "quantity")]
    pub quantity: i32,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderRefundLineInput")]
pub struct OrderRefundLineInput {

    #[graphql(name = "orderLineId")]
    pub order_line_id: ID,

    #[graphql(name = "quantity")]
    pub quantity: i32,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderRefundProductsInput")]
pub struct OrderRefundProductsInput {

    #[graphql(name = "orderLines")]
    pub order_lines: Option<Vec<OrderRefundLineInput>>,

    #[graphql(name = "fulfillmentLines")]
    pub fulfillment_lines: Option<Vec<OrderRefundFulfillmentLineInput>>,

    #[graphql(name = "amountToRefund")]
    pub amount_to_refund: Option<GenPositiveDecimal>,

    #[graphql(name = "includeShippingCosts")]
    pub include_shipping_costs: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderReturnFulfillmentLineInput")]
pub struct OrderReturnFulfillmentLineInput {

    #[graphql(name = "fulfillmentLineId")]
    pub fulfillment_line_id: ID,

    #[graphql(name = "quantity")]
    pub quantity: i32,

    #[graphql(name = "replace")]
    pub replace: Option<bool>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderReturnLineInput")]
pub struct OrderReturnLineInput {

    #[graphql(name = "orderLineId")]
    pub order_line_id: ID,

    #[graphql(name = "quantity")]
    pub quantity: i32,

    #[graphql(name = "replace")]
    pub replace: Option<bool>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderReturnProductsInput")]
pub struct OrderReturnProductsInput {

    #[graphql(name = "orderLines")]
    pub order_lines: Option<Vec<OrderReturnLineInput>>,

    #[graphql(name = "fulfillmentLines")]
    pub fulfillment_lines: Option<Vec<OrderReturnFulfillmentLineInput>>,

    #[graphql(name = "amountToRefund")]
    pub amount_to_refund: Option<GenPositiveDecimal>,

    #[graphql(name = "includeShippingCosts")]
    pub include_shipping_costs: Option<bool>,

    #[graphql(name = "refund")]
    pub refund: Option<bool>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderSettingsInput")]
pub struct OrderSettingsInput {

    #[graphql(name = "automaticallyConfirmAllNewOrders")]
    pub automatically_confirm_all_new_orders: Option<bool>,

    #[graphql(name = "automaticallyFulfillNonShippableGiftCard")]
    pub automatically_fulfill_non_shippable_gift_card: Option<bool>,

    #[graphql(name = "expireOrdersAfter")]
    pub expire_orders_after: Option<i32>,

    #[graphql(name = "deleteExpiredOrdersAfter")]
    pub delete_expired_orders_after: Option<i32>,

    #[graphql(name = "markAsPaidStrategy")]
    pub mark_as_paid_strategy: Option<MarkAsPaidStrategyEnum>,

    #[graphql(name = "allowUnpaidOrders")]
    pub allow_unpaid_orders: Option<bool>,

    #[graphql(name = "includeDraftOrderInVoucherUsage")]
    pub include_draft_order_in_voucher_usage: Option<bool>,

    #[graphql(name = "draftOrderLinePriceFreezePeriod")]
    pub draft_order_line_price_freeze_period: Option<i32>,

    #[graphql(name = "useLegacyLineDiscountPropagation")]
    pub use_legacy_line_discount_propagation: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderSettingsUpdateInput")]
pub struct OrderSettingsUpdateInput {

    #[graphql(name = "automaticallyConfirmAllNewOrders")]
    pub automatically_confirm_all_new_orders: Option<bool>,

    #[graphql(name = "automaticallyFulfillNonShippableGiftCard")]
    pub automatically_fulfill_non_shippable_gift_card: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderSortingInput")]
pub struct OrderSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: OrderSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderStatusEnumFilterInput")]
pub struct OrderStatusEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<OrderStatus>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<OrderStatus>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderUpdateInput")]
pub struct OrderUpdateInput {

    #[graphql(name = "billingAddress")]
    pub billing_address: Option<AddressInput>,

    #[graphql(name = "userEmail")]
    pub user_email: Option<String>,

    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<AddressInput>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "languageCode")]
    pub language_code: Option<LanguageCodeEnum>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderUpdateShippingInput")]
pub struct OrderUpdateShippingInput {

    #[graphql(name = "shippingMethod")]
    pub shipping_method: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OrderWhereInput")]
pub struct OrderWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<MetadataFilterInput>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "number")]
    pub number: Option<IntFilterInput>,

    #[graphql(name = "channelId")]
    pub channel_id: Option<GlobalIDFilterInput>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTimeRangeInput>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

    #[graphql(name = "user")]
    pub user: Option<GlobalIDFilterInput>,

    #[graphql(name = "userEmail")]
    pub user_email: Option<StringFilterInput>,

    #[graphql(name = "authorizeStatus")]
    pub authorize_status: Option<OrderAuthorizeStatusEnumFilterInput>,

    #[graphql(name = "chargeStatus")]
    pub charge_status: Option<OrderChargeStatusEnumFilterInput>,

    #[graphql(name = "status")]
    pub status: Option<OrderStatusEnumFilterInput>,

    #[graphql(name = "checkoutToken")]
    pub checkout_token: Option<UUIDFilterInput>,

    #[graphql(name = "checkoutId")]
    pub checkout_id: Option<GlobalIDFilterInput>,

    #[graphql(name = "isClickAndCollect")]
    pub is_click_and_collect: Option<bool>,

    #[graphql(name = "isGiftCardUsed")]
    pub is_gift_card_used: Option<bool>,

    #[graphql(name = "isGiftCardBought")]
    pub is_gift_card_bought: Option<bool>,

    #[graphql(name = "voucherCode")]
    pub voucher_code: Option<StringFilterInput>,

    #[graphql(name = "hasInvoices")]
    pub has_invoices: Option<bool>,

    #[graphql(name = "invoices")]
    pub invoices: Option<Vec<InvoiceFilterInput>>,

    #[graphql(name = "hasFulfillments")]
    pub has_fulfillments: Option<bool>,

    #[graphql(name = "fulfillments")]
    pub fulfillments: Option<Vec<FulfillmentFilterInput>>,

    #[graphql(name = "lines")]
    pub lines: Option<Vec<LinesFilterInput>>,

    #[graphql(name = "linesCount")]
    pub lines_count: Option<IntFilterInput>,

    #[graphql(name = "transactions")]
    pub transactions: Option<Vec<TransactionFilterInput>>,

    #[graphql(name = "totalGross")]
    pub total_gross: Option<PriceFilterInput>,

    #[graphql(name = "totalNet")]
    pub total_net: Option<PriceFilterInput>,

    #[graphql(name = "productTypeId")]
    pub product_type_id: Option<GlobalIDFilterInput>,

    #[graphql(name = "events")]
    pub events: Option<Vec<OrderEventFilterInput>>,

    #[graphql(name = "billingAddress")]
    pub billing_address: Option<AddressFilterInput>,

    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<AddressFilterInput>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<OrderWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<OrderWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "OtherPaymentMethodDetailsInput")]
pub struct OtherPaymentMethodDetailsInput {

    #[graphql(name = "name")]
    pub name: String,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PageCreateInput")]
pub struct PageCreateInput {

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "title")]
    pub title: Option<String>,

    #[graphql(name = "content")]
    pub content: Option<GenJSONString>,

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AttributeValueInput>>,

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "publicationDate")]
    pub publication_date: Option<String>,

    #[graphql(name = "publishedAt")]
    pub published_at: Option<DateTime<Utc>>,

    #[graphql(name = "seo")]
    pub seo: Option<SeoInput>,

    #[graphql(name = "pageType")]
    pub page_type: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PageFilterInput")]
pub struct PageFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "pageTypes")]
    pub page_types: Option<Vec<ID>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "slugs")]
    pub slugs: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PageInput")]
pub struct PageInput {

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "title")]
    pub title: Option<String>,

    #[graphql(name = "content")]
    pub content: Option<GenJSONString>,

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AttributeValueInput>>,

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "publicationDate")]
    pub publication_date: Option<String>,

    #[graphql(name = "publishedAt")]
    pub published_at: Option<DateTime<Utc>>,

    #[graphql(name = "seo")]
    pub seo: Option<SeoInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PageSortingInput")]
pub struct PageSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: PageSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PageTranslationInput")]
pub struct PageTranslationInput {

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "seoTitle")]
    pub seo_title: Option<String>,

    #[graphql(name = "seoDescription")]
    pub seo_description: Option<String>,

    #[graphql(name = "title")]
    pub title: Option<String>,

    #[graphql(name = "content")]
    pub content: Option<GenJSONString>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PageTypeCreateInput")]
pub struct PageTypeCreateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "addAttributes")]
    pub add_attributes: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PageTypeFilterInput")]
pub struct PageTypeFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "slugs")]
    pub slugs: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PageTypeSortingInput")]
pub struct PageTypeSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: PageTypeSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PageTypeUpdateInput")]
pub struct PageTypeUpdateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "addAttributes")]
    pub add_attributes: Option<Vec<ID>>,

    #[graphql(name = "removeAttributes")]
    pub remove_attributes: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PageWhereInput")]
pub struct PageWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<MetadataFilterInput>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "slug")]
    pub slug: Option<StringFilterInput>,

    #[graphql(name = "pageType")]
    pub page_type: Option<GlobalIDFilterInput>,

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AssignedAttributeWhereInput>>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<PageWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<PageWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PaymentCheckBalanceInput")]
pub struct PaymentCheckBalanceInput {

    #[graphql(name = "gatewayId")]
    pub gateway_id: String,

    #[graphql(name = "method")]
    pub method: String,

    #[graphql(name = "channel")]
    pub channel: String,

    #[graphql(name = "card")]
    pub card: CardInput,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PaymentFilterInput")]
pub struct PaymentFilterInput {

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "checkouts")]
    pub checkouts: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PaymentGatewayToInitialize")]
pub struct PaymentGatewayToInitialize {

    #[graphql(name = "id")]
    pub id: String,

    #[graphql(name = "data")]
    pub data: Option<serde_json::Value>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PaymentInput")]
pub struct PaymentInput {

    #[graphql(name = "gateway")]
    pub gateway: String,

    #[graphql(name = "token")]
    pub token: Option<String>,

    #[graphql(name = "amount")]
    pub amount: Option<GenPositiveDecimal>,

    #[graphql(name = "returnUrl")]
    pub return_url: Option<String>,

    #[graphql(name = "storePaymentMethod")]
    pub store_payment_method: Option<StorePaymentMethodEnum>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PaymentMethodDetailsCardFilterInput")]
pub struct PaymentMethodDetailsCardFilterInput {

    #[graphql(name = "brand")]
    pub brand: Option<StringFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PaymentMethodDetailsFilterInput")]
pub struct PaymentMethodDetailsFilterInput {

    #[graphql(name = "type")]
    pub r#type: Option<PaymentMethodTypeEnumFilterInput>,

    #[graphql(name = "card")]
    pub card: Option<PaymentMethodDetailsCardFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PaymentMethodDetailsInput")]
pub struct PaymentMethodDetailsInput {

    #[graphql(name = "card")]
    pub card: Option<CardPaymentMethodDetailsInput>,

    #[graphql(name = "other")]
    pub other: Option<OtherPaymentMethodDetailsInput>,

    #[graphql(name = "giftCard")]
    pub gift_card: Option<GiftCardPaymentMethodDetailsInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PaymentMethodTypeEnumFilterInput")]
pub struct PaymentMethodTypeEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<PaymentMethodTypeEnum>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<PaymentMethodTypeEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PaymentSettingsInput")]
pub struct PaymentSettingsInput {

    #[graphql(name = "defaultTransactionFlowStrategy")]
    pub default_transaction_flow_strategy: Option<TransactionFlowStrategyEnum>,

    #[graphql(name = "releaseFundsForExpiredCheckouts")]
    pub release_funds_for_expired_checkouts: Option<bool>,

    #[graphql(name = "checkoutTtlBeforeReleasingFunds")]
    pub checkout_ttl_before_releasing_funds: Option<i32>,

    #[graphql(name = "checkoutReleaseFundsCutOffDate")]
    pub checkout_release_funds_cut_off_date: Option<DateTime<Utc>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PermissionGroupCreateInput")]
pub struct PermissionGroupCreateInput {

    #[graphql(name = "addPermissions")]
    pub add_permissions: Option<Vec<PermissionEnum>>,

    #[graphql(name = "addUsers")]
    pub add_users: Option<Vec<ID>>,

    #[graphql(name = "addChannels")]
    pub add_channels: Option<Vec<ID>>,

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "restrictedAccessToChannels")]
    pub restricted_access_to_channels: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PermissionGroupFilterInput")]
pub struct PermissionGroupFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PermissionGroupSortingInput")]
pub struct PermissionGroupSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: PermissionGroupSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PermissionGroupUpdateInput")]
pub struct PermissionGroupUpdateInput {

    #[graphql(name = "addPermissions")]
    pub add_permissions: Option<Vec<PermissionEnum>>,

    #[graphql(name = "addUsers")]
    pub add_users: Option<Vec<ID>>,

    #[graphql(name = "addChannels")]
    pub add_channels: Option<Vec<ID>>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "removePermissions")]
    pub remove_permissions: Option<Vec<PermissionEnum>>,

    #[graphql(name = "removeUsers")]
    pub remove_users: Option<Vec<ID>>,

    #[graphql(name = "removeChannels")]
    pub remove_channels: Option<Vec<ID>>,

    #[graphql(name = "restrictedAccessToChannels")]
    pub restricted_access_to_channels: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PluginFilterInput")]
pub struct PluginFilterInput {

    #[graphql(name = "statusInChannels")]
    pub status_in_channels: Option<PluginStatusInChannelsInput>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "type")]
    pub r#type: Option<PluginConfigurationType>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PluginSortingInput")]
pub struct PluginSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: PluginSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PluginStatusInChannelsInput")]
pub struct PluginStatusInChannelsInput {

    #[graphql(name = "active")]
    pub active: bool,

    #[graphql(name = "channels")]
    pub channels: Vec<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PluginUpdateInput")]
pub struct PluginUpdateInput {

    #[graphql(name = "active")]
    pub active: Option<bool>,

    #[graphql(name = "configuration")]
    pub configuration: Option<Vec<ConfigurationItemInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PreorderSettingsInput")]
pub struct PreorderSettingsInput {

    #[graphql(name = "globalThreshold")]
    pub global_threshold: Option<i32>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTime<Utc>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PriceFilterInput")]
pub struct PriceFilterInput {

    #[graphql(name = "currency")]
    pub currency: Option<String>,

    #[graphql(name = "amount")]
    pub amount: DecimalFilterInput,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PriceInput")]
pub struct PriceInput {

    #[graphql(name = "currency")]
    pub currency: String,

    #[graphql(name = "amount")]
    pub amount: GenPositiveDecimal,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PriceRangeInput")]
pub struct PriceRangeInput {

    #[graphql(name = "gte")]
    pub gte: Option<f64>,

    #[graphql(name = "lte")]
    pub lte: Option<f64>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductAttributeAssignInput")]
pub struct ProductAttributeAssignInput {

    #[graphql(name = "id")]
    pub id: ID,

    #[graphql(name = "type")]
    pub r#type: ProductAttributeType,

    #[graphql(name = "variantSelection")]
    pub variant_selection: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductAttributeAssignmentUpdateInput")]
pub struct ProductAttributeAssignmentUpdateInput {

    #[graphql(name = "id")]
    pub id: ID,

    #[graphql(name = "variantSelection")]
    pub variant_selection: bool,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductBulkCreateInput")]
pub struct ProductBulkCreateInput {

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AttributeValueInput>>,

    #[graphql(name = "category")]
    pub category: Option<ID>,

    #[graphql(name = "chargeTaxes")]
    pub charge_taxes: Option<bool>,

    #[graphql(name = "collections")]
    pub collections: Option<Vec<ID>>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<ID>,

    #[graphql(name = "taxCode")]
    pub tax_code: Option<String>,

    #[graphql(name = "seo")]
    pub seo: Option<SeoInput>,

    #[graphql(name = "weight")]
    pub weight: Option<GenWeightScalar>,

    #[graphql(name = "rating")]
    pub rating: Option<f64>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "productType")]
    pub product_type: ID,

    #[graphql(name = "media")]
    pub media: Option<Vec<MediaInput>>,

    #[graphql(name = "channelListings")]
    pub channel_listings: Option<Vec<ProductChannelListingCreateInput>>,

    #[graphql(name = "variants")]
    pub variants: Option<Vec<ProductVariantBulkCreateInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductBulkTranslateInput")]
pub struct ProductBulkTranslateInput {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "languageCode")]
    pub language_code: LanguageCodeEnum,

    #[graphql(name = "translationFields")]
    pub translation_fields: TranslationInput,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductChannelListingAddInput")]
pub struct ProductChannelListingAddInput {

    #[graphql(name = "channelId")]
    pub channel_id: ID,

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "publicationDate")]
    pub publication_date: Option<DateTime<Utc>>,

    #[graphql(name = "publishedAt")]
    pub published_at: Option<DateTime<Utc>>,

    #[graphql(name = "visibleInListings")]
    pub visible_in_listings: Option<bool>,

    #[graphql(name = "isAvailableForPurchase")]
    pub is_available_for_purchase: Option<bool>,

    #[graphql(name = "availableForPurchaseDate")]
    pub available_for_purchase_date: Option<DateTime<Utc>>,

    #[graphql(name = "availableForPurchaseAt")]
    pub available_for_purchase_at: Option<DateTime<Utc>>,

    #[graphql(name = "addVariants")]
    pub add_variants: Option<Vec<ID>>,

    #[graphql(name = "removeVariants")]
    pub remove_variants: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductChannelListingCreateInput")]
pub struct ProductChannelListingCreateInput {

    #[graphql(name = "channelId")]
    pub channel_id: ID,

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "publishedAt")]
    pub published_at: Option<DateTime<Utc>>,

    #[graphql(name = "visibleInListings")]
    pub visible_in_listings: Option<bool>,

    #[graphql(name = "isAvailableForPurchase")]
    pub is_available_for_purchase: Option<bool>,

    #[graphql(name = "availableForPurchaseAt")]
    pub available_for_purchase_at: Option<DateTime<Utc>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductChannelListingUpdateInput")]
pub struct ProductChannelListingUpdateInput {

    #[graphql(name = "updateChannels")]
    pub update_channels: Option<Vec<ProductChannelListingAddInput>>,

    #[graphql(name = "removeChannels")]
    pub remove_channels: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductCreateInput")]
pub struct ProductCreateInput {

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AttributeValueInput>>,

    #[graphql(name = "category")]
    pub category: Option<ID>,

    #[graphql(name = "chargeTaxes")]
    pub charge_taxes: Option<bool>,

    #[graphql(name = "collections")]
    pub collections: Option<Vec<ID>>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<ID>,

    #[graphql(name = "taxCode")]
    pub tax_code: Option<String>,

    #[graphql(name = "seo")]
    pub seo: Option<SeoInput>,

    #[graphql(name = "weight")]
    pub weight: Option<GenWeightScalar>,

    #[graphql(name = "rating")]
    pub rating: Option<f64>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "productType")]
    pub product_type: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductFilterInput")]
pub struct ProductFilterInput {

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "collections")]
    pub collections: Option<Vec<ID>>,

    #[graphql(name = "categories")]
    pub categories: Option<Vec<ID>>,

    #[graphql(name = "hasCategory")]
    pub has_category: Option<bool>,

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AttributeInput>>,

    #[graphql(name = "stockAvailability")]
    pub stock_availability: Option<StockAvailability>,

    #[graphql(name = "stocks")]
    pub stocks: Option<ProductStockFilterInput>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "publishedFrom")]
    pub published_from: Option<DateTime<Utc>>,

    #[graphql(name = "isAvailable")]
    pub is_available: Option<bool>,

    #[graphql(name = "availableFrom")]
    pub available_from: Option<DateTime<Utc>>,

    #[graphql(name = "isVisibleInListing")]
    pub is_visible_in_listing: Option<bool>,

    #[graphql(name = "price")]
    pub price: Option<PriceRangeInput>,

    #[graphql(name = "minimalPrice")]
    pub minimal_price: Option<PriceRangeInput>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

    #[graphql(name = "productTypes")]
    pub product_types: Option<Vec<ID>>,

    #[graphql(name = "giftCard")]
    pub gift_card: Option<bool>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "hasPreorderedVariants")]
    pub has_preordered_variants: Option<bool>,

    #[graphql(name = "slugs")]
    pub slugs: Option<Vec<String>>,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductInput")]
pub struct ProductInput {

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AttributeValueInput>>,

    #[graphql(name = "category")]
    pub category: Option<ID>,

    #[graphql(name = "chargeTaxes")]
    pub charge_taxes: Option<bool>,

    #[graphql(name = "collections")]
    pub collections: Option<Vec<ID>>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<ID>,

    #[graphql(name = "taxCode")]
    pub tax_code: Option<String>,

    #[graphql(name = "seo")]
    pub seo: Option<SeoInput>,

    #[graphql(name = "weight")]
    pub weight: Option<GenWeightScalar>,

    #[graphql(name = "rating")]
    pub rating: Option<f64>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductMediaCreateInput")]
pub struct ProductMediaCreateInput {

    #[graphql(name = "alt")]
    pub alt: Option<String>,

    #[graphql(name = "image")]
    pub image: Option<GenUpload>,

    #[graphql(name = "product")]
    pub product: ID,

    #[graphql(name = "mediaUrl")]
    pub media_url: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductMediaUpdateInput")]
pub struct ProductMediaUpdateInput {

    #[graphql(name = "alt")]
    pub alt: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductOrder")]
pub struct ProductOrder {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

    #[graphql(name = "attributeId")]
    pub attribute_id: Option<ID>,

    #[graphql(name = "field")]
    pub field: Option<ProductOrderField>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductStockFilterInput")]
pub struct ProductStockFilterInput {

    #[graphql(name = "warehouseIds")]
    pub warehouse_ids: Option<Vec<ID>>,

    #[graphql(name = "quantity")]
    pub quantity: Option<IntRangeInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductTypeFilterInput")]
pub struct ProductTypeFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "configurable")]
    pub configurable: Option<ProductTypeConfigurable>,

    #[graphql(name = "productType")]
    pub product_type: Option<ProductTypeEnum>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "kind")]
    pub kind: Option<ProductTypeKindEnum>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "slugs")]
    pub slugs: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductTypeInput")]
pub struct ProductTypeInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "kind")]
    pub kind: Option<ProductTypeKindEnum>,

    #[graphql(name = "hasVariants")]
    pub has_variants: Option<bool>,

    #[graphql(name = "productAttributes")]
    pub product_attributes: Option<Vec<ID>>,

    #[graphql(name = "variantAttributes")]
    pub variant_attributes: Option<Vec<ID>>,

    #[graphql(name = "isShippingRequired")]
    pub is_shipping_required: Option<bool>,

    #[graphql(name = "isDigital")]
    pub is_digital: Option<bool>,

    #[graphql(name = "weight")]
    pub weight: Option<GenWeightScalar>,

    #[graphql(name = "taxCode")]
    pub tax_code: Option<String>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductTypeSortingInput")]
pub struct ProductTypeSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: ProductTypeSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantBulkCreateInput")]
pub struct ProductVariantBulkCreateInput {

    #[graphql(name = "attributes")]
    pub attributes: Vec<BulkAttributeValueInput>,

    #[graphql(name = "sku")]
    pub sku: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "trackInventory")]
    pub track_inventory: Option<bool>,

    #[graphql(name = "weight")]
    pub weight: Option<GenWeightScalar>,

    #[graphql(name = "preorder")]
    pub preorder: Option<PreorderSettingsInput>,

    #[graphql(name = "quantityLimitPerCustomer")]
    pub quantity_limit_per_customer: Option<i32>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "stocks")]
    pub stocks: Option<Vec<StockInput>>,

    #[graphql(name = "channelListings")]
    pub channel_listings: Option<Vec<ProductVariantChannelListingAddInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantBulkTranslateInput")]
pub struct ProductVariantBulkTranslateInput {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "languageCode")]
    pub language_code: LanguageCodeEnum,

    #[graphql(name = "translationFields")]
    pub translation_fields: NameTranslationInput,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantBulkUpdateInput")]
pub struct ProductVariantBulkUpdateInput {

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<BulkAttributeValueInput>>,

    #[graphql(name = "sku")]
    pub sku: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "trackInventory")]
    pub track_inventory: Option<bool>,

    #[graphql(name = "weight")]
    pub weight: Option<GenWeightScalar>,

    #[graphql(name = "preorder")]
    pub preorder: Option<PreorderSettingsInput>,

    #[graphql(name = "quantityLimitPerCustomer")]
    pub quantity_limit_per_customer: Option<i32>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "stocks")]
    pub stocks: Option<ProductVariantStocksUpdateInput>,

    #[graphql(name = "channelListings")]
    pub channel_listings: Option<ProductVariantChannelListingUpdateInput>,

    #[graphql(name = "id")]
    pub id: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantChannelListingAddInput")]
pub struct ProductVariantChannelListingAddInput {

    #[graphql(name = "channelId")]
    pub channel_id: ID,

    #[graphql(name = "price")]
    pub price: GenPositiveDecimal,

    #[graphql(name = "costPrice")]
    pub cost_price: Option<GenPositiveDecimal>,

    #[graphql(name = "priorPrice")]
    pub prior_price: Option<GenPositiveDecimal>,

    #[graphql(name = "preorderThreshold")]
    pub preorder_threshold: Option<i32>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantChannelListingUpdateInput")]
pub struct ProductVariantChannelListingUpdateInput {

    #[graphql(name = "create")]
    pub create: Option<Vec<ProductVariantChannelListingAddInput>>,

    #[graphql(name = "update")]
    pub update: Option<Vec<ChannelListingUpdateInput>>,

    #[graphql(name = "remove")]
    pub remove: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantCreateInput")]
pub struct ProductVariantCreateInput {

    #[graphql(name = "attributes")]
    pub attributes: Vec<AttributeValueInput>,

    #[graphql(name = "sku")]
    pub sku: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "trackInventory")]
    pub track_inventory: Option<bool>,

    #[graphql(name = "weight")]
    pub weight: Option<GenWeightScalar>,

    #[graphql(name = "preorder")]
    pub preorder: Option<PreorderSettingsInput>,

    #[graphql(name = "quantityLimitPerCustomer")]
    pub quantity_limit_per_customer: Option<i32>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "product")]
    pub product: ID,

    #[graphql(name = "stocks")]
    pub stocks: Option<Vec<StockInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantFilterInput")]
pub struct ProductVariantFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "sku")]
    pub sku: Option<Vec<String>>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "isPreorder")]
    pub is_preorder: Option<bool>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantInput")]
pub struct ProductVariantInput {

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AttributeValueInput>>,

    #[graphql(name = "sku")]
    pub sku: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "trackInventory")]
    pub track_inventory: Option<bool>,

    #[graphql(name = "weight")]
    pub weight: Option<GenWeightScalar>,

    #[graphql(name = "preorder")]
    pub preorder: Option<PreorderSettingsInput>,

    #[graphql(name = "quantityLimitPerCustomer")]
    pub quantity_limit_per_customer: Option<i32>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantSortingInput")]
pub struct ProductVariantSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: ProductVariantSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantStocksUpdateInput")]
pub struct ProductVariantStocksUpdateInput {

    #[graphql(name = "create")]
    pub create: Option<Vec<StockInput>>,

    #[graphql(name = "update")]
    pub update: Option<Vec<StockUpdateInput>>,

    #[graphql(name = "remove")]
    pub remove: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductVariantWhereInput")]
pub struct ProductVariantWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "sku")]
    pub sku: Option<StringFilterInput>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AssignedAttributeWhereInput>>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<ProductVariantWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<ProductVariantWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ProductWhereInput")]
pub struct ProductWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "name")]
    pub name: Option<StringFilterInput>,

    #[graphql(name = "slug")]
    pub slug: Option<StringFilterInput>,

    #[graphql(name = "productType")]
    pub product_type: Option<GlobalIDFilterInput>,

    #[graphql(name = "category")]
    pub category: Option<GlobalIDFilterInput>,

    #[graphql(name = "collection")]
    pub collection: Option<GlobalIDFilterInput>,

    #[graphql(name = "isAvailable")]
    pub is_available: Option<bool>,

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "isVisibleInListing")]
    pub is_visible_in_listing: Option<bool>,

    #[graphql(name = "publishedFrom")]
    pub published_from: Option<DateTime<Utc>>,

    #[graphql(name = "availableFrom")]
    pub available_from: Option<DateTime<Utc>>,

    #[graphql(name = "hasCategory")]
    pub has_category: Option<bool>,

    #[graphql(name = "price")]
    pub price: Option<DecimalFilterInput>,

    #[graphql(name = "minimalPrice")]
    pub minimal_price: Option<DecimalFilterInput>,

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AttributeInput>>,

    #[graphql(name = "stockAvailability")]
    pub stock_availability: Option<StockAvailability>,

    #[graphql(name = "stocks")]
    pub stocks: Option<ProductStockFilterInput>,

    #[graphql(name = "giftCard")]
    pub gift_card: Option<bool>,

    #[graphql(name = "hasPreorderedVariants")]
    pub has_preordered_variants: Option<bool>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeFilterInput>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<ProductWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<ProductWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PromotionCreateInput")]
pub struct PromotionCreateInput {

    #[graphql(name = "description")]
    pub description: Option<serde_json::Value>,

    #[graphql(name = "startDate")]
    pub start_date: Option<DateTime<Utc>>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTime<Utc>>,

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "type")]
    pub r#type: PromotionTypeEnum,

    #[graphql(name = "rules")]
    pub rules: Option<Vec<PromotionRuleInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PromotionRuleCreateInput")]
pub struct PromotionRuleCreateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<serde_json::Value>,

    #[graphql(name = "cataloguePredicate")]
    pub catalogue_predicate: Option<CataloguePredicateInput>,

    #[graphql(name = "orderPredicate")]
    pub order_predicate: Option<OrderPredicateInput>,

    #[graphql(name = "rewardValueType")]
    pub reward_value_type: Option<RewardValueTypeEnum>,

    #[graphql(name = "rewardValue")]
    pub reward_value: Option<GenPositiveDecimal>,

    #[graphql(name = "rewardType")]
    pub reward_type: Option<RewardTypeEnum>,

    #[graphql(name = "channels")]
    pub channels: Option<Vec<ID>>,

    #[graphql(name = "gifts")]
    pub gifts: Option<Vec<ID>>,

    #[graphql(name = "promotion")]
    pub promotion: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PromotionRuleInput")]
pub struct PromotionRuleInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<serde_json::Value>,

    #[graphql(name = "cataloguePredicate")]
    pub catalogue_predicate: Option<CataloguePredicateInput>,

    #[graphql(name = "orderPredicate")]
    pub order_predicate: Option<OrderPredicateInput>,

    #[graphql(name = "rewardValueType")]
    pub reward_value_type: Option<RewardValueTypeEnum>,

    #[graphql(name = "rewardValue")]
    pub reward_value: Option<GenPositiveDecimal>,

    #[graphql(name = "rewardType")]
    pub reward_type: Option<RewardTypeEnum>,

    #[graphql(name = "channels")]
    pub channels: Option<Vec<ID>>,

    #[graphql(name = "gifts")]
    pub gifts: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PromotionRuleTranslationInput")]
pub struct PromotionRuleTranslationInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<serde_json::Value>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PromotionRuleUpdateInput")]
pub struct PromotionRuleUpdateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<serde_json::Value>,

    #[graphql(name = "cataloguePredicate")]
    pub catalogue_predicate: Option<CataloguePredicateInput>,

    #[graphql(name = "orderPredicate")]
    pub order_predicate: Option<OrderPredicateInput>,

    #[graphql(name = "rewardValueType")]
    pub reward_value_type: Option<RewardValueTypeEnum>,

    #[graphql(name = "rewardValue")]
    pub reward_value: Option<GenPositiveDecimal>,

    #[graphql(name = "rewardType")]
    pub reward_type: Option<RewardTypeEnum>,

    #[graphql(name = "addChannels")]
    pub add_channels: Option<Vec<ID>>,

    #[graphql(name = "removeChannels")]
    pub remove_channels: Option<Vec<ID>>,

    #[graphql(name = "addGifts")]
    pub add_gifts: Option<Vec<ID>>,

    #[graphql(name = "removeGifts")]
    pub remove_gifts: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PromotionSortingInput")]
pub struct PromotionSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: PromotionSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PromotionTranslationInput")]
pub struct PromotionTranslationInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<serde_json::Value>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PromotionTypeEnumFilterInput")]
pub struct PromotionTypeEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<PromotionTypeEnum>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<PromotionTypeEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PromotionUpdateInput")]
pub struct PromotionUpdateInput {

    #[graphql(name = "description")]
    pub description: Option<serde_json::Value>,

    #[graphql(name = "startDate")]
    pub start_date: Option<DateTime<Utc>>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTime<Utc>>,

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PromotionWhereInput")]
pub struct PromotionWhereInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "name")]
    pub name: Option<StringFilterInput>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTimeFilterInput>,

    #[graphql(name = "startDate")]
    pub start_date: Option<DateTimeFilterInput>,

    #[graphql(name = "isOldSale")]
    pub is_old_sale: Option<bool>,

    #[graphql(name = "type")]
    pub r#type: Option<PromotionTypeEnumFilterInput>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<PromotionWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<PromotionWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "PublishableChannelListingInput")]
pub struct PublishableChannelListingInput {

    #[graphql(name = "channelId")]
    pub channel_id: ID,

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "publicationDate")]
    pub publication_date: Option<DateTime<Utc>>,

    #[graphql(name = "publishedAt")]
    pub published_at: Option<DateTime<Utc>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "RefundSettingsUpdateInput")]
pub struct RefundSettingsUpdateInput {

    #[graphql(name = "refundReasonReferenceType")]
    pub refund_reason_reference_type: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ReorderInput")]
pub struct ReorderInput {

    #[graphql(name = "id")]
    pub id: ID,

    #[graphql(name = "sortOrder")]
    pub sort_order: Option<i32>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ReturnSettingsUpdateInput")]
pub struct ReturnSettingsUpdateInput {

    #[graphql(name = "returnReasonReferenceType")]
    pub return_reason_reference_type: ID,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "SaleChannelListingAddInput")]
pub struct SaleChannelListingAddInput {

    #[graphql(name = "channelId")]
    pub channel_id: ID,

    #[graphql(name = "discountValue")]
    pub discount_value: GenPositiveDecimal,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "SaleChannelListingInput")]
pub struct SaleChannelListingInput {

    #[graphql(name = "addChannels")]
    pub add_channels: Option<Vec<SaleChannelListingAddInput>>,

    #[graphql(name = "removeChannels")]
    pub remove_channels: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "SaleFilterInput")]
pub struct SaleFilterInput {

    #[graphql(name = "status")]
    pub status: Option<Vec<DiscountStatusEnum>>,

    #[graphql(name = "saleType")]
    pub sale_type: Option<DiscountValueTypeEnum>,

    #[graphql(name = "started")]
    pub started: Option<DateTimeRangeInput>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTimeRangeInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "SaleInput")]
pub struct SaleInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "type")]
    pub r#type: Option<DiscountValueTypeEnum>,

    #[graphql(name = "value")]
    pub value: Option<GenPositiveDecimal>,

    #[graphql(name = "products")]
    pub products: Option<Vec<ID>>,

    #[graphql(name = "variants")]
    pub variants: Option<Vec<ID>>,

    #[graphql(name = "categories")]
    pub categories: Option<Vec<ID>>,

    #[graphql(name = "collections")]
    pub collections: Option<Vec<ID>>,

    #[graphql(name = "startDate")]
    pub start_date: Option<DateTime<Utc>>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTime<Utc>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "SaleSortingInput")]
pub struct SaleSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

    #[graphql(name = "field")]
    pub field: SaleSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "SeoInput")]
pub struct SeoInput {

    #[graphql(name = "title")]
    pub title: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShippingMethodChannelListingAddInput")]
pub struct ShippingMethodChannelListingAddInput {

    #[graphql(name = "channelId")]
    pub channel_id: ID,

    #[graphql(name = "price")]
    pub price: Option<GenPositiveDecimal>,

    #[graphql(name = "minimumOrderPrice")]
    pub minimum_order_price: Option<GenPositiveDecimal>,

    #[graphql(name = "maximumOrderPrice")]
    pub maximum_order_price: Option<GenPositiveDecimal>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShippingMethodChannelListingInput")]
pub struct ShippingMethodChannelListingInput {

    #[graphql(name = "addChannels")]
    pub add_channels: Option<Vec<ShippingMethodChannelListingAddInput>>,

    #[graphql(name = "removeChannels")]
    pub remove_channels: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShippingPostalCodeRulesCreateInputRange")]
pub struct ShippingPostalCodeRulesCreateInputRange {

    #[graphql(name = "start")]
    pub start: String,

    #[graphql(name = "end")]
    pub end: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShippingPriceExcludeProductsInput")]
pub struct ShippingPriceExcludeProductsInput {

    #[graphql(name = "products")]
    pub products: Vec<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShippingPriceInput")]
pub struct ShippingPriceInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "minimumOrderWeight")]
    pub minimum_order_weight: Option<GenWeightScalar>,

    #[graphql(name = "maximumOrderWeight")]
    pub maximum_order_weight: Option<GenWeightScalar>,

    #[graphql(name = "maximumDeliveryDays")]
    pub maximum_delivery_days: Option<i32>,

    #[graphql(name = "minimumDeliveryDays")]
    pub minimum_delivery_days: Option<i32>,

    #[graphql(name = "type")]
    pub r#type: Option<ShippingMethodTypeEnum>,

    #[graphql(name = "shippingZone")]
    pub shipping_zone: Option<ID>,

    #[graphql(name = "addPostalCodeRules")]
    pub add_postal_code_rules: Option<Vec<ShippingPostalCodeRulesCreateInputRange>>,

    #[graphql(name = "deletePostalCodeRules")]
    pub delete_postal_code_rules: Option<Vec<ID>>,

    #[graphql(name = "inclusionType")]
    pub inclusion_type: Option<PostalCodeRuleInclusionTypeEnum>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<ID>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShippingPriceTranslationInput")]
pub struct ShippingPriceTranslationInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShippingZoneCreateInput")]
pub struct ShippingZoneCreateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,

    #[graphql(name = "countries")]
    pub countries: Option<Vec<String>>,

    #[graphql(name = "default")]
    pub default: Option<bool>,

    #[graphql(name = "addWarehouses")]
    pub add_warehouses: Option<Vec<ID>>,

    #[graphql(name = "addChannels")]
    pub add_channels: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShippingZoneFilterInput")]
pub struct ShippingZoneFilterInput {

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "channels")]
    pub channels: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShippingZoneUpdateInput")]
pub struct ShippingZoneUpdateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,

    #[graphql(name = "countries")]
    pub countries: Option<Vec<String>>,

    #[graphql(name = "default")]
    pub default: Option<bool>,

    #[graphql(name = "addWarehouses")]
    pub add_warehouses: Option<Vec<ID>>,

    #[graphql(name = "addChannels")]
    pub add_channels: Option<Vec<ID>>,

    #[graphql(name = "removeWarehouses")]
    pub remove_warehouses: Option<Vec<ID>>,

    #[graphql(name = "removeChannels")]
    pub remove_channels: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShopSettingsInput")]
pub struct ShopSettingsInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "headerText")]
    pub header_text: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,

    #[graphql(name = "trackInventoryByDefault")]
    pub track_inventory_by_default: Option<bool>,

    #[graphql(name = "defaultWeightUnit")]
    pub default_weight_unit: Option<WeightUnitsEnum>,

    #[graphql(name = "fulfillmentAutoApprove")]
    pub fulfillment_auto_approve: Option<bool>,

    #[graphql(name = "fulfillmentAllowUnpaid")]
    pub fulfillment_allow_unpaid: Option<bool>,

    #[graphql(name = "defaultMailSenderName")]
    pub default_mail_sender_name: Option<String>,

    #[graphql(name = "defaultMailSenderAddress")]
    pub default_mail_sender_address: Option<String>,

    #[graphql(name = "customerSetPasswordUrl")]
    pub customer_set_password_url: Option<String>,

    #[graphql(name = "reserveStockDurationAnonymousUser")]
    pub reserve_stock_duration_anonymous_user: Option<i32>,

    #[graphql(name = "reserveStockDurationAuthenticatedUser")]
    pub reserve_stock_duration_authenticated_user: Option<i32>,

    #[graphql(name = "limitQuantityPerCheckout")]
    pub limit_quantity_per_checkout: Option<i32>,

    #[graphql(name = "enableAccountConfirmationByEmail")]
    pub enable_account_confirmation_by_email: Option<bool>,

    #[graphql(name = "allowLoginWithoutConfirmation")]
    pub allow_login_without_confirmation: Option<bool>,

    #[graphql(name = "allowStorefrontTraffic")]
    pub allow_storefront_traffic: Option<bool>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "preserveAllAddressFields")]
    pub preserve_all_address_fields: Option<bool>,

    #[graphql(name = "passwordLoginMode")]
    pub password_login_mode: Option<PasswordLoginModeEnum>,

    #[graphql(name = "useLegacyShippingZoneStockAvailability")]
    pub use_legacy_shipping_zone_stock_availability: Option<bool>,

    #[graphql(name = "includeTaxesInPrices")]
    pub include_taxes_in_prices: Option<bool>,

    #[graphql(name = "displayGrossPrices")]
    pub display_gross_prices: Option<bool>,

    #[graphql(name = "chargeTaxesOnShipping")]
    pub charge_taxes_on_shipping: Option<bool>,

    #[graphql(name = "useLegacyUpdateWebhookEmission")]
    pub use_legacy_update_webhook_emission: Option<bool>,

    #[graphql(name = "accountConfirmMergeMode")]
    pub account_confirm_merge_mode: Option<AccountConfirmModeEnum>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "ShopSettingsTranslationInput")]
pub struct ShopSettingsTranslationInput {

    #[graphql(name = "headerText")]
    pub header_text: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "SiteDomainInput")]
pub struct SiteDomainInput {

    #[graphql(name = "domain")]
    pub domain: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "StaffCreateInput")]
pub struct StaffCreateInput {

    #[graphql(name = "firstName")]
    pub first_name: Option<String>,

    #[graphql(name = "lastName")]
    pub last_name: Option<String>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "note")]
    pub note: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "addGroups")]
    pub add_groups: Option<Vec<ID>>,

    #[graphql(name = "redirectUrl")]
    pub redirect_url: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "StaffNotificationRecipientInput")]
pub struct StaffNotificationRecipientInput {

    #[graphql(name = "user")]
    pub user: Option<ID>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "active")]
    pub active: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "StaffUpdateInput")]
pub struct StaffUpdateInput {

    #[graphql(name = "firstName")]
    pub first_name: Option<String>,

    #[graphql(name = "lastName")]
    pub last_name: Option<String>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "note")]
    pub note: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "addGroups")]
    pub add_groups: Option<Vec<ID>>,

    #[graphql(name = "removeGroups")]
    pub remove_groups: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "StaffUserInput")]
pub struct StaffUserInput {

    #[graphql(name = "status")]
    pub status: Option<StaffMemberStatus>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "StockBulkUpdateInput")]
pub struct StockBulkUpdateInput {

    #[graphql(name = "variantId")]
    pub variant_id: Option<ID>,

    #[graphql(name = "variantExternalReference")]
    pub variant_external_reference: Option<String>,

    #[graphql(name = "warehouseId")]
    pub warehouse_id: Option<ID>,

    #[graphql(name = "warehouseExternalReference")]
    pub warehouse_external_reference: Option<String>,

    #[graphql(name = "quantity")]
    pub quantity: i32,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "StockFilterInput")]
pub struct StockFilterInput {

    #[graphql(name = "quantity")]
    pub quantity: Option<f64>,

    #[graphql(name = "search")]
    pub search: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "StockInput")]
pub struct StockInput {

    #[graphql(name = "warehouse")]
    pub warehouse: ID,

    #[graphql(name = "quantity")]
    pub quantity: i32,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "StockSettingsInput")]
pub struct StockSettingsInput {

    #[graphql(name = "allocationStrategy")]
    pub allocation_strategy: AllocationStrategyEnum,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "StockUpdateInput")]
pub struct StockUpdateInput {

    #[graphql(name = "stock")]
    pub stock: ID,

    #[graphql(name = "quantity")]
    pub quantity: i32,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "StringFilterInput")]
pub struct StringFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<String>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TaxClassCreateInput")]
pub struct TaxClassCreateInput {

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "createCountryRates")]
    pub create_country_rates: Option<Vec<CountryRateInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TaxClassFilterInput")]
pub struct TaxClassFilterInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "countries")]
    pub countries: Option<Vec<CountryCode>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TaxClassRateInput")]
pub struct TaxClassRateInput {

    #[graphql(name = "taxClassId")]
    pub tax_class_id: Option<ID>,

    #[graphql(name = "rate")]
    pub rate: Option<f64>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TaxClassSortingInput")]
pub struct TaxClassSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: TaxClassSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TaxClassUpdateInput")]
pub struct TaxClassUpdateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "updateCountryRates")]
    pub update_country_rates: Option<Vec<CountryRateUpdateInput>>,

    #[graphql(name = "removeCountryRates")]
    pub remove_country_rates: Option<Vec<CountryCode>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TaxConfigurationFilterInput")]
pub struct TaxConfigurationFilterInput {

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TaxConfigurationPerCountryInput")]
pub struct TaxConfigurationPerCountryInput {

    #[graphql(name = "countryCode")]
    pub country_code: CountryCode,

    #[graphql(name = "chargeTaxes")]
    pub charge_taxes: bool,

    #[graphql(name = "taxCalculationStrategy")]
    pub tax_calculation_strategy: Option<TaxCalculationStrategy>,

    #[graphql(name = "displayGrossPrices")]
    pub display_gross_prices: bool,

    #[graphql(name = "taxAppId")]
    pub tax_app_id: Option<String>,

    #[graphql(name = "useWeightedTaxForShipping")]
    pub use_weighted_tax_for_shipping: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TaxConfigurationUpdateInput")]
pub struct TaxConfigurationUpdateInput {

    #[graphql(name = "chargeTaxes")]
    pub charge_taxes: Option<bool>,

    #[graphql(name = "taxCalculationStrategy")]
    pub tax_calculation_strategy: Option<TaxCalculationStrategy>,

    #[graphql(name = "displayGrossPrices")]
    pub display_gross_prices: Option<bool>,

    #[graphql(name = "pricesEnteredWithTax")]
    pub prices_entered_with_tax: Option<bool>,

    #[graphql(name = "updateCountriesConfiguration")]
    pub update_countries_configuration: Option<Vec<TaxConfigurationPerCountryInput>>,

    #[graphql(name = "removeCountriesConfiguration")]
    pub remove_countries_configuration: Option<Vec<CountryCode>>,

    #[graphql(name = "useWeightedTaxForShipping")]
    pub use_weighted_tax_for_shipping: Option<bool>,

    #[graphql(name = "taxAppId")]
    pub tax_app_id: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TaxedMoneyInput")]
pub struct TaxedMoneyInput {

    #[graphql(name = "gross")]
    pub gross: GenPositiveDecimal,

    #[graphql(name = "net")]
    pub net: GenPositiveDecimal,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TimePeriodInputType")]
pub struct TimePeriodInputType {

    #[graphql(name = "amount")]
    pub amount: i32,

    #[graphql(name = "type")]
    pub r#type: TimePeriodTypeEnum,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TransactionCreateInput")]
pub struct TransactionCreateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "pspReference")]
    pub psp_reference: Option<String>,

    #[graphql(name = "availableActions")]
    pub available_actions: Option<Vec<TransactionActionEnum>>,

    #[graphql(name = "amountAuthorized")]
    pub amount_authorized: Option<MoneyInput>,

    #[graphql(name = "amountCharged")]
    pub amount_charged: Option<MoneyInput>,

    #[graphql(name = "amountRefunded")]
    pub amount_refunded: Option<MoneyInput>,

    #[graphql(name = "amountCanceled")]
    pub amount_canceled: Option<MoneyInput>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "externalUrl")]
    pub external_url: Option<String>,

    #[graphql(name = "paymentMethodDetails")]
    pub payment_method_details: Option<PaymentMethodDetailsInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TransactionEventFilterInput")]
pub struct TransactionEventFilterInput {

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTimeRangeInput>,

    #[graphql(name = "type")]
    pub r#type: Option<TransactionEventTypeEnumFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TransactionEventInput")]
pub struct TransactionEventInput {

    #[graphql(name = "pspReference")]
    pub psp_reference: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TransactionEventTypeEnumFilterInput")]
pub struct TransactionEventTypeEnumFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<TransactionEventTypeEnum>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<TransactionEventTypeEnum>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TransactionFilterInput")]
pub struct TransactionFilterInput {

    #[graphql(name = "paymentMethodDetails")]
    pub payment_method_details: Option<PaymentMethodDetailsFilterInput>,

    #[graphql(name = "pspReference")]
    pub psp_reference: Option<StringFilterInput>,

    #[graphql(name = "metadata")]
    pub metadata: Option<MetadataFilterInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TransactionSortingInput")]
pub struct TransactionSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: TransactionSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TransactionUpdateInput")]
pub struct TransactionUpdateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "pspReference")]
    pub psp_reference: Option<String>,

    #[graphql(name = "availableActions")]
    pub available_actions: Option<Vec<TransactionActionEnum>>,

    #[graphql(name = "amountAuthorized")]
    pub amount_authorized: Option<MoneyInput>,

    #[graphql(name = "amountCharged")]
    pub amount_charged: Option<MoneyInput>,

    #[graphql(name = "amountRefunded")]
    pub amount_refunded: Option<MoneyInput>,

    #[graphql(name = "amountCanceled")]
    pub amount_canceled: Option<MoneyInput>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "externalUrl")]
    pub external_url: Option<String>,

    #[graphql(name = "paymentMethodDetails")]
    pub payment_method_details: Option<PaymentMethodDetailsInput>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TransactionWhereInput")]
pub struct TransactionWhereInput {

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "pspReference")]
    pub psp_reference: Option<StringFilterInput>,

    #[graphql(name = "appIdentifier")]
    pub app_identifier: Option<StringFilterInput>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTimeRangeInput>,

    #[graphql(name = "modifiedAt")]
    pub modified_at: Option<DateTimeRangeInput>,

    #[graphql(name = "events")]
    pub events: Option<Vec<TransactionEventFilterInput>>,

    #[graphql(name = "AND")]
    pub and: Option<Vec<TransactionWhereInput>>,

    #[graphql(name = "OR")]
    pub or: Option<Vec<TransactionWhereInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "TranslationInput")]
pub struct TranslationInput {

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "seoTitle")]
    pub seo_title: Option<String>,

    #[graphql(name = "seoDescription")]
    pub seo_description: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "UUIDFilterInput")]
pub struct UUIDFilterInput {

    #[graphql(name = "eq")]
    pub eq: Option<String>,

    #[graphql(name = "oneOf")]
    pub one_of: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "UpdateInvoiceInput")]
pub struct UpdateInvoiceInput {

    #[graphql(name = "number")]
    pub number: Option<String>,

    #[graphql(name = "url")]
    pub url: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "UserCreateInput")]
pub struct UserCreateInput {

    #[graphql(name = "defaultBillingAddress")]
    pub default_billing_address: Option<AddressInput>,

    #[graphql(name = "defaultShippingAddress")]
    pub default_shipping_address: Option<AddressInput>,

    #[graphql(name = "firstName")]
    pub first_name: Option<String>,

    #[graphql(name = "lastName")]
    pub last_name: Option<String>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "note")]
    pub note: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Option<Vec<crate::common::MetadataInput>>,

    #[graphql(name = "languageCode")]
    pub language_code: Option<LanguageCodeEnum>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "isConfirmed")]
    pub is_confirmed: Option<bool>,

    #[graphql(name = "customerType")]
    pub customer_type: Option<ID>,

    #[graphql(name = "attributes")]
    pub attributes: Option<Vec<AttributeValueInput>>,

    #[graphql(name = "redirectUrl")]
    pub redirect_url: Option<String>,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "UserSortingInput")]
pub struct UserSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: UserSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "VoucherChannelListingAddInput")]
pub struct VoucherChannelListingAddInput {

    #[graphql(name = "channelId")]
    pub channel_id: ID,

    #[graphql(name = "discountValue")]
    pub discount_value: Option<GenPositiveDecimal>,

    #[graphql(name = "minAmountSpent")]
    pub min_amount_spent: Option<GenPositiveDecimal>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "VoucherChannelListingInput")]
pub struct VoucherChannelListingInput {

    #[graphql(name = "addChannels")]
    pub add_channels: Option<Vec<VoucherChannelListingAddInput>>,

    #[graphql(name = "removeChannels")]
    pub remove_channels: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "VoucherFilterInput")]
pub struct VoucherFilterInput {

    #[graphql(name = "status")]
    pub status: Option<Vec<DiscountStatusEnum>>,

    #[graphql(name = "timesUsed")]
    pub times_used: Option<IntRangeInput>,

    #[graphql(name = "discountType")]
    pub discount_type: Option<Vec<VoucherDiscountType>>,

    #[graphql(name = "started")]
    pub started: Option<DateTimeRangeInput>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "VoucherInput")]
pub struct VoucherInput {

    #[graphql(name = "type")]
    pub r#type: Option<VoucherTypeEnum>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "addCodes")]
    pub add_codes: Option<Vec<String>>,

    #[graphql(name = "startDate")]
    pub start_date: Option<DateTime<Utc>>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTime<Utc>>,

    #[graphql(name = "discountValueType")]
    pub discount_value_type: Option<DiscountValueTypeEnum>,

    #[graphql(name = "products")]
    pub products: Option<Vec<ID>>,

    #[graphql(name = "variants")]
    pub variants: Option<Vec<ID>>,

    #[graphql(name = "collections")]
    pub collections: Option<Vec<ID>>,

    #[graphql(name = "categories")]
    pub categories: Option<Vec<ID>>,

    #[graphql(name = "minCheckoutItemsQuantity")]
    pub min_checkout_items_quantity: Option<i32>,

    #[graphql(name = "countries")]
    pub countries: Option<Vec<String>>,

    #[graphql(name = "applyOncePerOrder")]
    pub apply_once_per_order: Option<bool>,

    #[graphql(name = "applyOncePerCustomer")]
    pub apply_once_per_customer: Option<bool>,

    #[graphql(name = "onlyForStaff")]
    pub only_for_staff: Option<bool>,

    #[graphql(name = "singleUse")]
    pub single_use: Option<bool>,

    #[graphql(name = "usageLimit")]
    pub usage_limit: Option<i32>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "VoucherSortingInput")]
pub struct VoucherSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "channel")]
    pub channel: Option<String>,

    #[graphql(name = "field")]
    pub field: VoucherSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "WarehouseCreateInput")]
pub struct WarehouseCreateInput {

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "name")]
    pub name: String,

    #[graphql(name = "address")]
    pub address: AddressInput,

    #[graphql(name = "shippingZones")]
    pub shipping_zones: Option<Vec<ID>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "WarehouseFilterInput")]
pub struct WarehouseFilterInput {

    #[graphql(name = "clickAndCollectOption")]
    pub click_and_collect_option: Option<WarehouseClickAndCollectOptionEnum>,

    #[graphql(name = "metadata")]
    pub metadata: Option<Vec<MetadataFilter>>,

    #[graphql(name = "search")]
    pub search: Option<String>,

    #[graphql(name = "ids")]
    pub ids: Option<Vec<ID>>,

    #[graphql(name = "isPrivate")]
    pub is_private: Option<bool>,

    #[graphql(name = "channels")]
    pub channels: Option<Vec<ID>>,

    #[graphql(name = "slugs")]
    pub slugs: Option<Vec<String>>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "WarehouseSortingInput")]
pub struct WarehouseSortingInput {

    #[graphql(name = "direction")]
    pub direction: OrderDirection,

    #[graphql(name = "field")]
    pub field: WarehouseSortField,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "WarehouseUpdateInput")]
pub struct WarehouseUpdateInput {

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "address")]
    pub address: Option<AddressInput>,

    #[graphql(name = "clickAndCollectOption")]
    pub click_and_collect_option: Option<WarehouseClickAndCollectOptionEnum>,

    #[graphql(name = "isPrivate")]
    pub is_private: Option<bool>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "WebhookCreateInput")]
pub struct WebhookCreateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "identifier")]
    pub identifier: Option<String>,

    #[graphql(name = "targetUrl")]
    pub target_url: Option<String>,

    #[graphql(name = "events")]
    pub events: Option<Vec<WebhookEventTypeEnum>>,

    #[graphql(name = "asyncEvents")]
    pub async_events: Option<Vec<WebhookEventTypeAsyncEnum>>,

    #[graphql(name = "syncEvents")]
    pub sync_events: Option<Vec<WebhookEventTypeSyncEnum>>,

    #[graphql(name = "app")]
    pub app: Option<ID>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "secretKey")]
    pub secret_key: Option<String>,

    #[graphql(name = "query")]
    pub query: Option<String>,

    #[graphql(name = "customHeaders")]
    pub custom_headers: Option<GenJSONString>,

}


#[derive(InputObject, Clone, Debug)]
#[graphql(name = "WebhookUpdateInput")]
pub struct WebhookUpdateInput {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "identifier")]
    pub identifier: Option<String>,

    #[graphql(name = "targetUrl")]
    pub target_url: Option<String>,

    #[graphql(name = "events")]
    pub events: Option<Vec<WebhookEventTypeEnum>>,

    #[graphql(name = "asyncEvents")]
    pub async_events: Option<Vec<WebhookEventTypeAsyncEnum>>,

    #[graphql(name = "syncEvents")]
    pub sync_events: Option<Vec<WebhookEventTypeSyncEnum>>,

    #[graphql(name = "app")]
    pub app: Option<ID>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "secretKey")]
    pub secret_key: Option<String>,

    #[graphql(name = "query")]
    pub query: Option<String>,

    #[graphql(name = "customHeaders")]
    pub custom_headers: Option<GenJSONString>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AccountUpdate")]
pub struct AccountUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

    #[graphql(name = "user")]
    pub user: Option<User>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AddressCreate")]
pub struct AddressCreate {

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

    #[graphql(name = "address")]
    pub address: Option<crate::order::GqlAddress>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AddressDelete")]
pub struct AddressDelete {

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AddressSetDefault")]
pub struct AddressSetDefault {

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AddressUpdate")]
pub struct AddressUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

    #[graphql(name = "address")]
    pub address: Option<crate::order::GqlAddress>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AddressValidationData")]
pub struct AddressValidationData {

    #[graphql(name = "allowedFields")]
    pub allowed_fields: Vec<String>,

    #[graphql(name = "countryAreaChoices")]
    pub country_area_choices: Vec<ChoiceValue>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Allocation")]
pub struct Allocation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "quantity")]
    pub quantity: Option<i32>,

    #[graphql(name = "warehouse")]
    pub warehouse: Option<Warehouse>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "App", complex)]
pub struct App {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "identifier")]
    pub identifier: Option<String>,

    #[graphql(name = "permissions")]
    pub permissions: Vec<crate::commerce::GqlPermission>,

    #[graphql(name = "created")]
    pub created: Option<DateTime<Utc>>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "tokens")]
    pub tokens: Vec<AppToken>,

    #[graphql(name = "webhooks")]
    pub webhooks: Vec<Webhook>,

    #[graphql(name = "aboutApp")]
    pub about_app: Option<String>,

    #[graphql(name = "dataPrivacyUrl")]
    pub data_privacy_url: Option<String>,

    #[graphql(name = "homepageUrl")]
    pub homepage_url: Option<String>,

    #[graphql(name = "supportUrl")]
    pub support_url: Option<String>,

    #[graphql(name = "appUrl")]
    pub app_url: Option<String>,

    #[graphql(name = "manifestUrl")]
    pub manifest_url: Option<String>,

    #[graphql(name = "version")]
    pub version: Option<String>,

    #[graphql(name = "accessToken")]
    pub access_token: Option<String>,

    #[graphql(name = "author")]
    pub author: Option<String>,

    #[graphql(name = "brand")]
    pub brand: Option<crate::apps::GqlAppBrand>,

}


#[ComplexObject]
impl App {

    #[graphql(name = "problems")]
    async fn problems(&self, #[graphql(name = "limit")] _arg_limit: Option<i32>) -> Vec<AppProblem> {

        vec![]

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppActivate")]
pub struct AppActivate {

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppCountableEdge")]
pub struct AppCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<App>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppCreate")]
pub struct AppCreate {

    #[graphql(name = "authToken")]
    pub auth_token: Option<String>,

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

    #[graphql(name = "app")]
    pub app: Option<App>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppDeactivate")]
pub struct AppDeactivate {

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppDelete")]
pub struct AppDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

    #[graphql(name = "app")]
    pub app: Option<App>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppDeleteFailedInstallation")]
pub struct AppDeleteFailedInstallation {

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

    #[graphql(name = "appInstallation")]
    pub app_installation: Option<AppInstallation>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppError")]
pub struct AppError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "permissions")]
    pub permissions: Vec<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppExtensionCountableEdge")]
pub struct AppExtensionCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<crate::apps::GqlAppExtension>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppFetchManifest")]
pub struct AppFetchManifest {

    #[graphql(name = "manifest")]
    pub manifest: Option<Manifest>,

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppInstall")]
pub struct AppInstall {

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

    #[graphql(name = "appInstallation")]
    pub app_installation: Option<AppInstallation>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppInstallation", complex)]
pub struct AppInstallation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "status")]
    pub status: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "appName")]
    pub app_name: Option<String>,

    #[graphql(name = "manifestUrl")]
    pub manifest_url: Option<String>,

    #[graphql(name = "brand")]
    pub brand: Option<crate::apps::GqlAppBrand>,

}


#[ComplexObject]
impl AppInstallation {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppManifestBrand")]
pub struct AppManifestBrand {

    #[graphql(name = "logo")]
    pub logo: Option<AppManifestBrandLogo>,

}


#[derive(Clone, Default)]
pub struct AppManifestBrandLogo;


#[Object(name = "AppManifestBrandLogo")]
impl AppManifestBrandLogo {

    async fn default(&self, #[graphql(name = "size")] _arg_size: Option<i32>, #[graphql(name = "format")] _arg_format: Option<crate::apps::IconThumbnailFormatEnum>) -> Option<String> {

        None

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppManifestExtension")]
pub struct AppManifestExtension {

    #[graphql(name = "permissions")]
    pub permissions: Vec<crate::commerce::GqlPermission>,

    #[graphql(name = "label")]
    pub label: Option<String>,

    #[graphql(name = "url")]
    pub url: Option<String>,

    #[graphql(name = "mountName")]
    pub mount_name: Option<String>,

    #[graphql(name = "targetName")]
    pub target_name: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppProblem")]
pub struct AppProblem {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,

    #[graphql(name = "count")]
    pub count: Option<i32>,

    #[graphql(name = "isCritical")]
    pub is_critical: Option<bool>,

    #[graphql(name = "dismissed")]
    pub dismissed: Option<AppProblemDismissed>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "key")]
    pub key: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppProblemDismiss")]
pub struct AppProblemDismiss {

    #[graphql(name = "errors")]
    pub errors: Vec<AppProblemDismissError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppProblemDismissError")]
pub struct AppProblemDismissError {

    #[graphql(name = "message")]
    pub message: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppProblemDismissed")]
pub struct AppProblemDismissed {

    #[graphql(name = "by")]
    pub by: Option<String>,

    #[graphql(name = "userEmail")]
    pub user_email: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppRetryInstall")]
pub struct AppRetryInstall {

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

    #[graphql(name = "appInstallation")]
    pub app_installation: Option<AppInstallation>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppToken")]
pub struct AppToken {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "authToken")]
    pub auth_token: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppTokenCreate")]
pub struct AppTokenCreate {

    #[graphql(name = "authToken")]
    pub auth_token: Option<String>,

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

    #[graphql(name = "appToken")]
    pub app_token: Option<AppToken>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppTokenDelete")]
pub struct AppTokenDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

    #[graphql(name = "appToken")]
    pub app_token: Option<AppToken>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AppUpdate")]
pub struct AppUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<AppError>,

    #[graphql(name = "app")]
    pub app: Option<App>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedBooleanAttribute", complex)]
pub struct AssignedBooleanAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<bool>,

}


#[ComplexObject]
impl AssignedBooleanAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedChoiceAttributeValue")]
pub struct AssignedChoiceAttributeValue {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedDateAttribute", complex)]
pub struct AssignedDateAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<DateTime<Utc>>,

}


#[ComplexObject]
impl AssignedDateAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedDateTimeAttribute", complex)]
pub struct AssignedDateTimeAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<DateTime<Utc>>,

}


#[ComplexObject]
impl AssignedDateTimeAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedFileAttribute", complex)]
pub struct AssignedFileAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<File>,

}


#[ComplexObject]
impl AssignedFileAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedMultiCategoryReferenceAttribute", complex)]
pub struct AssignedMultiCategoryReferenceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Vec<Category>,

}


#[ComplexObject]
impl AssignedMultiCategoryReferenceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedMultiChoiceAttribute", complex)]
pub struct AssignedMultiChoiceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Vec<AssignedChoiceAttributeValue>,

}


#[ComplexObject]
impl AssignedMultiChoiceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedMultiCollectionReferenceAttribute", complex)]
pub struct AssignedMultiCollectionReferenceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Vec<Collection>,

}


#[ComplexObject]
impl AssignedMultiCollectionReferenceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedMultiPageReferenceAttribute", complex)]
pub struct AssignedMultiPageReferenceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Vec<Page>,

}


#[ComplexObject]
impl AssignedMultiPageReferenceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedMultiProductReferenceAttribute", complex)]
pub struct AssignedMultiProductReferenceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Vec<Product>,

}


#[ComplexObject]
impl AssignedMultiProductReferenceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedMultiProductVariantReferenceAttribute", complex)]
pub struct AssignedMultiProductVariantReferenceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Vec<ProductVariant>,

}


#[ComplexObject]
impl AssignedMultiProductVariantReferenceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedNumericAttribute", complex)]
pub struct AssignedNumericAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<f64>,

}


#[ComplexObject]
impl AssignedNumericAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedPlainTextAttribute", complex)]
pub struct AssignedPlainTextAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<String>,

}


#[ComplexObject]
impl AssignedPlainTextAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedSingleCategoryReferenceAttribute", complex)]
pub struct AssignedSingleCategoryReferenceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<Category>,

}


#[ComplexObject]
impl AssignedSingleCategoryReferenceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedSingleChoiceAttribute", complex)]
pub struct AssignedSingleChoiceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<AssignedChoiceAttributeValue>,

}


#[ComplexObject]
impl AssignedSingleChoiceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedSingleCollectionReferenceAttribute", complex)]
pub struct AssignedSingleCollectionReferenceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<Collection>,

}


#[ComplexObject]
impl AssignedSingleCollectionReferenceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedSinglePageReferenceAttribute", complex)]
pub struct AssignedSinglePageReferenceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<Page>,

}


#[ComplexObject]
impl AssignedSinglePageReferenceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedSingleProductReferenceAttribute", complex)]
pub struct AssignedSingleProductReferenceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<Product>,

}


#[ComplexObject]
impl AssignedSingleProductReferenceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedSingleProductVariantReferenceAttribute", complex)]
pub struct AssignedSingleProductVariantReferenceAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<ProductVariant>,

}


#[ComplexObject]
impl AssignedSingleProductVariantReferenceAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedSwatchAttribute", complex)]
pub struct AssignedSwatchAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<AssignedSwatchAttributeValue>,

}


#[ComplexObject]
impl AssignedSwatchAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedSwatchAttributeValue")]
pub struct AssignedSwatchAttributeValue {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedTextAttribute", complex)]
pub struct AssignedTextAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "value")]
    pub value: Option<serde_json::Value>,

}


#[ComplexObject]
impl AssignedTextAttribute {

    pub async fn gen_iface_attribute(&self) -> Option<Attribute> {

        self.attribute.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AssignedVariantAttribute")]
pub struct AssignedVariantAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "variantSelection")]
    pub variant_selection: Option<bool>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Attribute", complex)]
pub struct Attribute {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "inputType")]
    pub input_type: Option<String>,

    #[graphql(name = "entityType")]
    pub entity_type: Option<String>,

    #[graphql(name = "referenceTypes")]
    pub reference_types: Vec<ReferenceType>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "unit")]
    pub unit: Option<String>,

    #[graphql(name = "valueRequired")]
    pub value_required: Option<bool>,

    #[graphql(name = "visibleInStorefront")]
    pub visible_in_storefront: Option<bool>,

    #[graphql(name = "filterableInStorefront")]
    pub filterable_in_storefront: Option<bool>,

    #[graphql(name = "availableInGrid")]
    pub available_in_grid: Option<bool>,

    #[graphql(name = "storefrontSearchPosition")]
    pub storefront_search_position: Option<i32>,

    #[graphql(name = "withChoices")]
    pub with_choices: Option<bool>,

}


#[ComplexObject]
impl Attribute {

    #[graphql(name = "choices")]
    async fn choices(&self, ctx: &Context<'_>, #[graphql(name = "sortBy")] _arg_sort_by: Option<AttributeChoicesSortingInput>, #[graphql(name = "filter")] _arg_filter: Option<AttributeValueFilterInput>, #[graphql(name = "where")] _arg_where: Option<AttributeValueWhereInput>, #[graphql(name = "search")] _arg_search: Option<String>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<AttributeValueCountableConnection> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let gid = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        let search = _arg_search.clone();
        crate::catalog::attribute_choices(&db, &gid, search, _arg_first.clone(), _arg_after.clone()).await
    }

    }

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<AttributeTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::attribute_translation(&db, eid, lang).await
    }

    }

    #[graphql(name = "productTypes")]
    async fn product_types(&self, ctx: &Context<'_>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<ProductTypeCountableConnection> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let gid = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        crate::catalog::attribute_assigned_types(&db, &gid, false).await
    }

    }

    #[graphql(name = "productVariantTypes")]
    async fn product_variant_types(&self, ctx: &Context<'_>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<ProductTypeCountableConnection> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let gid = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        crate::catalog::attribute_assigned_types(&db, &gid, true).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeBulkDelete")]
pub struct AttributeBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<AttributeError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeCountableConnection")]
pub struct AttributeCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<AttributeCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeCountableEdge")]
pub struct AttributeCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Attribute>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeCreate")]
pub struct AttributeCreate {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "errors")]
    pub errors: Vec<AttributeError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeDelete")]
pub struct AttributeDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<AttributeError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeError")]
pub struct AttributeError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeReorderValues")]
pub struct AttributeReorderValues {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "errors")]
    pub errors: Vec<AttributeError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeTranslatableContent", complex)]
pub struct AttributeTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

}


#[ComplexObject]
impl AttributeTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<AttributeTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.attribute.as_ref().and_then(|e| e.id.as_ref()).and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::attribute_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeTranslate")]
pub struct AttributeTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeTranslation")]
pub struct AttributeTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeUpdate")]
pub struct AttributeUpdate {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "errors")]
    pub errors: Vec<AttributeError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeValue", complex)]
pub struct AttributeValue {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "value")]
    pub value: Option<String>,

    #[graphql(name = "inputType")]
    pub input_type: Option<String>,

    #[graphql(name = "reference")]
    pub reference: Option<ID>,

    #[graphql(name = "file")]
    pub file: Option<File>,

    #[graphql(name = "richText")]
    pub rich_text: Option<GenJSONString>,

    #[graphql(name = "plainText")]
    pub plain_text: Option<String>,

    #[graphql(name = "boolean")]
    pub boolean: Option<bool>,

    #[graphql(name = "date")]
    pub date: Option<DateTime<Utc>>,

    #[graphql(name = "dateTime")]
    pub date_time: Option<DateTime<Utc>>,

}


#[ComplexObject]
impl AttributeValue {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<AttributeValueTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::attribute_value_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeValueBulkDelete")]
pub struct AttributeValueBulkDelete {

    #[graphql(name = "count")]
    pub count: Option<i32>,

    #[graphql(name = "errors")]
    pub errors: Vec<AttributeError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeValueCountableConnection")]
pub struct AttributeValueCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<AttributeValueCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeValueCountableEdge")]
pub struct AttributeValueCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<AttributeValue>,

    #[graphql(name = "cursor")]
    pub cursor: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeValueCreate")]
pub struct AttributeValueCreate {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "errors")]
    pub errors: Vec<AttributeError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeValueDelete")]
pub struct AttributeValueDelete {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "errors")]
    pub errors: Vec<AttributeError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeValueTranslatableContent", complex)]
pub struct AttributeValueTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "richText")]
    pub rich_text: Option<GenJSONString>,

    #[graphql(name = "plainText")]
    pub plain_text: Option<String>,

    #[graphql(name = "attributeValue")]
    pub attribute_value: Option<AttributeValue>,

    #[graphql(name = "attribute")]
    pub attribute: Option<AttributeTranslatableContent>,

}


#[ComplexObject]
impl AttributeValueTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<AttributeValueTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.attribute_value.as_ref().and_then(|e| e.id.as_ref()).and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::attribute_value_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeValueTranslate")]
pub struct AttributeValueTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "attributeValue")]
    pub attribute_value: Option<AttributeValue>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeValueTranslation")]
pub struct AttributeValueTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "language")]
    pub language: Option<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "richText")]
    pub rich_text: Option<GenJSONString>,

    #[graphql(name = "plainText")]
    pub plain_text: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "AttributeValueUpdate")]
pub struct AttributeValueUpdate {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "errors")]
    pub errors: Vec<AttributeError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "BulkProductError")]
pub struct BulkProductError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "index")]
    pub index: Option<i32>,

    #[graphql(name = "channels")]
    pub channels: Vec<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "BulkStockError")]
pub struct BulkStockError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "index")]
    pub index: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CardPaymentMethodDetails", complex)]
pub struct CardPaymentMethodDetails {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "brand")]
    pub brand: Option<String>,

    #[graphql(name = "firstDigits")]
    pub first_digits: Option<String>,

    #[graphql(name = "lastDigits")]
    pub last_digits: Option<String>,

    #[graphql(name = "expMonth")]
    pub exp_month: Option<i32>,

    #[graphql(name = "expYear")]
    pub exp_year: Option<i32>,

}


#[ComplexObject]
impl CardPaymentMethodDetails {

    pub async fn gen_iface_name(&self) -> Option<String> {

        self.name.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Category", complex)]
pub struct Category {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "seoTitle")]
    pub seo_title: Option<String>,

    #[graphql(name = "seoDescription")]
    pub seo_description: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "parent")]
    pub parent: Option<Box<Category>>,

    #[graphql(name = "level")]
    pub level: Option<i32>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,

}


#[ComplexObject]
impl Category {

    #[graphql(name = "ancestors")]
    async fn ancestors(&self, ctx: &Context<'_>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<CategoryCountableConnection> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let gid = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        crate::catalog::category_ancestors(&db, &gid, _arg_first.clone()).await
    }

    }

    #[graphql(name = "products")]
    async fn products(&self, ctx: &Context<'_>, #[graphql(name = "filter")] _arg_filter: Option<ProductFilterInput>, #[graphql(name = "where")] _arg_where: Option<ProductWhereInput>, #[graphql(name = "sortBy")] _arg_sort_by: Option<ProductOrder>, #[graphql(name = "search")] _arg_search: Option<String>, #[graphql(name = "channel")] _arg_channel: Option<String>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<crate::catalog::GqlProductConnection> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let gid = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        crate::catalog::category_products(&db, &gid).await
    }

    }

    #[graphql(name = "children")]
    async fn children(&self, ctx: &Context<'_>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<CategoryCountableConnection> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let gid = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        crate::catalog::category_children(&db, &gid, _arg_first.clone(), _arg_after.clone()).await
    }

    }

    #[graphql(name = "backgroundImage")]
    async fn background_image(&self, ctx: &Context<'_>, #[graphql(name = "size")] _arg_size: Option<i32>, #[graphql(name = "format")] _arg_format: Option<ThumbnailFormatEnum>) -> Option<crate::account::GqlImage> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let gid = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        crate::catalog::category_bg_image(&db, &gid).await
    }

    }

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<CategoryTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::category_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryBulkDelete")]
pub struct CategoryBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryCountableConnection")]
pub struct CategoryCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<CategoryCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryCountableEdge")]
pub struct CategoryCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Box<Category>>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryCreate")]
pub struct CategoryCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

    #[graphql(name = "category")]
    pub category: Option<Category>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryDelete")]
pub struct CategoryDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryTranslatableContent", complex)]
pub struct CategoryTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "category")]
    pub category: Option<Category>,

}


#[ComplexObject]
impl CategoryTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<CategoryTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.category.as_ref().and_then(|e| e.id.as_ref()).and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::category_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryTranslate")]
pub struct CategoryTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "category")]
    pub category: Option<Category>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryTranslation")]
pub struct CategoryTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "language")]
    pub language: Option<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "seoTitle")]
    pub seo_title: Option<String>,

    #[graphql(name = "seoDescription")]
    pub seo_description: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryUpdate")]
pub struct CategoryUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

    #[graphql(name = "category")]
    pub category: Option<Category>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Channel", complex)]
pub struct Channel {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "currencyCode")]
    pub currency_code: Option<String>,

    #[graphql(name = "hasOrders")]
    pub has_orders: Option<bool>,

    #[graphql(name = "defaultCountry")]
    pub default_country: Option<crate::common::GqlCountryDisplay>,

    #[graphql(name = "warehouses")]
    pub warehouses: Vec<Warehouse>,

    #[graphql(name = "stockSettings")]
    pub stock_settings: Option<crate::common::GqlStockSettings>,

    #[graphql(name = "orderSettings")]
    pub order_settings: Option<OrderSettings>,

    #[graphql(name = "checkoutSettings")]
    pub checkout_settings: Option<CheckoutSettings>,

    #[graphql(name = "paymentSettings")]
    pub payment_settings: Option<PaymentSettings>,

    #[graphql(name = "taxConfiguration")]
    pub tax_configuration: Option<TaxConfiguration>,

}


#[ComplexObject]
impl Channel {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ChannelActivate")]
pub struct ChannelActivate {

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

    #[graphql(name = "errors")]
    pub errors: Vec<ChannelError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ChannelCreate")]
pub struct ChannelCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<ChannelError>,

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ChannelDeactivate")]
pub struct ChannelDeactivate {

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

    #[graphql(name = "errors")]
    pub errors: Vec<ChannelError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ChannelDelete")]
pub struct ChannelDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ChannelError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ChannelError")]
pub struct ChannelError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ChannelReorderWarehouses")]
pub struct ChannelReorderWarehouses {

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

    #[graphql(name = "errors")]
    pub errors: Vec<ChannelError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ChannelUpdate")]
pub struct ChannelUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<ChannelError>,

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Checkout")]
pub struct Checkout {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "created")]
    pub created: Option<DateTime<Utc>>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CheckoutCountableConnection")]
pub struct CheckoutCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<CheckoutCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CheckoutCountableEdge")]
pub struct CheckoutCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Checkout>,

    #[graphql(name = "cursor")]
    pub cursor: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CheckoutSettings")]
pub struct CheckoutSettings {

    #[graphql(name = "automaticallyCompleteFullyPaidCheckouts")]
    pub automatically_complete_fully_paid_checkouts: Option<bool>,

    #[graphql(name = "automaticCompletionDelay")]
    pub automatic_completion_delay: Option<i32>,

    #[graphql(name = "automaticCompletionCutOffDate")]
    pub automatic_completion_cut_off_date: Option<DateTime<Utc>>,

    #[graphql(name = "allowLegacyGiftCardUse")]
    pub allow_legacy_gift_card_use: Option<bool>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ChoiceValue")]
pub struct ChoiceValue {

    #[graphql(name = "raw")]
    pub raw: Option<String>,

    #[graphql(name = "verbose")]
    pub verbose: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Collection", complex)]
pub struct Collection {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "seoTitle")]
    pub seo_title: Option<String>,

    #[graphql(name = "seoDescription")]
    pub seo_description: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "channelListings")]
    pub channel_listings: Vec<CollectionChannelListing>,

}


#[ComplexObject]
impl Collection {

    #[graphql(name = "products")]
    async fn products(&self, ctx: &Context<'_>, #[graphql(name = "filter")] _arg_filter: Option<ProductFilterInput>, #[graphql(name = "where")] _arg_where: Option<ProductWhereInput>, #[graphql(name = "search")] _arg_search: Option<String>, #[graphql(name = "sortBy")] _arg_sort_by: Option<ProductOrder>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<crate::catalog::GqlProductConnection> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let gid = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        crate::catalog::collection_products(&db, &gid, _arg_first.clone(), _arg_after.clone()).await
    }

    }

    #[graphql(name = "backgroundImage")]
    async fn background_image(&self, ctx: &Context<'_>, #[graphql(name = "size")] _arg_size: Option<i32>, #[graphql(name = "format")] _arg_format: Option<ThumbnailFormatEnum>) -> Option<crate::account::GqlImage> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let gid = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        crate::catalog::collection_bg_image(&db, &gid).await
    }

    }

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<CollectionTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::collection_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionAddProducts")]
pub struct CollectionAddProducts {

    #[graphql(name = "errors")]
    pub errors: Vec<CollectionError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionBulkDelete")]
pub struct CollectionBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<CollectionError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionChannelListing")]
pub struct CollectionChannelListing {

    #[graphql(name = "publishedAt")]
    pub published_at: Option<DateTime<Utc>>,

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionChannelListingError")]
pub struct CollectionChannelListingError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "channels")]
    pub channels: Vec<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionChannelListingUpdate")]
pub struct CollectionChannelListingUpdate {

    #[graphql(name = "collection")]
    pub collection: Option<Collection>,

    #[graphql(name = "errors")]
    pub errors: Vec<CollectionChannelListingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionCountableConnection")]
pub struct CollectionCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<CollectionCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionCountableEdge")]
pub struct CollectionCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Collection>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionCreate")]
pub struct CollectionCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<CollectionError>,

    #[graphql(name = "collection")]
    pub collection: Option<Collection>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionDelete")]
pub struct CollectionDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<CollectionError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionError")]
pub struct CollectionError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionRemoveProducts")]
pub struct CollectionRemoveProducts {

    #[graphql(name = "collection")]
    pub collection: Option<Collection>,

    #[graphql(name = "errors")]
    pub errors: Vec<CollectionError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionReorderProducts")]
pub struct CollectionReorderProducts {

    #[graphql(name = "errors")]
    pub errors: Vec<CollectionError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionTranslatableContent", complex)]
pub struct CollectionTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "collection")]
    pub collection: Option<Collection>,

}


#[ComplexObject]
impl CollectionTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<CollectionTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.collection.as_ref().and_then(|e| e.id.as_ref()).and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::collection_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionTranslate")]
pub struct CollectionTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "collection")]
    pub collection: Option<Collection>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionTranslation")]
pub struct CollectionTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "language")]
    pub language: Option<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "seoTitle")]
    pub seo_title: Option<String>,

    #[graphql(name = "seoDescription")]
    pub seo_description: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CollectionUpdate")]
pub struct CollectionUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<CollectionError>,

    #[graphql(name = "collection")]
    pub collection: Option<Collection>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ConfigurationItem")]
pub struct ConfigurationItem {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "value")]
    pub value: Option<String>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "helpText")]
    pub help_text: Option<String>,

    #[graphql(name = "label")]
    pub label: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerBulkDelete")]
pub struct CustomerBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerCreate")]
pub struct CustomerCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

    #[graphql(name = "user")]
    pub user: Option<User>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerDelete")]
pub struct CustomerDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerType", complex)]
pub struct CustomerType {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "isDefault")]
    pub is_default: Option<bool>,

    #[graphql(name = "attributes")]
    pub attributes: Vec<Attribute>,

}


#[ComplexObject]
impl CustomerType {

    #[graphql(name = "privateMetadata")]
    async fn private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        vec![]

    }

    #[graphql(name = "availableAttributes")]
    async fn available_attributes(&self, #[graphql(name = "where")] _arg_where: Option<AttributeWhereInput>, #[graphql(name = "search")] _arg_search: Option<String>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<AttributeCountableConnection> {

        None

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        vec![]

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeAssignAttributes")]
pub struct CustomerTypeAssignAttributes {

    #[graphql(name = "customerType")]
    pub customer_type: Option<CustomerType>,

    #[graphql(name = "errors")]
    pub errors: Vec<CustomerTypeAssignAttributesError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeAssignAttributesError")]
pub struct CustomerTypeAssignAttributesError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "attributes")]
    pub attributes: Vec<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeCountableConnection")]
pub struct CustomerTypeCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<CustomerTypeCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeCountableEdge")]
pub struct CustomerTypeCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<CustomerType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeCreate")]
pub struct CustomerTypeCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<CustomerTypeCreateError>,

    #[graphql(name = "customerType")]
    pub customer_type: Option<CustomerType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeCreateError")]
pub struct CustomerTypeCreateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeDelete")]
pub struct CustomerTypeDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<CustomerTypeDeleteError>,

    #[graphql(name = "customerType")]
    pub customer_type: Option<CustomerType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeDeleteError")]
pub struct CustomerTypeDeleteError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeReorderAttributes")]
pub struct CustomerTypeReorderAttributes {

    #[graphql(name = "customerType")]
    pub customer_type: Option<CustomerType>,

    #[graphql(name = "errors")]
    pub errors: Vec<CustomerTypeReorderAttributesError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeReorderAttributesError")]
pub struct CustomerTypeReorderAttributesError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "attributes")]
    pub attributes: Vec<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeUnassignAttributes")]
pub struct CustomerTypeUnassignAttributes {

    #[graphql(name = "customerType")]
    pub customer_type: Option<CustomerType>,

    #[graphql(name = "errors")]
    pub errors: Vec<CustomerTypeUnassignAttributesError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeUnassignAttributesError")]
pub struct CustomerTypeUnassignAttributesError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeUpdate")]
pub struct CustomerTypeUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<CustomerTypeUpdateError>,

    #[graphql(name = "customerType")]
    pub customer_type: Option<CustomerType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerTypeUpdateError")]
pub struct CustomerTypeUpdateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "CustomerUpdate")]
pub struct CustomerUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

    #[graphql(name = "user")]
    pub user: Option<User>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "DeleteMetadata")]
pub struct DeleteMetadata {

    #[graphql(name = "errors")]
    pub errors: Vec<MetadataError>,

    #[graphql(name = "item")]
    pub item: Option<ObjectWithMetadata>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "DeletePrivateMetadata")]
pub struct DeletePrivateMetadata {

    #[graphql(name = "errors")]
    pub errors: Vec<MetadataError>,

    #[graphql(name = "item")]
    pub item: Option<ObjectWithMetadata>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "DiscountError")]
pub struct DiscountError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "channels")]
    pub channels: Vec<ID>,

    #[graphql(name = "voucherCodes")]
    pub voucher_codes: Vec<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Domain")]
pub struct Domain {

    #[graphql(name = "host")]
    pub host: Option<String>,

    #[graphql(name = "url")]
    pub url: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "DraftOrderBulkDelete")]
pub struct DraftOrderBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "DraftOrderComplete")]
pub struct DraftOrderComplete {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "DraftOrderCreate")]
pub struct DraftOrderCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

    #[graphql(name = "order")]
    pub order: Option<Order>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "DraftOrderDelete")]
pub struct DraftOrderDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "DraftOrderUpdate")]
pub struct DraftOrderUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

    #[graphql(name = "order")]
    pub order: Option<Order>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "EventDelivery", complex)]
pub struct EventDelivery {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,

    #[graphql(name = "status")]
    pub status: Option<String>,

    #[graphql(name = "eventType")]
    pub event_type: Option<String>,

}


#[ComplexObject]
impl EventDelivery {

    #[graphql(name = "attempts")]
    async fn attempts(&self, #[graphql(name = "sortBy")] _arg_sort_by: Option<EventDeliveryAttemptSortingInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<crate::apps::GqlEventDeliveryAttemptConnection> {

        None

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "EventDeliveryAttempt", complex)]
pub struct EventDeliveryAttempt {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,

    #[graphql(name = "response")]
    pub response: Option<String>,

    #[graphql(name = "responseStatusCode")]
    pub response_status_code: Option<i32>,

    #[graphql(name = "status")]
    pub status: Option<String>,

}


#[ComplexObject]
impl EventDeliveryAttempt {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "EventDeliveryAttemptCountableEdge")]
pub struct EventDeliveryAttemptCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<EventDeliveryAttempt>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "EventDeliveryCountableEdge")]
pub struct EventDeliveryCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<EventDelivery>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ExportError")]
pub struct ExportError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ExportFile", complex)]
pub struct ExportFile {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "status")]
    pub status: Option<String>,

    #[graphql(name = "url")]
    pub url: Option<String>,

}


#[ComplexObject]
impl ExportFile {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ExportProducts")]
pub struct ExportProducts {

    #[graphql(name = "exportFile")]
    pub export_file: Option<ExportFile>,

    #[graphql(name = "errors")]
    pub errors: Vec<ExportError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ExternalAuthentication")]
pub struct ExternalAuthentication {

    #[graphql(name = "id")]
    pub id: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ExternalAuthenticationUrl")]
pub struct ExternalAuthenticationUrl {

    #[graphql(name = "authenticationData")]
    pub authentication_data: Option<GenJSONString>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ExternalLogout")]
pub struct ExternalLogout {

    #[graphql(name = "logoutData")]
    pub logout_data: Option<GenJSONString>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ExternalObtainAccessTokens")]
pub struct ExternalObtainAccessTokens {

    #[graphql(name = "token")]
    pub token: Option<String>,

    #[graphql(name = "refreshToken")]
    pub refresh_token: Option<String>,

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ExternalRefresh")]
pub struct ExternalRefresh {

    #[graphql(name = "token")]
    pub token: Option<String>,

    #[graphql(name = "refreshToken")]
    pub refresh_token: Option<String>,

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "File")]
pub struct File {

    #[graphql(name = "url")]
    pub url: Option<String>,

    #[graphql(name = "contentType")]
    pub content_type: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "FileUpload")]
pub struct FileUpload {

    #[graphql(name = "uploadedFile")]
    pub uploaded_file: Option<File>,

    #[graphql(name = "errors")]
    pub errors: Vec<UploadError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Fulfillment", complex)]
pub struct Fulfillment {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "fulfillmentOrder")]
    pub fulfillment_order: Option<i32>,

    #[graphql(name = "status")]
    pub status: Option<String>,

    #[graphql(name = "trackingNumber")]
    pub tracking_number: Option<String>,

    #[graphql(name = "created")]
    pub created: Option<DateTime<Utc>>,

    #[graphql(name = "lines")]
    pub lines: Vec<FulfillmentLine>,

    #[graphql(name = "warehouse")]
    pub warehouse: Option<Warehouse>,

    #[graphql(name = "shippingRefundedAmount")]
    pub shipping_refunded_amount: Option<crate::common::Money>,

    #[graphql(name = "totalRefundedAmount")]
    pub total_refunded_amount: Option<crate::common::Money>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<Page>,

}


#[ComplexObject]
impl Fulfillment {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "FulfillmentApprove")]
pub struct FulfillmentApprove {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "FulfillmentCancel")]
pub struct FulfillmentCancel {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "FulfillmentLine")]
pub struct FulfillmentLine {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "quantity")]
    pub quantity: Option<i32>,

    #[graphql(name = "orderLine")]
    pub order_line: Option<OrderLine>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<Page>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "FulfillmentRefundProducts")]
pub struct FulfillmentRefundProducts {

    #[graphql(name = "fulfillment")]
    pub fulfillment: Option<Fulfillment>,

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "FulfillmentReturnProducts")]
pub struct FulfillmentReturnProducts {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "replaceOrder")]
    pub replace_order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "FulfillmentUpdateTracking")]
pub struct FulfillmentUpdateTracking {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCard", complex)]
pub struct GiftCard {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "displayCode")]
    pub display_code: Option<String>,

    #[graphql(name = "last4CodeChars")]
    pub last4_code_chars: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "created")]
    pub created: Option<DateTime<Utc>>,

    #[graphql(name = "createdBy")]
    pub created_by: Option<Box<User>>,

    #[graphql(name = "createdByEmail")]
    pub created_by_email: Option<String>,

    #[graphql(name = "assignedTo")]
    pub assigned_to: Option<Box<User>>,

    #[graphql(name = "assignedToEmail")]
    pub assigned_to_email: Option<String>,

    #[graphql(name = "lastUsedOn")]
    pub last_used_on: Option<DateTime<Utc>>,

    #[graphql(name = "expiryDate")]
    pub expiry_date: Option<DateTime<Utc>>,

    #[graphql(name = "app")]
    pub app: Option<App>,

    #[graphql(name = "product")]
    pub product: Option<Product>,

    #[graphql(name = "events")]
    pub events: Vec<GiftCardEvent>,

    #[graphql(name = "tags")]
    pub tags: Vec<GiftCardTag>,

    #[graphql(name = "boughtInChannel")]
    pub bought_in_channel: Option<String>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "initialBalance")]
    pub initial_balance: Option<crate::common::Money>,

    #[graphql(name = "currentBalance")]
    pub current_balance: Option<crate::common::Money>,

}


#[ComplexObject]
impl GiftCard {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardActivate")]
pub struct GiftCardActivate {

    #[graphql(name = "giftCard")]
    pub gift_card: Option<GiftCard>,

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardAddNote")]
pub struct GiftCardAddNote {

    #[graphql(name = "giftCard")]
    pub gift_card: Option<GiftCard>,

    #[graphql(name = "event")]
    pub event: Option<GiftCardEvent>,

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardAssignUser")]
pub struct GiftCardAssignUser {

    #[graphql(name = "giftCard")]
    pub gift_card: Option<GiftCard>,

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardBulkActivate")]
pub struct GiftCardBulkActivate {

    #[graphql(name = "count")]
    pub count: Option<i32>,

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardBulkCreate")]
pub struct GiftCardBulkCreate {

    #[graphql(name = "giftCards")]
    pub gift_cards: Vec<GiftCard>,

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardBulkDeactivate")]
pub struct GiftCardBulkDeactivate {

    #[graphql(name = "count")]
    pub count: Option<i32>,

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardBulkDelete")]
pub struct GiftCardBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardCountableConnection")]
pub struct GiftCardCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<GiftCardCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardCountableEdge")]
pub struct GiftCardCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<GiftCard>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardCreate")]
pub struct GiftCardCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

    #[graphql(name = "giftCard")]
    pub gift_card: Option<GiftCard>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardDeactivate")]
pub struct GiftCardDeactivate {

    #[graphql(name = "giftCard")]
    pub gift_card: Option<GiftCard>,

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardDelete")]
pub struct GiftCardDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardError")]
pub struct GiftCardError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardEvent", complex)]
pub struct GiftCardEvent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "date")]
    pub date: Option<DateTime<Utc>>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "user")]
    pub user: Option<Box<User>>,

    #[graphql(name = "app")]
    pub app: Option<App>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "orderId")]
    pub order_id: Option<ID>,

    #[graphql(name = "orderNumber")]
    pub order_number: Option<String>,

    #[graphql(name = "tags")]
    pub tags: Vec<String>,

    #[graphql(name = "oldTags")]
    pub old_tags: Vec<String>,

    #[graphql(name = "balance")]
    pub balance: Option<GiftCardEventBalance>,

    #[graphql(name = "assignedTo")]
    pub assigned_to: Option<GiftCardEventAssignment>,

    #[graphql(name = "expiryDate")]
    pub expiry_date: Option<DateTime<Utc>>,

    #[graphql(name = "oldExpiryDate")]
    pub old_expiry_date: Option<DateTime<Utc>>,

}


#[ComplexObject]
impl GiftCardEvent {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardEventAssignment")]
pub struct GiftCardEventAssignment {

    #[graphql(name = "oldAssignedTo")]
    pub old_assigned_to: Option<Box<User>>,

    #[graphql(name = "currentAssignedTo")]
    pub current_assigned_to: Option<Box<User>>,

    #[graphql(name = "oldAssignedToEmail")]
    pub old_assigned_to_email: Option<String>,

    #[graphql(name = "currentAssignedToEmail")]
    pub current_assigned_to_email: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardEventBalance")]
pub struct GiftCardEventBalance {

    #[graphql(name = "initialBalance")]
    pub initial_balance: Option<crate::common::Money>,

    #[graphql(name = "currentBalance")]
    pub current_balance: Option<crate::common::Money>,

    #[graphql(name = "oldInitialBalance")]
    pub old_initial_balance: Option<crate::common::Money>,

    #[graphql(name = "oldCurrentBalance")]
    pub old_current_balance: Option<crate::common::Money>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardPaymentMethodDetails", complex)]
pub struct GiftCardPaymentMethodDetails {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "brand")]
    pub brand: Option<String>,

    #[graphql(name = "lastChars")]
    pub last_chars: Option<String>,

    #[graphql(name = "isSaleorGiftcard")]
    pub is_saleor_giftcard: Option<bool>,

}


#[ComplexObject]
impl GiftCardPaymentMethodDetails {

    pub async fn gen_iface_name(&self) -> Option<String> {

        self.name.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardResend")]
pub struct GiftCardResend {

    #[graphql(name = "giftCard")]
    pub gift_card: Option<GiftCard>,

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardSettings")]
pub struct GiftCardSettings {

    #[graphql(name = "expiryType")]
    pub expiry_type: Option<String>,

    #[graphql(name = "expiryPeriod")]
    pub expiry_period: Option<TimePeriod>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardSettingsError")]
pub struct GiftCardSettingsError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardSettingsUpdate")]
pub struct GiftCardSettingsUpdate {

    #[graphql(name = "giftCardSettings")]
    pub gift_card_settings: Option<GiftCardSettings>,

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardSettingsError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardTag")]
pub struct GiftCardTag {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardTagCountableConnection")]
pub struct GiftCardTagCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<GiftCardTagCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardTagCountableEdge")]
pub struct GiftCardTagCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<GiftCardTag>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardUnassignUser")]
pub struct GiftCardUnassignUser {

    #[graphql(name = "giftCard")]
    pub gift_card: Option<GiftCard>,

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GiftCardUpdate")]
pub struct GiftCardUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<GiftCardError>,

    #[graphql(name = "giftCard")]
    pub gift_card: Option<GiftCard>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Group", complex)]
pub struct Group {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "users")]
    pub users: Vec<Box<User>>,

    #[graphql(name = "permissions")]
    pub permissions: Vec<crate::commerce::GqlPermission>,

    #[graphql(name = "userCanManage")]
    pub user_can_manage: Option<bool>,

    #[graphql(name = "accessibleChannels")]
    pub accessible_channels: Vec<Channel>,

    #[graphql(name = "restrictedAccessToChannels")]
    pub restricted_access_to_channels: Option<bool>,

}


#[ComplexObject]
impl Group {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GroupCountableConnection")]
pub struct GroupCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<GroupCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "GroupCountableEdge")]
pub struct GroupCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Group>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Invoice", complex)]
pub struct Invoice {

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "status")]
    pub status: Option<String>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "number")]
    pub number: Option<String>,

    #[graphql(name = "url")]
    pub url: Option<String>,

}


#[ComplexObject]
impl Invoice {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "InvoiceError")]
pub struct InvoiceError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "InvoiceRequest")]
pub struct InvoiceRequest {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<InvoiceError>,

    #[graphql(name = "invoice")]
    pub invoice: Option<Invoice>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "InvoiceSendNotification")]
pub struct InvoiceSendNotification {

    #[graphql(name = "errors")]
    pub errors: Vec<InvoiceError>,

    #[graphql(name = "invoice")]
    pub invoice: Option<Invoice>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Manifest")]
pub struct Manifest {

    #[graphql(name = "identifier")]
    pub identifier: Option<String>,

    #[graphql(name = "version")]
    pub version: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "about")]
    pub about: Option<String>,

    #[graphql(name = "permissions")]
    pub permissions: Vec<crate::commerce::GqlPermission>,

    #[graphql(name = "appUrl")]
    pub app_url: Option<String>,

    #[graphql(name = "tokenTargetUrl")]
    pub token_target_url: Option<String>,

    #[graphql(name = "dataPrivacy")]
    pub data_privacy: Option<String>,

    #[graphql(name = "dataPrivacyUrl")]
    pub data_privacy_url: Option<String>,

    #[graphql(name = "homepageUrl")]
    pub homepage_url: Option<String>,

    #[graphql(name = "supportUrl")]
    pub support_url: Option<String>,

    #[graphql(name = "extensions")]
    pub extensions: Vec<AppManifestExtension>,

    #[graphql(name = "brand")]
    pub brand: Option<AppManifestBrand>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Menu", complex)]
pub struct Menu {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "items")]
    pub items: Vec<MenuItem>,

}


#[ComplexObject]
impl Menu {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuBulkDelete")]
pub struct MenuBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<MenuError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuCountableConnection")]
pub struct MenuCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<MenuCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuCountableEdge")]
pub struct MenuCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Menu>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuCreate")]
pub struct MenuCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<MenuError>,

    #[graphql(name = "menu")]
    pub menu: Option<Menu>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuDelete")]
pub struct MenuDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<MenuError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuError")]
pub struct MenuError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuItem", complex)]
pub struct MenuItem {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "menu")]
    pub menu: Option<Box<Menu>>,

    #[graphql(name = "category")]
    pub category: Option<Category>,

    #[graphql(name = "collection")]
    pub collection: Option<Collection>,

    #[graphql(name = "page")]
    pub page: Option<Page>,

    #[graphql(name = "level")]
    pub level: Option<i32>,

    #[graphql(name = "children")]
    pub children: Vec<Box<MenuItem>>,

    #[graphql(name = "url")]
    pub url: Option<String>,

}


#[ComplexObject]
impl MenuItem {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<MenuItemTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::menu_item_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuItemBulkDelete")]
pub struct MenuItemBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<MenuError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuItemCreate")]
pub struct MenuItemCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<MenuError>,

    #[graphql(name = "menuItem")]
    pub menu_item: Option<MenuItem>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuItemMove")]
pub struct MenuItemMove {

    #[graphql(name = "errors")]
    pub errors: Vec<MenuError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuItemTranslatableContent", complex)]
pub struct MenuItemTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "menuItem")]
    pub menu_item: Option<MenuItem>,

}


#[ComplexObject]
impl MenuItemTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<MenuItemTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.menu_item.as_ref().and_then(|e| e.id.as_ref()).and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::menu_item_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuItemTranslate")]
pub struct MenuItemTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "menuItem")]
    pub menu_item: Option<MenuItem>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuItemTranslation")]
pub struct MenuItemTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "language")]
    pub language: Option<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuItemUpdate")]
pub struct MenuItemUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<MenuError>,

    #[graphql(name = "menuItem")]
    pub menu_item: Option<MenuItem>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MenuUpdate")]
pub struct MenuUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<MenuError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MetadataError")]
pub struct MetadataError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "MoneyRange")]
pub struct MoneyRange {

    #[graphql(name = "start")]
    pub start: Option<crate::common::Money>,

    #[graphql(name = "stop")]
    pub stop: Option<crate::common::Money>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Order", complex)]
pub struct Order {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "created")]
    pub created: Option<DateTime<Utc>>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,

    #[graphql(name = "status")]
    pub status: Option<String>,

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "billingAddress")]
    pub billing_address: Option<crate::order::GqlAddress>,

    #[graphql(name = "shippingAddress")]
    pub shipping_address: Option<crate::order::GqlAddress>,

    #[graphql(name = "shippingMethodName")]
    pub shipping_method_name: Option<String>,

    #[graphql(name = "collectionPointName")]
    pub collection_point_name: Option<String>,

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

    #[graphql(name = "fulfillments")]
    pub fulfillments: Vec<Fulfillment>,

    #[graphql(name = "lines")]
    pub lines: Vec<OrderLine>,

    #[graphql(name = "actions")]
    pub actions: Vec<String>,

    #[graphql(name = "shippingMethods")]
    pub shipping_methods: Vec<ShippingMethod>,

    #[graphql(name = "invoices")]
    pub invoices: Vec<Invoice>,

    #[graphql(name = "number")]
    pub number: Option<String>,

    #[graphql(name = "isPaid")]
    pub is_paid: Option<bool>,

    #[graphql(name = "paymentStatus")]
    pub payment_status: Option<String>,

    #[graphql(name = "authorizeStatus")]
    pub authorize_status: Option<String>,

    #[graphql(name = "chargeStatus")]
    pub charge_status: Option<String>,

    #[graphql(name = "transactions")]
    pub transactions: Vec<TransactionItem>,

    #[graphql(name = "payments")]
    pub payments: Vec<Payment>,

    #[graphql(name = "total")]
    pub total: Option<crate::order::GqlTaxedMoney>,

    #[graphql(name = "undiscountedTotal")]
    pub undiscounted_total: Option<crate::order::GqlTaxedMoney>,

    #[graphql(name = "shippingMethod")]
    pub shipping_method: Option<ShippingMethod>,

    #[graphql(name = "shippingPrice")]
    pub shipping_price: Option<crate::order::GqlTaxedMoney>,

    #[graphql(name = "voucher")]
    pub voucher: Option<Voucher>,

    #[graphql(name = "voucherCode")]
    pub voucher_code: Option<String>,

    #[graphql(name = "giftCards")]
    pub gift_cards: Vec<GiftCard>,

    #[graphql(name = "customerNote")]
    pub customer_note: Option<String>,

    #[graphql(name = "subtotal")]
    pub subtotal: Option<crate::order::GqlTaxedMoney>,

    #[graphql(name = "totalAuthorized")]
    pub total_authorized: Option<crate::common::Money>,

    #[graphql(name = "totalCharged")]
    pub total_charged: Option<crate::common::Money>,

    #[graphql(name = "totalCanceled")]
    pub total_canceled: Option<crate::common::Money>,

    #[graphql(name = "events")]
    pub events: Vec<OrderEvent>,

    #[graphql(name = "totalBalance")]
    pub total_balance: Option<crate::common::Money>,

    #[graphql(name = "userEmail")]
    pub user_email: Option<String>,

    #[graphql(name = "isShippingRequired")]
    pub is_shipping_required: Option<bool>,

    #[graphql(name = "deliveryMethod")]
    pub delivery_method: Option<DeliveryMethod>,

    #[graphql(name = "discounts")]
    pub discounts: Vec<OrderDiscount>,

    #[graphql(name = "displayGrossPrices")]
    pub display_gross_prices: Option<bool>,

    #[graphql(name = "grantedRefunds")]
    pub granted_refunds: Vec<OrderGrantedRefund>,

    #[graphql(name = "totalGrantedRefund")]
    pub total_granted_refund: Option<crate::common::Money>,

    #[graphql(name = "totalRefunded")]
    pub total_refunded: Option<crate::common::Money>,

    #[graphql(name = "totalRefundPending")]
    pub total_refund_pending: Option<crate::common::Money>,

    #[graphql(name = "totalAuthorizePending")]
    pub total_authorize_pending: Option<crate::common::Money>,

    #[graphql(name = "totalChargePending")]
    pub total_charge_pending: Option<crate::common::Money>,

    #[graphql(name = "totalCancelPending")]
    pub total_cancel_pending: Option<crate::common::Money>,

    #[graphql(name = "totalRemainingGrant")]
    pub total_remaining_grant: Option<crate::common::Money>,

}


#[ComplexObject]
impl Order {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderCancel")]
pub struct OrderCancel {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderCapture")]
pub struct OrderCapture {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderConfirm")]
pub struct OrderConfirm {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderCountableEdge")]
pub struct OrderCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Order>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderDiscount", complex)]
pub struct OrderDiscount {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "translatedName")]
    pub translated_name: Option<String>,

    #[graphql(name = "valueType")]
    pub value_type: Option<String>,

    #[graphql(name = "value")]
    pub value: Option<GenPositiveDecimal>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "amount")]
    pub amount: Option<crate::common::Money>,

    #[graphql(name = "total")]
    pub total: Option<crate::common::Money>,

}


#[ComplexObject]
impl OrderDiscount {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderDiscountAdd")]
pub struct OrderDiscountAdd {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderDiscountDelete")]
pub struct OrderDiscountDelete {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderDiscountUpdate")]
pub struct OrderDiscountUpdate {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderError")]
pub struct OrderError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "warehouse")]
    pub warehouse: Option<ID>,

    #[graphql(name = "orderLines")]
    pub order_lines: Vec<ID>,

    #[graphql(name = "addressType")]
    pub address_type: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderEvent", complex)]
pub struct OrderEvent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "date")]
    pub date: Option<DateTime<Utc>>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "app")]
    pub app: Option<App>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "emailType")]
    pub email_type: Option<String>,

    #[graphql(name = "amount")]
    pub amount: Option<f64>,

    #[graphql(name = "quantity")]
    pub quantity: Option<i32>,

    #[graphql(name = "composedId")]
    pub composed_id: Option<String>,

    #[graphql(name = "orderNumber")]
    pub order_number: Option<String>,

    #[graphql(name = "invoiceNumber")]
    pub invoice_number: Option<String>,

    #[graphql(name = "lines")]
    pub lines: Vec<OrderEventOrderLineObject>,

    #[graphql(name = "warehouse")]
    pub warehouse: Option<Warehouse>,

    #[graphql(name = "transactionReference")]
    pub transaction_reference: Option<String>,

    #[graphql(name = "shippingCostsIncluded")]
    pub shipping_costs_included: Option<bool>,

    #[graphql(name = "relatedOrder")]
    pub related_order: Option<Box<Order>>,

    #[graphql(name = "related")]
    pub related: Option<Box<OrderEvent>>,

    #[graphql(name = "discount")]
    pub discount: Option<OrderEventDiscountObject>,

}


#[ComplexObject]
impl OrderEvent {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderEventCountableConnection")]
pub struct OrderEventCountableConnection {

    #[graphql(name = "edges")]
    pub edges: Vec<OrderEventCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderEventCountableEdge")]
pub struct OrderEventCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<OrderEvent>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderEventDiscountObject")]
pub struct OrderEventDiscountObject {

    #[graphql(name = "valueType")]
    pub value_type: Option<String>,

    #[graphql(name = "value")]
    pub value: Option<GenPositiveDecimal>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "amount")]
    pub amount: Option<crate::common::Money>,

    #[graphql(name = "oldValueType")]
    pub old_value_type: Option<String>,

    #[graphql(name = "oldValue")]
    pub old_value: Option<GenPositiveDecimal>,

    #[graphql(name = "oldAmount")]
    pub old_amount: Option<crate::common::Money>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderEventOrderLineObject")]
pub struct OrderEventOrderLineObject {

    #[graphql(name = "quantity")]
    pub quantity: Option<i32>,

    #[graphql(name = "orderLine")]
    pub order_line: Option<OrderLine>,

    #[graphql(name = "itemName")]
    pub item_name: Option<String>,

    #[graphql(name = "discount")]
    pub discount: Option<OrderEventDiscountObject>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderFulfill")]
pub struct OrderFulfill {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderGrantRefundCreate")]
pub struct OrderGrantRefundCreate {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "grantedRefund")]
    pub granted_refund: Option<OrderGrantedRefund>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderGrantRefundCreateError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderGrantRefundCreateError")]
pub struct OrderGrantRefundCreateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "lines")]
    pub lines: Vec<OrderGrantRefundCreateLineError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderGrantRefundCreateLineError")]
pub struct OrderGrantRefundCreateLineError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "lineId")]
    pub line_id: Option<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderGrantRefundUpdate")]
pub struct OrderGrantRefundUpdate {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderGrantRefundUpdateError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderGrantRefundUpdateError")]
pub struct OrderGrantRefundUpdateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "addLines")]
    pub add_lines: Vec<OrderGrantRefundUpdateLineError>,

    #[graphql(name = "removeLines")]
    pub remove_lines: Vec<OrderGrantRefundUpdateLineError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderGrantRefundUpdateLineError")]
pub struct OrderGrantRefundUpdateLineError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "lineId")]
    pub line_id: Option<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderGrantedRefund")]
pub struct OrderGrantedRefund {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,

    #[graphql(name = "amount")]
    pub amount: Option<crate::common::Money>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<Page>,

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "app")]
    pub app: Option<App>,

    #[graphql(name = "shippingCostsIncluded")]
    pub shipping_costs_included: Option<bool>,

    #[graphql(name = "lines")]
    pub lines: Vec<OrderGrantedRefundLine>,

    #[graphql(name = "status")]
    pub status: Option<String>,

    #[graphql(name = "transactionEvents")]
    pub transaction_events: Vec<TransactionEvent>,

    #[graphql(name = "transaction")]
    pub transaction: Option<TransactionItem>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderGrantedRefundLine")]
pub struct OrderGrantedRefundLine {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "quantity")]
    pub quantity: Option<i32>,

    #[graphql(name = "orderLine")]
    pub order_line: Option<OrderLine>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<Page>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderLine", complex)]
pub struct OrderLine {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "productName")]
    pub product_name: Option<String>,

    #[graphql(name = "variantName")]
    pub variant_name: Option<String>,

    #[graphql(name = "productSku")]
    pub product_sku: Option<String>,

    #[graphql(name = "isShippingRequired")]
    pub is_shipping_required: Option<bool>,

    #[graphql(name = "quantity")]
    pub quantity: Option<i32>,

    #[graphql(name = "quantityFulfilled")]
    pub quantity_fulfilled: Option<i32>,

    #[graphql(name = "taxRate")]
    pub tax_rate: Option<f64>,

    #[graphql(name = "unitPrice")]
    pub unit_price: Option<crate::order::GqlTaxedMoney>,

    #[graphql(name = "undiscountedUnitPrice")]
    pub undiscounted_unit_price: Option<crate::order::GqlTaxedMoney>,

    #[graphql(name = "unitDiscount")]
    pub unit_discount: Option<crate::common::Money>,

    #[graphql(name = "unitDiscountReason")]
    pub unit_discount_reason: Option<String>,

    #[graphql(name = "unitDiscountValue")]
    pub unit_discount_value: Option<GenPositiveDecimal>,

    #[graphql(name = "unitDiscountType")]
    pub unit_discount_type: Option<String>,

    #[graphql(name = "totalPrice")]
    pub total_price: Option<crate::order::GqlTaxedMoney>,

    #[graphql(name = "undiscountedTotalPrice")]
    pub undiscounted_total_price: Option<crate::order::GqlTaxedMoney>,

    #[graphql(name = "isPriceOverridden")]
    pub is_price_overridden: Option<bool>,

    #[graphql(name = "priceOverrideReason")]
    pub price_override_reason: Option<String>,

    #[graphql(name = "variant")]
    pub variant: Option<ProductVariant>,

    #[graphql(name = "allocations")]
    pub allocations: Vec<Allocation>,

    #[graphql(name = "quantityToFulfill")]
    pub quantity_to_fulfill: Option<i32>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<TaxClass>,

    #[graphql(name = "voucherCode")]
    pub voucher_code: Option<String>,

    #[graphql(name = "isGift")]
    pub is_gift: Option<bool>,

    #[graphql(name = "discounts")]
    pub discounts: Vec<OrderLineDiscount>,

}


#[ComplexObject]
impl OrderLine {

    #[graphql(name = "thumbnail")]
    async fn thumbnail(&self, ctx: &Context<'_>, #[graphql(name = "size")] _arg_size: Option<i32>, #[graphql(name = "format")] _arg_format: Option<ThumbnailFormatEnum>) -> Option<crate::account::GqlImage> {

        {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let line_id = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        // OrderLine ids are Saleor globals ("OrderLine:<uuid>") or bare UUIDs.
        let variant_id: Option<i32> = match crate::common::parse_uuid_gid(&line_id) {
            Some(u) => rustygod_db::entities::order_orderline::Entity::find()
                .select_only()
                .column(rustygod_db::entities::order_orderline::Column::VariantId)
                .filter(rustygod_db::entities::order_orderline::Column::Id.eq(u))
                .into_tuple::<Option<i32>>()
                .one(&db)
                .await
                .unwrap_or(None)
                .flatten(),
            None => None,
        };
        let pid: i32 = match variant_id {
            Some(vid) => rustygod_db::entities::product_productvariant::Entity::find_by_id(vid)
                .one(&db)
                .await
                .unwrap_or(None)
                .map(|v| v.product_id)
                .unwrap_or(-1),
            None => -1,
        };
        let img: Option<(String, String)> = rustygod_db::entities::product_productmedia::Entity::find()
            .select_only()
            .column(rustygod_db::entities::product_productmedia::Column::Image)
            .column(rustygod_db::entities::product_productmedia::Column::Alt)
            .filter(rustygod_db::entities::product_productmedia::Column::ProductId.eq(pid))
            .order_by_asc(rustygod_db::entities::product_productmedia::Column::SortOrder)
            .into_tuple()
            .all(&db)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(p, alt): (Option<String>, String)| p.map(|path| (path, alt)))
            .next();
        img.map(|(path, alt)| crate::account::GqlImage {
            url: crate::common::media_url(&path),
            alt: Some(alt),
        })
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderLineDelete")]
pub struct OrderLineDelete {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderLineDiscount")]
pub struct OrderLineDiscount {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "translatedName")]
    pub translated_name: Option<String>,

    #[graphql(name = "valueType")]
    pub value_type: Option<String>,

    #[graphql(name = "value")]
    pub value: Option<GenPositiveDecimal>,

    #[graphql(name = "reason")]
    pub reason: Option<String>,

    #[graphql(name = "total")]
    pub total: Option<crate::common::Money>,

    #[graphql(name = "unit")]
    pub unit: Option<crate::common::Money>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderLineDiscountRemove")]
pub struct OrderLineDiscountRemove {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderLineDiscountUpdate")]
pub struct OrderLineDiscountUpdate {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderLineUpdate")]
pub struct OrderLineUpdate {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

    #[graphql(name = "orderLine")]
    pub order_line: Option<OrderLine>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderLinesCreate")]
pub struct OrderLinesCreate {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderMarkAsPaid")]
pub struct OrderMarkAsPaid {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderNoteAdd")]
pub struct OrderNoteAdd {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderNoteAddError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderNoteAddError")]
pub struct OrderNoteAddError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderNoteUpdate")]
pub struct OrderNoteUpdate {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderNoteUpdateError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderNoteUpdateError")]
pub struct OrderNoteUpdateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderRefund")]
pub struct OrderRefund {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderSettings")]
pub struct OrderSettings {

    #[graphql(name = "automaticallyConfirmAllNewOrders")]
    pub automatically_confirm_all_new_orders: Option<bool>,

    #[graphql(name = "automaticallyFulfillNonShippableGiftCard")]
    pub automatically_fulfill_non_shippable_gift_card: Option<bool>,

    #[graphql(name = "expireOrdersAfter")]
    pub expire_orders_after: Option<i32>,

    #[graphql(name = "markAsPaidStrategy")]
    pub mark_as_paid_strategy: Option<String>,

    #[graphql(name = "deleteExpiredOrdersAfter")]
    pub delete_expired_orders_after: Option<i32>,

    #[graphql(name = "allowUnpaidOrders")]
    pub allow_unpaid_orders: Option<bool>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderUpdate")]
pub struct OrderUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

    #[graphql(name = "order")]
    pub order: Option<Order>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderUpdateShipping")]
pub struct OrderUpdateShipping {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OrderVoid")]
pub struct OrderVoid {

    #[graphql(name = "order")]
    pub order: Option<Order>,

    #[graphql(name = "errors")]
    pub errors: Vec<OrderError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "OtherPaymentMethodDetails", complex)]
pub struct OtherPaymentMethodDetails {

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[ComplexObject]
impl OtherPaymentMethodDetails {

    pub async fn gen_iface_name(&self) -> Option<String> {

        self.name.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Page", complex)]
pub struct Page {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "seoTitle")]
    pub seo_title: Option<String>,

    #[graphql(name = "seoDescription")]
    pub seo_description: Option<String>,

    #[graphql(name = "title")]
    pub title: Option<String>,

    #[graphql(name = "content")]
    pub content: Option<GenJSONString>,

    #[graphql(name = "publishedAt")]
    pub published_at: Option<DateTime<Utc>>,

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "pageType")]
    pub page_type: Option<PageType>,

    #[graphql(name = "attributes")]
    pub attributes: Vec<SelectedAttribute>,

}


#[ComplexObject]
impl Page {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageAttributeAssign")]
pub struct PageAttributeAssign {

    #[graphql(name = "pageType")]
    pub page_type: Option<PageType>,

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageAttributeUnassign")]
pub struct PageAttributeUnassign {

    #[graphql(name = "pageType")]
    pub page_type: Option<PageType>,

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageBulkDelete")]
pub struct PageBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageBulkPublish")]
pub struct PageBulkPublish {

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageCountableConnection")]
pub struct PageCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<PageCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageCountableEdge")]
pub struct PageCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Page>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageCreate")]
pub struct PageCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

    #[graphql(name = "page")]
    pub page: Option<Page>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageDelete")]
pub struct PageDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageError")]
pub struct PageError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "attributes")]
    pub attributes: Vec<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageTranslatableContent", complex)]
pub struct PageTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "page")]
    pub page: Option<Page>,

    #[graphql(name = "attributeValues")]
    pub attribute_values: Vec<AttributeValueTranslatableContent>,

}


#[ComplexObject]
impl PageTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<PageTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.page.as_ref().and_then(|e| e.id.as_ref()).and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::page_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageTranslate")]
pub struct PageTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "page")]
    pub page: Option<PageTranslatableContent>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageTranslation")]
pub struct PageTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "language")]
    pub language: Option<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "seoTitle")]
    pub seo_title: Option<String>,

    #[graphql(name = "seoDescription")]
    pub seo_description: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "title")]
    pub title: Option<String>,

    #[graphql(name = "content")]
    pub content: Option<GenJSONString>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageType", complex)]
pub struct PageType {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "attributes")]
    pub attributes: Vec<Box<Attribute>>,

    #[graphql(name = "hasPages")]
    pub has_pages: Option<bool>,

}


#[ComplexObject]
impl PageType {

    #[graphql(name = "availableAttributes")]
    async fn available_attributes(&self, #[graphql(name = "filter")] _arg_filter: Option<AttributeFilterInput>, #[graphql(name = "where")] _arg_where: Option<AttributeWhereInput>, #[graphql(name = "search")] _arg_search: Option<String>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<Box<AttributeCountableConnection>> {

        None

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageTypeBulkDelete")]
pub struct PageTypeBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageTypeCountableConnection")]
pub struct PageTypeCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<PageTypeCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageTypeCountableEdge")]
pub struct PageTypeCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<PageType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageTypeCreate")]
pub struct PageTypeCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

    #[graphql(name = "pageType")]
    pub page_type: Option<PageType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageTypeDelete")]
pub struct PageTypeDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

    #[graphql(name = "pageType")]
    pub page_type: Option<PageType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageTypeReorderAttributes")]
pub struct PageTypeReorderAttributes {

    #[graphql(name = "pageType")]
    pub page_type: Option<PageType>,

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageTypeUpdate")]
pub struct PageTypeUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

    #[graphql(name = "pageType")]
    pub page_type: Option<PageType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PageUpdate")]
pub struct PageUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<PageError>,

    #[graphql(name = "page")]
    pub page: Option<Page>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PasswordChange")]
pub struct PasswordChange {

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Payment", complex)]
pub struct Payment {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "gateway")]
    pub gateway: Option<String>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "modified")]
    pub modified: Option<DateTime<Utc>>,

    #[graphql(name = "paymentMethodType")]
    pub payment_method_type: Option<String>,

    #[graphql(name = "actions")]
    pub actions: Vec<String>,

    #[graphql(name = "total")]
    pub total: Option<crate::common::Money>,

    #[graphql(name = "capturedAmount")]
    pub captured_amount: Option<crate::common::Money>,

    #[graphql(name = "transactions")]
    pub transactions: Vec<Transaction>,

    #[graphql(name = "availableCaptureAmount")]
    pub available_capture_amount: Option<crate::common::Money>,

    #[graphql(name = "availableRefundAmount")]
    pub available_refund_amount: Option<crate::common::Money>,

}


#[ComplexObject]
impl Payment {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentGateway")]
pub struct PaymentGateway {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "id")]
    pub id: Option<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PaymentSettings")]
pub struct PaymentSettings {

    #[graphql(name = "defaultTransactionFlowStrategy")]
    pub default_transaction_flow_strategy: Option<String>,

    #[graphql(name = "releaseFundsForExpiredCheckouts")]
    pub release_funds_for_expired_checkouts: Option<bool>,

    #[graphql(name = "checkoutTtlBeforeReleasingFunds")]
    pub checkout_ttl_before_releasing_funds: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PermissionGroupCreate")]
pub struct PermissionGroupCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<PermissionGroupError>,

    #[graphql(name = "group")]
    pub group: Option<Group>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PermissionGroupDelete")]
pub struct PermissionGroupDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<PermissionGroupError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PermissionGroupError")]
pub struct PermissionGroupError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PermissionGroupUpdate")]
pub struct PermissionGroupUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<PermissionGroupError>,

    #[graphql(name = "group")]
    pub group: Option<Group>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Plugin")]
pub struct Plugin {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,

    #[graphql(name = "globalConfiguration")]
    pub global_configuration: Option<PluginConfiguration>,

    #[graphql(name = "channelConfigurations")]
    pub channel_configurations: Vec<PluginConfiguration>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PluginConfiguration")]
pub struct PluginConfiguration {

    #[graphql(name = "active")]
    pub active: Option<bool>,

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

    #[graphql(name = "configuration")]
    pub configuration: Vec<ConfigurationItem>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PluginCountableConnection")]
pub struct PluginCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<PluginCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PluginCountableEdge")]
pub struct PluginCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Plugin>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PluginError")]
pub struct PluginError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PluginUpdate")]
pub struct PluginUpdate {

    #[graphql(name = "plugin")]
    pub plugin: Option<Plugin>,

    #[graphql(name = "errors")]
    pub errors: Vec<PluginError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Product", complex)]
pub struct Product {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "seoTitle")]
    pub seo_title: Option<String>,

    #[graphql(name = "seoDescription")]
    pub seo_description: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "productType")]
    pub product_type: Option<ProductType>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "category")]
    pub category: Option<Category>,

    #[graphql(name = "created")]
    pub created: Option<DateTime<Utc>>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,

    #[graphql(name = "weight")]
    pub weight: Option<Weight>,

    #[graphql(name = "defaultVariant")]
    pub default_variant: Option<ProductVariant>,

    #[graphql(name = "rating")]
    pub rating: Option<f64>,

    #[graphql(name = "isAvailable")]
    pub is_available: Option<bool>,

    #[graphql(name = "attributes")]
    pub attributes: Vec<SelectedAttribute>,

    #[graphql(name = "channelListings")]
    pub channel_listings: Vec<ProductChannelListing>,

    #[graphql(name = "media")]
    pub media: Vec<ProductMedia>,

    #[graphql(name = "collections")]
    pub collections: Vec<Collection>,

    #[graphql(name = "availableForPurchaseAt")]
    pub available_for_purchase_at: Option<DateTime<Utc>>,

    #[graphql(name = "isAvailableForPurchase")]
    pub is_available_for_purchase: Option<bool>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<TaxClass>,

}


#[ComplexObject]
impl Product {

    #[graphql(name = "thumbnail")]
    async fn thumbnail(&self, ctx: &Context<'_>, #[graphql(name = "size")] _arg_size: Option<i32>, #[graphql(name = "format")] _arg_format: Option<ThumbnailFormatEnum>) -> Option<crate::account::GqlImage> {

        {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
        let pid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let img: Option<(String, String)> = rustygod_db::entities::product_productmedia::Entity::find()
            .select_only()
            .column(rustygod_db::entities::product_productmedia::Column::Image)
            .column(rustygod_db::entities::product_productmedia::Column::Alt)
            .filter(rustygod_db::entities::product_productmedia::Column::ProductId.eq(pid))
            .order_by_asc(rustygod_db::entities::product_productmedia::Column::SortOrder)
            .into_tuple()
            .all(&db)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(p, alt): (Option<String>, String)| p.map(|path| (path, alt)))
            .next();
        img.map(|(path, alt)| crate::account::GqlImage {
            url: crate::common::media_url(&path),
            alt: Some(alt),
        })
    }

    }

    #[graphql(name = "mediaById")]
    async fn media_by_id(&self, ctx: &Context<'_>, #[graphql(name = "id")] _arg_id: ID) -> Option<ProductMedia> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        crate::catalog::media_by_id(&db, &_arg_id.0).await.unwrap_or(None)
    }

    }

    #[graphql(name = "productVariants")]
    async fn product_variants(&self, ctx: &Context<'_>, #[graphql(name = "filter")] _arg_filter: Option<ProductVariantFilterInput>, #[graphql(name = "where")] _arg_where: Option<ProductVariantWhereInput>, #[graphql(name = "sortBy")] _arg_sort_by: Option<ProductVariantSortingInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<ProductVariantCountableConnection> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let pid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let search = _arg_filter.as_ref().and_then(|f| f.search.clone());
        crate::catalog::product_variants_page(&db, pid, search, _arg_first.clone(), _arg_after.clone()).await.unwrap_or(None)
    }

    }

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<ProductTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::product_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductAttributeAssign")]
pub struct ProductAttributeAssign {

    #[graphql(name = "productType")]
    pub product_type: Option<ProductType>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductAttributeAssignmentUpdate")]
pub struct ProductAttributeAssignmentUpdate {

    #[graphql(name = "productType")]
    pub product_type: Option<ProductType>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductAttributeUnassign")]
pub struct ProductAttributeUnassign {

    #[graphql(name = "productType")]
    pub product_type: Option<ProductType>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductBulkDelete")]
pub struct ProductBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductChannelListing", complex)]
pub struct ProductChannelListing {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "publishedAt")]
    pub published_at: Option<DateTime<Utc>>,

    #[graphql(name = "isPublished")]
    pub is_published: Option<bool>,

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

    #[graphql(name = "visibleInListings")]
    pub visible_in_listings: Option<bool>,

    #[graphql(name = "availableForPurchaseAt")]
    pub available_for_purchase_at: Option<DateTime<Utc>>,

    #[graphql(name = "isAvailableForPurchase")]
    pub is_available_for_purchase: Option<bool>,

    #[graphql(name = "pricing")]
    pub pricing: Option<ProductPricingInfo>,

}


#[ComplexObject]
impl ProductChannelListing {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductChannelListingError")]
pub struct ProductChannelListingError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "channels")]
    pub channels: Vec<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductChannelListingUpdate")]
pub struct ProductChannelListingUpdate {

    #[graphql(name = "product")]
    pub product: Option<Product>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductChannelListingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductCountableEdge")]
pub struct ProductCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Product>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductCreate")]
pub struct ProductCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

    #[graphql(name = "product")]
    pub product: Option<Product>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductDelete")]
pub struct ProductDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductError")]
pub struct ProductError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "attributes")]
    pub attributes: Vec<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductMedia", complex)]
pub struct ProductMedia {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "sortOrder")]
    pub sort_order: Option<i32>,

    #[graphql(name = "alt")]
    pub alt: Option<String>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "oembedData")]
    pub oembed_data: Option<GenJSONString>,

}


#[ComplexObject]
impl ProductMedia {

    #[graphql(name = "url")]
    async fn url(&self, ctx: &Context<'_>, #[graphql(name = "size")] _arg_size: Option<i32>, #[graphql(name = "format")] _arg_format: Option<ThumbnailFormatEnum>) -> Option<String> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let mid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        use sea_orm::{EntityTrait, QuerySelect};
        let path: Option<String> = rustygod_db::entities::product_productmedia::Entity::find_by_id(mid)
            .select_only().column(rustygod_db::entities::product_productmedia::Column::Image)
            .into_tuple::<Option<String>>().one(&db).await.unwrap_or(None).flatten();
        path.map(|pp| crate::common::media_url(&pp))
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductMediaBulkDelete")]
pub struct ProductMediaBulkDelete {

    #[graphql(name = "count")]
    pub count: Option<i32>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductMediaCreate")]
pub struct ProductMediaCreate {

    #[graphql(name = "product")]
    pub product: Option<Product>,

    #[graphql(name = "media")]
    pub media: Option<ProductMedia>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductMediaDelete")]
pub struct ProductMediaDelete {

    #[graphql(name = "product")]
    pub product: Option<Product>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductMediaReorder")]
pub struct ProductMediaReorder {

    #[graphql(name = "product")]
    pub product: Option<Product>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductMediaUpdate")]
pub struct ProductMediaUpdate {

    #[graphql(name = "product")]
    pub product: Option<Product>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductPricingInfo")]
pub struct ProductPricingInfo {

    #[graphql(name = "priceRange")]
    pub price_range: Option<TaxedMoneyRange>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductTranslatableContent", complex)]
pub struct ProductTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "product")]
    pub product: Option<Product>,

    #[graphql(name = "attributeValues")]
    pub attribute_values: Vec<AttributeValueTranslatableContent>,

}


#[ComplexObject]
impl ProductTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<ProductTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.product.as_ref().and_then(|e| e.id.as_ref()).and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::product_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductTranslate")]
pub struct ProductTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "product")]
    pub product: Option<Product>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductTranslation")]
pub struct ProductTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "language")]
    pub language: Option<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "seoTitle")]
    pub seo_title: Option<String>,

    #[graphql(name = "seoDescription")]
    pub seo_description: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductType", complex)]
pub struct ProductType {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "hasVariants")]
    pub has_variants: Option<bool>,

    #[graphql(name = "isShippingRequired")]
    pub is_shipping_required: Option<bool>,

    #[graphql(name = "weight")]
    pub weight: Option<Weight>,

    #[graphql(name = "kind")]
    pub kind: Option<String>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<TaxClass>,

    #[graphql(name = "assignedVariantAttributes")]
    pub assigned_variant_attributes: Vec<AssignedVariantAttribute>,

    #[graphql(name = "productAttributes")]
    pub product_attributes: Vec<Attribute>,

}


#[ComplexObject]
impl ProductType {

    #[graphql(name = "variantAttributes")]
    async fn variant_attributes(&self, #[graphql(name = "variantSelection")] _arg_variant_selection: Option<VariantAttributeScope>) -> Vec<Attribute> {

        vec![]

    }

    #[graphql(name = "availableAttributes")]
    async fn available_attributes(&self, #[graphql(name = "filter")] _arg_filter: Option<AttributeFilterInput>, #[graphql(name = "where")] _arg_where: Option<AttributeWhereInput>, #[graphql(name = "search")] _arg_search: Option<String>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<Box<AttributeCountableConnection>> {

        None

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductTypeBulkDelete")]
pub struct ProductTypeBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductTypeCountableConnection")]
pub struct ProductTypeCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<ProductTypeCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductTypeCountableEdge")]
pub struct ProductTypeCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Box<ProductType>>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductTypeCreate")]
pub struct ProductTypeCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

    #[graphql(name = "productType")]
    pub product_type: Option<ProductType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductTypeDelete")]
pub struct ProductTypeDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

    #[graphql(name = "productType")]
    pub product_type: Option<ProductType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductTypeReorderAttributes")]
pub struct ProductTypeReorderAttributes {

    #[graphql(name = "productType")]
    pub product_type: Option<ProductType>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductTypeUpdate")]
pub struct ProductTypeUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

    #[graphql(name = "productType")]
    pub product_type: Option<ProductType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductUpdate")]
pub struct ProductUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariant", complex)]
pub struct ProductVariant {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "sku")]
    pub sku: Option<String>,

    #[graphql(name = "product")]
    pub product: Option<Box<Product>>,

    #[graphql(name = "trackInventory")]
    pub track_inventory: Option<bool>,

    #[graphql(name = "quantityLimitPerCustomer")]
    pub quantity_limit_per_customer: Option<i32>,

    #[graphql(name = "weight")]
    pub weight: Option<Weight>,

    #[graphql(name = "channelListings")]
    pub channel_listings: Vec<ProductVariantChannelListing>,

    #[graphql(name = "media")]
    pub media: Vec<ProductMedia>,

    #[graphql(name = "stocks")]
    pub stocks: Vec<Stock>,

    #[graphql(name = "quantityAvailable")]
    pub quantity_available: Option<i32>,

    #[graphql(name = "updatedAt")]
    pub updated_at: Option<DateTime<Utc>>,

}


#[ComplexObject]
impl ProductVariant {

    #[graphql(name = "pricing")]
    async fn pricing(&self, #[graphql(name = "address")] _arg_address: Option<AddressInput>) -> Option<VariantPricingInfo> {

        None

    }

    #[graphql(name = "attributes")]
    async fn attributes(&self, ctx: &Context<'_>, #[graphql(name = "variantSelection")] _arg_variant_selection: Option<VariantAttributeScope>) -> Vec<SelectedAttribute> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return vec![],
        };
        let gid = self.id.as_ref().map(|i| i.0.clone()).unwrap_or_default();
        crate::catalog::variant_attributes(&db, &gid).await
    }

    }

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<ProductVariantTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::product_variant_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantBulkCreate")]
pub struct ProductVariantBulkCreate {

    #[graphql(name = "productVariants")]
    pub product_variants: Vec<ProductVariant>,

    #[graphql(name = "results")]
    pub results: Vec<ProductVariantBulkResult>,

    #[graphql(name = "errors")]
    pub errors: Vec<BulkProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantBulkDelete")]
pub struct ProductVariantBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantBulkError")]
pub struct ProductVariantBulkError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "attributes")]
    pub attributes: Vec<ID>,

    #[graphql(name = "values")]
    pub values: Vec<ID>,

    #[graphql(name = "warehouses")]
    pub warehouses: Vec<ID>,

    #[graphql(name = "channels")]
    pub channels: Vec<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantBulkResult")]
pub struct ProductVariantBulkResult {

    #[graphql(name = "productVariant")]
    pub product_variant: Option<ProductVariant>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductVariantBulkError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantBulkUpdate")]
pub struct ProductVariantBulkUpdate {

    #[graphql(name = "results")]
    pub results: Vec<ProductVariantBulkResult>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductVariantBulkError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantChannelListing", complex)]
pub struct ProductVariantChannelListing {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

    #[graphql(name = "price")]
    pub price: Option<crate::common::Money>,

    #[graphql(name = "costPrice")]
    pub cost_price: Option<crate::common::Money>,

}


#[ComplexObject]
impl ProductVariantChannelListing {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantChannelListingUpdate")]
pub struct ProductVariantChannelListingUpdate {

    #[graphql(name = "variant")]
    pub variant: Option<ProductVariant>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductChannelListingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantCountableConnection")]
pub struct ProductVariantCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<ProductVariantCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantCountableEdge")]
pub struct ProductVariantCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<ProductVariant>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantCreate")]
pub struct ProductVariantCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

    #[graphql(name = "productVariant")]
    pub product_variant: Option<ProductVariant>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantDelete")]
pub struct ProductVariantDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantReorder")]
pub struct ProductVariantReorder {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantSetDefault")]
pub struct ProductVariantSetDefault {

    #[graphql(name = "product")]
    pub product: Option<Product>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantStocksCreate")]
pub struct ProductVariantStocksCreate {

    #[graphql(name = "productVariant")]
    pub product_variant: Option<ProductVariant>,

    #[graphql(name = "errors")]
    pub errors: Vec<BulkStockError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantStocksDelete")]
pub struct ProductVariantStocksDelete {

    #[graphql(name = "productVariant")]
    pub product_variant: Option<ProductVariant>,

    #[graphql(name = "errors")]
    pub errors: Vec<StockError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantStocksUpdate")]
pub struct ProductVariantStocksUpdate {

    #[graphql(name = "productVariant")]
    pub product_variant: Option<ProductVariant>,

    #[graphql(name = "errors")]
    pub errors: Vec<BulkStockError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantTranslatableContent", complex)]
pub struct ProductVariantTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "productVariant")]
    pub product_variant: Option<ProductVariant>,

    #[graphql(name = "attributeValues")]
    pub attribute_values: Vec<AttributeValueTranslatableContent>,

}


#[ComplexObject]
impl ProductVariantTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<ProductVariantTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.product_variant.as_ref().and_then(|e| e.id.as_ref()).and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::product_variant_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantTranslate")]
pub struct ProductVariantTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "productVariant")]
    pub product_variant: Option<ProductVariant>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantTranslation")]
pub struct ProductVariantTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "language")]
    pub language: Option<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ProductVariantUpdate")]
pub struct ProductVariantUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

    #[graphql(name = "productVariant")]
    pub product_variant: Option<ProductVariant>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Promotion", complex)]
pub struct Promotion {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<serde_json::Value>,

    #[graphql(name = "startDate")]
    pub start_date: Option<DateTime<Utc>>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTime<Utc>>,

    #[graphql(name = "rules")]
    pub rules: Vec<PromotionRule>,

}


#[ComplexObject]
impl Promotion {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionCountableConnection")]
pub struct PromotionCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<PromotionCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionCountableEdge")]
pub struct PromotionCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Promotion>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionCreate")]
pub struct PromotionCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<PromotionCreateError>,

    #[graphql(name = "promotion")]
    pub promotion: Option<Promotion>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionCreateError")]
pub struct PromotionCreateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "index")]
    pub index: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionDelete")]
pub struct PromotionDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<PromotionDeleteError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionDeleteError")]
pub struct PromotionDeleteError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionRule", complex)]
pub struct PromotionRule {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<serde_json::Value>,

    #[graphql(name = "channels")]
    pub channels: Vec<Channel>,

    #[graphql(name = "rewardValue")]
    pub reward_value: Option<GenPositiveDecimal>,

    #[graphql(name = "rewardValueType")]
    pub reward_value_type: Option<String>,

    #[graphql(name = "cataloguePredicate")]
    pub catalogue_predicate: Option<serde_json::Value>,

    #[graphql(name = "orderPredicate")]
    pub order_predicate: Option<serde_json::Value>,

    #[graphql(name = "rewardType")]
    pub reward_type: Option<String>,

    #[graphql(name = "giftIds")]
    pub gift_ids: Vec<ID>,

}


#[ComplexObject]
impl PromotionRule {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionRuleCreate")]
pub struct PromotionRuleCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<PromotionRuleCreateError>,

    #[graphql(name = "promotionRule")]
    pub promotion_rule: Option<PromotionRule>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionRuleCreateError")]
pub struct PromotionRuleCreateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionRuleDelete")]
pub struct PromotionRuleDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<PromotionRuleDeleteError>,

    #[graphql(name = "promotionRule")]
    pub promotion_rule: Option<PromotionRule>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionRuleDeleteError")]
pub struct PromotionRuleDeleteError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionRuleUpdate")]
pub struct PromotionRuleUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<PromotionRuleUpdateError>,

    #[graphql(name = "promotionRule")]
    pub promotion_rule: Option<PromotionRule>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionRuleUpdateError")]
pub struct PromotionRuleUpdateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "channels")]
    pub channels: Vec<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionUpdate")]
pub struct PromotionUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<PromotionUpdateError>,

    #[graphql(name = "promotion")]
    pub promotion: Option<Promotion>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "PromotionUpdateError")]
pub struct PromotionUpdateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "RefreshToken")]
pub struct RefreshToken {

    #[graphql(name = "token")]
    pub token: Option<String>,

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "RefundReasonReferenceTypeClear")]
pub struct RefundReasonReferenceTypeClear {

    #[graphql(name = "errors")]
    pub errors: Vec<RefundReasonReferenceTypeClearError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "RefundReasonReferenceTypeClearError")]
pub struct RefundReasonReferenceTypeClearError {

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "RefundSettings")]
pub struct RefundSettings {

    #[graphql(name = "reasonReferenceType")]
    pub reason_reference_type: Option<PageType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "RefundSettingsUpdate")]
pub struct RefundSettingsUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<RefundSettingsUpdateError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "RefundSettingsUpdateError")]
pub struct RefundSettingsUpdateError {

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "RequestPasswordReset")]
pub struct RequestPasswordReset {

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ReturnReasonReferenceTypeClear")]
pub struct ReturnReasonReferenceTypeClear {

    #[graphql(name = "errors")]
    pub errors: Vec<ReturnReasonReferenceTypeClearError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ReturnReasonReferenceTypeClearError")]
pub struct ReturnReasonReferenceTypeClearError {

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ReturnSettings")]
pub struct ReturnSettings {

    #[graphql(name = "reasonReferenceType")]
    pub reason_reference_type: Option<PageType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ReturnSettingsUpdate")]
pub struct ReturnSettingsUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<ReturnSettingsUpdateError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ReturnSettingsUpdateError")]
pub struct ReturnSettingsUpdateError {

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Sale", complex)]
pub struct Sale {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "startDate")]
    pub start_date: Option<DateTime<Utc>>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTime<Utc>>,

    #[graphql(name = "channelListings")]
    pub channel_listings: Vec<SaleChannelListing>,

}


#[ComplexObject]
impl Sale {

    #[graphql(name = "categories")]
    async fn categories(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<CategoryCountableConnection> {

        None

    }

    #[graphql(name = "collections")]
    async fn collections(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<CollectionCountableConnection> {

        None

    }

    #[graphql(name = "products")]
    async fn products(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<crate::catalog::GqlProductConnection> {

        None

    }

    #[graphql(name = "variants")]
    async fn variants(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<ProductVariantCountableConnection> {

        None

    }

    #[graphql(name = "translation")]
    async fn translation(&self, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<SaleTranslation> {

        None

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "SaleChannelListing")]
pub struct SaleChannelListing {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

    #[graphql(name = "discountValue")]
    pub discount_value: Option<f64>,

    #[graphql(name = "currency")]
    pub currency: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "SaleCountableConnection")]
pub struct SaleCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<SaleCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "SaleCountableEdge")]
pub struct SaleCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Sale>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "SaleTranslatableContent", complex)]
pub struct SaleTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "sale")]
    pub sale: Option<Sale>,

}


#[ComplexObject]
impl SaleTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<SaleTranslation> {

        None

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "SaleTranslate")]
pub struct SaleTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "sale")]
    pub sale: Option<Sale>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "SaleTranslation")]
pub struct SaleTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "language")]
    pub language: Option<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "SelectedAttribute")]
pub struct SelectedAttribute {

    #[graphql(name = "attribute")]
    pub attribute: Option<Attribute>,

    #[graphql(name = "values")]
    pub values: Vec<AttributeValue>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "SetPassword")]
pub struct SetPassword {

    #[graphql(name = "token")]
    pub token: Option<String>,

    #[graphql(name = "refreshToken")]
    pub refresh_token: Option<String>,

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingError")]
pub struct ShippingError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "channels")]
    pub channels: Vec<ID>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingMethod", complex)]
pub struct ShippingMethod {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "price")]
    pub price: Option<crate::common::Money>,

    #[graphql(name = "active")]
    pub active: Option<bool>,

    #[graphql(name = "message")]
    pub message: Option<String>,

}


#[ComplexObject]
impl ShippingMethod {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingMethodChannelListing")]
pub struct ShippingMethodChannelListing {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "channel")]
    pub channel: Option<Box<Channel>>,

    #[graphql(name = "maximumOrderPrice")]
    pub maximum_order_price: Option<crate::common::Money>,

    #[graphql(name = "minimumOrderPrice")]
    pub minimum_order_price: Option<crate::common::Money>,

    #[graphql(name = "price")]
    pub price: Option<crate::common::Money>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingMethodChannelListingUpdate")]
pub struct ShippingMethodChannelListingUpdate {

    #[graphql(name = "shippingMethod")]
    pub shipping_method: Option<ShippingMethodType>,

    #[graphql(name = "errors")]
    pub errors: Vec<ShippingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingMethodPostalCodeRule")]
pub struct ShippingMethodPostalCodeRule {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "start")]
    pub start: Option<String>,

    #[graphql(name = "end")]
    pub end: Option<String>,

    #[graphql(name = "inclusionType")]
    pub inclusion_type: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingMethodTranslatableContent", complex)]
pub struct ShippingMethodTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "shippingMethodId")]
    pub shipping_method_id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "shippingMethod")]
    pub shipping_method: Option<ShippingMethodType>,

}


#[ComplexObject]
impl ShippingMethodTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<ShippingMethodTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.shipping_method.as_ref().and_then(|e| e.id.as_ref()).and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::shipping_method_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingMethodTranslation")]
pub struct ShippingMethodTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "language")]
    pub language: Option<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingMethodType", complex)]
pub struct ShippingMethodType {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<GenJSONString>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "channelListings")]
    pub channel_listings: Vec<ShippingMethodChannelListing>,

    #[graphql(name = "postalCodeRules")]
    pub postal_code_rules: Vec<ShippingMethodPostalCodeRule>,

    #[graphql(name = "minimumOrderWeight")]
    pub minimum_order_weight: Option<Weight>,

    #[graphql(name = "maximumOrderWeight")]
    pub maximum_order_weight: Option<Weight>,

    #[graphql(name = "maximumDeliveryDays")]
    pub maximum_delivery_days: Option<i32>,

    #[graphql(name = "minimumDeliveryDays")]
    pub minimum_delivery_days: Option<i32>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<TaxClass>,

}


#[ComplexObject]
impl ShippingMethodType {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<ShippingMethodTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::shipping_method_translation(&db, eid, lang).await
    }

    }

    #[graphql(name = "excludedProducts")]
    async fn excluded_products(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<crate::catalog::GqlProductConnection> {

        None

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingPriceCreate")]
pub struct ShippingPriceCreate {

    #[graphql(name = "shippingZone")]
    pub shipping_zone: Option<ShippingZone>,

    #[graphql(name = "shippingMethod")]
    pub shipping_method: Option<ShippingMethodType>,

    #[graphql(name = "errors")]
    pub errors: Vec<ShippingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingPriceDelete")]
pub struct ShippingPriceDelete {

    #[graphql(name = "shippingZone")]
    pub shipping_zone: Option<ShippingZone>,

    #[graphql(name = "errors")]
    pub errors: Vec<ShippingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingPriceExcludeProducts")]
pub struct ShippingPriceExcludeProducts {

    #[graphql(name = "errors")]
    pub errors: Vec<ShippingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingPriceRemoveProductFromExclude")]
pub struct ShippingPriceRemoveProductFromExclude {

    #[graphql(name = "errors")]
    pub errors: Vec<ShippingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingPriceTranslate")]
pub struct ShippingPriceTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "shippingMethod")]
    pub shipping_method: Option<ShippingMethodType>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingPriceUpdate")]
pub struct ShippingPriceUpdate {

    #[graphql(name = "shippingMethod")]
    pub shipping_method: Option<ShippingMethodType>,

    #[graphql(name = "errors")]
    pub errors: Vec<ShippingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingZone", complex)]
pub struct ShippingZone {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "default")]
    pub default: Option<bool>,

    #[graphql(name = "priceRange")]
    pub price_range: Option<MoneyRange>,

    #[graphql(name = "countries")]
    pub countries: Vec<crate::common::GqlCountryDisplay>,

    #[graphql(name = "shippingMethods")]
    pub shipping_methods: Vec<ShippingMethodType>,

    #[graphql(name = "warehouses")]
    pub warehouses: Vec<Box<Warehouse>>,

    #[graphql(name = "channels")]
    pub channels: Vec<Box<Channel>>,

    #[graphql(name = "description")]
    pub description: Option<String>,

}


#[ComplexObject]
impl ShippingZone {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingZoneBulkDelete")]
pub struct ShippingZoneBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ShippingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingZoneCountableConnection")]
pub struct ShippingZoneCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<ShippingZoneCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingZoneCountableEdge")]
pub struct ShippingZoneCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<ShippingZone>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingZoneCreate")]
pub struct ShippingZoneCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<ShippingError>,

    #[graphql(name = "shippingZone")]
    pub shipping_zone: Option<ShippingZone>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingZoneDelete")]
pub struct ShippingZoneDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ShippingError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShippingZoneUpdate")]
pub struct ShippingZoneUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<ShippingError>,

    #[graphql(name = "shippingZone")]
    pub shipping_zone: Option<ShippingZone>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Shop", complex)]
pub struct Shop {

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "availablePaymentGateways")]
    pub available_payment_gateways: Vec<PaymentGateway>,

    #[graphql(name = "availableExternalAuthentications")]
    pub available_external_authentications: Vec<ExternalAuthentication>,

    #[graphql(name = "channelCurrencies")]
    pub channel_currencies: Vec<String>,

    #[graphql(name = "defaultCountry")]
    pub default_country: Option<crate::common::GqlCountryDisplay>,

    #[graphql(name = "defaultMailSenderName")]
    pub default_mail_sender_name: Option<String>,

    #[graphql(name = "defaultMailSenderAddress")]
    pub default_mail_sender_address: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,

    #[graphql(name = "domain")]
    pub domain: Option<Domain>,

    #[graphql(name = "languages")]
    pub languages: Vec<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "permissions")]
    pub permissions: Vec<crate::commerce::GqlPermission>,

    #[graphql(name = "fulfillmentAutoApprove")]
    pub fulfillment_auto_approve: Option<bool>,

    #[graphql(name = "fulfillmentAllowUnpaid")]
    pub fulfillment_allow_unpaid: Option<bool>,

    #[graphql(name = "allowStorefrontTraffic")]
    pub allow_storefront_traffic: Option<bool>,

    #[graphql(name = "defaultWeightUnit")]
    pub default_weight_unit: Option<String>,

    #[graphql(name = "reserveStockDurationAnonymousUser")]
    pub reserve_stock_duration_anonymous_user: Option<i32>,

    #[graphql(name = "reserveStockDurationAuthenticatedUser")]
    pub reserve_stock_duration_authenticated_user: Option<i32>,

    #[graphql(name = "limitQuantityPerCheckout")]
    pub limit_quantity_per_checkout: Option<i32>,

    #[graphql(name = "companyAddress")]
    pub company_address: Option<crate::order::GqlAddress>,

    #[graphql(name = "customerSetPasswordUrl")]
    pub customer_set_password_url: Option<String>,

    #[graphql(name = "staffNotificationRecipients")]
    pub staff_notification_recipients: Vec<StaffNotificationRecipient>,

    #[graphql(name = "enableAccountConfirmationByEmail")]
    pub enable_account_confirmation_by_email: Option<bool>,

    #[graphql(name = "limits")]
    pub limits: Option<crate::commerce::GqlLimitInfo>,

    #[graphql(name = "announcements")]
    pub announcements: Vec<crate::commerce::GqlAnnouncement>,

    #[graphql(name = "version")]
    pub version: Option<String>,

    #[graphql(name = "availableTaxApps")]
    pub available_tax_apps: Vec<App>,

    #[graphql(name = "preserveAllAddressFields")]
    pub preserve_all_address_fields: Option<bool>,

    #[graphql(name = "passwordLoginMode")]
    pub password_login_mode: Option<String>,

    #[graphql(name = "useLegacyShippingZoneStockAvailability")]
    pub use_legacy_shipping_zone_stock_availability: Option<bool>,

    #[graphql(name = "useLegacyUpdateWebhookEmission")]
    pub use_legacy_update_webhook_emission: Option<bool>,

}


#[ComplexObject]
impl Shop {

    #[graphql(name = "countries")]
    async fn countries(&self, #[graphql(name = "languageCode")] _arg_language_code: Option<LanguageCodeEnum>, #[graphql(name = "filter")] _arg_filter: Option<CountryFilterInput>) -> Vec<crate::common::GqlCountryDisplay> {

        vec![]

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShopAddressUpdate")]
pub struct ShopAddressUpdate {

    #[graphql(name = "shop")]
    pub shop: Option<Shop>,

    #[graphql(name = "errors")]
    pub errors: Vec<ShopError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "ShopError")]
pub struct ShopError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "StaffCreate")]
pub struct StaffCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<StaffError>,

    #[graphql(name = "user")]
    pub user: Option<User>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "StaffDelete")]
pub struct StaffDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<StaffError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "StaffError")]
pub struct StaffError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "StaffNotificationRecipient")]
pub struct StaffNotificationRecipient {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "active")]
    pub active: Option<bool>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "StaffNotificationRecipientCreate")]
pub struct StaffNotificationRecipientCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<ShopError>,

    #[graphql(name = "staffNotificationRecipient")]
    pub staff_notification_recipient: Option<StaffNotificationRecipient>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "StaffNotificationRecipientDelete")]
pub struct StaffNotificationRecipientDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<ShopError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "StaffUpdate")]
pub struct StaffUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<StaffError>,

    #[graphql(name = "user")]
    pub user: Option<User>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Stock", complex)]
pub struct Stock {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "warehouse")]
    pub warehouse: Option<Warehouse>,

    #[graphql(name = "quantity")]
    pub quantity: Option<i32>,

    #[graphql(name = "quantityAllocated")]
    pub quantity_allocated: Option<i32>,

}


#[ComplexObject]
impl Stock {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "StockError")]
pub struct StockError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxClass", complex)]
pub struct TaxClass {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "countries")]
    pub countries: Vec<TaxClassCountryRate>,

}


#[ComplexObject]
impl TaxClass {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxClassCountableConnection")]
pub struct TaxClassCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<TaxClassCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxClassCountableEdge")]
pub struct TaxClassCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<TaxClass>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxClassCountryRate")]
pub struct TaxClassCountryRate {

    #[graphql(name = "country")]
    pub country: Option<crate::common::GqlCountryDisplay>,

    #[graphql(name = "rate")]
    pub rate: Option<f64>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<Box<TaxClass>>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxClassCreate")]
pub struct TaxClassCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<TaxClassCreateError>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<TaxClass>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxClassCreateError")]
pub struct TaxClassCreateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxClassDelete")]
pub struct TaxClassDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<TaxClassDeleteError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxClassDeleteError")]
pub struct TaxClassDeleteError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxClassUpdate")]
pub struct TaxClassUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<TaxClassUpdateError>,

    #[graphql(name = "taxClass")]
    pub tax_class: Option<TaxClass>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxClassUpdateError")]
pub struct TaxClassUpdateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxConfiguration", complex)]
pub struct TaxConfiguration {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "channel")]
    pub channel: Option<Box<Channel>>,

    #[graphql(name = "chargeTaxes")]
    pub charge_taxes: Option<bool>,

    #[graphql(name = "taxCalculationStrategy")]
    pub tax_calculation_strategy: Option<String>,

    #[graphql(name = "displayGrossPrices")]
    pub display_gross_prices: Option<bool>,

    #[graphql(name = "pricesEnteredWithTax")]
    pub prices_entered_with_tax: Option<bool>,

    #[graphql(name = "countries")]
    pub countries: Vec<TaxConfigurationPerCountry>,

    #[graphql(name = "taxAppId")]
    pub tax_app_id: Option<String>,

}


#[ComplexObject]
impl TaxConfiguration {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxConfigurationCountableConnection")]
pub struct TaxConfigurationCountableConnection {

    #[graphql(name = "edges")]
    pub edges: Vec<TaxConfigurationCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxConfigurationCountableEdge")]
pub struct TaxConfigurationCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<TaxConfiguration>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxConfigurationPerCountry")]
pub struct TaxConfigurationPerCountry {

    #[graphql(name = "country")]
    pub country: Option<crate::common::GqlCountryDisplay>,

    #[graphql(name = "chargeTaxes")]
    pub charge_taxes: Option<bool>,

    #[graphql(name = "taxCalculationStrategy")]
    pub tax_calculation_strategy: Option<String>,

    #[graphql(name = "displayGrossPrices")]
    pub display_gross_prices: Option<bool>,

    #[graphql(name = "taxAppId")]
    pub tax_app_id: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxConfigurationUpdate")]
pub struct TaxConfigurationUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<TaxConfigurationUpdateError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxConfigurationUpdateError")]
pub struct TaxConfigurationUpdateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxCountryConfiguration")]
pub struct TaxCountryConfiguration {

    #[graphql(name = "country")]
    pub country: Option<crate::common::GqlCountryDisplay>,

    #[graphql(name = "taxClassCountryRates")]
    pub tax_class_country_rates: Vec<TaxClassCountryRate>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxCountryConfigurationDelete")]
pub struct TaxCountryConfigurationDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<TaxCountryConfigurationDeleteError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxCountryConfigurationDeleteError")]
pub struct TaxCountryConfigurationDeleteError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxCountryConfigurationUpdate")]
pub struct TaxCountryConfigurationUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<TaxCountryConfigurationUpdateError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxCountryConfigurationUpdateError")]
pub struct TaxCountryConfigurationUpdateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TaxedMoneyRange")]
pub struct TaxedMoneyRange {

    #[graphql(name = "start")]
    pub start: Option<crate::order::GqlTaxedMoney>,

    #[graphql(name = "stop")]
    pub stop: Option<crate::order::GqlTaxedMoney>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TimePeriod")]
pub struct TimePeriod {

    #[graphql(name = "amount")]
    pub amount: Option<i32>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Transaction")]
pub struct Transaction {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "created")]
    pub created: Option<DateTime<Utc>>,

    #[graphql(name = "token")]
    pub token: Option<String>,

    #[graphql(name = "kind")]
    pub kind: Option<String>,

    #[graphql(name = "isSuccess")]
    pub is_success: Option<bool>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TransactionCreate")]
pub struct TransactionCreate {

    #[graphql(name = "transaction")]
    pub transaction: Option<TransactionItem>,

    #[graphql(name = "errors")]
    pub errors: Vec<TransactionCreateError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TransactionCreateError")]
pub struct TransactionCreateError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TransactionEvent", complex)]
pub struct TransactionEvent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,

    #[graphql(name = "pspReference")]
    pub psp_reference: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "reasonReference")]
    pub reason_reference: Option<Page>,

    #[graphql(name = "externalUrl")]
    pub external_url: Option<String>,

    #[graphql(name = "amount")]
    pub amount: Option<crate::common::Money>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "createdBy")]
    pub created_by: Option<UserOrApp>,

}


#[ComplexObject]
impl TransactionEvent {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TransactionItem", complex)]
pub struct TransactionItem {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,

    #[graphql(name = "actions")]
    pub actions: Vec<String>,

    #[graphql(name = "authorizedAmount")]
    pub authorized_amount: Option<crate::common::Money>,

    #[graphql(name = "authorizePendingAmount")]
    pub authorize_pending_amount: Option<crate::common::Money>,

    #[graphql(name = "refundedAmount")]
    pub refunded_amount: Option<crate::common::Money>,

    #[graphql(name = "refundPendingAmount")]
    pub refund_pending_amount: Option<crate::common::Money>,

    #[graphql(name = "canceledAmount")]
    pub canceled_amount: Option<crate::common::Money>,

    #[graphql(name = "cancelPendingAmount")]
    pub cancel_pending_amount: Option<crate::common::Money>,

    #[graphql(name = "chargedAmount")]
    pub charged_amount: Option<crate::common::Money>,

    #[graphql(name = "chargePendingAmount")]
    pub charge_pending_amount: Option<crate::common::Money>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "pspReference")]
    pub psp_reference: Option<String>,

    #[graphql(name = "events")]
    pub events: Vec<TransactionEvent>,

    #[graphql(name = "createdBy")]
    pub created_by: Option<UserOrApp>,

    #[graphql(name = "externalUrl")]
    pub external_url: Option<String>,

    #[graphql(name = "paymentMethodDetails")]
    pub payment_method_details: Option<PaymentMethodDetails>,

}


#[ComplexObject]
impl TransactionItem {

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TransactionRequestAction")]
pub struct TransactionRequestAction {

    #[graphql(name = "transaction")]
    pub transaction: Option<TransactionItem>,

    #[graphql(name = "errors")]
    pub errors: Vec<TransactionRequestActionError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TransactionRequestActionError")]
pub struct TransactionRequestActionError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TransactionRequestRefundForGrantedRefund")]
pub struct TransactionRequestRefundForGrantedRefund {

    #[graphql(name = "transaction")]
    pub transaction: Option<TransactionItem>,

    #[graphql(name = "errors")]
    pub errors: Vec<TransactionRequestRefundForGrantedRefundError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TransactionRequestRefundForGrantedRefundError")]
pub struct TransactionRequestRefundForGrantedRefundError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TranslatableItemConnection")]
pub struct TranslatableItemConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<TranslatableItemEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TranslatableItemEdge")]
pub struct TranslatableItemEdge {

    #[graphql(name = "node")]
    pub node: Option<TranslatableItem>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "TranslationError")]
pub struct TranslationError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "UpdateMetadata")]
pub struct UpdateMetadata {

    #[graphql(name = "errors")]
    pub errors: Vec<MetadataError>,

    #[graphql(name = "item")]
    pub item: Option<ObjectWithMetadata>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "UpdatePrivateMetadata")]
pub struct UpdatePrivateMetadata {

    #[graphql(name = "errors")]
    pub errors: Vec<MetadataError>,

    #[graphql(name = "item")]
    pub item: Option<ObjectWithMetadata>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "UploadError")]
pub struct UploadError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "User", complex)]
pub struct User {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "firstName")]
    pub first_name: Option<String>,

    #[graphql(name = "lastName")]
    pub last_name: Option<String>,

    #[graphql(name = "isStaff")]
    pub is_staff: Option<bool>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "isConfirmed")]
    pub is_confirmed: Option<bool>,

    #[graphql(name = "addresses")]
    pub addresses: Vec<crate::order::GqlAddress>,

    #[graphql(name = "note")]
    pub note: Option<String>,

    #[graphql(name = "userPermissions")]
    pub user_permissions: Vec<crate::account::GqlUserPermission>,

    #[graphql(name = "permissionGroups")]
    pub permission_groups: Vec<Group>,

    #[graphql(name = "editableGroups")]
    pub editable_groups: Vec<Group>,

    #[graphql(name = "accessibleChannels")]
    pub accessible_channels: Vec<Channel>,

    #[graphql(name = "restrictedAccessToChannels")]
    pub restricted_access_to_channels: Option<bool>,

    #[graphql(name = "defaultShippingAddress")]
    pub default_shipping_address: Option<crate::order::GqlAddress>,

    #[graphql(name = "defaultBillingAddress")]
    pub default_billing_address: Option<crate::order::GqlAddress>,

    #[graphql(name = "externalReference")]
    pub external_reference: Option<String>,

    #[graphql(name = "customerType")]
    pub customer_type: Option<CustomerType>,

    #[graphql(name = "lastLogin")]
    pub last_login: Option<DateTime<Utc>>,

    #[graphql(name = "dateJoined")]
    pub date_joined: Option<DateTime<Utc>>,

}


#[ComplexObject]
impl User {

    #[graphql(name = "assignedAttributes")]
    async fn assigned_attributes(&self, #[graphql(name = "limit")] _arg_limit: Option<i32>) -> Vec<AssignedAttribute> {

        vec![]

    }

    #[graphql(name = "giftCards")]
    async fn gift_cards(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<GiftCardCountableConnection> {

        None

    }

    #[graphql(name = "orders")]
    async fn orders(&self, ctx: &Context<'_>, #[graphql(name = "where")] _arg_where: Option<CustomerOrderWhereInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<crate::order::GqlOrderConnection> {

        {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        let uid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let n = rustygod_db::entities::order_order::Entity::find()
            .select_only()
            .column(rustygod_db::entities::order_order::Column::Id)
            .filter(rustygod_db::entities::order_order::Column::UserId.eq(uid))
            .into_tuple::<uuid::Uuid>()
            .all(&db)
            .await
            .unwrap_or_default()
            .len();
        Some(crate::order::GqlOrderConnection {
            total_count: Some(n as i32),
            edges: vec![],
            page_info: crate::common::PageInfo {
                has_next_page: false,
                has_previous_page: false,
                start_cursor: None,
                end_cursor: None,
            },
        })
    }

    }

    #[graphql(name = "avatar")]
    async fn avatar(&self, #[graphql(name = "size")] _arg_size: Option<i32>, #[graphql(name = "format")] _arg_format: Option<ThumbnailFormatEnum>) -> Option<crate::account::GqlImage> {

        None

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "UserAvatarDelete")]
pub struct UserAvatarDelete {

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "UserAvatarUpdate")]
pub struct UserAvatarUpdate {

    #[graphql(name = "user")]
    pub user: Option<User>,

    #[graphql(name = "errors")]
    pub errors: Vec<crate::account::GqlAccountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "UserCountableConnection")]
pub struct UserCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<UserCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "UserCountableEdge")]
pub struct UserCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<User>,

    #[graphql(name = "cursor")]
    pub cursor: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VariantMediaAssign")]
pub struct VariantMediaAssign {

    #[graphql(name = "productVariant")]
    pub product_variant: Option<ProductVariant>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VariantMediaUnassign")]
pub struct VariantMediaUnassign {

    #[graphql(name = "productVariant")]
    pub product_variant: Option<ProductVariant>,

    #[graphql(name = "errors")]
    pub errors: Vec<ProductError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VariantPricingInfo")]
pub struct VariantPricingInfo {

    #[graphql(name = "onSale")]
    pub on_sale: Option<bool>,

    #[graphql(name = "price")]
    pub price: Option<crate::order::GqlTaxedMoney>,

    #[graphql(name = "priceUndiscounted")]
    pub price_undiscounted: Option<crate::order::GqlTaxedMoney>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Voucher", complex)]
pub struct Voucher {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "usageLimit")]
    pub usage_limit: Option<i32>,

    #[graphql(name = "used")]
    pub used: Option<i32>,

    #[graphql(name = "startDate")]
    pub start_date: Option<DateTime<Utc>>,

    #[graphql(name = "endDate")]
    pub end_date: Option<DateTime<Utc>>,

    #[graphql(name = "applyOncePerOrder")]
    pub apply_once_per_order: Option<bool>,

    #[graphql(name = "applyOncePerCustomer")]
    pub apply_once_per_customer: Option<bool>,

    #[graphql(name = "singleUse")]
    pub single_use: Option<bool>,

    #[graphql(name = "onlyForStaff")]
    pub only_for_staff: Option<bool>,

    #[graphql(name = "minCheckoutItemsQuantity")]
    pub min_checkout_items_quantity: Option<i32>,

    #[graphql(name = "countries")]
    pub countries: Vec<crate::common::GqlCountryDisplay>,

    #[graphql(name = "discountValueType")]
    pub discount_value_type: Option<String>,

    #[graphql(name = "type")]
    pub r#type: Option<String>,

    #[graphql(name = "channelListings")]
    pub channel_listings: Vec<VoucherChannelListing>,

}


#[ComplexObject]
impl Voucher {

    #[graphql(name = "codes")]
    async fn codes(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<VoucherCodeCountableConnection> {

        None

    }

    #[graphql(name = "categories")]
    async fn categories(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<CategoryCountableConnection> {

        None

    }

    #[graphql(name = "collections")]
    async fn collections(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<CollectionCountableConnection> {

        None

    }

    #[graphql(name = "products")]
    async fn products(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<crate::catalog::GqlProductConnection> {

        None

    }

    #[graphql(name = "variants")]
    async fn variants(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<ProductVariantCountableConnection> {

        None

    }

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<VoucherTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.id.as_ref().and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::voucher_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherAddCatalogues")]
pub struct VoucherAddCatalogues {

    #[graphql(name = "voucher")]
    pub voucher: Option<Voucher>,

    #[graphql(name = "errors")]
    pub errors: Vec<DiscountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherBulkDelete")]
pub struct VoucherBulkDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<DiscountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherChannelListing")]
pub struct VoucherChannelListing {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "channel")]
    pub channel: Option<Channel>,

    #[graphql(name = "discountValue")]
    pub discount_value: Option<f64>,

    #[graphql(name = "currency")]
    pub currency: Option<String>,

    #[graphql(name = "minSpent")]
    pub min_spent: Option<crate::common::Money>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherChannelListingUpdate")]
pub struct VoucherChannelListingUpdate {

    #[graphql(name = "voucher")]
    pub voucher: Option<Voucher>,

    #[graphql(name = "errors")]
    pub errors: Vec<DiscountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherCode")]
pub struct VoucherCode {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "code")]
    pub code: Option<String>,

    #[graphql(name = "used")]
    pub used: Option<i32>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherCodeBulkDelete")]
pub struct VoucherCodeBulkDelete {

    #[graphql(name = "count")]
    pub count: Option<i32>,

    #[graphql(name = "errors")]
    pub errors: Vec<VoucherCodeBulkDeleteError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherCodeBulkDeleteError")]
pub struct VoucherCodeBulkDeleteError {

    #[graphql(name = "path")]
    pub path: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherCodeCountableConnection")]
pub struct VoucherCodeCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<VoucherCodeCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherCodeCountableEdge")]
pub struct VoucherCodeCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<VoucherCode>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherCountableConnection")]
pub struct VoucherCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<VoucherCountableEdge>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherCountableEdge")]
pub struct VoucherCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Voucher>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherCreate")]
pub struct VoucherCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<DiscountError>,

    #[graphql(name = "voucher")]
    pub voucher: Option<Voucher>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherDelete")]
pub struct VoucherDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<DiscountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherRemoveCatalogues")]
pub struct VoucherRemoveCatalogues {

    #[graphql(name = "voucher")]
    pub voucher: Option<Voucher>,

    #[graphql(name = "errors")]
    pub errors: Vec<DiscountError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherTranslatableContent", complex)]
pub struct VoucherTranslatableContent {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "voucher")]
    pub voucher: Option<Voucher>,

}


#[ComplexObject]
impl VoucherTranslatableContent {

    #[graphql(name = "translation")]
    async fn translation(&self, ctx: &Context<'_>, #[graphql(name = "languageCode")] _arg_language_code: LanguageCodeEnum) -> Option<VoucherTranslation> {

        {
        let db = match ctx.data_opt::<crate::context::GqlContext>().and_then(|g| g.db().ok()) {
            Some(d) => d.clone(),
            None => return None,
        };
        let eid: i32 = self.voucher.as_ref().and_then(|e| e.id.as_ref()).and_then(|i| rustygod_db::catalog::parse_gid(&i.0)).unwrap_or(-1);
        let lang = crate::gen::language_code_value(&_arg_language_code);
        crate::translations::voucher_translation(&db, eid, lang).await
    }

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherTranslate")]
pub struct VoucherTranslate {

    #[graphql(name = "errors")]
    pub errors: Vec<TranslationError>,

    #[graphql(name = "voucher")]
    pub voucher: Option<Voucher>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherTranslation")]
pub struct VoucherTranslation {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "language")]
    pub language: Option<crate::commerce::GqlLanguageDisplay>,

    #[graphql(name = "name")]
    pub name: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "VoucherUpdate")]
pub struct VoucherUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<DiscountError>,

    #[graphql(name = "voucher")]
    pub voucher: Option<Voucher>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Warehouse", complex)]
pub struct Warehouse {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "privateMetadata")]
    pub private_metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "metadata")]
    pub metadata: Vec<crate::common::MetadataItem>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "slug")]
    pub slug: Option<String>,

    #[graphql(name = "email")]
    pub email: Option<String>,

    #[graphql(name = "isPrivate")]
    pub is_private: Option<bool>,

    #[graphql(name = "address")]
    pub address: Option<crate::order::GqlAddress>,

    #[graphql(name = "clickAndCollectOption")]
    pub click_and_collect_option: Option<String>,

}


#[ComplexObject]
impl Warehouse {

    #[graphql(name = "shippingZones")]
    async fn shipping_zones(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<ShippingZoneCountableConnection> {

        None

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

    pub async fn gen_iface_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.metadata.clone()

    }

    pub async fn gen_iface_private_metadata(&self) -> Vec<crate::common::MetadataItem> {

        self.private_metadata.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WarehouseCountableConnection")]
pub struct WarehouseCountableConnection {

    #[graphql(name = "pageInfo")]
    pub page_info: Option<crate::common::PageInfo>,

    #[graphql(name = "edges")]
    pub edges: Vec<WarehouseCountableEdge>,

    #[graphql(name = "totalCount")]
    pub total_count: Option<i32>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WarehouseCountableEdge")]
pub struct WarehouseCountableEdge {

    #[graphql(name = "node")]
    pub node: Option<Warehouse>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WarehouseCreate")]
pub struct WarehouseCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<WarehouseError>,

    #[graphql(name = "warehouse")]
    pub warehouse: Option<Warehouse>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WarehouseDelete")]
pub struct WarehouseDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<WarehouseError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WarehouseError")]
pub struct WarehouseError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WarehouseUpdate")]
pub struct WarehouseUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<WarehouseError>,

    #[graphql(name = "warehouse")]
    pub warehouse: Option<Warehouse>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Webhook", complex)]
pub struct Webhook {

    #[graphql(name = "id")]
    pub id: Option<ID>,

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "syncEvents")]
    pub sync_events: Vec<WebhookEventSync>,

    #[graphql(name = "asyncEvents")]
    pub async_events: Vec<WebhookEventAsync>,

    #[graphql(name = "app")]
    pub app: Option<Box<App>>,

    #[graphql(name = "targetUrl")]
    pub target_url: Option<String>,

    #[graphql(name = "isActive")]
    pub is_active: Option<bool>,

    #[graphql(name = "secretKey")]
    pub secret_key: Option<String>,

    #[graphql(name = "subscriptionQuery")]
    pub subscription_query: Option<String>,

    #[graphql(name = "customHeaders")]
    pub custom_headers: Option<GenJSONString>,

}


#[ComplexObject]
impl Webhook {

    #[graphql(name = "eventDeliveries")]
    async fn event_deliveries(&self, #[graphql(name = "sortBy")] _arg_sort_by: Option<EventDeliverySortingInput>, #[graphql(name = "filter")] _arg_filter: Option<EventDeliveryFilterInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<crate::apps::GqlEventDeliveryConnection> {

        None

    }

    pub async fn gen_iface_id(&self) -> Option<ID> {

        self.id.clone()

    }

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WebhookCreate")]
pub struct WebhookCreate {

    #[graphql(name = "errors")]
    pub errors: Vec<WebhookError>,

    #[graphql(name = "webhook")]
    pub webhook: Option<Webhook>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WebhookDelete")]
pub struct WebhookDelete {

    #[graphql(name = "errors")]
    pub errors: Vec<WebhookError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WebhookDryRun")]
pub struct WebhookDryRun {

    #[graphql(name = "payload")]
    pub payload: Option<GenJSONString>,

    #[graphql(name = "errors")]
    pub errors: Vec<WebhookDryRunError>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WebhookDryRunError")]
pub struct WebhookDryRunError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WebhookError")]
pub struct WebhookError {

    #[graphql(name = "field")]
    pub field: Option<String>,

    #[graphql(name = "message")]
    pub message: Option<String>,

    #[graphql(name = "code")]
    pub code: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WebhookEventAsync")]
pub struct WebhookEventAsync {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "eventType")]
    pub event_type: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WebhookEventSync")]
pub struct WebhookEventSync {

    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "eventType")]
    pub event_type: Option<String>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "WebhookUpdate")]
pub struct WebhookUpdate {

    #[graphql(name = "errors")]
    pub errors: Vec<WebhookError>,

    #[graphql(name = "webhook")]
    pub webhook: Option<Webhook>,

}


#[derive(SimpleObject, Clone)]
#[graphql(name = "Weight")]
pub struct Weight {

    #[graphql(name = "unit")]
    pub unit: Option<String>,

    #[graphql(name = "value")]
    pub value: Option<f64>,

}


#[derive(Union, Clone)]
#[graphql(name = "DeliveryMethod")]
pub enum DeliveryMethod {

    ShippingMethod(ShippingMethod),

    Warehouse(Warehouse),

}


#[derive(Union, Clone)]
#[graphql(name = "IssuingPrincipal")]
pub enum IssuingPrincipal {

    App(App),

    User(User),

}


#[derive(Union, Clone)]
#[graphql(name = "OrderOrCheckout")]
pub enum OrderOrCheckout {

    Checkout(Checkout),

    Order(Order),

}


#[derive(Union, Clone)]
#[graphql(name = "ReferenceType")]
pub enum ReferenceType {

    PageType(PageType),

    ProductType(ProductType),

}


#[derive(Union, Clone)]
#[graphql(name = "TaxSourceLine")]
pub enum TaxSourceLine {

    OrderLine(OrderLine),

}


#[derive(Union, Clone)]
#[graphql(name = "TaxSourceObject")]
pub enum TaxSourceObject {

    Checkout(Checkout),

    Order(Order),

}


#[derive(Union, Clone)]
#[graphql(name = "TranslatableItem")]
pub enum TranslatableItem {

    AttributeTranslatableContent(AttributeTranslatableContent),

    AttributeValueTranslatableContent(AttributeValueTranslatableContent),

    CategoryTranslatableContent(CategoryTranslatableContent),

    CollectionTranslatableContent(CollectionTranslatableContent),

    MenuItemTranslatableContent(MenuItemTranslatableContent),

    PageTranslatableContent(PageTranslatableContent),

    ProductTranslatableContent(ProductTranslatableContent),

    ProductVariantTranslatableContent(ProductVariantTranslatableContent),

    SaleTranslatableContent(SaleTranslatableContent),

    ShippingMethodTranslatableContent(ShippingMethodTranslatableContent),

    VoucherTranslatableContent(VoucherTranslatableContent),

}


#[derive(Union, Clone)]
#[graphql(name = "UserOrApp")]
pub enum UserOrApp {

    App(App),

    User(User),

}


#[derive(Interface, Clone)]
#[graphql(field(name = "attribute", method = "gen_iface_attribute", ty = "Option<Attribute>"))]
#[graphql(name = "AssignedAttribute")]
pub enum AssignedAttribute {

    AssignedBooleanAttribute(AssignedBooleanAttribute),

    AssignedDateAttribute(AssignedDateAttribute),

    AssignedDateTimeAttribute(AssignedDateTimeAttribute),

    AssignedFileAttribute(AssignedFileAttribute),

    AssignedMultiCategoryReferenceAttribute(AssignedMultiCategoryReferenceAttribute),

    AssignedMultiChoiceAttribute(AssignedMultiChoiceAttribute),

    AssignedMultiCollectionReferenceAttribute(AssignedMultiCollectionReferenceAttribute),

    AssignedMultiPageReferenceAttribute(AssignedMultiPageReferenceAttribute),

    AssignedMultiProductReferenceAttribute(AssignedMultiProductReferenceAttribute),

    AssignedMultiProductVariantReferenceAttribute(AssignedMultiProductVariantReferenceAttribute),

    AssignedNumericAttribute(AssignedNumericAttribute),

    AssignedPlainTextAttribute(AssignedPlainTextAttribute),

    AssignedSingleCategoryReferenceAttribute(AssignedSingleCategoryReferenceAttribute),

    AssignedSingleChoiceAttribute(AssignedSingleChoiceAttribute),

    AssignedSingleCollectionReferenceAttribute(AssignedSingleCollectionReferenceAttribute),

    AssignedSinglePageReferenceAttribute(AssignedSinglePageReferenceAttribute),

    AssignedSingleProductReferenceAttribute(AssignedSingleProductReferenceAttribute),

    AssignedSingleProductVariantReferenceAttribute(AssignedSingleProductVariantReferenceAttribute),

    AssignedSwatchAttribute(AssignedSwatchAttribute),

    AssignedTextAttribute(AssignedTextAttribute),

}


#[derive(Interface, Clone)]
#[graphql(field(name = "id", method = "gen_iface_id", ty = "Option<ID>"))]
#[graphql(name = "Node")]
pub enum Node {

    Address(crate::order::GqlAddress),

    App(App),

    AppInstallation(AppInstallation),

    Attribute(Attribute),

    AttributeTranslatableContent(AttributeTranslatableContent),

    AttributeValue(AttributeValue),

    AttributeValueTranslatableContent(AttributeValueTranslatableContent),

    Category(Category),

    CategoryTranslatableContent(CategoryTranslatableContent),

    Channel(Channel),

    Collection(Collection),

    CollectionTranslatableContent(CollectionTranslatableContent),

    CustomerType(CustomerType),

    EventDeliveryAttempt(EventDeliveryAttempt),

    ExportFile(ExportFile),

    Fulfillment(Fulfillment),

    GiftCard(GiftCard),

    GiftCardEvent(GiftCardEvent),

    Group(Group),

    Invoice(Invoice),

    Menu(Menu),

    MenuItem(MenuItem),

    MenuItemTranslatableContent(MenuItemTranslatableContent),

    Order(Order),

    OrderDiscount(OrderDiscount),

    OrderEvent(OrderEvent),

    OrderLine(OrderLine),

    Page(Page),

    PageTranslatableContent(PageTranslatableContent),

    PageType(PageType),

    Payment(Payment),

    Product(Product),

    ProductChannelListing(ProductChannelListing),

    ProductMedia(ProductMedia),

    ProductTranslatableContent(ProductTranslatableContent),

    ProductType(ProductType),

    ProductVariant(ProductVariant),

    ProductVariantChannelListing(ProductVariantChannelListing),

    ProductVariantTranslatableContent(ProductVariantTranslatableContent),

    Promotion(Promotion),

    PromotionRule(PromotionRule),

    Sale(Sale),

    SaleTranslatableContent(SaleTranslatableContent),

    ShippingMethod(ShippingMethod),

    ShippingMethodTranslatableContent(ShippingMethodTranslatableContent),

    ShippingMethodType(ShippingMethodType),

    ShippingZone(ShippingZone),

    Stock(Stock),

    TaxClass(TaxClass),

    TaxConfiguration(TaxConfiguration),

    TransactionEvent(TransactionEvent),

    TransactionItem(TransactionItem),

    User(User),

    Voucher(Voucher),

    VoucherTranslatableContent(VoucherTranslatableContent),

    Warehouse(Warehouse),

    Webhook(Webhook),

}


#[derive(Interface, Clone)]
#[graphql(field(name = "metadata", method = "gen_iface_metadata", ty = "Vec<crate::common::MetadataItem>"), field(name = "privateMetadata", method = "gen_iface_private_metadata", ty = "Vec<crate::common::MetadataItem>"))]
#[graphql(name = "ObjectWithMetadata")]
pub enum ObjectWithMetadata {

    Address(crate::order::GqlAddress),

    App(App),

    Attribute(Attribute),

    Category(Category),

    Channel(Channel),

    Collection(Collection),

    CustomerType(CustomerType),

    Fulfillment(Fulfillment),

    GiftCard(GiftCard),

    Invoice(Invoice),

    Menu(Menu),

    MenuItem(MenuItem),

    Order(Order),

    OrderLine(OrderLine),

    Page(Page),

    PageType(PageType),

    Payment(Payment),

    Product(Product),

    ProductMedia(ProductMedia),

    ProductType(ProductType),

    ProductVariant(ProductVariant),

    Promotion(Promotion),

    Sale(Sale),

    ShippingMethod(ShippingMethod),

    ShippingMethodType(ShippingMethodType),

    ShippingZone(ShippingZone),

    Shop(Shop),

    TaxClass(TaxClass),

    TaxConfiguration(TaxConfiguration),

    TransactionItem(TransactionItem),

    User(User),

    Voucher(Voucher),

    Warehouse(Warehouse),

}


#[derive(Interface, Clone)]
#[graphql(field(name = "name", method = "gen_iface_name", ty = "Option<String>"))]
#[graphql(name = "PaymentMethodDetails")]
pub enum PaymentMethodDetails {

    CardPaymentMethodDetails(CardPaymentMethodDetails),

    GiftCardPaymentMethodDetails(GiftCardPaymentMethodDetails),

    OtherPaymentMethodDetails(OtherPaymentMethodDetails),

}


#[derive(Default)]
pub struct GenQuery;

#[Object]
impl GenQuery {

    #[graphql(name = "webhook")]
    async fn webhook(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<Webhook> {

        None

    }

    #[graphql(name = "warehouse")]
    async fn warehouse(&self, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "externalReference")] _arg_external_reference: Option<String>) -> Option<Warehouse> {

        None

    }

    #[graphql(name = "taxCountryConfigurations")]
    async fn tax_country_configurations(&self) -> Vec<TaxCountryConfiguration> {

        vec![]

    }

    #[graphql(name = "giftCardSettings")]
    async fn gift_card_settings(&self) -> Option<GiftCardSettings> {

        None

    }

    #[graphql(name = "refundSettings")]
    async fn refund_settings(&self) -> Option<RefundSettings> {

        None

    }

    #[graphql(name = "returnSettings")]
    async fn return_settings(&self) -> Option<ReturnSettings> {

        None

    }

    #[graphql(name = "productType")]
    async fn product_type(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<ProductType> {

        None

    }

    #[graphql(name = "productTypes")]
    async fn product_types(&self, #[graphql(name = "filter")] _arg_filter: Option<ProductTypeFilterInput>, #[graphql(name = "sortBy")] _arg_sort_by: Option<ProductTypeSortingInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<ProductTypeCountableConnection> {

        Some(ProductTypeCountableConnection { edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

    #[graphql(name = "productVariant")]
    async fn product_variant(&self, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "sku")] _arg_sku: Option<String>, #[graphql(name = "externalReference")] _arg_external_reference: Option<String>, #[graphql(name = "channel")] _arg_channel: Option<String>) -> Option<ProductVariant> {

        None

    }

    #[graphql(name = "productVariants")]
    async fn product_variants(&self, #[graphql(name = "ids")] _arg_ids: Option<Vec<ID>>, #[graphql(name = "channel")] _arg_channel: Option<String>, #[graphql(name = "filter")] _arg_filter: Option<ProductVariantFilterInput>, #[graphql(name = "where")] _arg_where: Option<ProductVariantWhereInput>, #[graphql(name = "search")] _arg_search: Option<String>, #[graphql(name = "sortBy")] _arg_sort_by: Option<ProductVariantSortingInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<ProductVariantCountableConnection> {

        Some(ProductVariantCountableConnection { total_count: None, edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

    #[graphql(name = "page")]
    async fn page(&self, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "slug")] _arg_slug: Option<String>, #[graphql(name = "slugLanguageCode")] _arg_slug_language_code: Option<LanguageCodeEnum>, #[graphql(name = "channel")] _arg_channel: Option<String>) -> Option<Page> {

        None

    }

    #[graphql(name = "pageType")]
    async fn page_type(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<PageType> {

        None

    }

    #[graphql(name = "pageTypes")]
    async fn page_types(&self, #[graphql(name = "sortBy")] _arg_sort_by: Option<PageTypeSortingInput>, #[graphql(name = "filter")] _arg_filter: Option<PageTypeFilterInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<PageTypeCountableConnection> {

        Some(PageTypeCountableConnection { edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

    #[graphql(name = "homepageEvents")]
    async fn homepage_events(&self, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<OrderEventCountableConnection> {

        Some(OrderEventCountableConnection { edges: vec![] })

    }

    #[graphql(name = "draftOrders")]
    async fn draft_orders(&self, #[graphql(name = "sortBy")] _arg_sort_by: Option<OrderSortingInput>, #[graphql(name = "filter")] _arg_filter: Option<OrderDraftFilterInput>, #[graphql(name = "where")] _arg_where: Option<DraftOrderWhereInput>, #[graphql(name = "search")] _arg_search: Option<String>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<crate::order::GqlOrderConnection> {

        Some(crate::order::GqlOrderConnection { total_count: None, edges: vec![], page_info: PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None } })

    }

    #[graphql(name = "ordersTotal")]
    async fn orders_total(&self, #[graphql(name = "period")] _arg_period: Option<ReportingPeriod>, #[graphql(name = "channel")] _arg_channel: Option<String>) -> Option<crate::order::GqlTaxedMoney> {

        None

    }

    #[graphql(name = "giftCardTags")]
    async fn gift_card_tags(&self, #[graphql(name = "filter")] _arg_filter: Option<GiftCardTagFilterInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<GiftCardTagCountableConnection> {

        Some(GiftCardTagCountableConnection { total_count: None, edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

    #[graphql(name = "plugin")]
    async fn plugin(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<Plugin> {

        None

    }

    #[graphql(name = "plugins")]
    async fn plugins(&self, #[graphql(name = "filter")] _arg_filter: Option<PluginFilterInput>, #[graphql(name = "sortBy")] _arg_sort_by: Option<PluginSortingInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<PluginCountableConnection> {

        Some(PluginCountableConnection { edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

    #[graphql(name = "sale")]
    async fn sale(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "channel")] _arg_channel: Option<String>) -> Option<Sale> {

        None

    }

    #[graphql(name = "sales")]
    async fn sales(&self, #[graphql(name = "filter")] _arg_filter: Option<SaleFilterInput>, #[graphql(name = "sortBy")] _arg_sort_by: Option<SaleSortingInput>, #[graphql(name = "query")] _arg_query: Option<String>, #[graphql(name = "channel")] _arg_channel: Option<String>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<SaleCountableConnection> {

        Some(SaleCountableConnection { edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

    #[graphql(name = "voucher")]
    async fn voucher(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "channel")] _arg_channel: Option<String>) -> Option<Voucher> {

        None

    }

    #[graphql(name = "vouchers")]
    async fn vouchers(&self, #[graphql(name = "filter")] _arg_filter: Option<VoucherFilterInput>, #[graphql(name = "sortBy")] _arg_sort_by: Option<VoucherSortingInput>, #[graphql(name = "query")] _arg_query: Option<String>, #[graphql(name = "channel")] _arg_channel: Option<String>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<VoucherCountableConnection> {

        Some(VoucherCountableConnection { edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

    #[graphql(name = "exportFile")]
    async fn export_file(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<ExportFile> {

        None

    }

    #[graphql(name = "checkouts")]
    async fn checkouts(&self, #[graphql(name = "sortBy")] _arg_sort_by: Option<CheckoutSortingInput>, #[graphql(name = "filter")] _arg_filter: Option<CheckoutFilterInput>, #[graphql(name = "channel")] _arg_channel: Option<String>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<CheckoutCountableConnection> {

        Some(CheckoutCountableConnection { edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

    #[graphql(name = "appsInstallations")]
    async fn apps_installations(&self) -> Vec<AppInstallation> {

        vec![]

    }

    #[graphql(name = "app")]
    async fn app(&self, #[graphql(name = "id")] _arg_id: Option<ID>) -> Option<App> {

        None

    }

    #[graphql(name = "addressValidationRules")]
    async fn address_validation_rules(&self, #[graphql(name = "countryCode")] _arg_country_code: CountryCode, #[graphql(name = "countryArea")] _arg_country_area: Option<String>, #[graphql(name = "city")] _arg_city: Option<String>, #[graphql(name = "cityArea")] _arg_city_area: Option<String>) -> Option<AddressValidationData> {

        None

    }

    #[graphql(name = "customerType")]
    async fn customer_type(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<CustomerType> {

        None

    }

    #[graphql(name = "customerTypes")]
    async fn customer_types(&self, #[graphql(name = "where")] _arg_where: Option<CustomerTypeWhereInput>, #[graphql(name = "search")] _arg_search: Option<String>, #[graphql(name = "sortBy")] _arg_sort_by: Option<CustomerTypeSortingInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<CustomerTypeCountableConnection> {

        Some(CustomerTypeCountableConnection { edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

    #[graphql(name = "permissionGroups")]
    async fn permission_groups(&self, #[graphql(name = "filter")] _arg_filter: Option<PermissionGroupFilterInput>, #[graphql(name = "sortBy")] _arg_sort_by: Option<PermissionGroupSortingInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<GroupCountableConnection> {

        Some(GroupCountableConnection { edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

    #[graphql(name = "permissionGroup")]
    async fn permission_group(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<Group> {

        None

    }

    #[graphql(name = "staffUsers")]
    async fn staff_users(&self, #[graphql(name = "filter")] _arg_filter: Option<StaffUserInput>, #[graphql(name = "sortBy")] _arg_sort_by: Option<UserSortingInput>, #[graphql(name = "before")] _arg_before: Option<String>, #[graphql(name = "after")] _arg_after: Option<String>, #[graphql(name = "first")] _arg_first: Option<i32>, #[graphql(name = "last")] _arg_last: Option<i32>) -> Option<UserCountableConnection> {

        Some(UserCountableConnection { total_count: None, edges: vec![], page_info: Some(PageInfo { has_next_page: false, has_previous_page: false, start_cursor: None, end_cursor: None }) })

    }

}


#[derive(Default)]
pub struct GenMutation;

#[Object]
impl GenMutation {

    #[graphql(name = "webhookCreate")]
    async fn webhook_create(&self, #[graphql(name = "input")] _arg_input: WebhookCreateInput) -> Option<WebhookCreate> {

        Some(WebhookCreate { errors: vec![], webhook: None })

    }

    #[graphql(name = "webhookDelete")]
    async fn webhook_delete(&self, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "identifier")] _arg_identifier: Option<String>) -> Option<WebhookDelete> {

        Some(WebhookDelete { errors: vec![] })

    }

    #[graphql(name = "webhookUpdate")]
    async fn webhook_update(&self, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "identifier")] _arg_identifier: Option<String>, #[graphql(name = "input")] _arg_input: WebhookUpdateInput) -> Option<WebhookUpdate> {

        Some(WebhookUpdate { errors: vec![], webhook: None })

    }

    #[graphql(name = "webhookDryRun")]
    async fn webhook_dry_run(&self, #[graphql(name = "objectId")] _arg_object_id: ID, #[graphql(name = "query")] _arg_query: String) -> Option<WebhookDryRun> {

        Some(WebhookDryRun { payload: None, errors: vec![] })

    }

    #[graphql(name = "createWarehouse")]
    async fn create_warehouse(&self, #[graphql(name = "input")] _arg_input: WarehouseCreateInput) -> Option<WarehouseCreate> {

        Some(WarehouseCreate { errors: vec![], warehouse: None })

    }

    #[graphql(name = "updateWarehouse")]
    async fn update_warehouse(&self, #[graphql(name = "externalReference")] _arg_external_reference: Option<String>, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "input")] _arg_input: WarehouseUpdateInput) -> Option<WarehouseUpdate> {

        Some(WarehouseUpdate { errors: vec![], warehouse: None })

    }

    #[graphql(name = "deleteWarehouse")]
    async fn delete_warehouse(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<WarehouseDelete> {

        Some(WarehouseDelete { errors: vec![] })

    }

    #[graphql(name = "taxClassCreate")]
    async fn tax_class_create(&self, #[graphql(name = "input")] _arg_input: TaxClassCreateInput) -> Option<TaxClassCreate> {

        Some(TaxClassCreate { errors: vec![], tax_class: None })

    }

    #[graphql(name = "taxClassDelete")]
    async fn tax_class_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<TaxClassDelete> {

        Some(TaxClassDelete { errors: vec![] })

    }

    #[graphql(name = "taxClassUpdate")]
    async fn tax_class_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: TaxClassUpdateInput) -> Option<TaxClassUpdate> {

        Some(TaxClassUpdate { errors: vec![], tax_class: None })

    }

    #[graphql(name = "taxConfigurationUpdate")]
    async fn tax_configuration_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: TaxConfigurationUpdateInput) -> Option<TaxConfigurationUpdate> {

        Some(TaxConfigurationUpdate { errors: vec![] })

    }

    #[graphql(name = "taxCountryConfigurationUpdate")]
    async fn tax_country_configuration_update(&self, #[graphql(name = "countryCode")] _arg_country_code: CountryCode, #[graphql(name = "updateTaxClassRates")] _arg_update_tax_class_rates: Vec<TaxClassRateInput>) -> Option<TaxCountryConfigurationUpdate> {

        Some(TaxCountryConfigurationUpdate { errors: vec![] })

    }

    #[graphql(name = "taxCountryConfigurationDelete")]
    async fn tax_country_configuration_delete(&self, #[graphql(name = "countryCode")] _arg_country_code: CountryCode) -> Option<TaxCountryConfigurationDelete> {

        Some(TaxCountryConfigurationDelete { errors: vec![] })

    }

    #[graphql(name = "staffNotificationRecipientCreate")]
    async fn staff_notification_recipient_create(&self, #[graphql(name = "input")] _arg_input: StaffNotificationRecipientInput) -> Option<StaffNotificationRecipientCreate> {

        Some(StaffNotificationRecipientCreate { errors: vec![], staff_notification_recipient: None })

    }

    #[graphql(name = "staffNotificationRecipientDelete")]
    async fn staff_notification_recipient_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<StaffNotificationRecipientDelete> {

        Some(StaffNotificationRecipientDelete { errors: vec![] })

    }

    #[graphql(name = "shopAddressUpdate")]
    async fn shop_address_update(&self, #[graphql(name = "input")] _arg_input: Option<AddressInput>) -> Option<ShopAddressUpdate> {

        Some(ShopAddressUpdate { shop: None, errors: vec![] })

    }

    #[graphql(name = "giftCardSettingsUpdate")]
    async fn gift_card_settings_update(&self, #[graphql(name = "input")] _arg_input: GiftCardSettingsUpdateInput) -> Option<GiftCardSettingsUpdate> {

        Some(GiftCardSettingsUpdate { gift_card_settings: None, errors: vec![] })

    }

    #[graphql(name = "refundSettingsUpdate")]
    async fn refund_settings_update(&self, #[graphql(name = "input")] _arg_input: RefundSettingsUpdateInput) -> Option<RefundSettingsUpdate> {

        Some(RefundSettingsUpdate { errors: vec![] })

    }

    #[graphql(name = "refundReasonReferenceClear")]
    async fn refund_reason_reference_clear(&self) -> Option<RefundReasonReferenceTypeClear> {

        Some(RefundReasonReferenceTypeClear { errors: vec![] })

    }

    #[graphql(name = "returnSettingsUpdate")]
    async fn return_settings_update(&self, #[graphql(name = "input")] _arg_input: ReturnSettingsUpdateInput) -> Option<ReturnSettingsUpdate> {

        Some(ReturnSettingsUpdate { errors: vec![] })

    }

    #[graphql(name = "returnReasonReferenceClear")]
    async fn return_reason_reference_clear(&self) -> Option<ReturnReasonReferenceTypeClear> {

        Some(ReturnReasonReferenceTypeClear { errors: vec![] })

    }

    #[graphql(name = "shippingMethodChannelListingUpdate")]
    async fn shipping_method_channel_listing_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: ShippingMethodChannelListingInput) -> Option<ShippingMethodChannelListingUpdate> {

        Some(ShippingMethodChannelListingUpdate { shipping_method: None, errors: vec![] })

    }

    #[graphql(name = "shippingPriceCreate")]
    async fn shipping_price_create(&self, #[graphql(name = "input")] _arg_input: ShippingPriceInput) -> Option<ShippingPriceCreate> {

        Some(ShippingPriceCreate { shipping_zone: None, shipping_method: None, errors: vec![] })

    }

    #[graphql(name = "shippingPriceDelete")]
    async fn shipping_price_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<ShippingPriceDelete> {

        Some(ShippingPriceDelete { shipping_zone: None, errors: vec![] })

    }

    #[graphql(name = "shippingPriceUpdate")]
    async fn shipping_price_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: ShippingPriceInput) -> Option<ShippingPriceUpdate> {

        Some(ShippingPriceUpdate { shipping_method: None, errors: vec![] })

    }

    #[graphql(name = "shippingPriceExcludeProducts")]
    async fn shipping_price_exclude_products(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: ShippingPriceExcludeProductsInput) -> Option<ShippingPriceExcludeProducts> {

        Some(ShippingPriceExcludeProducts { errors: vec![] })

    }

    #[graphql(name = "shippingPriceRemoveProductFromExclude")]
    async fn shipping_price_remove_product_from_exclude(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "products")] _arg_products: Vec<ID>) -> Option<ShippingPriceRemoveProductFromExclude> {

        Some(ShippingPriceRemoveProductFromExclude { errors: vec![] })

    }

    #[graphql(name = "shippingZoneCreate")]
    async fn shipping_zone_create(&self, #[graphql(name = "input")] _arg_input: ShippingZoneCreateInput) -> Option<ShippingZoneCreate> {

        Some(ShippingZoneCreate { errors: vec![], shipping_zone: None })

    }

    #[graphql(name = "shippingZoneDelete")]
    async fn shipping_zone_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<ShippingZoneDelete> {

        Some(ShippingZoneDelete { errors: vec![] })

    }

    #[graphql(name = "shippingZoneBulkDelete")]
    async fn shipping_zone_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<ShippingZoneBulkDelete> {

        Some(ShippingZoneBulkDelete { errors: vec![] })

    }

    #[graphql(name = "categoryBulkDelete")]
    async fn category_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<CategoryBulkDelete> {

        Some(CategoryBulkDelete { errors: vec![] })

    }

    #[graphql(name = "collectionBulkDelete")]
    async fn collection_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<CollectionBulkDelete> {

        Some(CollectionBulkDelete { errors: vec![] })

    }

    #[graphql(name = "collectionChannelListingUpdate")]
    async fn collection_channel_listing_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: CollectionChannelListingUpdateInput) -> Option<CollectionChannelListingUpdate> {

        Some(CollectionChannelListingUpdate { collection: None, errors: vec![] })

    }

    #[graphql(name = "productBulkDelete")]
    async fn product_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<ProductBulkDelete> {

        Some(ProductBulkDelete { errors: vec![] })

    }

    #[graphql(name = "productMediaCreate")]
    async fn product_media_create(&self, #[graphql(name = "input")] _arg_input: ProductMediaCreateInput) -> Option<ProductMediaCreate> {

        Some(ProductMediaCreate { product: None, media: None, errors: vec![] })

    }

    #[graphql(name = "productVariantReorder")]
    async fn product_variant_reorder(&self, #[graphql(name = "moves")] _arg_moves: Vec<ReorderInput>, #[graphql(name = "productId")] _arg_product_id: ID) -> Option<ProductVariantReorder> {

        Some(ProductVariantReorder { errors: vec![] })

    }

    #[graphql(name = "productMediaDelete")]
    async fn product_media_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<ProductMediaDelete> {

        Some(ProductMediaDelete { product: None, errors: vec![] })

    }

    #[graphql(name = "productMediaBulkDelete")]
    async fn product_media_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<ProductMediaBulkDelete> {

        Some(ProductMediaBulkDelete { count: None, errors: vec![] })

    }

    #[graphql(name = "productMediaReorder")]
    async fn product_media_reorder(&self, #[graphql(name = "mediaIds")] _arg_media_ids: Vec<ID>, #[graphql(name = "productId")] _arg_product_id: ID) -> Option<ProductMediaReorder> {

        Some(ProductMediaReorder { product: None, errors: vec![] })

    }

    #[graphql(name = "productMediaUpdate")]
    async fn product_media_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: ProductMediaUpdateInput) -> Option<ProductMediaUpdate> {

        Some(ProductMediaUpdate { product: None, errors: vec![] })

    }

    #[graphql(name = "productTypeCreate")]
    async fn product_type_create(&self, #[graphql(name = "input")] _arg_input: ProductTypeInput) -> Option<ProductTypeCreate> {

        Some(ProductTypeCreate { errors: vec![], product_type: None })

    }

    #[graphql(name = "productTypeDelete")]
    async fn product_type_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<ProductTypeDelete> {

        Some(ProductTypeDelete { errors: vec![], product_type: None })

    }

    #[graphql(name = "productTypeBulkDelete")]
    async fn product_type_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<ProductTypeBulkDelete> {

        Some(ProductTypeBulkDelete { errors: vec![] })

    }

    #[graphql(name = "productTypeUpdate")]
    async fn product_type_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: ProductTypeInput) -> Option<ProductTypeUpdate> {

        Some(ProductTypeUpdate { errors: vec![], product_type: None })

    }

    #[graphql(name = "productTypeReorderAttributes")]
    async fn product_type_reorder_attributes(&self, #[graphql(name = "moves")] _arg_moves: Vec<ReorderInput>, #[graphql(name = "productTypeId")] _arg_product_type_id: ID, #[graphql(name = "type")] _arg_type: ProductAttributeType) -> Option<ProductTypeReorderAttributes> {

        Some(ProductTypeReorderAttributes { product_type: None, errors: vec![] })

    }

    #[graphql(name = "productVariantStocksDelete")]
    async fn product_variant_stocks_delete(&self, #[graphql(name = "sku")] _arg_sku: Option<String>, #[graphql(name = "variantId")] _arg_variant_id: Option<ID>, #[graphql(name = "warehouseIds")] _arg_warehouse_ids: Option<Vec<ID>>) -> Option<ProductVariantStocksDelete> {

        Some(ProductVariantStocksDelete { product_variant: None, errors: vec![] })

    }

    #[graphql(name = "productVariantSetDefault")]
    async fn product_variant_set_default(&self, #[graphql(name = "productId")] _arg_product_id: ID, #[graphql(name = "variantId")] _arg_variant_id: ID) -> Option<ProductVariantSetDefault> {

        Some(ProductVariantSetDefault { product: None, errors: vec![] })

    }

    #[graphql(name = "variantMediaAssign")]
    async fn variant_media_assign(&self, #[graphql(name = "mediaId")] _arg_media_id: ID, #[graphql(name = "variantId")] _arg_variant_id: ID) -> Option<VariantMediaAssign> {

        Some(VariantMediaAssign { product_variant: None, errors: vec![] })

    }

    #[graphql(name = "variantMediaUnassign")]
    async fn variant_media_unassign(&self, #[graphql(name = "mediaId")] _arg_media_id: ID, #[graphql(name = "variantId")] _arg_variant_id: ID) -> Option<VariantMediaUnassign> {

        Some(VariantMediaUnassign { product_variant: None, errors: vec![] })

    }

    #[graphql(name = "transactionCreate")]
    async fn transaction_create(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "transaction")] _arg_transaction: TransactionCreateInput, #[graphql(name = "transactionEvent")] _arg_transaction_event: Option<TransactionEventInput>) -> Option<TransactionCreate> {

        Some(TransactionCreate { transaction: None, errors: vec![] })

    }

    #[graphql(name = "transactionRequestAction")]
    async fn transaction_request_action(&self, #[graphql(name = "actionType")] _arg_action_type: TransactionActionEnum, #[graphql(name = "amount")] _arg_amount: Option<GenPositiveDecimal>, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "refundReason")] _arg_refund_reason: Option<String>, #[graphql(name = "refundReasonReference")] _arg_refund_reason_reference: Option<ID>, #[graphql(name = "token")] _arg_token: Option<String>) -> Option<TransactionRequestAction> {

        Some(TransactionRequestAction { transaction: None, errors: vec![] })

    }

    #[graphql(name = "transactionRequestRefundForGrantedRefund")]
    async fn transaction_request_refund_for_granted_refund(&self, #[graphql(name = "grantedRefundId")] _arg_granted_refund_id: ID, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "token")] _arg_token: Option<String>) -> Option<TransactionRequestRefundForGrantedRefund> {

        Some(TransactionRequestRefundForGrantedRefund { transaction: None, errors: vec![] })

    }

    #[graphql(name = "pageCreate")]
    async fn page_create(&self, #[graphql(name = "input")] _arg_input: PageCreateInput) -> Option<PageCreate> {

        Some(PageCreate { errors: vec![], page: None })

    }

    #[graphql(name = "pageDelete")]
    async fn page_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<PageDelete> {

        Some(PageDelete { errors: vec![] })

    }

    #[graphql(name = "pageBulkDelete")]
    async fn page_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<PageBulkDelete> {

        Some(PageBulkDelete { errors: vec![] })

    }

    #[graphql(name = "pageBulkPublish")]
    async fn page_bulk_publish(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>, #[graphql(name = "isPublished")] _arg_is_published: bool) -> Option<PageBulkPublish> {

        Some(PageBulkPublish { errors: vec![] })

    }

    #[graphql(name = "pageUpdate")]
    async fn page_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: PageInput) -> Option<PageUpdate> {

        Some(PageUpdate { errors: vec![], page: None })

    }

    #[graphql(name = "pageTypeCreate")]
    async fn page_type_create(&self, #[graphql(name = "input")] _arg_input: PageTypeCreateInput) -> Option<PageTypeCreate> {

        Some(PageTypeCreate { errors: vec![], page_type: None })

    }

    #[graphql(name = "pageTypeUpdate")]
    async fn page_type_update(&self, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "input")] _arg_input: PageTypeUpdateInput) -> Option<PageTypeUpdate> {

        Some(PageTypeUpdate { errors: vec![], page_type: None })

    }

    #[graphql(name = "pageTypeDelete")]
    async fn page_type_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<PageTypeDelete> {

        Some(PageTypeDelete { errors: vec![], page_type: None })

    }

    #[graphql(name = "pageTypeBulkDelete")]
    async fn page_type_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<PageTypeBulkDelete> {

        Some(PageTypeBulkDelete { errors: vec![] })

    }

    #[graphql(name = "pageAttributeAssign")]
    async fn page_attribute_assign(&self, #[graphql(name = "attributeIds")] _arg_attribute_ids: Vec<ID>, #[graphql(name = "pageTypeId")] _arg_page_type_id: ID) -> Option<PageAttributeAssign> {

        Some(PageAttributeAssign { page_type: None, errors: vec![] })

    }

    #[graphql(name = "pageAttributeUnassign")]
    async fn page_attribute_unassign(&self, #[graphql(name = "attributeIds")] _arg_attribute_ids: Vec<ID>, #[graphql(name = "pageTypeId")] _arg_page_type_id: ID) -> Option<PageAttributeUnassign> {

        Some(PageAttributeUnassign { page_type: None, errors: vec![] })

    }

    #[graphql(name = "pageTypeReorderAttributes")]
    async fn page_type_reorder_attributes(&self, #[graphql(name = "moves")] _arg_moves: Vec<ReorderInput>, #[graphql(name = "pageTypeId")] _arg_page_type_id: ID) -> Option<PageTypeReorderAttributes> {

        Some(PageTypeReorderAttributes { page_type: None, errors: vec![] })

    }

    #[graphql(name = "draftOrderComplete")]
    async fn draft_order_complete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<DraftOrderComplete> {

        Some(DraftOrderComplete { order: None, errors: vec![] })

    }

    #[graphql(name = "draftOrderCreate")]
    async fn draft_order_create(&self, #[graphql(name = "input")] _arg_input: DraftOrderCreateInput) -> Option<DraftOrderCreate> {

        Some(DraftOrderCreate { errors: vec![], order: None })

    }

    #[graphql(name = "draftOrderDelete")]
    async fn draft_order_delete(&self, #[graphql(name = "externalReference")] _arg_external_reference: Option<String>, #[graphql(name = "id")] _arg_id: Option<ID>) -> Option<DraftOrderDelete> {

        Some(DraftOrderDelete { errors: vec![] })

    }

    #[graphql(name = "draftOrderBulkDelete")]
    async fn draft_order_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<DraftOrderBulkDelete> {

        Some(DraftOrderBulkDelete { errors: vec![] })

    }

    #[graphql(name = "draftOrderUpdate")]
    async fn draft_order_update(&self, #[graphql(name = "externalReference")] _arg_external_reference: Option<String>, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "input")] _arg_input: DraftOrderInput) -> Option<DraftOrderUpdate> {

        Some(DraftOrderUpdate { errors: vec![], order: None })

    }

    #[graphql(name = "orderCapture")]
    async fn order_capture(&self, #[graphql(name = "amount")] _arg_amount: GenPositiveDecimal, #[graphql(name = "id")] _arg_id: ID) -> Option<OrderCapture> {

        Some(OrderCapture { order: None, errors: vec![] })

    }

    #[graphql(name = "orderConfirm")]
    async fn order_confirm(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<OrderConfirm> {

        Some(OrderConfirm { order: None, errors: vec![] })

    }

    #[graphql(name = "orderFulfillmentCancel")]
    async fn order_fulfillment_cancel(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: Option<FulfillmentCancelInput>) -> Option<FulfillmentCancel> {

        Some(FulfillmentCancel { order: None, errors: vec![] })

    }

    #[graphql(name = "orderFulfillmentApprove")]
    async fn order_fulfillment_approve(&self, #[graphql(name = "allowStockToBeExceeded")] _arg_allow_stock_to_be_exceeded: Option<bool>, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "notifyCustomer")] _arg_notify_customer: bool) -> Option<FulfillmentApprove> {

        Some(FulfillmentApprove { order: None, errors: vec![] })

    }

    #[graphql(name = "orderFulfillmentUpdateTracking")]
    async fn order_fulfillment_update_tracking(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: FulfillmentUpdateTrackingInput) -> Option<FulfillmentUpdateTracking> {

        Some(FulfillmentUpdateTracking { order: None, errors: vec![] })

    }

    #[graphql(name = "orderFulfillmentRefundProducts")]
    async fn order_fulfillment_refund_products(&self, #[graphql(name = "input")] _arg_input: OrderRefundProductsInput, #[graphql(name = "order")] _arg_order: ID) -> Option<FulfillmentRefundProducts> {

        Some(FulfillmentRefundProducts { fulfillment: None, order: None, errors: vec![] })

    }

    #[graphql(name = "orderFulfillmentReturnProducts")]
    async fn order_fulfillment_return_products(&self, #[graphql(name = "input")] _arg_input: OrderReturnProductsInput, #[graphql(name = "order")] _arg_order: ID) -> Option<FulfillmentReturnProducts> {

        Some(FulfillmentReturnProducts { order: None, replace_order: None, errors: vec![] })

    }

    #[graphql(name = "orderGrantRefundCreate")]
    async fn order_grant_refund_create(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: OrderGrantRefundCreateInput) -> Option<OrderGrantRefundCreate> {

        Some(OrderGrantRefundCreate { order: None, granted_refund: None, errors: vec![] })

    }

    #[graphql(name = "orderGrantRefundUpdate")]
    async fn order_grant_refund_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: OrderGrantRefundUpdateInput) -> Option<OrderGrantRefundUpdate> {

        Some(OrderGrantRefundUpdate { order: None, errors: vec![] })

    }

    #[graphql(name = "orderLinesCreate")]
    async fn order_lines_create(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: Vec<OrderLineCreateInput>) -> Option<OrderLinesCreate> {

        Some(OrderLinesCreate { order: None, errors: vec![] })

    }

    #[graphql(name = "orderLineDelete")]
    async fn order_line_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<OrderLineDelete> {

        Some(OrderLineDelete { order: None, errors: vec![] })

    }

    #[graphql(name = "orderLineUpdate")]
    async fn order_line_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: OrderLineInput) -> Option<OrderLineUpdate> {

        Some(OrderLineUpdate { order: None, errors: vec![], order_line: None })

    }

    #[graphql(name = "orderDiscountAdd")]
    async fn order_discount_add(&self, #[graphql(name = "input")] _arg_input: OrderDiscountCommonInput, #[graphql(name = "orderId")] _arg_order_id: ID) -> Option<OrderDiscountAdd> {

        Some(OrderDiscountAdd { order: None, errors: vec![] })

    }

    #[graphql(name = "orderDiscountUpdate")]
    async fn order_discount_update(&self, #[graphql(name = "discountId")] _arg_discount_id: ID, #[graphql(name = "input")] _arg_input: OrderDiscountCommonInput) -> Option<OrderDiscountUpdate> {

        Some(OrderDiscountUpdate { order: None, errors: vec![] })

    }

    #[graphql(name = "orderDiscountDelete")]
    async fn order_discount_delete(&self, #[graphql(name = "discountId")] _arg_discount_id: ID) -> Option<OrderDiscountDelete> {

        Some(OrderDiscountDelete { order: None, errors: vec![] })

    }

    #[graphql(name = "orderLineDiscountUpdate")]
    async fn order_line_discount_update(&self, #[graphql(name = "input")] _arg_input: OrderDiscountCommonInput, #[graphql(name = "orderLineId")] _arg_order_line_id: ID) -> Option<OrderLineDiscountUpdate> {

        Some(OrderLineDiscountUpdate { order: None, errors: vec![] })

    }

    #[graphql(name = "orderLineDiscountRemove")]
    async fn order_line_discount_remove(&self, #[graphql(name = "orderLineId")] _arg_order_line_id: ID) -> Option<OrderLineDiscountRemove> {

        Some(OrderLineDiscountRemove { order: None, errors: vec![] })

    }

    #[graphql(name = "orderNoteAdd")]
    async fn order_note_add(&self, #[graphql(name = "order")] _arg_order: ID, #[graphql(name = "input")] _arg_input: OrderNoteInput) -> Option<OrderNoteAdd> {

        Some(OrderNoteAdd { order: None, errors: vec![] })

    }

    #[graphql(name = "orderNoteUpdate")]
    async fn order_note_update(&self, #[graphql(name = "note")] _arg_note: ID, #[graphql(name = "input")] _arg_input: OrderNoteInput) -> Option<OrderNoteUpdate> {

        Some(OrderNoteUpdate { order: None, errors: vec![] })

    }

    #[graphql(name = "orderMarkAsPaid")]
    async fn order_mark_as_paid(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "transactionReference")] _arg_transaction_reference: Option<String>) -> Option<OrderMarkAsPaid> {

        Some(OrderMarkAsPaid { order: None, errors: vec![] })

    }

    #[graphql(name = "orderRefund")]
    async fn order_refund(&self, #[graphql(name = "amount")] _arg_amount: GenPositiveDecimal, #[graphql(name = "id")] _arg_id: ID) -> Option<OrderRefund> {

        Some(OrderRefund { order: None, errors: vec![] })

    }

    #[graphql(name = "orderUpdate")]
    async fn order_update(&self, #[graphql(name = "externalReference")] _arg_external_reference: Option<String>, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "input")] _arg_input: OrderUpdateInput) -> Option<OrderUpdate> {

        Some(OrderUpdate { errors: vec![], order: None })

    }

    #[graphql(name = "orderUpdateShipping")]
    async fn order_update_shipping(&self, #[graphql(name = "order")] _arg_order: ID, #[graphql(name = "input")] _arg_input: OrderUpdateShippingInput) -> Option<OrderUpdateShipping> {

        Some(OrderUpdateShipping { order: None, errors: vec![] })

    }

    #[graphql(name = "orderVoid")]
    async fn order_void(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<OrderVoid> {

        Some(OrderVoid { order: None, errors: vec![] })

    }

    #[graphql(name = "menuCreate")]
    async fn menu_create(&self, #[graphql(name = "input")] _arg_input: MenuCreateInput) -> Option<MenuCreate> {

        Some(MenuCreate { errors: vec![], menu: None })

    }

    #[graphql(name = "menuDelete")]
    async fn menu_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<MenuDelete> {

        Some(MenuDelete { errors: vec![] })

    }

    #[graphql(name = "menuBulkDelete")]
    async fn menu_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<MenuBulkDelete> {

        Some(MenuBulkDelete { errors: vec![] })

    }

    #[graphql(name = "menuUpdate")]
    async fn menu_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: MenuInput) -> Option<MenuUpdate> {

        Some(MenuUpdate { errors: vec![] })

    }

    #[graphql(name = "menuItemCreate")]
    async fn menu_item_create(&self, #[graphql(name = "input")] _arg_input: MenuItemCreateInput) -> Option<MenuItemCreate> {

        Some(MenuItemCreate { errors: vec![], menu_item: None })

    }

    #[graphql(name = "menuItemBulkDelete")]
    async fn menu_item_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<MenuItemBulkDelete> {

        Some(MenuItemBulkDelete { errors: vec![] })

    }

    #[graphql(name = "menuItemUpdate")]
    async fn menu_item_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: MenuItemInput) -> Option<MenuItemUpdate> {

        Some(MenuItemUpdate { errors: vec![], menu_item: None })

    }

    #[graphql(name = "menuItemMove")]
    async fn menu_item_move(&self, #[graphql(name = "menu")] _arg_menu: ID, #[graphql(name = "moves")] _arg_moves: Vec<MenuItemMoveInput>) -> Option<MenuItemMove> {

        Some(MenuItemMove { errors: vec![] })

    }

    #[graphql(name = "invoiceRequest")]
    async fn invoice_request(&self, #[graphql(name = "number")] _arg_number: Option<String>, #[graphql(name = "orderId")] _arg_order_id: ID) -> Option<InvoiceRequest> {

        Some(InvoiceRequest { order: None, errors: vec![], invoice: None })

    }

    #[graphql(name = "invoiceSendNotification")]
    async fn invoice_send_notification(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<InvoiceSendNotification> {

        Some(InvoiceSendNotification { errors: vec![], invoice: None })

    }

    #[graphql(name = "giftCardActivate")]
    async fn gift_card_activate(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<GiftCardActivate> {

        Some(GiftCardActivate { gift_card: None, errors: vec![] })

    }

    #[graphql(name = "giftCardAssignUser")]
    async fn gift_card_assign_user(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "userId")] _arg_user_id: ID) -> Option<GiftCardAssignUser> {

        Some(GiftCardAssignUser { gift_card: None, errors: vec![] })

    }

    #[graphql(name = "giftCardUnassignUser")]
    async fn gift_card_unassign_user(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<GiftCardUnassignUser> {

        Some(GiftCardUnassignUser { gift_card: None, errors: vec![] })

    }

    #[graphql(name = "giftCardCreate")]
    async fn gift_card_create(&self, #[graphql(name = "input")] _arg_input: GiftCardCreateInput) -> Option<GiftCardCreate> {

        Some(GiftCardCreate { errors: vec![], gift_card: None })

    }

    #[graphql(name = "giftCardDelete")]
    async fn gift_card_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<GiftCardDelete> {

        Some(GiftCardDelete { errors: vec![] })

    }

    #[graphql(name = "giftCardDeactivate")]
    async fn gift_card_deactivate(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<GiftCardDeactivate> {

        Some(GiftCardDeactivate { gift_card: None, errors: vec![] })

    }

    #[graphql(name = "giftCardUpdate")]
    async fn gift_card_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: GiftCardUpdateInput) -> Option<GiftCardUpdate> {

        Some(GiftCardUpdate { errors: vec![], gift_card: None })

    }

    #[graphql(name = "giftCardResend")]
    async fn gift_card_resend(&self, #[graphql(name = "input")] _arg_input: GiftCardResendInput) -> Option<GiftCardResend> {

        Some(GiftCardResend { gift_card: None, errors: vec![] })

    }

    #[graphql(name = "giftCardAddNote")]
    async fn gift_card_add_note(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: GiftCardAddNoteInput) -> Option<GiftCardAddNote> {

        Some(GiftCardAddNote { gift_card: None, event: None, errors: vec![] })

    }

    #[graphql(name = "giftCardBulkCreate")]
    async fn gift_card_bulk_create(&self, #[graphql(name = "input")] _arg_input: GiftCardBulkCreateInput) -> Option<GiftCardBulkCreate> {

        Some(GiftCardBulkCreate { gift_cards: vec![], errors: vec![] })

    }

    #[graphql(name = "giftCardBulkDelete")]
    async fn gift_card_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<GiftCardBulkDelete> {

        Some(GiftCardBulkDelete { errors: vec![] })

    }

    #[graphql(name = "giftCardBulkActivate")]
    async fn gift_card_bulk_activate(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<GiftCardBulkActivate> {

        Some(GiftCardBulkActivate { count: None, errors: vec![] })

    }

    #[graphql(name = "giftCardBulkDeactivate")]
    async fn gift_card_bulk_deactivate(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<GiftCardBulkDeactivate> {

        Some(GiftCardBulkDeactivate { count: None, errors: vec![] })

    }

    #[graphql(name = "pluginUpdate")]
    async fn plugin_update(&self, #[graphql(name = "channelId")] _arg_channel_id: Option<ID>, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: PluginUpdateInput) -> Option<PluginUpdate> {

        Some(PluginUpdate { plugin: None, errors: vec![] })

    }

    #[graphql(name = "promotionCreate")]
    async fn promotion_create(&self, #[graphql(name = "input")] _arg_input: PromotionCreateInput) -> Option<PromotionCreate> {

        Some(PromotionCreate { errors: vec![], promotion: None })

    }

    #[graphql(name = "promotionUpdate")]
    async fn promotion_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: PromotionUpdateInput) -> Option<PromotionUpdate> {

        Some(PromotionUpdate { errors: vec![], promotion: None })

    }

    #[graphql(name = "promotionDelete")]
    async fn promotion_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<PromotionDelete> {

        Some(PromotionDelete { errors: vec![] })

    }

    #[graphql(name = "promotionRuleCreate")]
    async fn promotion_rule_create(&self, #[graphql(name = "input")] _arg_input: PromotionRuleCreateInput) -> Option<PromotionRuleCreate> {

        Some(PromotionRuleCreate { errors: vec![], promotion_rule: None })

    }

    #[graphql(name = "promotionRuleUpdate")]
    async fn promotion_rule_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: PromotionRuleUpdateInput) -> Option<PromotionRuleUpdate> {

        Some(PromotionRuleUpdate { errors: vec![], promotion_rule: None })

    }

    #[graphql(name = "promotionRuleDelete")]
    async fn promotion_rule_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<PromotionRuleDelete> {

        Some(PromotionRuleDelete { errors: vec![], promotion_rule: None })

    }

    #[graphql(name = "voucherCreate")]
    async fn voucher_create(&self, #[graphql(name = "input")] _arg_input: VoucherInput) -> Option<VoucherCreate> {

        Some(VoucherCreate { errors: vec![], voucher: None })

    }

    #[graphql(name = "voucherDelete")]
    async fn voucher_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<VoucherDelete> {

        Some(VoucherDelete { errors: vec![] })

    }

    #[graphql(name = "voucherBulkDelete")]
    async fn voucher_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<VoucherBulkDelete> {

        Some(VoucherBulkDelete { errors: vec![] })

    }

    #[graphql(name = "voucherUpdate")]
    async fn voucher_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: VoucherInput) -> Option<VoucherUpdate> {

        Some(VoucherUpdate { errors: vec![], voucher: None })

    }

    #[graphql(name = "voucherCataloguesAdd")]
    async fn voucher_catalogues_add(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: CatalogueInput) -> Option<VoucherAddCatalogues> {

        Some(VoucherAddCatalogues { voucher: None, errors: vec![] })

    }

    #[graphql(name = "voucherCataloguesRemove")]
    async fn voucher_catalogues_remove(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: CatalogueInput) -> Option<VoucherRemoveCatalogues> {

        Some(VoucherRemoveCatalogues { voucher: None, errors: vec![] })

    }

    #[graphql(name = "voucherChannelListingUpdate")]
    async fn voucher_channel_listing_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: VoucherChannelListingInput) -> Option<VoucherChannelListingUpdate> {

        Some(VoucherChannelListingUpdate { voucher: None, errors: vec![] })

    }

    #[graphql(name = "voucherCodeBulkDelete")]
    async fn voucher_code_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<VoucherCodeBulkDelete> {

        Some(VoucherCodeBulkDelete { count: None, errors: vec![] })

    }

    #[graphql(name = "exportProducts")]
    async fn export_products(&self, #[graphql(name = "input")] _arg_input: ExportProductsInput) -> Option<ExportProducts> {

        Some(ExportProducts { export_file: None, errors: vec![] })

    }

    #[graphql(name = "fileUpload")]
    async fn file_upload(&self, #[graphql(name = "file")] _arg_file: GenUpload) -> Option<FileUpload> {

        Some(FileUpload { uploaded_file: None, errors: vec![] })

    }

    #[graphql(name = "channelCreate")]
    async fn channel_create(&self, #[graphql(name = "input")] _arg_input: ChannelCreateInput) -> Option<ChannelCreate> {

        Some(ChannelCreate { errors: vec![], channel: None })

    }

    #[graphql(name = "channelUpdate")]
    async fn channel_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: ChannelUpdateInput) -> Option<ChannelUpdate> {

        Some(ChannelUpdate { errors: vec![], channel: None })

    }

    #[graphql(name = "channelDelete")]
    async fn channel_delete(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: Option<ChannelDeleteInput>) -> Option<ChannelDelete> {

        Some(ChannelDelete { errors: vec![] })

    }

    #[graphql(name = "channelActivate")]
    async fn channel_activate(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<ChannelActivate> {

        Some(ChannelActivate { channel: None, errors: vec![] })

    }

    #[graphql(name = "channelDeactivate")]
    async fn channel_deactivate(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<ChannelDeactivate> {

        Some(ChannelDeactivate { channel: None, errors: vec![] })

    }

    #[graphql(name = "channelReorderWarehouses")]
    async fn channel_reorder_warehouses(&self, #[graphql(name = "channelId")] _arg_channel_id: ID, #[graphql(name = "moves")] _arg_moves: Vec<ReorderInput>) -> Option<ChannelReorderWarehouses> {

        Some(ChannelReorderWarehouses { channel: None, errors: vec![] })

    }

    #[graphql(name = "appCreate")]
    async fn app_create(&self, #[graphql(name = "input")] _arg_input: AppInput) -> Option<AppCreate> {

        Some(AppCreate { auth_token: None, errors: vec![], app: None })

    }

    #[graphql(name = "appUpdate")]
    async fn app_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: AppInput) -> Option<AppUpdate> {

        Some(AppUpdate { errors: vec![], app: None })

    }

    #[graphql(name = "appDelete")]
    async fn app_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<AppDelete> {

        Some(AppDelete { errors: vec![], app: None })

    }

    #[graphql(name = "appTokenCreate")]
    async fn app_token_create(&self, #[graphql(name = "input")] _arg_input: AppTokenInput) -> Option<AppTokenCreate> {

        Some(AppTokenCreate { auth_token: None, errors: vec![], app_token: None })

    }

    #[graphql(name = "appTokenDelete")]
    async fn app_token_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<AppTokenDelete> {

        Some(AppTokenDelete { errors: vec![], app_token: None })

    }

    #[graphql(name = "appInstall")]
    async fn app_install(&self, #[graphql(name = "input")] _arg_input: AppInstallInput) -> Option<AppInstall> {

        Some(AppInstall { errors: vec![], app_installation: None })

    }

    #[graphql(name = "appRetryInstall")]
    async fn app_retry_install(&self, #[graphql(name = "activateAfterInstallation")] _arg_activate_after_installation: Option<bool>, #[graphql(name = "id")] _arg_id: ID) -> Option<AppRetryInstall> {

        Some(AppRetryInstall { errors: vec![], app_installation: None })

    }

    #[graphql(name = "appDeleteFailedInstallation")]
    async fn app_delete_failed_installation(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<AppDeleteFailedInstallation> {

        Some(AppDeleteFailedInstallation { errors: vec![], app_installation: None })

    }

    #[graphql(name = "appFetchManifest")]
    async fn app_fetch_manifest(&self, #[graphql(name = "manifestUrl")] _arg_manifest_url: String) -> Option<AppFetchManifest> {

        Some(AppFetchManifest { manifest: None, errors: vec![] })

    }

    #[graphql(name = "appActivate")]
    async fn app_activate(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<AppActivate> {

        Some(AppActivate { errors: vec![] })

    }

    #[graphql(name = "appDeactivate")]
    async fn app_deactivate(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<AppDeactivate> {

        Some(AppDeactivate { errors: vec![] })

    }

    #[graphql(name = "appProblemDismiss")]
    async fn app_problem_dismiss(&self, #[graphql(name = "input")] _arg_input: AppProblemDismissInput) -> Option<AppProblemDismiss> {

        Some(AppProblemDismiss { errors: vec![] })

    }

    #[graphql(name = "externalAuthenticationUrl")]
    async fn external_authentication_url(&self, #[graphql(name = "input")] _arg_input: GenJSONString, #[graphql(name = "pluginId")] _arg_plugin_id: String) -> Option<ExternalAuthenticationUrl> {

        Some(ExternalAuthenticationUrl { authentication_data: None, errors: vec![] })

    }

    #[graphql(name = "externalObtainAccessTokens")]
    async fn external_obtain_access_tokens(&self, #[graphql(name = "input")] _arg_input: GenJSONString, #[graphql(name = "pluginId")] _arg_plugin_id: String) -> Option<ExternalObtainAccessTokens> {

        Some(ExternalObtainAccessTokens { token: None, refresh_token: None, user: None, errors: vec![] })

    }

    #[graphql(name = "externalRefresh")]
    async fn external_refresh(&self, #[graphql(name = "input")] _arg_input: GenJSONString, #[graphql(name = "pluginId")] _arg_plugin_id: String) -> Option<ExternalRefresh> {

        Some(ExternalRefresh { token: None, refresh_token: None, user: None, errors: vec![] })

    }

    #[graphql(name = "externalLogout")]
    async fn external_logout(&self, #[graphql(name = "input")] _arg_input: GenJSONString, #[graphql(name = "pluginId")] _arg_plugin_id: String) -> Option<ExternalLogout> {

        Some(ExternalLogout { logout_data: None, errors: vec![] })

    }

    #[graphql(name = "requestPasswordReset")]
    async fn request_password_reset(&self, #[graphql(name = "channel")] _arg_channel: Option<String>, #[graphql(name = "email")] _arg_email: String, #[graphql(name = "redirectUrl")] _arg_redirect_url: String) -> Option<RequestPasswordReset> {

        Some(RequestPasswordReset { errors: vec![] })

    }

    #[graphql(name = "setPassword")]
    async fn set_password(&self, #[graphql(name = "email")] _arg_email: String, #[graphql(name = "password")] _arg_password: String, #[graphql(name = "token")] _arg_token: String) -> Option<SetPassword> {

        Some(SetPassword { token: None, refresh_token: None, user: None, errors: vec![] })

    }

    #[graphql(name = "passwordChange")]
    async fn password_change(&self, #[graphql(name = "newPassword")] _arg_new_password: String, #[graphql(name = "oldPassword")] _arg_old_password: Option<String>) -> Option<PasswordChange> {

        Some(PasswordChange { errors: vec![] })

    }

    #[graphql(name = "addressCreate")]
    async fn address_create(&self, #[graphql(name = "input")] _arg_input: AddressInput, #[graphql(name = "userId")] _arg_user_id: ID) -> Option<AddressCreate> {

        Some(AddressCreate { user: None, errors: vec![], address: None })

    }

    #[graphql(name = "addressUpdate")]
    async fn address_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: AddressInput) -> Option<AddressUpdate> {

        Some(AddressUpdate { errors: vec![], address: None })

    }

    #[graphql(name = "addressDelete")]
    async fn address_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<AddressDelete> {

        Some(AddressDelete { user: None, errors: vec![] })

    }

    #[graphql(name = "addressSetDefault")]
    async fn address_set_default(&self, #[graphql(name = "addressId")] _arg_address_id: ID, #[graphql(name = "type")] _arg_type: AddressTypeEnum, #[graphql(name = "userId")] _arg_user_id: ID) -> Option<AddressSetDefault> {

        Some(AddressSetDefault { user: None, errors: vec![] })

    }

    #[graphql(name = "customerCreate")]
    async fn customer_create(&self, #[graphql(name = "input")] _arg_input: UserCreateInput) -> Option<CustomerCreate> {

        Some(CustomerCreate { errors: vec![], user: None })

    }

    #[graphql(name = "customerUpdate")]
    async fn customer_update(&self, #[graphql(name = "externalReference")] _arg_external_reference: Option<String>, #[graphql(name = "id")] _arg_id: Option<ID>, #[graphql(name = "input")] _arg_input: CustomerInput) -> Option<CustomerUpdate> {

        Some(CustomerUpdate { errors: vec![], user: None })

    }

    #[graphql(name = "customerDelete")]
    async fn customer_delete(&self, #[graphql(name = "externalReference")] _arg_external_reference: Option<String>, #[graphql(name = "id")] _arg_id: Option<ID>) -> Option<CustomerDelete> {

        Some(CustomerDelete { errors: vec![] })

    }

    #[graphql(name = "customerBulkDelete")]
    async fn customer_bulk_delete(&self, #[graphql(name = "ids")] _arg_ids: Vec<ID>) -> Option<CustomerBulkDelete> {

        Some(CustomerBulkDelete { errors: vec![] })

    }

    #[graphql(name = "customerTypeCreate")]
    async fn customer_type_create(&self, #[graphql(name = "input")] _arg_input: CustomerTypeCreateInput) -> Option<CustomerTypeCreate> {

        Some(CustomerTypeCreate { errors: vec![], customer_type: None })

    }

    #[graphql(name = "customerTypeUpdate")]
    async fn customer_type_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: CustomerTypeUpdateInput) -> Option<CustomerTypeUpdate> {

        Some(CustomerTypeUpdate { errors: vec![], customer_type: None })

    }

    #[graphql(name = "customerTypeDelete")]
    async fn customer_type_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<CustomerTypeDelete> {

        Some(CustomerTypeDelete { errors: vec![], customer_type: None })

    }

    #[graphql(name = "customerTypeAssignAttributes")]
    async fn customer_type_assign_attributes(&self, #[graphql(name = "attributeIds")] _arg_attribute_ids: Vec<ID>, #[graphql(name = "customerTypeId")] _arg_customer_type_id: ID) -> Option<CustomerTypeAssignAttributes> {

        Some(CustomerTypeAssignAttributes { customer_type: None, errors: vec![] })

    }

    #[graphql(name = "customerTypeUnassignAttributes")]
    async fn customer_type_unassign_attributes(&self, #[graphql(name = "attributeIds")] _arg_attribute_ids: Vec<ID>, #[graphql(name = "customerTypeId")] _arg_customer_type_id: ID) -> Option<CustomerTypeUnassignAttributes> {

        Some(CustomerTypeUnassignAttributes { customer_type: None, errors: vec![] })

    }

    #[graphql(name = "customerTypeReorderAttributes")]
    async fn customer_type_reorder_attributes(&self, #[graphql(name = "customerTypeId")] _arg_customer_type_id: ID, #[graphql(name = "moves")] _arg_moves: Vec<ReorderInput>) -> Option<CustomerTypeReorderAttributes> {

        Some(CustomerTypeReorderAttributes { customer_type: None, errors: vec![] })

    }

    #[graphql(name = "staffCreate")]
    async fn staff_create(&self, #[graphql(name = "input")] _arg_input: StaffCreateInput) -> Option<StaffCreate> {

        Some(StaffCreate { errors: vec![], user: None })

    }

    #[graphql(name = "staffUpdate")]
    async fn staff_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: StaffUpdateInput) -> Option<StaffUpdate> {

        Some(StaffUpdate { errors: vec![], user: None })

    }

    #[graphql(name = "staffDelete")]
    async fn staff_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<StaffDelete> {

        Some(StaffDelete { errors: vec![] })

    }

    #[graphql(name = "userAvatarUpdate")]
    async fn user_avatar_update(&self, #[graphql(name = "image")] _arg_image: GenUpload) -> Option<UserAvatarUpdate> {

        Some(UserAvatarUpdate { user: None, errors: vec![] })

    }

    #[graphql(name = "userAvatarDelete")]
    async fn user_avatar_delete(&self) -> Option<UserAvatarDelete> {

        Some(UserAvatarDelete { user: None, errors: vec![] })

    }

    #[graphql(name = "permissionGroupCreate")]
    async fn permission_group_create(&self, #[graphql(name = "input")] _arg_input: PermissionGroupCreateInput) -> Option<PermissionGroupCreate> {

        Some(PermissionGroupCreate { errors: vec![], group: None })

    }

    #[graphql(name = "permissionGroupUpdate")]
    async fn permission_group_update(&self, #[graphql(name = "id")] _arg_id: ID, #[graphql(name = "input")] _arg_input: PermissionGroupUpdateInput) -> Option<PermissionGroupUpdate> {

        Some(PermissionGroupUpdate { errors: vec![], group: None })

    }

    #[graphql(name = "permissionGroupDelete")]
    async fn permission_group_delete(&self, #[graphql(name = "id")] _arg_id: ID) -> Option<PermissionGroupDelete> {

        Some(PermissionGroupDelete { errors: vec![] })

    }

}

