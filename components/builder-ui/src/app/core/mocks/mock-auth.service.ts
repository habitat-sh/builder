import { Injectable } from '@angular/core';
import { Observable, of } from 'rxjs';
import { delay } from 'rxjs/operators';

/**
 * Mock authentication service for development
 * This service simulates OAuth authentication flow for GitHub
 */
@Injectable({
  providedIn: 'root'
})
export class MockAuthService {
  
  // Store the OAuth state for CSRF protection
  private state = '';
  
  // Store simulated user data
  private mockUser = {
    id: 'user-12345',
    name: 'Demo User',
    email: 'demo@example.com',
    avatar: 'https://avatars.githubusercontent.com/u/12345?v=4',
    role: 'contributor',
    permissions: ['read:packages', 'write:packages', 'read:origins', 'create:origins']
  };
  
  // Whether the user is authenticated
  private _isAuthenticated = false;
  
  // The access token
  private token = '';
  
  /**
   * Generate an authorization URL for GitHub OAuth
   */
  getAuthorizationUrl(): string {
    // Generate a random state string
    this.state = this.generateRandomState();
    
    // Store the state in localStorage for verification
    localStorage.setItem('oauth_state', this.state);
    
    // For mock purposes, create a local URL that can be intercepted
    const callbackUrl = `http://localhost:4200/auth/mock-callback?state=${this.state}`;
    
    return callbackUrl;
  }
  
  /**
   * Generate a random state string for CSRF protection
   */
  private generateRandomState(): string {
    const randomArray = new Uint8Array(16);
    window.crypto.getRandomValues(randomArray);
    return Array.from(randomArray)
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
  }
  
  /**
   * Simulate the OAuth callback process
   * @param state The state parameter from the callback
   * @param code The authorization code from the callback
   */
  handleCallback(state: string, code: string): Observable<any> {
    // Verify state parameter
    const storedState = localStorage.getItem('oauth_state');
    if (!state || state !== storedState) {
      return of({ error: 'invalid_state' }).pipe(delay(800));
    }
    
    // Clear the stored state
    localStorage.removeItem('oauth_state');
    
    // Simulate token generation
    this.token = 'mock_token_' + Math.random().toString(36).substring(2);
    this._isAuthenticated = true;
    
    // Return mock user data
    return of({
      token: this.token,
      user: this.mockUser
    }).pipe(delay(1200)); // Add delay to simulate network request
  }
  
  /**
   * Exchange OAuth code for access token (mock implementation)
   * @param code Authorization code from OAuth provider
   * @returns Observable with user data
   */
  exchangeCodeForToken(code: string): Observable<any> {
    // Generate a mock token
    this.token = 'mock_token_' + Math.random().toString(36).substring(2);
    this._isAuthenticated = true;
    
    // Return mock user data
    return of({
      token: this.token,
      user: this.mockUser
    }).pipe(delay(1200)); // Add delay to simulate network request
  }
  
  /**
   * Check if the user is authenticated
   */
  isAuthenticated(): boolean {
    return this._isAuthenticated;
  }
  
  /**
   * Get the current user data
   */
  getUserData(): Observable<any> {
    if (!this._isAuthenticated) {
      return of(null);
    }
    
    return of(this.mockUser).pipe(delay(500));
  }
  
  /**
   * Simulate user logout
   */
  logout(): Observable<boolean> {
    this._isAuthenticated = false;
    this.token = '';
    return of(true).pipe(delay(300));
  }
  
  /**
   * Check if the user has a specific permission
   */
  hasPermission(permission: string): boolean {
    if (!this._isAuthenticated || !this.mockUser.permissions) {
      return false;
    }
    return this.mockUser.permissions.includes(permission);
  }
  
  /**
   * Check if the user has any of the specified roles
   */
  hasRole(roles: string | string[]): boolean {
    if (!this._isAuthenticated || !this.mockUser.role) {
      return false;
    }
    
    const rolesToCheck = Array.isArray(roles) ? roles : [roles];
    return rolesToCheck.includes(this.mockUser.role);
  }
  
  /**
   * Set redirect URL for after login
   */
  setRedirectUrl(url: string): void {
    // Store the redirect URL
    localStorage.setItem('auth_redirect_url', url);
  }
  
  /**
   * Get and clear the redirect URL
   */
  getAndClearRedirectUrl(): string | null {
    const url = localStorage.getItem('auth_redirect_url');
    localStorage.removeItem('auth_redirect_url');
    return url;
  }
}
