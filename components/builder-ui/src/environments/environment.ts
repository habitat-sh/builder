// This file can be replaced during build by using the `fileReplacements` array.
// `ng build --configuration=production` replaces `environment.ts` with `environment.prod.ts`.

export const environment = {
  production: false,
  apiUrl: 'http://localhost:9636',
  useMocks: true,
  featureFlags: {
    enableNewFeatures: true,
    enableEvents: true,
    enableSaasEvents: false,
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
