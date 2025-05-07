import { Component } from '@angular/core';
import { MatCardModule } from '@angular/material/card';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-dashboard',
  standalone: true,
  imports: [MatCardModule, MatIconModule, MatButtonModule, RouterLink],
  template: `
    <div class="dashboard-container">
      <h1>Welcome to Habitat Builder</h1>
      <p class="intro-text">A platform for building, deploying, and managing applications.</p>
      
      <div class="dashboard-grid">
        <mat-card class="dashboard-card">
          <mat-card-header>
            <mat-icon mat-card-avatar>inventory_2</mat-icon>
            <mat-card-title>Packages</mat-card-title>
            <mat-card-subtitle>Browse and manage packages</mat-card-subtitle>
          </mat-card-header>
          <mat-card-content>
            <p>
              Explore packages across all origins or search for specific packages.
            </p>
          </mat-card-content>
          <mat-card-actions>
            <button mat-button color="primary" routerLink="/pkgs">
              VIEW PACKAGES
            </button>
          </mat-card-actions>
        </mat-card>

        <mat-card class="dashboard-card">
          <mat-card-header>
            <mat-icon mat-card-avatar>business</mat-icon>
            <mat-card-title>Origins</mat-card-title>
            <mat-card-subtitle>Manage origins and keys</mat-card-subtitle>
          </mat-card-header>
          <mat-card-content>
            <p>
              Create and manage origins, invite members, and handle origin keys.
            </p>
          </mat-card-content>
          <mat-card-actions>
            <button mat-button color="primary" routerLink="/origins">
              VIEW ORIGINS
            </button>
          </mat-card-actions>
        </mat-card>

        <mat-card class="dashboard-card">
          <mat-card-header>
            <mat-icon mat-card-avatar>build</mat-icon>
            <mat-card-title>Builds</mat-card-title>
            <mat-card-subtitle>Track builds and build history</mat-card-subtitle>
          </mat-card-header>
          <mat-card-content>
            <p>
              View build history, track ongoing builds, and manage build configuration.
            </p>
          </mat-card-content>
          <mat-card-actions>
            <button mat-button color="primary" routerLink="/builds">
              VIEW BUILDS
            </button>
          </mat-card-actions>
        </mat-card>
      </div>
    </div>
  `,
  styles: [`
    .dashboard-container {
      max-width: 1200px;
      margin: 0 auto;
      padding: 20px;
    }
    
    .intro-text {
      font-size: 18px;
      margin-bottom: 32px;
      color: rgba(0, 0, 0, 0.6);
    }
    
    .dashboard-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
      gap: 24px;
    }
    
    .dashboard-card {
      height: 100%;
    }
    
    mat-card-content {
      padding: 16px 0;
    }
  `]
})
export class DashboardComponent {}
