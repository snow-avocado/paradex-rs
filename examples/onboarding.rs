//! Demonstrates how to create a Paradex REST client starting from an Ethereum private key
//! and optionally attach onboarding metadata (marketing/referral/UTM fields).

use clap::Parser;

#[cfg(feature = "onboarding")]
use paradex::{
    rest::Client,
    structs::{OnboardingRequest, OnboardingUtm},
    url::URL,
};

#[derive(Parser, Debug)]
#[command(version, about = "Submit the onboarding payload using an Ethereum key", long_about = None)]
struct Args {
    /// Use production instead of testnet endpoints
    #[arg(long, action)]
    production: bool,

    /// Hex-encoded Ethereum private key that controls the Paradex account
    #[arg(long)]
    ethereum_private_key: String,

    /// Optional marketing code to attach to the onboarding payload
    #[arg(long)]
    marketing_code: Option<String>,

    /// Optional referral code to attach to the onboarding payload
    #[arg(long)]
    referral_code: Option<String>,

    /// Optional UTM source value
    #[arg(long)]
    utm_source: Option<String>,

    /// Optional UTM medium value
    #[arg(long)]
    utm_medium: Option<String>,

    /// Optional UTM campaign value
    #[arg(long)]
    utm_campaign: Option<String>,
}

#[cfg(not(feature = "onboarding"))]
fn main() {
    eprintln!("Rebuild with --features onboarding to run this example");
}

#[cfg(feature = "onboarding")]
#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let args = Args::parse();
    let url = if args.production {
        URL::Production
    } else {
        URL::Testnet
    };
    let onboarding_request = build_onboarding_request(&args);
    let eth_private_key = args.ethereum_private_key.trim().to_string();
    let client = Client::new_with_eth_private_key(url, eth_private_key, Some(onboarding_request))
        .await
        .expect("failed to run onboarding flow");

    log::info!("JWT token acquired: {:?}", client.jwt().await.ok());
}

#[cfg(feature = "onboarding")]
fn build_onboarding_request(args: &Args) -> OnboardingRequest {
    let mut request = OnboardingRequest::default();

    if let Some(code) = args.marketing_code.clone() {
        request = request.with_marketing_code(code);
    }

    if let Some(code) = args.referral_code.clone() {
        request = request.with_referral_code(code);
    }

    let utm = OnboardingUtm {
        campaign: args.utm_campaign.clone(),
        medium: args.utm_medium.clone(),
        source: args.utm_source.clone(),
    };

    if utm != OnboardingUtm::default() {
        request = request.with_utm(utm);
    }

    request
}
