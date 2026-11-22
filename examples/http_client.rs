use kui::HTTPClient;

fn main() -> kui::Result<()> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://example.com".into());
    let client = HTTPClient::new()?;
    let response = client.fetch(&url, 10.0)?;
    println!(
        "HTTP {}, {} bytes, {}",
        response.status,
        response.body.len(),
        response.content_type
    );
    Ok(())
}
