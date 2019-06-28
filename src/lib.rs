//! Minimalistic [JSON web token (JWT)][JWT] implementation with focus on type safety
//! and secure cryptographic primitives.
//!
//! # Design choices
//!
//! - JWT signature algorithms (i.e., cryptographic algorithms providing JWT integrity)
//!   are expressed via the [`Algorithm`] trait, which uses fully typed keys and signatures.
//! - [JWT header] is represented by the [`Header`] struct. Notably, `Header` does not
//!   expose the [`alg` field].
//!   Instead, `alg` is filled automatically during token creation, and is compared to the
//!   expected value during verification. (If you do not know the JWT signature algorithm during
//!   verification, you're doing something wrong.) This eliminates the possibility
//!   of [algorithm switching attacks][switching].
//!
//! # Additional features
//!
//! - The crate supports more compact [CBOR] encoding of the claims. The compactly encoded JWTs
//!   have [`cty` field] (content type) in their header set to `"CBOR"`.
//! - The crate supports `EdDSA` algorithm with the Ed25519 elliptic curve, and `ES256K` algorithm
//!   with the secp256k1 elliptic curve.
//!
//! ## Supported algorithms
//!
//! | Algorithm(s) | Feature | Description |
//! |--------------|---------|-------------|
//! | `HS256`, `HS384`, `HS512` | - | Uses pure Rust [`sha2`] crate |
//! | `EdDSA` (Ed25519) | [`exonum-crypto`] | [`libsodium`] binding. Enabled by default |
//! | `EdDSA` (Ed25519) | [`ed25519-dalek`] | Pure Rust implementation |
//! | `ES256K` | [`secp256k1`] | Binding for [`libsecp256k1`] |
//!
//! Standard `RS*`, `PS*` and `ES*` algorithms are not (yet?) implemented. The reasons (besides
//! laziness and non-friendly APIs in the relevant crypto backends) are as follows:
//!
//! - RSA algorithms (i.e., `RS*` and `PS*`) are outdated / produce bloated signatures
//! - Elliptic curves in `ES*` algs use a maybe-something-up-my-sleeve generation procedure
//!   and thus may be backdoored
//!
//! `EdDSA` and `ES256K` algorithms are non-standard. They both work with elliptic curves
//! (Curve25519 and secp256k1; both are widely used in crypto community and believed to be
//! securely generated). These algs have 128-bit security, making them an alternative
//! to `ES256`.
//!
//! [JWT]: https://jwt.io/
//! [switching]: https://auth0.com/blog/critical-vulnerabilities-in-json-web-token-libraries/
//! [JWT header]: https://tools.ietf.org/html/rfc7519#section-5
//! [`alg` field]: https://tools.ietf.org/html/rfc7515#section-4.1.1
//! [`cty` field]: https://tools.ietf.org/html/rfc7515#section-4.1.10
//! [CBOR]: https://tools.ietf.org/html/rfc7049
//! [`sha2`]: https://docs.rs/sha2/
//! [`libsodium`]: https://download.libsodium.org/doc/
//! [`exonum-crypto`]: https://docs.rs/exonum-crypto/
//! [`ed25519-dalek`]: https://doc.dalek.rs/ed25519_dalek/
//! [`secp256k1`]: https://docs.rs/secp256k1/
//! [`libsecp256k1`]: https://github.com/bitcoin-core/secp256k1
//! [`Header`]: struct.Header.html
//! [`Algorithm`]: trait.Algorithm.html
//!
//! # Examples
//!
//! Basic JWT lifecycle:
//!
//! ```
//! # use failure::Error;
//! use chrono::{Duration, Utc};
//! use jwt_compact::{prelude::*, alg::{Hs256, Hs256Key}};
//! use serde_derive::*;
//! use std::convert::TryFrom;
//!
//! /// Custom claims encoded in the token.
//! #[derive(Debug, PartialEq, Serialize, Deserialize)]
//! struct CustomClaims {
//!     /// `sub` is a standard claim which denotes claim subject:
//!     /// https://tools.ietf.org/html/rfc7519#section-4.1.2
//!     #[serde(rename = "sub")]
//!     subject: String,
//! }
//!
//! # fn main() -> Result<(), Error> {
//! // Create a symmetric HMAC key, which will be used both to create and verify tokens.
//! let key = Hs256Key::from(b"super_secret_key_donut_steel" as &[_]);
//! // Create a token.
//! let header = Header {
//!     key_id: Some("my-key".to_owned()),
//!     ..Default::default()
//! };
//! let claims = Claims::new(CustomClaims { subject: "alice".to_owned() })
//!     .set_duration_and_issuance(Duration::days(7))
//!     .set_not_before(Utc::now() - Duration::hours(1));
//! let token_string = Hs256.token(header, &claims, &key)?;
//! println!("token: {}", token_string);
//!
//! // Parse the token.
//! let token = UntrustedToken::try_from(token_string.as_str())?;
//! // Before verifying the token, we might find the key which has signed the token
//! // using the `Header.key_id` field.
//! assert_eq!(token.header().key_id, Some("my-key".to_owned()));
//! // Validate the token integrity.
//! let token: Token<CustomClaims> = Hs256.validate_integrity(&token, &key)?;
//! // Validate additional conditions.
//! token
//!     .claims()
//!     .validate_expiration(TimeOptions::default())?
//!     .validate_maturity(TimeOptions::default())?;
//! // Now, we can extract information from the token (e.g., its subject).
//! let subject = &token.claims().custom.subject;
//! assert_eq!(subject, "alice");
//! # Ok(())
//! # } // end main()
//! ```
//!
//! Compact JWT:
//!
//! ```
//! # use chrono::Duration;
//! # use failure::Error;
//! # use hex_buffer_serde::{Hex as _, HexForm};
//! # use jwt_compact::{prelude::*, alg::{Hs256, Hs256Key}};
//! # use serde_derive::*;
//! # use std::convert::TryFrom;
//! /// Custom claims encoded in the token.
//! #[derive(Debug, PartialEq, Serialize, Deserialize)]
//! struct CustomClaims {
//!     /// `sub` is a standard claim which denotes claim subject:
//!     ///     https://tools.ietf.org/html/rfc7519#section-4.1.2
//!     /// The custom serializer we use allows to efficiently
//!     /// encode the subject in CBOR.
//!     #[serde(rename = "sub", with = "HexForm")]
//!     subject: [u8; 32],
//! }
//!
//! # fn main() -> Result<(), Error> {
//! let key = Hs256Key::from(b"super_secret_key_donut_steel" as &[_]);
//! let claims = Claims::new(CustomClaims { subject: [111; 32] })
//!     .set_duration_and_issuance(Duration::days(7));
//! let token = Hs256.token(Header::default(), &claims, &key)?;
//! println!("token: {}", token);
//! let compact_token = Hs256.compact_token(Header::default(), &claims, &key)?;
//! println!("compact token: {}", compact_token);
//! // The compact token should be ~40 chars shorter.
//!
//! // Parse the compact token.
//! let token = UntrustedToken::try_from(compact_token.as_str())?;
//! let token: Token<CustomClaims> = Hs256.validate_integrity(&token, &key)?;
//! token.claims().validate_expiration(TimeOptions::default())?;
//! // Now, we can extract information from the token (e.g., its subject).
//! assert_eq!(token.claims().custom.subject, [111; 32]);
//! # Ok(())
//! # } // end main()
//! ```

