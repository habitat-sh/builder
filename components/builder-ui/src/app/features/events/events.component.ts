// filepath: /Users/psajja/Workspace/habitat-sh/builder/components/builder-ui/src/app/features/events/events.component.ts
import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { ReactiveFormsModule, FormControl } from '@angular/forms';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { MatButtonModule } from '@angular/material/button';
import { MatCardModule } from '@angular/material/card';
import { MatTableModule } from '@angular/material/table';
import { MatPaginatorModule } from '@angular/material/paginator';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatDatepickerModule } from '@angular/material/datepicker';
import { MatNativeDateModule } from '@angular/material/core';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import { finalize } from 'rxjs';

import { BaseEventsComponent } from './base-events.component';
import { Event, EventsResponse, EventsSearchParams } from './models/event.model';
import { AuthService } from '../../core/services/auth.service';
import { HeaderTitleDirective, HeaderActionsDirective } from '../../core/layout/shared';
import { DateFilterComponent } from './components/date-filter/date-filter.component';
import { EventsResultsComponent } from './components/events-results/events-results.component';
import { DateFilter, getDateRange } from './utils/date-util';

@Component({
  selector: 'app-events',
  standalone: true,
  imports: [
    CommonModule,
    RouterLink,
    ReactiveFormsModule,
    MatFormFieldModule,
    MatInputModule,
    MatSelectModule,
    MatButtonModule,
    MatCardModule,
    MatTableModule,
    MatPaginatorModule,
    MatProgressSpinnerModule,
    MatDatepickerModule,
    MatNativeDateModule,
    MatIconModule,
    MatTooltipModule,
    HeaderTitleDirective,
    HeaderActionsDirective,
    DateFilterComponent,
    EventsResultsComponent
  ],
  template: `
    <!-- Header Title Template -->
    <ng-template habHeaderTitle>
      <h1>Builder Events</h1>
      <h2 *ngIf="queryControl.value">Search Results</h2>
    </ng-template>
    
    <div class="events-component">
      <div class="body">
        <div class="content">
          <section class="events-filter">
            <input
              type="search"
              [formControl]="queryControl"
              placeholder="Search Events&hellip;">
              
            <app-date-filter
              [dateFilterChanged]="onDateFilterChanged"
              [currentFilter]="currentDateFilter">
            </app-date-filter>
          </section>
          
          <section>
            <div class="loading-spinner" *ngIf="loading">
              <mat-spinner diameter="50"></mat-spinner>
            </div>
            
            <app-events-results
              *ngIf="!loading"
              [events]="events"
              [noEvents]="events.length === 0"
              [errorMessage]="errorMessage">
            </app-events-results>
          </section>
          
          <section class="more" *ngIf="events.length < totalEvents">
            Showing {{events.length}} of {{totalEvents}} events.
            <a (click)="loadMoreEvents()" class="load-more">
              Load {{(totalEvents - events.length) > pageSize ? pageSize : totalEvents - events.length}} more.
            </a>
          </section>
        </div>
      </div>
    </div>
  `,
  styles: [`
    /* Header styles - from builder-web */
    .events-component {
      width: 100%;
    }
    
    header {
      position: relative;
      
      h1 {
        font-size: 20px;
        font-weight: normal;
        margin: 0;
        line-height: 44px;
        font-family: "Titillium Web", "Helvetica Neue", Helvetica, Roboto, Arial, sans-serif;
        display: block;
      }
      
      h2 {
        font-size: 16px;
        font-weight: normal;
        margin: 0;
        color: rgba(0, 0, 0, 0.7);
        line-height: 22px;
        font-family: "Titillium Web", "Helvetica Neue", Helvetica, Roboto, Arial, sans-serif;
        display: block;
      }
    }
    
    .body {
      display: flex;
      flex-direction: column;
      padding: 20px;
      
      .content {
        width: 100%;
        min-width: 100%;
        padding-right: 0;
      }
    }
    
    .actions {
      margin-bottom: 20px;
    }
    
    .events-filter {
      display: flex;
      justify-content: space-between;
      flex-direction: row;
      margin-bottom: 24px;
      
      > input {
        max-width: 65%;
        padding: 12px;
        font-size: 16px;
        border: 1px solid #ccc;
        border-radius: 4px;
      }
    }
    
    .loading-spinner {
      display: flex;
      justify-content: center;
      align-items: center;
      padding: 40px 0;
    }
    
    .more {
      margin-top: 20px;
      text-align: center;
      color: #666;
      
      a {
        cursor: pointer;
        color: #0366d6;
        text-decoration: none;
        
        &:hover {
          text-decoration: underline;
        }
      }
    }
    
    @media screen and (max-width: 768px) {
      .events-filter {
        align-items: start !important;
        flex-direction: column !important;
        justify-content: start !important;
      }
    }
  `]
})
export class EventsComponent extends BaseEventsComponent {
  // User information for header
  isAuthenticated = false;
  username = '';
  avatarUrl = '';
  
