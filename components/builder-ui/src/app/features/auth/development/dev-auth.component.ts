import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { MatCardModule } from '@angular/material/card';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatButtonModule } from '@angular/material/button';
import { MatSelectModule } from '@angular/material/select';
import { MatSlideToggleModule } from '@angular/material/slide-toggle';
import { MatIconModule } from '@angular/material/icon';
import { MatDividerModule } from '@angular/material/divider';
import { RouterModule } from '@angular/router';

import { environment } from '../../../../environments/environment';
import { DevAuthUtilsService } from './dev-auth-utils.service';
import { AuthService } from '../../../core/services/auth.service';

@Component({
  selector: 'app-dev-auth',
  standalone: true,
  imports: [
    CommonModule, 
    FormsModule,
    RouterModule,
    MatCardModule,
    MatFormFieldModule, 
    MatInputModule,
    MatButtonModule,
    MatSelectModule,
    MatSlideToggleModule,
    MatIconModule,
    MatDividerModule
  ],
  template: `
    <div class="dev-auth-container">
      <mat-card class="dev-auth-card">
        <mat-card-header>
          <mat-icon color="accent">bug_report</mat-icon>
          <mat-card-title>Development Authentication Utilities</mat-card-title>
          <mat-card-subtitle>Use this utility to test authentication flows in development</mat-card-subtitle>
        </mat-card-header>
        
        <mat-card-content>
          <div class="warning" *ngIf="!environment.useMocks">
            <mat-icon>warning</mat-icon>
            <span>
              Mock authentication is disabled! This utility requires <code>useMocks: true</code> in your environment.
            </span>
          </div>
          
          <div class="status-section">
            <h3>Current Authentication Status</h3>
            <div class="status-info" [ngClass]="authService.isAuthenticated() ? 'authenticated' : 'not-authenticated'">
              <mat-icon>{{ authService.isAuthenticated() ? 'check_circle' : 'cancel' }}</mat-icon>
              <span>{{ authService.isAuthenticated() ? 'Authenticated' : 'Not Authenticated' }}</span>
            </div>
            
            <div class="user-info" *ngIf="authService.isAuthenticated() && authService.currentUser() as user">
              <p><strong>User ID:</strong> {{ user.id }}</p>
              <p><strong>Name:</strong> {{ user.name }}</p>
              <p><strong>Email:</strong> {{ user.email }}</p>
              <p><strong>Role:</strong> {{ user.role }}</p>
              
              <div class="permissions" *ngIf="user.permissions">
                <p><strong>Permissions:</strong></p>
                <ul>
                  <li *ngFor="let permission of user.permissions">{{ permission }}</li>
                </ul>
              </div>
            </div>
          </div>
          
          <mat-divider></mat-divider>
          
          <div class="create-session-section" *ngIf="!authService.isAuthenticated()">
            <h3>Create Mock Session</h3>
            
            <form (submit)="createSession($event)">
              <mat-form-field appearance="outline">
                <mat-label>Name</mat-label>
                <input matInput [(ngModel)]="mockUser.name" name="name">
              </mat-form-field>
              
              <mat-form-field appearance="outline">
                <mat-label>Email</mat-label>
                <input matInput [(ngModel)]="mockUser.email" name="email">
              </mat-form-field>
              
              <mat-form-field appearance="outline">
                <mat-label>Role</mat-label>
                <mat-select [(ngModel)]="mockUser.role" name="role">
                  <mat-option value="contributor">Contributor</mat-option>
                  <mat-option value="admin">Admin</mat-option>
                  <mat-option value="owner">Owner</mat-option>
                </mat-select>
              </mat-form-field>
              
              <button mat-raised-button color="primary" type="submit">
                Create Mock Session
              </button>
            </form>
          </div>
          
          <div class="end-session-section" *ngIf="authService.isAuthenticated()">
            <button mat-raised-button color="warn" (click)="endSession()">
              End Mock Session
            </button>
          </div>
        </mat-card-content>
        
        <mat-divider></mat-divider>
        
        <mat-card-actions>
          <button mat-button routerLink="/sign-in">Go to Sign In</button>
          <button mat-button routerLink="/">Go to Dashboard</button>
        </mat-card-actions>
      </mat-card>
    </div>
  `,
  styles: [`
    .dev-auth-container {
      padding: 20px;
      max-width: 800px;
      margin: 0 auto;
      font-family: Roboto, "Helvetica Neue", sans-serif;
    }
    
    .dev-auth-card {
      margin-top: 20px;
    }
    
    mat-card-header mat-icon {
      font-size: 32px;
      height: 32px;
      width: 32px;
      margin-right: 16px;
    }
    
    .warning {
      display: flex;
      align-items: center;
      padding: 16px;
      background-color: #fff3e0;
      border-radius: 4px;
      margin: 16px 0;
    }
    
    .warning mat-icon {
      color: #ff9800;
      margin-right: 8px;
    }
    
    .status-section, .create-session-section {
      margin: 24px 0;
    }
    
    .status-info {
      display: flex;
      align-items: center;
      padding: 12px;
      border-radius: 4px;
      margin-bottom: 16px;
    }
    
    .status-info mat-icon {
      margin-right: 8px;
    }
    
    .authenticated {
      background-color: #e8f5e9;
      color: #2e7d32;
    }
    
    .not-authenticated {
      background-color: #fbe9e7;
      color: #d32f2f;
    }
    
    .user-info {
      padding: 16px;
      background-color: #f5f5f5;
      border-radius: 4px;
    }
    
    .user-info p {
      margin: 8px 0;
    }
    
    .permissions ul {
      margin: 8px 0;
      padding-left: 24px;
    }
    
    form {
      display: flex;
      flex-direction: column;
    }
    
    button {
      margin-top: 16px;
      align-self: flex-start;
    }
    
    .end-session-section {
      margin: 24px 0;
    }
    
    mat-divider {
      margin: 16px 0;
    }
  `]
})
export class DevAuthComponent {
  // Inject services
  authService = inject(AuthService);
  devAuthUtils = inject(DevAuthUtilsService);
  
  // Expose environment for template access
  environment = environment;
  
  // Form model for creating mock user
  mockUser = {
    name: 'Development User',
    email: 'dev@example.com',
    role: 'contributor'
  };
  
  createSession(event: Event) {
    event.preventDefault();
    
    // Create a mock session with the provided user details
    this.devAuthUtils.createMockSession({
      name: this.mockUser.name,
      email: this.mockUser.email,
      role: this.mockUser.role
    });
  }
  
  endSession() {
    this.devAuthUtils.endMockSession();
  }
}
