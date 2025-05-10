# Auth Module

This module handles user authentication for the Habitat Builder application, specifically focusing on GitHub OAuth integration.

## Components

### SignInComponent

The sign-in page allows users to authenticate using GitHub OAuth. It displays the GitHub login button and handles the EULA confirmation flow before redirecting to GitHub's authorization page.

Features:
- GitHub OAuth authorization flow
- Terms of Service/EULA confirmation dialog
- Error message handling
- Support for account creation via GitHub

### OAuthCallbackComponent

Handles the OAuth callback after GitHub authentication. This component processes the authorization code, exchanges it for an access token, and completes the authentication process.

Features:
- OAuth code exchange for access token
- Handles error responses
- Redirects users to their intended destination after login

## Services

The authentication flow depends on two core services:

### AuthService

Handles user authentication, token management, and session state:
- OAuth authorization URL generation
- Token exchange and storage
- User session management
- Mock authentication for development

### ConfigService

Provides configuration for the authentication flow:
- OAuth client ID
- Provider URLs
- Redirect URI settings

## Development Testing

For local development, the authentication flow can operate in two modes:

### Mock Mode

When `environment.useMocks` is set to `true`, the OAuth flow is simulated without requiring a real GitHub connection:
- A mock token is generated
- A mock user profile is created
- No actual GitHub communication occurs

### Live Mode

When integrating with a real GitHub OAuth application:
- Uses the client ID from environment configuration
- Requires backend API support for token exchange
- Follows the complete OAuth flow with GitHub

## Configuration

Authentication settings are defined in the environment files:
- `oauthClientId` - GitHub OAuth application client ID
- `apiUrl` - Backend API URL for token exchange

## Testing

Unit tests are provided for both the sign-in and callback components, with mocked services to simulate the authentication flow.
