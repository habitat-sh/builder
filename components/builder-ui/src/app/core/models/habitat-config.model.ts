/**
 * Interface defining the Habitat Builder configuration options
 */
export interface HabitatConfig {
  // Company information, for analytics (optional)
  company_id: string;
  company_name: string;

  // Cookie domain (optional; e.g., 'bldr.company.co')
  cookie_domain: string;

  // URL for documentation
  docs_url: string;

  // Enable builder-specific features
  enable_builder: boolean;

  // Ability to set project visibility
  enable_visibility: boolean;

  // Supported container-registry integrations
  enable_publisher_amazon: boolean;
  enable_publisher_azure: boolean;
  enable_publisher_docker: boolean;

  // Environment in which we're running. If "production", enable production mode
  environment: 'development' | 'staging' | 'production';

  // URL for GitHub API
  github_api_url: string;

  // Habitat Builder GitHub app URL
  github_app_url: string;

  // Habitat Builder GitHub app ID
  github_app_id: string;

  // Whether we're running in a SaaS environment
  is_saas: boolean;

  // OAuth properties
  oauth_authorize_url: string;
  oauth_client_id: string;
  oauth_provider: 'github' | 'bitbucket';
  oauth_redirect_url: string;
  oauth_signup_url: string;

  // URL for the Habitat source code
  source_code_url: string;

  // Status URL
  status_url: string;

  // URL for tutorials
  tutorials_url: string;

  // Whether to use Gravatar for users whose profiles have email addresses
  use_gravatar: boolean;

  // Version of the software we're running
  version: string;

  // Main site URL
  www_url: string;

  // Enable/Disable builder events
  enable_builder_events: boolean;

  // Enable/Disable builder events from SaaS
  // 'enable_builder_events' property also needs to be set to enable SaaS events
  enable_builder_events_saas: boolean;

  // Enable/Disable LTS channel from SaaS
  enable_base: boolean;

  // Default base channel name
  latest_base_default_channel: string;
}
