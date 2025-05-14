/**
 * This file contains the template and styles that should be used in the events-saas.component.ts file
 * after fixing the corruption issues.
 */

// Template for events-saas.component.ts
const eventsSaasTemplate = `
<div class="events-component">
  <header>
    <h1>Builder Events (SaaS)</h1>
    <h2 *ngIf="searchForm.get('query')?.value">Search Results</h2>
  </header>
  
  <div class="body">
    <div class="content">
      <div class="actions">
        <a mat-button color="primary" [routerLink]="['/events']">Back to Events</a>
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

// The styles are the same as for events.component.ts
