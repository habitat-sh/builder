import { Component, Input, Output, EventEmitter, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { MatToolbarModule } from '@angular/material/toolbar';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatMenuModule } from '@angular/material/menu';
import { MatDividerModule } from '@angular/material/divider';
import { MatTooltipModule } from '@angular/material/tooltip';
import { AssetLoaderService } from '../../../shared/services/asset-loader.service';
import { FallbackImageDirective } from '../../../shared/directives/fallback-image.directive';
import { AuthService } from '../../services/auth.service';

export interface UserInfo {
  name: string;
  email: string;
  avatar?: string;
}

@Component({
  selector: 'app-header, header',
  standalone: true,
  imports: [
    CommonModule,
    RouterLink,
    MatToolbarModule,
    MatButtonModule,
    MatIconModule,
    MatMenuModule,
    MatDividerModule,
    MatTooltipModule,
    FallbackImageDirective
  ],
  styleUrls: ['./header.component.scss'],
  template: `
    <mat-toolbar color="primary" class="header">
      <div class="header-left">
        <button 
          mat-icon-button 
          (click)="toggleSideNav.emit()" 
          matTooltip="Toggle navigation"
          class="toggle-button">
          <mat-icon>menu</mat-icon>
        </button>
        
        <div class="logo-container" *ngIf="showLogo">
          <img 
            [src]="logoPath"
            alt="Habitat Logo" 
            class="logo"
            habFallbackImage
            fallbackType="logo"
            (error)="onLogoError($event)"
            (load)="onLogoLoad()"
            routerLink="/">
        </div>
        
        <h1 class="title">{{ title }}</h1>
      </div>
      
      <div class="spacer"></div>
      
      <div class="header-right">
        <ng-content select="[header-actions]"></ng-content>
        
        <!-- Sign In button (shown when user is not signed in) -->
        <button 
          *ngIf="!isSignedIn"
          mat-flat-button
          color="accent"
          class="sign-in-button"
          routerLink="/sign-in">
          Sign In
        </button>
        
        <!-- Search button -->
        <button 
          mat-icon-button 
          matTooltip="Search" 
          (click)="onSearchClick()"
          class="action-button">
          <mat-icon>search</mat-icon>
        </button>
        
        <!-- Help menu -->
        <button 
          mat-icon-button 
          [matMenuTriggerFor]="helpMenu" 
          matTooltip="Help"
          class="action-button">
          <mat-icon>help</mat-icon>
        </button>
        <mat-menu #helpMenu="matMenu">
          <a mat-menu-item href="https://www.habitat.sh/docs" target="_blank">
            <mat-icon>library_books</mat-icon>
            <span>Documentation</span>
          </a>
          <a mat-menu-item href="https://www.habitat.sh/tutorials" target="_blank">
            <mat-icon>school</mat-icon>
            <span>Tutorials</span>
          </a>
          <a mat-menu-item href="https://github.com/habitat-sh/habitat/issues" target="_blank">
            <mat-icon>bug_report</mat-icon>
            <span>Report an Issue</span>
          </a>
        </mat-menu>
        
        <!-- User menu -->
        <button 
          mat-icon-button 
          [matMenuTriggerFor]="userMenu"
          matTooltip="User menu"
          class="action-button"
          *ngIf="isSignedIn">
          <div class="avatar-container" *ngIf="avatarUrl; else defaultAvatar">
            <img [src]="avatarUrl" [alt]="username || 'User'" habFallbackImage fallbackType="avatar" (error)="onAvatarError($event)">
          </div>
          <ng-template #defaultAvatar>
            <mat-icon>account_circle</mat-icon>
          </ng-template>
        </button>
        <mat-menu #userMenu="matMenu">
          <div class="user-info">
            <div class="user-avatar">
              <div class="avatar-container" *ngIf="avatarUrl; else userDefaultAvatar">
                <img [src]="avatarUrl" [alt]="username || 'User'" habFallbackImage fallbackType="avatar" (error)="onAvatarError($event)">
              </div>
              <ng-template #userDefaultAvatar>
                <div class="default-avatar">
                  <mat-icon>account_circle</mat-icon>
                </div>
              </ng-template>
            </div>
            <div class="user-details">
              <div class="user-name">{{ username }}</div>
              <div class="user-email">{{ user?.email || '' }}</div>
            </div>
          </div>
          
          <mat-divider></mat-divider>
          
          <button mat-menu-item routerLink="/profile">
            <mat-icon>person</mat-icon>
            <span>My Profile</span>
          </button>
          
          <button mat-menu-item routerLink="/settings">
            <mat-icon>settings</mat-icon>
            <span>Settings</span>
          </button>
          
          <a mat-menu-item href="https://www.chef.io/patents" target="_blank">
            <mat-icon>gavel</mat-icon>
            <span>Chef Patents</span>
            <mat-icon class="external-link-icon">open_in_new</mat-icon>
          </a>
          
          <mat-divider></mat-divider>
          
          <button mat-menu-item (click)="logout.emit()">
            <mat-icon>exit_to_app</mat-icon>
            <span>Log Out</span>
          </button>
        </mat-menu>
      </div>
    </mat-toolbar>
  `,
  styles: []
})
export class HeaderComponent implements OnInit {
  @Input() title = 'Habitat Builder';
  @Input() showLogo = true;
  @Input() user: UserInfo | null = null;
  @Input() username = '';
  @Input() avatarUrl = '';
  @Input() isSignedIn = false; // Accept isSignedIn as input property
  