#![deny(missing_debug_implementations, missing_docs, bare_trait_objects)]

use serde::{de::DeserializeOwned, Serialize};
use serde_derive::*;
use smallvec::{smallvec, SmallVec};

use std::{borrow::Cow, convert::TryFrom};

pub mod alg;
mod claims;
mod error;

pub use crate::{
    claims::{Claims, TimeOptions},
    error::{CreationError, ParseError, ValidationError},
};

/// Prelude to neatly import all necessary stuff from the crate.
pub mod prelude {
    pub use crate::{AlgorithmExt as _, Claims, Header, TimeOptions, Token, UntrustedToken};
}

/// Maximum "reasonable" signature size in bytes.
const SIGNATURE_SIZE: usize = 128;

/// Signature for a certain JWT signing `Algorithm`.
///
/// We require that signature can be restored from a byte slice,
/// and can be represented as a byte slice.
pub trait AlgorithmSignature: Sized {
    /// Attempts to restore a signature from a byte slice. This method may fail
    /// if the slice is malformed (e.g., has a wrong length).
    fn try_from_slice(slice: &[u8]) -> Result<Self, failure::Error>;

    /// Represents this signature as bytes.
    fn as_bytes(&self) -> Cow<[u8]>;
}

/// JWT signing algorithm.
pub trait Algorithm {
    /// Key used when issuing new tokens.
    type SigningKey;
    /// Key used when verifying tokens. May coincide with `SigningKey` for symmetric
    /// algorithms (e.g., `HS*`).
    type VerifyingKey;
    /// Signature produced by the algorithm.
    type Signature: AlgorithmSignature;

