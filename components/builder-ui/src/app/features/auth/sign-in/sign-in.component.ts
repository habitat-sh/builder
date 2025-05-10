import { Component, OnInit, OnDestroy, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute } from '@angular/router';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule, MatIconRegistry } from '@angular/material/icon';
import { MatDialogModule, MatDialog } from '@angular/material/dialog';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { Title, DomSanitizer } from '@angular/platform-browser';
import { environment } from '../../../../environments/environment';

import { AuthService } from '../../../core/services/auth.service';
import { ConfigService } from '../../../core/services/config.service';
import { EulaConfirmDialogComponent } from '../../../shared/dialogs/eula-confirm-dialog/eula-confirm-dialog.component';

@Component({
  selector: 'app-sign-in',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatIconModule, MatDialogModule, MatProgressSpinnerModule],
  templateUrl: './sign-in.component.html',
  styleUrls: ['./sign-in.component.scss']
})
export class SignInComponent implements OnInit, OnDestroy {
  private authService = inject(AuthService);
  private configService = inject(ConfigService);
  private titleService = inject(Title);
  private dialog = inject(MatDialog);
  private route = inject(ActivatedRoute);
  private iconRegistry = inject(MatIconRegistry);
  private domSanitizer = inject(DomSanitizer);
  
  // Provider information
  providerName = 'GitHub';  // Default to GitHub
  signupUrl = 'https://github.com/join';  // Default GitHub signup URL
  wwwUrl = 'https://www.habitat.sh';  // Default Habitat URL
  
  // Login URL for OAuth
  loginUrl = '';
  
  // Error message from OAuth callback
  errorMessage = '';
  
  ngOnInit() {
    // Set page title
    this.titleService.setTitle(`Sign In | Habitat Builder`);
    
    // Add the sign-in-page class to the body and app wrapper for proper styling
    document.body.classList.add('sign-in-page');
    
    // Find closest parent with an app class if it exists
    const appElement = document.querySelector('.app');
    if (appElement) {
      appElement.classList.add('sign-in');
      appElement.classList.remove('full');
    }
    
    // Register GitHub icon locally to ensure it's available
    this.iconRegistry.addSvgIcon(
      'github',
      this.domSanitizer.bypassSecurityTrustResourceUrl('assets/images/icons/github.svg')
    );
    
    // Sign out any existing session
    this.authService.logout(false);
    
    // Check for error message from OAuth callback
    this.route.queryParams.subscribe((params: Record<string, string>) => {
      if (params['error']) {
        this.errorMessage = params['error'];
      }
    });
    
    // Get OAuth configuration
    this.configService.getConfig().subscribe(config => {
      if (config) {
        this.providerName = config.oauthProvider || 'GitHub';
        this.signupUrl = config.oauthSignupUrl || this.signupUrl;
        this.wwwUrl = config.wwwUrl as string || this.wwwUrl;
        
        // Construct OAuth login URL
        this.loginUrl = this.authService.getAuthorizationUrl();
      }
    });
  }
  
  /**
   * Show the EULA confirmation dialog before proceeding with login
   */
  showEulaPopup() {
    // Check if EULA has already been accepted
    if (!localStorage.getItem('loginEulaAccept')) {
      this.dialog.open(EulaConfirmDialogComponent, {
        width: '530px',
        disableClose: true,
        data: {
          heading: 'End Users License Agreement',
          action: 'Continue',
          signupUrl: this.loginUrl
        }
      }).afterClosed().subscribe(result => {
        if (result) {
          localStorage.setItem('loginEulaAccept', 'true');
          localStorage.setItem('loginShowEulaPopup', 'false');
          this.redirectToOAuthProvider();
        }
      });
    } else {
      this.redirectToOAuthProvider();
    }
  }
  
  /**
   * Redirect to the OAuth provider's authorization endpoint
   */
  private redirectToOAuthProvider() {
    if (this.loginUrl) {
      // Use normal navigation instead of direct window location change
      // This allows our mock callback to work properly in development
      if (environment.useMocks && this.loginUrl.includes('localhost')) {
        console.log('Mock auth: Redirecting to mock OAuth provider');
        try {
          // Store the current time to track auth flow timing
          localStorage.setItem('auth_start_time', Date.now().toString());
          window.location.href = this.loginUrl;
        } catch (error) {
          console.error('Failed to redirect to mock OAuth provider:', error);
          this.errorMessage = 'Failed to start authentication process. Please try again.';
        }
      } else {
        // In production, redirect to the actual provider
        try {
          // Store the current time to track auth flow timing
          localStorage.setItem('auth_start_time', Date.now().toString());
          window.location.href = this.loginUrl;
        } catch (error) {
          console.error('Failed to redirect to OAuth provider:', error);
          this.errorMessage = 'Failed to start authentication process. Please try again.';
        }
      }
    } else {
      console.error('No login URL available for OAuth provider');
      this.errorMessage = 'Authentication configuration error. Please contact support.';
    }
  }
  
  /**
   * Clean up when component is destroyed
   */
  ngOnDestroy() {
    // Remove the sign-in-page class from the body when navigating away
    document.body.classList.remove('sign-in-page');
    
    // Clean up app classes if they exist
    const appElement = document.querySelector('.app');
    if (appElement) {
      appElement.classList.remove('sign-in');
      appElement.classList.add('full');
    }
  }
}
