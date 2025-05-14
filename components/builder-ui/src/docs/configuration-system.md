# Configuration System

## Overview

The Builder UI uses an external configuration system that loads settings from a `habitat.conf.js` file. This approach allows for easy customization across different environments (development, staging, production) without recompiling the application.

## How It Works

1. The `habitat.conf.js` file is loaded before the Angular application starts
2. Configuration is globally available via `window.Habitat.config`
3. The `HabitatConfigService` provides type-safe access to configuration values
4. Default values are used as fallbacks if specific settings are not present

## Configuration Flow

Here's what happens when the application loads:

1. The browser loads the `index.html` file
2. The initialization script in `index.html` creates the `window.Habitat` object
3. The script tries to load `habitat.conf.js` from various locations
4. If found, the configuration file calls the `habitatConfig()` function
5. The Angular application initializes and `HabitatConfigService` reads from `window.Habitat.config`
6. The service provides the configuration throughout the application

## Configuration Structure

The configuration is defined by the `HabitatConfig` interface, which includes properties such as:

```typescript
export interface HabitatConfig {
  company_id: string;
  company_name: string;
  oauth_provider: 'github' | 'bitbucket';
  oauth_client_id: string;
  oauth_redirect_url: string;
  // ...and more
}
```

See `habitat-config.model.ts` for the complete interface definition.

## Development Setup

During development:
- Copy `habitat.conf.sample.js` (from project root) to `public/habitat.conf.js`
- Customize your local configuration in `public/habitat.conf.js` as needed
- The sample file contains reasonable defaults, but you should update them with your actual values when ready
- If no configuration file is found, the application will use built-in defaults and show an informational notice
- This file is excluded from git to prevent committing sensitive credentials

## Production Deployment

For production:
- The `habitat.conf.js` file should be deployed separately from the application bundle
- It should be served from the root URL of the site
- The configuration should be environment-specific

## Custom Configuration for Different Environments

You can create different configuration files for different environments:

```
habitat.conf.development.js
habitat.conf.staging.js
habitat.conf.production.js
```

Then deploy the appropriate file as `habitat.conf.js` based on the environment.

## Error Handling

If the configuration file fails to load:
1. The application will try to load it from multiple locations
2. If still not found, it will fall back to the sample configuration
3. A warning will be shown in the UI if the sample configuration is used
4. The application will use default values for any missing configurations
