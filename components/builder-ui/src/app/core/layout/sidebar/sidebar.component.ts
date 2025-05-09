import { Component, Input, Output, EventEmitter, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { MatListModule } from '@angular/material/list';
import { MatIconModule } from '@angular/material/icon';
import { MatDividerModule } from '@angular/material/divider';
import { MatTooltipModule } from '@angular/material/tooltip';
import { AssetLoaderService } from '../../../shared/services/asset-loader.service';
import { FallbackImageDirective } from '../../../shared/directives/fallback-image.directive';

export interface NavigationItem {
  label: string;
  icon?: string;
  route?: string;
  children?: NavigationItem[];
  expanded?: boolean;
  divider?: boolean;
  permissions?: string[];
}

@Component({
  selector: 'app-sidebar',
  standalone: true,
  imports: [
    CommonModule,
    RouterLink,
    RouterLinkActive,
    MatListModule,
    MatIconModule,
    MatDividerModule,
    MatTooltipModule,
    FallbackImageDirective
  ],
  template: `
    <div class="sidebar-container">
      <!-- App logo -->
      <div class="logo-container" *ngIf="showLogo">
        <div class="logo-wrapper">
          <img 
            [src]="logoUrl"
            alt="Habitat Builder Logo"
            class="logo"
            habFallbackImage
            fallbackType="logo"
            (error)="handleLogoError($event)"
            (load)="handleLogoLoad()">
        </div>
        
        <!-- Mobile close button -->
        <button *ngIf="isMobileView" class="close-button" (click)="closeMenu()">
          <mat-icon>close</mat-icon>
        </button>
      </div>
      
      <!-- Navigation items -->
      <div class="nav-items">
        <ng-container *ngFor="let item of navigationItems">
          <!-- Divider with title -->
          <ng-container *ngIf="item.divider">
            <mat-divider *ngIf="item.label !== 'Builder'"></mat-divider>
            <h3 *ngIf="item.label && !collapsed" [class.first]="item.label === 'Builder'">{{ item.label }}</h3>
          </ng-container>
          
          <!-- Regular navigation item with link -->
          <a 
            *ngIf="!item.divider && item.route && !item.children?.length && (!item.permissions || (item.permissions.includes('isSignedIn') && isSignedIn))"
            mat-list-item
            [routerLink]="isExternalLink(item.route) ? undefined : item.route"
            [attr.href]="isExternalLink(item.route) ? item.route : undefined"
            [attr.target]="isExternalLink(item.route) ? '_blank' : undefined"
            routerLinkActive="active-link"
            class="nav-item"
            [matTooltip]="collapsed ? item.label : ''"
            [matTooltipPosition]="'right'">
            <mat-icon *ngIf="item.icon" matListItemIcon>{{ item.icon }}</mat-icon>
            <span matListItemTitle *ngIf="!collapsed">{{ item.label }}</span>
          </a>
          
          <!-- Group with children -->
          <div *ngIf="!item.divider && item.children?.length" class="nav-group">
            <div 
              class="nav-group-header"
              [class.active]="item.expanded"
              (click)="toggleGroup(item)">
              <mat-icon *ngIf="item.icon">{{ item.icon }}</mat-icon>
              <span *ngIf="!collapsed">{{ item.label }}</span>
              <span class="spacer"></span>
              <mat-icon *ngIf="!collapsed" class="expand-icon">
                {{ item.expanded ? 'expand_less' : 'expand_more' }}
              </mat-icon>
            </div>
            
            <div class="nav-group-items" *ngIf="item.expanded && !collapsed">
              <a 
                *ngFor="let child of item.children"
                mat-list-item
                [routerLink]="isExternalLink(child.route) ? undefined : child.route"
                [attr.href]="isExternalLink(child.route) ? child.route : undefined" 
                [attr.target]="isExternalLink(child.route) ? '_blank' : undefined"
                routerLinkActive="active-child"
                class="nav-child-item">
                <mat-icon *ngIf="child.icon" matListItemIcon>{{ child.icon }}</mat-icon>
                <span matListItemTitle>{{ child.label }}</span>
              </a>
            </div>
          </div>
        </ng-container>
      </div>
      
      <!-- Footer slot -->
      <div class="sidebar-footer" *ngIf="!collapsed">
        <ng-content select="[sidebar-footer]"></ng-content>
      </div>
    </div>
  `,
  styles: [`
    .sidebar-container {
      display: flex;
      flex-direction: column;
      height: 100%;
      overflow-y: auto;
      overflow-x: hidden;
      box-sizing: border-box;
      width: 100%;
      background: linear-gradient(to top, #556F84, #283C4C);
      color: #ffffff;
      padding: 16px 32px;
    }
    
    .logo-container {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 16px 16px 24px;
      border-bottom: 1px solid rgba(255, 255, 255, 0.15);
      margin-bottom: 16px;
    }
    
    .logo-wrapper {
      display: flex;
      justify-content: center;
      width: 100%;
    }
    
    .logo {
      width: 160px;
      height: auto;
      display: block;
    }
    
    .logo-fallback {
      width: 160px;
      height: 160px;
      background-color: #FF9012;
      border-radius: 4px;
      color: white;
      display: flex;
      align-items: center;
      justify-content: center;
      font-weight: bold;
      font-size: 16px;
    }
    
    .app-name {
      margin: 0 0 0 12px;
      font-weight: 600;
      font-size: 18px;
      color: #ffffff;
      text-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
      letter-spacing: 0.5px;
      white-space: nowrap;
    }
    
    .nav-items {
      flex: 1;
      overflow-y: auto;
      padding: 0;
    }
    
    .nav-item {
      height: 40px;
      padding: 0 16px 0 24px;
      margin: 2px 0;
      display: flex;
      align-items: center;
      transition: all 0.2s ease;
      border-left: 3px solid transparent;
      color: #D8D8D8;
      text-decoration: none;
      font-weight: 600;
      font-size: 16px;
      line-height: 32px;
    }
    
    .nav-item:hover {
      background-color: rgba(255, 255, 255, 0.1);
      border-left-color: rgba(255, 255, 255, 0.5);
      color: #ffffff;
    }
    
    .active-link {
      background-color: rgba(255, 255, 255, 0.15);
      color: #ffffff;
      border-left: 3px solid #FF9012;
      font-weight: 600;
    }
    
    h3 {
      font-size: 14px;
      text-transform: uppercase;
      color: rgba(255, 255, 255, 0.6);
      margin: 24px 0 8px 24px;
      font-weight: 600;
      letter-spacing: 1px;
      
      &.first {
        margin-top: 0;
      }
    }
    
    .nav-group {
      margin: 12px 0;
    }
    
    .nav-group-header {
      display: flex;
      align-items: center;
      padding: 0 16px 0 24px;
      height: 44px;
      cursor: pointer;
      transition: all 0.2s ease;
      color: rgba(255, 255, 255, 0.85);
      font-weight: 500;
    }
    
    .nav-group-header:hover {
      background-color: rgba(255, 255, 255, 0.1);
      color: #ffffff;
    }
    
    .nav-group-header.active {
      background-color: rgba(0, 0, 0, 0.04);
    }
    
    .nav-group-header mat-icon {
      margin-right: 16px;
    }
    
    .nav-group-items {
      padding-left: 16px;
    }
    
    .nav-child-item {
      height: 40px;
      font-size: 14px;
    }
    
    .close-button {
      position: absolute;
      top: 12px;
      right: 12px;
      background: none;
      border: none;
      color: rgba(255, 255, 255, 0.7);
      width: 36px;
      height: 36px;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      cursor: pointer;
      transition: all 0.2s ease;
      
      &:hover {
        background-color: rgba(255, 255, 255, 0.1);
        color: #ffffff;
      }
      
      @media (min-width: 768px) {
        display: none;
      }
    }
    
    .active-child {
      background-color: rgba(33, 150, 243, 0.05);
      color: #1976d2;
    }
    
    .expand-icon {
      transition: transform 0.3s;
      font-size: 18px;
      width: 18px;
      height: 18px;
    }
    
    .spacer {
      flex: 1;
    }
    
    .sidebar-footer {
      border-top: 1px solid rgba(0, 0, 0, 0.12);
      padding: 16px;
    }
  `]
})
export class SidebarComponent implements OnInit {
  @Input() appName = 'Habitat Builder';
  @Input() logoUrl = 'assets/images/builder-habitat-logo.svg';
  @Input() collapsed = false;
  @Input() showLogo = true;
  @Input() navigationItems: NavigationItem[] = [];
  @Input() isSignedIn = false;
  @Input() enabledEvents: boolean = true;
  @Input() enabledSaasEvents: boolean = false;
  @Input() config: any = {};
  
  // Track if logo loaded successfully
  logoLoaded: boolean = true;
  
  @Output() closeMobileSidebar = new EventEmitter<void>();

  ngOnInit() {
    // Initialize default navigation items if none provided
    if (this.navigationItems.length === 0) {
      this.initDefaultNavItems();
    }
    
    // Report that we're loading the logo
    this.assetLoader.reportAssetLoading(this.logoUrl);
    
    // Try additional logo sources if the default one fails
    this.tryAdditionalLogoSources();
  }
  
  /**
   * Try multiple logo sources to find one that works
   * This helps ensure we always display a logo no matter what
   */
  private tryAdditionalLogoSources(): void {
    const alternativePaths = [
      'assets/images/habitat-logo.svg',
      '/assets/images/habitat-logo.svg',
      'assets/images/builder-habitat-logo.svg',
      '/assets/images/builder-habitat-logo.svg'
    ];
    
    // Create image elements to test loading
    for (const path of alternativePaths) {
      if (path === this.logoUrl) continue;
      
      const img = new Image();
      img.onload = () => {
        console.log(`Alternative logo loaded: ${path}`);
        this.assetLoader.reportAssetSuccess(path);
        // If current logo fails, we have this as backup
      };
      img.onerror = () => {
        console.error(`Alternative logo failed: ${path}`);
        this.assetLoader.reportAssetError(path);
      };
      this.assetLoader.reportAssetLoading(path);
      img.src = path;
    }
  }
  
  toggleGroup(item: NavigationItem): void {
    if (!this.collapsed) {
      item.expanded = !item.expanded;
    }
  }
  
  closeMenu(): void {
    this.closeMobileSidebar.emit();
  }
  
  get isMobileView(): boolean {
    return window.innerWidth < 960;
  }
  
  /**
   * Check if a route is an external link
   * @param route The route to check
   * @returns True if the route is an external link
   */
  isExternalLink(route?: string): boolean {
    if (!route) return false;
    return route.startsWith('http://') || route.startsWith('https://');
  }

  private initDefaultNavItems() {
    const mainNavItems: NavigationItem[] = [];
    
    // Add Builder section header
    mainNavItems.push({ divider: true, label: 'Builder' });
    
    // Only show My Origins if signed in
    if (this.isSignedIn) {
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

    // Add Events navigation if enabled
    if (this.enabledEvents) {
      mainNavItems.push({
        label: 'Events',
        icon: 'event',
        route: '/events'
      });
    }

    // Add SaaS Events navigation if both flags are enabled
    if (this.enabledEvents && this.enabledSaasEvents) {
      mainNavItems.push({
        label: 'Events (SaaS)',
        icon: 'cloud',
        route: '/events/saas'
      });
    }

    // Add section title for quick links
    mainNavItems.push({ divider: true, label: 'Quick Links' });

    // Quick links section - exactly matching the original side-nav.component.html
    const quickLinks: NavigationItem[] = [
      {
        label: 'Download Habitat',
        icon: 'file_download',
        route: this.config['docs_url'] ? `${this.config['docs_url']}/install-habitat/` : 'https://www.habitat.sh/docs/install-habitat/'
      },
      {
        label: 'Docs',
        icon: 'description',
        route: this.config['docs_url'] || 'https://docs.chef.io/habitat/'
      },
      {
        label: 'Tutorials',
        icon: 'explore',
        route: this.config['tutorials_url'] || 'https://learn.chef.io/habitat/'
      },
      {
        label: 'Blog',
        icon: 'rss_feed',
        route: this.config['www_url'] ? `${this.config['www_url']}/blog` : 'https://www.habitat.sh/blog'
      },
      {
        label: 'Website',
        icon: 'language',
        route: this.config['www_url'] || 'https://www.habitat.sh'
      },
      {
        label: 'GitHub',
        icon: 'code',
        route: this.config['source_code_url'] || 'https://github.com/habitat-sh/habitat'
      }
    ];
    
    // Add Service Status section if config is SaaS
    if (this.config && this.config.is_saas) {
      mainNavItems.push({ divider: true, label: 'Service Status' });
      // We'll need to implement the statuspage component separately
      // For now, add a placeholder
      mainNavItems.push({
        label: 'Status',
        icon: 'info',
        route: 'https://status.chef.io/'
      });
    }
    
    // Append quick links to mainNavItems rather than combining them separately
    mainNavItems.push(...quickLinks);
    this.navigationItems = mainNavItems;
  }
  
  // Service for asset loading diagnostics
  private assetLoader = inject(AssetLoaderService);
  
  /**
   * Handle image loading error
   * @param event The error event
   */
  handleLogoError(event: any): void {
    console.error(`Logo failed to load: ${this.logoUrl}`);
    this.assetLoader.reportAssetError(this.logoUrl);
    this.logoLoaded = false;
    
    // Try a different path approach
    if (this.logoUrl.startsWith('assets/')) {
      console.log('Trying with / prefix...');
      const newPath = '/' + this.logoUrl;
      this.assetLoader.reportAssetLoading(newPath);
      this.logoUrl = newPath;
      this.logoLoaded = true;
      return;
    }
    
    // Try to fall back to a different logo
    if (this.logoUrl.includes('builder-habitat-logo')) {
      console.log('Trying fallback logo...');
      const fallbackPath = 'assets/images/habitat-logo.svg';
      this.assetLoader.reportAssetLoading(fallbackPath);
      this.logoUrl = fallbackPath;
      this.logoLoaded = true;
    }
  }
  
  /**
   * Handle successful image load
   */
  handleLogoLoad(): void {
    console.log(`Logo loaded successfully: ${this.logoUrl}`);
    this.assetLoader.reportAssetSuccess(this.logoUrl);
    this.logoLoaded = true;
  }
}
