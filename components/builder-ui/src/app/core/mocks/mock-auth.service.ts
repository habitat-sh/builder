import { Injectable } from '@angular/core';
import { Observable, of, BehaviorSubject } from 'rxjs';
import { delay } from 'rxjs/operators';
import { Router } from '@angular/router';
import { signal, computed } from '@angular/core';

/**
 * Mock authentication service for development
 * This service simulates OAuth authentication flow for GitHub
 */
@Injectable({
  providedIn: 'root'
})
export class MockAuthService {
  // Storage keys
  private readonly AUTH_TOKEN_KEY = 'mock_auth_token';
  private readonly USER_DATA_KEY = 'mock_user_data';
  
  // Store the OAuth state for CSRF protection
  private state = '';
  
  // Legacy status subject for components that use the observable API
  private authStatusSource = new BehaviorSubject<boolean>(false);
  authStatus$ = this.authStatusSource.asObservable();
  
  // Signal-based state (for compatibility with real AuthService)
  private _authSignal = signal<{
    isAuthenticated: boolean,
    user: any | null,
    token: string | null
  }>({
    isAuthenticated: false,
    user: null,
    token: null
  });
  
  readonly isAuthenticated = computed(() => {
    const result = this._authSignal().isAuthenticated;
    console.log('MockAuthService: isAuthenticated computed property called, returning:', result);
    return result;
  });
  readonly token = computed(() => this._authSignal().token);
  readonly user = computed(() => this._authSignal().user);
  
  // Store simulated user data
  private mockUser = {
    id: 'user-12345',
    name: 'Demo User',
    email: 'demo@example.com',
    avatar: 'https://avatars.githubusercontent.com/u/12345?v=4',
    role: 'contributor',
    permissions: ['read:packages', 'write:packages', 'read:origins', 'create:origins']
  };
  
  // Whether the user is authenticated (legacy property)
  private _isAuthenticated = false;
  
  // The access token (legacy property)
  private token_value = '';
  
  constructor(private router: Router) {
    this.loadAuthStateFromStorage();
  }
  
  /**
   * Generate an authorization URL for GitHub OAuth
   */
  getAuthorizationUrl(): string {
    // Generate a random state string
    this.state = this.generateRandomState();
    
    // Store the state in localStorage for verification
    localStorage.setItem('oauth_state', this.state);
    
    // For mock purposes, create a local URL that can be intercepted
    // Use current window location origin to avoid hardcoded port issues
    const callbackUrl = `${window.location.origin}/auth/mock-callback?state=${this.state}`;
    
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
    console.log('MockAuthService: Handling OAuth callback');
    
    // Verify state parameter
    const storedState = localStorage.getItem('oauth_state');
    if (!state || state !== storedState) {
      return of({ error: 'invalid_state' }).pipe(delay(800));
    }
    
    // Clear the stored state
    localStorage.removeItem('oauth_state');
    
    // Simulate token generation
    this.token_value = 'mock_token_' + Math.random().toString(36).substring(2);
    this._isAuthenticated = true;
    
    // Save authentication state to local storage
    this.saveAuthState(this.token_value, this.mockUser);
    
    // Update signal state
    this._authSignal.set({
      isAuthenticated: true,
      user: this.mockUser,
      token: this.token_value
    });
    
    // Update legacy status subject - this is critical for components that use the observable API
    this.authStatusSource.next(true);
    
    console.log('MockAuthService: Authentication successful', { 
      authenticated: this._isAuthenticated,
      user: this.mockUser
    });
    
    // Return mock user data
    return of({
      token: this.token_value,
      user: this.mockUser
    }).pipe(delay(1200)); // Add delay to simulate network request
  }
  
