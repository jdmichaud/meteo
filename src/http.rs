use reqwest::blocking::Client;

/// Build the shared HTTP client used for geocoding and weather fetches.
///
/// When `ignore_ssl_cert` is set, TLS certificate validation is disabled
/// (`danger_accept_invalid_certs`). This is insecure and only intended for
/// environments behind a TLS-intercepting proxy.
pub fn build_client(ignore_ssl_cert: bool) -> Result<Client, reqwest::Error> {
    Client::builder()
        .danger_accept_invalid_certs(ignore_ssl_cert)
        .build()
}