    /// Name of the algorithm mentioned in the `alg` field of the JWT header.
    const NAME: &'static str;

    /// Signs a `message` with the `signing_key`.
    fn sign(&self, signing_key: &Self::SigningKey, message: &[u8]) -> Self::Signature;

    /// Verifies the `message` against the `signature` and `verifying_key`.
    fn verify_signature(
        &self,
        signature: &Self::Signature,
        verifying_key: &Self::VerifyingKey,
        message: &[u8],
    ) -> bool;
}

/// Automatically implemented extensions of the `Algorithm` trait.
pub trait AlgorithmExt: Algorithm {
    /// Creates a new token and serializes it to string.
    fn token<T>(
        &self,
        header: Header,
        claims: &Claims<T>,
        signing_key: &Self::SigningKey,
    ) -> Result<String, CreationError>
    where
        T: Serialize;

    /// Creates a new token with CBOR-encoded claims and serializes it to string.
    fn compact_token<T>(
        &self,
        header: Header,
        claims: &Claims<T>,
        signing_key: &Self::SigningKey,
    ) -> Result<String, CreationError>
    where
        T: Serialize;

    /// Validates the token integrity against the provided `verifying_key`.
    fn validate_integrity<T>(
        &self,
        token: &UntrustedToken,
        verifying_key: &Self::VerifyingKey,
    ) -> Result<Token<T>, ValidationError>
    where
        T: DeserializeOwned;
}

impl<A: Algorithm> AlgorithmExt for A {
    fn token<T>(
        &self,
        header: Header,
        claims: &Claims<T>,
        signing_key: &Self::SigningKey,
    ) -> Result<String, CreationError>
    where
        T: Serialize,
    {
        let complete_header = CompleteHeader {
            algorithm: Self::NAME.to_owned(),
            content_type: None,
            inner: header,
        };
        let header = serde_json::to_string(&complete_header).map_err(CreationError::Header)?;
        let mut buffer = base64::encode_config(&header, base64::URL_SAFE_NO_PAD);

        buffer.push('.');
        let claims = serde_json::to_string(claims).map_err(CreationError::Claims)?;
        base64::encode_config_buf(&claims, base64::URL_SAFE_NO_PAD, &mut buffer);

        let signature = self.sign(signing_key, buffer.as_bytes());
        buffer.push('.');
        base64::encode_config_buf(
            signature.as_bytes().as_ref(),
            base64::URL_SAFE_NO_PAD,
            &mut buffer,
        );

        Ok(buffer)
    }

    fn compact_token<T>(
        &self,
        header: Header,
        claims: &Claims<T>,
        signing_key: &Self::SigningKey,
    ) -> Result<String, CreationError>
    where
        T: Serialize,
    {
        let complete_header = CompleteHeader {
            algorithm: Self::NAME.to_owned(),
            content_type: Some("CBOR".to_owned()),
            inner: header,
        };
        let header = serde_json::to_string(&complete_header).map_err(CreationError::Header)?;
        let mut buffer = base64::encode_config(&header, base64::URL_SAFE_NO_PAD);

        buffer.push('.');
        let claims = serde_cbor::to_vec(claims).map_err(CreationError::CborClaims)?;
        base64::encode_config_buf(&claims, base64::URL_SAFE_NO_PAD, &mut buffer);

        let signature = self.sign(signing_key, buffer.as_bytes());
        buffer.push('.');
        base64::encode_config_buf(
            signature.as_bytes().as_ref(),
            base64::URL_SAFE_NO_PAD,
            &mut buffer,
        );

        Ok(buffer)
    }

