habitatConfig({
    company_id: "{{cfg.analytics.company_id}}",
    company_name: "{{cfg.analytics.company_name}}",
    cookie_domain: "{{cfg.cookie_domain}}",
    demo_app_url: "{{cfg.demo_app_url}}",
    docs_url: "{{cfg.docs_url}}",
    enable_builder: {{ cfg.enable_builder }},
    enable_visibility: {{ cfg.enable_visibility }},
    enable_publisher_amazon: {{ cfg.enable_publisher_amazon }},
    enable_publisher_azure: {{ cfg.enable_publisher_azure }},
    enable_publisher_docker: {{ cfg.enable_publisher_docker }},
    enable_statuspage: {{ cfg.hosted }},
    environment: "{{cfg.environment}}",
    github_api_url: "{{cfg.github.api_url}}",
    github_app_url: "{{cfg.github.app_url}}",
    github_app_id: "{{cfg.github.app_id}}",
    oauth_authorize_url: "{{cfg.oauth.authorize_url}}",
    oauth_client_id: "{{cfg.oauth.client_id}}",
    oauth_provider: "{{cfg.oauth.provider}}",
    oauth_redirect_url: "{{cfg.oauth.redirect_url}}",
    oauth_signup_url: "{{cfg.oauth.signup_url}}",
    source_code_url: "{{cfg.source_code_url}}",
    status_url: "{{cfg.status_url}}",
    tutorials_url: "{{cfg.tutorials_url}}",
    use_gravatar: {{ cfg.use_gravatar }},
    version: "{{pkg.ident}}",
    www_url: "{{cfg.www_url}}",
    enable_builder_events: {{ cfg.enable_builder_events }},
    enable_builder_events_saas: {{ cfg.enable_builder_events_saas }},
    enable_base: {{ cfg.enable_base }},
    latest_base_default_channel: "{{ cfg.latest_base_default_channel }}",
});
