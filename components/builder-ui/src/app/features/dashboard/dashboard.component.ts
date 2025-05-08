import { Component, inject, signal, computed, Signal } from '@angular/core';
import { MatCardModule } from '@angular/material/card';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { RouterLink } from '@angular/router';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatChipsModule } from '@angular/material/chips';
import { NgClass } from '@angular/common';

import { AuthService } from '../../core/services/auth.service';
import { DashboardActivity, DashboardFeatureCard, DashboardStat } from './dashboard.model';

@Component({
  selector: 'app-dashboard',
  standalone: true,
  imports: [
    MatCardModule, 
    MatIconModule, 
    MatButtonModule, 
    RouterLink, 
    MatProgressSpinnerModule,
    MatChipsModule,
    NgClass
  ],
  template: `
    <div class="dashboard-container">
      <h1>Welcome to Habitat Builder</h1>
      <p class="intro-text">A platform for building, deploying, and managing applications.</p>
      
      <!-- Stats section using the new @for control flow -->
      <div class="stats-grid">
        @for (stat of dashboardStats(); track stat.title) {
          <mat-card class="stat-card" [ngClass]="stat.color">
            <mat-card-header>
              <mat-icon mat-card-avatar>{{stat.icon}}</mat-icon>
              <mat-card-title>{{stat.value}}</mat-card-title>
              <mat-card-subtitle>{{stat.title}}</mat-card-subtitle>
            </mat-card-header>
          </mat-card>
        }
      </div>
      
      <div class="dashboard-grid">
        <!-- Feature cards using @for and @if for conditional rendering -->
        @for (card of featureCards(); track card.title) {
          <mat-card class="dashboard-card">
            <mat-card-header>
              <mat-icon mat-card-avatar>{{card.icon}}</mat-icon>
              <mat-card-title>{{card.title}}</mat-card-title>
              <mat-card-subtitle>{{card.subtitle}}</mat-card-subtitle>
            </mat-card-header>
            <mat-card-content>
              <p>{{card.description}}</p>
              
              @if (card.requiresAuthentication && !isAuthenticated()) {
                <mat-chip-set>
                  <mat-chip highlighted color="warn">Login required</mat-chip>
                </mat-chip-set>
              }
            </mat-card-content>
            <mat-card-actions>
              <button mat-button color="primary" [routerLink]="card.routerLink">
                {{card.buttonText}}
              </button>
            </mat-card-actions>
          </mat-card>
        }

      </div>
      
      <!-- Using @defer for deferred loading of resource-intensive content -->
      @defer {
        <section class="activity-section">
          <h2>Recent Activity</h2>
          <div class="activity-container">
            @if (recentActivity().length) {
              @for (activity of recentActivity(); track activity.id) {
                <div class="activity-item">
                  <mat-icon class="activity-icon">{{activity.icon}}</mat-icon>
                  <div class="activity-details">
                    <strong>{{activity.title}}</strong>
                    <p>{{activity.description}}</p>
                    <small>{{activity.time}}</small>
                  </div>
                </div>
              }
            } @else {
              <p class="no-activity">No recent activity</p>
            }
          </div>
        </section>
      } @loading {
        <div class="loading-container">
          <mat-spinner diameter="40"></mat-spinner>
          <p>Loading recent activity...</p>
        </div>
      } @error {
        <div class="error-container">
          <mat-icon color="warn">error</mat-icon>
          <p>Failed to load recent activity</p>
        </div>
      }
    </div>
  `,
  styleUrls: ['./dashboard.component.scss']
})
export class DashboardComponent {
  private authService = inject(AuthService);
  
  // Using signals to manage component state
  private _dashboardStats = signal<DashboardStat[]>([
    { 
      title: 'Total Packages', 
      value: 1250, 
      icon: 'inventory_2', 
      description: 'Total packages across all origins', 
      color: 'blue-card' 
    },
    { 
      title: 'Active Origins', 
      value: 48, 
      icon: 'business', 
      description: 'Number of active origins',
      color: 'green-card' 
    },
    { 
      title: 'Recent Builds', 
      value: 342, 
      icon: 'build', 
      description: 'Builds in the last 7 days',
      color: 'purple-card' 
    },
    { 
      title: 'Users', 
      value: 256, 
      icon: 'people', 
      description: 'Registered user accounts',
      color: 'orange-card' 
    }
  ]);
  
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
    }
  ]);
  
  private _recentActivity = signal<DashboardActivity[]>([
    { id: 1, title: 'New package uploaded', description: 'core/nginx 1.21.4', time: '10 minutes ago', icon: 'upload' },
    { id: 2, title: 'Build completed', description: 'core/redis build #42', time: '25 minutes ago', icon: 'check_circle' },
    { id: 3, title: 'New origin created', description: 'acme-corp origin', time: '2 hours ago', icon: 'add_business' },
    { id: 4, title: 'Package promoted', description: 'core/postgresql promoted to stable', time: '5 hours ago', icon: 'trending_up' }
  ]);
  
  // Public read-only signals
  public dashboardStats = this._dashboardStats.asReadonly();
  public featureCards = this._featureCards.asReadonly();
  public recentActivity = this._recentActivity.asReadonly();
  
  // Computed signal that depends on the authService
  public isAuthenticated = computed(() => this.authService.isAuthenticated());
}
