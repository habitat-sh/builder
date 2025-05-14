/**
 * This file contains the template and styles that should be used in the events.component.ts file
 * after fixing the corruption issues.
 */

// Template for events.component.ts
const eventsTemplate = `
<div class="events-component">
  <header>
    <h1>Builder Events</h1>
    <h2 *ngIf="searchForm.get('query')?.value">Search Results</h2>
  </header>
  
  <div class="body">
    <div class="content">
      <div class="actions">
        <a mat-button color="primary" [routerLink]="['/events/saas']">View SaaS Events</a>
      </div>
      
      <section class="events-filter">
        <input
          type="search"
          formControlName="query"
          placeholder="Search Events&hellip;"
          [formControl]="searchForm.controls.query">
          
        <div class="date-filter-container">
          <mat-form-field appearance="outline">
            <mat-label>Channel</mat-label>
            <mat-select formControlName="channel" [formControl]="searchForm.controls.channel">
              <mat-option *ngFor="let channel of availableChannels" [value]="channel">
                {{ channel }}
              </mat-option>
            </mat-select>
          </mat-form-field>

          <mat-form-field appearance="outline">
            <mat-label>From Date</mat-label>
            <input matInput [matDatepicker]="fromPicker" [formControl]="searchForm.controls.fromDate">
            <mat-datepicker-toggle matSuffix [for]="fromPicker"></mat-datepicker-toggle>
            <mat-datepicker #fromPicker></mat-datepicker>
          </mat-form-field>

          <mat-form-field appearance="outline">
            <mat-label>To Date</mat-label>
            <input matInput [matDatepicker]="toPicker" [formControl]="searchForm.controls.toDate">
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
`;

// Styles for events.component.ts
const eventsStyles = `
.events-component {
  width: 100%;
  padding: 20px;
}
header {
  margin-bottom: 20px;
}
header h1 {
  font-size: 24px;
  margin-bottom: 8px;
}
header h2 {
  font-size: 18px;
  font-weight: normal;
  color: #666;
}
.body {
  display: flex;
  flex-direction: column;
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
`;
