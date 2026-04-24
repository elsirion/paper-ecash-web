use fedimint_core::config::ClientConfig;
use fedimint_core::invite_code::InviteCode;
use fedimint_core::PeerId;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "client.json".to_string());

    let json = std::fs::read_to_string(&path)?;
    let config: ClientConfig = serde_json::from_str(&json)?;

    let federation_id = config.calculate_federation_id();
    let peer = PeerId::from(0);
    let url = config
        .global
        .api_endpoints
        .get(&peer)
        .ok_or_else(|| anyhow::anyhow!("No API endpoint for peer 0"))?
        .url
        .clone();

    let invite = InviteCode::new(url, peer, federation_id, None);
    println!("{invite}");
    Ok(())
}
