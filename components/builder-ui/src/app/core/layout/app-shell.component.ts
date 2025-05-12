import { Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule, Router, NavigationEnd } from '@angular/router';
import { MatIconModule } from '@angular/material/icon';
import { filter, map } from 'rxjs/operators';

// Import header, sidebar and footer components with proper relative paths
import { HeaderComponent } from './header/header.component';
import { SidebarComponent, NavigationItem } from './sidebar/sidebar.component';
import { FooterComponent } from './footer/footer.component'; 
import { AuthService } from '../services/auth.service';
import { ConfigService } from '../services/config.service';

@Component({
  selector: 'app-shell',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatIconModule,
    HeaderComponent,
    SidebarComponent,
    FooterComponent
  ],
  template: `
    <div class="app-shell">
      <div class="wrapper">
        <app-sidebar 
          class="menu"
          [class.open]="menuOpen()"
          [isSignedIn]="isSignedIn()" 
          [navigationItems]="navigationItems"
          [enabledEvents]="eventsEnabled()"
          [enabledSaasEvents]="saasEventsEnabled()"
          [config]="configService"
          (closeMobileSidebar)="toggleMenu(false)">
        </app-sidebar>
        <main>
          <div class="menu-toggle" (click)="toggleMenu()">
            <span class="sr-only">Toggle menu</span>
            <mat-icon>menu</mat-icon>
          </div>
          @if (showHeader()) {
            <app-header 
              [username]="username()"
              [avatarUrl]="avatarUrl()"
              [isSignedIn]="isSignedIn()"
              [user]="{ name: username(), email: '', avatar: avatarUrl() }"
              (signOut)="handleSignOut()"
              (logout)="handleSignOut()">
            </app-header>
          }
          <div class="content-container">
            <router-outlet></router-outlet>
          </div>
          <app-footer></app-footer>
        </main>
      </div>
    </div>
  `,
  styleUrls: ['./app-shell.component.scss']
})
export class AppShellComponent implements OnInit {
  private authService = inject(AuthService);
  private router = inject(Router);
  public configService = inject(ConfigService);
  
  menuOpen = signal<boolean>(false);
  isSignedIn = signal<boolean>(false);
  username = signal<string>('');
  avatarUrl = signal<string>('');
  showHeader = signal<boolean>(true);
  
  // Navigation items to match the builder-web side-nav
  navigationItems: NavigationItem[] = [];
  
  private initNavigationItems() {
    const mainNavItems: NavigationItem[] = [];
    
    // Add Builder section header
    mainNavItems.push({ divider: true, label: 'Builder' });
    
    // Core navigation - only show My Origins if signed in
    if (this.isSignedIn()) {
      mainNavItems.push({
        label: 'My Origins',
        icon: 'group',
        route: '/origins'
      });
    }
    
    // Always show Search Packages
    mainNavItems.push({
      label: 'Search Packages',
      icon: 'search',
      route: '/pkgs'
    });
    
    // Add Events if enabled with new feature flag name
    if (this.configService.isFeatureEnabled('enable_builder_events')) {
      mainNavItems.push({
        label: 'Events',
        icon: 'event',
        route: '/events'
      });
      
      // Add SaaS Events if both flags enabled
      if (this.configService.isFeatureEnabled('enable_builder_events_saas')) {
        mainNavItems.push({
          label: 'Events (SaaS)',
          icon: 'cloud',
          route: '/events-saas'
        });
      }
    }
    
    // Add section divider for quick links
    mainNavItems.push({ divider: true, label: 'Quick Links' });
    
    // Quick links section - exactly matching the original side-nav.component.html
    const quickLinks: NavigationItem[] = [
      {
        label: 'Download Habitat',
        icon: 'file_download',
        route: this.configService.getUrl('download') || 'https://www.habitat.sh/docs/install-habitat/'
      },
      {
        label: 'Docs',
        icon: 'description',
        route: this.configService.getUrl('docs') || 'https://docs.chef.io/habitat/'
      },
      {
        label: 'Tutorials',
        icon: 'explore',
        route: this.configService.getUrl('tutorials') || 'https://learn.chef.io/habitat/'
      },
      {
        label: 'Blog',
        icon: 'rss_feed',
        route: this.configService.getUrl('blog') || 'https://www.habitat.sh/blog'
      },
      {
        label: 'Website',
        icon: 'language',
        route: this.configService.getUrl('website') || 'https://www.habitat.sh'
      },
      {
        label: 'GitHub',
        icon: 'code',
        route: this.configService.getUrl('sourceCode') || 'https://github.com/habitat-sh/habitat'
      }
    ];
    
    // Add Service Status section if config is SaaS
    if (this.configService.isFeatureEnabled('saas')) {
      mainNavItems.push({ divider: true, label: 'Service Status' });
      mainNavItems.push({
        label: 'Status',
        icon: 'info',
        route: 'https://status.chef.io/'
      });
    }
    
    // Append quick links to mainNavItems
    mainNavItems.push(...quickLinks);
    this.navigationItems = mainNavItems;
  }
  
