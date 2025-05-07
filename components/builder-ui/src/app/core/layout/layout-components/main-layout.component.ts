import { Component, OnInit, ViewChild } from '@angular/core';
import { Router, RouterOutlet, RouterLink, RouterLinkActive } from '@angular/router';
import { CommonModule } from '@angular/common';
import { MatSidenavModule, MatSidenav } from '@angular/material/sidenav';
import { MatToolbarModule } from '@angular/material/toolbar';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { MatListModule } from '@angular/material/list';
import { MatMenuModule } from '@angular/material/menu';
import { MatDividerModule } from '@angular/material/divider';
import { MatTooltipModule } from '@angular/material/tooltip';

import { LoadingSpinnerComponent } from '../../../shared/components/loading-spinner/loading-spinner.component';
import { AuthService } from '../../services/auth.service';
import { LoadingService } from '../../services/loading.service';

export interface User {
  name: string;
  email: string;
}

@Component({
  selector: 'app-main-layout',
  standalone: true,
  imports: [
    CommonModule,
    RouterOutlet,
    RouterLink,
    RouterLinkActive,
    MatSidenavModule,
    MatToolbarModule,
    MatIconModule,
    MatButtonModule,
    MatListModule,
    MatMenuModule,
    MatDividerModule,
    MatTooltipModule,
    LoadingSpinnerComponent
  ],
  template: `
    <div class="app-container">
      <!-- Main application toolbar -->
      <mat-toolbar color="primary" class="toolbar">
        <button mat-icon-button (click)="toggleSidenav()" matTooltip="Toggle menu">
          <mat-icon>menu</mat-icon>
        </button>
        
        <span class="app-title">Habitat Builder</span>
        
        <span class="spacer"></span>
        
        <!-- Notifications icon -->
        <button mat-icon-button matTooltip="Notifications">
          <mat-icon>notifications</mat-icon>
        </button>
        
        <!-- User menu -->
        <button mat-icon-button [matMenuTriggerFor]="userMenu" matTooltip="User menu">
          <mat-icon>account_circle</mat-icon>
        </button>
        <mat-menu #userMenu="matMenu">
          <div class="user-info" *ngIf="currentUser">
            <div class="user-name">{{ currentUser.name }}</div>
            <div class="user-email">{{ currentUser.email }}</div>
          </div>
          <mat-divider></mat-divider>
          <button mat-menu-item routerLink="/profile">
            <mat-icon>person</mat-icon>
            <span>Profile</span>
          </button>
          <button mat-menu-item (click)="logout()">
            <mat-icon>logout</mat-icon>
            <span>Logout</span>
          </button>
        </mat-menu>
      </mat-toolbar>

      <!-- Main content with sidebar navigation -->
      <mat-sidenav-container class="sidenav-container">
        <mat-sidenav #sidenav [opened]="sidenavOpened" [mode]="sidenavMode" class="sidenav">
          <div class="logo-container">
            <img src="/assets/images/habitat-logo.svg" alt="Habitat Logo" class="logo">
          </div>
          
          <mat-nav-list>
            <a mat-list-item routerLink="/dashboard" routerLinkActive="active-link">
              <mat-icon matListItemIcon>dashboard</mat-icon>
              <span matListItemTitle>Dashboard</span>
            </a>
            <a mat-list-item routerLink="/pkgs" routerLinkActive="active-link">
              <mat-icon matListItemIcon>inventory_2</mat-icon>
              <span matListItemTitle>Packages</span>
            </a>
            <a mat-list-item routerLink="/origins" routerLinkActive="active-link">
              <mat-icon matListItemIcon>business</mat-icon>
              <span matListItemTitle>Origins</span>
            </a>
            <a mat-list-item routerLink="/builds" routerLinkActive="active-link">
              <mat-icon matListItemIcon>build</mat-icon>
              <span matListItemTitle>Builds</span>
            </a>
            <a mat-list-item routerLink="/projects" routerLinkActive="active-link">
              <mat-icon matListItemIcon>code</mat-icon>
              <span matListItemTitle>Projects</span>
            </a>
          </mat-nav-list>
        </mat-sidenav>
        
        <mat-sidenav-content class="content">
          <!-- Global loading indicator -->
          <app-loading-spinner 
            *ngIf="loadingService.isLoading()" 
            [overlay]="true"
            [message]="loadingService.message() || ''">
          </app-loading-spinner>
          
          <div class="content-container">
            <router-outlet></router-outlet>
          </div>
        </mat-sidenav-content>
      </mat-sidenav-container>
    </div>
  `,
  styles: [`
    .app-container {
      display: flex;
      flex-direction: column;
      height: 100vh;
    }
    
    .toolbar {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      z-index: 2;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    }
    
    .app-title {
      margin-left: 16px;
      font-weight: 500;
    }
    
    .spacer {
      flex: 1 1 auto;
    }
    
    .sidenav-container {
      flex: 1;
      margin-top: 64px;
    }
    
    .sidenav {
      width: 250px;
      box-shadow: 2px 0 4px rgba(0, 0, 0, 0.1);
    }
    
    .content {
      background-color: #f5f5f5;
    }
    
    .content-container {
      padding: 24px;
      max-width: 1400px;
      margin: 0 auto;
    }
    
    .logo-container {
      padding: 16px;
      text-align: center;
      border-bottom: 1px solid rgba(0, 0, 0, 0.12);
    }
    
    .logo {
      height: 40px;
    }
    
    .active-link {
      background-color: rgba(0, 0, 0, 0.04);
      color: var(--primary-color);
      border-left: 4px solid var(--primary-color);
    }
    
    .user-info {
      padding: 16px;
      min-width: 200px;
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
      .sidenav-container {
        margin-top: 56px;
      }
      
      .content-container {
        padding: 16px;
      }
    }
  `]
})
export class MainLayoutComponent implements OnInit {
  @ViewChild('sidenav') sidenav!: MatSidenav;
  
  sidenavOpened = true;
  sidenavMode: 'side' | 'over' = 'side';
  currentUser: User | null = null;
  
  constructor(
    private authService: AuthService,
    public loadingService: LoadingService, // Public so it can be used in the template
    private router: Router
  ) {}
  
  ngOnInit(): void {
    // Get current user
    this.currentUser = this.authService.currentUser();
    
    // With signals, we just need to use the signal directly in the template
    // No need to subscribe, just set up property getters

    // Adjust sidenav based on screen size
    this.checkScreenSize();
    window.addEventListener('resize', () => this.checkScreenSize());
  }
  
  toggleSidenav(): void {
    this.sidenav.toggle();
  }
  
  logout(): void {
    this.authService.logout();
    this.router.navigate(['/auth/login']);
  }
  
  private checkScreenSize(): void {
    if (window.innerWidth < 992) {
      this.sidenavMode = 'over';
      this.sidenavOpened = false;
    } else {
      this.sidenavMode = 'side';
      this.sidenavOpened = true;
    }
  }
}
