import { Component, Input, Output, EventEmitter, OnInit, AfterContentInit, ContentChild, inject, OnDestroy, TemplateRef } from '@angular/core';
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
import { HeaderService } from '../../services/header.service';

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
        <ng-content select="[header-title]"></ng-content>
        <ng-container *ngIf="headerService?.titleTemplateRef">
          <ng-template [ngTemplateOutlet]="headerService?.titleTemplateRef || null"></ng-template>
        </ng-container>
        <ng-container *ngIf="!headerTitleContent && !headerService?.titleTemplateRef">
          <h1 class="title">{{ title }}</h1>
        </ng-container>
      </div>
      
      <div class="spacer"></div>
      
      <div class="header-right">
        <ng-content select="[header-actions]"></ng-content>
        <ng-container *ngIf="headerService?.actionsTemplateRef">
          <ng-template [ngTemplateOutlet]="headerService?.actionsTemplateRef || null"></ng-template>
        </ng-container>
        
        <!-- Sign In button (shown when user is not signed in) -->
        <button 
          *ngIf="!isSignedIn"
          mat-flat-button
          color="accent"
          class="sign-in-button"
          routerLink="/sign-in">
          Sign In
        </button>
        
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
          
          <button mat-menu-item (click)="handleSignOut()">
            <mat-icon>exit_to_app</mat-icon>
            <span>Log Out</span>
          </button>
        </mat-menu>
      </div>
    </mat-toolbar>
  `,
  styles: [`
    .header-left h1.title {
      margin: 0;
      font-size: 24px;
      font-weight: normal;
    }
    
    .header-left ::ng-deep h1 {
      margin: 0;
      font-size: 24px;
      font-weight: normal;
    }
    
    .header-left ::ng-deep h2 {
      margin: 0;
      font-size: 16px;
      font-weight: normal;
      opacity: 0.8;
    }
  `]
})
export class HeaderComponent implements OnInit, AfterContentInit, OnDestroy {
  @Input() title = 'Habitat Builder';
  @Input() user: UserInfo | null = null;
  @Input() username = '';
  @Input() avatarUrl = '';
  @Input() isSignedIn = false; // Accept isSignedIn as input property
  @Input() headerService?: HeaderService; // Service can be injected or provided by parent
  
  // Content child to detect if there is header-title content projected
  @ContentChild('[header-title]') headerTitleContent: any;
  
  // Track avatar image loading errors
  hasAvatarError = false;
  
  // Service injections
  private assetLoader = inject(AssetLoaderService);
  private authService = inject(AuthService);
  
  // Dynamically determine if we have a custom title
  get hasCustomTitle(): boolean {
    return !!this.headerTitleContent || !!this.headerService?.titleTemplateRef;
  }
  
  // Get title text from the service
  get serviceTitleText(): string {
    return this.headerService?.titleText || 'Habitat Builder';
  }
  
  @Output() logout = new EventEmitter<void>();
  @Output() signOut = new EventEmitter<string | undefined>();
  
  ngOnInit(): void {
    // Initialize with default avatar if none provided
    if (this.isSignedIn && !this.avatarUrl) {
      this.avatarUrl = 'assets/images/avatar.svg';
    }
    
    // Use service title if no input title is provided
    if (!this.title) {
      this.title = this.serviceTitleText;
    }
  }
  
  /**
   * After content init lifecycle hook to log if there is custom title content
   */
  ngAfterContentInit(): void {
    // Just log the status - actual value comes from getter
    console.log('HeaderComponent: Has custom title content:', this.hasCustomTitle);
  }
  
  ngOnDestroy(): void {
    // No cleanup needed yet
  }
  
  /**
   * Handle sign out action with optional return URL
   * @param returnUrl Optional URL to return to after next login
   */
  handleSignOut(returnUrl?: string): void {
    // Both events for compatibility with different parent components
    this.signOut.emit(returnUrl);
    this.logout.emit();
    
    console.log('HeaderComponent: Sign out initiated for user', this.username);
    
    // Clear avatar error state on logout
    this.hasAvatarError = false;
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
}