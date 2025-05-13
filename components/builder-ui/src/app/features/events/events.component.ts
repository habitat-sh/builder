import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-events',
  standalone: true,
  imports: [CommonModule, MatCardModule, MatButtonModule, RouterLink],
  template: `
    <div class="page-container">
      <div class="page-header">
        <h1>Events</h1>
      </div>
      <div class="page-content">
        <mat-card>
          <mat-card-content>
            <p>This is the Events page. Content will be implemented in future iterations.</p>
            <div class="actions">
              <a mat-button color="primary" [routerLink]="['/events/saas']">View SaaS Events</a>
            </div>
          </mat-card-content>
        </mat-card>
      </div>
    </div>
  `,
  styles: [`
    .page-container {
      padding: 20px;
    }
    .page-header {
      margin-bottom: 20px;
    }
    .page-content {
      max-width: 1200px;
    }
    .actions {
      margin-top: 20px;
    }
  `]
})
export class EventsComponent { }
