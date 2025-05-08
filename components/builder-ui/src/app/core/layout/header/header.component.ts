import { Component, Input, Output, EventEmitter } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { MatToolbarModule } from '@angular/material/toolbar';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatMenuModule } from '@angular/material/menu';
import { MatDividerModule } from '@angular/material/divider';
import { MatTooltipModule } from '@angular/material/tooltip';

export interface UserInfo {
  name: string;
  email: string;
  avatar?: string;
}

@Component({
  selector: 'app-header',
  standalone: true,
  imports: [
    CommonModule,
    RouterLink,
    MatToolbarModule,
    MatButtonModule,
    MatIconModule,
    MatMenuModule,
    MatDividerModule,
    MatTooltipModule
  ],
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
            src="/assets/images/habitat-logo-white.svg" 
            alt="Habitat Logo" 
            class="logo"
            routerLink="/">
        </div>
        
        <h1 class="title">{{ title }}</h1>
      </div>
      
      <div class="spacer"></div>
      
      <div class="header-right">
        <ng-content select="[header-actions]"></ng-content>
        
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
          *ngIf="user">
          <mat-icon>account_circle</mat-icon>
        </button>
        <mat-menu #userMenu="matMenu">
          <div class="user-info">
            <div class="user-avatar" *ngIf="user?.avatar">
              <img [src]="user?.avatar || ''" [alt]="user?.name || 'User'">
            </div>
            <div class="user-details">
              <div class="user-name">{{ user?.name || username }}</div>
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
          
          <mat-divider></mat-divider>
          
          <button mat-menu-item (click)="logout.emit()">
            <mat-icon>exit_to_app</mat-icon>
            <span>Log Out</span>
          </button>
        </mat-menu>
      </div>
    </mat-toolbar>
  `,
  styles: [`
    .header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0 16px;
      height: 64px;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    }
    
    .header-left {
      display: flex;
      align-items: center;
    }
    
    .header-right {
      display: flex;
      align-items: center;
    }
    
    .logo-container {
      height: 40px;
      margin-right: 16px;
      cursor: pointer;
    }
    
    .logo {
      height: 100%;
    }
    
    .title {
      font-size: 20px;
      font-weight: 500;
      margin: 0;
    }
    
    .spacer {
      flex: 1 1 auto;
    }
    
    .action-button {
      margin-left: 8px;
    }
    
    .user-info {
      display: flex;
      padding: 16px;
      min-width: 200px;
      align-items: center;
    }
    
    .user-avatar {
      width: 40px;
      height: 40px;
      border-radius: 50%;
      overflow: hidden;
      margin-right: 12px;
    }
    
    .user-avatar img {
      width: 100%;
      height: 100%;
      object-fit: cover;
    }
    
    .user-name {
      font-weight: 500;
      margin-bottom: 4px;
    }
    
    .user-email {
      font-size: 12px;
      color: rgba(0, 0, 0, 0.6);
    }
    
    @media (max-width: 768px) {
      .header {
        height: 56px;
      }
      
      .title {
        font-size: 18px;
      }
      
      .logo-container {
        height: 32px;
      }
    }
  `]
})
export class HeaderComponent {
  @Input() title = 'Habitat Builder';
  @Input() showLogo = true;
  @Input() user: UserInfo | null = null;
  @Input() isSignedIn = false;
  @Input() username = '';
  @Input() avatarUrl = '';
  
  @Output() toggleSideNav = new EventEmitter<void>();
  @Output() search = new EventEmitter<void>();
  @Output() logout = new EventEmitter<void>();
  @Output() signOut = new EventEmitter<void>();
  
  onSearchClick(): void {
    this.search.emit();
  }
  
  handleSignOut(): void {
    this.signOut.emit();
    this.logout.emit(); // For backward compatibility
  }
}
