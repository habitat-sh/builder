import { Injectable, signal, computed, inject } from '@angular/core';
import { Router } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { Observable, throwError, of, BehaviorSubject } from 'rxjs';
import { catchError, map, tap, switchMap } from 'rxjs/operators';
import { jwtDecode } from 'jwt-decode';
import { environment } from '../../../environments/environment';
import { ApiService } from './api.service';
import { ConfigService } from './config.service';

export interface User {
  id: string;
  name: string;
  email: string;
  avatar?: string;
  role?: string;
  permissions?: string[];
  [key: string]: any; // For additional properties
}

export interface AuthState {
  isAuthenticated: boolean;
  user: User | null;
  token: string | null;
  tokenExpiration: Date | null;
}

@Injectable({
  providedIn: 'root'
})
export class AuthService {
  // Storage keys
  private readonly AUTH_TOKEN_KEY = 'auth_token';
  private readonly USER_DATA_KEY = 'user_data';

  // Config service for OAuth settings
  private configService = inject(ConfigService);
  
  // State signals
  private _authState = signal<AuthState>({
    isAuthenticated: false,
    user: null,
    token: null,
    tokenExpiration: null
  });
  
  // Public readable signals
  readonly isAuthenticated = computed(() => this._authState().isAuthenticated);
  readonly currentUser = computed(() => this._authState().user);
  readonly token = computed(() => this._authState().token);
  readonly tokenExpiration = computed(() => this._authState().tokenExpiration);
  
  // Login status subject for legacy components
  private authStatusSource = new BehaviorSubject<boolean>(false);
  authStatus$ = this.authStatusSource.asObservable();
  
  // For redirect after login
  private redirectUrl: string | null = null;

  // OAuth state for security
  private oauthState = '';
  
  constructor(
    private http: HttpClient,
    private apiService: ApiService,
    private router: Router
  ) {
    this.loadAuthStateFromStorage();
    // Generate random state value for OAuth security
    this.oauthState = this.generateRandomState();
    
    // Initialize cross-tab authentication sync
    this.initCrossTabSync();
  }
  
  /**
   * Initialize cross-tab authentication synchronization
   * This ensures all tabs have consistent auth state
   */
  private initCrossTabSync(): void {
    // Listen for storage events to detect auth changes in other tabs
    window.addEventListener('storage', (event) => {
      if (event.key === this.AUTH_TOKEN_KEY) {
        console.log('AuthService: Auth token changed in another tab');
        
        if (event.newValue) {
          // Another tab logged in or refreshed the token
          this.loadAuthStateFromStorage();
        } else {
          // Another tab logged out, clear our state too
          if (this.isAuthenticated()) {
            console.log('AuthService: Logout detected in another tab');
            // Use logout without redirect to avoid navigation loops
            this.logout(false);
          }
        }
      }
    });
    
    // Periodically check token expiration (every minute)
    setInterval(() => {
      if (this.isAuthenticated() && this.isTokenExpired()) {
        console.log('AuthService: Token is expired or about to expire, refreshing');
        this.refreshToken().subscribe({
          next: (success) => {
            if (!success) {
              console.error('AuthService: Auto-refresh failed, logging out');
              this.logout(true);
            }
          },
          error: () => {
            console.error('AuthService: Auto-refresh error, logging out');
            this.logout(true);
          }
        });
      }
    }, 60000); // Check every minute
  }

