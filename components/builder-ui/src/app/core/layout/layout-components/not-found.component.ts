import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

@Component({
  selector: 'app-not-found',
  standalone: true,
  imports: [RouterLink, MatButtonModule, MatIconModule],
  template: `
    <div class="not-found-container">
      <mat-icon class="not-found-icon">error_outline</mat-icon>
      <h1>404 - Page Not Found</h1>
      <p>The page you are looking for doesn't exist or has been moved.</p>
      <button mat-raised-button color="primary" routerLink="/">
        <mat-icon>home</mat-icon> Go to Home
      </button>
    </div>
  `,
  styles: [`
    .not-found-container {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100%;
      text-align: center;
      padding: 20px;
    }
    
    .not-found-icon {
      font-size: 80px;
      height: 80px;
      width: 80px;
      margin-bottom: 20px;
      color: #f44336;
    }
    
    h1 {
      font-size: 32px;
      margin-bottom: 16px;
    }
    
    p {
      font-size: 18px;
      margin-bottom: 32px;
      color: rgba(0, 0, 0, 0.6);
    }
  `]
})
export class NotFoundComponent {}