  // Error message for displaying API errors
  errorMessage = '';
  
  // Date filter
  override currentDateFilter: DateFilter = { 
    label: 'Last 1 Week', 
    type: 'days', 
    interval: 7
  };
  
  // Auth service to get user info
  private authService = inject(AuthService);
  
  constructor() {
    super();
  }
  
  override ngOnInit(): void {
    super.ngOnInit();
    this.setupUserInfo();
  }
  
  /**
   * Handle date filter changes
   */
  override onDateFilterChanged = (filter: DateFilter): void => {
    this.currentDateFilter = filter;
    const dateRange = getDateRange(filter);
    
    // Update the form controls
    this.searchForm.patchValue({
      fromDate: dateRange.fromDate,
      toDate: dateRange.toDate
    });
    
    // Trigger search with new date range
    this.onSearch();
  }
  
  /**
   * Reset all filters to default values
   */
  override resetFilters(): void {
    // Reset date filter to default
    this.currentDateFilter = { 
      label: 'Last 1 Week', 
      type: 'days', 
      interval: 7
    };
    
    // Call the parent method to reset form values
    super.resetFilters();
  }
  
  /**
   * Build search parameters from form values
   */
  protected override buildSearchParams(): EventsSearchParams {
    const formValues = this.searchForm.value;
    const params: EventsSearchParams = {
      range: this.pageSize,
      channel: formValues.channel || 'stable',
      from_date: formValues.fromDate,
      to_date: formValues.toDate,
      query: formValues.query || ''
    };
    
    return params;
  }
  
  // Helper getters to access form controls with correct typing
  get queryControl(): FormControl {
    return this.searchForm.get('query') as FormControl;
  }

  get channelControl(): FormControl {
    return this.searchForm.get('channel') as FormControl;
  }

  get fromDateControl(): FormControl {
    return this.searchForm.get('fromDate') as FormControl;
  }

  get toDateControl(): FormControl {
    return this.searchForm.get('toDate') as FormControl;
  }
  
  /**
   * Set up user information for header
   */
  private setupUserInfo(): void {
    this.isAuthenticated = this.authService.isAuthenticated();
    const user = this.authService.currentUser();
    if (user) {
      this.username = user.name;
      this.avatarUrl = user.avatar || '';
    }
  }
  
  /**
   * Handle logout event from header
   */
  handleLogout(): void {
    this.authService.logout();
  }

  /**
   * Load events from API
   */
  protected override loadEvents(): void {
    this.loading = true;
    const params: EventsSearchParams = this.buildSearchParams();

    this.subscriptions.add(
      this.eventsService.getEvents(params)
        .pipe(finalize(() => this.loading = false))
        .subscribe({
          next: (response) => this.handleEventsResponse(response),
          error: (error) => {
            this.handleError(error);
            this.errorMessage = 'Failed to load events. Please try again.';
          }
        })
    );
  }
}
