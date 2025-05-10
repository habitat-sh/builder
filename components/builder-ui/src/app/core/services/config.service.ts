import { Injectable } from '@angular/core';
import { environment } from '../../../environments/environment';
import { Observable, of } from 'rxjs';

/**
 * Configuration interface for the application
 */
export interface AppConfig {
  oauthProvider?: string;
  oauthSignupUrl?: string;
  oauthClientId?: string;
  oauthAuthorizationUrl?: string;
  oauthRedirectUrl?: string;
  wwwUrl?: string;
  [key: string]: any;
}

/**
 * Service for accessing application configuration values.
 * Centralizes access to environment variables and feature flags.
 */
@Injectable({
  providedIn: 'root'
})
export class ConfigService {
  
  /**
   * Get the application configuration
   * @returns An observable with the application config
   */
  getConfig(): Observable<AppConfig> {
    // Create config object with values from environment
    const config: AppConfig = {
      oauthProvider: 'GitHub', // Default OAuth provider
      oauthSignupUrl: 'https://github.com/join',
      oauthClientId: environment.oauthClientId || '{{developerClientId}}', // Will be replaced in production
      oauthAuthorizationUrl: 'https://github.com/login/oauth/authorize',
      oauthRedirectUrl: `${window.location.origin}/auth/callback`,
      wwwUrl: 'https://www.habitat.sh'
    };
    
    return of(config);
  }
  
  /**
   * Check if a feature flag is enabled
   * @param flag The feature flag to check
   * @returns True if the feature is enabled
   */
  isFeatureEnabled(flag: string): boolean {
    return environment.featureFlags && 
           environment.featureFlags[flag as keyof typeof environment.featureFlags] === true;
  }
  
  /**
   * Get a URL from the environment configuration
   * @param key The URL key
   * @returns The configured URL
   */
  getUrl(key: string): string {
    return environment.urls && 
           environment.urls[key as keyof typeof environment.urls] || '';
  }
  
  /**
   * Get the API URL prefix
   * @returns The API URL
   */
  getApiUrl(): string {
    return environment.apiUrl;
  }
  
  /**
   * Check if the application is running in production mode
   * @returns True if in production mode
   */
  isProduction(): boolean {
    return environment.production;
  }
  
  /**
   * Check if mock data should be used
   * @returns True if mocks should be used
   */
  useMocks(): boolean {
    return environment.useMocks;
  }
  
  /**
   * Get all feature flags
   * @returns The feature flags object
   */
  getFeatureFlags(): Record<string, boolean> {
    return environment.featureFlags;
  }
  
  /**
   * Get all URL configurations
   * @returns The URLs object
   */
  getUrls(): Record<string, string> {
    return environment.urls;
  }
}
