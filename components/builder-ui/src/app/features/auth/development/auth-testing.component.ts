// Optional component for testing in development environment - can be removed in production
import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatButtonModule } from '@angular/material/button';
import { MatCardModule } from '@angular/material/card';
import { MatIconModule } from '@angular/material/icon';
import { Router } from '@angular/router';
import { environment } from '../../../../environments/environment';

@Component({
  selector: 'app-auth-testing',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatCardModule, MatIconModule],
  template: `
    <div class="testing-container" *ngIf="!environment.production">
      <mat-card>
        <mat-card-header>
          <mat-icon color="accent">science</mat-icon>
          <mat-card-title>Auth Testing Tools</mat-card-title>
          <mat-card-subtitle>Testing tools for development only</mat-card-subtitle>
        </mat-card-header>
        
        <mat-card-content>
          <p>These tools are for development and testing purposes only.</p>
          <p>The complete sign-in authentication flow consists of:</p>
          
          <ol>
            <li>User clicks "Sign In with GitHub" on the sign-in page</li>
            <li>EULA popup appears for confirmation</li>
            <li>User is redirected to GitHub (or mock auth in development)</li>
            <li>GitHub redirects back to callback URL with auth code</li>
            <li>Backend exchanges code for token</li>
            <li>User is logged in and redirected to the home page</li>
          </ol>
        </mat-card-content>
        
        <mat-card-actions>
          <button mat-raised-button color="primary" (click)="navigateToSignIn()">
            Test Sign-In Flow
          </button>
          <button mat-raised-button color="accent" (click)="navigateToDevTools()">
            Advanced Dev Tools
          </button>
        </mat-card-actions>
      </mat-card>
    </div>
  `,
  styles: [`
    .testing-container {
      max-width: 800px;
      margin: 40px auto;
      padding: 0 20px;
    }
    
    mat-card-header mat-icon {
      font-size: 32px;
      height: 32px;
      width: 32px;
      margin-right: 16px;
    }
    
    ol {
      margin: 20px 0;
      padding-left: 20px;
    }
    
    li {
      margin-bottom: 8px;
    }
    
    button {
      margin-right: 8px;
    }
  `]
})
export class AuthTestingComponent {
  router = inject(Router);
  
  // Expose environment for template access
  environment = environment;
  
  navigateToSignIn() {
    this.router.navigate(['/sign-in']);
  }
  
  navigateToDevTools() {
    this.router.navigate(['/auth/dev']);
  }
}