  ngOnInit() {
    // Check for auth state during initialization
    this.checkAndUpdateAuthState();
    
    // Update navigation items with URLs from config
    this.updateNavigationUrls();
    
    // Initialize route listeners to hide header on landing page
    this.setupRouteListeners();
  }

  /**
   * Check current auth state and update UI accordingly
   * This ensures proper state when app loads or refreshes
   */
  private checkAndUpdateAuthState() {
    // Get the current authenticated state from service
    const isAuthenticated = this.authService.isAuthenticated();
    
    console.log('AppShell: Checking auth state on init:', { 
      isAuthenticated: isAuthenticated,
      userExists: this.authService.currentUser() !== null,
      user: this.authService.currentUser()
    });
    
    // Update our local state
    this.isSignedIn.set(isAuthenticated);
    
    // Update user information if authenticated
    if (isAuthenticated) {
      const user = this.authService.currentUser();
      if (user) {
        this.username.set(user.name);
        this.avatarUrl.set(user.avatar || 'assets/images/avatar.svg');
      }
    }
    
    // Subscribe to auth state changes using the legacy observable
    this.authService.authStatus$.subscribe(isAuth => {
      // Always check the signal value for the most accurate state
      const signalAuthenticated = this.authService.isAuthenticated();
      const currentUser = this.authService.currentUser();
      
      console.log('AppShell: Auth state changed:', { 
        eventIsAuth: isAuth,
        signalAuthenticated: signalAuthenticated,
        user: currentUser,
        username: currentUser?.name || null
      });
      
      // Use the more reliable signal value if available, otherwise use the event value
      const effectiveAuthState = signalAuthenticated;
      this.isSignedIn.set(effectiveAuthState);
      
      if (effectiveAuthState) {
        // If authenticated, set up user data in the UI
        if (currentUser) {
          this.username.set(currentUser.name);
          // Set user avatar with fallback
          this.avatarUrl.set(currentUser.avatar || 'assets/images/avatar.svg');
        }
        
        // Always show header when authenticated, regardless of route
        this.showHeader.set(true);
      } else {
        this.username.set('');
        this.avatarUrl.set('');
        
        // Always show header even when not authenticated
        // This provides consistent UI and ensures the sign-in button is accessible
        this.showHeader.set(true);
      }
      
      // Make sure navigation items are updated after auth state changes
      this.updateNavigationUrls();
    });
  }
  
  // Update navigation based on authentication state and config
  private updateNavigationUrls() {
    this.initNavigationItems();
  }
  
  toggleMenu(forcedState?: boolean) {
    const newState = forcedState !== undefined ? forcedState : !this.menuOpen();
    this.menuOpen.set(newState);
  }
  
  handleSignOut() {
    // Use the AuthService logout method
    this.authService.logout();
    // Ensure our local state is updated
    this.isSignedIn.set(false);
    this.username.set('');
    this.avatarUrl.set('');
  }
  
  // Helper methods for feature flags with updated flag names
  eventsEnabled(): boolean {
    return this.configService.isFeatureEnabled('enable_builder_events');
  }
  
  saasEventsEnabled(): boolean {
    return this.configService.isFeatureEnabled('enable_builder_events_saas');
  }
  
  // Setup route listeners to hide header on landing page when not logged in
  private setupRouteListeners() {
    // Check the initial route
    const currentUrl = this.router.url;
    const isHomePage = currentUrl === '/' || currentUrl === '' || currentUrl === '/home';
    
    // Always show header if authenticated, or hide on homepage when not authenticated
    // Initially set the header visibility
    this.showHeader.set(true);
    
    // Listen for route changes
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd),
      map((event: NavigationEnd) => event.url)
    ).subscribe(url => {
      // Always show header after authentication
      // The header contains the user profile dropdown needed for logout
      this.showHeader.set(true);
    });
  }
}
