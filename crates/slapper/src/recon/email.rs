use crate::error::Result;
use regex::Regex;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::utils::create_http_client;

static EMAIL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").expect("valid email pattern")
});

static PHONE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"\+?1?[-.\s]?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}")
            .expect("valid US phone pattern"),
        Regex::new(r"\+?[0-9]{1,4}[-.\s]?[0-9]{2,4}[-.\s]?[0-9]{2,4}[-.\s]?[0-9]{2,4}")
            .expect("valid international phone pattern"),
        Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").expect("valid compact phone pattern"),
    ]
});

static SOCIAL_PATTERNS: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    vec![
        (
            "Facebook",
            Regex::new(r"facebook\.com/([a-zA-Z0-9._-]+)").expect("valid Facebook pattern"),
        ),
        (
            "Twitter",
            Regex::new(r"twitter\.com/([a-zA-Z0-9._-]+)").expect("valid Twitter pattern"),
        ),
        (
            "X",
            Regex::new(r"x\.com/([a-zA-Z0-9._-]+)").expect("valid X pattern"),
        ),
        (
            "Instagram",
            Regex::new(r"instagram\.com/([a-zA-Z0-9._-]+)").expect("valid Instagram pattern"),
        ),
        (
            "LinkedIn",
            Regex::new(r"linkedin\.com/in/([a-zA-Z0-9._-]+)")
                .expect("valid LinkedIn profile pattern"),
        ),
        (
            "LinkedIn",
            Regex::new(r"linkedin\.com/company/([a-zA-Z0-9._-]+)")
                .expect("valid LinkedIn company pattern"),
        ),
        (
            "GitHub",
            Regex::new(r"github\.com/([a-zA-Z0-9._-]+)").expect("valid GitHub pattern"),
        ),
        (
            "YouTube",
            Regex::new(r"youtube\.com/@([a-zA-Z0-9._-]+)").expect("valid YouTube pattern"),
        ),
        (
            "TikTok",
            Regex::new(r"tiktok\.com/@([a-zA-Z0-9._-]+)").expect("valid TikTok pattern"),
        ),
    ]
});

