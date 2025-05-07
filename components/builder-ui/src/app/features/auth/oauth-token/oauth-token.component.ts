import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';
import { MatButtonModule } from '@angular/material/button';
import { MatCardModule } from '@angular/material/card';

@Component({
  selector: 'app-oauth-token',
  standalone: true,
  imports: [RouterLink, MatButtonModule, MatCardModule],
  template: `
    <div class="oauth-token-container">
      <mat-card class="oauth-token-card">
        <mat-card-header>
          <mat-card-title>OAuth Authentication</mat-card-title>
          <mat-card-subtitle>Processing authentication token</mat-card-subtitle>
        </mat-card-header>
        <mat-card-content>
          <p class="message">This is a placeholder for the OAuth Token processing feature. It will be implemented in future phases.</p>
        </mat-card-content>
        <mat-card-actions>
          <button mat-raised-button color="primary" routerLink="/">Go to Dashboard</button>
        </mat-card-actions>
      </mat-card>
    </div>
  `,
  styles: [`
    .oauth-token-container {
      display: flex;
      justify-content: center;
      align-items: center;
      height: 100vh;
      background-color: #f5f5f5;
    }
    
    .oauth-token-card {
      width: 100%;
      max-width: 400px;
      padding: 20px;
    }
    
    .message {
      margin: 20px 0;
    }
    
    mat-card-actions {
      display: flex;
      justify-content: center;
    }
  `]
})
export class OAuthTokenComponent {}
