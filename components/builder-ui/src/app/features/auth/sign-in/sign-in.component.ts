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
import { HabitatConfigService } from '../../../core/services/habitat-config.service';
import { MockAuthService } from '../../../core/mocks/mock-auth.service';
import { EulaConfirmDialogComponent } from '../../../shared/dialogs/eula-confirm-dialog/eula-confirm-dialog.component';

@Component({
  selector: 'app-sign-in',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatIconModule, MatDialogModule, MatProgressSpinnerModule],
  templateUrl: './sign-in.component.html',
  styleUrls: ['./sign-in.component.scss']
})
export class SignInComponent implements OnInit, OnDestroy {
  private realAuthService = inject(AuthService);
  private mockAuthService = inject(MockAuthService);
  private configService = inject(ConfigService);
  private habitatConfig = inject(HabitatConfigService);
  private titleService = inject(Title);
  private dialog = inject(MatDialog);
  private route = inject(ActivatedRoute);
  private iconRegistry = inject(MatIconRegistry);
  private domSanitizer = inject(DomSanitizer);
  
  // Use the appropriate auth service based on environment
  private get authService() {
    return environment.useMocks ? this.mockAuthService : this.realAuthService;
  }
  
  // Provider information
  providerName = 'GitHub';  // Default to GitHub
  providerIcon = 'github';  // Default to GitHub icon
  signupUrl = 'https://github.com/join';  // Default GitHub signup URL
  wwwUrl = 'https://www.habitat.sh';  // Default Habitat URL
  
  // Login URL for OAuth
  loginUrl = '';
  
  // Error message from OAuth callback
  errorMessage = '';
  
  // Flag to track if configuration is loaded from file (vs defaults)
  configLoadedProperly = true;
  
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
    
    // Register provider icons - we'll register both since we might need either
    this.registerProviderIcons();
    
    // Only sign out if explicitly navigated to sign-in page
    // Don't sign out if being redirected from an OAuth flow
    if (!this.route.snapshot.queryParams['code'] && !this.route.snapshot.queryParams['error']) {
      // Record start time for auth flow analytics
      localStorage.setItem('auth_start_time', Date.now().toString());
      // Sign out any existing session but don't redirect
      this.authService.logout(false);
    }
    
    // Check for error message from OAuth callback
    this.route.queryParams.subscribe((params: Record<string, string>) => {
      if (params['error']) {
        this.errorMessage = params['error'];
      }
    });
    
    // Get OAuth configuration from Habitat Config
    const config = this.habitatConfig.config;
    
    // Get the configuration loading status from the service directly
    this.configLoadedProperly = this.habitatConfig.isLoadedFromFile;
    
    // Set provider information from habitat config
    const provider = config.oauth_provider || 'github';
    this.providerName = provider === 'github' ? 'GitHub' : 
                       provider === 'bitbucket' ? 'Bitbucket' : 
                       'GitHub';
    this.providerIcon = provider;
    this.signupUrl = config.oauth_signup_url || this.signupUrl;
    this.wwwUrl = config.www_url || this.wwwUrl;
    
    // Construct OAuth login URL - authService getter will return the appropriate service
    this.loginUrl = this.authService.getAuthorizationUrl();
    
    if (environment.useMocks) {
      console.log('Using mock authentication service');
    }
    
    // Log configuration usage for debugging
    console.log('Using Habitat configuration:', {
      provider: this.providerName,
      icon: this.providerIcon,
      wwwUrl: this.wwwUrl,
      loadedFromFile: this.configLoadedProperly
    });
  }
  
  /**
   * Register provider icons
   */
  private registerProviderIcons() {
    // Register GitHub icon
    this.iconRegistry.addSvgIcon(
      'github',
      this.domSanitizer.bypassSecurityTrustResourceUrl('assets/images/icons/github.svg')
    );
    
    // Register Bitbucket icon if we have it
    try {
      this.iconRegistry.addSvgIcon(
        'bitbucket',
        this.domSanitizer.bypassSecurityTrustResourceUrl('assets/images/icons/bitbucket.svg')
      );
    } catch (e) {
      console.warn('Bitbucket icon not available, will fallback to GitHub icon if needed');
    }
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
      try {
        // Store the current time to track auth flow timing
        localStorage.setItem('auth_start_time', Date.now().toString());
        
        // Store any additional parameters from the query string that need to be preserved
        const params = this.route.snapshot.queryParams;
        if (params['returnUrl']) {
          // Store returnUrl for post-authentication redirection
          this.authService.setRedirectUrl(params['returnUrl']);
        }
        
        // Use the login URL (which will be from the appropriate service based on environment)
        if (environment.useMocks) {
          console.log('Mock auth: Redirecting to mock OAuth provider');
        }
        
        window.location.href = this.loginUrl;
      } catch (error) {
        console.error('Failed to redirect to OAuth provider:', error);
        this.errorMessage = 'Failed to start authentication process. Please try again.';
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
