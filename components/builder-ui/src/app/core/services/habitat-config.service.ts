import { Injectable, signal } from '@angular/core';
import { HabitatConfig } from '../models/habitat-config.model';

/**
 * Service to access the Habitat configuration loaded from habitat.conf.js
 * 
 * This service provides type-safe access to the window.Habitat.config object
 * which is loaded before the Angular application starts.
 * 
 * DEVELOPER NOTE:
 * ----------------------------------
 * Configuration Flow:
 * 1. index.html loads habitat.conf.js before Angular starts
 * 2. habitat.conf.js calls habitatConfig() with configuration object
 * 3. habitatConfig() sets window.Habitat.config
 * 4. This service reads from window.Habitat.config
 * 
 * To set up your development environment:
 * 1. Copy habitat.conf.sample.js to public/habitat.conf.js
 * 2. Edit habitat.conf.js with your OAuth credentials
 * 
 * For complete documentation, see:
 * src/docs/configuration-system.md
 * ----------------------------------
 */
@Injectable({
  providedIn: 'root'
})
export class HabitatConfigService {
  // Default configuration with sensible defaults
  private defaultConfig: HabitatConfig = {
    company_id: 'builder-dev',
    company_name: 'Habitat Builder Dev',
    cookie_domain: '',
    docs_url: 'https://www.habitat.sh/docs',
    enable_builder: true,
    enable_visibility: true,
    enable_publisher_amazon: false,
    enable_publisher_azure: false,
    enable_publisher_docker: false,
    environment: 'development',
    github_api_url: 'https://api.github.com',
    github_app_url: 'https://github.com/apps/habitat-builder-dev',
    github_app_id: '5629',
    is_saas: false,
    oauth_authorize_url: 'https://github.com/login/oauth/authorize',
    oauth_client_id: 'Iv1.732260b62f84db15',
    oauth_provider: 'github',
    oauth_redirect_url: 'http://localhost:3000/',
    oauth_signup_url: 'https://github.com/join',
    source_code_url: 'https://github.com/habitat-sh/habitat',
    status_url: 'https://status.habitat.sh/',
    tutorials_url: 'https://www.habitat.sh/learn',
    use_gravatar: true,
    version: '',
    www_url: 'https://www.habitat.sh',
    enable_builder_events: false,
    enable_builder_events_saas: false,
    enable_base: false,
    latest_base_default_channel: 'base'
  };

  // Signals for configuration state
  private _isLoadedFromFile = signal<boolean>(false);
  private _config: ReturnType<typeof signal<HabitatConfig>>;
  
  constructor() {
    // Initialize the config signal after the class is fully constructed
    this._config = signal<HabitatConfig>(this.loadConfig());
  }

  /**
   * Get the current configuration
   */
  get config(): HabitatConfig {
    return this._config();
  }

  /**
   * Check if configuration was loaded from external file
   * or if we're using default values
   */
  get isLoadedFromFile(): boolean {
    return this._isLoadedFromFile();
  }

  /**
   * Load configuration from window.Habitat.config
   * Falls back to default configuration if not available
   */
  private loadConfig(): HabitatConfig {
    let loadedFromFile = false;
    let config = this.defaultConfig;
    
    try {
      // Check if window.Habitat exists and has config
      if (typeof window !== 'undefined') {
        const habitatConfig = (window as any).Habitat;
        if (habitatConfig && habitatConfig.config) {
          // Merge the window config with default config - any missing values will use defaults
          loadedFromFile = true;
          config = { 
            ...this.defaultConfig, 
            ...habitatConfig.config 
          };
        }
      }
    } catch (error: any) {
      console.error('Failed to load Habitat configuration:', error);
    }

    if (!loadedFromFile) {
      console.log('Using default Habitat configuration - file not loaded or not found');
    } else {
      console.log('Habitat configuration loaded successfully from file');
    }
    
    // Update the isLoadedFromFile signal after initialization
    this._isLoadedFromFile.set(loadedFromFile);
    
    return config;
  }
  
  /**
   * Reload configuration from window.Habitat.config
   * Useful if configuration is updated at runtime
   */
  reloadConfig(): void {
    // Call loadConfig to get the new configuration
    const newConfig = this.loadConfig();
    this._config.set(newConfig);
    
    // The loadConfig method already updates the isLoadedFromFile signal
  }
}
