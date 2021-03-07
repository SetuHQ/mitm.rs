use std::collections::HashMap;
use std::env;
use std::sync::Mutex;

use lazy_static::lazy_static;
use openssl::asn1::Asn1Time;
use openssl::bn::{BigNum, MsbOption};
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, PKeyRef, Private};
use openssl::rsa::Rsa;
use openssl::x509::extension::{
  AuthorityKeyIdentifier, BasicConstraints, KeyUsage, SubjectAlternativeName, SubjectKeyIdentifier
};
use openssl::x509::{X509NameBuilder, X509Ref, X509Req, X509ReqBuilder, X509VerifyResult, X509};

#[derive(Clone)]
pub struct CertPair {
  pub key:  PKey<Private>,
  pub cert: X509,
}

// The global client certificates cache
lazy_static! {
  pub static ref CERTIFICATES: Mutex<HashMap<String, CertPair>> = Mutex::new(HashMap::new());
}

/// Make a CA certificate and private key
pub fn mk_ca_cert() -> Result<(X509, PKey<Private>), ErrorStack> {
  let rsa = Rsa::generate(2048)?;
  let privkey = PKey::from_rsa(rsa)?;

  let mut x509_name = X509NameBuilder::new()?;
  x509_name.append_entry_by_text("C", &env::var("CA_C").unwrap_or("IN".to_string()))?;
  x509_name.append_entry_by_text("ST", &env::var("CA_ST").unwrap_or("KA".to_string()))?;
  x509_name.append_entry_by_text("O", &env::var("CA_O").unwrap_or("mitm.rs".to_string()))?;
  x509_name.append_entry_by_text("CN", &env::var("CA_CN").unwrap_or("mitm.rs".to_string()))?;
  let x509_name = x509_name.build();

  let mut cert_builder = X509::builder()?;
  cert_builder.set_version(2)?;
  let serial_number = {
    let mut serial = BigNum::new()?;
    serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
    serial.to_asn1_integer()?
  };
  cert_builder.set_serial_number(&serial_number)?;
  cert_builder.set_subject_name(&x509_name)?;
  cert_builder.set_issuer_name(&x509_name)?;
  cert_builder.set_pubkey(&privkey)?;
  let not_before = Asn1Time::days_from_now(0)?;
  cert_builder.set_not_before(&not_before)?;
  let not_after = Asn1Time::days_from_now(env::var("CA_EXPIRY").unwrap_or("1095".to_string()).parse::<u32>().unwrap())?;
  cert_builder.set_not_after(&not_after)?;

  cert_builder.append_extension(BasicConstraints::new().critical().ca().build()?)?;
  cert_builder.append_extension(KeyUsage::new().critical().key_cert_sign().crl_sign().build()?)?;

  let subject_key_identifier = SubjectKeyIdentifier::new().build(&cert_builder.x509v3_context(None, None))?;
  cert_builder.append_extension(subject_key_identifier)?;

  cert_builder.sign(&privkey, MessageDigest::sha256())?;
  let cert = cert_builder.build();

  Ok((cert, privkey))
}

/// Make a X509 request with the given private key
fn mk_request(privkey: &PKey<Private>) -> Result<X509Req, ErrorStack> {
  let mut req_builder = X509ReqBuilder::new()?;
  req_builder.set_pubkey(&privkey)?;

  let mut x509_name = X509NameBuilder::new()?;
  x509_name.append_entry_by_text("C", &env::var("CA_C").unwrap_or("IN".to_string()))?;
  x509_name.append_entry_by_text("ST", &env::var("CA_ST").unwrap_or("KA".to_string()))?;
  x509_name.append_entry_by_text("O", &env::var("CA_O").unwrap_or("mitm.rs".to_string()))?;
  x509_name.append_entry_by_text("CN", &env::var("CA_CN").unwrap_or("mitm.rs".to_string()))?;
  let x509_name = x509_name.build();
  req_builder.set_subject_name(&x509_name)?;

  req_builder.sign(&privkey, MessageDigest::sha256())?;
  let req = req_builder.build();
  Ok(req)
}

/// Make a certificate and private key signed by the given CA cert and private key
pub fn mk_ca_signed_cert(
  ca_cert: &X509Ref,
  ca_privkey: &PKeyRef<Private>,
) -> Result<(X509, PKey<Private>), ErrorStack> {
  let rsa = Rsa::generate(2048)?;
  let privkey = PKey::from_rsa(rsa)?;

  let req = mk_request(&privkey)?;

  let mut cert_builder = X509::builder()?;
  cert_builder.set_version(2)?;
  let serial_number = {
    let mut serial = BigNum::new()?;
    serial.rand(159, MsbOption::MAYBE_ZERO, false)?;
    serial.to_asn1_integer()?
  };
  cert_builder.set_serial_number(&serial_number)?;
  cert_builder.set_subject_name(req.subject_name())?;
  cert_builder.set_issuer_name(ca_cert.subject_name())?;
  cert_builder.set_pubkey(&privkey)?;
  let not_before = Asn1Time::days_from_now(0)?;
  cert_builder.set_not_before(&not_before)?;
  let not_after = Asn1Time::days_from_now(365)?;
  cert_builder.set_not_after(&not_after)?;

  cert_builder.append_extension(BasicConstraints::new().build()?)?;

  cert_builder
    .append_extension(KeyUsage::new().critical().non_repudiation().digital_signature().key_encipherment().build()?)?;

  let subject_key_identifier = SubjectKeyIdentifier::new().build(&cert_builder.x509v3_context(Some(ca_cert), None))?;
  cert_builder.append_extension(subject_key_identifier)?;

  let auth_key_identifier = AuthorityKeyIdentifier::new()
    .keyid(false)
    .issuer(false)
    .build(&cert_builder.x509v3_context(Some(ca_cert), None))?;
  cert_builder.append_extension(auth_key_identifier)?;

  let subject_alt_name = SubjectAlternativeName::new()
    .dns("*.example.com")
    .dns("hello.com")
    .build(&cert_builder.x509v3_context(Some(ca_cert), None))?;
  cert_builder.append_extension(subject_alt_name)?;

  cert_builder.sign(&ca_privkey, MessageDigest::sha256())?;
  let cert = cert_builder.build();

  Ok((cert, privkey))
}

// Verify that this cert was issued by this ca
pub fn verify(ca_cert: X509, cert: X509) -> Result<(), ErrorStack> {
  match ca_cert.issued(&cert) {
    X509VerifyResult::OK => println!("Certificate verified!"),
    ver_err => println!("Failed to verify certificate: {}", ver_err),
  };

  Ok(())
}

// parse certificate string as X509 cert
pub fn read_cert(cert: String) -> Result<X509, ErrorStack> { X509::from_pem(cert.as_bytes()) }

pub fn read_pkey(pkey: String) -> Result<PKey<Private>, ErrorStack> { PKey::private_key_from_pem(pkey.as_bytes()) }
