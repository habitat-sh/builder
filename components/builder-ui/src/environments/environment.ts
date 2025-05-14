// This file can be replaced during build by using the `fileReplacements` array.
// `ng build --configuration=production` replaces `environment.ts` with `environment.prod.ts`.

export const environment = {
  production: false,
  apiUrl: 'http://localhost:9636',
  useMocks: true, // Keep this as true to use our new mock events interceptor
  oauthClientId: 'e058b7c2a5907b8c11e6', // Development GitHub OAuth client ID
  featureFlags: {
    enableNewFeatures: true,
    // Updated feature flags for events
    enable_builder_events: true,
    enable_builder_events_saas: true,
    enableVisibility: true,
    saas: true  // Enable Service Status link in development mode
  },
  urls: {
    docs: 'https://www.habitat.sh/docs',
    tutorials: 'https://learn.chef.io/habitat/',
    source: 'https://github.com/habitat-sh/habitat',
    slack: 'https://slack.habitat.sh',
    download: 'https://www.habitat.sh/docs/install-habitat/'
  }
};