static ADDRESS_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"\d+\s+[A-Za-z\s]+(?:Street|St|Avenue|Ave|Road|Rd|Boulevard|Blvd|Drive|Dr|Lane|Ln|Court|Ct|Way|Place|Pl)\.?\s*,?\s*(?:[A-Za-z\s]+,)?\s*[A-Z]{2}\s*\d{5}(?:-\d{4})?").expect("valid street address pattern"),
        Regex::new(r"[A-Z][a-z]+,\s*[A-Z]{2}\s*\d{5}").expect("valid city-state-zip pattern"),
    ]
});

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailDiscovery {
    pub url: String,
    pub emails: Vec<EmailContact>,
    pub phone_numbers: Vec<PhoneNumber>,
    pub social_media: Vec<SocialMedia>,
    pub addresses: Vec<PhysicalAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct EmailContact {
    pub email: String,
    pub context: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct PhoneNumber {
    pub number: String,
    pub context: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct SocialMedia {
    pub platform: String,
    pub url: String,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalAddress {
    pub address: String,
    pub context: String,
    pub source: String,
}

pub struct EmailDiscoveryClient {
    client: reqwest::Client,
}

impl EmailDiscoveryClient {
    pub fn new() -> Result<Self> {
        let client = create_http_client(30)?;

        Ok(Self { client })
    }

    pub async fn discover(&self, url: &str) -> Result<EmailDiscovery> {
        let response = self.client.get(url).send().await?;
        let html = response.text().await?;

        let emails = self.extract_emails(&html);
        let phone_numbers = self.extract_phones(&html);
        let social_media = self.extract_social_media(&html);
        let addresses = self.extract_addresses(&html);

        Ok(EmailDiscovery {
            url: url.to_string(),
            emails,
            phone_numbers,
            social_media,
            addresses,
        })
    }

    fn extract_emails(&self, content: &str) -> Vec<EmailContact> {
        let mut emails = FxHashSet::default();
        for cap in EMAIL_PATTERN.find_iter(content) {
            let email = cap.as_str().to_lowercase();
            if !email.contains("example.com")
                && !email.contains("test.com")
                && !email.contains("localhost")
            {
                emails.insert(EmailContact {
                    email: email.clone(),
                    context: content
                        .lines()
                        .find(|l| l.contains(&email))
                        .map(|l| l.trim().chars().take(100).collect())
                        .unwrap_or_else(|| {
                            tracing::debug!("email context line not found for {}", email);
                            String::new()
                        }),
                    source: "website".to_string(),
                });
            }
        }

        emails.into_iter().collect()
    }

    fn extract_phones(&self, content: &str) -> Vec<PhoneNumber> {
        let mut phones = FxHashSet::default();

        for re in PHONE_PATTERNS.iter() {
            for cap in re.find_iter(content) {
                let number = cap.as_str().to_string();
                if number.len() >= 10 {
                    phones.insert(PhoneNumber {
                        number,
                        context: String::new(),
                        source: "website".to_string(),
                    });
                }
            }
        }

        phones.into_iter().collect()
    }

    fn extract_social_media(&self, content: &str) -> Vec<SocialMedia> {
        let mut socials = FxHashSet::default();

        for (platform, re) in SOCIAL_PATTERNS.iter() {
            for cap in re.captures_iter(content) {
                if let Some(handle) = cap.get(1) {
                    let url = format!(
                        "https://{}.com/{}",
                        platform.to_lowercase(),
                        handle.as_str()
                    );
                    socials.insert(SocialMedia {
                        platform: platform.to_string(),
                        url,
                        handle: Some(handle.as_str().to_string()),
                    });
                }
            }
        }

        socials.into_iter().collect()
    }

    fn extract_addresses(&self, content: &str) -> Vec<PhysicalAddress> {
        let mut addresses = Vec::new();

        for re in ADDRESS_PATTERNS.iter() {
            for cap in re.find_iter(content) {
                addresses.push(PhysicalAddress {
                    address: cap.as_str().to_string(),
                    context: String::new(),
                    source: "website".to_string(),
                });
            }
        }

        addresses
    }
}

pub async fn discover_contacts(url: &str) -> Result<EmailDiscovery> {
    let client = EmailDiscoveryClient::new()?;
    client.discover(url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_emails_basic() {
        let client = EmailDiscoveryClient::new().unwrap();
        let content = "Contact us at admin@company.com or support@business.org";
        let emails = client.extract_emails(content);
        assert!(!emails.is_empty());
    }

    #[test]
    fn test_extract_emails_filters_examples() {
        let client = EmailDiscoveryClient::new().unwrap();
        let content = "Email: user@example.com and test@test.com";
        let emails = client.extract_emails(content);
        assert!(
            emails.is_empty(),
            "Should filter out example.com and test.com"
        );
    }

    #[test]
    fn test_extract_emails_deduplicates() {
        let client = EmailDiscoveryClient::new().unwrap();
        let content = "admin@company.com admin@company.com admin@company.com";
        let emails = client.extract_emails(content);
        assert_eq!(emails.len(), 1);
    }

    #[test]
    fn test_extract_phones_basic() {
        let client = EmailDiscoveryClient::new().unwrap();
        let content = "Call us at +1-555-123-4567 or (555) 987-6543";
        let phones = client.extract_phones(content);
        assert!(!phones.is_empty());
    }

    #[test]
    fn test_extract_social_media_links() {
        let client = EmailDiscoveryClient::new().unwrap();
        let content = r#"<a href="https://github.com/slapper">GitHub</a>"#;
        let socials = client.extract_social_media(content);
        assert!(socials.iter().any(|s| s.platform == "GitHub"));
    }

    #[test]
    fn test_extract_social_media_twitter() {
        let client = EmailDiscoveryClient::new().unwrap();
        let content = r#"<a href="https://twitter.com/slapper_sec">Twitter</a>"#;
        let socials = client.extract_social_media(content);
        assert!(socials
            .iter()
            .any(|s| s.platform == "Twitter" || s.platform == "X"));
    }
}
