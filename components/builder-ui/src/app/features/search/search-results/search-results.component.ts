import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { MatIconModule } from '@angular/material/icon';
import { MatCardModule } from '@angular/material/card';
import { MatChipsModule } from '@angular/material/chips';
import { MatTooltipModule } from '@angular/material/tooltip';
import { Package, PackageVisibility } from '../../../shared/models/package.model';
import { packageString, packageRoutePath } from '../../../shared/utils/package.utils';

@Component({
  selector: 'app-search-results',
  standalone: true,
  imports: [
    CommonModule,
    RouterLink,
    MatIconModule,
    MatCardModule,
    MatChipsModule,
    MatTooltipModule
  ],
  template: `
    <div class="results-container">
      <!-- Loading state -->
      <div class="loading-skeletons" *ngIf="isLoading" role="status" aria-label="Loading search results">
        <div class="skeleton-item" *ngFor="let i of [1, 2, 3, 4, 5]">
          <div class="skeleton-header">
            <div class="skeleton-title"></div>
            <div class="skeleton-badge"></div>
          </div>
          <div class="skeleton-info">
            <div class="skeleton-text-short"></div>
          </div>
          <div class="skeleton-channels">
            <div class="skeleton-chip"></div>
            <div class="skeleton-chip"></div>
          </div>
          <div class="skeleton-target">
            <div class="skeleton-icon"></div>
            <div class="skeleton-text-short"></div>
          </div>
        </div>
      </div>
      
      <!-- No results state -->
      <div class="no-results" *ngIf="noPackages && !isLoading" role="alert" aria-live="polite">
        <mat-icon aria-hidden="true">search_off</mat-icon>
        <p>No packages found matching your search criteria.</p>
        <div class="no-results-suggestions">
          <h3>Try one of these suggestions:</h3>
          <ul>
            <li>Try different keywords</li>
            <li>Check the origin name is correct</li>
            <li>Try a more general search term</li>
            <li>Try searching in all origins with origin set to "*"</li>
          </ul>
        </div>
      </div>
      
      <!-- Results list -->
      <div class="results-list" *ngIf="!noPackages && !isLoading" role="list" aria-label="Search results">
        <mat-card class="package-item" *ngFor="let pkg of packages; let i = index" 
          [ngClass]="getVisibilityClass(pkg)" 
          role="listitem">
          <a [routerLink]="routeFor(pkg)" class="package-link" 
            [attr.aria-label]="'View details for ' + packageString(pkg)">
            <div class="package-details">
              <div class="package-header">
                <span class="package-name">{{ getPackageName(pkg) }}</span>
                
                <!-- Visibility badge -->
                <span 
                  class="visibility-badge" 
                  *ngIf="pkg.visibility !== PackageVisibility.Public" 
                  [matTooltip]="pkg.visibility === PackageVisibility.Private ? 'Private package' : 'Hidden package'"
                  [attr.aria-label]="pkg.visibility === PackageVisibility.Private ? 'Private package' : 'Hidden package'">
                  <mat-icon aria-hidden="true">{{ pkg.visibility === PackageVisibility.Private ? 'lock' : 'visibility_off' }}</mat-icon>
                </span>
              </div>
              
              <div class="package-info" 
                aria-label="Package identification">
                <span class="package-origin">{{ pkg.ident.origin }}</span>
                <span class="package-version" *ngIf="pkg.ident.version">{{ pkg.ident.version }}</span>
                <span class="package-release" *ngIf="pkg.ident.release">{{ pkg.ident.release }}</span>
              </div>
              
              <!-- Channels chips -->
              <div class="package-channels" *ngIf="pkg.channels && pkg.channels.length" 
                aria-label="Available in channels">
                <mat-chip-set aria-label="Package channels">
                  <mat-chip *ngFor="let channel of getTopChannels(pkg)" 
                    [matTooltip]="'Available in ' + channel"
                    [attr.aria-label]="'Available in ' + channel + ' channel'">
                    {{ channel }}
                  </mat-chip>
                </mat-chip-set>
              </div>
              
              <div class="package-target" *ngIf="pkg.target?.name"
                aria-label="Target platform">
                <mat-icon aria-hidden="true">computer</mat-icon>
                <span>{{ pkg.target.name }}</span>
              </div>
            </div>
            <mat-icon class="nav-icon" aria-hidden="true">chevron_right</mat-icon>
          </a>
        </mat-card>
      </div>
      
      <!-- Keyboard navigation hint for screen readers -->
      <div class="sr-only" aria-live="polite" *ngIf="!noPackages && !isLoading && packages.length > 0">
        Press tab to navigate through search results. Press enter to view package details.
      </div>
    </div>
  `,
  styles: [`
    .results-container {
      margin-bottom: 24px;
    }
    
    .no-results {
      text-align: center;
      padding: 48px 0;
      color: #666;
    }
    
    .no-results mat-icon {
      font-size: 48px;
      height: 48px;
      width: 48px;
      margin-bottom: 16px;
      color: #999;
    }
    
    .no-results p {
      font-size: 18px;
      margin: 0;
    }
    
    .no-results-suggestions {
      margin-top: 24px;
      text-align: left;
      max-width: 450px;
      margin-left: auto;
      margin-right: auto;
      background-color: #f5f5f5;
      padding: 16px 24px;
      border-radius: 8px;
    }
    
    .no-results-suggestions p {
      font-size: 16px;
      margin-bottom: 10px;
      color: #333;
    }
    
    .no-results-suggestions ul {
      padding-left: 20px;
      margin: 0;
      color: #555;
    }
    
    .no-results-suggestions li {
      margin-bottom: 8px;
    }
    
    /* Loading skeleton styles */
    .loading-skeletons {
      display: flex;
      flex-direction: column;
      gap: 8px;
    }
    
    .skeleton-item {
      background: white;
      border-radius: 4px;
      padding: 16px;
      border-left: 4px solid #e0e0e0;
      display: flex;
      flex-direction: column;
      gap: 12px;
      box-shadow: 0 2px 4px rgba(0,0,0,0.05);
      animation: pulse 1.5s infinite;
    }
    
    .skeleton-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
    }
    
    .skeleton-title {
      height: 20px;
      width: 200px;
      background: #f0f0f0;
      border-radius: 4px;
    }
    
    .skeleton-badge {
      height: 16px;
      width: 16px;
      border-radius: 50%;
      background: #f0f0f0;
    }
    
    .skeleton-info {
      display: flex;
      gap: 8px;
    }
    
    .skeleton-text-short {
      height: 16px;
      width: 120px;
      background: #f0f0f0;
      border-radius: 4px;
    }
    
    .skeleton-channels {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
    }
    
    .skeleton-chip {
      height: 24px;
      width: 80px;
      background: #f0f0f0;
      border-radius: 16px;
    }
    
    .skeleton-target {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    
    .skeleton-icon {
      height: 16px;
      width: 16px;
      background: #f0f0f0;
      border-radius: 50%;
    }
    
    @keyframes pulse {
      0% {
        opacity: 1;
      }
      50% {
        opacity: 0.7;
      }
      100% {
        opacity: 1;
      }
    }
    
    /* Screen reader only class for accessibility */
    .sr-only {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }
    
    .results-list {
      display: flex;
      flex-direction: column;
      gap: 8px;
    }
    
    .package-item {
      border-left: 4px solid #1976d2;
      transition: transform 0.2s, box-shadow 0.2s;
    }
    
    .package-item.private {
      border-left-color: #f57c00;
    }
    
    .package-item.hidden {
      border-left-color: #757575;
    }
    
    .package-item:hover {
      transform: translateY(-2px);
      box-shadow: 0 4px 8px rgba(0,0,0,0.1);
    }
    
    .package-link {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 16px;
      text-decoration: none;
      color: inherit;
    }
    
    .package-details {
      display: flex;
      flex-direction: column;
      flex: 1;
    }
    
    .package-header {
      display: flex;
      align-items: center;
    }
    
    .package-name {
      font-weight: 500;
      font-size: 16px;
      color: #333;
    }
    
    .visibility-badge {
      display: inline-flex;
      align-items: center;
      margin-left: 8px;
    }
    
    .visibility-badge mat-icon {
      font-size: 16px;
      height: 16px;
      width: 16px;
      color: #757575;
    }
    
    .package-info {
      font-size: 14px;
      color: #666;
      margin-top: 4px;
    }
    
    .package-origin {
      color: #1976d2;
    }
    
    .package-version, .package-release {
      margin-left: 8px;
      color: #666;
    }
    
    .package-version::before {
      content: "v";
    }
    
    .package-channels {
      margin-top: 8px;
    }
    
    mat-chip-set {
      display: flex;
      flex-wrap: wrap;
    }
    
    mat-chip {
      font-size: 12px !important;
      height: 24px !important;
      min-height: 24px !important;
      background-color: #e3f2fd !important;
    }
    
    .package-target {
      display: flex;
      align-items: center;
      margin-top: 8px;
      font-size: 12px;
      color: #666;
    }
    
    .package-target mat-icon {
      font-size: 14px;
      height: 14px;
      width: 14px;
      margin-right: 4px;
    }
    
    .nav-icon {
      color: #1976d2;
    }
  `]
})
export class SearchResultsComponent {
  @Input() packages: Package[] = [];
  @Input() noPackages = false;
  @Input() isLoading = false;
  
