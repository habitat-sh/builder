import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router } from '@angular/router';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { AuthService } from '../../../core/services/auth.service';

@Component({
  selector: 'app-oauth-callback',
  standalone: true,
  imports: [CommonModule, MatProgressSpinnerModule],
  template: `
    <div class="oauth-callback-container">
      <mat-spinner diameter="40"></mat-spinner>
      <h2>Processing authentication...</h2>
      <p>Please wait while we complete your sign-in.</p>
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
    }
    
    h2 {
      margin-top: 20px;
      color: #333;
    }
    
    p {
      color: #666;
      margin-top: 10px;
    }
  `]
})
export class OAuthCallbackComponent implements OnInit {
  private route = inject(ActivatedRoute);
  private router = inject(Router);
  private authService = inject(AuthService);
  
  private errorMessages: Record<string, string> = {
    'access_denied': 'Access was denied by the authentication provider.',
    'invalid_request': 'The authentication request was invalid.',
    'invalid_state': 'Security verification failed. Please try again.',
    'server_error': 'The server encountered an error. Please try again.'
  };
  
  ngOnInit(): void {
    // Extract parameters from the URL
    this.route.queryParams.subscribe(params => {
      // Check for error from OAuth provider
      if (params['error']) {
        const errorMessage = this.errorMessages[params['error']] || 
          'An error occurred during authentication. Please try again.';
        
        // Handle error - redirect to login with error message
        this.router.navigate(['/sign-in'], { 
          queryParams: { error: errorMessage }
        });
        return;
      }
      
      // Get authorization code and state from URL
      const code = params['code'];
      const state = params['state'];
      
      // Verify state parameter against stored state (prevents CSRF)
      const storedState = localStorage.getItem('oauth_state');
      if (!state || state !== storedState) {
        this.router.navigate(['/sign-in'], { 
          queryParams: { error: 'Security verification failed. Please try again.' }
        });
        return;
      }
      
      // Clear stored state
      localStorage.removeItem('oauth_state');
      
      // Exchange code for access token
      if (code) {
        this.authService.exchangeCodeForToken(code).subscribe({
          next: () => {
            // Get user's redirect URL if any
            const redirectUrl = this.authService.getAndClearRedirectUrl() || '/';
            this.router.navigateByUrl(redirectUrl);
          },
          error: (error) => {
            console.error('Token exchange error', error);
            this.router.navigate(['/sign-in'], {
              queryParams: { error: 'Authentication failed. Please try again.' }
            });
          }
        });
      } else {
        // No code provided, redirect to login
        this.router.navigate(['/sign-in']);
      }
    });
  }
}
