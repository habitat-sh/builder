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
    HeaderActionsDirective
  ],
  template: `
    <!-- Header Title Template -->
    <ng-template habHeaderTitle>
      <h1>Builder Events</h1>
      <h2 *ngIf="queryControl.value">Search Results</h2>
    </ng-template>
    
    <!-- Header Actions Template (empty for now) -->
    <ng-template habHeaderActions>
      <!-- Actions can be added here if needed -->
    </ng-template>
    
    <div class="events-component">
      <div class="body">
        <div class="content">
          <div class="actions">
            <a mat-button color="primary" [routerLink]="['/events/saas']">View SaaS Events</a>
          </div>
          
          <section class="events-filter">
            <input
              type="search"
              [formControl]="queryControl"
              placeholder="Search Events&hellip;">
              
            <div class="date-filter-container">
              <mat-form-field appearance="outline">
                <mat-label>Channel</mat-label>
                <mat-select [formControl]="channelControl">
                  <mat-option *ngFor="let channel of availableChannels" [value]="channel">
                    {{ channel }}
                  </mat-option>
                </mat-select>
              </mat-form-field>

              <mat-form-field appearance="outline">
                <mat-label>From Date</mat-label>
                <input matInput [matDatepicker]="fromPicker" [formControl]="fromDateControl">
                <mat-datepicker-toggle matSuffix [for]="fromPicker"></mat-datepicker-toggle>
                <mat-datepicker #fromPicker></mat-datepicker>
              </mat-form-field>

              <mat-form-field appearance="outline">
                <mat-label>To Date</mat-label>
                <input matInput [matDatepicker]="toPicker" [formControl]="toDateControl">
                <mat-datepicker-toggle matSuffix [for]="toPicker"></mat-datepicker-toggle>
                <mat-datepicker #toPicker></mat-datepicker>
              </mat-form-field>
              
              <div class="button-row">
                <button mat-raised-button color="primary" (click)="onSearch()" [disabled]="loading">
                  <span>Search</span>
                </button>
                <button mat-stroked-button type="button" (click)="resetFilters()" [disabled]="loading">
                  Reset
                </button>
              </div>
            </div>
          </section>
          
          <section>
            <div class="results-container">
              <div class="loading-spinner" *ngIf="loading">
                <mat-spinner diameter="50"></mat-spinner>
              </div>

              <ng-container *ngIf="!loading">
                <div class="no-results" *ngIf="events.length === 0">
                  No events found. Try adjusting your search criteria.
                </div>
                
                <ng-container *ngIf="events.length > 0">
                  <table mat-table [dataSource]="events" class="events-table">
                    <!-- Operation Column -->
                    <ng-container matColumnDef="operation">
                      <th mat-header-cell *matHeaderCellDef> Operation </th>
                      <td mat-cell *matCellDef="let event"> {{ event.operation }} </td>
                    </ng-container>

                    <!-- Date Column -->
                    <ng-container matColumnDef="created_at">
                      <th mat-header-cell *matHeaderCellDef> Date </th>
                      <td mat-cell *matCellDef="let event"> {{ formatDate(event.created_at) }} </td>
                    </ng-container>

                    <!-- Origin Column -->
                    <ng-container matColumnDef="origin">
                      <th mat-header-cell *matHeaderCellDef> Origin </th>
                      <td mat-cell *matCellDef="let event"> {{ event.origin }} </td>
                    </ng-container>

                    <!-- Package Column -->
                    <ng-container matColumnDef="package">
                      <th mat-header-cell *matHeaderCellDef> Package </th>
                      <td mat-cell *matCellDef="let event" [matTooltip]="formatPackageIdent(event.package_ident)"> 
                        {{ event.package_ident.name }}/{{ event.package_ident.version }}/{{ event.package_ident.release }}
                      </td>
                    </ng-container>

                    <!-- Channel Column -->
                    <ng-container matColumnDef="channel">
                      <th mat-header-cell *matHeaderCellDef> Channel </th>
                      <td mat-cell *matCellDef="let event"> {{ event.channel }} </td>
                    </ng-container>

                    <tr mat-header-row *matHeaderRowDef="['operation', 'created_at', 'origin', 'package', 'channel']"></tr>
                    <tr mat-row *matRowDef="let row; columns: ['operation', 'created_at', 'origin', 'package', 'channel'];"></tr>
                  </table>
                </ng-container>
              </ng-container>
            </div>
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
    .events-component {
      width: 100%;
    }
    .body {
      display: flex;
      flex-direction: column;
      padding: 20px;
    }
    .content {
      width: 100%;
    }
    .actions {
      margin-bottom: 20px;
    }
    .events-filter {
      margin-bottom: 24px;
    }
    .events-filter input[type="search"] {
      width: 100%;
      padding: 12px;
      font-size: 16px;
      border: 1px solid #ccc;
      border-radius: 4px;
      margin-bottom: 16px;
    }
    .date-filter-container {
      display: flex;
      flex-wrap: wrap;
      gap: 16px;
      align-items: flex-start;
    }
    .button-row {
      display: flex;
      gap: 10px;
      margin: 16px 0;
    }
    .loading-spinner {
      display: flex;
      justify-content: center;
      align-items: center;
      padding: 40px 0;
    }
    .no-results {
      text-align: center;
      padding: 40px 0;
      color: #666;
    }
    .events-table {
      width: 100%;
    }
    .more {
      margin-top: 20px;
      text-align: center;
      color: #666;
    }
    .load-more {
      cursor: pointer;
      color: #0366d6;
      text-decoration: none;
    }
    .load-more:hover {
      text-decoration: underline;
    }
  `]
})
export class EventsComponent extends BaseEventsComponent {
  // User information for header
  isAuthenticated = false;
  username = '';
  avatarUrl = '';
  
  // Auth service to get user info
  private authService = inject(AuthService);
  
  override ngOnInit(): void {
    super.ngOnInit();
    this.setupUserInfo();
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
          error: (error) => this.handleError(error)
        })
    );
  }
}
