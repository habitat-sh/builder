import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router } from '@angular/router';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

import { MockAuthService } from '../../../core/mocks/mock-auth.service';
import { environment } from '../../../../environments/environment';

@Component({
  selector: 'app-mock-oauth-callback',
  standalone: true,
  imports: [CommonModule, MatProgressSpinnerModule],
  template: `
    <div class="mock-callback-container">
      <mat-spinner diameter="40"></mat-spinner>
      <h2>Processing mock authentication...</h2>
      <p class="info">This is a simulated OAuth callback for development.</p>
      <p class="progress" *ngIf="isProcessing">Processing authentication code...</p>
      <p class="error" *ngIf="errorMessage">{{ errorMessage }}</p>
    </div>
  `,
  styles: [`
    .mock-callback-container {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100vh;
      text-align: center;
      padding: 0 20px;
      background-color: #f5f8fa;
    }
    
    h2 {
      margin-top: 20px;
      color: #333;
    }
    
    .info {
      color: #666;
      margin-top: 10px;
    }
    
    .progress {
      color: #4296b4;
      margin-top: 10px;
    }
    
    .error {
      color: #e85600;
      margin-top: 10px;
      font-weight: bold;
    }
  `]
})
export class MockOAuthCallbackComponent implements OnInit {
  private route = inject(ActivatedRoute);
  private router = inject(Router);
  private mockAuthService = inject(MockAuthService);
  
  isProcessing = true;
  errorMessage = '';
  
  ngOnInit(): void {
    // Only use this component in non-production environments
    if (environment.production) {
      this.router.navigate(['/sign-in']);
      return;
    }
    
    // Extract state and mock code from query parameters
    this.route.queryParams.subscribe(params => {
      const state = params['state'];
      
      // Generate a mock code
      const code = 'mock_code_' + Math.random().toString(36).substring(2);
      
      console.log('MockOAuthCallbackComponent: Starting authentication with state:', state);
      
      // Handle the mock callback
      this.mockAuthService.handleCallback(state, code).subscribe({
        next: (response: any) => {
          if (response.error) {
            this.isProcessing = false;
            this.errorMessage = 'Authentication failed: ' + response.error;
            console.error('MockOAuthCallbackComponent: Authentication error:', response.error);
            
            // Redirect back to sign-in after error
            setTimeout(() => {
              this.router.navigate(['/sign-in'], { 
                queryParams: { error: this.errorMessage } 
              });
            }, 2000);
            
            return;
          }
          
          // Successful authentication
          this.isProcessing = false;
          console.log('MockOAuthCallbackComponent: Authentication successful', {
            authState: {
              isAuthenticated: this.mockAuthService.isAuthenticated(),
              user: this.mockAuthService.currentUser()
            },
            response
          });
          
          // Store token in localStorage for debugging/inspection
          localStorage.setItem('builder_auth_token', response.token);
          
          // Store authentication timing for analytics
          const authStartTime = localStorage.getItem('auth_start_time');
          if (authStartTime) {
            const authDuration = Date.now() - parseInt(authStartTime, 10);
            console.log(`MockOAuthCallbackComponent: Authentication completed in ${authDuration}ms`);
            localStorage.removeItem('auth_start_time');
          }
          
          // Get the redirect URL from auth service or default to home page
          const redirectUrl = this.mockAuthService.getAndClearRedirectUrl() || '/home';
          console.log('MockOAuthCallbackComponent: Redirecting to', redirectUrl);
          
          // Redirect to the stored URL or home
          console.log('MockOAuthCallbackComponent: Preparing to navigate to', redirectUrl);
          
          // Make sure the authentication state is fully updated before redirecting
          // This ensures any components that depend on the auth state will be properly updated
          setTimeout(() => {
            console.log('MockOAuthCallbackComponent: Navigating to', redirectUrl, 
              'AuthState:', {
                isAuthenticated: this.mockAuthService.isAuthenticated(),
                user: this.mockAuthService.currentUser()
              }
            );
            this.router.navigateByUrl(redirectUrl);
          }, 1000);
        },
        error: (error) => {
          this.isProcessing = false;
          this.errorMessage = 'Authentication failed: ' + (error.message || 'Unknown error');
          
          // Redirect back to sign-in after error
          setTimeout(() => {
            this.router.navigate(['/sign-in'], { 
              queryParams: { error: this.errorMessage } 
            });
          }, 2000);
        }
      });
    });
  }
}
