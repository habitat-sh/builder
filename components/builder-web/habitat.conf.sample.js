habitatConfig({

    // Cookie domain (optional; e.g., 'bldr.company.co')
    cookie_domain: "",

    // The URL for the Builder demo app
    demo_app_url: "https://www.habitat.sh/demo/build-system/steps/1/",

    // The URL for documentation
    docs_url: "https://www.habitat.sh/docs",

    // Enable Builder-specific features
    enable_builder: true,

    // Enable StatusPage.io integration
    enable_statuspage: false,

    // The environment in which we're running. If "production", enable production mode
    environment: "production",

    // The API URL for GitHub
    github_api_url: "https://api.github.com",

    // The Habitat Builder GitHub app
    github_app_url: "https://github.com/apps/habitat-builder-dev",

    // The Habitat Builder GitHub app ID
    github_app_id: "5629",

    // The URL for the Habitat API service (including the API version.) If
    // running the API services locally with `start-builder` from the root
    // of the builder repo, this will be localhost (if running Docker for Mac or
    // Linux) or the result of `$(docker-machine ip default)` if using Docker
    // in a virtual Machine.
    habitat_api_url: "http://localhost:9636",

    // OAuth properties
    oauth_authorize_url: "https://github.com/login/oauth/authorize",
    oauth_client_id: "Iv1.732260b62f84db15",
    oauth_provider: "github",
    oauth_redirect_url: "http://localhost:3000/",
    oauth_signup_url: "https://github.com/join",

    // oauth_authorize_url: "https://bitbucket.org/site/oauth2/authorize",
    // oauth_client_id: "5U6LKcQf4DvHMRFBeS",
    // oauth_provider: "bitbucket",
    // oauth_redirect_url: "http://localhost:3000/",
    // oauth_signup_url: "https://bitbucket.org/account/signup/",

    // The URL for the Habitat source code
    source_code_url: "https://github.com/habitat-sh/habitat",

    // The URL for status
    status_url: "https://status.habitat.sh/",

    // The URL for tutorials
    tutorials_url: "https://www.habitat.sh/learn",

    // Use Gravatar for users whose profiles have email addresses
    use_gravatar: true,

    // The version of the software we're running. In production, this should
    // be automatically populated by Habitat
    version: "",

    // The main website URL
    www_url: "https://www.habitat.sh"
});
