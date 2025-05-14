# BuilderUi

This project was generated using [Angular CLI](https://github.com/angular/angular-cli) version 19.0.5.

## Configuration Setup

Before starting development, you need to set up your local configuration:

1. Copy the sample configuration file to the public directory:
   ```bash
   cp habitat.conf.sample.js public/habitat.conf.js
   ```
   
2. Edit `public/habitat.conf.js` with your specific development settings:
   ```javascript
   // You can customize these settings for your development environment
   oauth_client_id: "your-actual-client-id", 
   
   // Update the redirect URL if needed
   oauth_redirect_url: "http://localhost:4200/",
   ```

> **Important Notes:** 
> 
> - The `habitat.conf.js` file is git-ignored and will not be committed to the repository. This allows each developer to have their own local configuration without affecting others.
> - In production, `habitat.conf.js` should be deployed separately from the application bundle and served from the root URL of the site.
> - If no `habitat.conf.js` file is found, the application will use default values and show an informational notice.

### Configuration Safety

Remember that `habitat.conf.js` may contain sensitive credentials and is excluded from Git tracking.
Always be careful not to accidentally commit this file to the repository.

## Configuration System

The Builder UI uses an external configuration system that loads settings from a `habitat.conf.js` file located in the public directory. This allows for easy customization across different environments (development, staging, production) without recompiling the application.

For detailed information, see [Configuration System Documentation](src/docs/configuration-system.md).

### How the Configuration Works

1. The `habitat.conf.js` file is loaded before the Angular application starts
2. Configuration is globally available via `window.Habitat.config`
3. The `HabitatConfigService` provides type-safe access to configuration values
4. Default values are used as fallbacks if specific settings are not present

### Customizing Configuration

To customize the configuration for different environments:

1. Edit `public/habitat.conf.js` for local development
2. For production deployments, place a customized `habitat.conf.js` at the root of the deployment

```javascript
// Example habitat.conf.js
habitatConfig({
  // Company information
  company_id: "my-company-id",
  company_name: "My Company Name",
  
  // OAuth settings 
  oauth_provider: "github",  // can be "github" or "bitbucket"
  oauth_client_id: "your-oauth-client-id",
  oauth_redirect_url: "https://your-site.com/",
  oauth_authorize_url: "https://github.com/login/oauth/authorize",
  
  // Environment settings
  environment: "production", // can be "development", "staging", or "production"
  
  // ... other settings
});
```

See `habitat-config.model.ts` for the full list of available configuration options.

## Development server

To start a local development server, run:

```bash
ng serve
```

Once the server is running, open your browser and navigate to `http://localhost:4200/`. The application will automatically reload whenever you modify any of the source files.

## Code scaffolding

Angular CLI includes powerful code scaffolding tools. To generate a new component, run:

```bash
ng generate component component-name
```

For a complete list of available schematics (such as `components`, `directives`, or `pipes`), run:

```bash
ng generate --help
```

## Building

To build the project run:

```bash
ng build
```

This will compile your project and store the build artifacts in the `dist/` directory. By default, the production build optimizes your application for performance and speed.

## Running unit tests

To execute unit tests with the [Karma](https://karma-runner.github.io) test runner, use the following command:

```bash
ng test
```

## Image Loading Strategy

This application implements a comprehensive strategy for reliable image loading with proper fallbacks. See [Image Loading Documentation](src/docs/image-loading.md) for details on how images are managed and how to implement proper fallbacks.

## Typography and Font System

The application uses a consistent typography system that matches the original builder-web implementation. See [Font Documentation](src/docs/fonts.md) for details on the font families and their implementation.

## Responsive Design

The application implements responsive design patterns to ensure proper display across different device sizes. See [Responsive Design Documentation](src/docs/responsive-design.md) for details on our approach to responsive layouts, including the footer component.

## Running end-to-end tests

For end-to-end (e2e) testing, run:

```bash
ng e2e
```

Angular CLI does not come with an end-to-end testing framework by default. You can choose one that suits your needs.

## Style Guide and Design Standards

This project follows the centralized style guide located at:
```
/components/builder-web/STYLE_GUIDE.md
```

The style guide contains comprehensive information on:
- Color palette
- Typography
- Layout guidelines
- Component styling
- Responsive design principles
- Accessibility standards

When making UI changes, please refer to this central document to ensure consistency across the application.

## Component Documentation

Additional documentation for specific components and layout implementations:

- [Fonts Implementation](/src/docs/fonts.md)
- [Responsive Design Guidelines](/src/docs/responsive-design.md)
- [Footer Layout](/src/docs/footer-layout.md)

## Additional Resources

For more information on using the Angular CLI, including detailed command references, visit the [Angular CLI Overview and Command Reference](https://angular.dev/tools/cli) page.