    fn validate_integrity<T>(
        &self,
        token: &UntrustedToken,
        verifying_key: &Self::VerifyingKey,
    ) -> Result<Token<T>, ValidationError>
    where
        T: DeserializeOwned,
    {
        if Self::NAME != token.algorithm {
            return Err(ValidationError::AlgorithmMismatch);
        }

        let signature = Self::Signature::try_from_slice(&token.signature[..])
            .map_err(ValidationError::MalformedSignature)?;
        // We assume that parsing claims is less computationally demanding than
        // validating a signature.
        let claims: Claims<T> = match token.content_type {
            ContentType::Json => serde_json::from_slice(&token.serialized_claims)
                .map_err(ValidationError::MalformedClaims)?,
            ContentType::Cbor => serde_cbor::from_slice(&token.serialized_claims)
                .map_err(ValidationError::MalformedCborClaims)?,
        };
        if !self.verify_signature(&signature, verifying_key, token.signed_data) {
            return Err(ValidationError::InvalidSignature);
        }

        Ok(Token {
            header: token.header.clone(),
            claims,
        })
    }
}

/// JWT header.
///
/// See [RFC 7515](https://tools.ietf.org/html/rfc7515#section-4.1) for the description
/// of the fields. The purpose of all fields except `signature_type` is to determine
/// the verifying key. Since these values will be provided by the adversary in the case of
/// an attack, they require additional verification (e.g., a provided certificate might
/// be checked against the list of "acceptable" certificate authorities).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Header {
    /// URL of the JSON Web Key Set containing the key that has signed the token.
    /// This field is renamed to `jku` for serialization.
    #[serde(rename = "jku", default, skip_serializing_if = "Option::is_none")]
    pub key_set_url: Option<String>,

    /// Identifier of the key that has signed the token. This field is renamed to `kid`
    /// for serialization.
    #[serde(rename = "kid", default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,

    /// URL of the X.509 certificate for the signing key. This field is renamed to `x5u`
    /// for serialization.
    #[serde(rename = "x5u", default, skip_serializing_if = "Option::is_none")]
    pub certificate_url: Option<String>,

    /// Thumbprint of the X.509 certificate for the signing key. This field is renamed to `x5t`
    /// for serialization.
    #[serde(rename = "x5t", default, skip_serializing_if = "Option::is_none")]
    pub certificate_thumbprint: Option<String>,

    /// Application-specific signature type. This field is renamed to `typ` for serialization.
    #[serde(rename = "typ", default, skip_serializing_if = "Option::is_none")]
    pub signature_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompleteHeader {
    #[serde(rename = "alg")]
    algorithm: String,

    #[serde(rename = "cty", default, skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,

    #[serde(flatten)]
    inner: Header,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentType {
    Json,
    Cbor,
}

/// Parsed, but unvalidated token.
#[derive(Debug, Clone)]
pub struct UntrustedToken<'a> {
    signed_data: &'a [u8],
    header: Header,
    algorithm: String,
    content_type: ContentType,
    serialized_claims: Vec<u8>,
    signature: SmallVec<[u8; SIGNATURE_SIZE]>,
}

/// Token with validated integrity.
///
/// Claims encoded in the token can be verified by invoking [`Claims`] methods
/// via [`claims()`] getter.
///
/// [`Claims`]: struct.Claims.html
/// [`claims()`]: #fn.claims
#[derive(Debug, Clone)]
pub struct Token<T> {
    header: Header,
    claims: Claims<T>,
}

impl<T> Token<T> {
    /// Gets token header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Gets token claims.
    pub fn claims(&self) -> &Claims<T> {
        &self.claims
    }
}

