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
   */
  logout(redirect = true): void {
    // Clear the auth state
    this._authState.set({
      isAuthenticated: false,
      user: null,
      token: null,
      tokenExpiration: null
    });
    
    // Update legacy status
    this.authStatusSource.next(false);
    
    // Clear storage
    localStorage.removeItem(this.AUTH_TOKEN_KEY);
    localStorage.removeItem(this.USER_DATA_KEY);
    
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
   * Check if the user has a specific permission
   */
  hasPermission(permission: string): boolean {
    const user = this._authState().user;
    return !!user?.permissions?.includes(permission);
  }
  
  /**
   * Check if the current user has any of the specified roles
   */
  hasRole(roles: string | string[]): boolean {
    const user = this._authState().user;
    if (!user || !user.role) return false;
    
    const rolesToCheck = Array.isArray(roles) ? roles : [roles];
    return rolesToCheck.includes(user.role);
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
    
    try {
      const decodedToken = jwtDecode<{ exp: number }>(token);
      tokenExpiration = new Date(decodedToken.exp * 1000);
      isExpired = new Date() >= tokenExpiration;
    } catch (error) {
      console.error('Failed to decode token', error);
    }
    
    // If token is expired, don't restore the session
    if (isExpired) {
      localStorage.removeItem(this.AUTH_TOKEN_KEY);
      localStorage.removeItem(this.USER_DATA_KEY);
      return;
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
