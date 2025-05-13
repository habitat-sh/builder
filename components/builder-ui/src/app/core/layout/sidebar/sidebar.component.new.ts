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
  isExternal?: boolean; // Add this property to identify external links explicitly
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
        <ng-container *ngFor="let section of sectionsCache">
          <!-- Section title -->
          <h3 *ngIf="section.title && !collapsed" [class.first]="section.title === 'Builder'">{{ section.title }}</h3>
          
          <!-- Section items -->
          <ul *ngIf="section.items.length > 0">
            <li *ngFor="let item of section.items">
              <!-- Regular navigation item -->
              <ng-container *ngIf="!item.children?.length">
                <a 
                  [routerLink]="(isExternalLink(item.route) || item.isExternal) ? undefined : item.route"
                  [attr.href]="(isExternalLink(item.route) || item.isExternal) ? item.route : undefined"
                  [attr.target]="(isExternalLink(item.route) || item.isExternal) ? '_blank' : undefined"
                  routerLinkActive="active-link"
                  [matTooltip]="collapsed ? item.label : ''"
                  [matTooltipPosition]="'right'">
                  <mat-icon *ngIf="item.icon">{{ item.icon }}</mat-icon>
                  <span *ngIf="!collapsed">{{ item.label }}</span>
                </a>
              </ng-container>
              
              <!-- Group with children -->
              <ng-container *ngIf="item.children?.length">
                <div class="nav-group">
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
                    <ng-container *ngFor="let child of item.children">
                      <a 
                        [routerLink]="(isExternalLink(child.route) || child.isExternal) ? undefined : child.route"
                        [attr.href]="(isExternalLink(child.route) || child.isExternal) ? child.route : undefined" 
                        [attr.target]="(isExternalLink(child.route) || child.isExternal) ? '_blank' : undefined"
                        routerLinkActive="active-child"
                        class="nav-child-item">
                        <mat-icon *ngIf="child.icon">{{ child.icon }}</mat-icon>
                        <span>{{ child.label }}</span>
                      </a>
                    </ng-container>
                  </div>
                </div>
              </ng-container>
            </li>
          </ul>
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
      font-family: 'Titillium Web', 'Helvetica Neue', Helvetica, Roboto, Arial, sans-serif;
    }
    
    .logo-container {
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 0 0 24px;
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
    
    ul {
      list-style-type: none;
      margin: 0;
      padding: 0;
    }
    
    li {
      margin-bottom: 6px;
    }
    
    h3 {
      font-size: 14px;
      font-weight: 400;
      margin: 16px 0 8px 8px;
      color: #a8bcc8;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      user-select: none;
    }
    
    h3.first {
      margin-top: 8px;
    }
    
    a {
      display: flex;
      align-items: center;
      padding: 8px 12px;
      text-decoration: none;
      color: #e9f0f4;
      border-radius: 4px;
      transition: background-color 0.2s ease;
      cursor: pointer;
    }
    
    a:hover {
      background-color: rgba(255, 255, 255, 0.1);
    }
    
    a.active-link {
      background-color: rgba(255, 255, 255, 0.18);
      font-weight: 500;
    }
    
    .nav-group {
      margin-bottom: 6px;
    }
    
    .nav-group-header {
      display: flex;
      align-items: center;
      padding: 8px 12px;
      cursor: pointer;
      border-radius: 4px;
      transition: background-color 0.2s ease;
    }
    
    .nav-group-header:hover {
      background-color: rgba(255, 255, 255, 0.1);
    }
    
    .nav-group-header.active {
      background-color: rgba(255, 255, 255, 0.12);
    }
    
    .nav-group-items {
      padding-left: 16px;
      margin-top: 2px;
    }
    
    .nav-child-item {
      padding: 6px 12px;
      font-size: 0.95em;
    }
    
    .nav-child-item.active-child {
      font-weight: 500;
    }
    
    mat-icon {
      margin-right: 12px;
      color: #c4d5e0;
      font-size: 18px;
      height: 18px;
      width: 18px;
    }
    
    .collapse-icon {
      margin-right: 0;
      font-size: 20px;
      width: 20px;
      height: 20px;
    }
    
    span {
      white-space: nowrap;
    }
    
    .spacer {
      flex: 1;
    }
    
    .expand-icon {
      margin-right: 0;
      transition: transform 0.2s ease;
    }
    
    .close-button {
      background: none;
      border: none;
      color: #e9f0f4;
      padding: 4px;
      margin-left: auto;
      cursor: pointer;
      display: none;
    }
    
    @media (max-width: 959px) {
      .close-button {
        display: block;
      }
      
      .logo-container {
        justify-content: space-between;
      }
    }
    
    .sidebar-footer {
      margin-top: 20px;
      padding: 16px 0;
      font-size: 0.85em;
    }
  `]
})
export class SidebarComponent implements OnInit {
  @Input() navigationItems: NavigationItem[] = [];
  @Input() logoUrl = 'assets/images/builder-habitat-logo.svg';
  @Input() showLogo = true;
  @Input() collapsed = false;
  
  @Output() closeMobileSidebar = new EventEmitter<void>();
  
  sectionsCache: {title: string; items: NavigationItem[]}[] = [];
  private logoLoadAttempts = 0;
  private maxLogoLoadAttempts = 3;
  
  private assetLoader = inject(AssetLoaderService);
  
  ngOnInit(): void {
    this.tryAdditionalLogoSources();
    
    // Initialize sections cache
    this.updateSectionsCache();
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
  
  /**
   * Check if a route is an external link
   * @param route The route to check
   * @returns True if it's an external link
   */
  isExternalLink(route?: string): boolean {
    if (!route) return false;
    
    // Check for common external URL patterns
    return route.startsWith('http://') || 
           route.startsWith('https://') || 
           route.includes('://');
  }
  
  get isMobileView(): boolean {
    return window.innerWidth < 960;
  }

  /**
   * Update the sections cache when navigation items change
   */
  updateSectionsCache(): void {
    this.sectionsCache = this.groupNavigationBySection(this.navigationItems);
  }
  
  /**
   * Groups navigation items by section
   * @param items The navigation items to group
   * @returns An array of sections with their items
   */
  groupNavigationBySection(items: NavigationItem[]) {
    const sections: {title: string; items: NavigationItem[]}[] = [];
    let currentSection: {title: string; items: NavigationItem[]} | null = null;
    
    for (const item of items) {
      if (item.divider) {
        // Start a new section
        if (currentSection) {
          sections.push(currentSection);
        }
        currentSection = {
          title: item.label || '',
          items: []
        };
      } else if (currentSection) {
        // Add item to current section
        currentSection.items.push(item);
      }
    }
    
    // Add the last section if it exists
    if (currentSection) {
      sections.push(currentSection);
    }
    
    return sections;
  }
  
  /**
   * Handle logo loading errors
   */
  handleLogoError(event: ErrorEvent): void {
    console.warn('Logo loading failed:', event);
    this.logoLoadAttempts++;
    
    this.assetLoader.reportAssetError(this.logoUrl);
    
    // Try with a '/' prefix if needed
    if (this.logoLoadAttempts < this.maxLogoLoadAttempts && !this.logoUrl.startsWith('/')) {
      console.log('Trying with / prefix...');
      const newPath = '/' + this.logoUrl;
      this.logoUrl = newPath;
      this.assetLoader.reportAssetLoading(newPath);
    }
    
    // Try to fall back to a different logo
    if (this.logoLoadAttempts >= this.maxLogoLoadAttempts) {
      const fallbackPath = 'assets/images/habitat-logo.svg';
      
      if (this.logoUrl.includes('builder-habitat-logo')) {
        console.log('Falling back to standard habitat logo');
        this.logoUrl = fallbackPath;
        this.assetLoader.reportAssetLoading(fallbackPath);
      }
    }
  }
  
  /**
   * Handle successful logo loading
   */
  handleLogoLoad(): void {
    console.log('Logo loaded successfully:', this.logoUrl);
    this.assetLoader.reportAssetSuccess(this.logoUrl);
  }
}