impl<'a> TryFrom<&'a str> for UntrustedToken<'a> {
    type Error = ParseError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let token_parts: Vec<_> = s.splitn(4, '.').collect();
        match &token_parts[..] {
            [header, claims, signature] => {
                let header = base64::decode_config(header, base64::URL_SAFE_NO_PAD)?;
                let serialized_claims = base64::decode_config(claims, base64::URL_SAFE_NO_PAD)?;
                let mut decoded_signature = smallvec![0; 3 * (signature.len() + 3) / 4];
                let signature_len = base64::decode_config_slice(
                    signature,
                    base64::URL_SAFE_NO_PAD,
                    &mut decoded_signature[..],
                )?;
                decoded_signature.truncate(signature_len);

                let header: CompleteHeader =
                    serde_json::from_slice(&header).map_err(ParseError::MalformedHeader)?;
                let content_type = match header.content_type {
                    None => ContentType::Json,
                    Some(ref s) if s.eq_ignore_ascii_case("json") => ContentType::Json,
                    Some(ref s) if s.eq_ignore_ascii_case("cbor") => ContentType::Cbor,
                    Some(s) => return Err(ParseError::UnsupportedContentType(s)),
                };

                Ok(Self {
                    signed_data: s.rsplitn(2, '.').nth(1).unwrap().as_bytes(),
                    header: header.inner,
                    algorithm: header.algorithm,
                    content_type,
                    serialized_claims,
                    signature: decoded_signature,
                })
            }
            _ => Err(ParseError::InvalidTokenStructure),
        }
    }
}