  /**
   * Exchange OAuth code for access token (mock implementation)
   * @param code Authorization code from OAuth provider
   * @returns Observable with user data
   */
  exchangeCodeForToken(code: string): Observable<any> {
    console.log('MockAuthService: Exchanging code for token');
    
    // Generate a mock token
    this.token_value = 'mock_token_' + Math.random().toString(36).substring(2);
    this._isAuthenticated = true;
    
    // Save authentication state to local storage
    this.saveAuthState(this.token_value, this.mockUser);
    
    // Update signal state
    this._authSignal.set({
      isAuthenticated: true,
      user: this.mockUser,
      token: this.token_value
    });
    
    // Update legacy status subject - this is critical for components that use the observable API
    this.authStatusSource.next(true);
    
    console.log('MockAuthService: Authentication successful', { 
      authenticated: this._isAuthenticated,
      user: this.mockUser
    });
    
    // Return mock user data
    return of({
      token: this.token_value,
      user: this.mockUser
    }).pipe(delay(1200)); // Add delay to simulate network request
  }
  
  /**
   * Check if the user is authenticated (legacy method)
   */
  isUserAuthenticated(): boolean {
    return this._authSignal().isAuthenticated;
  }
  
  /**
   * Get the current user data
   */
  getUserData(): Observable<any> {
    if (!this._authSignal().isAuthenticated) {
      return of(null);
    }
    
    return of(this._authSignal().user).pipe(delay(500));
  }
  
  /**
   * Simulate user logout
   * @param redirect Whether to redirect after logout (matches real AuthService signature)
   */
  logout(redirect: boolean = true): Observable<boolean> {
    this._isAuthenticated = false;
    this.token_value = '';
    
    // Clear auth data from local storage
    localStorage.removeItem(this.AUTH_TOKEN_KEY);
    localStorage.removeItem(this.USER_DATA_KEY);
    
    // Update signal state
    this._authSignal.set({
      isAuthenticated: false,
      user: null,
      token: null
    });
    
    // Update legacy status subject
    this.authStatusSource.next(false);
    
    if (redirect) {
      this.router.navigate(['/sign-in']);
    }
    
    return of(true).pipe(delay(300));
  }
  
  /**
   * Save authentication state to local storage
   */
  private saveAuthState(token: string, user: any): void {
    // Save to local storage
    localStorage.setItem(this.AUTH_TOKEN_KEY, token);
    localStorage.setItem(this.USER_DATA_KEY, JSON.stringify(user));
  }
  
  /**
   * Load authentication state from local storage
   */
  private loadAuthStateFromStorage(): void {
    const token = localStorage.getItem(this.AUTH_TOKEN_KEY);
    if (!token) {
      console.log('MockAuthService: No token found in storage, user is not authenticated');
      return;
    }
    
    const userJson = localStorage.getItem(this.USER_DATA_KEY);
    if (!userJson) {
      console.log('MockAuthService: No user data found in storage, user is not authenticated');
      return;
    }
    
    try {
      const user = JSON.parse(userJson);
      this.token_value = token;
      this._isAuthenticated = true;
      this.mockUser = user;
      
      // Update signal state
      this._authSignal.set({
        isAuthenticated: true,
        user: this.mockUser,
        token: this.token_value
      });
      
      // Update legacy status subject - this is critical for components that use the observable API
      this.authStatusSource.next(true);
      
      console.log('MockAuthService: Loaded auth state from storage', { 
        authenticated: this._isAuthenticated,
        isAuthenticatedSignal: this._authSignal().isAuthenticated,
        user: this.mockUser
      });
    } catch (error) {
      console.error('MockAuthService: Failed to parse mock user data', error);
    }
  }
  
  /**
   * Check if the user has a specific permission
   */
  hasPermission(permission: string): boolean {
    const user = this._authSignal().user;
    if (!this._authSignal().isAuthenticated || !user || !user.permissions) {
      return false;
    }
    return user.permissions.includes(permission);
  }
  
  /**
   * Check if the user has any of the specified roles
   */
  hasRole(roles: string | string[]): boolean {
    const user = this._authSignal().user;
    if (!this._authSignal().isAuthenticated || !user || !user.role) {
      return false;
    }
    
    const rolesToCheck = Array.isArray(roles) ? roles : [roles];
    return rolesToCheck.includes(user.role);
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
  
  /**
   * Get current user (API compatible with real AuthService)
   */
  currentUser() {
    return this._authSignal().isAuthenticated ? this._authSignal().user : null;
  }
  
  /**
   * Get current token (API compatible with real AuthService)
   */
  getToken(): string {
    return this._authSignal().token || '';
  }
}