  /**
   * Generate a URL for OAuth authorization
   * @returns The full authorization URL to redirect to
   */
  getAuthorizationUrl(): string {
    // Generate a random state value for CSRF protection
    const state = this.generateRandomState();
    localStorage.setItem('oauth_state', state);
    
    // Get OAuth parameters from environment
    const clientId = environment.oauthClientId || '';
    const redirectUri = `${window.location.origin}/auth/callback`;
    const scope = 'user:email,read:org';
    
    // Construct the authorization URL
    const authUrl = new URL('https://github.com/login/oauth/authorize');
    authUrl.searchParams.set('client_id', clientId);
    authUrl.searchParams.set('redirect_uri', redirectUri);
    authUrl.searchParams.set('scope', scope);
    authUrl.searchParams.set('state', state);
    
    return authUrl.toString();
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
   * Attempt to login with username and password
   */
  login(username: string, password: string): Observable<User> {
    return this.apiService.post<{ token: string, user: User }>('auth/login', { username, password }).pipe(
      tap(response => {
        this.setAuthState(response.token, response.user);
      }),
      map(response => response.user),
      catchError(error => {
        console.error('Login error', error);
        return throwError(() => new Error('Invalid username or password'));
      })
    );
  }
  
  /**
   * Log out the current user
   * @param redirect Whether to redirect to the sign-in page after logout
   * @param returnTo Optional URL to redirect to after next sign-in
   */
  logout(redirect = true, returnTo?: string): void {
    // Keep track of the previous authentication state for analytics
    const wasAuthenticated = this.isAuthenticated();
    const userId = this._authState().user?.id;
    
    // Log if we're logging out an authenticated user
    if (wasAuthenticated) {
      console.log(`AuthService: Logging out user ${userId || 'unknown'}`);
      
      // Track logout timing for analytics
      const authStartTime = localStorage.getItem('auth_start_time');
      if (authStartTime) {
        const authDuration = Date.now() - parseInt(authStartTime, 10);
        console.log(`AuthService: Session duration was ${authDuration}ms`);
        localStorage.removeItem('auth_start_time');
      }
    }

    // Clear the auth state
    this._authState.set({
      isAuthenticated: false,
      user: null,
      token: null,
      tokenExpiration: null
    });
    
    // Update legacy status
    this.authStatusSource.next(false);
    
    // Clear auth-related storage but preserve other app settings
    localStorage.removeItem(this.AUTH_TOKEN_KEY);
    localStorage.removeItem(this.USER_DATA_KEY);
    localStorage.removeItem('oauth_state');
    
    // Store return URL if provided
    if (returnTo) {
      this.setRedirectUrl(returnTo);
    }
    
    // Redirect to login if specified
    if (redirect) {
      this.router.navigate(['/sign-in']);
    }
  }
  
  /**
   * Process OAuth callback
   */
  handleOAuthCallback(token: string): Observable<User> {
    return this.apiService.get<User>('auth/me', {}, {
      headers: { Authorization: `Bearer ${token}` }
    }).pipe(
      tap(user => {
        this.setAuthState(token, user);
      }),
      catchError(error => {
        console.error('OAuth callback error', error);
        return throwError(() => new Error('Failed to authenticate with OAuth provider'));
      })
    );
  }
  
  /**
   * Exchange OAuth code for access token
   * @param code Authorization code from OAuth provider
   * @returns Observable with user data
   */
  exchangeCodeForToken(code: string): Observable<User> {
    // In production, this would call the backend API to exchange the code
    // For development/testing with mocks, we'll simulate the exchange
    if (environment.useMocks) {
      console.log('Mock: Exchanging authorization code for token');
      
      // Create a mock token (for development only)
      const mockToken = 'mock_' + Math.random().toString(36).substring(2);
      
      // Create a mock user
      const mockUser: User = {
        id: '12345',
        name: 'Demo User',
        email: 'demo@example.com',
        avatar: 'https://avatars.githubusercontent.com/u/12345?v=4',
        role: 'contributor',
        permissions: ['read:packages', 'write:packages']
      };
      
      // Set authentication state
      this.setAuthState(mockToken, mockUser);
      
      return of(mockUser);
    } else {
      // In production, call the backend API to exchange the code for a token
      return this.apiService.post<{ token: string, user: User }>('auth/github/callback', {
        code: code
      }).pipe(
        tap(response => {
          this.setAuthState(response.token, response.user);
        }),
        map(response => response.user),
        catchError(error => {
          console.error('Token exchange error', error);
          return throwError(() => new Error('Failed to authenticate with OAuth provider'));
        })
      );
    }
  }
  
  /**
   * Check if the current user has a specific permission
   * @param permission The permission to check for
   * @returns boolean indicating whether user has the permission
   */
  hasPermission(permission: string): boolean {
    const user = this._authState().user;
    if (!user || !user.permissions) {
      return false;
    }
    
    return user.permissions.includes(permission);
  }
  
  /**
   * Check if the current user has any of the specified roles
   * @param roles A single role or array of roles to check for
   * @returns boolean indicating whether user has any of the roles
   */
  hasRole(roles: string | string[]): boolean {
    const user = this._authState().user;
    if (!user || !user.role) {
      return false;
    }
    
    if (Array.isArray(roles)) {
      return roles.includes(user.role);
    } else {
      return user.role === roles;
    }
  }
  
  /**
   * Refresh the authentication token
   */
  refreshToken(): Observable<boolean> {
    const currentToken = this._authState().token;
    
    if (!currentToken) {
      return of(false);
    }
    
    return this.apiService.post<{ token: string }>('auth/refresh', {
      token: currentToken
    }).pipe(
      switchMap(response => {
        const user = this._authState().user;
        if (user) {
          this.setAuthState(response.token, user);
          return of(true);
        } else {
          // If we have a token but no user, fetch the user data
          return this.apiService.get<User>('auth/me').pipe(
            tap(user => {
              this.setAuthState(response.token, user);
            }),
            map(() => true)
          );
        }
      }),
      catchError(() => {
        // If refresh fails, log out
        this.logout();
        return of(false);
      })
    );
  }
  
  /**
   * Check if current token is expired
   */
  isTokenExpired(): boolean {
    const expiration = this._authState().tokenExpiration;
    if (!expiration) return true;
    
    // Add buffer of 60 seconds
    return expiration.getTime() <= (Date.now() + (60 * 1000));
  }
  
  /**
   * Set authentication state and persist to storage
   */
  private setAuthState(token: string, user: User): void {
    // Decode token to get expiration
    let tokenExpiration: Date | null = null;
    try {
      const decodedToken = jwtDecode<{ exp: number }>(token);
      tokenExpiration = new Date(decodedToken.exp * 1000);
    } catch (error) {
      console.error('Failed to decode token', error);
    }
    
    // Update state
    this._authState.set({
      isAuthenticated: true,
      user,
      token,
      tokenExpiration
    });
    
    // Update legacy status
    this.authStatusSource.next(true);
    
    // Save to local storage
    localStorage.setItem(this.AUTH_TOKEN_KEY, token);
    localStorage.setItem(this.USER_DATA_KEY, JSON.stringify(user));
  }
  
  /**
   * Set redirect URL for after login
   */
  setRedirectUrl(url: string): void {
    this.redirectUrl = url;
  }
  
  /**
   * Get and clear the redirect URL
   */
  getAndClearRedirectUrl(): string | null {
    const url = this.redirectUrl;
    this.redirectUrl = null;
    return url;
  }
  
  /**
   * Load authentication state from local storage
   */
  private loadAuthStateFromStorage(): void {
    const token = localStorage.getItem(this.AUTH_TOKEN_KEY);
    if (!token) return;
    
    // Check if token is expired
    let tokenExpiration: Date | null = null;
    let isExpired = true;
    let tokenNeedsRefresh = false;
    
    try {
      const decodedToken = jwtDecode<{ exp: number }>(token);
      tokenExpiration = new Date(decodedToken.exp * 1000);
      const now = new Date();
      isExpired = now >= tokenExpiration;
      
      // Check if token needs to be refreshed (less than 5 minutes left)
      const fiveMinutesFromNow = new Date(now.getTime() + 5 * 60 * 1000);
      tokenNeedsRefresh = tokenExpiration < fiveMinutesFromNow;
      
    } catch (error) {
      console.error('Failed to decode token', error);
    }
    
    // If token is expired, don't restore the session
    if (isExpired) {
      localStorage.removeItem(this.AUTH_TOKEN_KEY);
      localStorage.removeItem(this.USER_DATA_KEY);
      return;
    }
    
    // If token needs refresh soon, trigger a refresh in the background
    if (tokenNeedsRefresh) {
      console.log('Token expires soon, triggering refresh');
      this.refreshToken().subscribe({
        next: () => console.log('Token refreshed successfully'),
        error: (error) => console.error('Failed to refresh token', error)
      });
    }
    
    // Get user data from storage
    const userJson = localStorage.getItem(this.USER_DATA_KEY);
    if (!userJson) return;
    
    try {
      const user = JSON.parse(userJson) as User;
      
      // Update state
      this._authState.set({
        isAuthenticated: true,
        user,
        token,
        tokenExpiration
      });
      
      // Update legacy status
      this.authStatusSource.next(true);
    } catch (error) {
      console.error('Failed to parse user data', error);
    }
  }
}
