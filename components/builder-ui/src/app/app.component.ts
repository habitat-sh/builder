import { Component, OnInit, Renderer2, inject } from '@angular/core';
import { RouterOutlet, Router, Event, NavigationEnd } from '@angular/router';
import { Meta, Title } from '@angular/platform-browser';
import { IconService } from './core/services/icon.service';
import { AuthService } from './core/services/auth.service';
import { filter } from 'rxjs/operators';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [RouterOutlet, CommonModule],
  template: `
    <div [ngClass]="{'app': true, 'sign-in': isSignInRoute, 'full': !isSignInRoute}" [style.height]="isSignInRoute ? '100vh' : '100vh'" [style.overflow]="isSignInRoute ? 'hidden' : 'auto'">
      <router-outlet></router-outlet>
    </div>
  `,
  styles: []
})
export class AppComponent implements OnInit {
  title = 'Habitat Builder';
  isSignInRoute = false;
  private authService = inject(AuthService);

  constructor(
    private meta: Meta,
    private titleService: Title,
    private iconService: IconService,
    private router: Router,
    private renderer: Renderer2
  ) {}

  ngOnInit() {
    // Check if we're coming from a forced reload due to auth issues
    const authForcedReload = sessionStorage.getItem('auth_forced_reload');
    if (authForcedReload === 'true') {
      console.log('App: Detected forced reload for auth, special handling activated');
      // Clear the flag so we don't loop
      sessionStorage.removeItem('auth_forced_reload');
      
      // Give priority to auth validation
      setTimeout(() => this.validateAndFixAuthState(), 100);
    }
    
    // Normal app initialization
    this.initializeApp();
  }
  
  /**
   * Dedicated method for auth state validation with enhanced error recovery
   */
  private validateAndFixAuthState(): void {
    console.log('App: Running enhanced auth state validation');
    
    try {
      if (typeof this.authService.validateAuthState === 'function') {
        this.authService.validateAuthState();
        
        // Double-check auth state after validation
        if (typeof this.authService.isAuthenticated === 'function') {
          const isAuth = this.authService.isAuthenticated();
          console.log('App: Post-validation auth state:', isAuth);
          
          // If we have auth_success in session but still not authenticated, try one more approach
          if (!isAuth && sessionStorage.getItem('auth_success') === 'true') {
            console.log('App: Auth inconsistency detected, attempting to restore state from storage');
            // Trigger any methods that might help restore auth state
            if (typeof this.authService['loadAuthStateFromStorage'] === 'function') {
              this.authService['loadAuthStateFromStorage']();
            }
          }
        }
      }
    } catch (error) {
      console.error('App: Error during enhanced auth validation:', error);
    }
  }
  
  /**
   * Initialize the application
   */
  private initializeApp(): void {
    // Ensure authentication state is loaded and validated
    console.log('App: Initializing and validating authentication state');
    try {
      if (typeof this.authService.validateAuthState === 'function') {
        this.authService.validateAuthState();
      } else {
        console.log('App: validateAuthState method not available, using alternative approach');
        // We'll rely on isAuthenticated which should work
      }
    } catch (error) {
      console.error('App: Error validating auth state:', error);
    }
    
    // Safely log authentication status
    if (typeof this.authService.isAuthenticated === 'function') {
      console.log('App: Authentication status:', this.authService.isAuthenticated());
    } else {
      console.log('App: Unable to determine authentication status - isAuthenticated method not available');
    }

    // Set document title
    this.titleService.setTitle(this.title);
    
    // Add theme-color meta tag for browsers
    this.meta.addTag({ name: 'theme-color', content: '#3292bf' });
    
    // Add description meta tag
    this.meta.addTag({ 
      name: 'description', 
      content: 'Habitat Builder: A platform for building, deploying, and managing applications with native integration to Chef Habitat.'
    });
    
    // Register custom SVG icons
    this.iconService.registerIcons();
    
    // Listen for route changes to update body classes
    this.router.events.pipe(
      filter((event: Event): event is NavigationEnd => event instanceof NavigationEnd)
    ).subscribe((event: NavigationEnd) => {
      // Check if it's a sign-in route
      this.isSignInRoute = event.url.includes('/sign-in');
      
      // Add or remove classes for app structure and body
      if (this.isSignInRoute) {
        this.renderer.addClass(document.body, 'sign-in-page');
        
        // Remove any previous scrolling position
        window.scrollTo(0, 0);
        
        // Remove any overflow restrictions
        this.renderer.removeClass(document.body, 'no-scroll');
        
        // Ensure proper overflow handling
        this.renderer.setStyle(document.body, 'overflow-x', 'hidden');
      } else {
        this.renderer.removeClass(document.body, 'sign-in-page');
      }
    });
  }
}
