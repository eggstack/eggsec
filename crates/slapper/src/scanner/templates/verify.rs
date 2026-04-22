//! Template signing and verification using Ed25519.
//!
//! This module provides cryptographic signing and verification for community
//! templates to ensure authenticity and integrity.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::{Result, SlapperError};
use crate::scanner::templates::models::VulnerabilityTemplate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTemplate {
    pub template: VulnerabilityTemplate,
    pub signature: String,
    pub public_key: String,
    pub signer_info: SignerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerInfo {
    pub name: String,
    pub email: Option<String>,
    pub organization: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct TemplateSigner {
    signing_key: SigningKey,
    public_key: VerifyingKey,
    signer_info: SignerInfo,
}

impl TemplateSigner {
    pub fn new(signer_info: SignerInfo) -> Result<Self> {
        let signing_key = SigningKey::generate(&mut OsRng);
        let public_key = signing_key.verifying_key();

        Ok(Self {
            signing_key,
            public_key,
            signer_info,
        })
    }

    pub fn from_keypair(
        signing_key: &[u8],
        signer_info: SignerInfo,
    ) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(
            signing_key.try_into().map_err(|_| {
                SlapperError::Validation("Invalid signing key length".to_string())
            })?
        );
        let public_key = signing_key.verifying_key();

        Ok(Self {
            signing_key,
            public_key,
            signer_info,
        })
    }

    pub fn sign(&self, template: &VulnerabilityTemplate) -> Result<SignedTemplate> {
        let template_bytes = serde_yaml_neo::to_string(template)
            .map_err(|e| SlapperError::Parse(e.to_string()))?;

        let signature = self.signing_key.sign(template_bytes.as_bytes());

        Ok(SignedTemplate {
            template: template.clone(),
            signature: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, signature.to_bytes()),
            public_key: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self.public_key.to_bytes()),
            signer_info: self.signer_info.clone(),
        })
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.public_key.to_bytes()
    }

    pub fn public_key_string(&self) -> String {
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self.public_key.to_bytes())
    }

    pub fn save_private_key(&self, path: &Path) -> Result<()> {
        let key_bytes = self.signing_key.to_bytes();
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key_bytes);
        fs::write(path, encoded)?;
        Ok(())
    }

    pub fn save_public_key(&self, path: &Path) -> Result<()> {
        let encoded = self.public_key_string();
        fs::write(path, encoded)?;
        Ok(())
    }
}

pub struct TemplateVerifier {
    verifying_key: Option<VerifyingKey>,
}

impl TemplateVerifier {
    pub fn new() -> Self {
        Self { verifying_key: None }
    }

    pub fn with_public_key(public_key: &str) -> Result<Self> {
        let key_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, public_key)
            .map_err(|e| SlapperError::Parse(e.to_string()))?;

        let key_array: [u8; 32] = key_bytes.try_into().map_err(|_| {
            SlapperError::Validation("Invalid public key length".to_string())
        })?;

        let verifying_key = VerifyingKey::from_bytes(&key_array)
            .map_err(|e| SlapperError::Validation(format!("Invalid public key: {}", e)))?;

        Ok(Self {
            verifying_key: Some(verifying_key),
        })
    }

    pub fn with_public_key_bytes(key_bytes: &[u8]) -> Result<Self> {
        let key_array: [u8; 32] = key_bytes.try_into().map_err(|_| {
            SlapperError::Validation("Invalid public key length".to_string())
        })?;

        let verifying_key = VerifyingKey::from_bytes(&key_array)
            .map_err(|e| SlapperError::Validation(format!("Invalid public key: {}", e)))?;

        Ok(Self {
            verifying_key: Some(verifying_key),
        })
    }

    pub fn verify(&self, signed_template: &SignedTemplate) -> Result<VerificationResult> {
        let verifying_key = self.verifying_key.as_ref().ok_or_else(|| {
            SlapperError::InvalidState("No public key configured for verification".to_string())
        })?;

        let template_bytes = serde_yaml_neo::to_string(&signed_template.template)
            .map_err(|e| SlapperError::Parse(e.to_string()))?;

        let signature_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &signed_template.signature)
            .map_err(|e| SlapperError::Parse(e.to_string()))?;

        let signature = Signature::from_bytes(&signature_bytes.try_into().map_err(|_| {
            SlapperError::Validation("Invalid signature length".to_string())
        })?);

        match verifying_key.verify(template_bytes.as_bytes(), &signature) {
            Ok(()) => Ok(VerificationResult {
                valid: true,
                template_id: signed_template.template.id.clone(),
                signer_info: signed_template.signer_info.clone(),
                error: None,
            }),
            Err(e) => Ok(VerificationResult {
                valid: false,
                template_id: signed_template.template.id.clone(),
                signer_info: signed_template.signer_info.clone(),
                error: Some(format!("Signature verification failed: {}", e)),
            }),
        }
    }

    pub fn verify_raw(&self, template: &VulnerabilityTemplate, signature: &str) -> Result<bool> {
        let verifying_key = self.verifying_key.as_ref().ok_or_else(|| {
            SlapperError::InvalidState("No public key configured for verification".to_string())
        })?;

        let template_bytes = serde_yaml_neo::to_string(template)
            .map_err(|e| SlapperError::Parse(e.to_string()))?;

        let signature_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, signature)
            .map_err(|e| SlapperError::Parse(e.to_string()))?;

        let signature = Signature::from_bytes(&signature_bytes.try_into().map_err(|_| {
            SlapperError::Validation("Invalid signature length".to_string())
        })?;

        Ok(verifying_key.verify(template_bytes.as_bytes(), &signature).is_ok())
    }
}