impl<'a> UntrustedToken<'a> {
    /// Gets the token header.
    pub fn header(&self) -> &Header {
        &self.header
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alg::*;

    use assert_matches::assert_matches;
    use chrono::{Duration, Utc};
    use hex_buffer_serde::{Hex as _, HexForm};
    use rand::thread_rng;
    use serde_json::json;

    use std::collections::HashMap;

    type Obj = serde_json::Map<String, serde_json::Value>;

    const HS256_TOKEN: &str = "eyJ0eXAiOiJKV1QiLA0KICJhbGciOiJIUzI1NiJ9.\
                               eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFt\
                               cGxlLmNvbS9pc19yb290Ijp0cnVlfQ.\
                               dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const HS256_KEY: &str = "AyM1SysPpbyDfgZld3umj1qzKObwVMkoqQ-EstJQLr_T-1qS0gZH75\
                             aKtMN3Yj0iPS4hcgUuTwjAzZr1Z9CAow";

    #[test]
    fn hs256_reference() {
        //! Example from https://tools.ietf.org/html/rfc7515#appendix-A.1

        let token = UntrustedToken::try_from(HS256_TOKEN).unwrap();
        assert_eq!(token.algorithm, "HS256");

        let key = base64::decode_config(HS256_KEY, base64::URL_SAFE_NO_PAD).unwrap();
        let key = Hs256Key::from(&*key);
        let token = Hs256.validate_integrity::<Obj>(&token, &key).unwrap();
        assert_eq!(
            token.claims().expiration_date.unwrap().timestamp(),
            1_300_819_380
        );
        assert_eq!(token.claims().custom["iss"], json!("joe"));
        assert_eq!(
            token.claims().custom["http://example.com/is_root"],
            json!(true)
        );
    }

    #[test]
    fn invalid_token_structure() {
        let mangled_str = HS256_TOKEN.replace('.', "");
        assert_matches!(
            UntrustedToken::try_from(mangled_str.as_str()).unwrap_err(),
            ParseError::InvalidTokenStructure
        );

        let mut mangled_str = HS256_TOKEN.to_owned();
        let signature_start = mangled_str.rfind('.').unwrap();
        mangled_str.truncate(signature_start);
        assert_matches!(
            UntrustedToken::try_from(mangled_str.as_str()).unwrap_err(),
            ParseError::InvalidTokenStructure
        );

        let mut mangled_str = HS256_TOKEN.to_owned();
        mangled_str.push('.');
        assert_matches!(
            UntrustedToken::try_from(mangled_str.as_str()).unwrap_err(),
            ParseError::InvalidTokenStructure
        );
    }

    #[test]
    fn base64_error_during_parsing() {
        let mangled_str = HS256_TOKEN.replace('0', "+");
        assert_matches!(
            UntrustedToken::try_from(mangled_str.as_str()).unwrap_err(),
            ParseError::Base64(_)
        );

        let mut mangled_str = HS256_TOKEN.to_owned();
        mangled_str.truncate(mangled_str.len() - 1);
        assert_matches!(
            UntrustedToken::try_from(mangled_str.as_str()).unwrap_err(),
            ParseError::Base64(_)
        );
    }

    #[test]
    fn malformed_header() {
        let mangled_headers = [
            // Missing closing brace
            r#"{"alg":"HS256""#,
            // Missing necessary `alg` field
            "{}",
            // `alg` field is not a string
            r#"{"alg":5}"#,
            r#"{"alg":[1,"foo"]}"#,
            r#"{"alg":false}"#,
            // Duplicate `alg` field
            r#"{"alg":"HS256","alg":"none"}"#,
        ];

        for mangled_header in &mangled_headers {
            let mangled_header = base64::encode_config(mangled_header, base64::URL_SAFE_NO_PAD);
            let mut mangled_str = HS256_TOKEN.to_owned();
            mangled_str.replace_range(..mangled_str.find('.').unwrap(), &mangled_header);
            assert_matches!(
                UntrustedToken::try_from(mangled_str.as_str()).unwrap_err(),
                ParseError::MalformedHeader(_)
            );
        }
    }

    #[test]
    fn unsupported_content_type() {
        let mangled_header = r#"{"alg":"HS256","cty":"txt"}"#;
        let mangled_header = base64::encode_config(mangled_header, base64::URL_SAFE_NO_PAD);
        let mut mangled_str = HS256_TOKEN.to_owned();
        mangled_str.replace_range(..mangled_str.find('.').unwrap(), &mangled_header);
        assert_matches!(
            UntrustedToken::try_from(mangled_str.as_str()).unwrap_err(),
            ParseError::UnsupportedContentType(ref s) if s == "txt"
        );
    }

    #[test]
    fn malformed_json_claims() {
        let malformed_claims = [
            // Missing closing brace
            r#"{"exp":1500000000"#,
            // `exp` claim is not a number
            r#"{"exp":"1500000000"}"#,
            r#"{"exp":false}"#,
            // Duplicate `exp` claim
            r#"{"exp":1500000000,"nbf":1400000000,"exp":1510000000}"#,
            // Too large `exp` value
            r#"{"exp":1500000000000000000000000000000000}"#,
        ];

        let claims_start = HS256_TOKEN.find('.').unwrap() + 1;
        let claims_end = HS256_TOKEN.rfind('.').unwrap();
        let key = base64::decode_config(HS256_KEY, base64::URL_SAFE_NO_PAD).unwrap();
        let key = Hs256Key::from(&*key);

        for claims in &malformed_claims {
            let encoded_claims = base64::encode_config(claims.as_bytes(), base64::URL_SAFE_NO_PAD);
            let mut mangled_str = HS256_TOKEN.to_owned();
            mangled_str.replace_range(claims_start..claims_end, &encoded_claims);
            let token = UntrustedToken::try_from(mangled_str.as_str()).unwrap();
            assert_matches!(
                Hs256.validate_integrity::<Obj>(&token, &key).unwrap_err(),
                ValidationError::MalformedClaims(_),
                "Failing claims: {}",
                claims
            );
        }
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct CustomClaims {
        /// We use a public claim (https://tools.ietf.org/html/rfc7519#section-4.1.2)
        /// with a custom (de)serializer. This allows to store the `subject` efficiently
        /// in the CBOR encoding.
        #[serde(rename = "sub", with = "HexForm")]
        subject: [u8; 32],
    }

    fn create_claims() -> Claims<CustomClaims> {
        let now = Utc::now();
        let now = now - Duration::nanoseconds(i64::from(now.timestamp_subsec_nanos()));

        Claims {
            issued_at: Some(now),
            expiration_date: Some(now + Duration::days(7)),
            not_before: None,
            custom: CustomClaims { subject: [1; 32] },
        }
    }

    fn test_algorithm<A: Algorithm>(
        algorithm: &A,
        signing_key: &A::SigningKey,
        verifying_key: &A::VerifyingKey,
    ) {
        let claims = create_claims();

        // Successful case with a compact token.
        let token_string = algorithm
            .compact_token(Header::default(), &claims, signing_key)
            .unwrap();
        let token = UntrustedToken::try_from(token_string.as_str()).unwrap();
        let token = algorithm.validate_integrity(&token, verifying_key).unwrap();
        assert_eq!(*token.claims(), claims);

        // Successful case.
        let token_string = algorithm
            .token(Header::default(), &claims, signing_key)
            .unwrap();
        let token = UntrustedToken::try_from(token_string.as_str()).unwrap();
        let token = algorithm.validate_integrity(&token, verifying_key).unwrap();
        assert_eq!(*token.claims(), claims);

        // Mutate each bit of the signature.
        let signature = token_string.rsplit('.').next().unwrap();
        let signature_start = token_string.rfind('.').unwrap() + 1;
        let signature = base64::decode_config(signature, base64::URL_SAFE_NO_PAD).unwrap();
        for i in 0..(signature.len() * 8) {
            let mut mangled_signature = signature.clone();
            mangled_signature[i / 8] ^= 1 << (i % 8) as u8;
            let mangled_signature =
                base64::encode_config(&mangled_signature, base64::URL_SAFE_NO_PAD);

            let mut mangled_str = token_string.clone();
            mangled_str.replace_range(signature_start.., &mangled_signature);
            let token = UntrustedToken::try_from(mangled_str.as_str()).unwrap();
            match algorithm
                .validate_integrity::<Obj>(&token, verifying_key)
                .unwrap_err()
            {
                ValidationError::InvalidSignature | ValidationError::MalformedSignature(_) => {}
                e => panic!("{:?}", e),
            }
        }

        // Mutate header.
        // Mutate claims.
    }

    #[test]
    fn hs256_algorithm() {
        let key = Hs256Key::generate(&mut thread_rng());
        test_algorithm(&Hs256, &key, &key);
    }

    #[test]
    fn hs384_algorithm() {
        let key = Hs384Key::generate(&mut thread_rng());
        test_algorithm(&Hs384, &key, &key);
    }

    #[test]
    fn hs512_algorithm() {
        let key = Hs512Key::generate(&mut thread_rng());
        test_algorithm(&Hs512, &key, &key);
    }

    #[test]
    fn compact_token_hs256() {
        let claims = create_claims();
        let key = Hs256Key::generate(&mut thread_rng());
        let long_token_str = Hs256.token(Header::default(), &claims, &key).unwrap();
        let token_str = Hs256
            .compact_token(Header::default(), &claims, &key)
            .unwrap();
        assert!(
            token_str.len() < long_token_str.len() - 40,
            "Full token length = {}, compact token length = {}",
            long_token_str.len(),
            token_str.len(),
        );
        let untrusted_token = UntrustedToken::try_from(&*token_str).unwrap();
        let token = Hs256.validate_integrity(&untrusted_token, &key).unwrap();
        assert_eq!(*token.claims(), claims);

        // Check that we can collect unknown / hard to parse claims into `Claims.custom`.
        let generic_token: Token<HashMap<String, serde_cbor::Value>> =
            Hs256.validate_integrity(&untrusted_token, &key).unwrap();
        assert_matches!(
            generic_token.claims().custom["sub"],
            serde_cbor::Value::Bytes(_)
        );
    }

    #[cfg(feature = "exonum-crypto")]
    #[test]
    fn ed25519_algorithm() {
        use exonum_crypto::gen_keypair;
        let (verifying_key, signing_key) = gen_keypair();
        test_algorithm(&Ed25519, &signing_key, &verifying_key);
    }

    #[cfg(feature = "ed25519-dalek")]
    #[test]
    fn ed25519_algorithm() {
        use ed25519_dalek::Keypair;
        let keypair = Keypair::generate(&mut thread_rng());
        test_algorithm(&Ed25519, &keypair, &keypair.public);
    }

    #[cfg(feature = "secp256k1")]
    #[test]
    fn es256k_algorithm() {
        use rand::Rng;
        use secp256k1::{PublicKey, Secp256k1, SecretKey};

        let mut rng = thread_rng();
        let signing_key = loop {
            let bytes: [u8; 32] = rng.gen();
            if let Ok(key) = SecretKey::from_slice(&bytes) {
                break key;
            }
        };
        let context = Secp256k1::new();
        let verifying_key = PublicKey::from_secret_key(&context, &signing_key);
        let es256k: Es256k<sha2::Sha256> = Es256k::new(context);
        test_algorithm(&es256k, &signing_key, &verifying_key);
    }
}
