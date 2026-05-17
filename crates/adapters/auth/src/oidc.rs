// Stub: extend this when auth_oidc = true.
pub struct OidcAdapter;

impl OidcAdapter {
    pub fn new() -> Self { Self }
}

impl Default for OidcAdapter {
    fn default() -> Self { Self::new() }
}
