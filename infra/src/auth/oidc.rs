use anyhow::anyhow;
use openidconnect::{
    AccessTokenHash, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    EmptyAdditionalClaims, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    StandardErrorResponse, TokenResponse, UserInfoClaims,
    core::{
        CoreAuthDisplay, CoreAuthPrompt, CoreAuthenticationFlow, CoreClient, CoreErrorResponseType,
        CoreGenderClaim, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm, CoreProviderMetadata,
        CoreRevocableToken, CoreRevocationErrorResponse, CoreTokenIntrospectionResponse,
        CoreTokenResponse,
    },
    reqwest,
};

pub type OidcClient = Client<
    EmptyAdditionalClaims,
    CoreAuthDisplay,
    CoreGenderClaim,
    CoreJweContentEncryptionAlgorithm,
    CoreJsonWebKey,
    CoreAuthPrompt,
    StandardErrorResponse<CoreErrorResponseType>,
    CoreTokenResponse,
    CoreTokenIntrospectionResponse,
    CoreRevocableToken,
    CoreRevocationErrorResponse,
    EndpointSet,      // HasAuthUrl (Required and guaranteed by discovery)
    EndpointNotSet,   // HasDeviceAuthUrl
    EndpointNotSet,   // HasIntrospectionUrl
    EndpointNotSet,   // HasRevocationUrl
    EndpointMaybeSet, // HasTokenUrl (Discovered, might be missing)
    EndpointMaybeSet, // HasUserInfoUrl (Discovered, might be missing)
>;

#[derive(Clone)]
pub struct OidcService {
    client: OidcClient,
    resource_id: Option<String>,
}

#[derive(Debug)]
pub struct OidcUser {
    pub subject: String,
    pub email: String,
}

impl OidcService {
    //todo: replace Strings with newtypes
    pub async fn new(
        issuer: String,
        client_id: String,
        client_secret: String,
        redirect_url: String,
        resource_id: Option<String>,
    ) -> anyhow::Result<Self> {
        let client_id = client_id.trim().to_string();
        let redirect_url = redirect_url.trim().to_string();
        let issuer = issuer.trim().to_string();

        // 2. Handle Empty Secret (For PKCE/Public Clients)
        let client_secret_clean = client_secret.trim();
        let client_secret_opt = if client_secret_clean.is_empty() {
            None
        } else {
            Some(ClientSecret::new(client_secret_clean.to_string()))
        };

        tracing::debug!("🔵 OIDC Setup: Client ID = '{}'", client_id);
        tracing::debug!("🔵 OIDC Setup: Redirect  = '{}'", redirect_url);
        tracing::debug!(
            "🔵 OIDC Setup: Secret    = {:?}",
            if client_secret_opt.is_some() {
                "SET"
            } else {
                "NONE"
            }
        );

        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let provider_metadata =
            CoreProviderMetadata::discover_async(IssuerUrl::new(issuer)?, &http_client).await?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(client_id),
            client_secret_opt,
        )
        .set_redirect_uri(RedirectUrl::new(redirect_url)?);

        Ok(Self {
            client,
            resource_id,
        })
    }

    // todo: replace this tuple with newtype
    pub fn get_authorization_url(&self) -> (String, String, String, String) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token, nonce) = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        (
            auth_url.to_string(),
            csrf_token.secret().to_string(),
            nonce.secret().to_string(),
            pkce_verifier.secret().to_string(),
        )
    }

    //todo: replace strings with newtype
    pub async fn resolve_callback(
        &self,
        code: String,
        nonce: String,
        pkce_verifier: String,
    ) -> anyhow::Result<OidcUser> {
        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let pkce_verifier = PkceCodeVerifier::new(pkce_verifier);
        let nonce = Nonce::new(nonce);

        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code))?
            .set_pkce_verifier(pkce_verifier)
            .request_async(&http_client)
            .await?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| anyhow!("Server did not return an ID token"))?;

        let mut id_token_verifier = self.client.id_token_verifier().clone();

        let trusted_resource_id = self.resource_id.clone();

        if let Some(resource_id) = trusted_resource_id {
            id_token_verifier = id_token_verifier
                .set_other_audience_verifier_fn(move |aud| aud.as_str() == resource_id);
        }

        let claims = id_token.claims(&id_token_verifier, &nonce)?;

        if let Some(expected_access_token_hash) = claims.access_token_hash() {
            let actual_access_token_hash = AccessTokenHash::from_token(
                token_response.access_token(),
                id_token.signing_alg()?,
                id_token.signing_key(&id_token_verifier)?,
            )?;

            if actual_access_token_hash != *expected_access_token_hash {
                return Err(anyhow!("Invalid access token"));
            }
        }

        let email = if let Some(email) = claims.email() {
            Some(email.as_str().to_string())
        } else {
            // Fallback: Call UserInfo Endpoint using the Access Token
            tracing::debug!("🔵 Email missing in ID Token, fetching UserInfo...");

            let user_info: UserInfoClaims<EmptyAdditionalClaims, CoreGenderClaim> = self
                .client
                .user_info(token_response.access_token().clone(), None)?
                .request_async(&http_client)
                .await?;

            user_info.email().map(|e| e.as_str().to_string())
        };

        // If email is still missing, we must error out because your app requires valid emails
        let email =
            email.ok_or_else(|| anyhow!("User has no verified email address in ZITADEL"))?;

        Ok(OidcUser {
            subject: claims.subject().to_string(),
            email,
        })
    }
}
