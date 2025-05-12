import { Component, OnInit, OnDestroy, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router } from '@angular/router';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { Subscription, timer } from 'rxjs';
import { take } from 'rxjs/operators';

import { AuthService } from '../../../core/services/auth.service';

@Component({
  selector: 'app-oauth-callback',
  standalone: true,
  imports: [CommonModule, MatProgressSpinnerModule, MatButtonModule, MatIconModule],
  template: `
    <div class="oauth-callback-container">
      @if (loading()) {
        <mat-spinner diameter="40"></mat-spinner>
        <h2>Processing authentication...</h2>
        <p>Please wait while we complete your sign-in.</p>
        @if (retryCount() > 0) {
          <p class="retry-message">Retry attempt {{ retryCount() }} of 3...</p>
        }
      } @else if (error()) {
        <div class="error-container">
          <mat-icon class="error-icon">error_outline</mat-icon>
          <h2>Authentication Error</h2>
          <p>{{ error() }}</p>
          <button mat-raised-button color="primary" (click)="retryAuthentication()">
            Try Again
          </button>
          <button mat-button (click)="navigateToSignIn()">
            Back to Sign In
          </button>
        </div>
      }
    </div>
  `,
  styles: [`
    .oauth-callback-container {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100vh;
      text-align: center;
      padding: 0 20px;
      max-width: 500px;
      margin: 0 auto;
    }
    
    h2 {
      margin-top: 20px;
      color: #333;
    }
    
    p {
      color: #666;
      margin-top: 10px;
    }
    
    .retry-message {
      color: #ff9800;
      margin-top: 10px;
      font-style: italic;
    }
    
    .error-container {
      display: flex;
      flex-direction: column;
      align-items: center;
      background: #fff;
      padding: 30px;
      border-radius: 8px;
      box-shadow: 0 2px 10px rgba(0,0,0,0.1);
    }
    
    .error-icon {
      font-size: 48px;
      height: 48px;
      width: 48px;
      color: #f44336;
      margin-bottom: 16px;
    }
    
    button {
      margin-top: 20px;
    }
    
    button + button {
      margin-top: 10px;
    }
  `]
})
export class OAuthCallbackComponent implements OnInit, OnDestroy {
  private route = inject(ActivatedRoute);
  private router = inject(Router);
  private authService = inject(AuthService);
  
  // State signals and variables
  loading = signal(true);
  error = signal<string | null>(null);
  retryCount = signal(0);
  private maxRetries = 3;
  private originalCode: string | null = null;
  private originalState: string | null = null;
  private subscriptions = new Subscription();

  private errorMessages: Record<string, string> = {
    'access_denied': 'Access was denied by the authentication provider.',
    'invalid_request': 'The authentication request was invalid.',
    'invalid_state': 'Security verification failed. Please try again.',
    'server_error': 'The server encountered an error. Please try again.',
    'unauthorized_client': 'This application is not authorized to use this authentication method.',
    'unsupported_response_type': 'The authentication flow is not supported.',
    'bad_verification_code': 'The authorization code has expired or is invalid.',
    'incorrect_client_credentials': 'The OAuth client credentials are incorrect.'
  };
  
  ngOnInit(): void {
    // Extract parameters from the URL
    this.route.queryParams.subscribe(params => {
      // Check for error from OAuth provider
      if (params['error']) {
        const errorCode = params['error'];
        const errorMessage = this.errorMessages[errorCode] || 
          'An error occurred during authentication. Please try again.';
        
        console.error('OAuth error:', errorCode, params['error_description'] || '');
        this.loading.set(false);
        this.error.set(errorMessage);
        return;
      }
      
      // Get authorization code and state from URL
      const code = params['code'];
      const state = params['state'];
      
      // Save original values for potential retries
      this.originalCode = code;
      this.originalState = state;
      
      // Process the authentication
      this.processAuthentication(code, state);
    });
  }
  
