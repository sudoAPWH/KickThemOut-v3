use std::sync::Arc;

use serde::Deserialize;
use tokio::sync::Semaphore;

use crate::scanner::Host;

const MAC_API_URL: &str = "https://api.maclookup.app/v2/macs/{mac}";

#[derive(Debug, Deserialize)]
struct VendorResponse {
    #[serde(rename = "isPrivate")]
    is_private: bool,
    company: Option<String>,
}

pub struct VendorResolver {
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
}

impl VendorResolver {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .unwrap(),
            semaphore: Arc::new(Semaphore::new(10)), // Limit concurrent requests
        }
    }

    /// Resolve vendors for all hosts in parallel
    pub async fn resolve_batch(&self, hosts: &mut [Host]) {
        let indices: Vec<usize> = (0..hosts.len()).collect();
        let results: Vec<Option<String>> =
            futures::future::join_all(indices.into_iter().map(|i| {
                let permit = self.semaphore.clone();
                let client = self.client.clone();
                let mac = hosts[i].mac.clone();

                async move {
                    let _permit = permit.acquire().await.ok()?;
                    let vendor = resolve_single(&client, &mac).await;
                    drop(_permit);
                    vendor
                }
            }))
            .await;

        // Update hosts with resolved vendors
        for (i, vendor) in results.into_iter().enumerate() {
            if let Some(v) = vendor {
                hosts[i].vendor = v;
            }
        }
    }
}

async fn resolve_single(client: &reqwest::Client, mac: &str) -> Option<String> {
    // Convert MAC from "xx:xx:xx:xx:xx:xx" to "xx-xx-xx-xx-xx-xx"
    let formatted_mac = mac.replace(':', "-");

    let url = MAC_API_URL.replace("{mac}", &formatted_mac);

    match client.get(&url).send().await {
        Ok(response) => {
            if response.status().is_success() {
                if let Ok(data) = response.json::<VendorResponse>().await {
                    if data.is_private {
                        return Some("Private".to_string());
                    }
                    return data.company.or(Some("Unknown".to_string()));
                }
            }
            None
        }
        Err(_) => None,
    }
}

impl Default for VendorResolver {
    fn default() -> Self {
        Self::new()
    }
}