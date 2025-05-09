import { Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { MatIconModule } from '@angular/material/icon';

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
          <app-header 
            [isSignedIn]="isSignedIn()" 
            [username]="username()"
            [avatarUrl]="avatarUrl()"
            [user]="{ name: username(), email: '', avatar: avatarUrl() }"
            (signOut)="handleSignOut()">
          </app-header>
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
  public configService = inject(ConfigService);
  
  menuOpen = signal<boolean>(false);
  isSignedIn = signal<boolean>(false);
  username = signal<string>('');
  avatarUrl = signal<string>('');
  
  // Navigation items to match the builder-web side-nav
  navigationItems: NavigationItem[] = [];
  
  private initNavigationItems() {
    this.navigationItems = [];
    
    // Core navigation - only show My Origins if signed in
    if (this.isSignedIn()) {
      this.navigationItems.push({
        label: 'My Origins',
        icon: 'group',
        route: '/origins'
      });
    }
    
    // Always show Search Packages
    this.navigationItems.push({
      label: 'Search Packages',
      icon: 'search',
      route: '/pkgs'
    });
    
    // Add Events if enabled
    if (this.configService.isFeatureEnabled('enableEvents')) {
      this.navigationItems.push({
        label: 'Events',
        icon: 'event',
        route: '/events'
      });
      
      // Add SaaS Events if both flags enabled
      if (this.configService.isFeatureEnabled('enableSaasEvents')) {
        this.navigationItems.push({
          label: 'Events (SaaS)',
          icon: 'cloud',
          route: '/events/saas'
        });
      }
    }
    
    // Add section divider
    this.navigationItems.push({
      divider: true,
      label: 'Quick Links'
    });
    
    // Quick links from configuration
    this.navigationItems.push({
      label: 'Download Habitat',
      icon: 'file_download',
      route: this.configService.getUrl('download')
    });
    
    this.navigationItems.push({
      label: 'Documentation',
      icon: 'description',
      route: this.configService.getUrl('docs')
    });
    
    this.navigationItems.push({
      label: 'Tutorials',
      icon: 'school',
      route: this.configService.getUrl('tutorials')
    });
  }
  
  ngOnInit() {
    // Set initial state based on authService
    this.isSignedIn.set(this.authService.isAuthenticated());
    
    // Update navigation items with URLs from config
    this.updateNavigationUrls();
    
    if (this.authService.isAuthenticated()) {
      const user = this.authService.currentUser();
      if (user) {
        this.username.set(user.name);
        this.avatarUrl.set(user.avatar || 'assets/images/avatar.svg');
      }
    }
    
    // Subscribe to auth state changes using the legacy observable
    this.authService.authStatus$.subscribe(isAuth => {
      this.isSignedIn.set(isAuth);
      
      if (isAuth) {
        const user = this.authService.currentUser();
        if (user) {
          this.username.set(user.name);
          // Set user avatar with fallback
          this.avatarUrl.set(user.avatar || 'assets/images/avatar.svg');
        }
      } else {
        this.username.set('');
        this.avatarUrl.set('');
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
  
  // Helper methods for feature flags
  eventsEnabled(): boolean {
    return this.configService.isFeatureEnabled('enableEvents');
  }
  
  saasEventsEnabled(): boolean {
    return this.configService.isFeatureEnabled('enableSaasEvents');
  }
}