impl Default for TemplateVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub valid: bool,
    pub template_id: String,
    pub signer_info: SignerInfo,
    pub error: Option<String>,
}

impl SignedTemplate {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        serde_yaml_neo::from_str(&content).map_err(|e| SlapperError::Parse(e.to_string()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yaml_neo::to_string(self)
            .map_err(|e| SlapperError::Serialize(e.to_string()))?;
        fs::write(path, content)?;
        Ok(())
    }
}

pub fn load_public_key(path: &Path) -> Result<String> {
    let content = fs::read_to_string(path)?;
    Ok(content.trim().to_string())
}

pub fn load_private_key(path: &Path) -> Result<[u8; 32]> {
    let content = fs::read_to_string(path)?;
    let key_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, content.trim())
        .map_err(|e| SlapperError::Parse(e.to_string()))?;
    key_bytes.try_into().map_err(|_| {
        SlapperError::Validation("Invalid private key length".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_template() -> VulnerabilityTemplate {
        VulnerabilityTemplate {
            id: "test-template".to_string(),
            info: crate::scanner::templates::models::TemplateInfo {
                name: "Test Template".to_string(),
                author: "test".to_string(),
                severity: "high".to_string(),
                description: "A test template".to_string(),
                tags: vec!["test".to_string()],
                references: vec![],
                remediation: "Fix the issue".to_string(),
            },
            matchers: vec![],
            requests: vec![],
        }
    }

    fn create_test_signer_info() -> SignerInfo {
        SignerInfo {
            name: "Test Signer".to_string(),
            email: Some("test@example.com".to_string()),
            organization: Some("Test Org".to_string()),
            timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_signer_creation() {
        let signer_info = create_test_signer_info();
        let signer = TemplateSigner::new(signer_info).unwrap();
        assert!(!signer.public_key_string().is_empty());
    }

    #[test]
    fn test_sign_and_verify() {
        let signer_info = create_test_signer_info();
        let signer = TemplateSigner::new(signer_info).unwrap();
        let template = create_test_template();

        let signed = signer.sign(&template).unwrap();
        assert!(!signed.signature.is_empty());
        assert!(!signed.public_key.is_empty());

        let verifier = TemplateVerifier::with_public_key(&signed.public_key).unwrap();
        let result = verifier.verify(&signed).unwrap();
        assert!(result.valid);
        assert_eq!(result.template_id, "test-template");
    }

    #[test]
    fn test_signature_verification_fails_on_tampered_template() {
        let signer_info = create_test_signer_info();
        let signer = TemplateSigner::new(signer_info).unwrap();
        let template = create_test_template();

        let mut signed = signer.sign(&template).unwrap();
        signed.template.info.severity = "low".to_string();

        let verifier = TemplateVerifier::with_public_key(&signed.public_key).unwrap();
        let result = verifier.verify(&signed).unwrap();
        assert!(!result.valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_verify_without_public_key() {
        let signer_info = create_test_signer_info();
        let signer = TemplateSigner::new(signer_info).unwrap();
        let template = create_test_template();

        let signed = signer.sign(&template).unwrap();

        let verifier = TemplateVerifier::new();
        let result = verifier.verify(&signed);
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_and_verify_raw() {
        let signer_info = create_test_signer_info();
        let signer = TemplateSigner::new(signer_info).unwrap();
        let template = create_test_template();

        let signed = signer.sign(&template).unwrap();

        let verifier = TemplateVerifier::with_public_key(&signed.public_key).unwrap();
        let is_valid = verifier.verify_raw(&signed.template, &signed.signature).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_public_key_string_format() {
        let signer_info = create_test_signer_info();
        let signer = TemplateSigner::new(signer_info).unwrap();
        let public_key_str = signer.public_key_string();

        let verifier = TemplateVerifier::with_public_key(&public_key_str).unwrap();
        assert!(verifier.verify_raw(&create_test_template(), "").is_err());
    }

    #[test]
    fn test_signed_template_serialization() {
        let signer_info = create_test_signer_info();
        let signer = TemplateSigner::new(signer_info).unwrap();
        let template = create_test_template();
        let signed = signer.sign(&template).unwrap();

        let json = serde_json::to_string(&signed).unwrap();
        let deserialized: SignedTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.template.id, signed.template.id);
        assert_eq!(deserialized.signature, signed.signature);
    }
}