  // Make PackageVisibility available to the template
  readonly PackageVisibility = PackageVisibility;
  
  /**
   * Get the route for a package detail page
   * 
   * Important: We must use the original package name (not the display name)
   * in the URL to ensure proper routing, especially for packages where
   * the name already contains the origin prefix (e.g., 'core/hab').
   * Not doing so would result in URLs like '/pkgs/core/hab' instead of
   * the correct '/pkgs/core/core/hab'.
   */
  routeFor(pkg: Package): string[] {
    return packageRoutePath(pkg);
  }
  
  /**
   * Get a clean package name for display purposes only
   * Removes the origin prefix if present in the name
   * 
   * WARNING: This method should ONLY be used for display purposes.
   * Do NOT use this for constructing URLs or API paths, as it strips
   * important parts of the package identifier.
   */
  getPackageName(pkg: Package): string {
    // Remove origin prefix if present in package name
    const name = pkg.ident.name;
    if (name.startsWith(`${pkg.ident.origin}/`)) {
      return name.substring(pkg.ident.origin.length + 1);
    }
    return name;
  }
  
  /**
   * Format a package identifier string
   */
  packageString(pkg: Package): string {
    return packageString(pkg);
  }
  
  /**
   * Get CSS class based on package visibility
   */
  getVisibilityClass(pkg: Package): string {
    if (pkg.visibility === PackageVisibility.Private) {
      return 'private';
    } else if (pkg.visibility === PackageVisibility.Hidden) {
      return 'hidden';
    }
    return '';
  }
  
  /**
   * Get the top channels (limited to 3) for display as chips
   */
  getTopChannels(pkg: Package): string[] {
    if (!pkg.channels || !pkg.channels.length) {
      return [];
    }
    
    // Prioritize stable and unstable channels
    const priorityChannels = ['stable', 'unstable'];
    const sortedChannels = [...pkg.channels].sort((a, b) => {
      const aIndex = priorityChannels.indexOf(a);
      const bIndex = priorityChannels.indexOf(b);
      
      // If both are priority channels, sort by priority
      if (aIndex >= 0 && bIndex >= 0) {
        return aIndex - bIndex;
      }
      
      // Priority channels come first
      if (aIndex >= 0) return -1;
      if (bIndex >= 0) return 1;
      
      // Otherwise alphanumeric sort
      return a.localeCompare(b);
    });
    
    // Return up to 3 channels
    return sortedChannels.slice(0, 3);
  }
}
