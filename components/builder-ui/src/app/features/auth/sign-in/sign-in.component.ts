import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';
import { MatButtonModule } from '@angular/material/button';
import { MatCardModule } from '@angular/material/card';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';

@Component({
  selector: 'app-sign-in',
  standalone: true,
  imports: [RouterLink, MatButtonModule, MatCardModule, MatFormFieldModule, MatInputModule],
  template: `
    <div class="sign-in-container">
      <mat-card class="sign-in-card">
        <mat-card-header>
          <mat-card-title>Sign In</mat-card-title>
          <mat-card-subtitle>Log in to Habitat Builder</mat-card-subtitle>
        </mat-card-header>
        <mat-card-content>
          <p class="message">This is a placeholder for the Sign In feature. It will be implemented in future phases.</p>
        </mat-card-content>
        <mat-card-actions>
          <button mat-raised-button color="primary" routerLink="/">Go to Dashboard</button>
        </mat-card-actions>
      </mat-card>
    </div>
  `,
  styles: [`
    .sign-in-container {
      display: flex;
      justify-content: center;
      align-items: center;
      height: 100vh;
      background-color: #f5f5f5;
    }
    
    .sign-in-card {
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
export class SignInComponent {}
