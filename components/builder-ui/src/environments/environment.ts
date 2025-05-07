// This file can be replaced during build by using the `fileReplacements` array.
// `ng build --configuration=production` replaces `environment.ts` with `environment.prod.ts`.

export const environment = {
  production: false,
  apiUrl: 'http://localhost:9636',
  useMocks: true,
  featureFlags: {
    enableNewFeatures: true
  }
};
