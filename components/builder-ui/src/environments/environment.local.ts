// Local development environment configuration
// This file should be gitignored and not committed to version control

import { environment as devEnvironment } from './environment';

export const environment = {
  ...devEnvironment,
  useMocks: true, // Use mock data for development
  oauthClientId: 'e058b7c2a5907b8c11e6', // Development GitHub OAuth client ID
  oauthClientSecret: '', // Leave blank and set this in your local environment
};
