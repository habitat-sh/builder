import { Component, OnInit, inject } from '@angular/core';
import { RouterLink, Router, ActivatedRoute } from '@angular/router';
import { MatButtonModule } from '@angular/material/button';
import { MatCardModule } from '@angular/material/card';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { CommonModule } from '@angular/common';
import { AuthService } from '../../../core/services/auth.service';

@Component({
  selector: 'app-oauth-token',
  standalone: true,
  imports: [CommonModule, RouterLink, MatButtonModule, MatCardModule, MatProgressSpinnerModule],
  template: `
    <div class="oauth-token-container">
      <mat-card class="oauth-token-card">
        <mat-card-header>
          <mat-card-title>OAuth Authentication</mat-card-title>
          <mat-card-subtitle>Processing authentication token</mat-card-subtitle>
        </mat-card-header>
        <mat-card-content>
          <div *ngIf="isLoading" class="loading">
            <mat-spinner diameter="40"></mat-spinner>
            <p>Processing your authentication...</p>
          </div>
          <div *ngIf="error" class="error">
            <p>{{ error }}</p>
          </div>
          <div *ngIf="success" class="success">
            <p>Successfully authenticated!</p>
          </div>
        </mat-card-content>
        <mat-card-actions>
          <button *ngIf="!isLoading" mat-raised-button color="primary" routerLink="/">Go to Dashboard</button>
          <button *ngIf="error" mat-stroked-button routerLink="/sign-in">Try Again</button>
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
    
    .loading, .error, .success {
      margin: 20px 0;
      text-align: center;
    }
    
    .loading p {
      margin-top: 16px;
    }
    
    .error {
      color: #d32f2f;
    }
    
    .success {
      color: #2e7d32;
    }
    
    mat-card-actions {
      display: flex;
      justify-content: center;
    }
  `]
})
export class OAuthTokenComponent implements OnInit {
  private authService = inject(AuthService);
  private router = inject(Router);
  private route = inject(ActivatedRoute);
  
  isLoading = true;
  error = '';
  success = false;
  
  ngOnInit(): void {
    // Get token from URL hash fragment or query param
    const params = new URLSearchParams(window.location.hash.substring(1) || window.location.search.substring(1));
    const token = params.get('token');
    
    if (!token) {
      this.isLoading = false;
      this.error = 'No authentication token provided';
      return;
    }
    
    // Process the token
    this.authService.handleOAuthCallback(token).subscribe({
      next: () => {
        this.isLoading = false;
        this.success = true;
        setTimeout(() => {
          const redirectUrl = this.authService.getAndClearRedirectUrl() || '/';
          this.router.navigateByUrl(redirectUrl);
        }, 1000);
      },
      error: (error) => {
        this.isLoading = false;
        this.error = 'Authentication failed: ' + (error.message || 'Unknown error');
        console.error('Token processing error', error);
      }
    });
  }
}
