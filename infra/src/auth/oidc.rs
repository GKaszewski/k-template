use anyhow::anyhow;
use domain::{
    AuthorizationCode, AuthorizationUrlData, ClientId, ClientSecret, CsrfToken, IssuerUrl,
    OidcNonce, PkceVerifier, RedirectUrl, ResourceId,
};
use openidconnect::{
    AccessTokenHash, Client, EmptyAdditionalClaims, EndpointMaybeSet, EndpointNotSet, EndpointSet,
    OAuth2TokenResponse, PkceCodeChallenge, Scope, StandardErrorResponse, TokenResponse,
    UserInfoClaims,
    core::{
        CoreAuthDisplay, CoreAuthPrompt, CoreAuthenticationFlow, CoreClient, CoreErrorResponseType,
        CoreGenderClaim, CoreJsonWebKey, CoreJweContentEncryptionAlgorithm, CoreProviderMetadata,
        CoreRevocableToken, CoreRevocationErrorResponse, CoreTokenIntrospectionResponse,
        CoreTokenResponse,
    },
    reqwest,
};
use serde::{Deserialize, Serialize};

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

/// Serializable OIDC state stored in an encrypted cookie during the auth code flow
#[derive(Debug, Serialize, Deserialize)]
pub struct OidcState {
    pub csrf_token: CsrfToken,
    pub nonce: OidcNonce,
    pub pkce_verifier: PkceVerifier,
}

#[derive(Clone)]
pub struct OidcService {
    client: OidcClient,
    http_client: reqwest::Client,
    resource_id: Option<ResourceId>,
}

#[derive(Debug)]
pub struct OidcUser {
    pub subject: String,
    pub email: String,
}

impl OidcService {
    /// Create a new OIDC service with validated configuration newtypes
    pub async fn new(
        issuer: IssuerUrl,
        client_id: ClientId,
        client_secret: Option<ClientSecret>,
        redirect_url: RedirectUrl,
        resource_id: Option<ResourceId>,
    ) -> anyhow::Result<Self> {
        tracing::debug!("🔵 OIDC Setup: Client ID = '{}'", client_id);
        tracing::debug!("🔵 OIDC Setup: Redirect  = '{}'", redirect_url);
        tracing::debug!(
            "🔵 OIDC Setup: Secret    = {:?}",
            if client_secret.is_some() { "SET" } else { "NONE" }
        );

        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let provider_metadata = CoreProviderMetadata::discover_async(
            openidconnect::IssuerUrl::new(issuer.as_ref().to_string())?,
            &http_client,
        )
        .await?;

        let oidc_client_id = openidconnect::ClientId::new(client_id.as_ref().to_string());
        let oidc_client_secret = client_secret
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| openidconnect::ClientSecret::new(s.as_ref().to_string()));
        let oidc_redirect_url =
            openidconnect::RedirectUrl::new(redirect_url.as_ref().to_string())?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            oidc_client_id,
            oidc_client_secret,
        )
        .set_redirect_uri(oidc_redirect_url);

        Ok(Self {
            client,
            http_client,
            resource_id,
        })
    }

    /// Get the authorization URL and associated state for OIDC login.
    ///
    /// Returns `(AuthorizationUrlData, OidcState)` where `OidcState` should be
    /// serialized and stored in an encrypted cookie for the duration of the flow.
    pub fn get_authorization_url(&self) -> (AuthorizationUrlData, OidcState) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token, nonce) = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                openidconnect::CsrfToken::new_random,
                openidconnect::Nonce::new_random,
            )
            .add_scope(Scope::new("profile".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        let oidc_state = OidcState {
            csrf_token: CsrfToken::new(csrf_token.secret().to_string()),
            nonce: OidcNonce::new(nonce.secret().to_string()),
            pkce_verifier: PkceVerifier::new(pkce_verifier.secret().to_string()),
        };

        let auth_data = AuthorizationUrlData {
            url: auth_url.into(),
            csrf_token: oidc_state.csrf_token.clone(),
            nonce: oidc_state.nonce.clone(),
            pkce_verifier: oidc_state.pkce_verifier.clone(),
        };

        (auth_data, oidc_state)
    }

    /// Resolve the OIDC callback with type-safe parameters
    pub async fn resolve_callback(
        &self,
        code: AuthorizationCode,
        nonce: OidcNonce,
        pkce_verifier: PkceVerifier,
    ) -> anyhow::Result<OidcUser> {
        let oidc_pkce_verifier =
            openidconnect::PkceCodeVerifier::new(pkce_verifier.as_ref().to_string());
        let oidc_nonce = openidconnect::Nonce::new(nonce.as_ref().to_string());

        let token_response = self
            .client
            .exchange_code(openidconnect::AuthorizationCode::new(
                code.as_ref().to_string(),
            ))?
            .set_pkce_verifier(oidc_pkce_verifier)
            .request_async(&self.http_client)
            .await?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| anyhow!("Server did not return an ID token"))?;

        let mut id_token_verifier = self.client.id_token_verifier().clone();

        if let Some(resource_id) = &self.resource_id {
            let trusted_resource_id = resource_id.as_ref().to_string();
            id_token_verifier = id_token_verifier
                .set_other_audience_verifier_fn(move |aud| aud.as_str() == trusted_resource_id);
        }

        let claims = id_token.claims(&id_token_verifier, &oidc_nonce)?;

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
            tracing::debug!("🔵 Email missing in ID Token, fetching UserInfo...");

            let user_info: UserInfoClaims<EmptyAdditionalClaims, CoreGenderClaim> = self
                .client
                .user_info(token_response.access_token().clone(), None)?
                .request_async(&self.http_client)
                .await?;

            user_info.email().map(|e| e.as_str().to_string())
        };

        let email =
            email.ok_or_else(|| anyhow!("User has no verified email address in ZITADEL"))?;

        Ok(OidcUser {
            subject: claims.subject().to_string(),
            email,
        })
    }
}
