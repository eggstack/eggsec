use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::utils::create_http_client;

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
        let email_regex = Regex::new(
            r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"
        ).unwrap();

        let mut emails = HashSet::new();
        for cap in email_regex.find_iter(content) {
            let email = cap.as_str().to_lowercase();
            if !email.contains("example.com")
                && !email.contains("test.com")
                && !email.contains("localhost")
            {
                emails.insert(EmailContact {
                    email: email.clone(),
                    context: content.lines()
                        .find(|l| l.contains(&email))
                        .map(|l| l.trim().chars().take(100).collect())
                        .unwrap_or_default(),
                    source: "website".to_string(),
                });
            }
        }

        emails.into_iter().collect()
    }

    fn extract_phones(&self, content: &str) -> Vec<PhoneNumber> {
        let phone_patterns = [
            r"\+?1?[-.\s]?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}",
            r"\+?[0-9]{1,4}[-.\s]?[0-9]{2,4}[-.\s]?[0-9]{2,4}[-.\s]?[0-9]{2,4}",
            r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b",
        ];

        let mut phones = HashSet::new();

        for pattern in phone_patterns {
            if let Ok(re) = Regex::new(pattern) {
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
        }

        phones.into_iter().collect()
    }

    fn extract_social_media(&self, content: &str) -> Vec<SocialMedia> {
        let patterns = [
            (r"facebook\.com/([a-zA-Z0-9._-]+)", "Facebook"),
            (r"twitter\.com/([a-zA-Z0-9._-]+)", "Twitter"),
            (r"x\.com/([a-zA-Z0-9._-]+)", "X"),
            (r"instagram\.com/([a-zA-Z0-9._-]+)", "Instagram"),
            (r"linkedin\.com/in/([a-zA-Z0-9._-]+)", "LinkedIn"),
            (r"linkedin\.com/company/([a-zA-Z0-9._-]+)", "LinkedIn"),
            (r"github\.com/([a-zA-Z0-9._-]+)", "GitHub"),
            (r"youtube\.com/@([a-zA-Z0-9._-]+)", "YouTube"),
            (r"tiktok\.com/@([a-zA-Z0-9._-]+)", "TikTok"),
        ];

        let mut socials = HashSet::new();

        for (pattern, platform) in patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(content) {
                    if let Some(handle) = cap.get(1) {
                        let url = format!("https://{}.com/{}", platform.to_lowercase(), handle.as_str());
                        socials.insert(SocialMedia {
                            platform: platform.to_string(),
                            url,
                            handle: Some(handle.as_str().to_string()),
                        });
                    }
                }
            }
        }

        socials.into_iter().collect()
    }

    fn extract_addresses(&self, content: &str) -> Vec<PhysicalAddress> {
        let address_patterns = [
            r"\d+\s+[A-Za-z\s]+(?:Street|St|Avenue|Ave|Road|Rd|Boulevard|Blvd|Drive|Dr|Lane|Ln|Court|Ct|Way|Place|Pl)\.?\s*,?\s*(?:[A-Za-z\s]+,)?\s*[A-Z]{2}\s*\d{5}(?:-\d{4})?",
            r"[A-Z][a-z]+,\s*[A-Z]{2}\s*\d{5}",
        ];

        let mut addresses = Vec::new();

        for pattern in address_patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.find_iter(content) {
                    addresses.push(PhysicalAddress {
                        address: cap.as_str().to_string(),
                        context: String::new(),
                        source: "website".to_string(),
                    });
                }
            }
        }

        addresses
    }
}

pub async fn discover_contacts(url: &str) -> Result<EmailDiscovery> {
    let client = EmailDiscoveryClient::new()?;
    client.discover(url).await
}
