import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router } from '@angular/router';
import { MatIconModule } from '@angular/material/icon';
import { JobStatusIconComponent } from '../job-status-icon/job-status-icon.component';
import { packageString } from '../../utils/event-util';
import { Event } from '../../models/event.model';

@Component({
  selector: 'app-events-results',
  standalone: true,
  imports: [
    CommonModule,
    MatIconModule,
    JobStatusIconComponent
  ],
  template: `
    <div class="results-component">
      <div class="results-table-container">
        <table class="events-table">
          <thead>
            <tr>
              <th>Status</th>
              <th>Origin</th>
              <th>Channel</th>
              <th>Package Ident</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            <tr *ngIf="noEvents" class="no-results">
              <td colspan="5">No events found.</td>
            </tr>
            <tr *ngFor="let event of events" (click)="onClick(event)" class="event-row">
              <td class="status-column">
                <app-job-status-icon [status]="event.operation"></app-job-status-icon>
              </td>
              <td class="origin-column">
                {{ event.origin }}
              </td>
              <td class="channel-column">
                {{ event.channel }}
              </td>
              <td class="package-column">
                {{ getPackageString(event) }}
              </td>
              <td class="date-column">
                {{ dateFor(event.created_at) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      
      <div *ngIf="errorMessage" class="error-message">
        <mat-icon color="warn">error</mat-icon>
        <p>{{ errorMessage }}</p>
      </div>
    </div>
  `,
  styles: [`
    .results-component {
      display: block;
      width: 100%;
    }
    
    .results-table-container {
      overflow-x: auto;
      width: 100%;
    }
    
    .events-table {
      width: 100%;
      border-collapse: collapse;
      
      th, td {
        padding: 12px;
        text-align: left;
        border-bottom: 1px solid #eee;
      }
      
      th {
        font-weight: 500;
        color: #333;
        white-space: nowrap;
      }
      
      .event-row {
        cursor: pointer;
        transition: background-color 0.2s;
        
        &:hover {
          background-color: #f5f5f5;
        }
        
        .status-column {
          width: 60px;
        }
        
        .origin-column, .channel-column {
          white-space: nowrap;
        }
        
        .package-column {
          max-width: 400px;
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }
        
        .date-column {
          white-space: nowrap;
          text-align: right;
        }
      }
    }
    
    .no-results {
      text-align: center;
      color: #666;
      
      td {
        padding: 32px 16px;
      }
    }
    
    .error-message {
      display: flex;
      align-items: center;
      padding: 16px;
      margin-top: 16px;
      background-color: #fff8f7;
      border-left: 4px solid #d32f2f;
      border-radius: 4px;
      
      mat-icon {
        margin-right: 8px;
      }
      
      p {
        margin: 0;
        color: #666;
      }
    }
  `]
})
export class EventsResultsComponent {
  @Input() errorMessage = '';
  @Input() noEvents = false;
  @Input() events: Event[] = [];
  @Input() fromSaas = false;

  constructor(private router: Router) {}

  /**
   * Handle click on an event row
   */
  onClick(event: Event): void {
    if (this.fromSaas) {
      // Open in a new tab for SaaS events
      const url = `https://bldr.habitat.sh/#/pkgs/${event.origin}/${event.package_ident.name}/${event.package_ident.version}/${event.package_ident.release}`;
      window.open(url, '_blank');
    } else {
      // Navigate within the app for regular events
      this.router.navigate([
        '/pkgs', 
        event.origin, 
        event.package_ident.name, 
        event.package_ident.version, 
        event.package_ident.release
      ]);
    }
  }

  /**
   * Format the package identifier as a string
   */
  getPackageString(event: Event): string {
    return packageString(event.package_ident);
  }

  /**
   * Format the date in relative time (e.g. "2 hours ago")
   */
  dateFor(timestamp: string): string {
    const date = new Date(timestamp);
    const now = new Date();
    const diffSeconds = Math.floor((now.getTime() - date.getTime()) / 1000);
    
    if (diffSeconds < 60) {
      return 'just now';
    }
    
    const diffMinutes = Math.floor(diffSeconds / 60);
    if (diffMinutes < 60) {
      return `${diffMinutes} minute${diffMinutes !== 1 ? 's' : ''} ago`;
    }
    
    const diffHours = Math.floor(diffMinutes / 60);
    if (diffHours < 24) {
      return `${diffHours} hour${diffHours !== 1 ? 's' : ''} ago`;
    }
    
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays < 30) {
      return `${diffDays} day${diffDays !== 1 ? 's' : ''} ago`;
    }
    
    const diffMonths = Math.floor(diffDays / 30);
    if (diffMonths < 12) {
      return `${diffMonths} month${diffMonths !== 1 ? 's' : ''} ago`;
    }
    
    const diffYears = Math.floor(diffMonths / 12);
    return `${diffYears} year${diffYears !== 1 ? 's' : ''} ago`;
  }
}
