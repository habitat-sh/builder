import { Component, inject, signal, computed, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';
import { MatIconModule, MatIconRegistry } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { RouterLink } from '@angular/router';
import { MatChipsModule } from '@angular/material/chips';
import { MatProgressBarModule } from '@angular/material/progress-bar';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { DomSanitizer } from '@angular/platform-browser';

import { AuthService } from '../../core/services/auth.service';
import { ConfigService } from '../../core/services/config.service';
import { HomeFeatureCard } from './home.model';

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [
    CommonModule,
    MatCardModule, 
    MatIconModule, 
    MatButtonModule, 
    RouterLink, 
    MatChipsModule,
    MatProgressBarModule,
    MatProgressSpinnerModule
  ],
  template: `
    <div class="container home-container">
      <h1 class="page-title">Welcome to Habitat Builder</h1>
      <p class="intro-text">
        @if (!isAuthenticated()) {
          Browse packages or sign in to access all features of Chef Habitat's platform for building, deploying, and managing applications.
        } @else {
          A platform for building, deploying, and managing applications with native integration to Chef Habitat.
        }
      </p>
      
      <!-- User profile section removed as per requirements -->
      
      <!-- Stats summary section removed as per requirements -->
      
      <!-- Login card shown only when user is not authenticated -->
      @if (!isAuthenticated()) {
        <div class="login-section">
          <mat-card class="login-card">
            <div class="login-card-accent"></div>
            <mat-card-header>
              <mat-icon mat-card-avatar>account_circle</mat-icon>
              <mat-card-title>Sign In with GitHub</mat-card-title>
              <mat-card-subtitle>Access all features and manage your packages</mat-card-subtitle>
            </mat-card-header>
            <mat-card-content>
              <p>Sign in with GitHub to create origins, upload packages, and manage your Habitat content.</p>
              <ul class="feature-list">
                <li><mat-icon class="feature-icon">check_circle</mat-icon> Create and manage origins</li>
                <li><mat-icon class="feature-icon">check_circle</mat-icon> Upload and build packages</li>
                <li><mat-icon class="feature-icon">check_circle</mat-icon> Set up integrations with GitHub</li>
                <li><mat-icon class="feature-icon">check_circle</mat-icon> Access package analytics</li>
              </ul>
            </mat-card-content>
            <mat-card-actions>
              <a mat-raised-button color="primary" class="github-signin-button" routerLink="/sign-in">
                <mat-icon svgIcon="github"></mat-icon>
                <span>Sign In with GitHub</span>
              </a>
            </mat-card-actions>
          </mat-card>
        </div>
      }
      
      <!-- Feature cards only shown when user is logged in -->
      @if (isAuthenticated()) {
        <div class="home-grid">
          @for (card of featureCards(); track card.title) {
            <mat-card class="home-card">
              <mat-card-header>
                <mat-icon mat-card-avatar>{{card.icon}}</mat-icon>
                <mat-card-title>{{card.title}}</mat-card-title>
                <mat-card-subtitle>{{card.subtitle}}</mat-card-subtitle>
              </mat-card-header>
              <mat-card-content>
                <p>{{card.description}}</p>
              </mat-card-content>
              <mat-card-actions>
                <button mat-button color="primary" [routerLink]="card.routerLink">
                  {{card.buttonText}}
                </button>
              </mat-card-actions>
            </mat-card>
          }
        </div>
      }
      
      <!-- Authentication status bar removed as per requirements -->
      
      <div class="resources-section">
        <h2>Additional Resources</h2>
        <div class="resources-grid">
          <a [href]="getUrl('docs')" target="_blank" class="resource-link">
            <mat-icon>library_books</mat-icon>
            <span>Documentation</span>
          </a>
          <a [href]="getUrl('tutorials')" target="_blank" class="resource-link">
            <mat-icon>school</mat-icon>
            <span>Tutorials</span>
          </a>
          <a [href]="getUrl('source')" target="_blank" class="resource-link">
            <mat-icon>code</mat-icon>
            <span>GitHub Repository</span>
          </a>
          <a [href]="getUrl('slack')" target="_blank" class="resource-link">
            <mat-icon>chat</mat-icon>
            <span>Community Slack</span>
          </a>
          <a [href]="getUrl('download')" target="_blank" class="resource-link">
            <mat-icon>download</mat-icon>
            <span>Download Habitat</span>
          </a>
        </div>
      </div>
    </div>
  `,
  styleUrls: ['./home.component.scss']
})
export class HomeComponent implements OnInit, OnDestroy {
  private authService = inject(AuthService);
  private configService = inject(ConfigService);
  private matIconRegistry = inject(MatIconRegistry);
  private domSanitizer = inject(DomSanitizer);
  
  // State signals
  private _loading = signal(false);
  
  // User data signal
  private _userData = signal<any>(null);

  isLoading = computed(() => this._loading());
  userData = computed(() => this._userData());
  
  ngOnInit() {
    // Register GitHub icon for the sign-in button
    this.matIconRegistry.addSvgIcon(
      'github',
      this.domSanitizer.bypassSecurityTrustResourceUrl('assets/images/icons/github.svg')
    );
    
    // Load user data if authenticated (stats loading removed)
    if (this.isAuthenticated()) {
      this.loadUserData();
    } else {
      // Set loading to false since we won't load stats
      this._loading.set(false);
    }
  }
  
  ngOnDestroy() {
    // Cleanup any subscriptions if needed
  }
  
  /**
   * Load user data from the auth service
   */
  private loadUserData() {
    const user = this.authService.currentUser();
    if (user) {
      this._userData.set(user);
      this._loading.set(false);
    }
  }
  
  private _featureCards = signal<HomeFeatureCard[]>([
    {
      title: 'Packages',
      subtitle: 'Browse and manage packages',
      description: 'Explore packages across all origins or search for specific packages.',
      icon: 'inventory_2',
      routerLink: '/pkgs',
      buttonText: 'VIEW PACKAGES',
      requiresAuthentication: false
    },
    {
      title: 'Origins',
      subtitle: 'Manage origins and keys',
      description: 'Create and manage origin keys, members, and integrations.',
      icon: 'business',
      routerLink: '/origins',
      buttonText: 'MANAGE ORIGINS',
      requiresAuthentication: true
    },
    {
      title: 'Profile',
      subtitle: 'Manage your account',
      description: 'Update your profile and manage your personal access tokens.',
      icon: 'person',
      routerLink: '/profile',
      buttonText: 'MY PROFILE',
      requiresAuthentication: true
    }
    // Events tiles removed from home but still accessible via nav bar with feature flags
  ]);
  
  // Computed signal that filters feature cards based on authentication status and feature flags
  public featureCards = computed(() => {
    // Hide all feature cards when user is not logged in
    if (!this.isAuthenticated()) {
      return [];
    }

    return this._featureCards().filter(card => {
      // Filter by feature flag if present
      if (card.featureFlag && !this.configService.isFeatureEnabled(card.featureFlag)) {
        return false;
      }
      
      return true;
    });
  });
  
  // Computed signal that depends on the authService
  public isAuthenticated = computed(() => this.authService.isAuthenticated());

  // Helper method to get URLs from config service
  getUrl(key: string): string {
    return this.configService.getUrl(key);
  }
  
  /**
   * Log the user out
   */
  logout() {
    this.authService.logout();
  }
}
