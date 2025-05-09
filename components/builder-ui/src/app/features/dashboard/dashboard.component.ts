// filepath: /Users/psajja/Workspace/habitat-sh/builder/components/builder-ui/src/app/features/dashboard/dashboard.component.ts
import { Component, inject, signal, computed } from '@angular/core';
import { MatCardModule } from '@angular/material/card';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { RouterLink } from '@angular/router';
import { MatChipsModule } from '@angular/material/chips';
import { NgClass } from '@angular/common';

import { AuthService } from '../../core/services/auth.service';
import { ConfigService } from '../../core/services/config.service';
import { DashboardFeatureCard } from './dashboard.model';

@Component({
  selector: 'app-dashboard',
  standalone: true,
  imports: [
    MatCardModule, 
    MatIconModule, 
    MatButtonModule, 
    RouterLink, 
    MatChipsModule,
    NgClass
  ],
  template: `
    <div class="container dashboard-container">
      <h1 class="page-title">Welcome to Habitat Builder</h1>
      <p class="intro-text">A platform for building, deploying, and managing applications with native integration to Chef Habitat.</p>
      
      <div class="dashboard-grid">
        <!-- Feature cards using @for and @if for conditional rendering -->
        @for (card of featureCards(); track card.title) {
          <mat-card class="dashboard-card" [ngClass]="{'restricted': card.requiresAuthentication && !isAuthenticated()}">
            <mat-card-header>
              <mat-icon mat-card-avatar>{{card.icon}}</mat-icon>
              <mat-card-title>{{card.title}}</mat-card-title>
              <mat-card-subtitle>{{card.subtitle}}</mat-card-subtitle>
            </mat-card-header>
            <mat-card-content>
              <p>{{card.description}}</p>
              
              @if (card.requiresAuthentication && !isAuthenticated()) {
                <mat-chip-set>
                  <mat-chip color="warn">Login required</mat-chip>
                </mat-chip-set>
              }
            </mat-card-content>
            <mat-card-actions>
              <button mat-button color="primary" [routerLink]="card.routerLink" [disabled]="card.requiresAuthentication && !isAuthenticated()">
                {{card.buttonText}}
              </button>
            </mat-card-actions>
          </mat-card>
        }
      </div>
      
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
        </div>
      </div>
    </div>
  `,
  styleUrls: ['./dashboard.component.scss']
})
export class DashboardComponent {
  private authService = inject(AuthService);
  private configService = inject(ConfigService);
  
  private _featureCards = signal<DashboardFeatureCard[]>([
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
      title: 'Builds',
      subtitle: 'View and manage builds',
      description: 'Monitor build status and manage your build processes.',
      icon: 'build',
      routerLink: '/builds',
      buttonText: 'VIEW BUILDS',
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
    },
    {
      title: 'Events',
      subtitle: 'View system events',
      description: 'View and monitor system events and notifications.',
      icon: 'event',
      routerLink: '/events',
      buttonText: 'VIEW EVENTS',
      requiresAuthentication: true
    }
  ]);
  
  // Computed signal that filters feature cards based on feature flags
  public featureCards = computed(() => {
    return this._featureCards().filter(card => {
      // Filter out Events card if events feature is disabled
      if (card.title === 'Events' && !this.configService.isFeatureEnabled('enableEvents')) {
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
}
