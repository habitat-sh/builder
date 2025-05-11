export const environment = {
  production: true,
  apiUrl: '/v1',
  useMocks: false,
  featureFlags: {
    enableNewFeatures: false,
    enable_builder_events: true,
    enable_builder_events_saas: false,
    enableVisibility: true
  },
  urls: {
    docs: 'https://www.habitat.sh/docs',
    tutorials: 'https://learn.chef.io/habitat/',
    source: 'https://github.com/habitat-sh/habitat',
    slack: 'https://slack.habitat.sh',
    download: 'https://www.habitat.sh/docs/install-habitat/'
  }
};