  /**
   * Process the authentication code
   */
  private processAuthentication(code: string | null, state: string | null): void {
    // Reset error state and set loading
    this.error.set(null);
    this.loading.set(true);
    
    // Verify state parameter against stored state (prevents CSRF)
    const storedState = localStorage.getItem('oauth_state');
    if (!state || state !== storedState) {
      this.loading.set(false);
      this.error.set('Security verification failed. Please try again.');
      return;
    }
    
    // Clear stored state if it matches
    localStorage.removeItem('oauth_state');
    
    // Exchange code for access token
    if (code) {
      // Log authentication attempt
      console.log('OAuthCallbackComponent: Exchanging code for token');
      
      this.subscriptions.add(
        this.authService.exchangeCodeForToken(code).subscribe({
          next: (response) => {
            console.log('OAuthCallbackComponent: Authentication successful');
            
            // Get user's redirect URL if any
            const redirectUrl = this.authService.getAndClearRedirectUrl() || '/home';
            
            // Store authentication timing for analytics
            const authStartTime = localStorage.getItem('auth_start_time');
            if (authStartTime) {
              const authDuration = Date.now() - parseInt(authStartTime, 10);
              console.log(`OAuthCallbackComponent: Authentication completed in ${authDuration}ms`);
              localStorage.removeItem('auth_start_time');
            }
            
            // Store authentication timestamp to keep track of the session
            localStorage.setItem('auth_timestamp', Date.now().toString());
            sessionStorage.setItem('auth_timestamp', Date.now().toString());
            
            // Set flag for successful auth to help with UI state on refresh 
            sessionStorage.setItem('auth_success', 'true');
            
            // Add a small delay to ensure auth state is properly updated
            setTimeout(() => {
              // Navigate to the target URL
              console.log('OAuthCallbackComponent: Navigating to', redirectUrl);
              this.router.navigateByUrl(redirectUrl);
            }, 300);
          },
          error: (error) => {
            console.error('Token exchange error', error);
            
            // Store details about the error for debugging
            localStorage.setItem('auth_error', JSON.stringify({
              message: error.message || 'Unknown error',
              timestamp: new Date().toISOString(),
              retryCount: this.retryCount(),
              statusCode: error.status || 'unknown'
            }));
            
            // Check if we should retry (for network or server errors)
            if (this.retryCount() < this.maxRetries && this.isRetryableError(error)) {
              this.retryAuthentication();
            } else {
              // Max retries reached or non-retryable error
              this.loading.set(false);
              this.error.set('Authentication failed. The server might be temporarily unavailable. Please try again.');
            }
          }
        })
      );
    } else {
      // No code provided, show error
      console.error('OAuthCallbackComponent: No authorization code provided');
      this.loading.set(false);
      this.error.set('No authorization code provided. Please try again.');
    }
  }
  
  /**
   * Retry authentication with exponential backoff
   */
  retryAuthentication(): void {
    // Increment retry counter
    const currentRetry = this.retryCount() + 1;
    this.retryCount.set(currentRetry);
    
    if (currentRetry <= this.maxRetries) {
      console.log(`OAuthCallbackComponent: Retry attempt ${currentRetry} of ${this.maxRetries}`);
      
      // Calculate backoff delay using exponential backoff strategy
      const delayMs = Math.min(1000 * Math.pow(2, currentRetry - 1), 8000);
      
      // Reset error and set loading state
      this.error.set(null);
      this.loading.set(true);
      
      // Wait for backoff delay before retrying
      this.subscriptions.add(
        timer(delayMs).pipe(take(1)).subscribe(() => {
          // Retry with the original code and state
          if (this.originalCode) {
            this.processAuthentication(this.originalCode, this.originalState);
          } else {
            // If no original code, redirect to login
            this.navigateToSignIn();
          }
        })
      );
    } else {
      // Max retries reached
      this.navigateToSignIn();
    }
  }
  
  /**
   * Navigate back to sign-in page
   */
  navigateToSignIn(): void {
    this.router.navigate(['/sign-in']);
  }
  
  /**
   * Determine if an error is retryable (network or temporary server errors)
   */
  private isRetryableError(error: any): boolean {
    // Network errors or server errors (500s) are candidates for retry
    if (!navigator.onLine) return true;
    if (!error.status) return true; // Network error
    
    // Server errors (500-599) are generally retryable
    if (error.status >= 500 && error.status <= 599) return true;
    
    // Other specific cases that might benefit from retry
    if (error.status === 429) return true; // Too many requests
    if (error.status === 408) return true; // Request timeout
    
    // Non-retryable error types
    return false;
  }
  
  ngOnDestroy(): void {
    this.subscriptions.unsubscribe();
  }
}
