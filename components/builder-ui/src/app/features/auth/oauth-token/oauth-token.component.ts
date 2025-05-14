import { Component, OnInit, inject } from '@angular/core';
import { RouterLink, Router, ActivatedRoute } from '@angular/router';
import { MatButtonModule } from '@angular/material/button';
import { MatCardModule } from '@angular/material/card';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { CommonModule } from '@angular/common';
import { AuthService } from '../../../core/services/auth.service';

@Component({
  selector: 'app-oauth-token',
  standalone: true,
  imports: [CommonModule, RouterLink, MatButtonModule, MatCardModule, MatProgressSpinnerModule],
  template: `
    <div class="oauth-token-container">
      <mat-card class="oauth-token-card">
        <mat-card-header>
          <mat-card-title>OAuth Authentication</mat-card-title>
          <mat-card-subtitle>Processing authentication token</mat-card-subtitle>
        </mat-card-header>
        <mat-card-content>
          <div *ngIf="isLoading" class="loading">
            <mat-spinner diameter="40"></mat-spinner>
            <p>Processing your authentication...</p>
          </div>
          <div *ngIf="error" class="error">
            <p>{{ error }}</p>
          </div>
          <div *ngIf="success" class="success">
            <p>Successfully authenticated!</p>
          </div>
        </mat-card-content>
        <mat-card-actions>
          <button *ngIf="!isLoading" mat-raised-button color="primary" routerLink="/">Go to Home</button>
          <button *ngIf="error" mat-stroked-button routerLink="/sign-in">Try Again</button>
        </mat-card-actions>
      </mat-card>
    </div>
  `,
  styles: [`
    .oauth-token-container {
      display: flex;
      justify-content: center;
      align-items: center;
      height: 100vh;
      background-color: #f5f5f5;
    }
    
    .oauth-token-card {
      width: 100%;
      max-width: 400px;
      padding: 20px;
    }
    
    .loading, .error, .success {
      margin: 20px 0;
      text-align: center;
    }
    
    .loading p {
      margin-top: 16px;
    }
    
    .error {
      color: #d32f2f;
    }
    
    .success {
      color: #2e7d32;
    }
    
    mat-card-actions {
      display: flex;
      justify-content: center;
    }
  `]
})
export class OAuthTokenComponent implements OnInit {
  private authService = inject(AuthService);
  private router = inject(Router);
  private route = inject(ActivatedRoute);
  
  isLoading = true;
  error = '';
  success = false;
  private maxRetries = 3;
  private retryCount = 0;
  private retryDelay = 500; // ms
  
  ngOnInit(): void {
    // Get token from URL hash fragment or query param
    const params = new URLSearchParams(window.location.hash.substring(1) || window.location.search.substring(1));
    const token = params.get('token');
    
    if (!token) {
      this.isLoading = false;
      this.error = 'No authentication token provided';
      return;
    }
    
    // Process the token
    this.authService.handleOAuthCallback(token).subscribe({
      next: () => {
        this.isLoading = false;
        this.success = true;
        
        // Clear any URL fragments or params that contain auth tokens to prevent security issues
        if (window.history && window.history.replaceState) {
          window.history.replaceState({}, document.title, window.location.pathname);
        }
        
        // Store authentication timestamp and flags in sessionStorage for better refresh handling
        const timestamp = Date.now().toString();
        sessionStorage.setItem('auth_timestamp', timestamp);
        sessionStorage.setItem('auth_success', 'true');
        sessionStorage.setItem('auth_just_completed', 'true');
        
        // Also add a broadcast channel if supported to notify other tabs
        this.broadcastAuthSuccess();
        
        // Ensure the auth state is fully recognized across the application
        console.log('OAuth token: Authentication successful, verifying app state');
        
        // Manually validate the auth state to ensure it's properly loaded
        // Do this multiple times to make sure it propagates
        for (let i = 0; i < 3; i++) {
          if (typeof this.authService.validateAuthState === 'function') {
            this.authService.validateAuthState();
          }
        }
        
        // Dispatch a custom auth success event for components to listen for
        try {
          // Try to get user data from authService or localStorage
          const userJson = localStorage.getItem('user_data');
          let user = this.authService.currentUser();
          
          if (!user && userJson) {
            try {
              user = JSON.parse(userJson);
            } catch (err) {
              console.warn('OAuth token: Failed to parse user data', err);
            }
          }
          
          if (user) {
            const event = new CustomEvent('habitat-auth-success', { 
              detail: { user, timestamp } 
            });
            document.dispatchEvent(event);
            console.log('OAuth token: Dispatched custom auth success event');
          }
        } catch (err) {
          console.warn('OAuth token: Error dispatching auth event', err);
        }
        
        // Allow a small delay before starting auth checks to give the app time to process the token
        console.log('OAuth token: Adding initial delay before starting auth state checks');
        setTimeout(() => {
          // Use a more robust approach to ensure auth state is fully propagated
          this.waitForAuthStateAndRedirect();
        }, 800);
      },
      error: (error) => {
        this.isLoading = false;
        this.error = 'Authentication failed: ' + (error.message || 'Unknown error');
        console.error('Token processing error', error);
        
        // Store error information for diagnostics
        sessionStorage.setItem('auth_error', JSON.stringify({
          message: error.message || 'Unknown error',
          timestamp: new Date().toISOString(),
          location: 'oauth_token_component'
        }));
        
        // Clear any partial authentication state to prevent UI breaking
        localStorage.removeItem('auth_token');
        localStorage.removeItem('user_data');
        sessionStorage.removeItem('auth_success');
      }
    });
  }