  // Track avatar image loading errors
  hasAvatarError = false;
  logoLoaded = false;
  
  // List of logo paths to try
  private logoPaths = [
    'assets/images/habitat-logo.svg',
    '/assets/images/habitat-logo.svg'
  ];
  
  private currentLogoIndex = 0;
  
  get logoPath(): string {
    return this.logoPaths[this.currentLogoIndex];
  }
  
  private assetLoader = inject(AssetLoaderService);
  private authService = inject(AuthService);
  
  @Output() toggleSideNav = new EventEmitter<void>();
  @Output() search = new EventEmitter<void>();
  @Output() logout = new EventEmitter<void>();
  @Output() signOut = new EventEmitter<void>();
  
  ngOnInit(): void {
    // Initialize with default avatar if none provided
    if (this.isSignedIn && !this.avatarUrl) {
      this.avatarUrl = 'assets/images/avatar.svg';
    }
    
    // Report that we're loading the logo
    this.assetLoader.reportAssetLoading(this.logoPath);
  }
  
  onSearchClick(): void {
    this.search.emit();
  }
  
  handleSignOut(): void {
    this.signOut.emit();
    this.logout.emit(); // For backward compatibility
  }
  
  /**
   * Handle avatar image loading errors
   * @param event The error event
   */
  onAvatarError(event: Event): void {
    console.error('Avatar failed to load:', this.avatarUrl);
    this.assetLoader.reportAssetError(this.avatarUrl);
    this.hasAvatarError = true;
    
    // Try to load from assets if it's a relative URL
    if (this.avatarUrl && !this.avatarUrl.startsWith('http') && !this.avatarUrl.includes('avatar.svg')) {
      console.log('Trying default avatar...');
      this.avatarUrl = 'assets/images/avatar.svg';
      this.assetLoader.reportAssetLoading(this.avatarUrl);
      // Reset error flag to try the new URL
      this.hasAvatarError = false;
    }
  }
  
  /**
   * Handle logo image loading errors
   * @param event The error event
   */
  onLogoError(event: Event): void {
    console.error('Logo failed to load:', this.logoPath);
    this.assetLoader.reportAssetError(this.logoPath);
    
    // Try next logo in the path list
    if (this.currentLogoIndex < this.logoPaths.length - 1) {
      this.currentLogoIndex++;
      this.assetLoader.reportAssetLoading(this.logoPath);
      return;
    }
    
    // If all paths failed, apply CSS fallback for logo
    const imgElement = event.target as HTMLImageElement;
    if (imgElement) {
      imgElement.style.display = 'none';
      
      // Create text fallback
      const container = imgElement.parentElement;
      if (container) {
        const fallback = document.createElement('div');
        fallback.textContent = 'H';
        fallback.style.width = '36px';
        fallback.style.height = '36px';
        fallback.style.backgroundColor = '#FF9012';
        fallback.style.borderRadius = '4px';
        fallback.style.color = 'white';
        fallback.style.display = 'flex';
        fallback.style.alignItems = 'center';
        fallback.style.justifyContent = 'center';
        fallback.style.fontWeight = 'bold';
        container.appendChild(fallback);
      }
    }
  }
  
  /**
   * Handle successful logo load
   */
  onLogoLoad(): void {
    console.log('Logo loaded successfully:', this.logoPath);
    this.assetLoader.reportAssetSuccess(this.logoPath);
    this.logoLoaded = true;
  }
}