  /**
   * Use BroadcastChannel API to notify other tabs about auth success if supported
   */
  private broadcastAuthSuccess(): void {
    try {
      if ('BroadcastChannel' in window) {
        const authChannel = new BroadcastChannel('auth_channel');
        authChannel.postMessage({ event: 'auth_success', timestamp: Date.now() });
        console.log('OAuth token: Broadcasted auth success to other tabs');
        
        // Close the channel after sending
        setTimeout(() => authChannel.close(), 1000);
      }
    } catch (err) {
      console.warn('OAuth token: BroadcastChannel not supported or error occurred', err);
    }
  }

  /**
   * Wait for auth state to be fully propagated with retry mechanism
   */
  private waitForAuthStateAndRedirect(): void {
    // First check - is auth state already ready?
    if (this.authService.isAuthenticated()) {
      console.log('OAuth token: Auth state is already set, proceeding with redirect');
      this.redirectToTarget();
      return;
    }

    // Use a more robust approach with retries
    this.retryAuthStateCheck();
  }

  /**
   * Retry checking auth state until it's ready or max retries reached
   */
  private retryAuthStateCheck(): void {
    // Force a validation of auth state
    if (typeof this.authService.validateAuthState === 'function') {
      this.authService.validateAuthState();
    }
    
    // Check if now authenticated
    if (this.authService.isAuthenticated()) {
      console.log(`OAuth token: Auth state ready after ${this.retryCount} retries, proceeding with redirect`);
      this.redirectToTarget();
      return;
    }
    
    // If still not authenticated, retry if under max retries
    this.retryCount++;
    if (this.retryCount < this.maxRetries) {
      console.log(`OAuth token: Auth state not ready, retry ${this.retryCount}/${this.maxRetries} in ${this.retryDelay}ms`);
      setTimeout(() => this.retryAuthStateCheck(), this.retryDelay);
      // Increase delay for next retry (exponential backoff)
      this.retryDelay *= 1.5;
    } else {
      // Last attempt with longer timeout before giving up
      console.log(`OAuth token: Final auth state check before forced reload`);
      setTimeout(() => {
        // Force one last validation attempt
        if (typeof this.authService.validateAuthState === 'function') {
          this.authService.validateAuthState();
        }
        
        // No need to check auth state again - just force a reload to root path
        // This is the most reliable solution for ensuring auth state is properly initialized
        console.log('OAuth token: Forcing page reload to root path for reliable auth state initialization');
        
        // Set additional flags in sessionStorage to help diagnose the issue
        sessionStorage.setItem('auth_forced_reload', 'true');
        sessionStorage.setItem('auth_debug_info', JSON.stringify({
          timestamp: new Date().toISOString(),
          isAuthenticated: this.authService.isAuthenticated(),
          retryCount: this.retryCount,
          authTokenExists: !!localStorage.getItem('auth_token'),
          userDataExists: !!localStorage.getItem('user_data')
        }));
        
        // Force navigation to root with hard reload
        window.location.href = '/';
      }, 2000);
    }
  }

  /**
   * Redirect to the target URL after successful authentication
   */
  private redirectToTarget(): void {
    // Always use root path and ignore any stored redirectUrl
    const targetUrl = '/';
    console.log(`OAuth token: Redirecting to root path with hard reload, auth state: ${this.authService.isAuthenticated()}`);
    
    // Record final auth state for diagnostics
    sessionStorage.setItem('auth_redirect_state', JSON.stringify({
      isAuthenticated: this.authService.isAuthenticated(),
      timestamp: new Date().toISOString(),
      retryCount: this.retryCount,
      targetUrl: targetUrl,
      originalRedirectUrl: this.authService.getAndClearRedirectUrl() || '/'
    }));
    
    // Store a special flag that will be checked by app-shell
    sessionStorage.setItem('auth_just_completed', 'true');
    sessionStorage.setItem('auth_completed_at', Date.now().toString());
    
    // Ensure that the app will see auth state on the next page load
    // This is a fallback in case the service's internal state isn't properly updated
    try {
      // Try to directly update the DOM to show success while we're still on this page
      const userDataStr = localStorage.getItem('user_data');
      if (userDataStr) {
        try {
          const userData = JSON.parse(userDataStr);
          const event = new CustomEvent('habitat-auth-success', { 
            detail: { user: userData, timestamp: Date.now() } 
          });
          document.dispatchEvent(event);
          console.log('OAuth token: Dispatched custom auth success event');
        } catch (err) {
          console.warn('OAuth token: Failed to parse user data', err);
        }
      }
    } catch (err) {
      console.warn('OAuth token: Error preparing auth state', err);
    }
    
    // Force a browser location change with hard refresh
    // This is the most reliable way to ensure proper auth state initialization
    window.location.href = targetUrl;
  }
}
